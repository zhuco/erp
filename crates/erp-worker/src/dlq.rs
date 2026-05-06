#![allow(dead_code)]

use anyhow::anyhow;
use async_nats::HeaderMap;
use chrono::Utc;
use erp_adapter::AdapterError;
use erp_common::trace::TRACE_ID_HEADER;
use erp_domain::DeadLetterReplayPayload;
use erp_outbox::envelope::OutboxEnvelope;
use erp_outbox::publisher::{subject_for, subject_for_print_job};
use erp_repo::db::Db;
use erp_repo::shipments::{SHIPMENT_CONFIRMED_EVENT, SHIPMENT_SHIPPED_EVENT};
use serde_json::Value;
use sqlx::PgPool;
use std::future::Future;
use std::time::Duration;

pub const ATTEMPT_HEADER: &str = "X-Worker-Attempt";
pub const DLQ_REPLAY_HEADER: &str = "X-Worker-Replay";
pub const DLQ_REPLAY_ID_HEADER: &str = "X-Worker-Replay-Id";
pub const NEXT_ACTION_MANUAL_REPLAY: &str = "MANUAL_REPLAY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Transient,
    Permanent,
}

impl FailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            FailureClass::Transient => "TRANSIENT",
            FailureClass::Permanent => "PERMANENT",
        }
    }
}

impl std::fmt::Display for FailureClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl RetryPolicy {
    pub fn next_delay(&self, attempt: u32) -> Duration {
        let exponent = attempt.saturating_sub(1).min(20);
        let delay = self
            .base_delay_ms
            .saturating_mul(2_u64.saturating_pow(exponent))
            .min(self.max_delay_ms);
        Duration::from_millis(delay.max(1))
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 500,
            max_delay_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeadLetterRow {
    pub dead_letter_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub outbox_id: uuid::Uuid,
    pub event_type: String,
    pub payload: Value,
    pub error: String,
    pub attempts: i32,
    pub classification: String,
    pub next_action: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ReplayMessage {
    pub subject: String,
    pub payload: Vec<u8>,
    pub headers: HeaderMap,
}

#[derive(Debug)]
pub enum FailureOutcome {
    Retry {
        delay: Duration,
        next_attempt: u32,
        classification: FailureClass,
    },
    DeadLettered {
        dead_letter_id: uuid::Uuid,
        classification: FailureClass,
    },
}

pub fn attempt_from_headers(headers: Option<&HeaderMap>) -> u32 {
    headers
        .and_then(|map| map.get(ATTEMPT_HEADER))
        .and_then(|value| value.as_str().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

pub fn retry_headers(next_attempt: u32, trace_id: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(ATTEMPT_HEADER, next_attempt.to_string());
    if let Some(trace_id) = trace_id {
        headers.insert(TRACE_ID_HEADER, trace_id.to_string());
    }
    headers
}

pub fn replay_headers(dead_letter_id: uuid::Uuid, trace_id: Option<&str>) -> HeaderMap {
    let mut headers = retry_headers(1, trace_id);
    headers.insert(DLQ_REPLAY_HEADER, "true".to_string());
    headers.insert(DLQ_REPLAY_ID_HEADER, dead_letter_id.to_string());
    headers
}

pub fn classify_error(err: &anyhow::Error) -> FailureClass {
    if let Some(adapter_error) = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<AdapterError>())
    {
        return match adapter_error {
            AdapterError::Auth(_) | AdapterError::BadRequest(_) | AdapterError::Unsupported => {
                FailureClass::Permanent
            }
            AdapterError::RateLimited { .. }
            | AdapterError::Temporary(_)
            | AdapterError::Unexpected(_) => FailureClass::Transient,
        };
    }

    if err
        .chain()
        .any(|cause| cause.downcast_ref::<serde_json::Error>().is_some())
    {
        return FailureClass::Permanent;
    }

    FailureClass::Transient
}

pub fn classify_error_for_event(event_type: &str, err: &anyhow::Error) -> FailureClass {
    if event_type == SHIPMENT_SHIPPED_EVENT || event_type == SHIPMENT_CONFIRMED_EVENT {
        if let Some(adapter_error) = err
            .chain()
            .find_map(|cause| cause.downcast_ref::<AdapterError>())
        {
            if matches!(
                adapter_error,
                AdapterError::Auth(_) | AdapterError::BadRequest(_)
            ) {
                return FailureClass::Transient;
            }
        }
    }
    classify_error(err)
}

pub async fn handle_processing_failure(
    pool: &PgPool,
    env: &OutboxEnvelope,
    err: &anyhow::Error,
    attempt: u32,
    policy: &RetryPolicy,
) -> anyhow::Result<FailureOutcome> {
    let classification = classify_error_for_event(&env.event_type, err);

    if classification == FailureClass::Transient && attempt <= policy.max_attempts {
        return Ok(FailureOutcome::Retry {
            delay: policy.next_delay(attempt),
            next_attempt: attempt + 1,
            classification,
        });
    }

    let dead_letter_id = insert_dead_letter(
        pool,
        env,
        &err.to_string(),
        attempt,
        classification,
        NEXT_ACTION_MANUAL_REPLAY,
    )
    .await?;
    Ok(FailureOutcome::DeadLettered {
        dead_letter_id,
        classification,
    })
}

pub async fn insert_dead_letter(
    pool: &PgPool,
    env: &OutboxEnvelope,
    error: &str,
    attempts: u32,
    classification: FailureClass,
    next_action: &str,
) -> anyhow::Result<uuid::Uuid> {
    let dead_letter_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        insert into dead_letters (
          tenant_id,
          outbox_id,
          event_type,
          payload,
          error,
          attempts,
          classification,
          next_action,
          status
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, 'PENDING')
        on conflict (tenant_id, outbox_id)
        do update set
          error = excluded.error,
          attempts = excluded.attempts,
          classification = excluded.classification,
          next_action = excluded.next_action,
          status = 'PENDING',
          updated_at = now()
        returning dead_letter_id
        "#,
    )
    .bind(uuid::Uuid::from(env.tenant_id))
    .bind(env.outbox_id)
    .bind(&env.event_type)
    .bind(&env.payload)
    .bind(error)
    .bind(attempts as i32)
    .bind(classification.as_str())
    .bind(next_action)
    .fetch_one(pool)
    .await?;
    Ok(dead_letter_id)
}

pub async fn fetch_dead_letter(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    dead_letter_id: uuid::Uuid,
) -> anyhow::Result<Option<DeadLetterRow>> {
    #[derive(sqlx::FromRow)]
    struct Row {
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
    }

    let row: Option<Row> = sqlx::query_as(
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
          status
        from dead_letters
        where tenant_id = $1 and dead_letter_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(dead_letter_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| DeadLetterRow {
        dead_letter_id: row.dead_letter_id,
        tenant_id: row.tenant_id,
        outbox_id: row.outbox_id,
        event_type: row.event_type,
        payload: row.payload,
        error: row.error,
        attempts: row.attempts,
        classification: row.classification,
        next_action: row.next_action,
        status: row.status,
    }))
}

pub async fn mark_replayed(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
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
    .bind(tenant_id)
    .bind(dead_letter_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn build_replay_message(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    dead_letter_id: uuid::Uuid,
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
    .bind(tenant_id)
    .bind(dead_letter_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Err(anyhow!("dead letter not found"));
    };

    let envelope = OutboxEnvelope {
        tenant_id: erp_types::TenantId(row.tenant_id),
        outbox_id: row.outbox_id,
        event_type: row.event_type.clone(),
        payload: row.payload,
        produced_at: Utc::now(),
        trace_id: None,
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
    let headers = replay_headers(dead_letter_id, None);
    let payload = serde_json::to_vec(&envelope)?;
    Ok(ReplayMessage {
        subject,
        payload,
        headers,
    })
}

pub async fn replay_dead_letter_with<F, Fut>(
    pool: &PgPool,
    tenant_id: uuid::Uuid,
    dead_letter_id: uuid::Uuid,
    publish: F,
) -> anyhow::Result<ReplayMessage>
where
    F: FnOnce(ReplayMessage) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let message = build_replay_message(pool, tenant_id, dead_letter_id).await?;
    publish(message.clone()).await?;
    mark_replayed(pool, tenant_id, dead_letter_id).await?;
    Ok(message)
}

pub async fn replay_dead_letter(
    pool: &PgPool,
    nats: &async_nats::Client,
    tenant_id: uuid::Uuid,
    dead_letter_id: uuid::Uuid,
) -> anyhow::Result<ReplayMessage> {
    let nats = nats.clone();
    replay_dead_letter_with(pool, tenant_id, dead_letter_id, move |message| {
        let nats = nats.clone();
        async move {
            nats.publish_with_headers(message.subject, message.headers, message.payload.into())
                .await?;
            Ok(())
        }
    })
    .await
}

pub(super) async fn handle_dead_letter_replay(
    db: &Db,
    nats: &async_nats::Client,
    payload: DeadLetterReplayPayload,
) -> anyhow::Result<()> {
    replay_dead_letter(
        db.pool(),
        nats,
        uuid::Uuid::from(payload.tenant_id),
        payload.dead_letter_id,
    )
    .await?;
    Ok(())
}
