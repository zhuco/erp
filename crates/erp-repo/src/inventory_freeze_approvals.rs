use chrono::{DateTime, Utc};
use erp_types::{TenantId, UserId};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone)]
pub struct InventoryFreezeApproval {
    pub approval_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub action: String,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub reason: Option<String>,
    pub freeze_id: Option<uuid::Uuid>,
    pub status: String,
    pub requested_by: Option<UserId>,
    pub approved_by: Option<UserId>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approval_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InventoryFreezeApprovalCreate {
    pub tenant_id: TenantId,
    pub action: String,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub reason: Option<String>,
    pub freeze_id: Option<uuid::Uuid>,
    pub requested_by: Option<UserId>,
}

#[derive(Debug, Clone)]
pub struct InventoryFreezeApprovalListFilters {
    pub action: Option<String>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub sku_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct InventoryFreezeApprovalListResult {
    pub items: Vec<InventoryFreezeApproval>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct InventoryFreezeApprovalUpdate {
    pub status: String,
    pub freeze_id: Option<uuid::Uuid>,
    pub approved_by: Option<UserId>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approval_note: Option<String>,
}

pub struct InventoryFreezeApprovalsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InventoryFreezeApprovalsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_approvals(
        &self,
        tenant_id: TenantId,
        filters: InventoryFreezeApprovalListFilters,
    ) -> anyhow::Result<InventoryFreezeApprovalListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from inventory_freeze_approvals where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                approval_id, action, warehouse_id, sku_id, qty, reason,
                freeze_id, status, requested_by, approved_by, approved_at,
                approval_note, created_at, updated_at
            from inventory_freeze_approvals
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, approval_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        let rows: Vec<InventoryFreezeApprovalRow> =
            qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_approval(tenant_id))
            .collect();

        Ok(InventoryFreezeApprovalListResult { items, total })
    }

    pub async fn get_approval(
        &self,
        tenant_id: TenantId,
        approval_id: uuid::Uuid,
    ) -> anyhow::Result<Option<InventoryFreezeApproval>> {
        let row = sqlx::query_as::<_, InventoryFreezeApprovalRow>(
            r#"
            select
                approval_id, action, warehouse_id, sku_id, qty, reason,
                freeze_id, status, requested_by, approved_by, approved_at,
                approval_note, created_at, updated_at
            from inventory_freeze_approvals
            where tenant_id = $1 and approval_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(approval_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_approval(tenant_id)))
    }

    pub async fn create_approval(
        &self,
        input: InventoryFreezeApprovalCreate,
    ) -> anyhow::Result<InventoryFreezeApproval> {
        let row = sqlx::query_as::<_, InventoryFreezeApprovalRow>(
            r#"
            insert into inventory_freeze_approvals(
                tenant_id, action, warehouse_id, sku_id, qty, reason,
                freeze_id, status, requested_by
            ) values ($1,$2,$3,$4,$5,$6,$7,'PENDING',$8)
            returning
                approval_id, action, warehouse_id, sku_id, qty, reason,
                freeze_id, status, requested_by, approved_by, approved_at,
                approval_note, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.action)
        .bind(input.warehouse_id)
        .bind(input.sku_id)
        .bind(input.qty)
        .bind(&input.reason)
        .bind(input.freeze_id)
        .bind(input.requested_by.map(uuid::Uuid::from))
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_approval(input.tenant_id))
    }

    pub async fn update_approval(
        &self,
        tenant_id: TenantId,
        approval_id: uuid::Uuid,
        input: InventoryFreezeApprovalUpdate,
    ) -> anyhow::Result<Option<InventoryFreezeApproval>> {
        let mut qb = QueryBuilder::<Postgres>::new("update inventory_freeze_approvals set ");
        let mut separated = qb.separated(", ");
        separated.push("status = ");
        separated.push_bind(input.status);

        separated.push("freeze_id = ");
        separated.push_bind(input.freeze_id);

        separated.push("approved_by = ");
        separated.push_bind(input.approved_by.map(uuid::Uuid::from));

        separated.push("approved_at = ");
        separated.push_bind(input.approved_at);

        separated.push("approval_note = ");
        separated.push_bind(input.approval_note);

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and approval_id = ");
        qb.push_bind(approval_id);
        qb.push(
            " returning approval_id, action, warehouse_id, sku_id, qty, reason, freeze_id, status, requested_by, approved_by, approved_at, approval_note, created_at, updated_at",
        );

        let row: Option<InventoryFreezeApprovalRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_approval(tenant_id)))
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a InventoryFreezeApprovalListFilters,
) {
    if let Some(action) = &filters.action {
        qb.push(" and action = ");
        qb.push_bind(action);
    }
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
struct InventoryFreezeApprovalRow {
    approval_id: uuid::Uuid,
    action: String,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    reason: Option<String>,
    freeze_id: Option<uuid::Uuid>,
    status: String,
    requested_by: Option<uuid::Uuid>,
    approved_by: Option<uuid::Uuid>,
    approved_at: Option<DateTime<Utc>>,
    approval_note: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl InventoryFreezeApprovalRow {
    fn into_approval(self, tenant_id: TenantId) -> InventoryFreezeApproval {
        InventoryFreezeApproval {
            approval_id: self.approval_id,
            tenant_id,
            action: self.action,
            warehouse_id: self.warehouse_id,
            sku_id: self.sku_id,
            qty: self.qty,
            reason: self.reason,
            freeze_id: self.freeze_id,
            status: self.status,
            requested_by: self.requested_by.map(UserId),
            approved_by: self.approved_by.map(UserId),
            approved_at: self.approved_at,
            approval_note: self.approval_note,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
