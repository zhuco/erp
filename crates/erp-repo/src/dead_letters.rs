use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetter {
    pub dead_letter_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub outbox_id: uuid::Uuid,
    pub event_type: String,
    pub payload: Value,
    pub error: String,
    pub attempts: i32,
    pub classification: String,
    pub next_action: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub replayed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct DeadLetterCursor {
    pub created_at: DateTime<Utc>,
    pub dead_letter_id: uuid::Uuid,
}

impl DeadLetterCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.dead_letter_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let dead_letter_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            dead_letter_id,
        })
    }
}

pub struct DeadLetterListFilters {
    pub status: Option<String>,
    pub cursor: Option<DeadLetterCursor>,
    pub limit: i64,
}

pub struct DeadLetterListResult {
    pub items: Vec<DeadLetter>,
    pub next_cursor: Option<DeadLetterCursor>,
}

#[derive(Debug, Clone)]
pub struct ReplayMessage {
    pub subject: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutboxEnvelope {
    tenant_id: TenantId,
    outbox_id: uuid::Uuid,
    event_type: String,
    payload: Value,
    produced_at: DateTime<Utc>,
    trace_id: Option<String>,
}

fn sanitize_subject_token(input: &str) -> String {
    let mut safe = String::with_capacity(input.len());
    for c in input.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            safe.push(c.to_ascii_lowercase());
        } else {
            safe.push('_');
        }
    }
    safe
}

fn subject_for(tenant_id: &str, event_type: &str) -> String {
    let safe = sanitize_subject_token(event_type);
    format!("erp.outbox.{tenant_id}.{safe}")
}

fn subject_for_print_job(tenant_id: &str, device_fingerprint: Option<&str>) -> String {
    let target = device_fingerprint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("broadcast");
    let safe = sanitize_subject_token(target);
    format!("erp.print.{tenant_id}.{safe}")
}

pub struct DeadLettersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> DeadLettersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_dead_letters(
        &self,
        tenant_id: TenantId,
        filters: DeadLetterListFilters,
    ) -> anyhow::Result<DeadLetterListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
              dead_letter_id,
              tenant_id,
              outbox_id,
              event_type,
              payload,
              error,
              attempts,
              classification,
              next_action,
              status,
              created_at,
              replayed_at
            from dead_letters
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, dead_letter_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.dead_letter_id);
            qb.push(")");
        }
        qb.push(" order by created_at desc, dead_letter_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<DeadLetterRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| DeadLetterCursor {
            created_at: row.created_at,
            dead_letter_id: row.dead_letter_id,
        });
        let items = rows
            .into_iter()
            .map(DeadLetterRow::into_dead_letter)
            .collect();
        Ok(DeadLetterListResult { items, next_cursor })
    }

    pub async fn get_dead_letter(
        &self,
        tenant_id: TenantId,
        dead_letter_id: uuid::Uuid,
    ) -> anyhow::Result<Option<DeadLetter>> {
        let row: Option<DeadLetterRow> = sqlx::query_as(
            r#"
            select
              dead_letter_id,
              tenant_id,
              outbox_id,
              event_type,
              payload,
              error,
              attempts,
              classification,
              next_action,
              status,
              created_at,
              replayed_at
            from dead_letters
            where tenant_id = $1 and dead_letter_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(dead_letter_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(DeadLetterRow::into_dead_letter))
    }

    pub async fn build_replay_message(
        &self,
        tenant_id: TenantId,
        dead_letter_id: uuid::Uuid,
        trace_id: Option<String>,
    ) -> anyhow::Result<ReplayMessage> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tenant_id: uuid::Uuid,
            outbox_id: uuid::Uuid,
            event_type: String,
            payload: Value,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select tenant_id, outbox_id, event_type, payload
            from dead_letters
            where tenant_id = $1 and dead_letter_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(dead_letter_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Err(anyhow!("dead letter not found"));
        };

        let envelope = OutboxEnvelope {
            tenant_id: TenantId(row.tenant_id),
            outbox_id: row.outbox_id,
            event_type: row.event_type.clone(),
            payload: row.payload,
            produced_at: Utc::now(),
            trace_id,
        };

        let subject = if row.event_type == "PRINT_JOB_QUEUED" {
            let device_fingerprint = envelope
                .payload
                .get("device_fingerprint")
                .and_then(|value| value.as_str());
            subject_for_print_job(&row.tenant_id.to_string(), device_fingerprint)
        } else {
            subject_for(&row.tenant_id.to_string(), &row.event_type)
        };
        let payload = serde_json::to_vec(&envelope)?;

        Ok(ReplayMessage { subject, payload })
    }

    pub async fn mark_replayed(
        &self,
        tenant_id: TenantId,
        dead_letter_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update dead_letters
            set status = 'REPLAYED',
                replayed_at = now(),
                updated_at = now()
            where tenant_id = $1 and dead_letter_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(dead_letter_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct DeadLetterRow {
    dead_letter_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    outbox_id: uuid::Uuid,
    event_type: String,
    payload: Value,
    error: String,
    attempts: i32,
    classification: String,
    next_action: String,
    status: String,
    created_at: DateTime<Utc>,
    replayed_at: Option<DateTime<Utc>>,
}

impl DeadLetterRow {
    fn into_dead_letter(self) -> DeadLetter {
        DeadLetter {
            dead_letter_id: self.dead_letter_id,
            tenant_id: TenantId(self.tenant_id),
            outbox_id: self.outbox_id,
            event_type: self.event_type,
            payload: self.payload,
            error: self.error,
            attempts: self.attempts,
            classification: self.classification,
            next_action: self.next_action,
            status: self.status,
            created_at: self.created_at,
            replayed_at: self.replayed_at,
        }
    }
}
