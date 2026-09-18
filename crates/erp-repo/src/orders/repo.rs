use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use erp_domain::OrderProjection;
use erp_types::{OrderId, Platform, TenantId};
use serde_json::Value;
use sqlx::PgPool;

use super::models::*;
use super::store::OrdersStore;

pub struct OrdersRepo<'a> {
    store: OrdersStore<'a>,
}

impl<'a> OrdersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: OrdersStore::new(pool),
        }
    }

    pub async fn upsert_order_projection(&self, order: &OrderProjection) -> Result<OrderId> {
        self.store.upsert_order_projection(order).await
    }

    pub async fn locate_by_platform_order_id(
        &self,
        tenant_id: TenantId,
        platform: Platform,
        platform_order_id: &str,
    ) -> Result<Option<(OrderId, NaiveDate)>> {
        self.store
            .locate_by_platform_order_id(tenant_id, platform, platform_order_id)
            .await
    }

    pub async fn get_order_shipping_info(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Option<OrderShippingInfo>> {
        self.store
            .get_order_shipping_info(tenant_id, order_id)
            .await
    }

    pub async fn list_order_items(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Vec<OrderItemLine>> {
        self.store.list_order_items(tenant_id, order_id).await
    }

    pub async fn list_order_item_stock_policies(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Vec<OrderItemStockPolicy>> {
        self.store
            .list_order_item_stock_policies(tenant_id, order_id)
            .await
    }

    pub async fn list_order_edit_items(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Vec<OrderEditItemInput>> {
        self.store.list_order_edit_items(tenant_id, order_id).await
    }

    pub async fn rematch_order_items(&self, tenant_id: TenantId, order_id: OrderId) -> Result<u64> {
        self.store.rematch_order_items(tenant_id, order_id).await
    }

    pub async fn has_mapping_missing(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<bool> {
        self.store.has_mapping_missing(tenant_id, order_id).await
    }

    pub async fn mark_order_shipped(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        shipped_at: DateTime<Utc>,
    ) -> Result<()> {
        self.store
            .mark_order_shipped(tenant_id, order_id, shipped_at)
            .await
    }

    pub async fn update_order_receiver(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        input: OrderReceiverUpdate,
    ) -> Result<bool> {
        self.store
            .update_order_receiver(tenant_id, order_id, input)
            .await
    }

    pub async fn get_order_raw(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Option<Value>> {
        self.store.get_order_raw(tenant_id, order_id).await
    }

    pub async fn list_orders(
        &self,
        tenant_id: TenantId,
        filters: OmsOrderListFilters,
    ) -> Result<OmsOrderListResult> {
        self.store.list_orders(tenant_id, filters).await
    }

    pub async fn get_order_detail(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Option<OmsOrderDetail>> {
        self.store.get_order_detail(tenant_id, order_id).await
    }

    pub async fn get_order_review_state(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Option<OrderReviewState>> {
        self.store.get_order_review_state(tenant_id, order_id).await
    }

    pub async fn get_order_review_check(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Option<OrderReviewCheck>> {
        self.store.get_order_review_check(tenant_id, order_id).await
    }

    pub async fn get_order_edit_state(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Option<OrderEditState>> {
        self.store.get_order_edit_state(tenant_id, order_id).await
    }

    pub async fn update_order_edit(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        input: OrderEditInput,
    ) -> Result<OrderEditOutcome> {
        self.store
            .update_order_edit(tenant_id, order_id, input)
            .await
    }

    pub async fn update_order_amounts(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        total_amount_cent: i64,
        goods_amount_cent: Option<i64>,
        postage_amount_cent: Option<i64>,
        discount_amount_cent: Option<i64>,
    ) -> Result<Option<()>> {
        self.store
            .update_order_amounts(
                tenant_id,
                order_id,
                total_amount_cent,
                goods_amount_cent,
                postage_amount_cent,
                discount_amount_cent,
            )
            .await
    }

    pub async fn update_order_review_state(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        input: OrderReviewUpdate,
    ) -> Result<Option<OrderReviewState>> {
        self.store
            .update_order_review_state(tenant_id, order_id, input)
            .await
    }

    pub async fn update_order_cancel_info(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        cancel_reason: Option<String>,
    ) -> Result<Option<()>> {
        self.store
            .update_order_cancel_info(tenant_id, order_id, cancel_reason)
            .await
    }

    pub async fn mark_order_refunded(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        cancel_reason: Option<String>,
    ) -> Result<Option<()>> {
        self.store
            .mark_order_refunded(tenant_id, order_id, cancel_reason)
            .await
    }

    pub async fn upsert_review_hits(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        hits: Vec<(uuid::Uuid, bool, String)>,
    ) -> Result<()> {
        self.store
            .upsert_review_hits(tenant_id, order_id, hits)
            .await
    }

    pub async fn upsert_exception_tickets(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        reason_codes: &[String],
    ) -> Result<Vec<OrderExceptionTicket>> {
        self.store
            .upsert_exception_tickets(tenant_id, order_id, reason_codes)
            .await
    }

    pub async fn update_exception_ticket_status(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        updates: &[OrderExceptionTicketUpdate],
    ) -> Result<Vec<OrderExceptionTicket>> {
        self.store
            .update_exception_ticket_status(tenant_id, order_id, updates)
            .await
    }

    pub async fn get_exception_stats(&self, tenant_id: TenantId) -> Result<OrderExceptionStats> {
        self.store.get_exception_stats(tenant_id).await
    }

    pub async fn add_order_tags(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        tags: Vec<OrderTagCreate>,
    ) -> Result<Vec<OrderTag>> {
        self.store.add_order_tags(tenant_id, order_id, tags).await
    }

    pub async fn add_order_note(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        note: OrderNoteCreate,
    ) -> Result<OrderNote> {
        self.store.add_order_note(tenant_id, order_id, note).await
    }

    pub async fn split_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        plan: Vec<OrderSplitPlanChild>,
    ) -> Result<OrderSplitResult> {
        self.store.split_order(tenant_id, order_id, plan).await
    }
}
