use anyhow::{anyhow, bail};
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use crate::inventory::{InventoryDelta, InventoryError, InventoryService};

#[derive(Debug, Clone)]
pub struct StockAdjustDoc {
    pub adjust_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub reason: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockAdjustItem {
    pub adjust_item_id: uuid::Uuid,
    pub adjust_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockAdjustDetail {
    pub doc: StockAdjustDoc,
    pub items: Vec<StockAdjustItem>,
}

#[derive(Debug, Clone)]
pub struct StockAdjustItemInput {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct StockAdjustCreateInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub reason: String,
    pub items: Vec<StockAdjustItemInput>,
}

#[derive(Debug, Clone)]
pub struct StockAdjustListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub reason: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct StockAdjustListResult {
    pub items: Vec<StockAdjustDoc>,
    pub total: i64,
}

pub struct StockAdjustmentsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> StockAdjustmentsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_adjustment(
        &self,
        input: StockAdjustCreateInput,
    ) -> anyhow::Result<StockAdjustDoc> {
        if input.items.is_empty() {
            bail!("adjust items required");
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockAdjustDocRow = sqlx::query_as(
            r#"
            insert into stock_adjust_docs(
                tenant_id, warehouse_id, reason, status
            ) values ($1,$2,$3,'DRAFT')
            returning adjust_id, tenant_id, warehouse_id, reason, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(&input.reason)
        .fetch_one(&mut *tx)
        .await?;

        for item in input.items {
            if item.qty <= 0 {
                bail!("qty must be positive");
            }
            sqlx::query(
                r#"
                insert into stock_adjust_items(
                    tenant_id, adjust_id, sku_id, qty
                ) values ($1,$2,$3,$4)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(row.adjust_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(row.into_doc())
    }

    pub async fn list_adjustments(
        &self,
        tenant_id: TenantId,
        filters: StockAdjustListFilters,
    ) -> anyhow::Result<StockAdjustListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from stock_adjust_docs
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_adjust_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select adjust_id, tenant_id, warehouse_id, reason, status, created_at, updated_at
            from stock_adjust_docs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_adjust_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, adjust_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<StockAdjustDocRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows.into_iter().map(StockAdjustDocRow::into_doc).collect();

        Ok(StockAdjustListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        adjust_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockAdjustDetail>> {
        let row: Option<StockAdjustDocRow> = sqlx::query_as(
            r#"
            select adjust_id, tenant_id, warehouse_id, reason, status, created_at, updated_at
            from stock_adjust_docs
            where tenant_id = $1 and adjust_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(adjust_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let items = self.list_items(tenant_id, adjust_id).await?;
        Ok(Some(StockAdjustDetail {
            doc: row.into_doc(),
            items,
        }))
    }

    pub async fn list_items(
        &self,
        tenant_id: TenantId,
        adjust_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<StockAdjustItem>> {
        let rows: Vec<StockAdjustItemRow> = sqlx::query_as(
            r#"
            select adjust_item_id, adjust_id, sku_id, qty, created_at, updated_at
            from stock_adjust_items
            where tenant_id = $1 and adjust_id = $2
            order by adjust_item_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(adjust_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(StockAdjustItemRow::into_item)
            .collect())
    }

    pub async fn apply_adjustment(
        &self,
        tenant_id: TenantId,
        adjust_id: uuid::Uuid,
    ) -> anyhow::Result<StockAdjustDoc> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockAdjustDocRow = sqlx::query_as(
            r#"
            select adjust_id, tenant_id, warehouse_id, reason, status, created_at, updated_at
            from stock_adjust_docs
            where tenant_id = $1 and adjust_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(adjust_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(anyhow!("adjust doc not found"))?;

        if row.status == "APPLIED" {
            return Ok(row.into_doc());
        }
        if row.status == "CANCELLED" {
            return Err(anyhow!("adjust doc cancelled"));
        }

        let items = self.list_items(tenant_id, adjust_id).await?;
        if items.is_empty() {
            return Err(anyhow!("adjust items missing"));
        }

        let direction = match row.reason.as_str() {
            "SCRAP" => -1,
            "GAIN" => 1,
            _ => return Err(anyhow!("invalid adjust reason")),
        };

        let service = InventoryService::new(self.pool);
        let occurred_at = Utc::now();
        for item in &items {
            let delta_qty = direction * item.qty;
            if direction < 0 {
                let balance: BalanceRow = sqlx::query_as(
                    r#"
                    select on_hand, reserved, frozen
                    from inventory_balances
                    where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                    for update
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(row.warehouse_id)
                .bind(item.sku_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(InventoryError::InsufficientAvailable {
                    available: 0,
                    requested: item.qty,
                })?;

                let available = balance.on_hand - balance.reserved - balance.frozen;
                if available < item.qty {
                    return Err(InventoryError::InsufficientAvailable {
                        available,
                        requested: item.qty,
                    }
                    .into());
                }
            }

            service
                .apply_with_tx(
                    &mut tx,
                    InventoryDelta {
                        tenant_id,
                        warehouse_id: row.warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: delta_qty,
                        delta_reserved: 0,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: format!("ADJUST_{}", row.reason),
                        ref_type: Some("STOCK_ADJUST".to_string()),
                        ref_id: Some(adjust_id.to_string()),
                        occurred_at,
                    },
                )
                .await?;
        }

        let updated: StockAdjustDocRow = sqlx::query_as(
            r#"
            update stock_adjust_docs
            set status = 'APPLIED',
                updated_at = now()
            where tenant_id = $1 and adjust_id = $2
            returning adjust_id, tenant_id, warehouse_id, reason, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(adjust_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated.into_doc())
    }

    pub async fn cancel_adjustment(
        &self,
        tenant_id: TenantId,
        adjust_id: uuid::Uuid,
    ) -> anyhow::Result<StockAdjustDoc> {
        let row: StockAdjustDocRow = sqlx::query_as(
            r#"
            update stock_adjust_docs
            set status = 'CANCELLED',
                updated_at = now()
            where tenant_id = $1 and adjust_id = $2
            returning adjust_id, tenant_id, warehouse_id, reason, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(adjust_id)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_doc())
    }
}

fn apply_adjust_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a StockAdjustListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
    if let Some(reason) = &filters.reason {
        qb.push(" and reason = ");
        qb.push_bind(reason);
    }
}

#[derive(sqlx::FromRow)]
struct StockAdjustDocRow {
    adjust_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    reason: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockAdjustDocRow {
    fn into_doc(self) -> StockAdjustDoc {
        StockAdjustDoc {
            adjust_id: self.adjust_id,
            tenant_id: TenantId(self.tenant_id),
            warehouse_id: self.warehouse_id,
            reason: self.reason,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StockAdjustItemRow {
    adjust_item_id: uuid::Uuid,
    adjust_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockAdjustItemRow {
    fn into_item(self) -> StockAdjustItem {
        StockAdjustItem {
            adjust_item_id: self.adjust_item_id,
            adjust_id: self.adjust_id,
            sku_id: self.sku_id,
            qty: self.qty,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct BalanceRow {
    on_hand: i64,
    reserved: i64,
    frozen: i64,
}
