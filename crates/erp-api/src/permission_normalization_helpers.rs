use super::*;

pub(crate) fn normalize_permission_code(perm: &str) -> String {
    match perm {
        "orders.read" => PermissionCode::OmsOrderRead.as_str().to_string(),
        "orders.write" => PermissionCode::OmsOrderWrite.as_str().to_string(),
        "orders.read_raw" => PermissionCode::OmsOrderRaw.as_str().to_string(),
        "orders.review" => PermissionCode::OmsOrderReview.as_str().to_string(),
        "orders.download" => PermissionCode::OmsOrderWrite.as_str().to_string(),
        "orders.create" => PermissionCode::OmsOrderWrite.as_str().to_string(),
        "orders.recheck" => PermissionCode::OmsOrderReview.as_str().to_string(),
        "orders.submit" => PermissionCode::OmsOrderWrite.as_str().to_string(),
        "shipping.read" => PermissionCode::OmsShippingRead.as_str().to_string(),
        "shipping.write" => PermissionCode::OmsShippingWrite.as_str().to_string(),
        "aftersales.read" => PermissionCode::OmsAfterSalesRead.as_str().to_string(),
        "aftersales.write" => PermissionCode::OmsAfterSalesWrite.as_str().to_string(),
        "aftersales.read_raw" => PermissionCode::OmsAfterSalesRaw.as_str().to_string(),
        "manufacturing.write" => PermissionCode::ManufacturingWrite.as_str().to_string(),
        "quality.concession" => PermissionCode::QualityConcession.as_str().to_string(),
        "inventory.read" => PermissionCode::WmsInventoryRead.as_str().to_string(),
        "inventory.write" => PermissionCode::WmsInventoryWrite.as_str().to_string(),
        "wms.read" => PermissionCode::WmsInventoryRead.as_str().to_string(),
        "wms.write" => PermissionCode::WmsInventoryWrite.as_str().to_string(),
        "wms.inbounds.read" => PermissionCode::WmsInventoryRead.as_str().to_string(),
        "wms.inbounds.write" => PermissionCode::WmsInventoryWrite.as_str().to_string(),
        "skus.read" => PermissionCode::BasicSkuRead.as_str().to_string(),
        "skus.write" => PermissionCode::BasicSkuWrite.as_str().to_string(),
        "kits.read" => PermissionCode::BasicKitRead.as_str().to_string(),
        "kits.write" => PermissionCode::BasicKitWrite.as_str().to_string(),
        "channels.read" => PermissionCode::BasicChannelRead.as_str().to_string(),
        "channels.write" => PermissionCode::BasicChannelWrite.as_str().to_string(),
        "couriers.read" => PermissionCode::BasicCourierRead.as_str().to_string(),
        "couriers.write" => PermissionCode::BasicCourierWrite.as_str().to_string(),
        "suppliers.read" => PermissionCode::BasicSupplierRead.as_str().to_string(),
        "suppliers.write" => PermissionCode::BasicSupplierWrite.as_str().to_string(),
        "customers.read" => PermissionCode::BasicCustomerRead.as_str().to_string(),
        "customers.write" => PermissionCode::BasicCustomerWrite.as_str().to_string(),
        "purchase_orders.read" => PermissionCode::ProcurementOrderRead.as_str().to_string(),
        "purchase_orders.write" => PermissionCode::ProcurementOrderWrite.as_str().to_string(),
        "purchase_suggestions.read" => PermissionCode::ProcurementSuggestionRead
            .as_str()
            .to_string(),
        "purchase_returns.read" => PermissionCode::ProcurementReturnRead.as_str().to_string(),
        "purchase_returns.write" => PermissionCode::ProcurementReturnWrite.as_str().to_string(),
        "procurement_requests.read" => PermissionCode::ProcurementRequestRead.as_str().to_string(),
        "procurement_requests.write" => {
            PermissionCode::ProcurementRequestWrite.as_str().to_string()
        }
        "procurement_plans.read" => PermissionCode::ProcurementPlanRead.as_str().to_string(),
        "procurement_plans.write" => PermissionCode::ProcurementPlanWrite.as_str().to_string(),
        "procurement_sourcing.read" => PermissionCode::ProcurementSourcingRead.as_str().to_string(),
        "procurement_sourcing.write" => PermissionCode::ProcurementSourcingWrite
            .as_str()
            .to_string(),
        "procurement_inspections.read" => PermissionCode::ProcurementInspectionRead
            .as_str()
            .to_string(),
        "procurement_inspections.write" => PermissionCode::ProcurementInspectionWrite
            .as_str()
            .to_string(),
        "procurement_reconciliations.read" => PermissionCode::ProcurementReconciliationRead
            .as_str()
            .to_string(),
        "procurement_reconciliations.write" => PermissionCode::ProcurementReconciliationWrite
            .as_str()
            .to_string(),
        "supplier_performances.read" => PermissionCode::ProcurementPerformanceRead
            .as_str()
            .to_string(),
        "supplier_performances.write" => PermissionCode::ProcurementPerformanceWrite
            .as_str()
            .to_string(),
        "channel_sku_mappings.read" => PermissionCode::BasicMappingRead.as_str().to_string(),
        "channel_sku_mappings.write" => PermissionCode::BasicMappingWrite.as_str().to_string(),
        "mapping_rules.read" => PermissionCode::BasicMappingRead.as_str().to_string(),
        "mapping_rules.write" => PermissionCode::BasicMappingWrite.as_str().to_string(),
        "warehouses.read" => PermissionCode::BasicWarehouseRead.as_str().to_string(),
        "warehouses.write" => PermissionCode::BasicWarehouseWrite.as_str().to_string(),
        "carriers.read" => PermissionCode::BasicCarrierRead.as_str().to_string(),
        "carriers.write" => PermissionCode::BasicCarrierWrite.as_str().to_string(),
        "logistics_products.read" => PermissionCode::BasicLogisticsProductRead
            .as_str()
            .to_string(),
        "logistics_products.write" => PermissionCode::BasicLogisticsProductWrite
            .as_str()
            .to_string(),
        "logistics_prices.read" => PermissionCode::BasicLogisticsPriceRead.as_str().to_string(),
        "logistics_prices.write" => PermissionCode::BasicLogisticsPriceWrite
            .as_str()
            .to_string(),
        "logistics_match_rules.read" => PermissionCode::BasicLogisticsMatchRuleRead
            .as_str()
            .to_string(),
        "logistics_match_rules.write" => PermissionCode::BasicLogisticsMatchRuleWrite
            .as_str()
            .to_string(),
        "order_tags.read" => PermissionCode::BasicOrderTagRead.as_str().to_string(),
        "order_tags.write" => PermissionCode::BasicOrderTagWrite.as_str().to_string(),
        "sku_tags.read" => PermissionCode::BasicSkuTagRead.as_str().to_string(),
        "sku_tags.write" => PermissionCode::BasicSkuTagWrite.as_str().to_string(),
        "shop_products.read" => PermissionCode::BasicShopProductRead.as_str().to_string(),
        "shop_products.write" => PermissionCode::BasicShopProductWrite.as_str().to_string(),
        "marketing_prices.read" => PermissionCode::MarketingPriceRead.as_str().to_string(),
        "marketing_prices.write" => PermissionCode::MarketingPriceWrite.as_str().to_string(),
        "marketing_promotions.read" => PermissionCode::MarketingPromotionRead.as_str().to_string(),
        "marketing_promotions.write" => {
            PermissionCode::MarketingPromotionWrite.as_str().to_string()
        }
        "marketing_gifts.read" => PermissionCode::MarketingGiftRead.as_str().to_string(),
        "marketing_gifts.write" => PermissionCode::MarketingGiftWrite.as_str().to_string(),
        "marketing_coupons.read" => PermissionCode::MarketingCouponRead.as_str().to_string(),
        "marketing_coupons.write" => PermissionCode::MarketingCouponWrite.as_str().to_string(),
        "marketing_costs.read" => PermissionCode::MarketingCostRead.as_str().to_string(),
        "marketing_costs.write" => PermissionCode::MarketingCostWrite.as_str().to_string(),
        "marketing_postmortems.read" => {
            PermissionCode::MarketingPostmortemRead.as_str().to_string()
        }
        "marketing_postmortems.write" => PermissionCode::MarketingPostmortemWrite
            .as_str()
            .to_string(),
        "print_devices.read" => PermissionCode::PrintDeviceRead.as_str().to_string(),
        "print_devices.write" => PermissionCode::PrintDeviceWrite.as_str().to_string(),
        "print_templates.read" => PermissionCode::PrintTemplateRead.as_str().to_string(),
        "print_templates.write" => PermissionCode::PrintTemplateWrite.as_str().to_string(),
        "print_jobs.read" => PermissionCode::PrintJobRead.as_str().to_string(),
        "print_jobs.write" => PermissionCode::PrintJobWrite.as_str().to_string(),
        "print_mappings.read" => PermissionCode::PrintMappingRead.as_str().to_string(),
        "print_mappings.write" => PermissionCode::PrintMappingWrite.as_str().to_string(),
        "reports.read" => PermissionCode::OpsReportRead.as_str().to_string(),
        "stats.read" => "OPS.STATS.READ".to_string(),
        "exports.read" => PermissionCode::OpsExportRead.as_str().to_string(),
        "exports.write" => PermissionCode::OpsExportWrite.as_str().to_string(),
        "settings.read" => PermissionCode::SysSettingsRead.as_str().to_string(),
        "operation_logs.read" => PermissionCode::OpsOperationLogRead.as_str().to_string(),
        "roles.read" => PermissionCode::SysRoleRead.as_str().to_string(),
        "roles.write" => PermissionCode::SysRoleWrite.as_str().to_string(),
        "users.read" => PermissionCode::SysUserRead.as_str().to_string(),
        "users.write" => PermissionCode::SysUserWrite.as_str().to_string(),
        "permissions.read" => PermissionCode::SysPermissionRead.as_str().to_string(),
        "audit.read" => PermissionCode::SysAuditRead.as_str().to_string(),
        "dlq.read" => PermissionCode::SysDlqRead.as_str().to_string(),
        "dlq.replay" => PermissionCode::SysDlqReplay.as_str().to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn normalize_permissions(perms: Vec<String>) -> Vec<String> {
    perms
        .into_iter()
        .map(|perm| normalize_permission_code(&perm))
        .collect()
}
