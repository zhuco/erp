use super::*;

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct OrderExceptionWorkbenchOrderRow {
    order_id: uuid::Uuid,
    platform: String,
    shop_id: uuid::Uuid,
    platform_order_id: String,
    receiver_name: Option<String>,
    total_amount_cent: i64,
    review_status: String,
    fulfillment_status: String,
    ship_warehouse_id: Option<uuid::Uuid>,
    exception_reason: Option<String>,
    exception_flags: Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct OrderExceptionTicketRow {
    pub(super) reason_code: String,
    pub(super) status: String,
    pub(super) handler_id: Option<uuid::Uuid>,
    pub(super) created_at: DateTime<Utc>,
    pub(super) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub(super) struct OrderExceptionLogSnapshot {
    action: String,
    message: String,
    operator_id: Option<String>,
    operator_name: Option<String>,
    assignee_id: Option<String>,
    assignee_name: Option<String>,
    audit_status: Option<String>,
    blocking_reason: Option<String>,
    pub(super) created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub(super) struct OrderExceptionTicketSelection {
    pub(super) status: String,
    handler_id: Option<String>,
    reason_codes: Vec<String>,
    blocking_reasons: Vec<String>,
    latest_action_at: Option<DateTime<Utc>>,
    sla_deadline_at: Option<DateTime<Utc>>,
    sla_remaining_minutes: i64,
    has_active_ticket: bool,
}

pub(super) fn normalize_exception_ticket_status(raw: &str) -> String {
    match raw.trim().to_uppercase().as_str() {
        "REJECTED" => "CLOSED".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn exception_ticket_status_rank(raw: &str) -> i32 {
    match normalize_exception_ticket_status(raw).as_str() {
        "ESCALATED" => 60,
        "PROCESSING" => 50,
        "ASSIGNED" => 40,
        "OPEN" => 30,
        "RESOLVED" => 20,
        "CLOSED" => 10,
        _ => 0,
    }
}

pub(super) fn is_active_exception_ticket_status(raw: &str) -> bool {
    matches!(
        normalize_exception_ticket_status(raw).as_str(),
        "OPEN" | "ASSIGNED" | "PROCESSING" | "ESCALATED"
    )
}

pub(super) fn exception_sla_minutes_for_reason(reason_code: &str) -> i64 {
    match reason_code.trim().to_uppercase().as_str() {
        "ADDRESS_INVALID"
        | "ADDRESS_ERROR"
        | "PAYMENT_ISSUE"
        | "CUSTOMER_BLACKLIST"
        | "BLACKLISTED_CUSTOMER"
        | "MANUAL_HOLD" => OMS_EXCEPTION_SLA_HIGH_PRIORITY_MINUTES,
        "MAPPING_MISSING" | "OUT_OF_STOCK" | "INVENTORY_SHORTAGE" => {
            OMS_EXCEPTION_SLA_MEDIUM_PRIORITY_MINUTES
        }
        _ => OMS_EXCEPTION_SLA_LOW_PRIORITY_MINUTES,
    }
}

pub(super) fn derive_order_exception_audit_status(
    review_status: OrderReviewStatus,
    ticket_status: &str,
    latest_log: Option<&OrderExceptionLogSnapshot>,
) -> String {
    if let Some(value) = latest_log.and_then(|log| log.audit_status.clone()) {
        return value;
    }
    match normalize_exception_ticket_status(ticket_status).as_str() {
        "PROCESSING" | "ESCALATED" => "UNDER_REVIEW".to_string(),
        "ASSIGNED" | "OPEN" => "REVIEW_REQUIRED".to_string(),
        "RESOLVED" => "REVIEW_PASSED".to_string(),
        "CLOSED" => {
            if review_status == OrderReviewStatus::Rejected {
                "REVIEW_REJECTED".to_string()
            } else {
                "PENDING_AUDIT".to_string()
            }
        }
        _ => {
            if review_status.is_approved() {
                "REVIEW_PASSED".to_string()
            } else {
                "REVIEW_REQUIRED".to_string()
            }
        }
    }
}

pub(super) fn select_order_exception_tickets(
    ticket_rows: &[OrderExceptionTicketRow],
    fallback_reason_codes: &[String],
) -> OrderExceptionTicketSelection {
    let reason_codes = if ticket_rows.is_empty() {
        dedup_string_values(fallback_reason_codes.iter().cloned())
    } else {
        dedup_string_values(ticket_rows.iter().map(|row| row.reason_code.clone()))
    };
    let blocking_reasons = if reason_codes.is_empty() {
        Vec::new()
    } else {
        dedup_string_values(reason_codes.iter().cloned())
    };
    let primary_ticket = ticket_rows.iter().max_by(|left, right| {
        exception_ticket_status_rank(&left.status)
            .cmp(&exception_ticket_status_rank(&right.status))
            .then_with(|| left.updated_at.cmp(&right.updated_at))
            .then_with(|| left.created_at.cmp(&right.created_at))
    });

    let status = primary_ticket
        .map(|ticket| normalize_exception_ticket_status(&ticket.status))
        .unwrap_or_else(|| {
            if reason_codes.is_empty() {
                "CLOSED".to_string()
            } else {
                "OPEN".to_string()
            }
        });
    let latest_action_at = ticket_rows.iter().map(|row| row.updated_at).max();
    let has_active_ticket = ticket_rows
        .iter()
        .any(|row| is_active_exception_ticket_status(&row.status));
    let deadline_candidates = ticket_rows
        .iter()
        .filter(|row| has_active_ticket == is_active_exception_ticket_status(&row.status))
        .map(|row| {
            row.created_at
                + chrono::Duration::minutes(exception_sla_minutes_for_reason(&row.reason_code))
        })
        .collect::<Vec<_>>();
    let sla_deadline_at = deadline_candidates.into_iter().min();
    let sla_remaining_minutes = sla_deadline_at
        .map(|deadline_at| {
            let remaining = (deadline_at - Utc::now()).num_minutes();
            if has_active_ticket {
                remaining
            } else {
                remaining.max(0)
            }
        })
        .unwrap_or(0);

    OrderExceptionTicketSelection {
        status,
        handler_id: primary_ticket
            .and_then(|ticket| ticket.handler_id.map(|value| value.to_string())),
        reason_codes,
        blocking_reasons,
        latest_action_at,
        sla_deadline_at,
        sla_remaining_minutes,
        has_active_ticket,
    }
}

pub(super) fn derive_order_exception_sla(
    selection: &OrderExceptionTicketSelection,
) -> (String, i64) {
    if !selection.has_active_ticket {
        return ("NORMAL".to_string(), selection.sla_remaining_minutes.max(0));
    }
    if selection.sla_remaining_minutes < 0 {
        return ("OVERDUE".to_string(), selection.sla_remaining_minutes);
    }
    if selection.sla_remaining_minutes <= OMS_EXCEPTION_SLA_DUE_SOON_MINUTES {
        return ("DUE_SOON".to_string(), selection.sla_remaining_minutes);
    }
    ("NORMAL".to_string(), selection.sla_remaining_minutes)
}

pub(super) async fn load_order_exception_workbench_order(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Option<OrderExceptionWorkbenchOrderRow>> {
    sqlx::query_as(
        r#"
        select
            o.order_id,
            o.platform,
            o.shop_id,
            o.platform_order_id,
            o.receiver_name,
            o.total_amount_cent,
            o.review_status,
            o.fulfillment_status,
            o.ship_warehouse_id,
            o.exception_reason,
            o.exception_flags
        from orders o
        join order_index i
          on i.tenant_id = o.tenant_id
         and i.order_id = o.order_id
         and i.order_month = o.order_month
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

pub(super) async fn load_order_exception_ticket_rows(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Vec<OrderExceptionTicketRow>> {
    sqlx::query_as(
        r#"
        select
            reason_code,
            status,
            handler_id,
            created_at,
            updated_at
        from order_exception_tickets
        where tenant_id = $1
          and order_id = $2
        order by updated_at desc, created_at desc, reason_code asc
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(state.db.pool())
    .await
    .map_err(Into::into)
}

pub(super) async fn load_order_exception_logs(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Vec<OrderExceptionLogSnapshot>> {
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
                limit: OMS_EXCEPTION_LOG_LIMIT,
            },
        )
        .await?;

    Ok(result
        .items
        .into_iter()
        .filter(|log| log.action.starts_with("OMS.EXCEPTION_"))
        .map(|log| {
            let action = log
                .action
                .strip_prefix("OMS.EXCEPTION_")
                .unwrap_or(log.action.as_str())
                .to_string();
            let operator_id = log
                .metadata
                .get("operator_id")
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .or_else(|| log.actor_user_id.map(|value| value.to_string()));
            OrderExceptionLogSnapshot {
                action: action.clone(),
                message: log
                    .metadata
                    .get("message")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .unwrap_or(action),
                operator_id,
                operator_name: log
                    .metadata
                    .get("operator_name")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                assignee_id: log
                    .metadata
                    .get("assignee_id")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                assignee_name: log
                    .metadata
                    .get("assignee_name")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                audit_status: log
                    .metadata
                    .get("audit_status")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                blocking_reason: log
                    .metadata
                    .get("blocking_reason")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
                    .or_else(|| {
                        log.metadata
                            .get("reason")
                            .and_then(|value| value.as_str())
                            .map(str::to_string)
                    }),
                created_at: log.created_at,
            }
        })
        .collect())
}

pub(super) async fn load_order_detail_response(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Option<OmsOrderDetailResponse>> {
    let orders_repo = OrdersRepo::new(state.db.pool());
    let Some(detail) = orders_repo.get_order_detail(tenant_id, order_id).await? else {
        return Ok(None);
    };
    let audit_repo = AuditLogsRepo::new(state.db.pool());
    let audit_logs = audit_repo
        .list_logs(
            tenant_id,
            erp_repo::audit_logs::AuditLogListFilters {
                action: None,
                entity_type: Some("ORDER".to_string()),
                entity_id: Some(order_id.to_string()),
                user_id: None,
                created_from: None,
                created_to: None,
                cursor: None,
                limit: 100,
            },
        )
        .await?
        .items;

    Ok(Some(OmsOrderDetailResponse {
        order: detail.order,
        items: detail.items,
        review_hits: detail.review_hits,
        tags: detail.tags,
        notes: detail.notes,
        audit_logs,
    }))
}

pub(super) async fn build_order_exception_summary_response(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Option<OrderExceptionSummaryResponse>> {
    let Some(order_row) = load_order_exception_workbench_order(state, tenant_id, order_id).await?
    else {
        return Ok(None);
    };
    let ticket_rows = load_order_exception_ticket_rows(state, tenant_id, order_id).await?;
    let fallback_reason_codes = json_array_to_strings(&order_row.exception_flags);
    if ticket_rows.is_empty() && fallback_reason_codes.is_empty() {
        return Ok(None);
    }
    let selection = select_order_exception_tickets(&ticket_rows, &fallback_reason_codes);
    let logs = load_order_exception_logs(state, tenant_id, order_id).await?;
    let latest_log = logs.first();
    let assignee_log = logs
        .iter()
        .find(|log| log.assignee_id.is_some() || log.assignee_name.is_some());
    let review_status = OrderReviewStatus::from_str(&order_row.review_status)
        .unwrap_or(OrderReviewStatus::ManualReview);
    let audit_status =
        derive_order_exception_audit_status(review_status, &selection.status, latest_log);
    let (sla_status, sla_remaining_minutes) = derive_order_exception_sla(&selection);
    let blocking_reason = latest_log
        .and_then(|log| log.blocking_reason.clone())
        .or_else(|| trim_to_option(order_row.exception_reason.as_deref()))
        .or_else(|| selection.blocking_reasons.first().cloned())
        .unwrap_or_else(|| "待处理异常".to_string());

    Ok(Some(OrderExceptionSummaryResponse {
        order_id: order_row.order_id.to_string(),
        order_no: order_row.platform_order_id,
        platform: order_row.platform,
        shop_id: order_row.shop_id.to_string(),
        receiver_name: order_row.receiver_name,
        total_amount_cent: order_row.total_amount_cent,
        audit_status,
        exception_status: selection.status,
        fulfillment_status: order_row.fulfillment_status,
        reserve_status: None,
        target_warehouse_id: order_row.ship_warehouse_id.map(|value| value.to_string()),
        blocking_reason,
        assignee_id: assignee_log
            .and_then(|log| log.assignee_id.clone())
            .or(selection.handler_id),
        assignee_name: assignee_log.and_then(|log| log.assignee_name.clone()),
        latest_action: latest_log.map(|log| log.message.clone()),
        latest_action_at: latest_log
            .map(|log| log.created_at)
            .or(selection.latest_action_at),
        sla_deadline_at: selection.sla_deadline_at,
        sla_status,
        sla_remaining_minutes,
    }))
}

pub(super) async fn build_order_exception_detail_response(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Option<OrderExceptionDetailResponse>> {
    let Some(summary) = build_order_exception_summary_response(state, tenant_id, order_id).await?
    else {
        return Ok(None);
    };
    let Some(order) = load_order_detail_response(state, tenant_id, order_id).await? else {
        return Ok(None);
    };
    let Some(order_row) = load_order_exception_workbench_order(state, tenant_id, order_id).await?
    else {
        return Ok(None);
    };
    let ticket_rows = load_order_exception_ticket_rows(state, tenant_id, order_id).await?;
    let selection = select_order_exception_tickets(
        &ticket_rows,
        &json_array_to_strings(&order_row.exception_flags),
    );
    let action_logs = load_order_exception_logs(state, tenant_id, order_id)
        .await?
        .into_iter()
        .rev()
        .map(|log| OrderExceptionActionRecordResponse {
            action: log.action,
            message: log.message,
            operator_id: log.operator_id,
            operator_name: log.operator_name,
            created_at: log.created_at,
        })
        .collect::<Vec<_>>();
    let mut blocking_reasons = selection.blocking_reasons;
    if let Some(reason) = trim_to_option(order_row.exception_reason.as_deref()) {
        if !blocking_reasons.iter().any(|item| item == &reason) {
            blocking_reasons.insert(0, reason);
        }
    }

    Ok(Some(OrderExceptionDetailResponse {
        order_id: summary.order_id,
        order_no: summary.order_no,
        platform: summary.platform,
        shop_id: summary.shop_id,
        receiver_name: summary.receiver_name,
        total_amount_cent: summary.total_amount_cent,
        audit_status: summary.audit_status,
        exception_status: summary.exception_status,
        fulfillment_status: summary.fulfillment_status,
        reserve_status: summary.reserve_status,
        target_warehouse_id: summary.target_warehouse_id,
        blocking_reason: summary.blocking_reason,
        assignee_id: summary.assignee_id,
        assignee_name: summary.assignee_name,
        latest_action: summary.latest_action,
        latest_action_at: summary.latest_action_at,
        sla_deadline_at: summary.sla_deadline_at,
        sla_status: summary.sla_status,
        sla_remaining_minutes: summary.sla_remaining_minutes,
        blocking_reasons,
        action_logs,
        order,
    }))
}

pub(super) async fn list_order_exceptions(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<OrderExceptionListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        order_id: uuid::Uuid,
    }

    let rows: Vec<Row> = match sqlx::query_as(
        r#"
        select order_id
        from order_exception_tickets
        where tenant_id = $1
        group by order_id
        order by max(updated_at) desc, order_id desc
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_all(state.db.pool())
    .await
    {
        Ok(value) => value,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
        }
    };

    let status_filter = query
        .status
        .as_deref()
        .map(|value| value.trim().to_uppercase());
    let assignee_filter = query
        .assignee_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let keyword_filter = query
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase());
    let offset = query.offset.unwrap_or(0).max(0) as usize;
    let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;

    let mut items = Vec::new();
    for row in rows {
        let order_id = OrderId(row.order_id);
        let Some(item) =
            (match build_order_exception_summary_response(&state, tenant_id, order_id).await {
                Ok(value) => value,
                Err(error) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
                }
            })
        else {
            continue;
        };

        let shop_id = match ShopId::from_str(&item.shop_id) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if ensure_shop_access(&ctx, shop_id).is_err() {
            continue;
        }
        if let Some(status) = status_filter.as_deref() {
            if item.exception_status.to_uppercase() != status {
                continue;
            }
        }
        if let Some(assignee_id) = assignee_filter.as_deref() {
            if item.assignee_id.as_deref() != Some(assignee_id) {
                continue;
            }
        }
        if let Some(keyword) = keyword_filter.as_deref() {
            let receiver_name = item
                .receiver_name
                .as_deref()
                .unwrap_or_default()
                .to_lowercase();
            let blocking_reason = item.blocking_reason.to_lowercase();
            if !item.order_id.to_lowercase().contains(keyword)
                && !item.order_no.to_lowercase().contains(keyword)
                && !receiver_name.contains(keyword)
                && !blocking_reason.contains(keyword)
            {
                continue;
            }
        }
        items.push(item);
    }

    let total = items.len() as i64;
    let paged = items
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let response = OrderExceptionListResponse {
        items: paged,
        total,
    };
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn get_order_exception_detail(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.read") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };

    let Some(summary) =
        (match build_order_exception_summary_response(&state, tenant_id, order_id).await {
            Ok(value) => value,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response()
            }
        })
    else {
        return not_found("order exception not found").into_response();
    };
    let shop_id = match ShopId::from_str(&summary.shop_id) {
        Ok(value) => value,
        Err(_) => return not_found("order exception not found").into_response(),
    };
    if let Err(resp) = ensure_shop_access(&ctx, shop_id) {
        return resp.into_response();
    }

    match build_order_exception_detail_response(&state, tenant_id, order_id).await {
        Ok(Some(value)) => (StatusCode::OK, Json(value)).into_response(),
        Ok(None) => not_found("order exception not found").into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response(),
    }
}

pub(super) async fn record_order_exception_workbench_log(
    state: &AppState,
    tenant_id: TenantId,
    order_id: OrderId,
    action_code: &str,
    actor_user_id: Option<UserId>,
    operator_name: Option<String>,
    assignee_id: Option<UserId>,
    assignee_name: Option<String>,
    audit_status: &str,
    blocking_reason: Option<String>,
    message: String,
) {
    let repo = OperationLogsRepo::new(state.db.pool());
    let _ = repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id,
            action: action_code.to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_id.to_string()),
            metadata: Some(serde_json::json!({
                "message": message,
                "operator_id": actor_user_id.map(|value| value.to_string()),
                "operator_name": operator_name,
                "assignee_id": assignee_id.map(|value| value.to_string()),
                "assignee_name": assignee_name,
                "audit_status": audit_status,
                "blocking_reason": blocking_reason,
            })),
        })
        .await;
}

pub(super) async fn execute_order_exception_workbench_action(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    tenant_id: TenantId,
    order_id: OrderId,
    action: &str,
    payload: OrderExceptionActionRequest,
) -> Result<OrderExceptionSummaryResponse, Response> {
    let Some(current_summary) = build_order_exception_summary_response(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?
    else {
        return Err(not_found("order exception not found").into_response());
    };
    let shop_id = ShopId::from_str(&current_summary.shop_id)
        .map_err(|_| not_found("order exception not found").into_response())?;
    if let Err(resp) = ensure_shop_access(ctx, shop_id) {
        return Err(resp.into_response());
    }

    let ticket_rows = load_order_exception_ticket_rows(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;
    if ticket_rows.is_empty() {
        return Err(not_found("order exception not found").into_response());
    }
    let current_selection = select_order_exception_tickets(&ticket_rows, &[]);
    let current_status = current_selection.status.clone();
    let reason_codes = current_selection.reason_codes.clone();
    if reason_codes.is_empty() {
        return Err(not_found("order exception not found").into_response());
    }

    let operator_id = parse_optional_user_id(payload.operator_id.as_deref(), "operator_id")?
        .unwrap_or(ctx.user_id);
    let operator_name = trim_to_option(payload.operator_name.as_deref());
    let requested_assignee_id =
        parse_optional_user_id(payload.assignee_id.as_deref(), "assignee_id")?;
    let requested_assignee_name = trim_to_option(payload.assignee_name.as_deref());
    let reason = trim_to_option(payload.reason.as_deref());

    let orders_repo = OrdersRepo::new(state.db.pool());
    let now = Utc::now();
    let (ticket_status, handler_id, handled_at, audit_status, message) = match action {
        "CLAIM" => {
            if !matches!(current_status.as_str(), "OPEN" | "ASSIGNED") {
                return Err(
                    conflict("exception cannot be claimed in current status").into_response()
                );
            }
            (
                "PROCESSING".to_string(),
                Some(operator_id),
                Some(now),
                "UNDER_REVIEW".to_string(),
                "异常工单已领取".to_string(),
            )
        }
        "ASSIGN" => {
            if !matches!(
                current_status.as_str(),
                "OPEN" | "ASSIGNED" | "PROCESSING" | "ESCALATED"
            ) {
                return Err(
                    conflict("exception cannot be assigned in current status").into_response()
                );
            }
            let assignee_id = requested_assignee_id
                .ok_or_else(|| bad_request("assignee_id is required").into_response())?;
            let assignee_name = requested_assignee_name
                .clone()
                .or_else(|| Some(assignee_id.to_string()))
                .unwrap_or_else(|| assignee_id.to_string());
            (
                "ASSIGNED".to_string(),
                Some(assignee_id),
                Some(now),
                "REVIEW_REQUIRED".to_string(),
                format!("异常工单已分派给 {assignee_name}"),
            )
        }
        "ESCALATE" => {
            if !matches!(current_status.as_str(), "ASSIGNED" | "PROCESSING") {
                return Err(
                    conflict("exception cannot be escalated in current status").into_response()
                );
            }
            (
                "ESCALATED".to_string(),
                requested_assignee_id
                    .or_else(|| {
                        parse_optional_user_id(
                            current_summary.assignee_id.as_deref(),
                            "assignee_id",
                        )
                        .ok()
                        .flatten()
                    })
                    .or(Some(operator_id)),
                Some(now),
                "UNDER_REVIEW".to_string(),
                reason
                    .clone()
                    .unwrap_or_else(|| "异常工单已升级处理".to_string()),
            )
        }
        "HOLD" => {
            apply_order_exception_action(
                state,
                ctx,
                req_ctx,
                tenant_id,
                order_id,
                "REOPEN".to_string(),
                Some(reason_codes.clone()),
                reason.clone(),
            )
            .await?;
            (
                "OPEN".to_string(),
                requested_assignee_id
                    .or_else(|| {
                        parse_optional_user_id(
                            current_summary.assignee_id.as_deref(),
                            "assignee_id",
                        )
                        .ok()
                        .flatten()
                    })
                    .or(Some(operator_id)),
                None,
                "ON_HOLD".to_string(),
                reason
                    .clone()
                    .unwrap_or_else(|| "异常挂起处理中".to_string()),
            )
        }
        "RESOLVE" => {
            apply_order_exception_action(
                state,
                ctx,
                req_ctx,
                tenant_id,
                order_id,
                "RESOLVE".to_string(),
                Some(reason_codes.clone()),
                reason.clone(),
            )
            .await?;
            (
                "RESOLVED".to_string(),
                Some(operator_id),
                Some(now),
                "REVIEW_PASSED".to_string(),
                reason.clone().unwrap_or_else(|| "异常已解决".to_string()),
            )
        }
        "REJECT" => {
            apply_order_exception_action(
                state,
                ctx,
                req_ctx,
                tenant_id,
                order_id,
                "REJECT".to_string(),
                Some(reason_codes.clone()),
                reason.clone(),
            )
            .await?;
            (
                "CLOSED".to_string(),
                Some(operator_id),
                Some(now),
                "REVIEW_REJECTED".to_string(),
                reason.clone().unwrap_or_else(|| "订单已驳回".to_string()),
            )
        }
        _ => return Err(bad_request("bad action").into_response()),
    };

    let updates = reason_codes
        .iter()
        .map(|reason_code| OrderExceptionTicketUpdate {
            reason_code: reason_code.clone(),
            status: ticket_status.clone(),
            handler_id,
            handled_at,
        })
        .collect::<Vec<_>>();
    orders_repo
        .update_exception_ticket_status(tenant_id, order_id, &updates)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?;

    let effective_assignee_id = match action {
        "ASSIGN" => requested_assignee_id,
        "ESCALATE" | "HOLD" => requested_assignee_id
            .or_else(|| {
                parse_optional_user_id(current_summary.assignee_id.as_deref(), "assignee_id")
                    .ok()
                    .flatten()
            })
            .or(handler_id),
        _ => handler_id,
    };
    let effective_assignee_name = match action {
        "CLAIM" => operator_name.clone(),
        "ASSIGN" => requested_assignee_name
            .clone()
            .or_else(|| requested_assignee_id.map(|value| value.to_string())),
        _ => requested_assignee_name
            .clone()
            .or_else(|| current_summary.assignee_name.clone()),
    };

    record_order_exception_workbench_log(
        state,
        tenant_id,
        order_id,
        match action {
            "CLAIM" => OMS_EXCEPTION_ACTION_CLAIM,
            "ASSIGN" => OMS_EXCEPTION_ACTION_ASSIGN,
            "ESCALATE" => OMS_EXCEPTION_ACTION_ESCALATE,
            "HOLD" => OMS_EXCEPTION_ACTION_HOLD,
            "RESOLVE" => OMS_EXCEPTION_ACTION_RESOLVE,
            "REJECT" => OMS_EXCEPTION_ACTION_REJECT,
            _ => unreachable!(),
        },
        Some(operator_id),
        operator_name.clone(),
        effective_assignee_id,
        effective_assignee_name,
        &audit_status,
        reason
            .clone()
            .or_else(|| Some(current_summary.blocking_reason.clone())),
        message,
    )
    .await;

    build_order_exception_summary_response(state, tenant_id, order_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, format!("{error}")).into_response())?
        .ok_or_else(|| not_found("order exception not found").into_response())
}

pub(super) async fn claim_order_exception(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderExceptionActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_exception_workbench_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "CLAIM", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn assign_order_exception(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderExceptionActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_exception_workbench_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "ASSIGN", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn escalate_order_exception(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderExceptionActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_exception_workbench_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "ESCALATE", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn resolve_order_exception(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderExceptionActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_exception_workbench_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "RESOLVE", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn hold_order_exception(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderExceptionActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_exception_workbench_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "HOLD", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub(super) async fn reject_order_exception(
    State(state): State<AppState>,
    Path((tenant_id, order_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(payload): Json<OrderExceptionActionRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "orders.write") {
        return resp.into_response();
    }
    let order_id = match OrderId::from_str(&order_id) {
        Ok(v) => v,
        Err(_) => return bad_request("bad order_id").into_response(),
    };
    match execute_order_exception_workbench_action(
        &state, &ctx, &req_ctx, tenant_id, order_id, "REJECT", payload,
    )
    .await
    {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}
