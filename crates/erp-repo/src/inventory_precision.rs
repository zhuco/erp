//! Exact inventory quantities in the original balance, ledger and outbox transaction.

use chrono::{DateTime, Utc};
use erp_domain::quantity::{QTY_SCALE, ScaledQty, format_qty};
use erp_types::TenantId;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    inventory::{INVENTORY_CHANGED_EVENT, InventoryBalance, InventoryService},
    outbox::OutboxRepo,
};

#[derive(Debug, thiserror::Error)]
pub enum InventoryPrecisionError {
    #[error(
        "insufficient exact available inventory: available {available_micros}, requested {requested_micros} micro-units"
    )]
    InsufficientAvailable {
        available_micros: i128,
        requested_micros: i128,
    },
    #[error("exact inventory change exceeds the current {0} quantity")]
    InvalidChange(&'static str),
    #[error("inventory quantity reconciliation required: {0}")]
    ReconciliationRequired(&'static str),
    #[error("inventory warehouse or SKU is outside the enterprise")]
    OutsideEnterprise,
}

#[derive(Debug, Clone, Copy)]
pub struct InventoryQuantityKey {
    pub tenant_id: TenantId,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct InventoryPreciseDelta {
    pub key: InventoryQuantityKey,
    pub on_hand: ScaledQty,
    pub reserved: ScaledQty,
    pub frozen: ScaledQty,
    pub in_transit: ScaledQty,
    pub reason_type: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InventoryPreciseBalance {
    pub key: InventoryQuantityKey,
    pub on_hand: ScaledQty,
    pub reserved: ScaledQty,
    pub frozen: ScaledQty,
    pub in_transit: ScaledQty,
    pub updated_at: DateTime<Utc>,
}

/// The real SKU ledger and its exact before/after balances, in the caller's tx.
#[derive(Debug, Clone)]
pub struct InventoryPreciseEffect {
    pub before: InventoryPreciseBalance,
    pub balance: InventoryPreciseBalance,
    pub ledger_id: Uuid,
}

impl InventoryPreciseBalance {
    pub fn available(&self) -> anyhow::Result<ScaledQty> {
        Ok(self.on_hand.try_sub(self.reserved)?.try_sub(self.frozen)?)
    }

    pub(crate) fn whole_projection(&self) -> anyhow::Result<InventoryBalance> {
        let available = self.available()?;
        Ok(InventoryBalance {
            tenant_id: self.key.tenant_id,
            warehouse_id: self.key.warehouse_id,
            sku_id: self.key.sku_id,
            on_hand: self.on_hand.as_micros() / QTY_SCALE,
            reserved: self.reserved.as_micros() / QTY_SCALE,
            frozen: self.frozen.as_micros() / QTY_SCALE,
            in_transit: self.in_transit.as_micros() / QTY_SCALE,
            updated_at: self.updated_at,
            quantities: crate::inventory_reads::InventoryReadQuantities {
                on_hand: format_qty(self.on_hand),
                reserved: format_qty(self.reserved),
                frozen: format_qty(self.frozen),
                in_transit: format_qty(self.in_transit),
                available: format_qty(available),
            },
            available_whole: available.as_micros() / QTY_SCALE,
        })
    }

    fn available_wide(&self) -> i128 {
        i128::from(self.on_hand.as_micros())
            - i128::from(self.reserved.as_micros())
            - i128::from(self.frozen.as_micros())
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.on_hand.is_negative()
            || self.reserved.is_negative()
            || self.frozen.is_negative()
            || self.in_transit.is_negative()
            || self.available_wide() < 0
        {
            return Err(InventoryPrecisionError::ReconciliationRequired(
                "stored physical, reserved, frozen, transit or available quantity is negative",
            )
            .into());
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct BalanceRow {
    on_hand: i64,
    reserved: i64,
    frozen: i64,
    in_transit: i64,
    on_hand_micros: Option<i64>,
    reserved_micros: Option<i64>,
    frozen_micros: Option<i64>,
    in_transit_micros: Option<i64>,
    updated_at: DateTime<Utc>,
}

impl BalanceRow {
    fn exact(&self, key: InventoryQuantityKey) -> anyhow::Result<InventoryPreciseBalance> {
        Ok(InventoryPreciseBalance {
            key,
            on_hand: ScaledQty::from_micros(self.on_hand_micros.ok_or(
                InventoryPrecisionError::ReconciliationRequired("exact on-hand missing"),
            )?),
            reserved: ScaledQty::from_micros(self.reserved_micros.ok_or(
                InventoryPrecisionError::ReconciliationRequired("exact reserved missing"),
            )?),
            frozen: ScaledQty::from_micros(self.frozen_micros.ok_or(
                InventoryPrecisionError::ReconciliationRequired("exact frozen missing"),
            )?),
            in_transit: ScaledQty::from_micros(self.in_transit_micros.ok_or(
                InventoryPrecisionError::ReconciliationRequired("exact transit missing"),
            )?),
            updated_at: self.updated_at,
        })
    }
}

impl InventoryService<'_> {
    pub async fn get_precise_balance(
        &self,
        key: InventoryQuantityKey,
    ) -> anyhow::Result<Option<InventoryPreciseBalance>> {
        let mut tx = crate::inventory_reads::begin_read(self.pool).await?;
        let balance = self.get_precise_balance_with_tx(&mut tx, key).await?;
        tx.commit().await?;
        Ok(balance)
    }

    /// A quote can read the SKU balance and its batches from one read-only
    /// snapshot. Historical activation remains an explicit write operation.
    pub(crate) async fn get_precise_balance_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        key: InventoryQuantityKey,
    ) -> anyhow::Result<Option<InventoryPreciseBalance>> {
        let owned: bool = sqlx::query_scalar("select exists(select 1 from warehouses where tenant_id=$1 and warehouse_id=$2) and exists(select 1 from skus where tenant_id=$1 and sku_id=$3)")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id).fetch_one(&mut **tx).await?;
        if !owned {
            return Err(InventoryPrecisionError::OutsideEnterprise.into());
        }
        let row:Option<BalanceRow>=sqlx::query_as("select on_hand,reserved,frozen,in_transit,on_hand_micros,reserved_micros,frozen_micros,in_transit_micros,updated_at from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id).fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            let source_exists: bool = sqlx::query_scalar(
                "select exists(select 1 from inventory_ledgers where tenant_id=$1 and warehouse_id=$2 and sku_id=$3) or exists(select 1 from mfg_qty_ledgers where tenant_id=$1 and warehouse_id=$2 and sku_id=$3) or exists(select 1 from shared_stock_ledgers where tenant_id=$1 and warehouse_id=$2 and sku_id=$3)",
            ).bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id)
                .fetch_one(&mut **tx).await?;
            if source_exists {
                return Err(InventoryPrecisionError::ReconciliationRequired(
                    "stock history exists without its physical balance",
                )
                .into());
            }
            return Ok(None);
        };
        if row.on_hand_micros.is_none() {
            return Err(InventoryPrecisionError::ReconciliationRequired(
                "historical key has not passed exact activation",
            )
            .into());
        }
        let balance = row.exact(key)?;
        balance.validate()?;
        Ok(Some(balance))
    }

    pub async fn apply_precise(
        &self,
        input: InventoryPreciseDelta,
    ) -> anyhow::Result<InventoryPreciseBalance> {
        let mut tx = self.pool.begin().await?;
        let balance = self.apply_precise_with_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(balance)
    }

    /// All legacy whole-unit writes also delegate here after checked conversion.
    pub async fn apply_precise_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryPreciseDelta,
    ) -> anyhow::Result<InventoryPreciseBalance> {
        Ok(self.apply_precise_effect_with_tx(tx, input).await?.balance)
    }

    pub async fn apply_precise_effect_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryPreciseDelta,
    ) -> anyhow::Result<InventoryPreciseEffect> {
        require_owned_key(tx, input.key).await?;
        // A real zero row serializes concurrent first creation. An unsuccessful
        // outgoing command rolls it back rather than recording negative stock.
        sqlx::query("insert into inventory_balances(tenant_id,warehouse_id,sku_id) values($1,$2,$3) on conflict(tenant_id,warehouse_id,sku_id) do nothing")
            .bind(Uuid::from(input.key.tenant_id)).bind(input.key.warehouse_id).bind(input.key.sku_id)
            .execute(&mut **tx).await?;
        let before = self.lock_precise_balance(tx, input.key).await?;
        let next = InventoryPreciseBalance {
            key: input.key,
            on_hand: before.on_hand.try_add(input.on_hand)?,
            reserved: before.reserved.try_add(input.reserved)?,
            frozen: before.frozen.try_add(input.frozen)?,
            in_transit: before.in_transit.try_add(input.in_transit)?,
            updated_at: before.updated_at,
        };
        for (bucket, quantity) in [
            ("reserved", next.reserved),
            ("frozen", next.frozen),
            ("in-transit", next.in_transit),
        ] {
            if quantity.is_negative() {
                return Err(InventoryPrecisionError::InvalidChange(bucket).into());
            }
        }
        if next.on_hand.is_negative() || next.available_wide() < 0 {
            return Err(InventoryPrecisionError::InsufficientAvailable {
                available_micros: before.available_wide(),
                requested_micros: before.available_wide() - next.available_wide(),
            }
            .into());
        }
        let updated_at: DateTime<Utc> = sqlx::query_scalar("update inventory_balances set on_hand_micros=$4,reserved_micros=$5,frozen_micros=$6,in_transit_micros=$7,updated_at=now() where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 returning updated_at")
            .bind(Uuid::from(input.key.tenant_id)).bind(input.key.warehouse_id).bind(input.key.sku_id)
            .bind(next.on_hand.as_micros()).bind(next.reserved.as_micros()).bind(next.frozen.as_micros()).bind(next.in_transit.as_micros())
            .fetch_one(&mut **tx).await?;
        let ledger_id: Uuid = sqlx::query_scalar("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,ref_type,ref_id,occurred_at) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) returning ledger_id")
            .bind(Uuid::from(input.key.tenant_id)).bind(input.key.warehouse_id).bind(input.key.sku_id)
            .bind(input.on_hand.as_micros()/QTY_SCALE).bind(input.reserved.as_micros()/QTY_SCALE)
            .bind(input.frozen.as_micros()/QTY_SCALE).bind(input.in_transit.as_micros()/QTY_SCALE)
            .bind(input.on_hand.as_micros()).bind(input.reserved.as_micros())
            .bind(input.frozen.as_micros()).bind(input.in_transit.as_micros())
            .bind(&input.reason_type).bind(&input.ref_type).bind(&input.ref_id).bind(input.occurred_at)
            .fetch_one(&mut **tx).await?;
        let next = InventoryPreciseBalance { updated_at, ..next };
        if before.available()? != next.available()? {
            let mapped:bool=sqlx::query_scalar("select exists(select 1 from channel_sku_mappings where tenant_id=$1 and sku_id=$2 and status='ACTIVE')")
                .bind(Uuid::from(input.key.tenant_id)).bind(input.key.sku_id).fetch_one(&mut **tx).await?;
            if mapped {
                let available = next.available()?.as_micros();
                let payload = serde_json::json!({
                    "tenant_id":input.key.tenant_id,"warehouse_id":input.key.warehouse_id,"sku_id":input.key.sku_id,
                    "quantity_scale":QTY_SCALE,
                    "on_hand_micros":next.on_hand.as_micros().to_string(),
                    "reserved_micros":next.reserved.as_micros().to_string(),
                    "frozen_micros":next.frozen.as_micros().to_string(),
                    "in_transit_micros":next.in_transit.as_micros().to_string(),
                    "available_micros":available.to_string(),
                    "on_hand":next.on_hand.as_micros()/QTY_SCALE,
                    "reserved":next.reserved.as_micros()/QTY_SCALE,
                    "frozen":next.frozen.as_micros()/QTY_SCALE,
                    "in_transit":next.in_transit.as_micros()/QTY_SCALE,
                    "available":available/QTY_SCALE,"occurred_at":input.occurred_at,
                });
                OutboxRepo::enqueue_with_tx(
                    &mut **tx,
                    input.key.tenant_id,
                    INVENTORY_CHANGED_EVENT,
                    &payload,
                    input.occurred_at,
                )
                .await?;
            }
        }
        Ok(InventoryPreciseEffect {
            before,
            balance: next,
            ledger_id,
        })
    }

    /// Existing material stock must exist; this method never fabricates an opening.
    pub async fn lock_precise_balance(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        key: InventoryQuantityKey,
    ) -> anyhow::Result<InventoryPreciseBalance> {
        require_owned_key(tx, key).await?;
        let row:BalanceRow=sqlx::query_as("select on_hand,reserved,frozen,in_transit,on_hand_micros,reserved_micros,frozen_micros,in_transit_micros,updated_at from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 for update")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id).fetch_one(&mut **tx).await?;
        if row.on_hand_micros.is_some() {
            let exact = row.exact(key)?;
            exact.validate()?;
            return Ok(exact);
        }
        // Existing exact ledger facts take precedence over rounded compatibility
        // columns. Only truly integer-only facts receive the checked scale factor.
        let totals: (String, String, String, String) = sqlx::query_as(
            "select coalesce(sum(coalesce(delta_on_hand_micros::numeric,delta_on_hand::numeric*1000000)),0)::text,
                    coalesce(sum(coalesce(delta_reserved_micros::numeric,delta_reserved::numeric*1000000)),0)::text,
                    coalesce(sum(coalesce(delta_frozen_micros::numeric,delta_frozen::numeric*1000000)),0)::text,
                    coalesce(sum(coalesce(delta_in_transit_micros::numeric,delta_in_transit::numeric*1000000)),0)::text
             from inventory_ledgers where tenant_id=$1 and warehouse_id=$2 and sku_id=$3",
        ).bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id)
            .fetch_one(&mut **tx).await?;
        let exact = InventoryPreciseBalance {
            key,
            on_hand: quantity_from_history(&totals.0)?,
            reserved: quantity_from_history(&totals.1)?,
            frozen: quantity_from_history(&totals.2)?,
            in_transit: quantity_from_history(&totals.3)?,
            updated_at: row.updated_at,
        };
        exact.validate()?;
        // Check real ledger evidence before activating the historical key. Missing
        // or different physical facts must be reconciled explicitly, never added.
        let projection = exact.whole_projection()?;
        let ledger_matches = (row.on_hand, row.reserved, row.frozen, row.in_transit)
            == (
                projection.on_hand,
                projection.reserved,
                projection.frozen,
                projection.in_transit,
            );
        let shadow:Option<(i64,i64)>=sqlx::query_as("select on_hand_micros,reserved_micros from mfg_qty_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 for update")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id).fetch_optional(&mut **tx).await?;
        if !ledger_matches
            || shadow.is_some_and(|v| v != (exact.on_hand.as_micros(), exact.reserved.as_micros()))
        {
            return Err(InventoryPrecisionError::ReconciliationRequired(
                "balance, ledger or manufacturing history differs",
            )
            .into());
        }
        let unmapped:bool=sqlx::query_scalar("select exists(select 1 from mfg_qty_ledgers m where m.tenant_id=$1 and m.warehouse_id=$2 and m.sku_id=$3 and m.delta_micros<>0 and (select count(*) from inventory_ledgers i where i.tenant_id=m.tenant_id and i.warehouse_id=m.warehouse_id and i.sku_id=m.sku_id and i.ref_type='mfg_material_move' and i.ref_id=m.move_id::text and coalesce(i.delta_on_hand_micros::numeric,i.delta_on_hand::numeric*1000000)=m.delta_micros)<>1)")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id).fetch_one(&mut **tx).await?;
        if unmapped {
            return Err(InventoryPrecisionError::ReconciliationRequired(
                "manufacturing physical effects lack unique inventory sources",
            )
            .into());
        }
        let unproven_shared: bool = sqlx::query_scalar(
            "select exists(select 1 from shared_stock_ledgers s
             where s.tenant_id=$1 and s.warehouse_id=$2 and s.sku_id=$3 and s.delta_micros<>0
              and (select count(*) from inventory_ledgers i
               where i.tenant_id=s.tenant_id and i.warehouse_id=s.warehouse_id and i.sku_id=s.sku_id
                and i.ref_type='shared_stock_ledger' and i.ref_id=s.ledger_id::text
                and case s.bucket
                 when 'ON_HAND' then coalesce(i.delta_on_hand_micros::numeric,i.delta_on_hand::numeric*1000000)
                 when 'RESERVED' then coalesce(i.delta_reserved_micros::numeric,i.delta_reserved::numeric*1000000)
                 when 'FROZEN' then coalesce(i.delta_frozen_micros::numeric,i.delta_frozen::numeric*1000000)
                 when 'IN_TRANSIT' then coalesce(i.delta_in_transit_micros::numeric,i.delta_in_transit::numeric*1000000)
                 else null
                end=s.delta_micros)<>1)",
        ).bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id)
            .fetch_one(&mut **tx).await?;
        if unproven_shared {
            return Err(InventoryPrecisionError::ReconciliationRequired(
                "shared stock physical effects lack unique inventory sources",
            )
            .into());
        }
        // Add precision fields without changing old business quantities or source IDs.
        sqlx::query("update inventory_ledgers set delta_on_hand_micros=coalesce(delta_on_hand_micros,(delta_on_hand::numeric*1000000)::bigint),delta_reserved_micros=coalesce(delta_reserved_micros,(delta_reserved::numeric*1000000)::bigint),delta_frozen_micros=coalesce(delta_frozen_micros,(delta_frozen::numeric*1000000)::bigint),delta_in_transit_micros=coalesce(delta_in_transit_micros,(delta_in_transit::numeric*1000000)::bigint) where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id).execute(&mut **tx).await?;
        sqlx::query("insert into inventory_quantity_source_maps(tenant_id,manufacturing_ledger_id,inventory_ledger_id,qty_micros) select m.tenant_id,m.ledger_id,i.ledger_id,m.delta_micros from mfg_qty_ledgers m join inventory_ledgers i on i.tenant_id=m.tenant_id and i.warehouse_id=m.warehouse_id and i.sku_id=m.sku_id and i.ref_type='mfg_material_move' and i.ref_id=m.move_id::text and i.delta_on_hand_micros=m.delta_micros where m.tenant_id=$1 and m.warehouse_id=$2 and m.sku_id=$3 and m.delta_micros<>0")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id).execute(&mut **tx).await?;
        sqlx::query("insert into inventory_quantity_activations(tenant_id,warehouse_id,sku_id,legacy_on_hand,legacy_reserved,legacy_frozen,legacy_in_transit) values($1,$2,$3,$4,$5,$6,$7)")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id)
            .bind(row.on_hand).bind(row.reserved).bind(row.frozen).bind(row.in_transit).execute(&mut **tx).await?;
        sqlx::query("update inventory_balances set on_hand_micros=$4,reserved_micros=$5,frozen_micros=$6,in_transit_micros=$7 where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
            .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id)
            .bind(exact.on_hand.as_micros()).bind(exact.reserved.as_micros()).bind(exact.frozen.as_micros()).bind(exact.in_transit.as_micros())
            .execute(&mut **tx).await?;
        Ok(exact)
    }
}

fn quantity_from_history(value: &str) -> Result<ScaledQty, InventoryPrecisionError> {
    value
        .parse::<i64>()
        .map(ScaledQty::from_micros)
        .map_err(|_| {
            InventoryPrecisionError::ReconciliationRequired(
                "inventory history exceeds the supported exact quantity range",
            )
        })
}

pub(crate) async fn require_owned_key(
    tx: &mut Transaction<'_, Postgres>,
    key: InventoryQuantityKey,
) -> anyhow::Result<()> {
    let owned:bool=sqlx::query_scalar("select exists(select 1 from warehouses where tenant_id=$1 and warehouse_id=$2) and exists(select 1 from skus where tenant_id=$1 and sku_id=$3)")
        .bind(Uuid::from(key.tenant_id)).bind(key.warehouse_id).bind(key.sku_id).fetch_one(&mut **tx).await?;
    if !owned {
        return Err(InventoryPrecisionError::OutsideEnterprise.into());
    }
    // DB writer guards use this same key, including when no balance row exists.
    sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
        .bind(Uuid::from(key.tenant_id).to_string())
        .bind(format!("inventory:{}:{}", key.warehouse_id, key.sku_id))
        .execute(&mut **tx)
        .await?;
    Ok(())
}
