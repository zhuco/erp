use anyhow::Result;
use chrono::{DateTime, Utc};
use erp_types::{OrderId, TenantId};
use sqlx::PgPool;

use super::FulfillmentActor;
use super::models::*;
use super::store::FulfillmentStore;

pub struct FulfillmentRepo<'a> {
    store: FulfillmentStore<'a>,
}

impl<'a> FulfillmentRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: FulfillmentStore::new(pool),
        }
    }

    pub fn with_actor(mut self, actor: FulfillmentActor) -> Self {
        self.store = self.store.with_actor(actor);
        self
    }

    pub async fn list_oms_shipments(
        &self,
        tenant_id: TenantId,
        filters: OmsShipmentListFilters,
    ) -> Result<OmsShipmentListResult> {
        self.store.list_oms_shipments(tenant_id, filters).await
    }

    pub async fn list_oms_shipment_items(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> Result<Vec<OmsShipmentItem>> {
        self.store
            .list_oms_shipment_items(tenant_id, shipment_id)
            .await
    }

    pub async fn get_oms_shipment(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> Result<Option<OmsShipment>> {
        self.store.get_oms_shipment(tenant_id, shipment_id).await
    }

    pub async fn allocate_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        carrier_code: Option<String>,
    ) -> Result<AllocationResult> {
        self.store
            .allocate_order(tenant_id, order_id, carrier_code)
            .await
    }

    pub async fn create_resend_shipments(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        carrier_code: Option<String>,
        origin_shipment_id: Option<uuid::Uuid>,
    ) -> Result<AllocationResult> {
        self.store
            .create_resend_shipments(tenant_id, order_id, carrier_code, origin_shipment_id)
            .await
    }

    pub async fn create_exchange_shipments(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        carrier_code: Option<String>,
        origin_shipment_id: Option<uuid::Uuid>,
    ) -> Result<AllocationResult> {
        self.store
            .create_exchange_shipments(tenant_id, order_id, carrier_code, origin_shipment_id)
            .await
    }

    pub async fn release_order_reservations(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<ReleaseResult> {
        self.store
            .release_order_reservations(tenant_id, order_id)
            .await
    }

    pub async fn release_order_reservations_keep_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<ReleaseResult> {
        self.store
            .release_order_reservations_keep_order(tenant_id, order_id)
            .await
    }

    pub async fn request_shipment_cancel(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        reason: Option<String>,
        requested_by: Option<uuid::Uuid>,
    ) -> Result<ShipmentCancelRequest> {
        self.store
            .request_shipment_cancel(tenant_id, shipment_id, reason, requested_by)
            .await
    }

    pub async fn ship_oms_shipment(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        carrier_code: Option<String>,
        tracking_no: Option<String>,
        shipped_at: DateTime<Utc>,
        weight_gram: Option<i32>,
    ) -> Result<(OmsShipment, bool)> {
        self.store
            .ship_oms_shipment(
                tenant_id,
                shipment_id,
                carrier_code,
                tracking_no,
                shipped_at,
                weight_gram,
            )
            .await
    }
}
