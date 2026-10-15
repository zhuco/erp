//! Internal Rust commands; HTTP DTOs and receipt identities retain their contracts.
use erp_types::TenantId;
use uuid::Uuid;

pub struct TradePurchaseRevisionInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub po_id: Option<Uuid>,
    pub supplier_code: &'a str,
    pub warehouse_code: &'a str,
    pub action: &'a str,
    pub lines: &'a [erp_domain::trade::PurchaseLine],
}

pub struct TradeSalesRevisionInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub so_id: Option<Uuid>,
    pub customer_code: &'a str,
    pub expected_version: Option<i32>,
    pub action: &'a str,
    pub lines: &'a [erp_domain::trade::SalesLine],
}

pub struct SalesDeliveryPlanInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub so_id: Uuid,
    pub line_no: i32,
    pub expected_version: i32,
    pub qty_micros: i64,
}

pub struct SalesDeliveryAdvanceInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub delivery_id: Uuid,
    pub action: &'a str,
    pub carrier_ref: &'a str,
    pub warehouse_id: Uuid,
    pub location_id: Uuid,
    pub sku_id: Uuid,
    pub batch_code: &'a str,
}

pub struct TradeQuoteRevisionInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub quote_id: Option<Uuid>,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub item_code: &'a str,
    pub qty_micros: i64,
    pub price_cent: i64,
    pub start_date: &'a str,
    pub end_date: &'a str,
}

pub struct TradeQuoteConversionInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub quote_id: Uuid,
    pub expected_version: i32,
    pub today: &'a str,
    pub qty_micros: i64,
}

pub struct TradePurchaseInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub supplier_id: Uuid,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub qty: i64,
    pub price_cent: i64,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub document_date: &'a str,
}

pub struct TradeSaleInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub customer_id: Uuid,
    pub sku_id: Uuid,
    pub purchased_qty: i64,
    pub unit_cost_cent: i64,
    pub sold_qty: i64,
    pub unit_price_cent: i64,
    pub returned_qty: i64,
    pub customer_paid_cent: i64,
    pub supplier_paid_cent: i64,
    pub partner_code: &'a str,
    pub currency: &'a str,
    pub document_date: &'a str,
    pub warehouse_id: Uuid,
}

pub struct SharedPartnerInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub name: &'a str,
    pub customer: bool,
    pub supplier: bool,
    pub customer_no: &'a str,
    pub supplier_no: &'a str,
    pub term_days: i32,
    pub tax_no: &'a str,
}

pub struct PartnerContactInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub partner_id: Uuid,
    pub contact_name: &'a str,
    pub phone: &'a str,
    pub address: &'a str,
}

pub struct ScaledTradeSaleInput<'a> {
    pub tenant_id: TenantId,
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub customer_id: Uuid,
    pub sku_id: Uuid,
    pub warehouse_id: Uuid,
    pub qty_text: &'a str,
    pub unit_price_cent: i64,
    pub currency: &'a str,
    pub partner_code: &'a str,
}
