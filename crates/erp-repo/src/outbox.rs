use anyhow::anyhow;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::time::Duration;

fn month_start(dt: DateTime<Utc>) -> NaiveDate {
    let d = dt.date_naive();
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
}

#[derive(Debug, Clone)]
pub struct OutboxRow {
    pub tenant_id: TenantId,
    pub outbox_id: uuid::Uuid,
    pub event_type: String,
    pub payload: Value,
    pub attempts: i32,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEvent {
    pub tenant_id: TenantId,
    pub outbox_id: uuid::Uuid,
    pub event_type: String,
    pub status: String,
    pub attempts: i32,
    pub dedup_key: Option<String>,
    pub last_error: String,
    pub next_retry_at: DateTime<Utc>,
    pub last_attempt_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OutboxCursor {
    pub created_at: DateTime<Utc>,
    pub outbox_id: uuid::Uuid,
}

impl OutboxCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.outbox_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let outbox_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            outbox_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OutboxListFilters {
    pub status: Option<String>,
    pub event_type: Option<String>,
    pub cursor: Option<OutboxCursor>,
    pub limit: i64,
}

pub struct OutboxListResult {
    pub items: Vec<OutboxEvent>,
    pub next_cursor: Option<OutboxCursor>,
}

pub struct OutboxRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> OutboxRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn enqueue(
        &self,
        tenant_id: TenantId,
        event_type: &str,
        payload: &Value,
    ) -> anyhow::Result<uuid::Uuid> {
        let now = Utc::now();
        Self::enqueue_with_time(self.pool, tenant_id, event_type, payload, None, None, now).await
    }

    pub async fn enqueue_with_trace(
        &self,
        tenant_id: TenantId,
        event_type: &str,
        payload: &Value,
        trace_id: Option<&str>,
    ) -> anyhow::Result<uuid::Uuid> {
        let now = Utc::now();
        Self::enqueue_with_time(
            self.pool, tenant_id, event_type, payload, trace_id, None, now,
        )
        .await
    }

    pub async fn enqueue_dedup(
        &self,
        tenant_id: TenantId,
        event_type: &str,
        payload: &Value,
        dedup_key: &str,
    ) -> anyhow::Result<uuid::Uuid> {
        let now = Utc::now();
        Self::enqueue_with_time(
            self.pool,
            tenant_id,
            event_type,
            payload,
            None,
            Some(dedup_key),
            now,
        )
        .await
    }

    pub async fn enqueue_dedup_with_trace(
        &self,
        tenant_id: TenantId,
        event_type: &str,
        payload: &Value,
        dedup_key: &str,
        trace_id: Option<&str>,
    ) -> anyhow::Result<uuid::Uuid> {
        let now = Utc::now();
        Self::enqueue_with_time(
            self.pool,
            tenant_id,
            event_type,
            payload,
            trace_id,
            Some(dedup_key),
            now,
        )
        .await
    }

    pub async fn enqueue_with_tx<'t, T>(
        executor: T,
        tenant_id: TenantId,
        event_type: &str,
        payload: &Value,
        occurred_at: DateTime<Utc>,
    ) -> anyhow::Result<uuid::Uuid>
    where
        T: sqlx::Executor<'t, Database = Postgres>,
    {
        Self::enqueue_with_time(
            executor,
            tenant_id,
            event_type,
            payload,
            None,
            None,
            occurred_at,
        )
        .await
    }

    async fn enqueue_with_time<'t, T>(
        executor: T,
        tenant_id: TenantId,
        event_type: &str,
        payload: &Value,
        trace_id: Option<&str>,
        dedup_key: Option<&str>,
        occurred_at: DateTime<Utc>,
    ) -> anyhow::Result<uuid::Uuid>
    where
        T: sqlx::Executor<'t, Database = Postgres>,
    {
        let event_month = month_start(occurred_at);
        let outbox_id: uuid::Uuid = if let Some(dedup_key) = dedup_key {
            sqlx::query_scalar(
                r#"
                insert into outbox_events(
                    tenant_id, event_month, event_type, payload, trace_id, dedup_key
                )
                values ($1, $2, $3, $4, $5, $6)
                on conflict (tenant_id, event_month, dedup_key)
                do update set updated_at = now()
                returning outbox_id
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(event_month)
            .bind(event_type)
            .bind(payload)
            .bind(trace_id)
            .bind(dedup_key)
            .fetch_one(executor)
            .await?
        } else {
            sqlx::query_scalar(
                r#"
                insert into outbox_events(tenant_id, event_month, event_type, payload, trace_id)
                values ($1, $2, $3, $4, $5)
                returning outbox_id
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(event_month)
            .bind(event_type)
            .bind(payload)
            .bind(trace_id)
            .fetch_one(executor)
            .await?
        };
        Ok(outbox_id)
    }

    pub async fn fetch_batch_for_dispatch(
        &self,
        batch_size: i64,
        dispatch_timeout: Duration,
    ) -> anyhow::Result<Vec<OutboxRow>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let stale_cutoff =
            Utc::now() - chrono::Duration::from_std(dispatch_timeout).unwrap_or_default();

        #[derive(sqlx::FromRow)]
        struct Row {
            tenant_id: uuid::Uuid,
            outbox_id: uuid::Uuid,
            event_type: String,
            payload: Value,
            attempts: i32,
            trace_id: Option<String>,
        }

        // 1) Lock rows
        let rows: Vec<Row> = sqlx::query_as::<_, Row>(
            r#"
            select tenant_id, outbox_id, event_type, payload, attempts, trace_id
            from outbox_events
            where (
                status in ('PENDING','FAILED')
                and next_retry_at <= now()
            ) or (
                status = 'DISPATCHING'
                and last_attempt_at <= $2
            )
            order by created_at asc
            limit $1
            for update skip locked
            "#,
        )
        .bind(batch_size)
        .bind(stale_cutoff)
        .fetch_all(&mut *tx)
        .await?;

        if rows.is_empty() {
            tx.commit().await?;
            return Ok(vec![]);
        }

        // 2) Mark as DISPATCHING and increment attempts
        let outbox_ids: Vec<uuid::Uuid> = rows.iter().map(|r| r.outbox_id).collect();
        sqlx::query(
            r#"
            update outbox_events
            set status = 'DISPATCHING',
                attempts = attempts + 1,
                last_attempt_at = now(),
                updated_at = now()
            where outbox_id = any($1)
            "#,
        )
        .bind(&outbox_ids)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(rows
            .into_iter()
            .map(|r| OutboxRow {
                tenant_id: TenantId(r.tenant_id),
                outbox_id: r.outbox_id,
                event_type: r.event_type,
                payload: r.payload,
                attempts: r.attempts + 1,
                trace_id: r.trace_id,
            })
            .collect())
    }

    pub async fn mark_sent(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update outbox_events
            set status = 'SENT',
                published_at = now(),
                updated_at = now()
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
        next_retry_at: DateTime<Utc>,
        last_error: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update outbox_events
            set status = 'FAILED',
                next_retry_at = $3,
                last_error = $4,
                last_attempt_at = now(),
                updated_at = now()
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .bind(next_retry_at)
        .bind(last_error)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_dead(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
        last_error: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update outbox_events
            set status = 'DEAD',
                last_error = $3,
                last_attempt_at = now(),
                updated_at = now()
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .bind(last_error)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn retry_failed_outbox(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        let status: Option<String> = sqlx::query_scalar(
            r#"
            select status
            from outbox_events
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(status) = status else {
            return Err(anyhow!("outbox event not found"));
        };

        if status == "PENDING" {
            return Ok(());
        }

        if status != "FAILED" {
            return Err(anyhow!("outbox status not retryable"));
        }

        sqlx::query(
            r#"
            update outbox_events
            set status = 'PENDING',
                next_retry_at = now(),
                last_error = '',
                last_attempt_at = now(),
                updated_at = now()
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn requeue_outbox(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        let status: Option<String> = sqlx::query_scalar(
            r#"
            select status
            from outbox_events
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(status) = status else {
            return Err(anyhow!("outbox event not found"));
        };

        if status == "PENDING" {
            return Ok(());
        }

        if !matches!(status.as_str(), "FAILED" | "DEAD" | "DISPATCHING") {
            return Err(anyhow!("outbox status not requeueable"));
        }

        sqlx::query(
            r#"
            update outbox_events
            set status = 'PENDING',
                next_retry_at = now(),
                last_error = '',
                last_attempt_at = now(),
                updated_at = now()
            where tenant_id = $1 and outbox_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn count_backlog(&self, dispatch_timeout: Duration) -> anyhow::Result<i64> {
        let stale_cutoff =
            Utc::now() - chrono::Duration::from_std(dispatch_timeout).unwrap_or_default();
        let count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from outbox_events
            where (
                status in ('PENDING','FAILED')
                and next_retry_at <= now()
            ) or (
                status = 'DISPATCHING'
                and last_attempt_at <= $1
            )
            "#,
        )
        .bind(stale_cutoff)
        .fetch_one(self.pool)
        .await?;
        Ok(count)
    }

    pub async fn list_outbox_events(
        &self,
        tenant_id: TenantId,
        filters: OutboxListFilters,
    ) -> anyhow::Result<OutboxListResult> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tenant_id: uuid::Uuid,
            outbox_id: uuid::Uuid,
            event_type: String,
            status: String,
            attempts: i32,
            dedup_key: Option<String>,
            last_error: String,
            next_retry_at: DateTime<Utc>,
            last_attempt_at: DateTime<Utc>,
            published_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
              tenant_id,
              outbox_id,
              event_type,
              status,
              attempts,
              dedup_key,
              last_error,
              next_retry_at,
              last_attempt_at,
              published_at,
              created_at,
              updated_at
            from outbox_events
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(event_type) = filters.event_type {
            qb.push(" and event_type = ");
            qb.push_bind(event_type);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, outbox_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.outbox_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, outbox_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| OutboxCursor {
            created_at: row.created_at,
            outbox_id: row.outbox_id,
        });
        let items = rows
            .into_iter()
            .map(|row| OutboxEvent {
                tenant_id: TenantId(row.tenant_id),
                outbox_id: row.outbox_id,
                event_type: row.event_type,
                status: row.status,
                attempts: row.attempts,
                dedup_key: row.dedup_key,
                last_error: row.last_error,
                next_retry_at: row.next_retry_at,
                last_attempt_at: row.last_attempt_at,
                published_at: row.published_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(OutboxListResult { items, next_cursor })
    }
}
