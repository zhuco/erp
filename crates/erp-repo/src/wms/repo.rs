use anyhow::Result;
use chrono::{DateTime, Utc};
use erp_types::{OrderId, TenantId};
use sqlx::PgPool;

use super::models::*;
use super::store::WmsStore;

pub struct WmsRepo<'a> {
    store: WmsStore<'a>,
}

impl<'a> WmsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: WmsStore::new(pool),
        }
    }

    pub async fn list_outbound_docs(
        &self,
        tenant_id: TenantId,
        filters: OutboundDocListFilters,
    ) -> Result<OutboundDocListResult> {
        self.store.list_outbound_docs(tenant_id, filters).await
    }

    pub async fn get_outbound_doc(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> Result<Option<OutboundDocDetail>> {
        self.store
            .get_outbound_doc(tenant_id, outbound_doc_id)
            .await
    }

    pub async fn audit_outbound_doc(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> Result<OutboundDocDetail> {
        self.store
            .audit_outbound_doc(tenant_id, outbound_doc_id)
            .await
    }

    pub async fn create_outbound_doc(
        &self,
        input: OutboundDocCreateInput,
    ) -> Result<OutboundDocDetail> {
        self.store.create_outbound_doc(input).await
    }

    pub async fn create_pick_tasks_for_outbound(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
        auto_pick: bool,
    ) -> Result<Vec<OutboundPickTask>> {
        self.store
            .create_pick_tasks_for_outbound(tenant_id, outbound_doc_id, auto_pick)
            .await
    }

    pub async fn list_waves(
        &self,
        tenant_id: TenantId,
        filters: WaveListFilters,
    ) -> Result<WaveListResult> {
        self.store.list_waves(tenant_id, filters).await
    }

    pub async fn get_wave(
        &self,
        tenant_id: TenantId,
        wave_id: uuid::Uuid,
    ) -> Result<Option<WaveDetail>> {
        self.store.get_wave(tenant_id, wave_id).await
    }

    pub async fn list_wave_rules(
        &self,
        tenant_id: TenantId,
        filters: WaveRuleListFilters,
    ) -> Result<WaveRuleListResult> {
        self.store.list_wave_rules(tenant_id, filters).await
    }

    pub async fn get_wave_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> Result<Option<WaveRule>> {
        self.store.get_wave_rule(tenant_id, rule_id).await
    }

    pub async fn create_wave_rule(&self, input: WaveRuleCreateInput) -> Result<WaveRule> {
        self.store.create_wave_rule(input).await
    }

    pub async fn update_wave_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: WaveRuleUpdateInput,
    ) -> Result<Option<WaveRule>> {
        self.store.update_wave_rule(tenant_id, rule_id, input).await
    }

    pub async fn delete_wave_rule(&self, tenant_id: TenantId, rule_id: uuid::Uuid) -> Result<bool> {
        self.store.delete_wave_rule(tenant_id, rule_id).await
    }

    pub async fn count_available_outbounds(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        status: &str,
        cutoff_at: DateTime<Utc>,
    ) -> Result<i64> {
        self.store
            .count_available_outbounds(tenant_id, warehouse_id, status, cutoff_at)
            .await
    }

    pub async fn list_wave_candidates(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        status: &str,
        cutoff_at: DateTime<Utc>,
        carrier_code: Option<&str>,
        limit: i64,
    ) -> Result<Vec<WaveCandidate>> {
        self.store
            .list_wave_candidates(
                tenant_id,
                warehouse_id,
                status,
                cutoff_at,
                carrier_code,
                limit,
            )
            .await
    }

    pub async fn create_wave(&self, input: WaveCreateInput) -> Result<WaveDetail> {
        self.store.create_wave(input).await
    }

    pub async fn create_wave_with_status(
        &self,
        input: WaveCreateInput,
        status: &str,
    ) -> Result<WaveDetail> {
        self.store.create_wave_with_status(input, status).await
    }

    pub async fn list_pick_tasks(
        &self,
        tenant_id: TenantId,
        filters: PickTaskListFilters,
    ) -> Result<PickTaskListResult> {
        self.store.list_pick_tasks(tenant_id, filters).await
    }

    pub async fn confirm_pick_task(&self, input: PickTaskConfirmInput) -> Result<()> {
        self.store.confirm_pick_task(input).await
    }

    pub async fn pack_outbound(&self, input: OutboundPackInput) -> Result<()> {
        self.store.pack_outbound(input).await
    }

    pub async fn record_pack_check_event(&self, input: PackCheckEventInput) -> Result<()> {
        self.store.record_pack_check_event(input).await
    }

    pub async fn update_pack_record_weight(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
        package_no: &str,
        weight_gram: i32,
        operator_id: Option<uuid::Uuid>,
    ) -> Result<()> {
        self.store
            .update_pack_record_weight(
                tenant_id,
                outbound_doc_id,
                package_no,
                weight_gram,
                operator_id,
            )
            .await
    }

    pub async fn has_unweighed_packs(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> Result<bool> {
        self.store
            .has_unweighed_packs(tenant_id, outbound_doc_id)
            .await
    }

    pub async fn has_unchecked_packs(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> Result<bool> {
        self.store
            .has_unchecked_packs(tenant_id, outbound_doc_id)
            .await
    }

    pub async fn reset_outbound_inspection(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> Result<()> {
        self.store
            .reset_outbound_inspection(tenant_id, outbound_doc_id)
            .await
    }

    pub async fn inspect_outbound(&self, input: OutboundInspectInput) -> Result<()> {
        self.store.inspect_outbound(input).await
    }

    pub async fn start_outbound_verify(
        &self,
        input: OutboundVerifyStartInput,
    ) -> Result<OutboundVerifyState> {
        self.store.start_outbound_verify(input).await
    }

    pub async fn scan_outbound_verify_item(
        &self,
        input: OutboundVerifyScanInput,
    ) -> Result<OutboundVerifyState> {
        self.store.scan_outbound_verify_item(input).await
    }

    pub async fn finish_outbound_verify(
        &self,
        input: OutboundVerifyFinishInput,
    ) -> Result<OutboundVerifyState> {
        self.store.finish_outbound_verify(input).await
    }

    pub async fn ship_outbound(&self, input: OutboundShipInput) -> Result<()> {
        self.store.ship_outbound(input).await
    }

    pub async fn cancel_outbounds_for_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> Result<Vec<uuid::Uuid>> {
        self.store
            .cancel_outbounds_for_order(tenant_id, order_id)
            .await
    }
}
