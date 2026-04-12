use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{OrderId, TenantId};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone)]
pub struct AfterSalesIntercept {
    pub intercept_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub after_sales_id: Option<uuid::Uuid>,
    pub carrier_name: Option<String>,
    pub tracking_no: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AfterSalesInterceptListFilters {
    pub status: Option<String>,
    pub order_id: Option<OrderId>,
    pub tracking_no: Option<String>,
    pub requested_from: Option<DateTime<Utc>>,
    pub requested_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct AfterSalesInterceptListResult {
    pub items: Vec<AfterSalesIntercept>,
    pub total: i64,
}

pub struct AfterSalesInterceptCreate {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub after_sales_id: Option<uuid::Uuid>,
    pub carrier_name: Option<String>,
    pub tracking_no: Option<String>,
    pub status: String,
    pub reason: Option<String>,
    pub requested_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct AfterSalesInterceptUpdate {
    pub status: Option<String>,
    pub reason: Option<String>,
    pub carrier_name: Option<String>,
    pub tracking_no: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

pub struct AfterSalesInterceptsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AfterSalesInterceptsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_intercepts(
        &self,
        tenant_id: TenantId,
        filters: AfterSalesInterceptListFilters,
    ) -> anyhow::Result<AfterSalesInterceptListResult> {
        let tracking_like = filters
            .tracking_no
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                intercept_id, order_id, after_sales_id, carrier_name, tracking_no,
                status, reason, requested_at, completed_at, created_at, updated_at
            from after_sales_intercepts
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status.as_ref() {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(order_id) = filters.order_id {
            qb.push(" and order_id = ");
            qb.push_bind(uuid::Uuid::from(order_id));
        }
        if let Some(tracking_like) = tracking_like.as_ref() {
            qb.push(" and tracking_no ilike ");
            qb.push_bind(tracking_like);
        }
        if let Some(from) = filters.requested_from {
            qb.push(" and requested_at >= ");
            qb.push_bind(from);
        }
        if let Some(to) = filters.requested_to {
            qb.push(" and requested_at <= ");
            qb.push_bind(to);
        }
        qb.push(" order by requested_at desc, created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<AfterSalesInterceptRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_intercept(tenant_id))
            .collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from after_sales_intercepts where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status.as_ref() {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        if let Some(order_id) = filters.order_id {
            count_qb.push(" and order_id = ");
            count_qb.push_bind(uuid::Uuid::from(order_id));
        }
        if let Some(tracking_like) = tracking_like.as_ref() {
            count_qb.push(" and tracking_no ilike ");
            count_qb.push_bind(tracking_like);
        }
        if let Some(from) = filters.requested_from {
            count_qb.push(" and requested_at >= ");
            count_qb.push_bind(from);
        }
        if let Some(to) = filters.requested_to {
            count_qb.push(" and requested_at <= ");
            count_qb.push_bind(to);
        }

        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;
        Ok(AfterSalesInterceptListResult { items, total })
    }

    pub async fn create_intercept(
        &self,
        input: AfterSalesInterceptCreate,
    ) -> anyhow::Result<AfterSalesIntercept> {
        let requested_at = input.requested_at.unwrap_or_else(Utc::now);
        let row: AfterSalesInterceptRow = sqlx::query_as(
            r#"
            insert into after_sales_intercepts(
                tenant_id, order_id, after_sales_id, carrier_name, tracking_no,
                status, reason, requested_at, completed_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            returning
                intercept_id, order_id, after_sales_id, carrier_name, tracking_no,
                status, reason, requested_at, completed_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.order_id))
        .bind(input.after_sales_id)
        .bind(input.carrier_name)
        .bind(input.tracking_no)
        .bind(input.status)
        .bind(input.reason)
        .bind(requested_at)
        .bind(input.completed_at)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_intercept(input.tenant_id))
    }

    pub async fn update_intercept(
        &self,
        tenant_id: TenantId,
        intercept_id: uuid::Uuid,
        update: AfterSalesInterceptUpdate,
    ) -> anyhow::Result<Option<AfterSalesIntercept>> {
        let mut qb = QueryBuilder::<Postgres>::new("update after_sales_intercepts set ");
        let mut separated = qb.separated(", ");
        let mut has_update = false;

        if let Some(status) = update.status {
            separated.push("status = ");
            separated.push_bind(status);
            has_update = true;
        }
        if let Some(reason) = update.reason {
            separated.push("reason = ");
            separated.push_bind(reason);
            has_update = true;
        }
        if let Some(carrier_name) = update.carrier_name {
            separated.push("carrier_name = ");
            separated.push_bind(carrier_name);
            has_update = true;
        }
        if let Some(tracking_no) = update.tracking_no {
            separated.push("tracking_no = ");
            separated.push_bind(tracking_no);
            has_update = true;
        }
        if let Some(completed_at) = update.completed_at {
            separated.push("completed_at = ");
            separated.push_bind(completed_at);
            has_update = true;
        }

        if !has_update {
            return Err(anyhow!("no fields to update"));
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and intercept_id = ");
        qb.push_bind(intercept_id);
        qb.push(
            " returning intercept_id, order_id, after_sales_id, carrier_name, tracking_no,
              status, reason, requested_at, completed_at, created_at, updated_at",
        );

        let row: Option<AfterSalesInterceptRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_intercept(tenant_id)))
    }
}

#[derive(sqlx::FromRow)]
struct AfterSalesInterceptRow {
    intercept_id: uuid::Uuid,
    order_id: uuid::Uuid,
    after_sales_id: Option<uuid::Uuid>,
    carrier_name: Option<String>,
    tracking_no: Option<String>,
    status: String,
    reason: Option<String>,
    requested_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AfterSalesInterceptRow {
    fn into_intercept(self, tenant_id: TenantId) -> AfterSalesIntercept {
        AfterSalesIntercept {
            intercept_id: self.intercept_id,
            tenant_id,
            order_id: OrderId(self.order_id),
            after_sales_id: self.after_sales_id,
            carrier_name: self.carrier_name,
            tracking_no: self.tracking_no,
            status: self.status,
            reason: self.reason,
            requested_at: self.requested_at,
            completed_at: self.completed_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
