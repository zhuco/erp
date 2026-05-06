use super::*;

pub(crate) fn shop_view(shop: Shop, token: Option<&PlatformToken>) -> ShopView {
    let token_status = token.map(shop_token_status);
    ShopView {
        shop_id: shop.shop_id,
        tenant_id: shop.tenant_id,
        platform: shop.platform,
        platform_shop_id: shop.platform_shop_id,
        name: shop.name,
        status: shop.status,
        auth_meta_json: shop.auth_meta_json,
        created_at: shop.created_at,
        updated_at: shop.updated_at,
        token_status,
    }
}

pub(crate) fn shop_sync_job_enabled(job: &SyncJob, sales_config: &SalesShopConfig) -> bool {
    if job.status.eq_ignore_ascii_case("DISABLED") {
        return false;
    }
    match job.job_type.as_str() {
        "order_sync" => sales_config.auto_download_orders,
        "refund_sync" => sales_config.enable_after_sales_download,
        "shipping_sync" => sales_config.enable_logistics_upload,
        _ => true,
    }
}

pub(crate) fn derive_shop_oauth_status(shop: &Shop, token: Option<&PlatformToken>) -> String {
    if let Some(status) = json_string_at_pointer(&shop.auth_meta_json, "/oauth_status") {
        return status.to_uppercase();
    }

    let token_status = token.map(shop_token_status);
    if let Some(token_status) = token_status {
        if token_status.has_access_token || token_status.has_refresh_token {
            if token_status.is_expired {
                return "FAILED".to_string();
            }
            return "SUCCESS".to_string();
        }
    }
    if shop.status.eq_ignore_ascii_case("EXPIRED") {
        "FAILED".to_string()
    } else {
        "PENDING".to_string()
    }
}

pub(crate) fn derive_shop_integration_status(
    shop: &Shop,
    oauth_status: &str,
    token_status: Option<&ShopTokenStatus>,
    failed_job_count: i64,
    webhook_health: &ShopWebhookHealthSummary,
) -> String {
    if shop.status.eq_ignore_ascii_case("DISABLED") {
        return "DISCONNECTED".to_string();
    }
    if oauth_status != "SUCCESS" {
        return "DISCONNECTED".to_string();
    }
    if token_status.map(|status| status.is_expired).unwrap_or(true) {
        return "DISCONNECTED".to_string();
    }
    if failed_job_count > 0
        || webhook_health.webhook_status == "WARNING"
        || matches!(
            webhook_health.webhook_sla_status.as_str(),
            "WARNING" | "DEAD"
        )
    {
        return "WARNING".to_string();
    }
    if let Some(token_status) = token_status {
        if token_status.is_expiring_soon {
            return "WARNING".to_string();
        }
        if token_status.refresh_fail_count > 0 && !token_status.last_error.trim().is_empty() {
            return "WARNING".to_string();
        }
    }
    "CONNECTED".to_string()
}

pub(crate) fn derive_webhook_health_summary(
    last_webhook_received_at: Option<DateTime<Utc>>,
    last_webhook_processed_at: Option<DateTime<Utc>>,
    webhook_failed_count: i64,
    webhook_verify_failed_count: i64,
    webhook_pending_count: i64,
    webhook_retrying_count: i64,
    webhook_dlq_count: i64,
    webhook_next_retry_at: Option<DateTime<Utc>>,
    oldest_pending_at: Option<DateTime<Utc>>,
) -> ShopWebhookHealthSummary {
    derive_webhook_health_summary_with_clock(
        last_webhook_received_at,
        last_webhook_processed_at,
        webhook_failed_count,
        webhook_verify_failed_count,
        webhook_pending_count,
        webhook_retrying_count,
        webhook_dlq_count,
        webhook_next_retry_at,
        oldest_pending_at,
        Utc::now(),
    )
}

pub(crate) fn derive_webhook_health_summary_with_clock(
    last_webhook_received_at: Option<DateTime<Utc>>,
    last_webhook_processed_at: Option<DateTime<Utc>>,
    webhook_failed_count: i64,
    webhook_verify_failed_count: i64,
    webhook_pending_count: i64,
    webhook_retrying_count: i64,
    webhook_dlq_count: i64,
    webhook_next_retry_at: Option<DateTime<Utc>>,
    oldest_pending_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> ShopWebhookHealthSummary {
    let webhook_status = if last_webhook_received_at.is_none() {
        "IDLE"
    } else if webhook_failed_count > 0 || webhook_verify_failed_count > 0 {
        "WARNING"
    } else {
        "HEALTHY"
    };
    let backlog_warning = oldest_pending_at
        .map(|pending_at| pending_at <= now - Duration::minutes(5))
        .unwrap_or(false);
    let webhook_sla_status = if webhook_dlq_count > 0 {
        "DEAD"
    } else if webhook_retrying_count > 0 || backlog_warning {
        "WARNING"
    } else if last_webhook_received_at.is_none()
        && last_webhook_processed_at.is_none()
        && webhook_pending_count == 0
        && webhook_retrying_count == 0
        && webhook_dlq_count == 0
    {
        "IDLE"
    } else {
        "HEALTHY"
    };

    ShopWebhookHealthSummary {
        webhook_status: webhook_status.to_string(),
        webhook_sla_status: webhook_sla_status.to_string(),
        last_webhook_received_at,
        last_webhook_processed_at,
        webhook_failed_count,
        webhook_verify_failed_count,
        webhook_pending_count,
        webhook_retrying_count,
        webhook_dlq_count,
        webhook_next_retry_at,
    }
}

pub(crate) fn push_webhook_alert_messages(
    alert_messages: &mut Vec<String>,
    webhook_health: &ShopWebhookHealthSummary,
) {
    if webhook_health.webhook_verify_failed_count > 0 {
        alert_messages.push("Webhook 验签失败".to_string());
    } else if webhook_health.webhook_failed_count > 0 {
        alert_messages.push("Webhook 处理失败".to_string());
    }
    if webhook_health.webhook_dlq_count > 0 {
        alert_messages.push("Webhook 死信待处理".to_string());
    } else if webhook_health.webhook_retrying_count > 0 {
        alert_messages.push("Webhook 自动重试中".to_string());
    } else if webhook_health.webhook_pending_count > 0
        && webhook_health.webhook_sla_status == "WARNING"
    {
        alert_messages.push("Webhook 处理积压".to_string());
    }
}

pub(crate) fn json_string_at_pointer(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn sales_shop_config_view(shop: Shop, config: SalesShopConfig) -> ShopSalesConfigView {
    ShopSalesConfigView {
        shop_id: shop.shop_id,
        platform: shop.platform,
        platform_shop_id: shop.platform_shop_id,
        name: shop.name,
        status: shop.status,
        auto_download_orders: config.auto_download_orders,
        auto_to_review: config.auto_to_review,
        download_interval_minutes: config.download_interval_minutes,
        auto_merge_non_shipping_time: config.auto_merge_non_shipping_time,
        non_shipping_time_start: config.non_shipping_time_start,
        non_shipping_time_end: config.non_shipping_time_end,
        enable_stock_sync: config.enable_stock_sync,
        enable_after_sales_download: config.enable_after_sales_download,
        enable_logistics_upload: config.enable_logistics_upload,
    }
}
