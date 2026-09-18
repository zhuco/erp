use crate::envelope::OutboxEnvelope;
use crate::publisher::{Publisher, subject_for, subject_for_print_job};
use async_nats::HeaderMap;
use chrono::{Duration, Utc};
use erp_common::telemetry::metrics;
use erp_common::trace::TRACE_ID_HEADER;
use erp_repo::outbox::OutboxRepo;
use sqlx::PgPool;
use std::time::Duration as StdDuration;
use tracing::{error, info};

pub struct OutboxDispatcher<P: Publisher> {
    pub publisher: P,
    pub pool: PgPool,
    pub batch_size: i64,
    pub poll_interval_ms: u64,
    pub max_tries: i32,
    pub dispatch_timeout: StdDuration,
}

impl<P: Publisher> OutboxDispatcher<P> {
    pub async fn run_forever(&self) -> anyhow::Result<()> {
        loop {
            self.run_once().await?;
            tokio::time::sleep(std::time::Duration::from_millis(self.poll_interval_ms)).await;
        }
    }

    pub async fn run_once(&self) -> anyhow::Result<()> {
        let repo = OutboxRepo::new(&self.pool);
        let backlog = repo.count_backlog(self.dispatch_timeout).await.unwrap_or(0);
        metrics().set_outbox_backlog(backlog as u64);
        let rows = repo
            .fetch_batch_for_dispatch(self.batch_size, self.dispatch_timeout)
            .await?;
        if rows.is_empty() {
            return Ok(());
        }

        for row in rows {
            let subject = if row.event_type == "PRINT_JOB_QUEUED" {
                let device_fingerprint = row
                    .payload
                    .get("device_fingerprint")
                    .and_then(|v| v.as_str());
                subject_for_print_job(&row.tenant_id.to_string(), device_fingerprint)
            } else {
                subject_for(&row.tenant_id.to_string(), &row.event_type)
            };
            let envelope = OutboxEnvelope {
                tenant_id: row.tenant_id,
                outbox_id: row.outbox_id,
                event_type: row.event_type.clone(),
                payload: row.payload.clone(),
                produced_at: Utc::now(),
                trace_id: row.trace_id.clone(),
            };
            let mut headers = HeaderMap::new();
            if let Some(trace_id) = row.trace_id.as_ref() {
                headers.insert(TRACE_ID_HEADER, trace_id.to_string());
            }

            metrics().inc_outbox_dispatch_attempt();
            let publish_result = if headers.is_empty() {
                self.publisher.publish_json(&subject, &envelope).await
            } else {
                let bytes = serde_json::to_vec(&envelope)?;
                self.publisher
                    .publish_with_headers(&subject, headers, bytes)
                    .await
            };

            match publish_result {
                Ok(_) => {
                    metrics().inc_outbox_dispatch_success();
                    repo.mark_sent(row.tenant_id, row.outbox_id).await?;
                    info!(tenant_id=%row.tenant_id, outbox_id=%row.outbox_id, subject=%subject, "outbox dispatched");
                }
                Err(e) => {
                    metrics().inc_outbox_dispatch_failed();
                    error!(tenant_id=%row.tenant_id, outbox_id=%row.outbox_id, error=%e, attempts=row.attempts, "outbox dispatch failed");
                    if row.attempts >= self.max_tries {
                        repo.mark_dead(row.tenant_id, row.outbox_id, &e.to_string())
                            .await?;
                    } else {
                        // Exponential backoff with cap
                        let backoff_secs = (2_i64.pow(row.attempts as u32)).min(300);
                        let next_retry = Utc::now() + Duration::seconds(backoff_secs);
                        repo.mark_failed(row.tenant_id, row.outbox_id, next_retry, &e.to_string())
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }
}
