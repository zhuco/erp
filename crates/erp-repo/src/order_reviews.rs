use chrono::{DateTime, Utc};
use erp_types::{OrderId, TenantId, UserId};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReview {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub status: String,
    pub reviewed_by: Option<UserId>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub notes: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrderReviewUpsert {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub status: String,
    pub reviewed_by: Option<UserId>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub notes: String,
}

pub struct OrderReviewsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> OrderReviewsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_review(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Option<OrderReview>> {
        let row: Option<OrderReviewRow> = sqlx::query_as(
            r#"
            select tenant_id, order_id, status, reviewed_by, reviewed_at, notes, updated_at
            from order_reviews
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(OrderReviewRow::into_review))
    }

    pub async fn upsert_review(&self, input: OrderReviewUpsert) -> anyhow::Result<OrderReview> {
        let row: OrderReviewRow = sqlx::query_as(
            r#"
            insert into order_reviews(
                tenant_id,
                order_id,
                status,
                reviewed_by,
                reviewed_at,
                notes
            ) values ($1, $2, $3, $4, $5, $6)
            on conflict (tenant_id, order_id)
            do update set
                status = excluded.status,
                reviewed_by = excluded.reviewed_by,
                reviewed_at = excluded.reviewed_at,
                notes = excluded.notes,
                updated_at = now()
            returning tenant_id, order_id, status, reviewed_by, reviewed_at, notes, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.order_id))
        .bind(&input.status)
        .bind(input.reviewed_by.map(uuid::Uuid::from))
        .bind(input.reviewed_at)
        .bind(&input.notes)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_review())
    }
}

#[derive(sqlx::FromRow)]
struct OrderReviewRow {
    tenant_id: uuid::Uuid,
    order_id: uuid::Uuid,
    status: String,
    reviewed_by: Option<uuid::Uuid>,
    reviewed_at: Option<DateTime<Utc>>,
    notes: String,
    updated_at: DateTime<Utc>,
}

impl OrderReviewRow {
    fn into_review(self) -> OrderReview {
        OrderReview {
            tenant_id: TenantId(self.tenant_id),
            order_id: OrderId(self.order_id),
            status: self.status,
            reviewed_by: self.reviewed_by.map(UserId),
            reviewed_at: self.reviewed_at,
            notes: self.notes,
            updated_at: self.updated_at,
        }
    }
}
