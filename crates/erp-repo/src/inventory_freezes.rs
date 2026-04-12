use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use crate::inventory::{InventoryDelta, InventoryService};

#[derive(Debug, Clone)]
pub struct InventoryFreeze {
    pub freeze_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub reason: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InventoryFreezeCreateInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InventoryFreezeListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub sku_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct InventoryFreezeListResult {
    pub items: Vec<InventoryFreeze>,
    pub total: i64,
}

pub struct InventoryFreezesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InventoryFreezesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_freeze(
        &self,
        input: InventoryFreezeCreateInput,
    ) -> anyhow::Result<InventoryFreeze> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: InventoryFreezeRow = sqlx::query_as(
            r#"
            insert into inventory_freezes(
                tenant_id, warehouse_id, sku_id, qty, reason, status
            ) values ($1,$2,$3,$4,$5,'ACTIVE')
            returning
                freeze_id, tenant_id, warehouse_id, sku_id, qty, reason, status,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(input.sku_id)
        .bind(input.qty)
        .bind(input.reason.clone())
        .fetch_one(&mut *tx)
        .await?;

        InventoryService::new(self.pool)
            .apply_with_tx(
                &mut tx,
                InventoryDelta {
                    tenant_id: input.tenant_id,
                    warehouse_id: input.warehouse_id,
                    sku_id: input.sku_id,
                    delta_on_hand: 0,
                    delta_reserved: 0,
                    delta_in_transit: 0,
                    delta_frozen: input.qty,
                    reason_type: "INVENTORY_FREEZE".to_string(),
                    ref_type: Some("INVENTORY_FREEZE".to_string()),
                    ref_id: Some(row.freeze_id.to_string()),
                    occurred_at: Utc::now(),
                },
            )
            .await?;

        tx.commit().await?;
        Ok(row.into_freeze())
    }

    pub async fn list_freezes(
        &self,
        tenant_id: TenantId,
        filters: InventoryFreezeListFilters,
    ) -> anyhow::Result<InventoryFreezeListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from inventory_freezes
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_freeze_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                freeze_id, tenant_id, warehouse_id, sku_id, qty, reason, status,
                created_at, updated_at
            from inventory_freezes
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_freeze_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, freeze_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<InventoryFreezeRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(InventoryFreezeRow::into_freeze)
            .collect();

        Ok(InventoryFreezeListResult { items, total })
    }

    pub async fn get_freeze(
        &self,
        tenant_id: TenantId,
        freeze_id: uuid::Uuid,
    ) -> anyhow::Result<Option<InventoryFreeze>> {
        let row: Option<InventoryFreezeRow> = sqlx::query_as(
            r#"
            select
                freeze_id, tenant_id, warehouse_id, sku_id, qty, reason, status,
                created_at, updated_at
            from inventory_freezes
            where tenant_id = $1 and freeze_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(freeze_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(InventoryFreezeRow::into_freeze))
    }

    pub async fn cancel_freeze(
        &self,
        tenant_id: TenantId,
        freeze_id: uuid::Uuid,
    ) -> anyhow::Result<InventoryFreeze> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: InventoryFreezeRow = sqlx::query_as(
            r#"
            select
                freeze_id, tenant_id, warehouse_id, sku_id, qty, reason, status,
                created_at, updated_at
            from inventory_freezes
            where tenant_id = $1 and freeze_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(freeze_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("freeze not found"))?;

        if row.status == "CANCELLED" {
            return Ok(row.into_freeze());
        }

        InventoryService::new(self.pool)
            .apply_with_tx(
                &mut tx,
                InventoryDelta {
                    tenant_id,
                    warehouse_id: row.warehouse_id,
                    sku_id: row.sku_id,
                    delta_on_hand: 0,
                    delta_reserved: 0,
                    delta_in_transit: 0,
                    delta_frozen: -row.qty,
                    reason_type: "INVENTORY_UNFREEZE".to_string(),
                    ref_type: Some("INVENTORY_FREEZE".to_string()),
                    ref_id: Some(freeze_id.to_string()),
                    occurred_at: Utc::now(),
                },
            )
            .await?;

        let updated: InventoryFreezeRow = sqlx::query_as(
            r#"
            update inventory_freezes
            set status = 'CANCELLED',
                updated_at = now()
            where tenant_id = $1 and freeze_id = $2
            returning
                freeze_id, tenant_id, warehouse_id, sku_id, qty, reason, status,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(freeze_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated.into_freeze())
    }
}

fn apply_freeze_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a InventoryFreezeListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
}

#[derive(sqlx::FromRow)]
struct InventoryFreezeRow {
    freeze_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    reason: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl InventoryFreezeRow {
    fn into_freeze(self) -> InventoryFreeze {
        InventoryFreeze {
            freeze_id: self.freeze_id,
            tenant_id: TenantId(self.tenant_id),
            warehouse_id: self.warehouse_id,
            sku_id: self.sku_id,
            qty: self.qty,
            reason: self.reason,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
