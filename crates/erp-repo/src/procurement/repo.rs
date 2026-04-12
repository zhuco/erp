use anyhow::Result;
use erp_types::TenantId;
use sqlx::PgPool;

use super::management_store::{
    ProcurementInspectionsStore, ProcurementPlansStore, ProcurementReconciliationsStore,
    ProcurementRequestsStore, ProcurementSourcingStore, SupplierPerformanceStore,
};
use super::models::*;
use super::store::{
    ApDocsStore, PurchaseOrdersStore, PurchaseReceiptsStore, PurchaseReturnsStore,
    PurchaseSuggestionsStore,
};

pub struct PurchaseOrdersRepo<'a> {
    store: PurchaseOrdersStore<'a>,
}

impl<'a> PurchaseOrdersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: PurchaseOrdersStore::new(pool),
        }
    }

    pub async fn list_orders(
        &self,
        tenant_id: TenantId,
        filters: PurchaseOrderListFilters,
    ) -> Result<PurchaseOrderListResult> {
        self.store.list_orders(tenant_id, filters).await
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
    ) -> Result<Option<PurchaseOrderDetail>> {
        self.store.get_detail(tenant_id, po_id).await
    }

    pub async fn create_order(&self, input: PurchaseOrderCreate) -> Result<PurchaseOrderDetail> {
        self.store.create_order(input).await
    }

    pub async fn update_order(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
        input: PurchaseOrderUpdate,
    ) -> Result<Option<PurchaseOrderDetail>> {
        self.store.update_order(tenant_id, po_id, input).await
    }

    pub async fn delete_order(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
    ) -> Result<Option<PurchaseOrder>> {
        self.store.delete_order(tenant_id, po_id).await
    }

    pub async fn mark_sent(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
    ) -> Result<Option<PurchaseOrder>> {
        self.store.mark_sent(tenant_id, po_id).await
    }

    pub async fn confirm_order(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
    ) -> Result<Option<PurchaseOrder>> {
        self.store.confirm_order(tenant_id, po_id).await
    }

    pub async fn receive_order(
        &self,
        input: PurchaseOrderReceiveInput,
    ) -> Result<PurchaseOrderReceiveResult> {
        self.store.receive_order(input).await
    }
}

pub struct PurchaseReceiptsRepo<'a> {
    store: PurchaseReceiptsStore<'a>,
}

impl<'a> PurchaseReceiptsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: PurchaseReceiptsStore::new(pool),
        }
    }

    pub async fn list_receipts(
        &self,
        tenant_id: TenantId,
        filters: PurchaseReceiptListFilters,
    ) -> Result<PurchaseReceiptListResult> {
        self.store.list_receipts(tenant_id, filters).await
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        receipt_id: uuid::Uuid,
    ) -> Result<Option<PurchaseReceiptDetail>> {
        self.store.get_detail(tenant_id, receipt_id).await
    }
}

pub struct PurchaseReturnsRepo<'a> {
    store: PurchaseReturnsStore<'a>,
}

impl<'a> PurchaseReturnsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: PurchaseReturnsStore::new(pool),
        }
    }

    pub async fn list_returns(
        &self,
        tenant_id: TenantId,
        filters: PurchaseReturnListFilters,
    ) -> Result<PurchaseReturnListResult> {
        self.store.list_returns(tenant_id, filters).await
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        return_id: uuid::Uuid,
    ) -> Result<Option<PurchaseReturnDetail>> {
        self.store.get_detail(tenant_id, return_id).await
    }

    pub async fn create_return(&self, input: PurchaseReturnCreate) -> Result<PurchaseReturnDetail> {
        self.store.create_return(input).await
    }

    pub async fn confirm_return(
        &self,
        tenant_id: TenantId,
        return_id: uuid::Uuid,
    ) -> Result<PurchaseReturnConfirmResult> {
        self.store.confirm_return(tenant_id, return_id).await
    }
}

pub struct PurchaseSuggestionsRepo<'a> {
    store: PurchaseSuggestionsStore<'a>,
}

impl<'a> PurchaseSuggestionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: PurchaseSuggestionsStore::new(pool),
        }
    }

    pub async fn refresh(&self, tenant_id: TenantId) -> Result<Vec<PurchaseSuggestion>> {
        self.store.refresh(tenant_id).await
    }

    pub async fn list_suggestions(
        &self,
        tenant_id: TenantId,
        filters: PurchaseSuggestionListFilters,
    ) -> Result<PurchaseSuggestionListResult> {
        self.store.list_suggestions(tenant_id, filters).await
    }
}

pub struct ApDocsRepo<'a> {
    store: ApDocsStore<'a>,
}

impl<'a> ApDocsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: ApDocsStore::new(pool),
        }
    }

    pub async fn list_docs(
        &self,
        tenant_id: TenantId,
        filters: ApDocListFilters,
    ) -> Result<ApDocListResult> {
        self.store.list_docs(tenant_id, filters).await
    }

    pub async fn get_doc(&self, tenant_id: TenantId, ap_id: uuid::Uuid) -> Result<Option<ApDoc>> {
        self.store.get_doc(tenant_id, ap_id).await
    }

    pub async fn mark_paid(&self, tenant_id: TenantId, ap_id: uuid::Uuid) -> Result<Option<ApDoc>> {
        self.store.mark_paid(tenant_id, ap_id).await
    }

    pub async fn close_doc(&self, tenant_id: TenantId, ap_id: uuid::Uuid) -> Result<Option<ApDoc>> {
        self.store.close_doc(tenant_id, ap_id).await
    }

    pub async fn update_settlement_status(
        &self,
        tenant_id: TenantId,
        ap_id: uuid::Uuid,
        settlement_status: &str,
    ) -> Result<Option<ApDoc>> {
        self.store
            .update_settlement_status(tenant_id, ap_id, settlement_status)
            .await
    }
}

pub struct ProcurementRequestsRepo<'a> {
    store: ProcurementRequestsStore<'a>,
}

impl<'a> ProcurementRequestsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: ProcurementRequestsStore::new(pool),
        }
    }

    pub async fn list_requests(
        &self,
        tenant_id: TenantId,
        filters: ProcurementRequestListFilters,
    ) -> Result<ProcurementRequestListResult> {
        self.store.list_requests(tenant_id, filters).await
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
    ) -> Result<Option<ProcurementRequestDetail>> {
        self.store.get_detail(tenant_id, request_id).await
    }

    pub async fn create_request(
        &self,
        input: ProcurementRequestCreate,
    ) -> Result<ProcurementRequestDetail> {
        self.store.create_request(input).await
    }

    pub async fn update_request(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
        input: ProcurementRequestUpdate,
    ) -> Result<Option<ProcurementRequestDetail>> {
        self.store
            .update_request(tenant_id, request_id, input)
            .await
    }

    pub async fn delete_request(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
    ) -> Result<Option<ProcurementRequest>> {
        self.store.delete_request(tenant_id, request_id).await
    }
}

pub struct ProcurementPlansRepo<'a> {
    store: ProcurementPlansStore<'a>,
}

impl<'a> ProcurementPlansRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: ProcurementPlansStore::new(pool),
        }
    }

    pub async fn list_plans(
        &self,
        tenant_id: TenantId,
        filters: ProcurementPlanListFilters,
    ) -> Result<ProcurementPlanListResult> {
        self.store.list_plans(tenant_id, filters).await
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
    ) -> Result<Option<ProcurementPlanDetail>> {
        self.store.get_detail(tenant_id, plan_id).await
    }

    pub async fn create_plan(&self, input: ProcurementPlanCreate) -> Result<ProcurementPlanDetail> {
        self.store.create_plan(input).await
    }

    pub async fn update_plan(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
        input: ProcurementPlanUpdate,
    ) -> Result<Option<ProcurementPlanDetail>> {
        self.store.update_plan(tenant_id, plan_id, input).await
    }

    pub async fn delete_plan(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
    ) -> Result<Option<ProcurementPlan>> {
        self.store.delete_plan(tenant_id, plan_id).await
    }
}

pub struct ProcurementSourcingRepo<'a> {
    store: ProcurementSourcingStore<'a>,
}

impl<'a> ProcurementSourcingRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: ProcurementSourcingStore::new(pool),
        }
    }

    pub async fn list_sourcing(
        &self,
        tenant_id: TenantId,
        filters: ProcurementSourcingListFilters,
    ) -> Result<ProcurementSourcingListResult> {
        self.store.list_sourcing(tenant_id, filters).await
    }

    pub async fn create_sourcing(
        &self,
        input: ProcurementSourcingCreate,
    ) -> Result<ProcurementSourcing> {
        self.store.create_sourcing(input).await
    }

    pub async fn update_sourcing(
        &self,
        tenant_id: TenantId,
        sourcing_id: uuid::Uuid,
        input: ProcurementSourcingUpdate,
    ) -> Result<Option<ProcurementSourcing>> {
        self.store
            .update_sourcing(tenant_id, sourcing_id, input)
            .await
    }

    pub async fn delete_sourcing(
        &self,
        tenant_id: TenantId,
        sourcing_id: uuid::Uuid,
    ) -> Result<Option<ProcurementSourcing>> {
        self.store.delete_sourcing(tenant_id, sourcing_id).await
    }
}

pub struct ProcurementInspectionsRepo<'a> {
    store: ProcurementInspectionsStore<'a>,
}

impl<'a> ProcurementInspectionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: ProcurementInspectionsStore::new(pool),
        }
    }

    pub async fn list_inspections(
        &self,
        tenant_id: TenantId,
        filters: ProcurementInspectionListFilters,
    ) -> Result<ProcurementInspectionListResult> {
        self.store.list_inspections(tenant_id, filters).await
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        inspection_id: uuid::Uuid,
    ) -> Result<Option<ProcurementInspectionDetail>> {
        self.store.get_detail(tenant_id, inspection_id).await
    }

    pub async fn create_inspection(
        &self,
        input: ProcurementInspectionCreate,
    ) -> Result<ProcurementInspectionDetail> {
        self.store.create_inspection(input).await
    }

    pub async fn update_inspection(
        &self,
        tenant_id: TenantId,
        inspection_id: uuid::Uuid,
        input: ProcurementInspectionUpdate,
    ) -> Result<Option<ProcurementInspectionDetail>> {
        self.store
            .update_inspection(tenant_id, inspection_id, input)
            .await
    }

    pub async fn delete_inspection(
        &self,
        tenant_id: TenantId,
        inspection_id: uuid::Uuid,
    ) -> Result<Option<ProcurementInspection>> {
        self.store.delete_inspection(tenant_id, inspection_id).await
    }
}

pub struct ProcurementReconciliationsRepo<'a> {
    store: ProcurementReconciliationsStore<'a>,
}

impl<'a> ProcurementReconciliationsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: ProcurementReconciliationsStore::new(pool),
        }
    }

    pub async fn list_reconciliations(
        &self,
        tenant_id: TenantId,
        filters: ProcurementReconciliationListFilters,
    ) -> Result<ProcurementReconciliationListResult> {
        self.store.list_reconciliations(tenant_id, filters).await
    }

    pub async fn create_reconciliation(
        &self,
        input: ProcurementReconciliationCreate,
    ) -> Result<ProcurementReconciliation> {
        self.store.create_reconciliation(input).await
    }

    pub async fn update_reconciliation(
        &self,
        tenant_id: TenantId,
        reconciliation_id: uuid::Uuid,
        input: ProcurementReconciliationUpdate,
    ) -> Result<Option<ProcurementReconciliation>> {
        self.store
            .update_reconciliation(tenant_id, reconciliation_id, input)
            .await
    }

    pub async fn delete_reconciliation(
        &self,
        tenant_id: TenantId,
        reconciliation_id: uuid::Uuid,
    ) -> Result<Option<ProcurementReconciliation>> {
        self.store
            .delete_reconciliation(tenant_id, reconciliation_id)
            .await
    }
}

pub struct SupplierPerformanceRepo<'a> {
    store: SupplierPerformanceStore<'a>,
}

impl<'a> SupplierPerformanceRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: SupplierPerformanceStore::new(pool),
        }
    }

    pub async fn list_performance(
        &self,
        tenant_id: TenantId,
        filters: SupplierPerformanceListFilters,
    ) -> Result<SupplierPerformanceListResult> {
        self.store.list_performance(tenant_id, filters).await
    }

    pub async fn create_performance(
        &self,
        input: SupplierPerformanceCreate,
    ) -> Result<SupplierPerformance> {
        self.store.create_performance(input).await
    }

    pub async fn update_performance(
        &self,
        tenant_id: TenantId,
        performance_id: uuid::Uuid,
        input: SupplierPerformanceUpdate,
    ) -> Result<Option<SupplierPerformance>> {
        self.store
            .update_performance(tenant_id, performance_id, input)
            .await
    }

    pub async fn delete_performance(
        &self,
        tenant_id: TenantId,
        performance_id: uuid::Uuid,
    ) -> Result<Option<SupplierPerformance>> {
        self.store
            .delete_performance(tenant_id, performance_id)
            .await
    }
}
