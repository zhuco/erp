use super::*;

#[async_trait]
pub trait AfterSaleRepo: Send + Sync {
    async fn list_cases(
        &self,
        tenant_id: TenantId,
        filters: erp_repo::after_sale_cases::AfterSaleCaseListFilters,
    ) -> anyhow::Result<Vec<erp_domain::AfterSaleCase>>;

    async fn get_case_detail(
        &self,
        tenant_id: TenantId,
        case_id: AfterSaleCaseId,
    ) -> anyhow::Result<Option<erp_domain::AfterSaleCaseDetail>>;

    async fn get_case_raw(
        &self,
        tenant_id: TenantId,
        case_id: AfterSaleCaseId,
    ) -> anyhow::Result<Value>;
}

#[async_trait]
pub trait ProductRepo: Send + Sync {
    async fn retry_copy_batch_review(
        &self,
        tenant: TenantId,
        actor: UserId,
        original_batch: uuid::Uuid,
        request_id: uuid::Uuid,
    ) -> Result<
        erp_repo::product_copy_batch_retries::RetryReview,
        erp_repo::product_copy_batch_retries::RetryError,
    >;
    async fn copy_batch_results(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
        after: Option<u64>,
        limit: u16,
        failed_only: bool,
    ) -> Result<
        erp_repo::product_copy_batch_results::Results,
        erp_repo::product_copy_batches::BatchError,
    >;
    async fn list_copy_batches(
        &self,
        tenant: TenantId,
        actor: UserId,
    ) -> Result<
        Vec<erp_repo::product_copy_batches::Status>,
        erp_repo::product_copy_batches::BatchError,
    >;
    async fn start_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        request: &erp_repo::product_copy_batches::StartRequest,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>;
    async fn copy_batch_status(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>;
    async fn advance_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
        max_items: u16,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>;
    async fn cancel_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>;
    async fn freeze_selection(
        &self,
        tenant: TenantId,
        actor: UserId,
        query: &erp_grid_core::paged_query::PageQuery,
    ) -> Result<
        erp_repo::product_selection::SelectionToken,
        erp_repo::product_selection::SelectionError,
    >;
    async fn selection_page(
        &self,
        tenant: TenantId,
        actor: UserId,
        token: uuid::Uuid,
        offset: u64,
        limit: u16,
        sorts: &[erp_grid_core::SortRule],
    ) -> Result<
        erp_repo::product_selection::SelectionPage,
        erp_repo::product_selection::SelectionError,
    >;
    async fn selection_page_query_order(
        &self,
        tenant: TenantId,
        actor: UserId,
        token: uuid::Uuid,
        offset: u64,
        limit: u16,
        sorts: &[erp_grid_core::SortRule],
        query_order: bool,
    ) -> Result<
        erp_repo::product_selection::SelectionPage,
        erp_repo::product_selection::SelectionError,
    > {
        if query_order {
            Err(erp_repo::product_selection::SelectionError::Invalid(
                "query-order selection paging is unavailable".into(),
            ))
        } else {
            self.selection_page(tenant, actor, token, offset, limit, sorts)
                .await
        }
    }
    async fn selection_count(
        &self,
        tenant: TenantId,
        actor: UserId,
        token: uuid::Uuid,
        excluded: &[uuid::Uuid],
    ) -> Result<u64, erp_repo::product_selection::SelectionError>;
    async fn product_options(
        &self,
        tenant: TenantId,
        query: &erp_repo::product_grid_options::OptionQuery,
    ) -> anyhow::Result<erp_repo::product_grid_options::OptionPage>;
    async fn execute_action(
        &self,
        tenant: TenantId,
        actor: UserId,
        command: &erp_grid_core::actions::ActionCommand,
    ) -> Result<erp_grid_core::actions::ActionReceipt, erp_repo::product_actions::ActionError>;
    async fn action_receipt(
        &self,
        tenant: TenantId,
        actor: UserId,
        operation: erp_grid_core::RowId,
    ) -> Result<Option<erp_grid_core::actions::ActionReceipt>, erp_repo::product_actions::ActionError>;
    async fn query_products(
        &self,
        tenant_id: TenantId,
        query: &erp_grid_core::paged_query::PageQuery,
    ) -> anyhow::Result<erp_repo::product_grid::ProductGridPage>;

    async fn list_products(
        &self,
        tenant_id: TenantId,
        filters: ProductListFilters,
    ) -> anyhow::Result<ProductListResult>;

    async fn get_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Product>>;

    async fn create_product(&self, input: ProductCreate) -> anyhow::Result<Product>;

    async fn update_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        input: ProductUpdate,
    ) -> anyhow::Result<Option<Product>>;

    async fn delete_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Product>>;
}

#[async_trait]
pub trait SkuRepo: Send + Sync {
    async fn execute_grid_action(
        &self,
        tenant: TenantId,
        actor: UserId,
        command: &erp_grid_core::actions::ActionCommand,
    ) -> Result<erp_grid_core::actions::ActionReceipt, erp_repo::product_actions::ActionError>;

    async fn grid_action_receipt(
        &self,
        tenant: TenantId,
        actor: UserId,
        operation: erp_grid_core::RowId,
    ) -> Result<Option<erp_grid_core::actions::ActionReceipt>, erp_repo::product_actions::ActionError>;

    async fn query_skus(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        query: &erp_grid_core::paged_query::PageQuery,
    ) -> anyhow::Result<erp_repo::sku_grid::SkuGridPage>;

    async fn query_sku_ranges(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        request: &erp_repo::sku_grid::SkuGridRangesRequest,
    ) -> anyhow::Result<erp_repo::sku_grid::SkuGridRangesPage>;

    async fn list_skus(
        &self,
        tenant_id: TenantId,
        filters: SkuListFilters,
    ) -> anyhow::Result<SkuListResult>;

    async fn search_skus(
        &self,
        tenant_id: TenantId,
        filters: SkuSearchFilters,
    ) -> anyhow::Result<Vec<RepoSkuSearchItem>>;

    async fn get_sku(&self, tenant_id: TenantId, sku_id: uuid::Uuid)
    -> anyhow::Result<Option<Sku>>;

    async fn create_sku(&self, input: SkuCreate) -> anyhow::Result<Sku>;

    async fn update_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        input: SkuUpdate,
    ) -> anyhow::Result<Option<Sku>>;

    async fn delete_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Sku>>;
}

#[async_trait]
pub trait SkuSupplierRepo: Send + Sync {
    async fn list_sku_suppliers(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<SkuSupplier>>;

    async fn replace_sku_suppliers(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        supplier_ids: Vec<uuid::Uuid>,
        primary_supplier_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<()>;
}

#[async_trait]
pub trait SkuCostRepo: Send + Sync {
    async fn list_cost_ledgers(
        &self,
        tenant_id: TenantId,
        filters: SkuCostLedgerListFilters,
    ) -> anyhow::Result<SkuCostLedgerListResult>;

    async fn adjust_cost(&self, input: SkuCostAdjustInput) -> anyhow::Result<SkuCostLedger>;
}

#[async_trait]
pub trait KitRepo: Send + Sync {
    async fn list_kits(
        &self,
        tenant_id: TenantId,
        filters: KitListFilters,
    ) -> anyhow::Result<KitListResult>;

    async fn get_kit_detail(
        &self,
        tenant_id: TenantId,
        kit_sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<KitDetail>>;

    async fn save_kit_bom(&self, input: KitBomSaveInput) -> anyhow::Result<Option<KitDetail>>;
}

#[async_trait]
pub trait SkuBatchRepo: Send + Sync {
    async fn list_batches(
        &self,
        tenant_id: TenantId,
        filters: SkuBatchListFilters,
    ) -> anyhow::Result<SkuBatchListResult>;

    async fn get_batch(
        &self,
        tenant_id: TenantId,
        batch_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuBatch>>;

    async fn create_batch(&self, input: SkuBatchCreate) -> anyhow::Result<SkuBatch>;

    async fn update_batch(
        &self,
        tenant_id: TenantId,
        batch_id: uuid::Uuid,
        input: SkuBatchUpdate,
    ) -> anyhow::Result<Option<SkuBatch>>;
}

#[async_trait]
pub trait SkuSerialRepo: Send + Sync {
    async fn list_serials(
        &self,
        tenant_id: TenantId,
        filters: SkuSerialListFilters,
    ) -> anyhow::Result<SkuSerialListResult>;

    async fn generate_serials(
        &self,
        input: SkuSerialGenerateInput,
    ) -> anyhow::Result<Vec<SkuSerial>>;

    async fn bind_serials_to_shipment(
        &self,
        input: RepoShipmentSerialBindInput,
    ) -> anyhow::Result<Vec<SkuSerial>>;

    async fn count_shipment_serials(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<ShipmentSerialCount>>;

    async fn mark_shipment_serials_shipped(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<()>;
}

#[async_trait]
pub trait ChannelSkuMappingRepo: Send + Sync {
    async fn list_mappings(
        &self,
        tenant_id: TenantId,
        filters: ChannelSkuMappingListFilters,
    ) -> anyhow::Result<ChannelSkuMappingListResult>;

    async fn create_mapping(
        &self,
        input: ChannelSkuMappingCreate,
    ) -> anyhow::Result<ChannelSkuMapping>;

    async fn update_mapping(
        &self,
        tenant_id: TenantId,
        mapping_id: uuid::Uuid,
        input: ChannelSkuMappingUpdate,
    ) -> anyhow::Result<Option<ChannelSkuMapping>>;
}

#[async_trait]
pub trait WarehouseRepo: Send + Sync {
    async fn list_warehouses(
        &self,
        tenant_id: TenantId,
        filters: WarehouseListFilters,
    ) -> anyhow::Result<WarehouseListResult>;

    async fn create_warehouse(&self, input: WarehouseCreate) -> anyhow::Result<Warehouse>;

    async fn update_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        input: WarehouseUpdate,
    ) -> anyhow::Result<Option<Warehouse>>;

    async fn delete_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Warehouse>>;

    async fn set_default_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Warehouse>>;

    async fn get_default_warehouse(&self, tenant_id: TenantId)
    -> anyhow::Result<Option<Warehouse>>;
}

#[async_trait]
pub trait CourierRepo: Send + Sync {
    async fn list_couriers(
        &self,
        tenant_id: TenantId,
        filters: CourierListFilters,
    ) -> anyhow::Result<CourierListResult>;

    async fn create_courier(&self, input: CourierCreate) -> anyhow::Result<Courier>;

    async fn update_courier(
        &self,
        tenant_id: TenantId,
        courier_id: uuid::Uuid,
        input: CourierUpdate,
    ) -> anyhow::Result<Option<Courier>>;

    async fn delete_courier(
        &self,
        tenant_id: TenantId,
        courier_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Courier>>;
}

#[async_trait]
pub trait ChannelRepo: Send + Sync {
    async fn list_channels(
        &self,
        tenant_id: TenantId,
        filters: ChannelListFilters,
    ) -> anyhow::Result<ChannelListResult>;

    async fn create_channel(&self, input: ChannelCreate) -> anyhow::Result<Channel>;

    async fn update_channel(
        &self,
        tenant_id: TenantId,
        channel_id: uuid::Uuid,
        input: ChannelUpdate,
    ) -> anyhow::Result<Option<Channel>>;

    async fn delete_channel(
        &self,
        tenant_id: TenantId,
        channel_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Channel>>;
}

#[async_trait]
pub trait SupplierRepo: Send + Sync {
    async fn list_suppliers(
        &self,
        tenant_id: TenantId,
        filters: SupplierListFilters,
    ) -> anyhow::Result<SupplierListResult>;

    async fn create_supplier(&self, input: SupplierCreate) -> anyhow::Result<Supplier>;

    async fn update_supplier(
        &self,
        tenant_id: TenantId,
        supplier_id: uuid::Uuid,
        input: SupplierUpdate,
    ) -> anyhow::Result<Option<Supplier>>;

    async fn delete_supplier(
        &self,
        tenant_id: TenantId,
        supplier_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Supplier>>;
}

#[async_trait]
pub trait InventoryRepo: Send + Sync {
    async fn list_inventory(
        &self,
        tenant_id: TenantId,
        filters: InventoryListFilters,
    ) -> anyhow::Result<InventoryListResult>;

    async fn adjust(&self, input: InventoryAdjustment) -> anyhow::Result<InventoryBalance>;

    async fn reserve_for_order_item(
        &self,
        input: InventoryReservationInput,
    ) -> anyhow::Result<InventoryBalance>;

    async fn release_reservation(
        &self,
        input: InventoryReservationAction,
    ) -> anyhow::Result<InventoryBalance>;
}

#[async_trait]
pub trait PrintDeviceRepo: Send + Sync {
    async fn list_devices(
        &self,
        tenant_id: TenantId,
        filters: PrintDeviceListFilters,
    ) -> anyhow::Result<PrintDeviceListResult>;

    async fn create_device(&self, input: PrintDeviceCreate) -> anyhow::Result<PrintDevice>;

    async fn update_device(
        &self,
        tenant_id: TenantId,
        device_id: uuid::Uuid,
        input: PrintDeviceUpdate,
    ) -> anyhow::Result<Option<PrintDevice>>;

    async fn get_default_device(&self, tenant_id: TenantId) -> anyhow::Result<Option<PrintDevice>>;
}

#[async_trait]
pub trait PrintTemplateRepo: Send + Sync {
    async fn list_templates(
        &self,
        tenant_id: TenantId,
        filters: PrintTemplateListFilters,
    ) -> anyhow::Result<PrintTemplateListResult>;

    async fn create_template(&self, input: PrintTemplateCreate) -> anyhow::Result<PrintTemplate>;

    async fn update_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
        input: PrintTemplateUpdate,
    ) -> anyhow::Result<Option<PrintTemplate>>;

    async fn get_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintTemplate>>;

    async fn get_default_template(
        &self,
        tenant_id: TenantId,
        template_type: &str,
        shop_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<Option<PrintTemplate>>;

    async fn delete_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintTemplate>>;
}

#[async_trait]
pub trait PrintJobRepo: Send + Sync {
    async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: PrintJobListFilters,
    ) -> anyhow::Result<PrintJobListResult>;

    async fn create_job(&self, input: PrintJobCreate) -> anyhow::Result<PrintJob>;

    async fn update_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        input: PrintJobUpdate,
    ) -> anyhow::Result<Option<PrintJob>>;

    async fn get_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintJob>>;

    async fn get_job_output(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<erp_repo::print_jobs::PrintJobOutput>>;

    async fn retry_failed_job(&self, tenant_id: TenantId, job_id: uuid::Uuid)
    -> anyhow::Result<()>;

    async fn dispatch_job(&self, tenant_id: TenantId, job_id: uuid::Uuid) -> anyhow::Result<()>;
}

pub(crate) struct AfterSaleRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl AfterSaleRepo for AfterSaleRepoDb {
    async fn list_cases(
        &self,
        tenant_id: TenantId,
        filters: erp_repo::after_sale_cases::AfterSaleCaseListFilters,
    ) -> anyhow::Result<Vec<erp_domain::AfterSaleCase>> {
        let repo = AfterSaleCaseRepo::new(self.db.pool());
        repo.list_cases(tenant_id, filters).await
    }

    async fn get_case_detail(
        &self,
        tenant_id: TenantId,
        case_id: AfterSaleCaseId,
    ) -> anyhow::Result<Option<erp_domain::AfterSaleCaseDetail>> {
        let repo = AfterSaleCaseRepo::new(self.db.pool());
        repo.get_case_detail(tenant_id, case_id).await
    }

    async fn get_case_raw(
        &self,
        _tenant_id: TenantId,
        _case_id: AfterSaleCaseId,
    ) -> anyhow::Result<Value> {
        Ok(serde_json::json!({}))
    }
}

pub(crate) struct SkusRepoDb {
    pub(crate) db: Db,
}

pub(crate) struct ProductsRepoDb {
    pub(crate) db: Db,
}

pub(crate) struct SkuSuppliersRepoDb {
    pub(crate) db: Db,
}

pub(crate) struct SkuCostsRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl ProductRepo for ProductsRepoDb {
    async fn retry_copy_batch_review(
        &self,
        tenant: TenantId,
        actor: UserId,
        original_batch: uuid::Uuid,
        request_id: uuid::Uuid,
    ) -> Result<
        erp_repo::product_copy_batch_retries::RetryReview,
        erp_repo::product_copy_batch_retries::RetryError,
    > {
        erp_repo::product_copy_batch_retries::prepare_retry(
            self.db.pool(),
            tenant,
            actor,
            original_batch,
            request_id,
        )
        .await
    }
    async fn copy_batch_results(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
        after: Option<u64>,
        limit: u16,
        failed_only: bool,
    ) -> Result<
        erp_repo::product_copy_batch_results::Results,
        erp_repo::product_copy_batches::BatchError,
    > {
        erp_repo::product_copy_batch_results::page(
            self.db.pool(),
            tenant,
            actor,
            batch_id,
            after,
            limit,
            failed_only,
        )
        .await
    }
    async fn list_copy_batches(
        &self,
        tenant: TenantId,
        actor: UserId,
    ) -> Result<
        Vec<erp_repo::product_copy_batches::Status>,
        erp_repo::product_copy_batches::BatchError,
    > {
        erp_repo::product_copy_batches::list(self.db.pool(), tenant, actor).await
    }
    async fn start_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        request: &erp_repo::product_copy_batches::StartRequest,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>
    {
        erp_repo::product_copy_batches::start(self.db.pool(), tenant, actor, request).await
    }
    async fn copy_batch_status(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>
    {
        erp_repo::product_copy_batches::status(self.db.pool(), tenant, actor, batch_id).await
    }
    async fn advance_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
        max_items: u16,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>
    {
        erp_repo::product_copy_batches::advance(self.db.pool(), tenant, actor, batch_id, max_items)
            .await
    }
    async fn cancel_copy_batch(
        &self,
        tenant: TenantId,
        actor: UserId,
        batch_id: uuid::Uuid,
    ) -> Result<erp_repo::product_copy_batches::Status, erp_repo::product_copy_batches::BatchError>
    {
        erp_repo::product_copy_batches::cancel(self.db.pool(), tenant, actor, batch_id).await
    }
    async fn freeze_selection(
        &self,
        tenant: TenantId,
        actor: UserId,
        query: &erp_grid_core::paged_query::PageQuery,
    ) -> Result<
        erp_repo::product_selection::SelectionToken,
        erp_repo::product_selection::SelectionError,
    > {
        erp_repo::product_selection::freeze(self.db.pool(), tenant, actor, query).await
    }
    async fn selection_page(
        &self,
        tenant: TenantId,
        actor: UserId,
        token: uuid::Uuid,
        offset: u64,
        limit: u16,
        sorts: &[erp_grid_core::SortRule],
    ) -> Result<
        erp_repo::product_selection::SelectionPage,
        erp_repo::product_selection::SelectionError,
    > {
        erp_repo::product_selection::page(
            self.db.pool(),
            tenant,
            actor,
            token,
            offset,
            limit,
            sorts,
        )
        .await
    }
    async fn selection_page_query_order(
        &self,
        tenant: TenantId,
        actor: UserId,
        token: uuid::Uuid,
        offset: u64,
        limit: u16,
        sorts: &[erp_grid_core::SortRule],
        query_order: bool,
    ) -> Result<
        erp_repo::product_selection::SelectionPage,
        erp_repo::product_selection::SelectionError,
    > {
        if query_order {
            erp_repo::product_selection::page_query_order(
                self.db.pool(), tenant, actor, token, offset, limit, sorts,
            )
            .await
        } else {
            self.selection_page(tenant, actor, token, offset, limit, sorts)
                .await
        }
    }
    async fn selection_count(
        &self,
        tenant: TenantId,
        actor: UserId,
        token: uuid::Uuid,
        excluded: &[uuid::Uuid],
    ) -> Result<u64, erp_repo::product_selection::SelectionError> {
        erp_repo::product_selection::affected_count(self.db.pool(), tenant, actor, token, excluded)
            .await
    }
    async fn product_options(
        &self,
        tenant: TenantId,
        query: &erp_repo::product_grid_options::OptionQuery,
    ) -> anyhow::Result<erp_repo::product_grid_options::OptionPage> {
        erp_repo::product_grid_options::query_options(self.db.pool(), tenant, query).await
    }
    async fn execute_action(
        &self,
        tenant: TenantId,
        actor: UserId,
        command: &erp_grid_core::actions::ActionCommand,
    ) -> Result<erp_grid_core::actions::ActionReceipt, erp_repo::product_actions::ActionError> {
        erp_repo::product_actions::execute(self.db.pool(), tenant, actor, command).await
    }
    async fn action_receipt(
        &self,
        tenant: TenantId,
        actor: UserId,
        operation: erp_grid_core::RowId,
    ) -> Result<Option<erp_grid_core::actions::ActionReceipt>, erp_repo::product_actions::ActionError>
    {
        erp_repo::product_actions::receipt(self.db.pool(), tenant, actor, operation).await
    }
    async fn query_products(
        &self,
        tenant_id: TenantId,
        query: &erp_grid_core::paged_query::PageQuery,
    ) -> anyhow::Result<erp_repo::product_grid::ProductGridPage> {
        erp_repo::product_grid::query_products(self.db.pool(), tenant_id, query).await
    }

    async fn list_products(
        &self,
        tenant_id: TenantId,
        filters: ProductListFilters,
    ) -> anyhow::Result<ProductListResult> {
        let repo = ProductsRepo::new(self.db.pool());
        repo.list_products(tenant_id, filters).await
    }

    async fn get_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Product>> {
        let repo = ProductsRepo::new(self.db.pool());
        repo.get_product(tenant_id, product_id).await
    }

    async fn create_product(&self, input: ProductCreate) -> anyhow::Result<Product> {
        let repo = ProductsRepo::new(self.db.pool());
        repo.create_product(input).await
    }

    async fn update_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        input: ProductUpdate,
    ) -> anyhow::Result<Option<Product>> {
        let repo = ProductsRepo::new(self.db.pool());
        repo.update_product(tenant_id, product_id, input).await
    }

    async fn delete_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Product>> {
        let repo = ProductsRepo::new(self.db.pool());
        repo.delete_product(tenant_id, product_id).await
    }
}

#[async_trait]
impl SkuRepo for SkusRepoDb {
    async fn execute_grid_action(
        &self,
        tenant: TenantId,
        actor: UserId,
        command: &erp_grid_core::actions::ActionCommand,
    ) -> Result<erp_grid_core::actions::ActionReceipt, erp_repo::product_actions::ActionError> {
        erp_repo::sku_actions::execute(self.db.pool(), tenant, actor, command).await
    }

    async fn grid_action_receipt(
        &self,
        tenant: TenantId,
        actor: UserId,
        operation: erp_grid_core::RowId,
    ) -> Result<Option<erp_grid_core::actions::ActionReceipt>, erp_repo::product_actions::ActionError>
    {
        erp_repo::sku_actions::receipt(self.db.pool(), tenant, actor, operation).await
    }

    async fn query_skus(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        query: &erp_grid_core::paged_query::PageQuery,
    ) -> anyhow::Result<erp_repo::sku_grid::SkuGridPage> {
        erp_repo::sku_grid::query_skus(self.db.pool(), tenant_id, product_id, query).await
    }

    async fn query_sku_ranges(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        request: &erp_repo::sku_grid::SkuGridRangesRequest,
    ) -> anyhow::Result<erp_repo::sku_grid::SkuGridRangesPage> {
        erp_repo::sku_grid::query_sku_ranges(
            self.db.pool(),
            tenant_id,
            product_id,
            request,
        )
        .await
    }

    async fn list_skus(
        &self,
        tenant_id: TenantId,
        filters: SkuListFilters,
    ) -> anyhow::Result<SkuListResult> {
        let repo = SkusRepo::new(self.db.pool());
        repo.list_skus(tenant_id, filters).await
    }

    async fn search_skus(
        &self,
        tenant_id: TenantId,
        filters: SkuSearchFilters,
    ) -> anyhow::Result<Vec<RepoSkuSearchItem>> {
        let repo = SkusRepo::new(self.db.pool());
        repo.search_skus(tenant_id, filters).await
    }

    async fn get_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Sku>> {
        let repo = SkusRepo::new(self.db.pool());
        repo.get_sku(tenant_id, sku_id).await
    }

    async fn create_sku(&self, input: SkuCreate) -> anyhow::Result<Sku> {
        let repo = SkusRepo::new(self.db.pool());
        repo.create_sku(input).await
    }

    async fn update_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        input: SkuUpdate,
    ) -> anyhow::Result<Option<Sku>> {
        let repo = SkusRepo::new(self.db.pool());
        repo.update_sku(tenant_id, sku_id, input).await
    }

    async fn delete_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Sku>> {
        let repo = SkusRepo::new(self.db.pool());
        repo.delete_sku(tenant_id, sku_id).await
    }
}

#[async_trait]
impl SkuSupplierRepo for SkuSuppliersRepoDb {
    async fn list_sku_suppliers(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<SkuSupplier>> {
        let repo = SkuSuppliersRepo::new(self.db.pool());
        repo.list_for_sku(tenant_id, sku_id).await
    }

    async fn replace_sku_suppliers(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        supplier_ids: Vec<uuid::Uuid>,
        primary_supplier_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<()> {
        let repo = SkuSuppliersRepo::new(self.db.pool());
        repo.replace_sku_suppliers(tenant_id, sku_id, supplier_ids, primary_supplier_id)
            .await
    }
}

#[async_trait]
impl SkuCostRepo for SkuCostsRepoDb {
    async fn list_cost_ledgers(
        &self,
        tenant_id: TenantId,
        filters: SkuCostLedgerListFilters,
    ) -> anyhow::Result<SkuCostLedgerListResult> {
        let repo = SkuCostsRepo::new(self.db.pool());
        repo.list_ledgers(tenant_id, filters).await
    }

    async fn adjust_cost(&self, input: SkuCostAdjustInput) -> anyhow::Result<SkuCostLedger> {
        let repo = SkuCostsRepo::new(self.db.pool());
        repo.adjust_cost(input).await
    }
}

pub(crate) struct KitsRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl KitRepo for KitsRepoDb {
    async fn list_kits(
        &self,
        tenant_id: TenantId,
        filters: KitListFilters,
    ) -> anyhow::Result<KitListResult> {
        let repo = KitsRepo::new(self.db.pool());
        repo.list_kits(tenant_id, filters).await
    }

    async fn get_kit_detail(
        &self,
        tenant_id: TenantId,
        kit_sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<KitDetail>> {
        let repo = KitsRepo::new(self.db.pool());
        repo.get_kit_detail(tenant_id, kit_sku_id).await
    }

    async fn save_kit_bom(&self, input: KitBomSaveInput) -> anyhow::Result<Option<KitDetail>> {
        let repo = KitsRepo::new(self.db.pool());
        repo.save_kit_bom(input).await
    }
}

pub(crate) struct SkuBatchesRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl SkuBatchRepo for SkuBatchesRepoDb {
    async fn list_batches(
        &self,
        tenant_id: TenantId,
        filters: SkuBatchListFilters,
    ) -> anyhow::Result<SkuBatchListResult> {
        let repo = SkuBatchesRepo::new(self.db.pool());
        repo.list_batches(tenant_id, filters).await
    }

    async fn get_batch(
        &self,
        tenant_id: TenantId,
        batch_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuBatch>> {
        let repo = SkuBatchesRepo::new(self.db.pool());
        repo.get_batch(tenant_id, batch_id).await
    }

    async fn create_batch(&self, input: SkuBatchCreate) -> anyhow::Result<SkuBatch> {
        let repo = SkuBatchesRepo::new(self.db.pool());
        repo.create_batch(input).await
    }

    async fn update_batch(
        &self,
        tenant_id: TenantId,
        batch_id: uuid::Uuid,
        input: SkuBatchUpdate,
    ) -> anyhow::Result<Option<SkuBatch>> {
        let repo = SkuBatchesRepo::new(self.db.pool());
        repo.update_batch(tenant_id, batch_id, input).await
    }
}

pub(crate) struct SkuSerialsRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl SkuSerialRepo for SkuSerialsRepoDb {
    async fn list_serials(
        &self,
        tenant_id: TenantId,
        filters: SkuSerialListFilters,
    ) -> anyhow::Result<SkuSerialListResult> {
        let repo = SkuSerialsRepo::new(self.db.pool());
        repo.list_serials(tenant_id, filters).await
    }

    async fn generate_serials(
        &self,
        input: SkuSerialGenerateInput,
    ) -> anyhow::Result<Vec<SkuSerial>> {
        let repo = SkuSerialsRepo::new(self.db.pool());
        repo.generate_serials(input).await
    }

    async fn bind_serials_to_shipment(
        &self,
        input: RepoShipmentSerialBindInput,
    ) -> anyhow::Result<Vec<SkuSerial>> {
        let repo = SkuSerialsRepo::new(self.db.pool());
        repo.bind_serials_to_shipment(input).await
    }

    async fn count_shipment_serials(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<ShipmentSerialCount>> {
        let repo = SkuSerialsRepo::new(self.db.pool());
        repo.count_shipment_serials(tenant_id, shipment_id).await
    }

    async fn mark_shipment_serials_shipped(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        let repo = SkuSerialsRepo::new(self.db.pool());
        repo.mark_shipment_serials_shipped(tenant_id, shipment_id)
            .await
    }
}

pub(crate) struct ChannelSkuMappingsRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl ChannelSkuMappingRepo for ChannelSkuMappingsRepoDb {
    async fn list_mappings(
        &self,
        tenant_id: TenantId,
        filters: ChannelSkuMappingListFilters,
    ) -> anyhow::Result<ChannelSkuMappingListResult> {
        let repo = ChannelSkuMappingsRepo::new(self.db.pool());
        repo.list_mappings(tenant_id, filters).await
    }

    async fn create_mapping(
        &self,
        input: ChannelSkuMappingCreate,
    ) -> anyhow::Result<ChannelSkuMapping> {
        let repo = ChannelSkuMappingsRepo::new(self.db.pool());
        repo.create_mapping(input).await
    }

    async fn update_mapping(
        &self,
        tenant_id: TenantId,
        mapping_id: uuid::Uuid,
        input: ChannelSkuMappingUpdate,
    ) -> anyhow::Result<Option<ChannelSkuMapping>> {
        let repo = ChannelSkuMappingsRepo::new(self.db.pool());
        repo.update_mapping(tenant_id, mapping_id, input).await
    }
}

pub(crate) struct WarehousesRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl WarehouseRepo for WarehousesRepoDb {
    async fn list_warehouses(
        &self,
        tenant_id: TenantId,
        filters: WarehouseListFilters,
    ) -> anyhow::Result<WarehouseListResult> {
        let repo = WarehousesRepo::new(self.db.pool());
        repo.list_warehouses(tenant_id, filters).await
    }

    async fn create_warehouse(&self, input: WarehouseCreate) -> anyhow::Result<Warehouse> {
        let repo = WarehousesRepo::new(self.db.pool());
        repo.create_warehouse(input).await
    }

    async fn update_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        input: WarehouseUpdate,
    ) -> anyhow::Result<Option<Warehouse>> {
        let repo = WarehousesRepo::new(self.db.pool());
        repo.update_warehouse(tenant_id, warehouse_id, input).await
    }

    async fn delete_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Warehouse>> {
        let repo = WarehousesRepo::new(self.db.pool());
        repo.delete_warehouse(tenant_id, warehouse_id).await
    }

    async fn set_default_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Warehouse>> {
        let repo = WarehousesRepo::new(self.db.pool());
        repo.set_default_warehouse(tenant_id, warehouse_id).await
    }

    async fn get_default_warehouse(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Option<Warehouse>> {
        let repo = WarehousesRepo::new(self.db.pool());
        repo.get_default_warehouse(tenant_id).await
    }
}

pub(crate) struct CouriersRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl CourierRepo for CouriersRepoDb {
    async fn list_couriers(
        &self,
        tenant_id: TenantId,
        filters: CourierListFilters,
    ) -> anyhow::Result<CourierListResult> {
        let repo = CouriersRepo::new(self.db.pool());
        repo.list_couriers(tenant_id, filters).await
    }

    async fn create_courier(&self, input: CourierCreate) -> anyhow::Result<Courier> {
        let repo = CouriersRepo::new(self.db.pool());
        repo.create_courier(input).await
    }

    async fn update_courier(
        &self,
        tenant_id: TenantId,
        courier_id: uuid::Uuid,
        input: CourierUpdate,
    ) -> anyhow::Result<Option<Courier>> {
        let repo = CouriersRepo::new(self.db.pool());
        repo.update_courier(tenant_id, courier_id, input).await
    }

    async fn delete_courier(
        &self,
        tenant_id: TenantId,
        courier_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Courier>> {
        let repo = CouriersRepo::new(self.db.pool());
        repo.delete_courier(tenant_id, courier_id).await
    }
}

pub(crate) struct ChannelsRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl ChannelRepo for ChannelsRepoDb {
    async fn list_channels(
        &self,
        tenant_id: TenantId,
        filters: ChannelListFilters,
    ) -> anyhow::Result<ChannelListResult> {
        let repo = ChannelsRepo::new(self.db.pool());
        repo.list_channels(tenant_id, filters).await
    }

    async fn create_channel(&self, input: ChannelCreate) -> anyhow::Result<Channel> {
        let repo = ChannelsRepo::new(self.db.pool());
        repo.create_channel(input).await
    }

    async fn update_channel(
        &self,
        tenant_id: TenantId,
        channel_id: uuid::Uuid,
        input: ChannelUpdate,
    ) -> anyhow::Result<Option<Channel>> {
        let repo = ChannelsRepo::new(self.db.pool());
        repo.update_channel(tenant_id, channel_id, input).await
    }

    async fn delete_channel(
        &self,
        tenant_id: TenantId,
        channel_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Channel>> {
        let repo = ChannelsRepo::new(self.db.pool());
        repo.delete_channel(tenant_id, channel_id).await
    }
}

pub(crate) struct SuppliersRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl SupplierRepo for SuppliersRepoDb {
    async fn list_suppliers(
        &self,
        tenant_id: TenantId,
        filters: SupplierListFilters,
    ) -> anyhow::Result<SupplierListResult> {
        let repo = SuppliersRepo::new(self.db.pool());
        repo.list_suppliers(tenant_id, filters).await
    }

    async fn create_supplier(&self, input: SupplierCreate) -> anyhow::Result<Supplier> {
        let repo = SuppliersRepo::new(self.db.pool());
        repo.create_supplier(input).await
    }

    async fn update_supplier(
        &self,
        tenant_id: TenantId,
        supplier_id: uuid::Uuid,
        input: SupplierUpdate,
    ) -> anyhow::Result<Option<Supplier>> {
        let repo = SuppliersRepo::new(self.db.pool());
        repo.update_supplier(tenant_id, supplier_id, input).await
    }

    async fn delete_supplier(
        &self,
        tenant_id: TenantId,
        supplier_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Supplier>> {
        let repo = SuppliersRepo::new(self.db.pool());
        repo.delete_supplier(tenant_id, supplier_id).await
    }
}

pub(crate) struct InventoryRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl InventoryRepo for InventoryRepoDb {
    async fn list_inventory(
        &self,
        tenant_id: TenantId,
        filters: InventoryListFilters,
    ) -> anyhow::Result<InventoryListResult> {
        let repo = InventoryRepoCore::new(self.db.pool());
        repo.list_inventory(tenant_id, filters).await
    }

    async fn adjust(&self, input: InventoryAdjustment) -> anyhow::Result<InventoryBalance> {
        let repo = InventoryRepoCore::new(self.db.pool());
        repo.adjust(input).await
    }

    async fn reserve_for_order_item(
        &self,
        input: InventoryReservationInput,
    ) -> anyhow::Result<InventoryBalance> {
        let repo = InventoryRepoCore::new(self.db.pool());
        repo.reserve_for_order_item(input).await
    }

    async fn release_reservation(
        &self,
        input: InventoryReservationAction,
    ) -> anyhow::Result<InventoryBalance> {
        let repo = InventoryRepoCore::new(self.db.pool());
        repo.release_reservation(input).await
    }
}

pub(crate) struct PrintDevicesRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl PrintDeviceRepo for PrintDevicesRepoDb {
    async fn list_devices(
        &self,
        tenant_id: TenantId,
        filters: PrintDeviceListFilters,
    ) -> anyhow::Result<PrintDeviceListResult> {
        let repo = PrintDevicesRepoCore::new(self.db.pool());
        repo.list_devices(tenant_id, filters).await
    }

    async fn create_device(&self, input: PrintDeviceCreate) -> anyhow::Result<PrintDevice> {
        let repo = PrintDevicesRepoCore::new(self.db.pool());
        repo.create_device(input).await
    }

    async fn update_device(
        &self,
        tenant_id: TenantId,
        device_id: uuid::Uuid,
        input: PrintDeviceUpdate,
    ) -> anyhow::Result<Option<PrintDevice>> {
        let repo = PrintDevicesRepoCore::new(self.db.pool());
        repo.update_device(tenant_id, device_id, input).await
    }

    async fn get_default_device(&self, tenant_id: TenantId) -> anyhow::Result<Option<PrintDevice>> {
        let repo = PrintDevicesRepoCore::new(self.db.pool());
        repo.get_default_device(tenant_id).await
    }
}

pub(crate) struct PrintTemplatesRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl PrintTemplateRepo for PrintTemplatesRepoDb {
    async fn list_templates(
        &self,
        tenant_id: TenantId,
        filters: PrintTemplateListFilters,
    ) -> anyhow::Result<PrintTemplateListResult> {
        let repo = PrintTemplatesRepoCore::new(self.db.pool());
        repo.list_templates(tenant_id, filters).await
    }

    async fn create_template(&self, input: PrintTemplateCreate) -> anyhow::Result<PrintTemplate> {
        let repo = PrintTemplatesRepoCore::new(self.db.pool());
        repo.create_template(input).await
    }

    async fn update_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
        input: PrintTemplateUpdate,
    ) -> anyhow::Result<Option<PrintTemplate>> {
        let repo = PrintTemplatesRepoCore::new(self.db.pool());
        repo.update_template(tenant_id, template_id, input).await
    }

    async fn get_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintTemplate>> {
        let repo = PrintTemplatesRepoCore::new(self.db.pool());
        repo.get_template(tenant_id, template_id).await
    }

    async fn get_default_template(
        &self,
        tenant_id: TenantId,
        template_type: &str,
        shop_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<Option<PrintTemplate>> {
        let repo = PrintTemplatesRepoCore::new(self.db.pool());
        repo.get_default_template(tenant_id, template_type, shop_id)
            .await
    }

    async fn delete_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintTemplate>> {
        let repo = PrintTemplatesRepoCore::new(self.db.pool());
        repo.delete_template(tenant_id, template_id).await
    }
}

pub(crate) struct PrintJobsRepoDb {
    pub(crate) db: Db,
}

#[async_trait]
impl PrintJobRepo for PrintJobsRepoDb {
    async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: PrintJobListFilters,
    ) -> anyhow::Result<PrintJobListResult> {
        let repo = PrintJobsRepoCore::new(self.db.pool());
        repo.list_jobs(tenant_id, filters).await
    }

    async fn create_job(&self, input: PrintJobCreate) -> anyhow::Result<PrintJob> {
        let repo = PrintJobsRepoCore::new(self.db.pool());
        repo.create_job(input).await
    }

    async fn update_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        input: PrintJobUpdate,
    ) -> anyhow::Result<Option<PrintJob>> {
        let repo = PrintJobsRepoCore::new(self.db.pool());
        repo.update_job(tenant_id, job_id, input).await
    }

    async fn get_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintJob>> {
        let repo = PrintJobsRepoCore::new(self.db.pool());
        repo.get_job(tenant_id, job_id).await
    }

    async fn get_job_output(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<Option<erp_repo::print_jobs::PrintJobOutput>> {
        let repo = PrintJobsRepoCore::new(self.db.pool());
        repo.get_job_output(tenant_id, job_id).await
    }

    async fn retry_failed_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        let repo = PrintJobsRepoCore::new(self.db.pool());
        repo.retry_failed_job(tenant_id, job_id).await
    }

    async fn dispatch_job(&self, tenant_id: TenantId, job_id: uuid::Uuid) -> anyhow::Result<()> {
        let repo = PrintJobsRepoCore::new(self.db.pool());
        repo.dispatch_job(tenant_id, job_id).await
    }
}
