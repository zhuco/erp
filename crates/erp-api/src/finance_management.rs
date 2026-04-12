use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

use erp_repo::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    finance_ap_plans::{
        ApPaymentPlan, ApPaymentPlanCreate, ApPaymentPlanListFilters, ApPaymentPlanUpdate,
        ApPaymentPlansRepo,
    },
    finance_ar::{ArDoc, ArDocCreate, ArDocListFilters, ArDocUpdate, ArDocsRepo},
    finance_consignment::{
        ConsignmentSettlement, ConsignmentSettlementCreate, ConsignmentSettlementListFilters,
        ConsignmentSettlementUpdate, ConsignmentSettlementsRepo,
    },
    finance_costing::{
        InventoryCostingRun, InventoryCostingRunCreate, InventoryCostingRunListFilters,
        InventoryCostingRunUpdate, InventoryCostingRunsRepo,
    },
    finance_expenses::{
        FinanceExpense, FinanceExpenseCreate, FinanceExpenseListFilters, FinanceExpenseUpdate,
        FinanceExpensesRepo,
    },
    finance_tax::{
        InvoiceAdjustment, InvoiceAdjustmentCreate, InvoiceAdjustmentListFilters,
        InvoiceAdjustmentsRepo, TaxRule, TaxRuleCreate, TaxRuleListFilters, TaxRuleUpdate,
        TaxRulesRepo,
    },
    procurement::{
        ApDoc, ApDocsRepo, AP_SETTLEMENT_STATUS_SETTLED, AP_SETTLEMENT_STATUS_SETTLING,
        AP_SETTLEMENT_STATUS_UNSETTLED, AP_STATUS_PAID,
    },
};
use erp_types::TenantId;

use crate::{
    bad_request, ensure_permission, ensure_tenant_access, not_found, AppState, AuthContext,
    RequestContext,
};

#[derive(Deserialize)]
pub(crate) struct ArDocListQuery {
    keyword: Option<String>,
    status: Option<String>,
    customer_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct ArDocListResponse {
    items: Vec<ArDoc>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct ArDocCreateRequest {
    customer_id: Option<String>,
    ref_type: String,
    ref_id: String,
    amount_cent: i64,
    currency: Option<String>,
    status: Option<String>,
    due_date: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub(crate) struct ArDocUpdateRequest {
    customer_id: Option<String>,
    ref_type: Option<String>,
    ref_id: Option<String>,
    amount_cent: Option<i64>,
    currency: Option<String>,
    status: Option<String>,
    due_date: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub(crate) struct ApPlanListQuery {
    status: Option<String>,
    supplier_id: Option<String>,
    ap_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct ApPlanListResponse {
    items: Vec<ApPaymentPlan>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct ApPlanCreateRequest {
    ap_id: String,
    supplier_id: Option<String>,
    planned_date: Option<NaiveDate>,
    amount_cent: i64,
    status: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ApPlanUpdateRequest {
    planned_date: Option<NaiveDate>,
    amount_cent: Option<i64>,
    status: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ExpenseListQuery {
    keyword: Option<String>,
    status: Option<String>,
    category: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct ExpenseListResponse {
    items: Vec<FinanceExpense>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct ExpenseCreateRequest {
    name: String,
    category: String,
    amount_cent: i64,
    currency: Option<String>,
    allocation_rule: Option<Value>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ExpenseUpdateRequest {
    name: Option<String>,
    category: Option<String>,
    amount_cent: Option<i64>,
    currency: Option<String>,
    allocation_rule: Option<Value>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct CostingRunListQuery {
    status: Option<String>,
    method: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct CostingRunListResponse {
    items: Vec<InventoryCostingRun>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct CostingRunCreateRequest {
    method: String,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    status: Option<String>,
    variance_cent: Option<i64>,
    result_json: Option<Value>,
}

#[derive(Deserialize)]
pub(crate) struct CostingRunUpdateRequest {
    method: Option<String>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    status: Option<String>,
    variance_cent: Option<i64>,
    result_json: Option<Value>,
}

#[derive(Deserialize)]
pub(crate) struct ConsignmentListQuery {
    keyword: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct ConsignmentListResponse {
    items: Vec<ConsignmentSettlement>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct ConsignmentCreateRequest {
    partner_name: String,
    agreement_no: Option<String>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    sales_amount_cent: i64,
    commission_rate: f64,
    commission_cent: i64,
    status: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ConsignmentUpdateRequest {
    partner_name: Option<String>,
    agreement_no: Option<String>,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    sales_amount_cent: Option<i64>,
    commission_rate: Option<f64>,
    commission_cent: Option<i64>,
    status: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct TaxRuleListQuery {
    tax_code: Option<String>,
    status: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct TaxRuleListResponse {
    items: Vec<TaxRule>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct TaxRuleCreateRequest {
    tax_code: String,
    tax_rate: f64,
    effective_from: Option<NaiveDate>,
    effective_to: Option<NaiveDate>,
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct TaxRuleUpdateRequest {
    tax_code: Option<String>,
    tax_rate: Option<f64>,
    effective_from: Option<NaiveDate>,
    effective_to: Option<NaiveDate>,
    status: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct InvoiceAdjustmentListQuery {
    status: Option<String>,
    invoice_id: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct InvoiceAdjustmentListResponse {
    items: Vec<InvoiceAdjustment>,
    total: i64,
}

#[derive(Deserialize)]
pub(crate) struct InvoiceAdjustmentCreateRequest {
    invoice_id: String,
    adjustment_type: String,
    reason: Option<String>,
    status: Option<String>,
}

fn parse_uuid_optional(
    raw: Option<String>,
) -> Result<Option<uuid::Uuid>, (StatusCode, Json<crate::ErrorResponse>)> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    match uuid::Uuid::parse_str(raw) {
        Ok(value) => Ok(Some(value)),
        Err(_) => Err(bad_request("bad uuid")),
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(crate) async fn list_ar_docs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ArDocListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }

    let customer_id = match parse_uuid_optional(query.customer_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = ArDocsRepo::new(state.db.pool());
    let result = match repo
        .list_docs(
            tenant_id,
            ArDocListFilters {
                keyword: normalize_optional_text(query.keyword),
                status: normalize_optional_text(query.status),
                customer_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(ArDocListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn get_ar_doc(
    State(state): State<AppState>,
    Path((tenant_id, ar_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let ar_id = match uuid::Uuid::parse_str(&ar_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad ar_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }

    let repo = ArDocsRepo::new(state.db.pool());
    let doc = match repo.get_doc(tenant_id, ar_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("ar doc not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (StatusCode::OK, Json(doc)).into_response()
}

pub(crate) async fn create_ar_doc(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ArDocCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let customer_id = match parse_uuid_optional(req.customer_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let ref_type = req.ref_type.trim();
    let ref_id = req.ref_id.trim();
    if ref_type.is_empty() || ref_id.is_empty() {
        return bad_request("ref_type and ref_id required").into_response();
    }

    let repo = ArDocsRepo::new(state.db.pool());
    let doc = match repo
        .create_doc(ArDocCreate {
            tenant_id,
            customer_id,
            ref_type: ref_type.to_string(),
            ref_id: ref_id.to_string(),
            amount_cent: req.amount_cent,
            currency: req.currency.unwrap_or_else(|| "CNY".to_string()),
            status: req.status.unwrap_or_else(|| "OPEN".to_string()),
            due_date: req.due_date,
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.AR.CREATE",
        "ar_doc",
        doc.ar_id.to_string(),
        None,
        Some(serde_json::to_value(&doc).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(doc)).into_response()
}

pub(crate) async fn update_ar_doc(
    State(state): State<AppState>,
    Path((tenant_id, ar_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ArDocUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let ar_id = match uuid::Uuid::parse_str(&ar_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad ar_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let customer_id = match parse_uuid_optional(req.customer_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = ArDocsRepo::new(state.db.pool());
    let before = match repo.get_doc(tenant_id, ar_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("ar doc not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    let updated = match repo
        .update_doc(
            tenant_id,
            ar_id,
            ArDocUpdate {
                customer_id,
                ref_type: normalize_optional_text(req.ref_type),
                ref_id: normalize_optional_text(req.ref_id),
                amount_cent: req.amount_cent,
                currency: normalize_optional_text(req.currency),
                status: normalize_optional_text(req.status),
                due_date: req.due_date,
                written_off_at: None,
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("ar doc not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.AR.UPDATE",
        "ar_doc",
        updated.ar_id.to_string(),
        Some(serde_json::to_value(&before).unwrap_or(Value::Null)),
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn delete_ar_doc(
    State(state): State<AppState>,
    Path((tenant_id, ar_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let ar_id = match uuid::Uuid::parse_str(&ar_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad ar_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = ArDocsRepo::new(state.db.pool());
    let deleted = match repo.delete_doc(tenant_id, ar_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("ar doc not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.AR.DELETE",
        "ar_doc",
        deleted.ar_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(crate) async fn write_off_ar_doc(
    State(state): State<AppState>,
    Path((tenant_id, ar_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let ar_id = match uuid::Uuid::parse_str(&ar_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad ar_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = ArDocsRepo::new(state.db.pool());
    let updated = match repo.mark_written_off(tenant_id, ar_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("ar doc not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.AR.WRITE_OFF",
        "ar_doc",
        updated.ar_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn list_ap_plans(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ApPlanListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }

    let supplier_id = match parse_uuid_optional(query.supplier_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let ap_id = match parse_uuid_optional(query.ap_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let repo = ApPaymentPlansRepo::new(state.db.pool());
    let result = match repo
        .list_plans(
            tenant_id,
            ApPaymentPlanListFilters {
                status: normalize_optional_text(query.status),
                supplier_id,
                ap_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(ApPlanListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_ap_plan(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ApPlanCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let ap_id = match uuid::Uuid::parse_str(&req.ap_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad ap_id").into_response(),
    };
    let supplier_id = match parse_uuid_optional(req.supplier_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };

    let repo = ApPaymentPlansRepo::new(state.db.pool());
    let plan = match repo
        .create_plan(ApPaymentPlanCreate {
            tenant_id,
            ap_id,
            supplier_id,
            planned_date: req.planned_date,
            amount_cent: req.amount_cent,
            status: req.status.unwrap_or_else(|| "PLANNED".to_string()),
            notes: normalize_optional_text(req.notes),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if let Err(err) = sync_ap_doc_settlement_status(&state, tenant_id, ap_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.AP_PLAN.CREATE",
        "ap_payment_plan",
        plan.plan_id.to_string(),
        None,
        Some(serde_json::to_value(&plan).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(plan)).into_response()
}

pub(crate) async fn update_ap_plan(
    State(state): State<AppState>,
    Path((tenant_id, plan_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ApPlanUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let plan_id = match uuid::Uuid::parse_str(&plan_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad plan_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = ApPaymentPlansRepo::new(state.db.pool());
    let updated = match repo
        .update_plan(
            tenant_id,
            plan_id,
            ApPaymentPlanUpdate {
                planned_date: req.planned_date,
                amount_cent: req.amount_cent,
                status: normalize_optional_text(req.status),
                paid_at: None,
                notes: normalize_optional_text(req.notes),
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("ap plan not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if let Err(err) = sync_ap_doc_settlement_status(&state, tenant_id, updated.ap_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.AP_PLAN.UPDATE",
        "ap_payment_plan",
        updated.plan_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn delete_ap_plan(
    State(state): State<AppState>,
    Path((tenant_id, plan_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let plan_id = match uuid::Uuid::parse_str(&plan_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad plan_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = ApPaymentPlansRepo::new(state.db.pool());
    let deleted = match repo.delete_plan(tenant_id, plan_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("ap plan not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if let Err(err) = sync_ap_doc_settlement_status(&state, tenant_id, deleted.ap_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.AP_PLAN.DELETE",
        "ap_payment_plan",
        deleted.plan_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(crate) async fn mark_ap_plan_paid(
    State(state): State<AppState>,
    Path((tenant_id, plan_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let plan_id = match uuid::Uuid::parse_str(&plan_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad plan_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = ApPaymentPlansRepo::new(state.db.pool());
    let updated = match repo.mark_paid(tenant_id, plan_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("ap plan not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };
    if let Err(err) = sync_ap_doc_settlement_status(&state, tenant_id, updated.ap_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response();
    }

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.AP_PLAN.PAID",
        "ap_payment_plan",
        updated.plan_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn list_expenses(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ExpenseListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = FinanceExpensesRepo::new(state.db.pool());
    let result = match repo
        .list_expenses(
            tenant_id,
            FinanceExpenseListFilters {
                keyword: normalize_optional_text(query.keyword),
                status: normalize_optional_text(query.status),
                category: normalize_optional_text(query.category),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(ExpenseListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_expense(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ExpenseCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let name = req.name.trim();
    let category = req.category.trim();
    if name.is_empty() || category.is_empty() {
        return bad_request("name and category required").into_response();
    }

    let repo = FinanceExpensesRepo::new(state.db.pool());
    let expense = match repo
        .create_expense(FinanceExpenseCreate {
            tenant_id,
            name: name.to_string(),
            category: category.to_string(),
            amount_cent: req.amount_cent,
            currency: req.currency.unwrap_or_else(|| "CNY".to_string()),
            allocation_rule: req
                .allocation_rule
                .unwrap_or_else(|| Value::Object(Default::default())),
            period_start: req.period_start,
            period_end: req.period_end,
            status: req.status.unwrap_or_else(|| "DRAFT".to_string()),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.EXPENSE.CREATE",
        "finance_expense",
        expense.expense_id.to_string(),
        None,
        Some(serde_json::to_value(&expense).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(expense)).into_response()
}

pub(crate) async fn update_expense(
    State(state): State<AppState>,
    Path((tenant_id, expense_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ExpenseUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let expense_id = match uuid::Uuid::parse_str(&expense_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad expense_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = FinanceExpensesRepo::new(state.db.pool());
    let updated = match repo
        .update_expense(
            tenant_id,
            expense_id,
            FinanceExpenseUpdate {
                name: normalize_optional_text(req.name),
                category: normalize_optional_text(req.category),
                amount_cent: req.amount_cent,
                currency: normalize_optional_text(req.currency),
                allocation_rule: req.allocation_rule,
                period_start: req.period_start,
                period_end: req.period_end,
                status: normalize_optional_text(req.status),
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("expense not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.EXPENSE.UPDATE",
        "finance_expense",
        updated.expense_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn delete_expense(
    State(state): State<AppState>,
    Path((tenant_id, expense_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let expense_id = match uuid::Uuid::parse_str(&expense_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad expense_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = FinanceExpensesRepo::new(state.db.pool());
    let deleted = match repo.delete_expense(tenant_id, expense_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("expense not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.EXPENSE.DELETE",
        "finance_expense",
        deleted.expense_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(crate) async fn list_costing_runs(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<CostingRunListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = InventoryCostingRunsRepo::new(state.db.pool());
    let result = match repo
        .list_runs(
            tenant_id,
            InventoryCostingRunListFilters {
                status: normalize_optional_text(query.status),
                method: normalize_optional_text(query.method),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(CostingRunListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_costing_run(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CostingRunCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let method = req.method.trim();
    if method.is_empty() {
        return bad_request("method required").into_response();
    }

    let repo = InventoryCostingRunsRepo::new(state.db.pool());
    let run = match repo
        .create_run(InventoryCostingRunCreate {
            tenant_id,
            method: method.to_string(),
            period_start: req.period_start,
            period_end: req.period_end,
            status: req.status.unwrap_or_else(|| "DRAFT".to_string()),
            variance_cent: req.variance_cent.unwrap_or(0),
            result_json: req
                .result_json
                .unwrap_or_else(|| Value::Object(Default::default())),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.COSTING.CREATE",
        "inventory_costing_run",
        run.costing_id.to_string(),
        None,
        Some(serde_json::to_value(&run).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(run)).into_response()
}

pub(crate) async fn update_costing_run(
    State(state): State<AppState>,
    Path((tenant_id, costing_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<CostingRunUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let costing_id = match uuid::Uuid::parse_str(&costing_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad costing_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = InventoryCostingRunsRepo::new(state.db.pool());
    let updated = match repo
        .update_run(
            tenant_id,
            costing_id,
            InventoryCostingRunUpdate {
                method: normalize_optional_text(req.method),
                period_start: req.period_start,
                period_end: req.period_end,
                status: normalize_optional_text(req.status),
                variance_cent: req.variance_cent,
                result_json: req.result_json,
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("costing run not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.COSTING.UPDATE",
        "inventory_costing_run",
        updated.costing_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn list_consignments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<ConsignmentListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = ConsignmentSettlementsRepo::new(state.db.pool());
    let result = match repo
        .list_settlements(
            tenant_id,
            ConsignmentSettlementListFilters {
                keyword: normalize_optional_text(query.keyword),
                status: normalize_optional_text(query.status),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(ConsignmentListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_consignment(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ConsignmentCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let partner_name = req.partner_name.trim();
    if partner_name.is_empty() {
        return bad_request("partner_name required").into_response();
    }

    let repo = ConsignmentSettlementsRepo::new(state.db.pool());
    let settlement = match repo
        .create_settlement(ConsignmentSettlementCreate {
            tenant_id,
            partner_name: partner_name.to_string(),
            agreement_no: normalize_optional_text(req.agreement_no),
            period_start: req.period_start,
            period_end: req.period_end,
            sales_amount_cent: req.sales_amount_cent,
            commission_rate: req.commission_rate,
            commission_cent: req.commission_cent,
            status: req.status.unwrap_or_else(|| "DRAFT".to_string()),
            notes: normalize_optional_text(req.notes),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.CONSIGNMENT.CREATE",
        "consignment_settlement",
        settlement.settlement_id.to_string(),
        None,
        Some(serde_json::to_value(&settlement).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(settlement)).into_response()
}

pub(crate) async fn update_consignment(
    State(state): State<AppState>,
    Path((tenant_id, settlement_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<ConsignmentUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let settlement_id = match uuid::Uuid::parse_str(&settlement_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad settlement_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = ConsignmentSettlementsRepo::new(state.db.pool());
    let updated = match repo
        .update_settlement(
            tenant_id,
            settlement_id,
            ConsignmentSettlementUpdate {
                partner_name: normalize_optional_text(req.partner_name),
                agreement_no: normalize_optional_text(req.agreement_no),
                period_start: req.period_start,
                period_end: req.period_end,
                sales_amount_cent: req.sales_amount_cent,
                commission_rate: req.commission_rate,
                commission_cent: req.commission_cent,
                status: normalize_optional_text(req.status),
                settled_at: None,
                notes: normalize_optional_text(req.notes),
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("consignment not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.CONSIGNMENT.UPDATE",
        "consignment_settlement",
        updated.settlement_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn delete_consignment(
    State(state): State<AppState>,
    Path((tenant_id, settlement_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let settlement_id = match uuid::Uuid::parse_str(&settlement_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad settlement_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = ConsignmentSettlementsRepo::new(state.db.pool());
    let deleted = match repo.delete_settlement(tenant_id, settlement_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("consignment not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.CONSIGNMENT.DELETE",
        "consignment_settlement",
        deleted.settlement_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(crate) async fn mark_consignment_settled(
    State(state): State<AppState>,
    Path((tenant_id, settlement_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let settlement_id = match uuid::Uuid::parse_str(&settlement_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad settlement_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = ConsignmentSettlementsRepo::new(state.db.pool());
    let updated = match repo.mark_settled(tenant_id, settlement_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("consignment not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.CONSIGNMENT.SETTLED",
        "consignment_settlement",
        updated.settlement_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn list_tax_rules(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<TaxRuleListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = TaxRulesRepo::new(state.db.pool());
    let result = match repo
        .list_rules(
            tenant_id,
            TaxRuleListFilters {
                tax_code: normalize_optional_text(query.tax_code),
                status: normalize_optional_text(query.status),
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(TaxRuleListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_tax_rule(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<TaxRuleCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let tax_code = req.tax_code.trim();
    if tax_code.is_empty() {
        return bad_request("tax_code required").into_response();
    }

    let repo = TaxRulesRepo::new(state.db.pool());
    let rule = match repo
        .create_rule(TaxRuleCreate {
            tenant_id,
            tax_code: tax_code.to_string(),
            tax_rate: req.tax_rate,
            effective_from: req.effective_from,
            effective_to: req.effective_to,
            status: req.status.unwrap_or_else(|| "ACTIVE".to_string()),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.TAX_RULE.CREATE",
        "tax_rule",
        rule.rule_id.to_string(),
        None,
        Some(serde_json::to_value(&rule).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(rule)).into_response()
}

pub(crate) async fn update_tax_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<TaxRuleUpdateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = TaxRulesRepo::new(state.db.pool());
    let updated = match repo
        .update_rule(
            tenant_id,
            rule_id,
            TaxRuleUpdate {
                tax_code: normalize_optional_text(req.tax_code),
                tax_rate: req.tax_rate,
                effective_from: req.effective_from,
                effective_to: req.effective_to,
                status: normalize_optional_text(req.status),
            },
        )
        .await
    {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("tax rule not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.TAX_RULE.UPDATE",
        "tax_rule",
        updated.rule_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

pub(crate) async fn delete_tax_rule(
    State(state): State<AppState>,
    Path((tenant_id, rule_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let rule_id = match uuid::Uuid::parse_str(&rule_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad rule_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = TaxRulesRepo::new(state.db.pool());
    let deleted = match repo.delete_rule(tenant_id, rule_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("tax rule not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.TAX_RULE.DELETE",
        "tax_rule",
        deleted.rule_id.to_string(),
        Some(serde_json::to_value(&deleted).unwrap_or(Value::Null)),
        None,
    )
    .await;

    (StatusCode::OK, Json(deleted)).into_response()
}

pub(crate) async fn list_invoice_adjustments(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Query(query): Query<InvoiceAdjustmentListQuery>,
    Extension(ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.read") {
        return resp.into_response();
    }

    let invoice_id = match parse_uuid_optional(query.invoice_id) {
        Ok(value) => value,
        Err(resp) => return resp.into_response(),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let repo = InvoiceAdjustmentsRepo::new(state.db.pool());
    let result = match repo
        .list_adjustments(
            tenant_id,
            InvoiceAdjustmentListFilters {
                status: normalize_optional_text(query.status),
                invoice_id,
                offset,
                limit,
            },
        )
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    (
        StatusCode::OK,
        Json(InvoiceAdjustmentListResponse {
            items: result.items,
            total: result.total,
        }),
    )
        .into_response()
}

pub(crate) async fn create_invoice_adjustment(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
    Json(req): Json<InvoiceAdjustmentCreateRequest>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let invoice_id = match uuid::Uuid::parse_str(&req.invoice_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad invoice_id").into_response(),
    };
    let adjustment_type = req.adjustment_type.trim();
    if adjustment_type.is_empty() {
        return bad_request("adjustment_type required").into_response();
    }

    let repo = InvoiceAdjustmentsRepo::new(state.db.pool());
    let adjustment = match repo
        .create_adjustment(InvoiceAdjustmentCreate {
            tenant_id,
            invoice_id,
            adjustment_type: adjustment_type.to_string(),
            reason: normalize_optional_text(req.reason),
            status: req.status.unwrap_or_else(|| "PENDING".to_string()),
            created_by: Some(ctx.user_id.to_string()),
        })
        .await
    {
        Ok(value) => value,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.INVOICE_ADJUSTMENT.CREATE",
        "invoice_adjustment",
        adjustment.adjustment_id.to_string(),
        None,
        Some(serde_json::to_value(&adjustment).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(adjustment)).into_response()
}

pub(crate) async fn mark_invoice_adjustment_applied(
    State(state): State<AppState>,
    Path((tenant_id, adjustment_id)): Path<(String, String)>,
    Extension(ctx): Extension<AuthContext>,
    Extension(req_ctx): Extension<RequestContext>,
) -> impl IntoResponse {
    let tenant_id = match TenantId::from_str(&tenant_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad tenant_id").into_response(),
    };
    let adjustment_id = match uuid::Uuid::parse_str(&adjustment_id) {
        Ok(value) => value,
        Err(_) => return bad_request("bad adjustment_id").into_response(),
    };
    if let Err(resp) = ensure_tenant_access(&ctx, tenant_id) {
        return resp.into_response();
    }
    if let Err(resp) = ensure_permission(&ctx, "finance.write") {
        return resp.into_response();
    }

    let repo = InvoiceAdjustmentsRepo::new(state.db.pool());
    let updated = match repo.mark_applied(tenant_id, adjustment_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return not_found("invoice adjustment not found").into_response(),
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{err}")).into_response(),
    };

    record_finance_audit(
        &state,
        &ctx,
        &req_ctx,
        "FINANCE.INVOICE_ADJUSTMENT.APPLIED",
        "invoice_adjustment",
        updated.adjustment_id.to_string(),
        None,
        Some(serde_json::to_value(&updated).unwrap_or(Value::Null)),
    )
    .await;

    (StatusCode::OK, Json(updated)).into_response()
}

#[allow(clippy::too_many_arguments)]
async fn record_finance_audit(
    state: &AppState,
    ctx: &AuthContext,
    req_ctx: &RequestContext,
    action: &str,
    entity_type: &str,
    entity_id: String,
    before: Option<Value>,
    after: Option<Value>,
) {
    let repo = AuditLogsRepo::new(state.db.pool());
    let _ = repo
        .insert(AuditLogCreate {
            tenant_id: ctx.tenant_id,
            user_id: Some(ctx.user_id),
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: Some(entity_id),
            before_json: before,
            after_json: after,
            ip: req_ctx.ip.clone(),
            ua: req_ctx.user_agent.clone(),
        })
        .await;
}

async fn sync_ap_doc_settlement_status(
    state: &AppState,
    tenant_id: TenantId,
    ap_id: uuid::Uuid,
) -> anyhow::Result<Option<ApDoc>> {
    let repo = ApDocsRepo::new(state.db.pool());
    let Some(doc) = repo.get_doc(tenant_id, ap_id).await? else {
        return Ok(None);
    };

    if doc.status == AP_STATUS_PAID {
        if doc.settlement_status != AP_SETTLEMENT_STATUS_SETTLED {
            return repo
                .update_settlement_status(tenant_id, ap_id, AP_SETTLEMENT_STATUS_SETTLED)
                .await;
        }
        return Ok(Some(doc));
    }

    let total_plans: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from ap_payment_plans
        where tenant_id = $1 and ap_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(ap_id)
    .fetch_one(state.db.pool())
    .await?;

    if total_plans == 0 {
        if doc.settlement_status != AP_SETTLEMENT_STATUS_UNSETTLED {
            return repo
                .update_settlement_status(tenant_id, ap_id, AP_SETTLEMENT_STATUS_UNSETTLED)
                .await;
        }
        return Ok(Some(doc));
    }

    let paid_amount: i64 = sqlx::query_scalar(
        r#"
        select coalesce(sum(amount_cent), 0)
        from ap_payment_plans
        where tenant_id = $1 and ap_id = $2 and status = 'PAID'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(ap_id)
    .fetch_one(state.db.pool())
    .await?;

    if paid_amount >= doc.amount_cent {
        return repo.mark_paid(tenant_id, ap_id).await;
    }

    if doc.settlement_status != AP_SETTLEMENT_STATUS_SETTLING {
        return repo
            .update_settlement_status(tenant_id, ap_id, AP_SETTLEMENT_STATUS_SETTLING)
            .await;
    }

    Ok(Some(doc))
}
