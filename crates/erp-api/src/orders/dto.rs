use super::*;

#[derive(Debug, Serialize)]
pub(super) struct OmsOrderChainResponse {
    pub(super) order: OmsOrderDetailResponse,
    pub(super) shipments: Vec<OmsShipmentSummary>,
    pub(super) after_sales: Vec<AfterSalesDetailView>,
    pub(super) inbounds: Vec<InboundDocSummaryResponse>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OrderDownloadRequest {
    pub(super) shop_id: String,
    pub(super) cursor: Option<String>,
    #[serde(alias = "last_time")]
    pub(super) watermark: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct ManualOrderItemRequest {
    pub(super) line_id: Option<String>,
    pub(super) sku_id: Option<String>,
    pub(super) title: Option<String>,
    pub(super) qty: i32,
    pub(super) price_cent: i64,
    pub(super) amount_cent: Option<i64>,
    pub(super) pay_amount_cent: Option<i64>,
    pub(super) discount_cent: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct ManualOrderCreateRequest {
    pub(super) shop_id: String,
    pub(super) platform_order_id: String,
    pub(super) total_amount_cent: Option<i64>,
    pub(super) goods_amount_cent: Option<i64>,
    pub(super) postage_amount_cent: Option<i64>,
    pub(super) discount_amount_cent: Option<i64>,
    pub(super) currency: Option<String>,
    pub(super) buyer_name: Option<String>,
    pub(super) receiver_name: Option<String>,
    pub(super) receiver_phone: Option<String>,
    pub(super) receiver_address: Option<String>,
    pub(super) logistics_company: Option<String>,
    pub(super) tracking_no: Option<String>,
    pub(super) buyer_message: Option<String>,
    pub(super) seller_remark: Option<String>,
    pub(super) order_created_at: Option<String>,
    pub(super) platform_updated_at: Option<String>,
    pub(super) items: Vec<ManualOrderItemRequest>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OrderRecheckRequest {
    pub(super) reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OrderSubmitRequest {
    pub(super) carrier_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct OmsReviewSettingsPayload {
    pub(super) review_enabled: bool,
    pub(super) auto_review_enabled: bool,
    pub(super) recheck_enabled: bool,
    pub(super) default_unmatched_action: String,
    pub(super) manual_order_force_recheck: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct OmsReviewSettingsResponse {
    pub(super) tenant_id: String,
    pub(super) review_enabled: bool,
    pub(super) auto_review_enabled: bool,
    pub(super) recheck_enabled: bool,
    pub(super) default_unmatched_action: String,
    pub(super) manual_order_force_recheck: bool,
    pub(super) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct OmsRecheckQueueQuery {
    pub(super) shop_id: Option<String>,
    pub(super) platform: Option<String>,
    pub(super) sales_channel: Option<String>,
    pub(super) order_source: Option<String>,
    pub(super) min_estimated_profit_rate_bp: Option<String>,
    pub(super) max_estimated_profit_rate_bp: Option<String>,
    pub(super) sku_id: Option<String>,
    pub(super) offset: Option<i64>,
    pub(super) limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct OmsRecheckBatchReviewRequest {
    pub(super) order_ids: Vec<String>,
    pub(super) action: String,
    pub(super) reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct OmsRecheckBatchReviewResponse {
    pub(super) total: i64,
    pub(super) accepted: i64,
    pub(super) rejected: i64,
    pub(super) action: String,
    pub(super) reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct OmsSubmitShipmentRequest {
    pub(super) warehouse_id: Option<String>,
    pub(super) carrier_code: Option<String>,
    pub(super) logistics_product_code: Option<String>,
    pub(super) priority: Option<String>,
    pub(super) remark: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct OmsSubmitShipmentResponse {
    pub(super) order_id: String,
    pub(super) outbound_doc_id: Option<String>,
    pub(super) wms_status: String,
}
