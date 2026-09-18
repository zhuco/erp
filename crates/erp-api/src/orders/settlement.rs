use super::*;

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct OrderSettlementRow {
    pub(super) order_id: uuid::Uuid,
    pub(super) platform: String,
    pub(super) shop_id: uuid::Uuid,
    pub(super) platform_order_id: String,
    pub(super) receiver_name: Option<String>,
    pub(super) total_amount_cent: i64,
    pub(super) order_created_at: DateTime<Utc>,
    pub(super) review_status: String,
    pub(super) exception_reason: Option<String>,
    pub(super) exception_flags: Value,
    pub(super) settlement_status: Option<String>,
    pub(super) locked_at: Option<DateTime<Utc>>,
    pub(super) export_job_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct OrderSettlementPreview {
    pub(super) discount_amount_cent: i64,
    pub(super) receivable_amount_cent: i64,
    pub(super) promotions: Vec<OrderSettlementPromotionResponse>,
}

#[derive(Debug, Clone)]
pub(super) struct OrderSettlementLogSnapshot {
    pub(super) action: String,
    pub(super) message: String,
    pub(super) operator_id: Option<String>,
    pub(super) operator_name: Option<String>,
    pub(super) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct OrderSettlementFilters {
    pub(super) platform: Option<String>,
    pub(super) shop_id: Option<uuid::Uuid>,
    pub(super) keyword: Option<String>,
    pub(super) status: Option<String>,
    pub(super) created_from: Option<DateTime<Utc>>,
    pub(super) created_to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub(super) struct OrderReconciliationBatchInput {
    pub(super) batch_id: String,
    pub(super) platform: String,
    pub(super) shop_id: uuid::Uuid,
    pub(super) period_start: NaiveDate,
    pub(super) period_end: NaiveDate,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct OrderReconciliationPersistedRow {
    pub(super) status: String,
    pub(super) note: Option<String>,
    pub(super) settled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct OrderSettlementPersistedRecord {
    status: String,
    locked_at: Option<DateTime<Utc>>,
    export_job_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct OrderReconciliationBatch {
    pub(super) summary: OrderReconciliationSummaryResponse,
    pub(super) note: Option<String>,
    pub(super) orders: Vec<OrderReconciliationOrderResponse>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct StoredOrderSettlementRule {
    pub(super) stack_mode: Option<String>,
    pub(super) member_discount_rate: Option<i64>,
    pub(super) promo_discount_rate: Option<i64>,
    pub(super) max_discount_rate: Option<i64>,
}

pub(super) fn default_order_settlement_rule() -> OrderSettlementRuleResponse {
    OrderSettlementRuleResponse {
        stack_mode: "STACKED".to_string(),
        member_discount_rate: 3,
        promo_discount_rate: 5,
        max_discount_rate: 12,
        version: 0,
        updated_at: None,
    }
}

pub(super) fn normalize_order_settlement_stack_mode(raw: &str) -> Option<String> {
    match raw.trim().to_uppercase().as_str() {
        "STACKED" => Some("STACKED".to_string()),
        "EXCLUSIVE" => Some("EXCLUSIVE".to_string()),
        "PRIORITY" => Some("PRIORITY".to_string()),
        _ => None,
    }
}

pub(super) fn normalize_order_settlement_status(raw: Option<&str>) -> String {
    match raw.unwrap_or("OPEN").trim().to_uppercase().as_str() {
        "LOCKED" => "LOCKED".to_string(),
        "EXPORTED" => "EXPORTED".to_string(),
        _ => "OPEN".to_string(),
    }
}

pub(super) fn derive_order_audit_status_from_review(review_status: &str) -> String {
    match OrderReviewStatus::from_str(review_status).unwrap_or(OrderReviewStatus::ManualReview) {
        OrderReviewStatus::AutoApproved | OrderReviewStatus::Approved => {
            "REVIEW_PASSED".to_string()
        }
        OrderReviewStatus::Rejected => "REVIEW_REJECTED".to_string(),
        OrderReviewStatus::NotReviewed
        | OrderReviewStatus::ManualReview
        | OrderReviewStatus::RolledBack => "REVIEW_REQUIRED".to_string(),
    }
}

pub(super) fn build_order_blocking_reason(
    exception_reason: Option<&str>,
    exception_flags: &Value,
) -> Option<String> {
    trim_to_option(exception_reason)
        .or_else(|| json_array_to_strings(exception_flags).into_iter().next())
}

pub(super) fn build_order_reconciliation_exception_status(row: &OrderSettlementRow) -> String {
    if json_array_to_strings(&row.exception_flags).is_empty()
        && trim_to_option(row.exception_reason.as_deref()).is_none()
    {
        "NONE".to_string()
    } else {
        "OPEN".to_string()
    }
}

pub(super) fn build_order_settlement_preview(
    total_amount_cent: i64,
    rule: &OrderSettlementRuleResponse,
) -> OrderSettlementPreview {
    let member_discount =
        ((total_amount_cent as f64) * (rule.member_discount_rate as f64) / 100.0).round() as i64;
    let promo_discount =
        ((total_amount_cent as f64) * (rule.promo_discount_rate as f64) / 100.0).round() as i64;
    let max_discount =
        ((total_amount_cent as f64) * (rule.max_discount_rate as f64) / 100.0).round() as i64;
    let promotions = vec![
        OrderSettlementPromotionResponse {
            name: "会员折扣".to_string(),
            promotion_type: "MEMBER".to_string(),
            rate: rule.member_discount_rate,
            amount_cent: member_discount,
        },
        OrderSettlementPromotionResponse {
            name: "渠道促销".to_string(),
            promotion_type: "PROMO".to_string(),
            rate: rule.promo_discount_rate,
            amount_cent: promo_discount,
        },
    ];

    match rule.stack_mode.as_str() {
        "EXCLUSIVE" => {
            let selected = if member_discount >= promo_discount {
                promotions[0].clone()
            } else {
                promotions[1].clone()
            };
            let discount_amount_cent = selected.amount_cent.min(max_discount);
            OrderSettlementPreview {
                discount_amount_cent,
                receivable_amount_cent: total_amount_cent - discount_amount_cent,
                promotions: vec![OrderSettlementPromotionResponse {
                    amount_cent: discount_amount_cent,
                    ..selected
                }],
            }
        }
        "PRIORITY" => {
            let selected = if rule.member_discount_rate >= rule.promo_discount_rate {
                promotions[0].clone()
            } else {
                promotions[1].clone()
            };
            let discount_amount_cent = selected.amount_cent.min(max_discount);
            OrderSettlementPreview {
                discount_amount_cent,
                receivable_amount_cent: total_amount_cent - discount_amount_cent,
                promotions: vec![OrderSettlementPromotionResponse {
                    amount_cent: discount_amount_cent,
                    ..selected
                }],
            }
        }
        _ => {
            let discount_amount_cent = (member_discount + promo_discount).min(max_discount);
            OrderSettlementPreview {
                discount_amount_cent,
                receivable_amount_cent: total_amount_cent - discount_amount_cent,
                promotions,
            }
        }
    }
}

pub(super) async fn load_order_settlement_rule(
    state: &AppState,
    tenant_id: TenantId,
) -> anyhow::Result<OrderSettlementRuleResponse> {
    let repo = SettingsRepo::new(state.db.pool());
    let setting = repo
        .list_settings(
            tenant_id,
            SettingListFilters {
                scope_type: Some("TENANT".to_string()),
                scope_id: None,
                key: Some(OMS_SETTLEMENT_RULE_KEY.to_string()),
            },
        )
        .await?
        .into_iter()
        .next();

    let mut rule = default_order_settlement_rule();
    let Some(setting) = setting else {
        return Ok(rule);
    };
    let stored = serde_json::from_value::<StoredOrderSettlementRule>(setting.value_json).unwrap_or(
        StoredOrderSettlementRule {
            stack_mode: None,
            member_discount_rate: None,
            promo_discount_rate: None,
            max_discount_rate: None,
        },
    );
    if let Some(stack_mode) = stored
        .stack_mode
        .as_deref()
        .and_then(normalize_order_settlement_stack_mode)
    {
        rule.stack_mode = stack_mode;
    }
    if let Some(value) = stored.member_discount_rate {
        rule.member_discount_rate = value.max(0);
    }
    if let Some(value) = stored.promo_discount_rate {
        rule.promo_discount_rate = value.max(0);
    }
    if let Some(value) = stored.max_discount_rate {
        rule.max_discount_rate = value.max(0);
    }
    rule.version = setting.version;
    rule.updated_at = Some(setting.updated_at);
    Ok(rule)
}

pub(super) async fn list_order_settlement_rows(
    state: &AppState,
    tenant_id: TenantId,
    filters: &OrderSettlementFilters,
) -> anyhow::Result<Vec<OrderSettlementRow>> {
    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        select
            o.order_id,
            o.platform,
            o.shop_id,
            o.platform_order_id,
            o.receiver_name,
            o.total_amount_cent,
            i.order_created_at,
            o.review_status,
            o.exception_reason,
            o.exception_flags,
            s.status as settlement_status,
            s.locked_at,
            s.export_job_id
        from orders o
        join order_index i
          on i.tenant_id = o.tenant_id
         and i.order_id = o.order_id
         and i.order_month = o.order_month
        left join order_settlement_records s
          on s.tenant_id = o.tenant_id
         and s.order_id = o.order_id
        where i.tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(tenant_id));

    if let Some(platform) = filters.platform.as_deref() {
        qb.push(" and o.platform = ");
        qb.push_bind(platform);
    }
    if let Some(shop_id) = filters.shop_id {
        qb.push(" and o.shop_id = ");
        qb.push_bind(shop_id);
    }
    if let Some(keyword) = filters.keyword.as_deref() {
        let keyword = format!("%{}%", keyword.trim());
        qb.push(" and (");
        qb.push("cast(o.order_id as text) ilike ");
        qb.push_bind(keyword.clone());
        qb.push(" or o.platform_order_id ilike ");
        qb.push_bind(keyword.clone());
        qb.push(" or coalesce(o.receiver_name, '') ilike ");
        qb.push_bind(keyword);
        qb.push(")");
    }
    if let Some(status) = filters.status.as_deref() {
        qb.push(" and coalesce(s.status, 'OPEN') = ");
        qb.push_bind(status);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and i.order_created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and i.order_created_at <= ");
        qb.push_bind(created_to);
    }

    qb.push(" order by i.order_created_at desc, o.order_id desc");
    qb.build_query_as()
        .fetch_all(state.db.pool())
        .await
        .map_err(Into::into)
}

pub(super) async fn load_order_settlement_row(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Option<OrderSettlementRow>> {
    sqlx::query_as(
        r#"
        select
            o.order_id,
            o.platform,
            o.shop_id,
            o.platform_order_id,
            o.receiver_name,
            o.total_amount_cent,
            i.order_created_at,
            o.review_status,
            o.exception_reason,
            o.exception_flags,
            s.status as settlement_status,
            s.locked_at,
            s.export_job_id
        from orders o
        join order_index i
          on i.tenant_id = o.tenant_id
         and i.order_id = o.order_id
         and i.order_month = o.order_month
        left join order_settlement_records s
          on s.tenant_id = o.tenant_id
         and s.order_id = o.order_id
        where i.tenant_id = $1
          and i.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_optional(state.db.pool())
    .await
    .map_err(Into::into)
}

pub(super) async fn load_order_settlement_logs(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Vec<OrderSettlementLogSnapshot>> {
    let repo = OperationLogsRepo::new(state.db.pool());
    let result = repo
        .list_logs(
            tenant_id,
            OperationLogListFilters {
                action: None,
                ref_type: Some("ORDER".to_string()),
                ref_id: Some(order_id.to_string()),
                actor_user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 50,
            },
        )
        .await?;

    Ok(result
        .items
        .into_iter()
        .filter(|log| log.action.starts_with("OMS.SETTLEMENT_"))
        .map(|log| OrderSettlementLogSnapshot {
            action: log
                .action
                .strip_prefix("OMS.SETTLEMENT_")
                .unwrap_or(log.action.as_str())
                .to_string(),
            message: log
                .metadata
                .get("message")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| log.action.clone()),
            operator_id: log
                .metadata
                .get("operator_id")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .or_else(|| log.actor_user_id.map(|value| value.to_string())),
            operator_name: log
                .metadata
                .get("operator_name")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            created_at: log.created_at,
        })
        .collect())
}

pub(super) fn build_order_settlement_summary_response(
    row: &OrderSettlementRow,
    rule: &OrderSettlementRuleResponse,
    logs: &[OrderSettlementLogSnapshot],
) -> OrderSettlementSummaryResponse {
    let preview = build_order_settlement_preview(row.total_amount_cent, rule);
    let latest_log = logs.first();
    OrderSettlementSummaryResponse {
        order_id: row.order_id.to_string(),
        order_no: row.platform_order_id.clone(),
        platform: row.platform.clone(),
        shop_id: row.shop_id.to_string(),
        receiver_name: row.receiver_name.clone(),
        created_at: row.order_created_at,
        total_amount_cent: row.total_amount_cent,
        discount_amount_cent: preview.discount_amount_cent,
        receivable_amount_cent: preview.receivable_amount_cent,
        settlement_status: normalize_order_settlement_status(row.settlement_status.as_deref()),
        locked_at: row.locked_at,
        export_job_id: row.export_job_id.clone(),
        latest_action: latest_log.map(|log| log.message.clone()),
        latest_action_at: latest_log.map(|log| log.created_at),
    }
}

pub(super) async fn build_order_settlement_detail_response(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
    rule: &OrderSettlementRuleResponse,
) -> anyhow::Result<Option<OrderSettlementDetailResponse>> {
    let Some(row) = load_order_settlement_row(state, tenant_id, order_id).await? else {
        return Ok(None);
    };
    let logs = load_order_settlement_logs(state, tenant_id, order_id).await?;
    let preview = build_order_settlement_preview(row.total_amount_cent, rule);
    let summary = build_order_settlement_summary_response(&row, rule, &logs);
    let action_logs = logs
        .into_iter()
        .rev()
        .map(|log| OrderSettlementActionRecordResponse {
            action: log.action,
            message: log.message,
            operator_id: log.operator_id,
            operator_name: log.operator_name,
            created_at: log.created_at,
        })
        .collect();

    Ok(Some(OrderSettlementDetailResponse {
        order_id: summary.order_id,
        order_no: summary.order_no,
        platform: summary.platform,
        shop_id: summary.shop_id,
        receiver_name: summary.receiver_name,
        created_at: summary.created_at,
        total_amount_cent: summary.total_amount_cent,
        discount_amount_cent: summary.discount_amount_cent,
        receivable_amount_cent: summary.receivable_amount_cent,
        settlement_status: summary.settlement_status,
        locked_at: summary.locked_at,
        export_job_id: summary.export_job_id,
        latest_action: summary.latest_action,
        latest_action_at: summary.latest_action_at,
        promotions: preview.promotions,
        action_logs,
    }))
}

pub(super) async fn upsert_order_settlement_record(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
    status: &str,
    locked_at: Option<DateTime<Utc>>,
    export_job_id: Option<String>,
) -> anyhow::Result<()> {
    let existing: Option<OrderSettlementPersistedRecord> = sqlx::query_as(
        r#"
        select status, locked_at, export_job_id
        from order_settlement_records
        where tenant_id = $1
          and order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_optional(state.db.pool())
    .await?;
    let (effective_status, effective_locked_at, effective_export_job_id) =
        resolve_order_settlement_transition(
            existing.as_ref(),
            status,
            locked_at,
            export_job_id.as_deref(),
        );
    sqlx::query(
        r#"
        insert into order_settlement_records (
            tenant_id, order_id, status, locked_at, export_job_id
        ) values ($1, $2, $3, $4, $5)
        on conflict (tenant_id, order_id)
        do update set
            status = excluded.status,
            locked_at = excluded.locked_at,
            export_job_id = excluded.export_job_id,
            updated_at = now()
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .bind(effective_status)
    .bind(effective_locked_at)
    .bind(effective_export_job_id)
    .execute(state.db.pool())
    .await?;
    Ok(())
}

fn normalize_order_settlement_export_job_id(raw: Option<&str>) -> Option<String> {
    trim_to_option(raw)
}

fn is_allowed_order_settlement_transition(current_status: &str, next_status: &str) -> bool {
    matches!(
        (current_status, next_status),
        ("OPEN", "OPEN")
            | ("OPEN", "LOCKED")
            | ("LOCKED", "OPEN")
            | ("LOCKED", "LOCKED")
            | ("LOCKED", "EXPORTED")
            | ("EXPORTED", "EXPORTED")
    )
}

fn resolve_order_settlement_transition(
    existing: Option<&OrderSettlementPersistedRecord>,
    requested_status: &str,
    requested_locked_at: Option<DateTime<Utc>>,
    requested_export_job_id: Option<&str>,
) -> (String, Option<DateTime<Utc>>, Option<String>) {
    let requested_status = normalize_order_settlement_status(Some(requested_status));
    let requested_export_job_id = normalize_order_settlement_export_job_id(requested_export_job_id);

    let Some(existing) = existing else {
        return match requested_status.as_str() {
            "OPEN" => ("OPEN".to_string(), None, requested_export_job_id),
            "LOCKED" => (
                "LOCKED".to_string(),
                requested_locked_at,
                requested_export_job_id,
            ),
            "EXPORTED" => (
                "EXPORTED".to_string(),
                requested_locked_at,
                requested_export_job_id,
            ),
            _ => ("OPEN".to_string(), None, requested_export_job_id),
        };
    };

    let current_status = normalize_order_settlement_status(Some(existing.status.as_str()));
    let current_locked_at = existing.locked_at;
    let current_export_job_id =
        normalize_order_settlement_export_job_id(existing.export_job_id.as_deref());

    if !is_allowed_order_settlement_transition(&current_status, &requested_status) {
        return (current_status, current_locked_at, current_export_job_id);
    }

    let merged_export_job_id = current_export_job_id.or(requested_export_job_id);
    match requested_status.as_str() {
        "OPEN" => ("OPEN".to_string(), None, merged_export_job_id),
        "LOCKED" => (
            "LOCKED".to_string(),
            current_locked_at.or(requested_locked_at),
            merged_export_job_id,
        ),
        "EXPORTED" => (
            "EXPORTED".to_string(),
            current_locked_at.or(requested_locked_at),
            merged_export_job_id,
        ),
        _ => ("OPEN".to_string(), None, merged_export_job_id),
    }
}

pub(super) async fn record_order_settlement_log(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
    action_code: &str,
    operator_id: Option<UserId>,
    operator_name: Option<String>,
    export_job_id: Option<String>,
    message: String,
) {
    let repo = OperationLogsRepo::new(state.db.pool());
    let _ = repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: operator_id,
            action: action_code.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "message": message,
                "operator_id": operator_id.map(|value| value.to_string()),
                "operator_name": operator_name,
                "export_job_id": export_job_id,
            })),
        })
        .await;
}

pub(super) fn build_order_reconciliation_batch_id(
    platform: &str,
    shop_id: uuid::Uuid,
    period_start: NaiveDate,
    period_end: NaiveDate,
) -> String {
    format!("{platform}:{shop_id}:{period_start}:{period_end}")
}

pub(super) fn parse_order_reconciliation_batch_id(
    raw: &str,
) -> Result<OrderReconciliationBatchInput, Response> {
    let parts = raw.split(':').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(bad_request("bad batch_id").into_response());
    }
    let shop_id =
        uuid::Uuid::parse_str(parts[1]).map_err(|_| bad_request("bad batch_id").into_response())?;
    let period_start = NaiveDate::parse_from_str(parts[2], "%Y-%m-%d")
        .map_err(|_| bad_request("bad batch_id").into_response())?;
    let period_end = NaiveDate::parse_from_str(parts[3], "%Y-%m-%d")
        .map_err(|_| bad_request("bad batch_id").into_response())?;
    if period_start > period_end {
        return Err(bad_request("bad batch_id").into_response());
    }
    Ok(OrderReconciliationBatchInput {
        batch_id: raw.to_string(),
        platform: parts[0].to_string(),
        shop_id,
        period_start,
        period_end,
    })
}

pub(super) async fn load_order_reconciliation_persisted_row(
    state: &AppState,
    tenant_id: TenantId,
    batch_id: &str,
) -> anyhow::Result<Option<OrderReconciliationPersistedRow>> {
    sqlx::query_as(
        r#"
        select status, note, settled_at
        from order_reconciliation_batches
        where tenant_id = $1
          and batch_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(batch_id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(Into::into)
}

pub(super) fn build_order_reconciliation_batch(
    rows: &[OrderSettlementRow],
    rule: &OrderSettlementRuleResponse,
    input: &OrderReconciliationBatchInput,
    persisted: Option<OrderReconciliationPersistedRow>,
) -> OrderReconciliationBatch {
    let mut reasons = Vec::new();
    let mut expected_cent = 0_i64;
    let mut actual_cent = 0_i64;
    let orders = rows
        .iter()
        .map(|row| {
            let preview = build_order_settlement_preview(row.total_amount_cent, rule);
            let audit_status = derive_order_audit_status_from_review(&row.review_status);
            let exception_status = build_order_reconciliation_exception_status(row);
            let blocking_reason =
                build_order_blocking_reason(row.exception_reason.as_deref(), &row.exception_flags)
                    .unwrap_or_else(|| "无差异".to_string());
            let unresolved = exception_status != "NONE" || audit_status == "REVIEW_REJECTED";
            expected_cent += preview.receivable_amount_cent;
            actual_cent += if unresolved {
                0
            } else {
                preview.receivable_amount_cent
            };
            if unresolved {
                for value in json_array_to_strings(&row.exception_flags) {
                    if !reasons.iter().any(|item| item == &value) {
                        reasons.push(value);
                    }
                }
                if let Some(reason) = trim_to_option(row.exception_reason.as_deref()) {
                    if !reasons.iter().any(|item| item == &reason) {
                        reasons.push(reason);
                    }
                }
            }
            OrderReconciliationOrderResponse {
                order_id: row.order_id.to_string(),
                order_no: row.platform_order_id.clone(),
                receiver_name: row.receiver_name.clone(),
                total_amount_cent: row.total_amount_cent,
                receivable_amount_cent: preview.receivable_amount_cent,
                settlement_status: normalize_order_settlement_status(
                    row.settlement_status.as_deref(),
                ),
                audit_status,
                exception_status,
                blocking_reason,
            }
        })
        .collect::<Vec<_>>();
    let diff_cent = expected_cent - actual_cent;
    let status = persisted
        .as_ref()
        .map(|item| item.status.clone())
        .unwrap_or_else(|| {
            if diff_cent == 0 {
                "DONE".to_string()
            } else {
                "PENDING".to_string()
            }
        });
    let summary = OrderReconciliationSummaryResponse {
        batch_id: input.batch_id.clone(),
        platform: input.platform.clone(),
        shop_id: input.shop_id.to_string(),
        period_start: input.period_start,
        period_end: input.period_end,
        order_count: orders.len() as i64,
        expected_cent,
        actual_cent,
        diff_cent,
        status,
        reasons,
        settled_at: persisted.as_ref().and_then(|item| item.settled_at),
    };
    OrderReconciliationBatch {
        summary,
        note: persisted.and_then(|item| item.note),
        orders,
    }
}

pub(super) async fn record_order_reconciliation_log(
    state: &AppState,
    tenant_id: TenantId,
    batch: &OrderReconciliationBatchInput,
    operator_id: Option<UserId>,
    operator_name: Option<String>,
    note: Option<String>,
) {
    let repo = OperationLogsRepo::new(state.db.pool());
    let _ = repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: operator_id,
            action: OMS_RECONCILIATION_ACTION_SETTLE.to_string(),
            ref_type: Some("ORDER_RECONCILIATION".to_string()),
            ref_id: Some(batch.batch_id.clone()),
            metadata: Some(serde_json::json!({
                "message": "销售对账批次已结案",
                "operator_id": operator_id.map(|value| value.to_string()),
                "operator_name": operator_name,
                "platform": batch.platform,
                "shop_id": batch.shop_id.to_string(),
                "period_start": batch.period_start.to_string(),
                "period_end": batch.period_end.to_string(),
                "note": note,
            })),
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, day, 10, 0, 0).unwrap()
    }

    #[test]
    fn settlement_transition_blocks_exported_to_locked() {
        let existing = OrderSettlementPersistedRecord {
            status: "EXPORTED".to_string(),
            locked_at: Some(ts(1)),
            export_job_id: Some("job-1".to_string()),
        };

        let (status, locked_at, export_job_id) = resolve_order_settlement_transition(
            Some(&existing),
            "LOCKED",
            Some(ts(2)),
            Some("job-2"),
        );

        assert_eq!(status, "EXPORTED");
        assert_eq!(locked_at, Some(ts(1)));
        assert_eq!(export_job_id.as_deref(), Some("job-1"));
    }

    #[test]
    fn settlement_transition_keeps_first_lock_timestamp_on_idempotent_lock() {
        let existing = OrderSettlementPersistedRecord {
            status: "LOCKED".to_string(),
            locked_at: Some(ts(3)),
            export_job_id: None,
        };

        let (status, locked_at, export_job_id) = resolve_order_settlement_transition(
            Some(&existing),
            "LOCKED",
            Some(ts(4)),
            Some("job-3"),
        );

        assert_eq!(status, "LOCKED");
        assert_eq!(locked_at, Some(ts(3)));
        assert_eq!(export_job_id.as_deref(), Some("job-3"));
    }

    #[test]
    fn settlement_transition_blocks_open_to_exported() {
        let existing = OrderSettlementPersistedRecord {
            status: "OPEN".to_string(),
            locked_at: None,
            export_job_id: None,
        };

        let (status, locked_at, export_job_id) = resolve_order_settlement_transition(
            Some(&existing),
            "EXPORTED",
            Some(ts(5)),
            Some("job-4"),
        );

        assert_eq!(status, "OPEN");
        assert_eq!(locked_at, None);
        assert_eq!(export_job_id, None);
    }
}

pub(super) async fn get_order_settlement_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    }
}

pub(super) async fn update_order_settlement_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Json(req): Json<OrderSettlementRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }

    let Some(stack_mode) = normalize_order_settlement_stack_mode(&req.stack_mode) else {
        return bad_request("bad stack_mode").into_response();
    };
    if !(0..=100).contains(&req.member_discount_rate)
        || !(0..=100).contains(&req.promo_discount_rate)
        || !(0..=100).contains(&req.max_discount_rate)
    {
        return bad_request("discount rate out of range").into_response();
    }

    let repo = SettingsRepo::new(state.db.pool());
    let value_json = serde_json::json!({
        "stack_mode": stack_mode,
        "member_discount_rate": req.member_discount_rate,
        "promo_discount_rate": req.promo_discount_rate,
        "max_discount_rate": req.max_discount_rate,
    });
    match repo
        .upsert_setting(SettingUpsert {
            tenant_id,
            scope_type: "TENANT".to_string(),
            scope_id: None,
            key: OMS_SETTLEMENT_RULE_KEY.to_string(),
            value_json,
            expected_version: req.expected_version,
        })
        .await
    {
        Ok(setting) => (
            StatusCode::OK,
            Json(OrderSettlementRuleResponse {
                stack_mode,
                member_discount_rate: req.member_discount_rate,
                promo_discount_rate: req.promo_discount_rate,
                max_discount_rate: req.max_discount_rate,
                version: setting.version,
                updated_at: Some(setting.updated_at),
            }),
        )
            .into_response(),
        Err(error) => {
            if error.to_string().contains("version conflict") {
                return conflict("settlement rule version conflict").into_response();
            }
            (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    }
}

pub(super) async fn list_order_settlements(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OrderSettlementListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let created_from = match parse_datetime_optional(query.created_from.as_deref(), "created_from")
    {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let created_to = match parse_datetime_optional(query.created_to.as_deref(), "created_to") {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let (Some(from), Some(to)) = (created_from, created_to) {
        if from > to {
            return bad_request("created_from after created_to").into_response();
        }
    }

    let shop_id = match query.shop_id.as_deref().and_then(non_empty) {
        Some(value) => match ShopId::from_str(&value) {
            Ok(parsed) => {
                if let Err(resp) = ensure_shop_access(&ctx, parsed) {
                    return resp.into_response();
                }
                Some(uuid::Uuid::from(parsed))
            }
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let status = match query.status.as_deref().and_then(non_empty) {
        Some(value) => Some(normalize_order_settlement_status(Some(value.as_str()))),
        None => None,
    };
    let filters = OrderSettlementFilters {
        platform: query.platform.as_deref().and_then(non_empty),
        shop_id,
        keyword: query.keyword.as_deref().and_then(non_empty),
        status,
        created_from,
        created_to,
    };
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;

    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let rows = match list_order_settlement_rows(&state, tenant_id, &filters).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let accessible_rows = rows
        .into_iter()
        .filter(|row| ensure_shop_access(&ctx, ShopId(row.shop_id)).is_ok())
        .collect::<Vec<_>>();
    let mut summary = OrderSettlementListSummaryResponse {
        total_orders: 0,
        open_orders: 0,
        locked_orders: 0,
        exported_orders: 0,
        total_amount_cent: 0,
        receivable_amount_cent: 0,
    };
    for row in &accessible_rows {
        let preview = build_order_settlement_preview(row.total_amount_cent, &rule);
        summary.total_orders += 1;
        summary.total_amount_cent += row.total_amount_cent;
        summary.receivable_amount_cent += preview.receivable_amount_cent;
        match normalize_order_settlement_status(row.settlement_status.as_deref()).as_str() {
            "LOCKED" => summary.locked_orders += 1,
            "EXPORTED" => summary.exported_orders += 1,
            _ => summary.open_orders += 1,
        }
    }

    let paged_rows = accessible_rows
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let mut items = Vec::with_capacity(paged_rows.len());
    for row in paged_rows {
        let logs = match load_order_settlement_logs(&state, tenant_id, OrderId(row.order_id)).await
        {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
            }
        };
        items.push(build_order_settlement_summary_response(&row, &rule, &logs));
    }

    (
        StatusCode::OK,
        Json(OrderSettlementListResponse {
            total: accessible_rows.len() as i64,
            items,
            summary,
        }),
    )
        .into_response()
}

pub(super) async fn get_order_settlement_detail(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let Some(row) = (match load_order_settlement_row(&state, tenant_id, order_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    }) else {
        return not_found("order settlement not found").into_response();
    };
    if let Err(resp) = ensure_shop_access(&ctx, ShopId(row.shop_id)) {
        return resp.into_response();
    }
    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    match build_order_settlement_detail_response(&state, tenant_id, order_id, &rule).await {
        Ok(Some(value)) => (StatusCode::OK, Json(value)).into_response(),
        Ok(None) => not_found("order settlement not found").into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    }
}

pub(super) async fn execute_order_settlement_action(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: TenantId,
    order_id: OrderId,
    action: &str,
    payload: OrderSettlementActionRequest,
) -> Result<OrderSettlementSummaryResponse, Response> {
    let Some(row) = load_order_settlement_row(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?
    else {
        return Err(not_found("order settlement not found").into_response());
    };
    if let Err(resp) = ensure_shop_access(ctx, ShopId(row.shop_id)) {
        return Err(resp.into_response());
    }

    let orders_repo = OrdersRepo::new(state.db.pool());
    let state_row = match orders_repo
        .get_order_review_state(tenant_id, order_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => return Err(not_found("order not found").into_response()),
        Err(e) => {
            return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response());
        }
    };

    let current_status = normalize_order_settlement_status(row.settlement_status.as_deref());
    let operator_id = parse_optional_user_id(payload.operator_id.as_deref(), "operator_id")?
        .unwrap_or(ctx.user_id);
    let operator_id_string = operator_id.to_string();
    let operator_name = trim_to_option(payload.operator_name.as_deref());
    let initial_review_status = row.review_status.clone();
    let initial_exception_flags = row.exception_flags.clone();
    let now = Utc::now();
    let (next_status, locked_at, export_job_id, action_code, message) = match action {
        "LOCK" => {
            if current_status == "EXPORTED" {
                return Err(conflict("exported settlement cannot be locked").into_response());
            }
            (
                "LOCKED".to_string(),
                Some(now),
                row.export_job_id.clone(),
                OMS_SETTLEMENT_ACTION_LOCK,
                "订单结算已锁定".to_string(),
            )
        }
        "UNLOCK" => {
            if current_status != "LOCKED" {
                return Err(conflict("only locked settlement can be unlocked").into_response());
            }
            (
                "OPEN".to_string(),
                None,
                row.export_job_id.clone(),
                OMS_SETTLEMENT_ACTION_UNLOCK,
                "订单结算已解锁".to_string(),
            )
        }
        "EXPORT" => {
            if current_status != "LOCKED" {
                return Err(conflict("only locked settlement can be exported").into_response());
            }
            let export_job_id = payload
                .export_job_id
                .as_deref()
                .and_then(non_empty)
                .ok_or_else(|| bad_request("export_job_id is required").into_response())?;
            (
                "EXPORTED".to_string(),
                row.locked_at.or(Some(now)),
                Some(export_job_id),
                OMS_SETTLEMENT_ACTION_EXPORT,
                "订单结算已导出".to_string(),
            )
        }
        _ => return Err(bad_request("bad action").into_response()),
    };

    upsert_order_settlement_record(
        state,
        tenant_id,
        order_id,
        &next_status,
        locked_at,
        export_job_id.clone(),
    )
    .await
    .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;
    record_order_settlement_log(
        state,
        tenant_id,
        order_id,
        action_code,
        Some(operator_id.clone()),
        operator_name.clone(),
        export_job_id.clone(),
        message,
    )
    .await;

    let request_payload = serde_json::json!({
        "action": action,
        "operator_id": operator_id_string.clone(),
        "operator_name": operator_name.clone(),
        "export_job_id": export_job_id.clone(),
    });
    let response_payload = serde_json::json!({
        "current_settlement_status": current_status,
        "next_settlement_status": next_status,
        "export_job_id": export_job_id.clone(),
    });

    record_order_state_logs(
        state,
        req_ctx,
        OrderStateLogInput {
            tenant_id,
            order_id,
            action_code: action_code.to_string(),
            from_status: state_row.order_status,
            to_status: state_row.order_status,
            operator_id: Some(operator_id),
            reason: None,
            request_payload,
            response_payload,
        },
    )
    .await;

    record_audit_log(
        state,
        ctx,
        req_ctx,
        action_code,
        "ORDER",
        Some(order_id.to_string()),
        Some(serde_json::json!({
            "settlement_status": current_status,
            "review_status": initial_review_status,
            "exception_flags": initial_exception_flags,
        })),
        Some(serde_json::json!({
            "settlement_status": next_status,
            "export_job_id": export_job_id.clone(),
            "operator_id": operator_id_string.clone(),
            "operator_name": operator_name,
        })),
    )
    .await;

    let rule = load_order_settlement_rule(state, tenant_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;
    let row = load_order_settlement_row(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?
        .ok_or_else(|| not_found("order settlement not found").into_response())?;
    let logs = load_order_settlement_logs(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;
    Ok(build_order_settlement_summary_response(&row, &rule, &logs))
}

pub(super) async fn lock_order_settlement(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderSettlementActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_settlement_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "LOCK", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn unlock_order_settlement(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderSettlementActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_settlement_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "UNLOCK", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn export_order_settlement(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderSettlementActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_settlement_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "EXPORT", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn list_order_reconciliations(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OrderReconciliationListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    let created_from = match parse_datetime_optional(query.created_from.as_deref(), "created_from")
    {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let created_to = match parse_datetime_optional(query.created_to.as_deref(), "created_to") {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    if let (Some(from), Some(to)) = (created_from, created_to) {
        if from > to {
            return bad_request("created_from after created_to").into_response();
        }
    }
    let shop_id = match query.shop_id.as_deref().and_then(non_empty) {
        Some(value) => match ShopId::from_str(&value) {
            Ok(parsed) => {
                if let Err(resp) = ensure_shop_access(&ctx, parsed) {
                    return resp.into_response();
                }
                Some(uuid::Uuid::from(parsed))
            }
            Err(_) => return bad_request("bad shop_id").into_response(),
        },
        None => None,
    };
    let status_filter = query
        .status
        .as_deref()
        .and_then(non_empty)
        .map(|value| value.to_uppercase());
    let filters = OrderSettlementFilters {
        platform: query.platform.as_deref().and_then(non_empty),
        shop_id,
        keyword: query.keyword.as_deref().and_then(non_empty),
        status: None,
        created_from,
        created_to,
    };
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;

    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let rows = match list_order_settlement_rows(&state, tenant_id, &filters).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let mut grouped: HashMap<(String, uuid::Uuid), Vec<OrderSettlementRow>> = HashMap::new();
    for row in rows {
        if ensure_shop_access(&ctx, ShopId(row.shop_id)).is_err() {
            continue;
        }
        grouped
            .entry((row.platform.clone(), row.shop_id))
            .or_default()
            .push(row);
    }

    let mut batches = Vec::new();
    for ((platform, shop_id), group_rows) in grouped {
        let period_start = filters
            .created_from
            .map(|value| value.date_naive())
            .unwrap_or_else(|| {
                group_rows
                    .iter()
                    .map(|row| row.order_created_at.date_naive())
                    .min()
                    .unwrap_or_else(|| Utc::now().date_naive())
            });
        let period_end = filters
            .created_to
            .map(|value| value.date_naive())
            .unwrap_or_else(|| {
                group_rows
                    .iter()
                    .map(|row| row.order_created_at.date_naive())
                    .max()
                    .unwrap_or_else(|| Utc::now().date_naive())
            });
        let input = OrderReconciliationBatchInput {
            batch_id: build_order_reconciliation_batch_id(
                &platform,
                shop_id,
                period_start,
                period_end,
            ),
            platform,
            shop_id,
            period_start,
            period_end,
        };
        let persisted =
            match load_order_reconciliation_persisted_row(&state, tenant_id, &input.batch_id).await
            {
                Ok(value) => value,
                Err(error) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
                }
            };
        let batch = build_order_reconciliation_batch(&group_rows, &rule, &input, persisted);
        if let Some(status) = status_filter.as_deref() {
            if batch.summary.status.to_uppercase() != status {
                continue;
            }
        }
        batches.push(batch);
    }

    batches.sort_by(|left, right| {
        right
            .summary
            .diff_cent
            .cmp(&left.summary.diff_cent)
            .then_with(|| right.summary.order_count.cmp(&left.summary.order_count))
            .then_with(|| left.summary.batch_id.cmp(&right.summary.batch_id))
    });

    let summary = OrderReconciliationListSummaryResponse {
        total_batches: batches.len() as i64,
        pending_batches: batches
            .iter()
            .filter(|item| item.summary.status == "PENDING")
            .count() as i64,
        done_batches: batches
            .iter()
            .filter(|item| item.summary.status == "DONE")
            .count() as i64,
        expected_total_cent: batches.iter().map(|item| item.summary.expected_cent).sum(),
        diff_total_cent: batches.iter().map(|item| item.summary.diff_cent).sum(),
    };
    let items = batches
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|item| item.summary)
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(OrderReconciliationListResponse {
            total: summary.total_batches,
            summary,
            items,
        }),
    )
        .into_response()
}

pub(super) async fn get_order_reconciliation_detail(
    State(state): State<AppState>,
    Path((tenant_id, batch_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let batch = match parse_order_reconciliation_batch_id(&batch_id) {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_shop_access(&ctx, ShopId(batch.shop_id)) {
        return resp.into_response();
    }

    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let rows = match list_order_settlement_rows(
        &state,
        tenant_id,
        &OrderSettlementFilters {
            platform: Some(batch.platform.clone()),
            shop_id: Some(batch.shop_id),
            keyword: None,
            status: None,
            created_from: Some(
                batch
                    .period_start
                    .and_hms_opt(0, 0, 0)
                    .expect("date should build")
                    .and_utc(),
            ),
            created_to: Some(
                batch
                    .period_end
                    .and_hms_opt(23, 59, 59)
                    .expect("date should build")
                    .and_utc(),
            ),
        },
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    if rows.is_empty() {
        return not_found("order reconciliation not found").into_response();
    }
    let persisted =
        match load_order_reconciliation_persisted_row(&state, tenant_id, &batch.batch_id).await {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
            }
        };
    let batch = build_order_reconciliation_batch(&rows, &rule, &batch, persisted);
    (
        StatusCode::OK,
        Json(OrderReconciliationDetailResponse {
            batch_id: batch.summary.batch_id,
            platform: batch.summary.platform,
            shop_id: batch.summary.shop_id,
            period_start: batch.summary.period_start,
            period_end: batch.summary.period_end,
            order_count: batch.summary.order_count,
            expected_cent: batch.summary.expected_cent,
            actual_cent: batch.summary.actual_cent,
            diff_cent: batch.summary.diff_cent,
            status: batch.summary.status,
            reasons: batch.summary.reasons,
            settled_at: batch.summary.settled_at,
            note: batch.note,
            orders: batch.orders,
        }),
    )
        .into_response()
}

pub(super) async fn settle_order_reconciliation(
    State(state): State<AppState>,
    Path((tenant_id, batch_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Json(payload): Json<OrderReconciliationActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let batch = match parse_order_reconciliation_batch_id(&batch_id) {
        Ok(value) => value,
        Err(resp) => return resp,
    };
    if let Err(resp) = ensure_shop_access(&ctx, ShopId(batch.shop_id)) {
        return resp.into_response();
    }
    let rows = match list_order_settlement_rows(
        &state,
        tenant_id,
        &OrderSettlementFilters {
            platform: Some(batch.platform.clone()),
            shop_id: Some(batch.shop_id),
            keyword: None,
            status: None,
            created_from: Some(
                batch
                    .period_start
                    .and_hms_opt(0, 0, 0)
                    .expect("date should build")
                    .and_utc(),
            ),
            created_to: Some(
                batch
                    .period_end
                    .and_hms_opt(23, 59, 59)
                    .expect("date should build")
                    .and_utc(),
            ),
        },
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    if rows.is_empty() {
        return not_found("order reconciliation not found").into_response();
    }
    let operator_id = match parse_optional_user_id(payload.operator_id.as_deref(), "operator_id") {
        Ok(value) => value.unwrap_or(ctx.user_id),
        Err(resp) => return resp,
    };
    let operator_name = trim_to_option(payload.operator_name.as_deref());
    let note = trim_to_option(payload.note.as_deref());
    let settled_at = Utc::now();
    if let Err(error) = sqlx::query(
        r#"
        insert into order_reconciliation_batches (
            tenant_id, batch_id, platform, shop_id, period_start, period_end,
            status, note, settled_at
        ) values ($1, $2, $3, $4, $5, $6, 'DONE', $7, $8)
        on conflict (tenant_id, batch_id)
        do update set
            status = 'DONE',
            note = excluded.note,
            settled_at = excluded.settled_at,
            updated_at = now()
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(&batch.batch_id)
    .bind(&batch.platform)
    .bind(batch.shop_id)
    .bind(batch.period_start)
    .bind(batch.period_end)
    .bind(note.clone())
    .bind(settled_at)
    .execute(state.db.pool())
    .await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
    }
    record_order_reconciliation_log(
        &state,
        tenant_id,
        &batch,
        Some(operator_id),
        operator_name,
        note,
    )
    .await;

    let rule = match load_order_settlement_rule(&state, tenant_id).await {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
        }
    };
    let persisted =
        match load_order_reconciliation_persisted_row(&state, tenant_id, &batch.batch_id).await {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response();
            }
        };
    let batch = build_order_reconciliation_batch(&rows, &rule, &batch, persisted);
    (StatusCode::OK, Json(batch.summary)).into_response()
}
