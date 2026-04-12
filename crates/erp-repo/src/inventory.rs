use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use thiserror::Error;

use crate::outbox::OutboxRepo;

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
}

impl InventoryBalance {
    pub fn available(&self) -> i64 {
        self.on_hand - self.reserved - self.frozen
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
}

impl InventoryRecord {
    pub fn available(&self) -> i64 {
        self.on_hand - self.reserved - self.frozen
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
}

impl InventorySummaryRecord {
    pub fn available(&self) -> i64 {
        self.on_hand - self.reserved - self.frozen
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
    pub unit_cost_cent: i64,
    pub total_cost_cent: i64,
}

impl InventoryCostRecord {
    pub fn available(&self) -> i64 {
        self.on_hand - self.reserved - self.frozen
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
    pub available_below: Option<i64>,
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
    pub available_below: Option<i64>,
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
    pub available_below: Option<i64>,
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
}

pub struct InventoryService<'a> {
    pool: &'a PgPool,
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
        #[derive(sqlx::FromRow)]
        struct BalanceRow {
            on_hand: i64,
            reserved: i64,
            frozen: i64,
        }

        let InventoryDelta {
            tenant_id,
            warehouse_id,
            sku_id,
            delta_on_hand,
            delta_reserved,
            delta_in_transit,
            delta_frozen,
            reason_type,
            ref_type,
            ref_id,
            occurred_at,
        } = input;

        let before_row: Option<BalanceRow> = sqlx::query_as(
            r#"
            select on_hand, reserved, frozen
            from inventory_balances
            where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .fetch_optional(&mut **tx)
        .await?;
        let before_available = before_row
            .as_ref()
            .map(|row| row.on_hand - row.reserved - row.frozen)
            .unwrap_or(0);
        let before_frozen = before_row.as_ref().map(|row| row.frozen).unwrap_or(0);
        let next_frozen = before_frozen + delta_frozen;
        if next_frozen < 0 {
            return Err(InventoryError::InsufficientFrozen {
                frozen: before_frozen,
                requested: delta_frozen.abs(),
            }
            .into());
        }

        #[derive(sqlx::FromRow)]
        struct UpdatedRow {
            on_hand: i64,
            reserved: i64,
            in_transit: i64,
            frozen: i64,
            updated_at: DateTime<Utc>,
        }

        let updated: UpdatedRow = sqlx::query_as(
            r#"
            insert into inventory_balances(
                tenant_id, warehouse_id, sku_id, on_hand, reserved, in_transit, frozen
            ) values ($1,$2,$3,$4,$5,$6,$7)
            on conflict (tenant_id, warehouse_id, sku_id)
            do update set
                on_hand = inventory_balances.on_hand + excluded.on_hand,
                reserved = inventory_balances.reserved + excluded.reserved,
                in_transit = inventory_balances.in_transit + excluded.in_transit,
                frozen = inventory_balances.frozen + excluded.frozen,
                updated_at = now()
            returning on_hand, reserved, in_transit, frozen, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .bind(delta_on_hand)
        .bind(delta_reserved)
        .bind(delta_in_transit)
        .bind(delta_frozen)
        .fetch_one(&mut **tx)
        .await?;

        sqlx::query(
            r#"
            insert into inventory_ledgers(
                tenant_id, warehouse_id, sku_id,
                delta_on_hand, delta_reserved, delta_in_transit, delta_frozen,
                reason_type, ref_type, ref_id, occurred_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .bind(delta_on_hand)
        .bind(delta_reserved)
        .bind(delta_in_transit)
        .bind(delta_frozen)
        .bind(reason_type)
        .bind(ref_type.clone())
        .bind(ref_id.clone())
        .bind(occurred_at)
        .execute(&mut **tx)
        .await?;

        let after_available = updated.on_hand - updated.reserved - updated.frozen;
        if after_available != before_available {
            let should_sync: bool = sqlx::query_scalar(
                r#"
                select exists(
                    select 1
                    from channel_sku_mappings
                    where tenant_id = $1 and sku_id = $2 and status = 'ACTIVE'
                )
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(sku_id)
            .fetch_one(&mut **tx)
            .await?;
            if should_sync {
                let payload = json!({
                    "tenant_id": tenant_id,
                    "sku_id": sku_id,
                    "warehouse_id": warehouse_id,
                    "on_hand": updated.on_hand,
                    "reserved": updated.reserved,
                    "frozen": updated.frozen,
                    "in_transit": updated.in_transit,
                    "available": after_available,
                    "occurred_at": occurred_at,
                });
                OutboxRepo::enqueue_with_tx(
                    &mut **tx,
                    tenant_id,
                    INVENTORY_CHANGED_EVENT,
                    &payload,
                    occurred_at,
                )
                .await?;
            }
        }

        Ok(InventoryBalance {
            tenant_id,
            warehouse_id,
            sku_id,
            on_hand: updated.on_hand,
            reserved: updated.reserved,
            frozen: updated.frozen,
            in_transit: updated.in_transit,
            updated_at: updated.updated_at,
        })
    }
}

pub struct InventoryRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InventoryRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_inventory(
        &self,
        tenant_id: TenantId,
        filters: InventoryListFilters,
    ) -> anyhow::Result<InventoryListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                ib.warehouse_id,
                ib.sku_id,
                s.sku_code,
                s.barcode,
                case
                    when s.sku_kind = 'KIT' then (
                        select coalesce(sum(b.qty * coalesce(c.cost_cent, 0)), 0)
                        from sku_boms b
                        join skus c
                          on c.sku_id = b.component_sku_id
                         and c.tenant_id = s.tenant_id
                        where b.tenant_id = s.tenant_id
                          and b.bundle_sku_id = s.sku_id
                    )
                    else s.cost_cent
                end as cost_cent,
                ib.on_hand,
                ib.reserved,
                ib.frozen,
                ib.in_transit,
                ib.updated_at
            from inventory_balances ib
            left join skus s on s.sku_id = ib.sku_id
            where ib.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(warehouse_id) = filters.warehouse_id {
            qb.push(" and ib.warehouse_id = ");
            qb.push_bind(warehouse_id);
        }
        if let Some(warehouse_ids) = filters.warehouse_ids.as_ref() {
            if !warehouse_ids.is_empty() {
                qb.push(" and ib.warehouse_id = any(");
                qb.push_bind(warehouse_ids);
                qb.push(")");
            }
        }
        if let Some(sku_id) = filters.sku_id {
            qb.push(" and ib.sku_id = ");
            qb.push_bind(sku_id);
        }
        if let Some(sku_code) = filters.sku_code {
            qb.push(" and s.sku_code = ");
            qb.push_bind(sku_code);
        }
        if let Some(barcode) = filters.barcode {
            qb.push(" and s.barcode = ");
            qb.push_bind(barcode);
        }
        if let Some(tag) = filters.tag {
            qb.push(" and exists (select 1 from unnest(s.tags) tag where tag ilike ");
            qb.push_bind(format!("%{}%", tag));
            qb.push(")");
        }
        if filters.on_hand_negative {
            qb.push(" and ib.on_hand < 0");
        }
        if let Some(threshold) = filters.available_below {
            qb.push(" and (ib.on_hand - ib.reserved - ib.frozen) < ");
            qb.push_bind(threshold);
        }
        if let Some(cursor) = filters.cursor.as_ref() {
            qb.push(" and (ib.updated_at, ib.warehouse_id, ib.sku_id) < (");
            qb.push_bind(cursor.updated_at);
            qb.push(", ");
            qb.push_bind(cursor.warehouse_id);
            qb.push(", ");
            qb.push_bind(cursor.sku_id);
            qb.push(")");
        }

        qb.push(" order by ib.updated_at desc, ib.warehouse_id desc, ib.sku_id desc");
        if filters.cursor.is_none() {
            if let Some(offset) = filters.offset {
                qb.push(" offset ");
                qb.push_bind(offset.max(0));
            }
        }
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            barcode: Option<String>,
            cost_cent: Option<i64>,
            on_hand: i64,
            reserved: i64,
            frozen: i64,
            in_transit: i64,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| InventoryCursor {
            updated_at: row.updated_at,
            warehouse_id: row.warehouse_id,
            sku_id: row.sku_id,
        });

        let items = rows
            .into_iter()
            .map(|row| InventoryRecord {
                warehouse_id: row.warehouse_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                barcode: row.barcode,
                cost_cent: row.cost_cent,
                on_hand: row.on_hand,
                reserved: row.reserved,
                frozen: row.frozen,
                in_transit: row.in_transit,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(InventoryListResult { items, next_cursor })
    }

    pub async fn list_inventory_summary(
        &self,
        tenant_id: TenantId,
        filters: InventorySummaryListFilters,
    ) -> anyhow::Result<InventorySummaryListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new("");
        append_inventory_summary_cte(&mut count_qb, tenant_id, &filters);
        count_qb.push(" select count(*) from summary where 1=1");
        apply_inventory_summary_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new("");
        append_inventory_summary_cte(&mut qb, tenant_id, &filters);
        qb.push(
            r#"
            select
                sku_id,
                sku_code,
                barcode,
                on_hand,
                reserved,
                frozen,
                in_transit,
                updated_at
            from summary
            where 1=1
            "#,
        );
        apply_inventory_summary_filters(&mut qb, &filters);
        qb.push(" order by updated_at desc, sku_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            barcode: Option<String>,
            on_hand: i64,
            reserved: i64,
            frozen: i64,
            in_transit: i64,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| InventorySummaryRecord {
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                barcode: row.barcode,
                on_hand: row.on_hand,
                reserved: row.reserved,
                frozen: row.frozen,
                in_transit: row.in_transit,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(InventorySummaryListResult { items, total })
    }

    pub async fn list_inventory_costs(
        &self,
        tenant_id: TenantId,
        filters: InventoryCostListFilters,
    ) -> anyhow::Result<InventoryCostListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new("");
        append_inventory_cost_cte(&mut count_qb, tenant_id, &filters);
        count_qb.push(" select count(*) from summary where 1=1");
        apply_inventory_cost_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new("");
        append_inventory_cost_cte(&mut qb, tenant_id, &filters);
        qb.push(
            r#"
            select
                sku_id,
                sku_code,
                barcode,
                on_hand,
                reserved,
                frozen,
                in_transit,
                updated_at,
                coalesce(unit_cost_cent, 0) as unit_cost_cent,
                coalesce(unit_cost_cent, 0) * on_hand as total_cost_cent
            from summary
            where 1=1
            "#,
        );
        apply_inventory_cost_filters(&mut qb, &filters);
        qb.push(" order by updated_at desc, sku_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            barcode: Option<String>,
            on_hand: i64,
            reserved: i64,
            frozen: i64,
            in_transit: i64,
            updated_at: DateTime<Utc>,
            unit_cost_cent: i64,
            total_cost_cent: i64,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| InventoryCostRecord {
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                barcode: row.barcode,
                on_hand: row.on_hand,
                reserved: row.reserved,
                frozen: row.frozen,
                in_transit: row.in_transit,
                updated_at: row.updated_at,
                unit_cost_cent: row.unit_cost_cent,
                total_cost_cent: row.total_cost_cent,
            })
            .collect();

        Ok(InventoryCostListResult { items, total })
    }

    pub async fn list_ledgers(
        &self,
        tenant_id: TenantId,
        filters: InventoryLedgerListFilters,
    ) -> anyhow::Result<InventoryLedgerListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from inventory_ledgers l
            left join skus s on s.sku_id = l.sku_id
            where l.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_ledger_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                l.ledger_id,
                l.warehouse_id,
                l.sku_id,
                s.sku_code,
                l.delta_on_hand,
                l.delta_reserved,
                l.delta_in_transit,
                l.delta_frozen,
                l.reason_type,
                l.ref_type,
                l.ref_id,
                l.occurred_at
            from inventory_ledgers l
            left join skus s on s.sku_id = l.sku_id
            where l.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_ledger_filters(&mut qb, &filters);
        qb.push(" order by l.occurred_at desc, l.ledger_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            ledger_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            delta_on_hand: i64,
            delta_reserved: i64,
            delta_in_transit: i64,
            delta_frozen: i64,
            reason_type: String,
            ref_type: Option<String>,
            ref_id: Option<String>,
            occurred_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| InventoryLedgerRecord {
                ledger_id: row.ledger_id,
                warehouse_id: row.warehouse_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                delta_on_hand: row.delta_on_hand,
                delta_reserved: row.delta_reserved,
                delta_in_transit: row.delta_in_transit,
                delta_frozen: row.delta_frozen,
                reason_type: row.reason_type,
                ref_type: row.ref_type,
                ref_id: row.ref_id,
                occurred_at: row.occurred_at,
            })
            .collect();

        Ok(InventoryLedgerListResult { items, total })
    }

    pub async fn get_balance(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<InventoryBalance>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            on_hand: i64,
            reserved: i64,
            in_transit: i64,
            frozen: i64,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select on_hand, reserved, in_transit, frozen, updated_at
            from inventory_balances
            where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| InventoryBalance {
            tenant_id,
            warehouse_id,
            sku_id,
            on_hand: row.on_hand,
            reserved: row.reserved,
            frozen: row.frozen,
            in_transit: row.in_transit,
            updated_at: row.updated_at,
        }))
    }

    pub async fn apply_adjustment(
        &self,
        input: InventoryAdjustment,
        reason_type: &str,
    ) -> anyhow::Result<InventoryBalance> {
        let InventoryAdjustment {
            tenant_id,
            warehouse_id,
            sku_id,
            batch_id,
            qty_delta,
            ref_type,
            ref_id,
        } = input;
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let service = InventoryService::new(self.pool);
        let balance = service
            .apply_with_tx(
                &mut tx,
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

        if let Some(batch_id) = batch_id {
            sqlx::query(
                r#"
                insert into inventory_batch_balances(
                    tenant_id, warehouse_id, sku_id, batch_id, on_hand, reserved
                ) values ($1,$2,$3,$4,$5,0)
                on conflict (tenant_id, warehouse_id, sku_id, batch_id)
                do update set
                    on_hand = inventory_batch_balances.on_hand + $5,
                    updated_at = now()
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .bind(sku_id)
            .bind(batch_id)
            .bind(qty_delta)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(balance)
    }

    pub async fn adjust(&self, input: InventoryAdjustment) -> anyhow::Result<InventoryBalance> {
        self.apply_adjustment(input, "STOCK_COUNT_ADJUST").await
    }

    pub async fn reserve_for_order_item(
        &self,
        input: InventoryReservationInput,
    ) -> anyhow::Result<InventoryBalance> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let service = InventoryService::new(self.pool);
        let items = self
            .expand_order_item(
                &mut tx,
                input.tenant_id,
                input.sku_id,
                input.order_item_id,
                input.qty,
            )
            .await?;

        let mut last_balance: Option<InventoryBalance> = None;

        for item in items {
            let inserted: Option<uuid::Uuid> = sqlx::query_scalar(
                r#"
                insert into inventory_reservations(
                    tenant_id, warehouse_id, sku_id, order_id, order_item_id, qty, status
                ) values ($1,$2,$3,$4,$5,$6,'RESERVED')
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
            .fetch_optional(&mut *tx)
            .await?;

            #[derive(sqlx::FromRow)]
            struct ReservationRow {
                warehouse_id: uuid::Uuid,
                sku_id: uuid::Uuid,
                qty: i64,
                status: String,
            }

            if inserted.is_none() {
                let reservation: ReservationRow = sqlx::query_as(
                    r#"
                    select warehouse_id, sku_id, qty, status
                    from inventory_reservations
                    where tenant_id = $1 and order_item_id = $2
                    for update
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(item.order_item_id)
                .fetch_one(&mut *tx)
                .await?;

                if reservation.status != "RESERVED" {
                    return Err(InventoryError::ReservationNotActive.into());
                }
                if reservation.warehouse_id != input.warehouse_id
                    || reservation.sku_id != item.sku_id
                    || reservation.qty != item.qty
                {
                    return Err(InventoryError::ReservationMismatch.into());
                }
            }

            #[derive(sqlx::FromRow)]
            struct BalanceRow {
                on_hand: i64,
                reserved: i64,
                in_transit: i64,
                frozen: i64,
                updated_at: DateTime<Utc>,
            }

            let balance: Option<BalanceRow> = sqlx::query_as(
                r#"
                select on_hand, reserved, in_transit, frozen, updated_at
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.warehouse_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(balance) = balance else {
                return Err(InventoryError::InsufficientAvailable {
                    available: 0,
                    requested: item.qty,
                }
                .into());
            };

            if inserted.is_none() {
                last_balance = Some(InventoryBalance {
                    tenant_id: input.tenant_id,
                    warehouse_id: input.warehouse_id,
                    sku_id: item.sku_id,
                    on_hand: balance.on_hand,
                    reserved: balance.reserved,
                    frozen: balance.frozen,
                    in_transit: balance.in_transit,
                    updated_at: balance.updated_at,
                });
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
                    &mut tx,
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

            last_balance = Some(updated);
        }

        tx.commit().await?;
        last_balance.ok_or_else(|| anyhow!("missing inventory balance"))
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
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let service = InventoryService::new(self.pool);

        #[derive(sqlx::FromRow)]
        struct ReservationRow {
            warehouse_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            qty: i64,
            status: String,
        }
        let items = self
            .expand_order_item(
                &mut tx,
                input.tenant_id,
                input.sku_id,
                input.order_item_id,
                input.qty,
            )
            .await?;

        let mut last_balance: Option<InventoryBalance> = None;

        #[derive(sqlx::FromRow)]
        struct BalanceRow {
            on_hand: i64,
            reserved: i64,
            in_transit: i64,
            frozen: i64,
            updated_at: DateTime<Utc>,
        }

        for item in items {
            let reservation: ReservationRow = sqlx::query_as(
                r#"
                select warehouse_id, sku_id, qty, status
                from inventory_reservations
                where tenant_id = $1 and order_item_id = $2
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(item.order_item_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(InventoryError::ReservationNotFound)?;

            if reservation.status == new_status {
                let row: BalanceRow = sqlx::query_as(
                    r#"
                    select on_hand, reserved, in_transit, frozen, updated_at
                    from inventory_balances
                    where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(input.warehouse_id)
                .bind(item.sku_id)
                .fetch_one(&mut *tx)
                .await?;

                last_balance = Some(InventoryBalance {
                    tenant_id: input.tenant_id,
                    warehouse_id: input.warehouse_id,
                    sku_id: item.sku_id,
                    on_hand: row.on_hand,
                    reserved: row.reserved,
                    frozen: row.frozen,
                    in_transit: row.in_transit,
                    updated_at: row.updated_at,
                });
                continue;
            }

            if reservation.status != "RESERVED" {
                return Err(InventoryError::ReservationNotActive.into());
            }
            if reservation.warehouse_id != input.warehouse_id
                || reservation.sku_id != item.sku_id
                || reservation.qty != item.qty
            {
                return Err(InventoryError::ReservationMismatch.into());
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
            .execute(&mut *tx)
            .await?;

            let consume_qty = if consume { item.qty } else { 0 };

            let balance: BalanceRow = sqlx::query_as(
                r#"
                select on_hand, reserved, in_transit, frozen, updated_at
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.warehouse_id)
            .bind(item.sku_id)
            .fetch_one(&mut *tx)
            .await?;

            if balance.reserved < item.qty || balance.on_hand < consume_qty {
                return Err(InventoryError::ReservationMismatch.into());
            }

            let reason_type = match ledger_kind {
                "RELEASE" => "RESERVATION_RELEASE",
                "SHIP" => "SALES_SHIP",
                other => other,
            };

            let updated = service
                .apply_with_tx(
                    &mut tx,
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
                self.consume_batches_by_fefo(
                    &mut tx,
                    input.tenant_id,
                    input.warehouse_id,
                    item.sku_id,
                    consume_qty,
                )
                .await?;
            }

            last_balance = Some(updated);
        }

        tx.commit().await?;
        last_balance.ok_or_else(|| anyhow!("missing inventory balance"))
    }

    async fn consume_batches_by_fefo(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        qty: i64,
    ) -> anyhow::Result<()> {
        if qty <= 0 {
            return Ok(());
        }

        let batch_enabled: Option<bool> = sqlx::query_scalar(
            r#"
            select batch_enabled
            from skus
            where tenant_id = $1 and sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_optional(&mut **tx)
        .await?;

        if !batch_enabled.unwrap_or(false) {
            return Ok(());
        }

        #[derive(sqlx::FromRow)]
        struct BatchRow {
            batch_id: uuid::Uuid,
            on_hand: i64,
            reserved: i64,
        }

        let rows: Vec<BatchRow> = sqlx::query_as(
            r#"
            select ibb.batch_id, ibb.on_hand, ibb.reserved
            from inventory_batch_balances ibb
            join sku_batches b on b.batch_id = ibb.batch_id
            where ibb.tenant_id = $1 and ibb.warehouse_id = $2 and ibb.sku_id = $3
            order by b.expire_at asc nulls last, ibb.batch_id
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(sku_id)
        .fetch_all(&mut **tx)
        .await?;

        let mut remaining = qty;
        for row in rows {
            if remaining <= 0 {
                break;
            }
            let available = row.on_hand - row.reserved;
            if available <= 0 {
                continue;
            }
            let consume_qty = available.min(remaining);
            sqlx::query(
                r#"
                update inventory_batch_balances
                set on_hand = on_hand - $5,
                    updated_at = now()
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3 and batch_id = $4
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .bind(sku_id)
            .bind(row.batch_id)
            .bind(consume_qty)
            .execute(&mut **tx)
            .await?;
            remaining -= consume_qty;
        }

        if remaining > 0 {
            return Err(InventoryError::InsufficientAvailable {
                available: qty - remaining,
                requested: qty,
            }
            .into());
        }

        Ok(())
    }

    async fn expand_order_item(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        order_item_id: uuid::Uuid,
        qty: i64,
    ) -> anyhow::Result<Vec<ReservationItem>> {
        #[derive(sqlx::FromRow)]
        struct ComponentRow {
            component_sku_id: uuid::Uuid,
            component_qty: i64,
        }

        let component_rows: Vec<ComponentRow> = sqlx::query_as(
            r#"
            select component_sku_id, component_qty
            from order_item_components
            where tenant_id = $1
              and order_item_id = $2
              and bundle_sku_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_item_id)
        .bind(sku_id)
        .fetch_all(&mut **tx)
        .await?;

        if !component_rows.is_empty() {
            return component_rows
                .into_iter()
                .map(|row| {
                    Ok(ReservationItem {
                        sku_id: row.component_sku_id,
                        order_item_id: component_order_item_id(order_item_id, row.component_sku_id),
                        qty: row.component_qty,
                    })
                })
                .collect();
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
            }

            let rows: Vec<BomRow> = sqlx::query_as(
                r#"
                select component_sku_id, qty
                from sku_boms
                where tenant_id = $1 and bundle_sku_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(sku_id)
            .fetch_all(&mut **tx)
            .await?;

            if rows.is_empty() {
                return Err(anyhow!("kit components not found"));
            }

            rows.into_iter()
                .map(|row| {
                    let total_qty = qty
                        .checked_mul(row.qty)
                        .ok_or_else(|| anyhow!("qty overflow"))?;
                    Ok(ReservationItem {
                        sku_id: row.component_sku_id,
                        order_item_id: component_order_item_id(order_item_id, row.component_sku_id),
                        qty: total_qty,
                    })
                })
                .collect()
        } else {
            Ok(vec![ReservationItem {
                sku_id,
                order_item_id,
                qty,
            }])
        }
    }
}

fn apply_ledger_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a InventoryLedgerListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and l.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(warehouse_ids) = &filters.warehouse_ids {
        if !warehouse_ids.is_empty() {
            qb.push(" and l.warehouse_id = any(");
            qb.push_bind(warehouse_ids);
            qb.push(")");
        }
    }
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and l.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(sku_code) = &filters.sku_code {
        qb.push(" and s.sku_code = ");
        qb.push_bind(sku_code);
    }
    if let Some(from) = filters.occurred_from {
        qb.push(" and l.occurred_at >= ");
        qb.push_bind(from);
    }
    if let Some(to) = filters.occurred_to {
        qb.push(" and l.occurred_at <= ");
        qb.push_bind(to);
    }
}

fn append_inventory_summary_cte<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    tenant_id: TenantId,
    filters: &'a InventorySummaryListFilters,
) {
    qb.push(
        r#"
        with summary as (
            select
                ib.sku_id,
                s.sku_code,
                s.barcode,
                sum(ib.on_hand)::bigint as on_hand,
                sum(ib.reserved)::bigint as reserved,
                sum(ib.frozen)::bigint as frozen,
                sum(ib.in_transit)::bigint as in_transit,
                max(ib.updated_at) as updated_at
            from inventory_balances ib
            left join skus s on s.sku_id = ib.sku_id
            where ib.tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(tenant_id));

    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and ib.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(warehouse_ids) = filters.warehouse_ids.as_ref() {
        if !warehouse_ids.is_empty() {
            qb.push(" and ib.warehouse_id = any(");
            qb.push_bind(warehouse_ids);
            qb.push(")");
        }
    }
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and ib.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(sku_code) = filters.sku_code.as_ref() {
        qb.push(" and s.sku_code = ");
        qb.push_bind(sku_code);
    }
    if let Some(barcode) = filters.barcode.as_ref() {
        qb.push(" and s.barcode = ");
        qb.push_bind(barcode);
    }
    if let Some(tag) = filters.tag.as_ref() {
        qb.push(" and exists (select 1 from unnest(s.tags) tag where tag ilike ");
        qb.push_bind(format!("%{}%", tag));
        qb.push(")");
    }

    qb.push(" group by ib.sku_id, s.sku_code, s.barcode");
    qb.push(")");
}

fn apply_inventory_summary_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a InventorySummaryListFilters,
) {
    if filters.on_hand_negative {
        qb.push(" and on_hand < 0");
    }
    if let Some(threshold) = filters.available_below {
        qb.push(" and (on_hand - reserved - frozen) < ");
        qb.push_bind(threshold);
    }
}

fn append_inventory_cost_cte<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    tenant_id: TenantId,
    filters: &'a InventoryCostListFilters,
) {
    qb.push(
        r#"
        with summary as (
            select
                ib.sku_id,
                s.sku_code,
                s.barcode,
                sum(ib.on_hand)::bigint as on_hand,
                sum(ib.reserved)::bigint as reserved,
                sum(ib.frozen)::bigint as frozen,
                sum(ib.in_transit)::bigint as in_transit,
                max(ib.updated_at) as updated_at,
                case
                    when s.sku_kind = 'KIT' then (
                        select coalesce(sum(b.qty * coalesce(c.cost_cent, 0)), 0)
                        from sku_boms b
                        join skus c
                            on c.sku_id = b.component_sku_id
                            and c.tenant_id = s.tenant_id
                        where b.tenant_id = s.tenant_id
                          and b.bundle_sku_id = s.sku_id
                    )
                    else s.cost_cent
                end as unit_cost_cent
            from inventory_balances ib
            left join skus s on s.sku_id = ib.sku_id
            where ib.tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(tenant_id));

    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and ib.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(warehouse_ids) = filters.warehouse_ids.as_ref() {
        if !warehouse_ids.is_empty() {
            qb.push(" and ib.warehouse_id = any(");
            qb.push_bind(warehouse_ids);
            qb.push(")");
        }
    }
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and ib.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(sku_code) = filters.sku_code.as_ref() {
        qb.push(" and s.sku_code = ");
        qb.push_bind(sku_code);
    }
    if let Some(barcode) = filters.barcode.as_ref() {
        qb.push(" and s.barcode = ");
        qb.push_bind(barcode);
    }
    if let Some(tag) = filters.tag.as_ref() {
        qb.push(" and exists (select 1 from unnest(s.tags) tag where tag ilike ");
        qb.push_bind(format!("%{}%", tag));
        qb.push(")");
    }

    qb.push(" group by ib.sku_id, s.sku_code, s.barcode, s.sku_kind, s.cost_cent, s.tenant_id");
    qb.push(")");
}

fn apply_inventory_cost_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a InventoryCostListFilters,
) {
    if filters.on_hand_negative {
        qb.push(" and on_hand < 0");
    }
    if let Some(threshold) = filters.available_below {
        qb.push(" and (on_hand - reserved - frozen) < ");
        qb.push_bind(threshold);
    }
}

#[derive(Debug, Clone)]
struct ReservationItem {
    sku_id: uuid::Uuid,
    order_item_id: uuid::Uuid,
    qty: i64,
}

fn component_order_item_id(order_item_id: uuid::Uuid, component_sku_id: uuid::Uuid) -> uuid::Uuid {
    uuid::Uuid::new_v5(&order_item_id, component_sku_id.as_bytes())
}
