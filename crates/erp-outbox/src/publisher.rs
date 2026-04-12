use async_nats::{Client, HeaderMap};
use serde::Serialize;

#[async_trait::async_trait]
pub trait Publisher: Send + Sync {
    async fn publish(&self, subject: &str, payload: Vec<u8>) -> anyhow::Result<()>;
    async fn publish_with_headers(
        &self,
        subject: &str,
        headers: HeaderMap,
        payload: Vec<u8>,
    ) -> anyhow::Result<()> {
        let _ = headers;
        self.publish(subject, payload).await
    }

    async fn publish_json<T: Serialize + Send + Sync>(
        &self,
        subject: &str,
        value: &T,
    ) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec(value)?;
        self.publish(subject, bytes).await
    }
}

pub struct NatsPublisher {
    client: Client,
}

impl NatsPublisher {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl Publisher for NatsPublisher {
    async fn publish(&self, subject: &str, payload: Vec<u8>) -> anyhow::Result<()> {
        self.client
            .publish(subject.to_string(), payload.into())
            .await?;
        Ok(())
    }

    async fn publish_with_headers(
        &self,
        subject: &str,
        headers: HeaderMap,
        payload: Vec<u8>,
    ) -> anyhow::Result<()> {
        self.client
            .publish_with_headers(subject.to_string(), headers, payload.into())
            .await?;
        Ok(())
    }
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

pub fn subject_for(tenant_id: &str, event_type: &str) -> String {
    let safe = sanitize_subject_token(event_type);
    format!("erp.outbox.{tenant_id}.{safe}")
}

pub fn subject_for_print_job(tenant_id: &str, device_fingerprint: Option<&str>) -> String {
    let target = device_fingerprint
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("broadcast");
    let safe = sanitize_subject_token(target);
    format!("erp.print.{tenant_id}.{safe}")
}
