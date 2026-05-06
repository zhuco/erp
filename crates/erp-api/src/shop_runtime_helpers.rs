use super::*;

pub(crate) fn shop_token_status(token: &PlatformToken) -> ShopTokenStatus {
    let now = Utc::now();
    let (expires_in_minutes, is_expired, is_expiring_soon) = match token.expires_at {
        Some(expires_at) => {
            let delta = expires_at.signed_duration_since(now).num_minutes();
            let expired = expires_at <= now;
            let expiring_soon = expires_at <= now + Duration::minutes(30);
            (Some(delta), expired, expiring_soon)
        }
        None => (None, false, false),
    };

    ShopTokenStatus {
        token_status: token.token_status.clone(),
        has_access_token: token.access_token.is_some(),
        has_refresh_token: token.refresh_token.is_some(),
        expires_at: token.expires_at,
        expires_in_minutes,
        is_expired,
        is_expiring_soon,
        scopes: token.scopes.clone(),
        last_refreshed_at: token.last_refreshed_at,
        refresh_fail_count: token.refresh_fail_count,
        last_error: token.last_error.clone(),
        updated_at: token.updated_at,
    }
}

pub(crate) async fn load_shop_credentials_for_shops(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    shop_ids: &[ShopId],
) -> anyhow::Result<HashMap<ShopId, Value>> {
    let mut credentials = HashMap::new();
    if shop_ids.is_empty() {
        return Ok(credentials);
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        shop_id: uuid::Uuid,
        credentials: Value,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select shop_id, credentials
        from shops
        where tenant_id = $1
          and shop_id = any($2)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(
        shop_ids
            .iter()
            .map(|id| uuid::Uuid::from(*id))
            .collect::<Vec<_>>(),
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        credentials.insert(ShopId(row.shop_id), row.credentials);
    }

    Ok(credentials)
}

pub(crate) async fn load_shop_webhook_health_for_shops(
    pool: &sqlx::PgPool,
    tenant_id: TenantId,
    shop_ids: &[ShopId],
) -> anyhow::Result<HashMap<ShopId, ShopWebhookHealthSummary>> {
    let mut health_by_shop = HashMap::new();
    if shop_ids.is_empty() {
        return Ok(health_by_shop);
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        shop_id: uuid::Uuid,
        last_webhook_received_at: Option<DateTime<Utc>>,
        last_webhook_processed_at: Option<DateTime<Utc>>,
        webhook_failed_count: i64,
        webhook_verify_failed_count: i64,
        webhook_pending_count: i64,
        webhook_retrying_count: i64,
        webhook_dlq_count: i64,
        webhook_next_retry_at: Option<DateTime<Utc>>,
        oldest_pending_at: Option<DateTime<Utc>>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        with requested_shops as (
          select unnest($2::uuid[]) as shop_id
        ),
        inbox as (
          select
            shop_id,
            max(received_at) as last_webhook_received_at,
            max(processed_at) as last_webhook_processed_at,
            count(*) filter (where status = 'FAILED')::bigint as webhook_failed_count,
            count(*) filter (where verify_status = 'FAILED')::bigint as webhook_verify_failed_count
          from inbox_events
          where tenant_id = $1
            and shop_id = any($2)
          group by shop_id
        ),
        outbox as (
          select
            (payload->>'shop_id')::uuid as shop_id,
            count(*) filter (where status = 'PENDING')::bigint as webhook_pending_count,
            count(*) filter (where status = 'FAILED')::bigint as webhook_retrying_count,
            min(next_retry_at) filter (where status = 'FAILED') as webhook_next_retry_at,
            min(created_at) filter (where status = 'PENDING') as oldest_pending_at
          from outbox_events
          where tenant_id = $1
            and event_type = 'INBOX_EVENT_RECEIVED'
            and payload ? 'shop_id'
            and (payload->>'shop_id')::uuid = any($2)
          group by (payload->>'shop_id')::uuid
        ),
        dlq as (
          select
            (payload->>'shop_id')::uuid as shop_id,
            count(*) filter (where status = 'PENDING')::bigint as webhook_dlq_count
          from dead_letters
          where tenant_id = $1
            and event_type = 'INBOX_EVENT_RECEIVED'
            and payload ? 'shop_id'
            and (payload->>'shop_id')::uuid = any($2)
          group by (payload->>'shop_id')::uuid
        )
        select
          requested_shops.shop_id,
          inbox.last_webhook_received_at,
          inbox.last_webhook_processed_at,
          coalesce(inbox.webhook_failed_count, 0)::bigint as webhook_failed_count,
          coalesce(inbox.webhook_verify_failed_count, 0)::bigint as webhook_verify_failed_count,
          coalesce(outbox.webhook_pending_count, 0)::bigint as webhook_pending_count,
          coalesce(outbox.webhook_retrying_count, 0)::bigint as webhook_retrying_count,
          coalesce(dlq.webhook_dlq_count, 0)::bigint as webhook_dlq_count,
          outbox.webhook_next_retry_at,
          outbox.oldest_pending_at
        from requested_shops
        left join inbox on inbox.shop_id = requested_shops.shop_id
        left join outbox on outbox.shop_id = requested_shops.shop_id
        left join dlq on dlq.shop_id = requested_shops.shop_id
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(
        shop_ids
            .iter()
            .map(|id| uuid::Uuid::from(*id))
            .collect::<Vec<_>>(),
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        health_by_shop.insert(
            ShopId(row.shop_id),
            derive_webhook_health_summary(
                row.last_webhook_received_at,
                row.last_webhook_processed_at,
                row.webhook_failed_count,
                row.webhook_verify_failed_count,
                row.webhook_pending_count,
                row.webhook_retrying_count,
                row.webhook_dlq_count,
                row.webhook_next_retry_at,
                row.oldest_pending_at,
            ),
        );
    }

    Ok(health_by_shop)
}
