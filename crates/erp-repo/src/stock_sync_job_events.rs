use chrono::{DateTime, Utc};
use erp_types::{ShopId, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSyncJobEvent {
    pub event_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub shop_id: ShopId,
    pub sku_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub status: String,
    pub attempt: i32,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

pub struct StockSyncJobEventCreate {
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub shop_id: ShopId,
    pub sku_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub status: String,
    pub attempt: i32,
    pub payload: Value,
}

pub struct StockSyncJobEventsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> StockSyncJobEventsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_event(
        &self,
        input: StockSyncJobEventCreate,
    ) -> anyhow::Result<StockSyncJobEvent> {
        #[derive(sqlx::FromRow)]
        struct Row {
            event_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            job_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            status: String,
            attempt: i32,
            payload: Value,
            created_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into stock_sync_job_events(
                tenant_id, job_id, shop_id, sku_id, warehouse_id, status, attempt, payload
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                event_id, tenant_id, job_id, shop_id, sku_id, warehouse_id,
                status, attempt, payload, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.job_id)
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(input.sku_id)
        .bind(input.warehouse_id)
        .bind(input.status)
        .bind(input.attempt)
        .bind(input.payload)
        .fetch_one(self.pool)
        .await?;

        Ok(StockSyncJobEvent {
            event_id: row.event_id,
            tenant_id: TenantId(row.tenant_id),
            job_id: row.job_id,
            shop_id: ShopId(row.shop_id),
            sku_id: row.sku_id,
            warehouse_id: row.warehouse_id,
            status: row.status,
            attempt: row.attempt,
            payload: row.payload,
            created_at: row.created_at,
        })
    }

    pub async fn list_events(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        status: Option<String>,
        offset: i64,
        limit: i64,
    ) -> anyhow::Result<Vec<StockSyncJobEvent>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            event_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            job_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            status: String,
            attempt: i32,
            payload: Value,
            created_at: DateTime<Utc>,
        }

        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            r#"
            select
                event_id, tenant_id, job_id, shop_id, sku_id, warehouse_id,
                status, attempt, payload, created_at
            from stock_sync_job_events
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and job_id = ");
        qb.push_bind(job_id);
        if let Some(status) = status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        qb.push(" order by created_at desc, event_id desc");
        qb.push(" offset ");
        qb.push_bind(offset);
        qb.push(" limit ");
        qb.push_bind(limit);

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| StockSyncJobEvent {
                event_id: row.event_id,
                tenant_id: TenantId(row.tenant_id),
                job_id: row.job_id,
                shop_id: ShopId(row.shop_id),
                sku_id: row.sku_id,
                warehouse_id: row.warehouse_id,
                status: row.status,
                attempt: row.attempt,
                payload: row.payload,
                created_at: row.created_at,
            })
            .collect())
    }
}
