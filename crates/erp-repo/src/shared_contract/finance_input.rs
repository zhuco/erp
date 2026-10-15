//! Internal Rust commands; HTTP DTOs and receipt identities retain their contracts.
use erp_types::TenantId;
use uuid::Uuid;

pub struct DocumentBalanceInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub document_id: Uuid,
    pub document_kind: &'a str,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub due_date: &'a str,
}

pub struct AdvanceBalanceInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub kind: erp_domain::trade::PartnerBalanceKind,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub source_id: &'a str,
    pub amount_cent: i64,
}

pub struct OpeningBalanceInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub kind: erp_domain::trade::PartnerBalanceKind,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub source_id: &'a str,
    pub amount_cent: i64,
    pub due_date: &'a str,
}

pub struct PartnerBalanceInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub kind: erp_domain::trade::PartnerBalanceKind,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub source_type: &'a str,
    pub source_id: &'a str,
    pub source_status: &'a str,
    pub amount_cent: i64,
    pub due_date: Option<&'a str>,
}

pub struct CitedInvoiceInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub number: &'a str,
    pub issuer: &'a str,
    pub invoice_date: &'a str,
    pub balance_id: Uuid,
    pub requested_official: bool,
}

pub struct InvoiceRecordInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub number: &'a str,
    pub issuer: &'a str,
    pub invoice_date: &'a str,
    pub currency: &'a str,
    pub gross_cent: i64,
    pub tax_cent: i64,
    pub source_type: &'a str,
    pub source_id: &'a str,
    pub balance_id: Option<Uuid>,
}

pub struct CostFactInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub period_code: &'a str,
    pub bucket: &'a str,
    pub amount_cent: i64,
    pub source_ref: &'a str,
}

pub struct PostedCostReconciliationInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub period_code: &'a str,
    pub currency: &'a str,
    pub cutoff_at: &'a str,
    pub books: &'a [(&'a str, i64)],
}

pub struct CostReconciliationInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub period_code: &'a str,
    pub currency: &'a str,
    pub cutoff_at: &'a str,
    pub buckets: &'a [(&'a str, i64, i64, &'a str)],
}

pub struct PeriodAdjustmentInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub period_code: &'a str,
    pub document_date: &'a str,
    pub amount_cent: i64,
    pub reason: &'a str,
}

pub struct CitedExpenseAllocationInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub category: &'a str,
    pub partner_code: &'a str,
    pub purchase_id: Uuid,
    pub currency: &'a str,
    pub gross_cent: i64,
    pub deductible_tax_cent: i64,
    pub weights: &'a [(&'a str, i64)],
}

pub struct ExpenseAllocationInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub category: &'a str,
    pub partner_code: &'a str,
    pub source_type: &'a str,
    pub source_id: &'a str,
    pub currency: &'a str,
    pub gross_cent: i64,
    pub deductible_tax_cent: i64,
    pub tax_is_cost: bool,
    pub weights: &'a [(&'a str, i64)],
}

pub struct ReturnCashOffsetInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub balance_id: Uuid,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub account_code: &'a str,
    pub return_cent: i64,
}

pub struct CashEntryInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub direction: &'a str,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub account_code: &'a str,
    pub amount_cent: i64,
    pub balance_id: Option<Uuid>,
    pub reverses_settlement_id: Option<Uuid>,
}
