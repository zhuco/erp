use chrono::{DateTime, Datelike, NaiveDate, Utc};
use erp_types::{Platform, ShopId, TenantId};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

fn month_start(dt: DateTime<Utc>) -> NaiveDate {
    let d = dt.date_naive();
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
}

pub struct InboxRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InboxRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_raw_payload(
        &self,
        tenant_id: TenantId,
        platform: Platform,
        shop_id: ShopId,
        headers: &Value,
        body: &[u8],
        received_at: DateTime<Utc>,
    ) -> anyhow::Result<i64> {
        let raw_id: i64 = sqlx::query_scalar(
            r#"
            insert into inbox_raw_payloads(tenant_id, platform, shop_id, headers, body, received_at)
            values ($1, $2, $3, $4, $5, $6)
            returning raw_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(platform.to_string())
        .bind(uuid::Uuid::from(shop_id))
        .bind(headers)
        .bind(body)
        .bind(received_at)
        .fetch_one(self.pool)
        .await?;

        Ok(raw_id)
    }

    /// Claim a dedup key. Returns true if first time seen, false if duplicate.
    pub async fn claim_dedup_key(
        &self,
        tenant_id: TenantId,
        dedup_key: &str,
    ) -> anyhow::Result<bool> {
        let inserted: bool = sqlx::query_scalar(
            r#"
            insert into inbox_dedup(tenant_id, dedup_key)
            values ($1, $2)
            on conflict (tenant_id, dedup_key)
            do update set last_seen_at = now(), seen_count = inbox_dedup.seen_count + 1
            returning (xmax = 0) as inserted
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(dedup_key)
        .fetch_one(self.pool)
        .await?;
        Ok(inserted)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_inbox_event(
        &self,
        tenant_id: TenantId,
        platform: Platform,
        shop_id: ShopId,
        dedup_key: &str,
        raw_id: i64,
        received_at: DateTime<Utc>,
        status: &str,
        verify_status: &str,
        failed_reason: Option<&str>,
    ) -> anyhow::Result<uuid::Uuid> {
        let event_month = month_start(received_at);
        let failed_reason = failed_reason.unwrap_or_default();
        let inbox_event_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            insert into inbox_events(
                tenant_id, event_month, platform, shop_id, dedup_key, raw_id, status, received_at,
                verify_status, failed_reason
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            on conflict (tenant_id, event_month, dedup_key)
            do update set updated_at = now()
            returning inbox_event_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(event_month)
        .bind(platform.to_string())
        .bind(uuid::Uuid::from(shop_id))
        .bind(dedup_key)
        .bind(raw_id)
        .bind(status)
        .bind(received_at)
        .bind(verify_status)
        .bind(failed_reason)
        .fetch_one(self.pool)
        .await?;
        Ok(inbox_event_id)
    }

    pub async fn mark_processed(
        &self,
        tenant_id: TenantId,
        inbox_event_id: uuid::Uuid,
        processed_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update inbox_events
            set status = 'PROCESSED', processed_at = $3, updated_at = now()
            where tenant_id = $1 and inbox_event_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inbox_event_id)
        .bind(processed_at)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(
        &self,
        tenant_id: TenantId,
        inbox_event_id: uuid::Uuid,
        error: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update inbox_events
            set status = 'FAILED', error = $3, updated_at = now()
            where tenant_id = $1 and inbox_event_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inbox_event_id)
        .bind(error)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_verify_failed(
        &self,
        tenant_id: TenantId,
        inbox_event_id: uuid::Uuid,
        reason: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update inbox_events
            set status = 'FAILED',
                error = $3,
                verify_status = 'FAILED',
                failed_reason = $3,
                updated_at = now()
            where tenant_id = $1 and inbox_event_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inbox_event_id)
        .bind(reason)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_event_with_raw(
        &self,
        tenant_id: TenantId,
        inbox_event_id: uuid::Uuid,
    ) -> anyhow::Result<InboxEventWithRaw> {
        #[derive(sqlx::FromRow)]
        struct Row {
            platform: String,
            shop_id: uuid::Uuid,
            headers: Value,
            body: Vec<u8>,
            received_at: DateTime<Utc>,
        }

        let r: Row = sqlx::query_as::<_, Row>(
            r#"
            select e.platform, e.shop_id, p.headers, p.body, e.received_at
            from inbox_events e
            join inbox_raw_payloads p on p.raw_id = e.raw_id
            where e.tenant_id = $1 and e.inbox_event_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inbox_event_id)
        .fetch_one(self.pool)
        .await?;

        let platform = r
            .platform
            .parse()
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;

        Ok(InboxEventWithRaw {
            tenant_id,
            platform,
            shop_id: ShopId(r.shop_id),
            inbox_event_id,
            headers: r.headers,
            body: r.body,
            received_at: r.received_at,
        })
    }

    pub async fn retry_failed_event(
        &self,
        tenant_id: TenantId,
        inbox_event_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        #[derive(sqlx::FromRow)]
        struct Row {
            status: String,
            platform: String,
            shop_id: uuid::Uuid,
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: Option<Row> = sqlx::query_as(
            r#"
            select status, platform, shop_id
            from inbox_events
            where tenant_id = $1 and inbox_event_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inbox_event_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Err(anyhow::anyhow!("inbox event not found"));
        };

        if row.status != "FAILED" {
            return Err(anyhow::anyhow!("inbox event status not retryable"));
        }

        let outbox_exists: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select outbox_id
            from outbox_events
            where tenant_id = $1
              and event_type = 'INBOX_EVENT_RECEIVED'
              and payload->>'inbox_event_id' = $2
              and status in ('PENDING','FAILED','DISPATCHING')
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inbox_event_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;

        if outbox_exists.is_none() {
            let payload = serde_json::json!({
                "tenant_id": tenant_id,
                "platform": row.platform,
                "shop_id": row.shop_id,
                "inbox_event_id": inbox_event_id,
            });
            let event_month = month_start(Utc::now());
            let trace_id: Option<String> = None;
            sqlx::query(
                r#"
                insert into outbox_events(tenant_id, event_month, event_type, payload, trace_id)
                values ($1, $2, 'INBOX_EVENT_RECEIVED', $3, $4)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(event_month)
            .bind(payload)
            .bind(trace_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

pub struct InboxEventWithRaw {
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub shop_id: ShopId,
    pub inbox_event_id: uuid::Uuid,
    pub headers: Value,
    pub body: Vec<u8>,
    pub received_at: DateTime<Utc>,
}
