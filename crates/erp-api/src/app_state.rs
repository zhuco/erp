use super::*;

#[derive(Clone)]
pub struct AppState {
    pub(crate) db: Db,
    pub(crate) jwt: Arc<JwtService>,
    #[allow(dead_code)]
    pub(crate) nats: Option<async_nats::Client>,
    pub(crate) after_sale_repo: Arc<dyn AfterSaleRepo>,
    pub(crate) products_repo: Arc<dyn ProductRepo>,
    pub(crate) skus_repo: Arc<dyn SkuRepo>,
    pub(crate) sku_suppliers_repo: Arc<dyn SkuSupplierRepo>,
    pub(crate) sku_costs_repo: Arc<dyn SkuCostRepo>,
    pub(crate) channel_sku_mappings_repo: Arc<dyn ChannelSkuMappingRepo>,
    pub(crate) kits_repo: Arc<dyn KitRepo>,
    pub(crate) sku_batches_repo: Arc<dyn SkuBatchRepo>,
    pub(crate) sku_serials_repo: Arc<dyn SkuSerialRepo>,
    pub(crate) warehouses_repo: Arc<dyn WarehouseRepo>,
    pub(crate) couriers_repo: Arc<dyn CourierRepo>,
    pub(crate) channels_repo: Arc<dyn ChannelRepo>,
    pub(crate) suppliers_repo: Arc<dyn SupplierRepo>,
    pub(crate) inventory_repo: Arc<dyn InventoryRepo>,
    pub(crate) print_devices_repo: Arc<dyn PrintDeviceRepo>,
    pub(crate) print_templates_repo: Arc<dyn PrintTemplateRepo>,
    pub(crate) print_jobs_repo: Arc<dyn PrintJobRepo>,
}

impl AppState {
    pub fn new(db: Db, jwt: Arc<JwtService>) -> Self {
        Self::new_with_nats(db, jwt, None)
    }

    pub fn new_with_nats(db: Db, jwt: Arc<JwtService>, nats: Option<async_nats::Client>) -> Self {
        let after_sale_repo = Arc::new(AfterSaleRepoDb { db: db.clone() });
        let products_repo = Arc::new(ProductsRepoDb { db: db.clone() });
        let skus_repo = Arc::new(SkusRepoDb { db: db.clone() });
        let sku_suppliers_repo = Arc::new(SkuSuppliersRepoDb { db: db.clone() });
        let sku_costs_repo = Arc::new(SkuCostsRepoDb { db: db.clone() });
        let channel_sku_mappings_repo = Arc::new(ChannelSkuMappingsRepoDb { db: db.clone() });
        let kits_repo = Arc::new(KitsRepoDb { db: db.clone() });
        let sku_batches_repo = Arc::new(SkuBatchesRepoDb { db: db.clone() });
        let sku_serials_repo = Arc::new(SkuSerialsRepoDb { db: db.clone() });
        let warehouses_repo = Arc::new(WarehousesRepoDb { db: db.clone() });
        let couriers_repo = Arc::new(CouriersRepoDb { db: db.clone() });
        let channels_repo = Arc::new(ChannelsRepoDb { db: db.clone() });
        let suppliers_repo = Arc::new(SuppliersRepoDb { db: db.clone() });
        let inventory_repo = Arc::new(InventoryRepoDb { db: db.clone() });
        let print_devices_repo = Arc::new(PrintDevicesRepoDb { db: db.clone() });
        let print_templates_repo = Arc::new(PrintTemplatesRepoDb { db: db.clone() });
        let print_jobs_repo = Arc::new(PrintJobsRepoDb { db: db.clone() });
        Self {
            db,
            jwt,
            nats,
            after_sale_repo,
            products_repo,
            skus_repo,
            sku_suppliers_repo,
            sku_costs_repo,
            channel_sku_mappings_repo,
            kits_repo,
            sku_batches_repo,
            sku_serials_repo,
            warehouses_repo,
            couriers_repo,
            channels_repo,
            suppliers_repo,
            inventory_repo,
            print_devices_repo,
            print_templates_repo,
            print_jobs_repo,
        }
    }

    pub fn with_after_sale_repo(
        db: Db,
        jwt: Arc<JwtService>,
        after_sale_repo: Arc<dyn AfterSaleRepo>,
    ) -> Self {
        let nats = None;
        let products_repo = Arc::new(ProductsRepoDb { db: db.clone() });
        let skus_repo = Arc::new(SkusRepoDb { db: db.clone() });
        let sku_suppliers_repo = Arc::new(SkuSuppliersRepoDb { db: db.clone() });
        let sku_costs_repo = Arc::new(SkuCostsRepoDb { db: db.clone() });
        let channel_sku_mappings_repo = Arc::new(ChannelSkuMappingsRepoDb { db: db.clone() });
        let kits_repo = Arc::new(KitsRepoDb { db: db.clone() });
        let sku_batches_repo = Arc::new(SkuBatchesRepoDb { db: db.clone() });
        let sku_serials_repo = Arc::new(SkuSerialsRepoDb { db: db.clone() });
        let warehouses_repo = Arc::new(WarehousesRepoDb { db: db.clone() });
        let couriers_repo = Arc::new(CouriersRepoDb { db: db.clone() });
        let channels_repo = Arc::new(ChannelsRepoDb { db: db.clone() });
        let suppliers_repo = Arc::new(SuppliersRepoDb { db: db.clone() });
        let inventory_repo = Arc::new(InventoryRepoDb { db: db.clone() });
        let print_devices_repo = Arc::new(PrintDevicesRepoDb { db: db.clone() });
        let print_templates_repo = Arc::new(PrintTemplatesRepoDb { db: db.clone() });
        let print_jobs_repo = Arc::new(PrintJobsRepoDb { db: db.clone() });
        Self {
            db,
            jwt,
            nats,
            after_sale_repo,
            products_repo,
            skus_repo,
            sku_suppliers_repo,
            sku_costs_repo,
            channel_sku_mappings_repo,
            kits_repo,
            sku_batches_repo,
            sku_serials_repo,
            warehouses_repo,
            couriers_repo,
            channels_repo,
            suppliers_repo,
            inventory_repo,
            print_devices_repo,
            print_templates_repo,
            print_jobs_repo,
        }
    }

    pub fn with_sku_repos(
        db: Db,
        jwt: Arc<JwtService>,
        skus_repo: Arc<dyn SkuRepo>,
        channel_sku_mappings_repo: Arc<dyn ChannelSkuMappingRepo>,
    ) -> Self {
        let nats = None;
        let after_sale_repo = Arc::new(AfterSaleRepoDb { db: db.clone() });
        let products_repo = Arc::new(ProductsRepoDb { db: db.clone() });
        let sku_suppliers_repo = Arc::new(SkuSuppliersRepoDb { db: db.clone() });
        let sku_costs_repo = Arc::new(SkuCostsRepoDb { db: db.clone() });
        let warehouses_repo = Arc::new(WarehousesRepoDb { db: db.clone() });
        let couriers_repo = Arc::new(CouriersRepoDb { db: db.clone() });
        let channels_repo = Arc::new(ChannelsRepoDb { db: db.clone() });
        let suppliers_repo = Arc::new(SuppliersRepoDb { db: db.clone() });
        let inventory_repo = Arc::new(InventoryRepoDb { db: db.clone() });
        let print_devices_repo = Arc::new(PrintDevicesRepoDb { db: db.clone() });
        let print_templates_repo = Arc::new(PrintTemplatesRepoDb { db: db.clone() });
        let print_jobs_repo = Arc::new(PrintJobsRepoDb { db: db.clone() });
        let kits_repo = Arc::new(KitsRepoDb { db: db.clone() });
        let sku_batches_repo = Arc::new(SkuBatchesRepoDb { db: db.clone() });
        let sku_serials_repo = Arc::new(SkuSerialsRepoDb { db: db.clone() });
        Self {
            db,
            jwt,
            nats,
            after_sale_repo,
            products_repo,
            skus_repo,
            sku_suppliers_repo,
            sku_costs_repo,
            channel_sku_mappings_repo,
            kits_repo,
            sku_batches_repo,
            sku_serials_repo,
            warehouses_repo,
            couriers_repo,
            channels_repo,
            suppliers_repo,
            inventory_repo,
            print_devices_repo,
            print_templates_repo,
            print_jobs_repo,
        }
    }
}
