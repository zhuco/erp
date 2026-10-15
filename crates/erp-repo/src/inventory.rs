use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use thiserror::Error;

use crate::inventory_precision::{
    InventoryPreciseDelta, InventoryPreciseEffect, InventoryQuantityKey,
};
use erp_domain::quantity::ScaledQty;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryBalance {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub on_hand: i64,
    pub reserved: i64,
    pub frozen: i64,
    pub in_transit: i64,
    pub updated_at: DateTime<Utc>,
    pub quantities: crate::inventory_reads::InventoryReadQuantities,
    pub(crate) available_whole: i64,
}

impl InventoryBalance {
    pub fn available(&self) -> i64 {
        self.available_whole
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryRecord {
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub cost_cent: Option<i64>,
    pub on_hand: i64,
    pub reserved: i64,
    pub frozen: i64,
    pub in_transit: i64,
    pub updated_at: DateTime<Utc>,
    pub quantities: crate::inventory_reads::InventoryReadQuantities,
    pub available_whole: i64,
}

impl InventoryRecord {
    pub fn available(&self) -> i64 {
        self.available_whole
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySummaryRecord {
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub on_hand: i64,
    pub reserved: i64,
    pub frozen: i64,
    pub in_transit: i64,
    pub updated_at: DateTime<Utc>,
    pub quantities: crate::inventory_reads::InventoryReadQuantities,
    pub available_whole: i64,
}

impl InventorySummaryRecord {
    pub fn available(&self) -> i64 {
        self.available_whole
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCostRecord {
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub on_hand: i64,
    pub reserved: i64,
    pub frozen: i64,
    pub in_transit: i64,
    pub updated_at: DateTime<Utc>,
    pub unit_cost_cent: Option<i64>,
    pub total_cost_cent: Option<i64>,
    pub estimated_value: Option<String>,
    pub quantities: crate::inventory_reads::InventoryReadQuantities,
    pub available_whole: i64,
}

impl InventoryCostRecord {
    pub fn available(&self) -> i64 {
        self.available_whole
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryLedgerRecord {
    pub ledger_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub delta_on_hand: i64,
    pub delta_reserved: i64,
    pub delta_in_transit: i64,
    pub delta_frozen: i64,
    pub reason_type: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub quantities: crate::inventory_reads::InventoryReadDelta,
}

pub const INVENTORY_CHANGED_EVENT: &str = "INVENTORY_CHANGED";

#[derive(Debug, Clone)]
pub struct InventoryCursor {
    pub updated_at: DateTime<Utc>,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
}

impl InventoryCursor {
    pub fn encode(&self) -> String {
        format!(
            "{}|{}|{}",
            self.updated_at.to_rfc3339(),
            self.warehouse_id,
            self.sku_id
        )
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let mut parts = raw.split('|');
        let ts = parts.next().ok_or_else(|| anyhow!("invalid cursor"))?;
        let warehouse_id = parts.next().ok_or_else(|| anyhow!("invalid cursor"))?;
        let sku_id = parts.next().ok_or_else(|| anyhow!("invalid cursor"))?;
        if parts.next().is_some() {
            return Err(anyhow!("invalid cursor"));
        }
        let updated_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        Ok(Self {
            updated_at,
            warehouse_id: uuid::Uuid::parse_str(warehouse_id)
                .map_err(|_| anyhow!("invalid cursor"))?,
            sku_id: uuid::Uuid::parse_str(sku_id).map_err(|_| anyhow!("invalid cursor"))?,
        })
    }
}

pub struct InventoryListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub warehouse_ids: Option<Vec<uuid::Uuid>>,
    pub sku_id: Option<uuid::Uuid>,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub tag: Option<String>,
    pub on_hand_negative: bool,
    pub available_below_micros: Option<i128>,
    pub available_min: Option<i128>,
    pub available_max: Option<i128>,
    pub cursor: Option<InventoryCursor>,
    pub offset: Option<i64>,
    pub limit: i64,
}

pub struct InventoryListResult {
    pub items: Vec<InventoryRecord>,
    pub next_cursor: Option<InventoryCursor>,
}

pub struct InventorySummaryListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub warehouse_ids: Option<Vec<uuid::Uuid>>,
    pub sku_id: Option<uuid::Uuid>,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub tag: Option<String>,
    pub on_hand_negative: bool,
    pub available_below_micros: Option<i128>,
    pub available_min: Option<i128>,
    pub available_max: Option<i128>,
    pub offset: i64,
    pub limit: i64,
}

pub struct InventorySummaryListResult {
    pub items: Vec<InventorySummaryRecord>,
    pub total: i64,
}

pub struct InventoryCostListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub warehouse_ids: Option<Vec<uuid::Uuid>>,
    pub sku_id: Option<uuid::Uuid>,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub tag: Option<String>,
    pub on_hand_negative: bool,
    pub available_below_micros: Option<i128>,
    pub available_min: Option<i128>,
    pub available_max: Option<i128>,
    pub offset: i64,
    pub limit: i64,
}

pub struct InventoryCostListResult {
    pub items: Vec<InventoryCostRecord>,
    pub total: i64,
}

pub struct InventoryLedgerListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub warehouse_ids: Option<Vec<uuid::Uuid>>,
    pub sku_id: Option<uuid::Uuid>,
    pub sku_code: Option<String>,
    pub occurred_from: Option<DateTime<Utc>>,
    pub occurred_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct InventoryLedgerListResult {
    pub items: Vec<InventoryLedgerRecord>,
    pub total: i64,
}

pub struct InventoryAdjustment {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub batch_id: Option<uuid::Uuid>,
    pub qty_delta: i64,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InventoryReservationInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub order_id: Option<uuid::Uuid>,
    pub order_item_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct InventoryReservationAction {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub order_item_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct InventoryDelta {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub delta_on_hand: i64,
    pub delta_reserved: i64,
    pub delta_in_transit: i64,
    pub delta_frozen: i64,
    pub reason_type: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum InventoryError {
    #[error("insufficient available inventory (available: {available}, requested: {requested})")]
    InsufficientAvailable { available: i64, requested: i64 },
    #[error("insufficient frozen inventory (frozen: {frozen}, requested: {requested})")]
    InsufficientFrozen { frozen: i64, requested: i64 },
    #[error("reservation not found")]
    ReservationNotFound,
    #[error("reservation not active")]
    ReservationNotActive,
    #[error("reservation mismatch")]
    ReservationMismatch,
    #[error("original reservation source requires reconciliation")]
    ReservationSourceUnverified,
    #[error("reservation component count must be between 1 and 1000")]
    ReservationComponentsLimit,
}

pub use crate::inventory_batch_effects::InventoryBatchLimit;

pub struct InventoryAdjustmentEffect {
    pub stock: InventoryPreciseEffect,
    pub batch_effects: Vec<crate::inventory_batch_effects::InventoryBatchEffect>,
}

pub struct InventoryService<'a> {
    pub(crate) pool: &'a PgPool,
}

impl<'a> InventoryService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn apply(&self, input: InventoryDelta) -> anyhow::Result<InventoryBalance> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let balance = self.apply_with_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(balance)
    }

    pub async fn apply_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryDelta,
    ) -> anyhow::Result<InventoryBalance> {
        self.apply_effect_with_tx(tx, input)
            .await?
            .balance
            .whole_projection()
    }

    pub(crate) async fn apply_effect_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryDelta,
    ) -> anyhow::Result<InventoryPreciseEffect> {
        self.apply_precise_effect_with_tx(
            tx,
            InventoryPreciseDelta {
                key: InventoryQuantityKey {
                    tenant_id: input.tenant_id,
                    warehouse_id: input.warehouse_id,
                    sku_id: input.sku_id,
                },
                on_hand: ScaledQty::from_legacy_whole_units(input.delta_on_hand)?,
                reserved: ScaledQty::from_legacy_whole_units(input.delta_reserved)?,
                frozen: ScaledQty::from_legacy_whole_units(input.delta_frozen)?,
                in_transit: ScaledQty::from_legacy_whole_units(input.delta_in_transit)?,
                reason_type: input.reason_type,
                ref_type: input.ref_type,
                ref_id: input.ref_id,
                occurred_at: input.occurred_at,
            },
        )
        .await
    }
}

pub struct InventoryRepo<'a> {
    pub(crate) pool: &'a PgPool,
}

impl<'a> InventoryRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_balance(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<InventoryBalance>> {
        InventoryService::new(self.pool)
            .get_precise_balance(InventoryQuantityKey {
                tenant_id,
                warehouse_id,
                sku_id,
            })
            .await?
            .map(|balance| balance.whole_projection())
            .transpose()
    }

    pub async fn apply_adjustment(
        &self,
        input: InventoryAdjustment,
        reason_type: &str,
    ) -> anyhow::Result<InventoryBalance> {
        let mut tx = self.pool.begin().await?;
        let balance = self
            .apply_adjustment_with_tx(&mut tx, input, reason_type)
            .await?;
        tx.commit().await?;
        Ok(balance)
    }

    /// Leaves commit to the caller so the movement and required audit facts
    /// share one transaction. Batch and stock writes are included here.
    pub async fn apply_adjustment_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryAdjustment,
        reason_type: &str,
    ) -> anyhow::Result<InventoryBalance> {
        self.apply_adjustment_effect_with_tx(tx, input, reason_type)
            .await?
            .stock
            .balance
            .whole_projection()
    }

    pub(crate) async fn apply_adjustment_effect_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryAdjustment,
        reason_type: &str,
    ) -> anyhow::Result<InventoryAdjustmentEffect> {
        let InventoryAdjustment {
            tenant_id,
            warehouse_id,
            sku_id,
            batch_id,
            qty_delta,
            ref_type,
            ref_id,
        } = input;
        let service = InventoryService::new(self.pool);
        let stock = service
            .apply_effect_with_tx(
                tx,
                InventoryDelta {
                    tenant_id,
                    warehouse_id,
                    sku_id,
                    delta_on_hand: qty_delta,
                    delta_reserved: 0,
                    delta_in_transit: 0,
                    delta_frozen: 0,
                    reason_type: reason_type.to_string(),
                    ref_type,
                    ref_id,
                    occurred_at: Utc::now(),
                },
            )
            .await?;

        let batch_effects = crate::inventory_batch_effects::apply(tx, &stock, batch_id).await?;
        Ok(InventoryAdjustmentEffect {
            stock,
            batch_effects,
        })
    }

    pub async fn adjust(&self, input: InventoryAdjustment) -> anyhow::Result<InventoryBalance> {
        self.apply_adjustment(input, "STOCK_COUNT_ADJUST").await
    }

    pub async fn reserve_for_order_item(
        &self,
        input: InventoryReservationInput,
    ) -> anyhow::Result<InventoryBalance> {
        let mut tx = self.pool.begin().await?;
        let outcome = self.reserve_for_order_item_with_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(outcome.balance)
    }

    pub(crate) async fn reserve_for_order_item_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryReservationInput,
    ) -> anyhow::Result<InventoryReservationOutcome> {
        self.reserve_frozen_order_item_with_tx(tx, input, None)
            .await
    }

    pub(crate) async fn reserve_frozen_order_item_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryReservationInput,
        prepared: Option<ReservationExpansion>,
    ) -> anyhow::Result<InventoryReservationOutcome> {
        validate_reservation_input(
            input.tenant_id,
            input.warehouse_id,
            input.sku_id,
            input.order_item_id,
            input.qty,
        )?;
        if input.order_id.is_some_and(|id| id.is_nil()) {
            return Err(InventoryError::ReservationMismatch.into());
        }
        let service = InventoryService::new(self.pool);
        let frozen = self
            .resolve_prepared_order_reservation_items(tx, &input, true, prepared)
            .await?;
        let source_id = frozen.source_id;
        let items = frozen.items;
        self.lock_reservation_keys(
            tx,
            input.tenant_id,
            input.warehouse_id,
            input.sku_id,
            &items,
        )
        .await?;
        let mut applied = false;

        let mut last_balance: Option<InventoryBalance> = None;

        for item in items {
            let inserted: Option<uuid::Uuid> = sqlx::query_scalar(
                r#"
                insert into inventory_reservations(
                    tenant_id, warehouse_id, sku_id, order_id, order_item_id, qty, status,
                    reservation_source_id
                ) values ($1,$2,$3,$4,$5,$6,'RESERVED',$7)
                on conflict (tenant_id, order_item_id) do nothing
                returning reservation_id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.warehouse_id)
            .bind(item.sku_id)
            .bind(input.order_id)
            .bind(item.order_item_id)
            .bind(item.qty)
            .bind(source_id)
            .fetch_optional(&mut **tx)
            .await?;

            #[derive(sqlx::FromRow)]
            struct ReservationRow {
                warehouse_id: uuid::Uuid,
                sku_id: uuid::Uuid,
                order_id: Option<uuid::Uuid>,
                qty: i64,
                status: String,
                reservation_source_id: Option<uuid::Uuid>,
            }

            if inserted.is_none() {
                let reservation: ReservationRow = sqlx::query_as(
                    r#"
                    select warehouse_id, sku_id, order_id, qty, status, reservation_source_id
                    from inventory_reservations
                    where tenant_id = $1 and order_item_id = $2
                    for update
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(item.order_item_id)
                .fetch_one(&mut **tx)
                .await?;

                if reservation.status != "RESERVED" {
                    return Err(InventoryError::ReservationNotActive.into());
                }
                if reservation.warehouse_id != input.warehouse_id
                    || reservation.sku_id != item.sku_id
                    || reservation.order_id != input.order_id
                    || reservation.qty != item.qty
                    || reservation.reservation_source_id != source_id
                {
                    return Err(InventoryError::ReservationMismatch.into());
                }
            }

            #[derive(sqlx::FromRow)]
            struct BalanceRow {
                on_hand: i64,
                reserved: i64,
                frozen: i64,
            }

            let balance: Option<BalanceRow> = sqlx::query_as(
                r#"
                select on_hand, reserved, frozen
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.warehouse_id)
            .bind(item.sku_id)
            .fetch_optional(&mut **tx)
            .await?;

            let Some(balance) = balance else {
                return Err(InventoryError::InsufficientAvailable {
                    available: 0,
                    requested: item.qty,
                }
                .into());
            };

            if inserted.is_none() {
                last_balance = Some(
                    service
                        .get_precise_balance_with_tx(
                            tx,
                            InventoryQuantityKey {
                                tenant_id: input.tenant_id,
                                warehouse_id: input.warehouse_id,
                                sku_id: item.sku_id,
                            },
                        )
                        .await?
                        .ok_or(InventoryError::ReservationNotFound)?
                        .whole_projection()?,
                );
                continue;
            }

            let available = balance.on_hand - balance.reserved - balance.frozen;
            if available < item.qty {
                return Err(InventoryError::InsufficientAvailable {
                    available,
                    requested: item.qty,
                }
                .into());
            }

            let updated = service
                .apply_with_tx(
                    tx,
                    InventoryDelta {
                        tenant_id: input.tenant_id,
                        warehouse_id: input.warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: 0,
                        delta_reserved: item.qty,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: "RESERVATION_CREATE".to_string(),
                        ref_type: Some("ORDER_ITEM".to_string()),
                        ref_id: Some(item.order_item_id.to_string()),
                        occurred_at: Utc::now(),
                    },
                )
                .await?;

            applied = true;
            last_balance = Some(updated);
        }

        Ok(InventoryReservationOutcome {
            balance: last_balance.ok_or_else(|| anyhow!("missing inventory balance"))?,
            applied,
        })
    }

    pub async fn release_reservation(
        &self,
        input: InventoryReservationAction,
    ) -> anyhow::Result<InventoryBalance> {
        self.update_reservation(input, "RELEASED", "RELEASE", false)
            .await
    }

    pub async fn consume_reservation(
        &self,
        input: InventoryReservationAction,
    ) -> anyhow::Result<InventoryBalance> {
        self.update_reservation(input, "CONSUMED", "SHIP", true)
            .await
    }

    async fn update_reservation(
        &self,
        input: InventoryReservationAction,
        new_status: &str,
        ledger_kind: &str,
        consume: bool,
    ) -> anyhow::Result<InventoryBalance> {
        let mut tx = self.pool.begin().await?;
        let outcome = self
            .update_reservation_with_tx(&mut tx, input, new_status, ledger_kind, consume)
            .await?;
        tx.commit().await?;
        Ok(outcome.balance)
    }

    pub(crate) async fn update_reservation_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: InventoryReservationAction,
        new_status: &str,
        ledger_kind: &str,
        consume: bool,
    ) -> anyhow::Result<InventoryReservationOutcome> {
        validate_reservation_input(
            input.tenant_id,
            input.warehouse_id,
            input.sku_id,
            input.order_item_id,
            input.qty,
        )?;
        let service = InventoryService::new(self.pool);

        #[derive(sqlx::FromRow)]
        struct ReservationRow {
            warehouse_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            qty: i64,
            status: String,
            reservation_source_id: Option<uuid::Uuid>,
        }
        let frozen = self
            .resolve_order_reservation_items(
                tx,
                &InventoryReservationInput {
                    tenant_id: input.tenant_id,
                    warehouse_id: input.warehouse_id,
                    sku_id: input.sku_id,
                    order_item_id: input.order_item_id,
                    order_id: None,
                    qty: input.qty,
                },
                false,
            )
            .await?;
        let source_id = frozen.source_id;
        let items = frozen.items;
        self.lock_reservation_keys(
            tx,
            input.tenant_id,
            input.warehouse_id,
            input.sku_id,
            &items,
        )
        .await?;
        let mut applied = false;

        let mut last_balance: Option<InventoryBalance> = None;

        #[derive(sqlx::FromRow)]
        struct BalanceRow {
            on_hand: i64,
            reserved: i64,
        }

        for item in items {
            let reservation: ReservationRow = sqlx::query_as(
                r#"
                select warehouse_id, sku_id, qty, status, reservation_source_id
                from inventory_reservations
                where tenant_id = $1 and order_item_id = $2
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(item.order_item_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(InventoryError::ReservationNotFound)?;

            // A terminal replay still cites the original physical identity and
            // quantity. Validate before returning any current balance.
            if reservation.warehouse_id != input.warehouse_id
                || reservation.sku_id != item.sku_id
                || reservation.qty != item.qty
                || reservation.reservation_source_id != source_id
            {
                return Err(InventoryError::ReservationMismatch.into());
            }
            if reservation.status == new_status {
                last_balance = Some(
                    service
                        .get_precise_balance_with_tx(
                            tx,
                            InventoryQuantityKey {
                                tenant_id: input.tenant_id,
                                warehouse_id: input.warehouse_id,
                                sku_id: item.sku_id,
                            },
                        )
                        .await?
                        .ok_or(InventoryError::ReservationNotFound)?
                        .whole_projection()?,
                );
                continue;
            }

            if reservation.status != "RESERVED" {
                return Err(InventoryError::ReservationNotActive.into());
            }

            sqlx::query(
                r#"
                update inventory_reservations
                set status = $3, updated_at = now()
                where tenant_id = $1 and order_item_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(item.order_item_id)
            .bind(new_status)
            .execute(&mut **tx)
            .await?;

            let consume_qty = if consume { item.qty } else { 0 };

            let balance: BalanceRow = sqlx::query_as(
                r#"
                select on_hand, reserved
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.warehouse_id)
            .bind(item.sku_id)
            .fetch_one(&mut **tx)
            .await?;

            if balance.reserved < item.qty || balance.on_hand < consume_qty {
                return Err(InventoryError::ReservationMismatch.into());
            }

            let reason_type = match ledger_kind {
                "RELEASE" => "RESERVATION_RELEASE",
                "SHIP" => "SALES_SHIP",
                other => other,
            };

            let effect = service
                .apply_effect_with_tx(
                    tx,
                    InventoryDelta {
                        tenant_id: input.tenant_id,
                        warehouse_id: input.warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: -consume_qty,
                        delta_reserved: -item.qty,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: reason_type.to_string(),
                        ref_type: Some("ORDER_ITEM".to_string()),
                        ref_id: Some(item.order_item_id.to_string()),
                        occurred_at: Utc::now(),
                    },
                )
                .await?;

            if consume_qty > 0 {
                self.consume_batches_by_fefo(tx, &effect).await?;
            }

            applied = true;
            last_balance = Some(effect.balance.whole_projection()?);
        }

        Ok(InventoryReservationOutcome {
            balance: last_balance.ok_or_else(|| anyhow!("missing inventory balance"))?,
            applied,
        })
    }

    pub(crate) async fn consume_batches_by_fefo(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        effect: &InventoryPreciseEffect,
    ) -> anyhow::Result<()> {
        crate::inventory_batch_consumption::consume(tx, effect).await
    }

    async fn lock_reservation_keys(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
        warehouse: uuid::Uuid,
        requested_sku: uuid::Uuid,
        items: &[ReservationItem],
    ) -> anyhow::Result<()> {
        let mut skus = vec![requested_sku];
        for item in items {
            validate_reservation_input(
                tenant,
                warehouse,
                item.sku_id,
                item.order_item_id,
                item.qty,
            )?;
            skus.push(item.sku_id);
        }
        skus.sort_unstable();
        skus.dedup();
        for sku in skus {
            crate::inventory_precision::require_owned_key(
                tx,
                InventoryQuantityKey {
                    tenant_id: tenant,
                    warehouse_id: warehouse,
                    sku_id: sku,
                },
            )
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn expand_order_item(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        order_item_id: uuid::Uuid,
        qty: i64,
        order_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<ReservationExpansion> {
        #[derive(sqlx::FromRow)]
        struct ComponentRow {
            order_id: uuid::Uuid,
            component_sku_id: uuid::Uuid,
            component_qty: i64,
            bom_version: Option<i32>,
        }

        let component_rows: Vec<ComponentRow> = sqlx::query_as(
            r#"
            select order_id, component_sku_id, component_qty, bom_version
            from order_item_components
            where tenant_id = $1
              and order_item_id = $2
              and bundle_sku_id = $3
            order by component_sku_id limit 1001
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_item_id)
        .bind(sku_id)
        .fetch_all(&mut **tx)
        .await?;

        if !component_rows.is_empty() {
            if component_rows
                .iter()
                .any(|row| order_id.is_some_and(|id| id != row.order_id))
            {
                return Err(InventoryError::ReservationMismatch.into());
            }
            let items = component_rows
                .into_iter()
                .map(|row| ReservationItem {
                    sku_id: row.component_sku_id,
                    order_item_id: component_order_item_id(order_item_id, row.component_sku_id),
                    qty: row.component_qty,
                    bom_version: row.bom_version,
                })
                .collect();
            return Ok(ReservationExpansion {
                definition_kind: "ORDER_COMPONENTS",
                items,
            });
        }

        let sku_kind: Option<String> = sqlx::query_scalar(
            r#"
            select sku_kind
            from skus
            where tenant_id = $1 and sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_optional(&mut **tx)
        .await?;

        let Some(sku_kind) = sku_kind else {
            return Err(anyhow!("sku not found"));
        };

        if sku_kind.eq_ignore_ascii_case("KIT") {
            #[derive(sqlx::FromRow)]
            struct BomRow {
                component_sku_id: uuid::Uuid,
                qty: i64,
                bom_version: i32,
            }

            let rows: Vec<BomRow> = sqlx::query_as(
                r#"
                select component_sku_id, qty, bom_version
                from sku_boms
                where tenant_id = $1 and bundle_sku_id = $2
                order by component_sku_id limit 1001
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(sku_id)
            .fetch_all(&mut **tx)
            .await?;

            if rows.is_empty() {
                return Err(anyhow!("kit components not found"));
            }

            let items = rows
                .into_iter()
                .map(|row| {
                    let total_qty = qty
                        .checked_mul(row.qty)
                        .ok_or_else(|| anyhow!("qty overflow"))?;
                    Ok(ReservationItem {
                        sku_id: row.component_sku_id,
                        order_item_id: component_order_item_id(order_item_id, row.component_sku_id),
                        qty: total_qty,
                        bom_version: Some(row.bom_version),
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(ReservationExpansion {
                definition_kind: "SKU_BOM",
                items,
            })
        } else {
            Ok(ReservationExpansion {
                definition_kind: "DIRECT",
                items: vec![ReservationItem {
                    sku_id,
                    order_item_id,
                    qty,
                    bom_version: None,
                }],
            })
        }
    }
}

pub(crate) struct InventoryReservationOutcome {
    pub(crate) balance: InventoryBalance,
    pub(crate) applied: bool,
}

pub(crate) fn validate_reservation_input(
    tenant: TenantId,
    warehouse: uuid::Uuid,
    sku: uuid::Uuid,
    source: uuid::Uuid,
    qty: i64,
) -> anyhow::Result<()> {
    if tenant.0.is_nil() || warehouse.is_nil() || sku.is_nil() || source.is_nil() || qty <= 0 {
        return Err(InventoryError::ReservationMismatch.into());
    }
    ScaledQty::from_legacy_whole_units(qty)?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct ReservationItem {
    pub(crate) sku_id: uuid::Uuid,
    pub(crate) order_item_id: uuid::Uuid,
    pub(crate) qty: i64,
    pub(crate) bom_version: Option<i32>,
}

pub(crate) struct ReservationExpansion {
    pub(crate) definition_kind: &'static str,
    pub(crate) items: Vec<ReservationItem>,
}

pub(crate) fn component_order_item_id(
    order_item_id: uuid::Uuid,
    component_sku_id: uuid::Uuid,
) -> uuid::Uuid {
    uuid::Uuid::new_v5(&order_item_id, component_sku_id.as_bytes())
}
