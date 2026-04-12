use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct WorkerEffect {
    pub tenant_id: TenantId,
    pub outbox_id: uuid::Uuid,
    pub event_type: String,
    pub status: String,
    pub last_error: String,
    pub processed_at: DateTime<Utc>,
}

pub struct WorkerEffectsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WorkerEffectsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn claim_processing(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
        event_type: &str,
    ) -> anyhow::Result<bool> {
        let row: Option<i64> = sqlx::query_scalar(
            r#"
            insert into worker_effects (
              tenant_id,
              outbox_id,
              event_type,
              status,
              last_error,
              processed_at
            )
            values ($1, $2, $3, 'PROCESSING', '', now())
            on conflict (tenant_id, outbox_id)
            do update set
              status = 'PROCESSING',
              last_error = '',
              processed_at = now(),
              event_type = excluded.event_type
            where worker_effects.status = 'FAILED'
            returning 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .bind(event_type)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.is_some())
    }

    pub async fn mark_succeeded(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update worker_effects
            set status = 'SUCCEEDED',
                last_error = '',
                processed_at = now()
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
        error: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update worker_effects
            set status = 'FAILED',
                last_error = $3,
                processed_at = now()
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .bind(error)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn ensure_effect(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
    ) -> anyhow::Result<WorkerEffect> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tenant_id: uuid::Uuid,
            outbox_id: uuid::Uuid,
            event_type: String,
            status: String,
            last_error: String,
            processed_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select tenant_id, outbox_id, event_type, status, last_error, processed_at
            from worker_effects
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Err(anyhow!("worker effect not found"));
        };

        Ok(WorkerEffect {
            tenant_id: TenantId(row.tenant_id),
            outbox_id: row.outbox_id,
            event_type: row.event_type,
            status: row.status,
            last_error: row.last_error,
            processed_at: row.processed_at,
        })
    }
}
