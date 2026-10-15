//! Internal Rust cost commands. HTTP DTOs and immutable receipts keep their contracts.
use erp_types::TenantId;

pub struct CostReceiptInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub item_code: &'a str,
    pub qty_micros: i64,
    pub amount_cent: i64,
    pub period_code: &'a str,
}

pub struct CostIssueInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub item_code: &'a str,
    pub qty_micros: i64,
    pub period_code: &'a str,
}

pub struct CostReturnCommand<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub item_code: &'a str,
    pub issue_request: &'a str,
    pub return_qty: i64,
    pub period_code: &'a str,
}

pub struct CostAdjustmentInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub item_code: &'a str,
    pub amount_cent: i64,
    pub period_code: &'a str,
}

pub struct ClosedCostAdjustmentInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub item_code: &'a str,
    pub amount_cent: i64,
    pub closed_period: &'a str,
    pub next_period: &'a str,
}
