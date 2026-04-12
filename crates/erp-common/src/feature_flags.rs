use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureScope {
    Tenant,
    Shop,
}

impl FeatureScope {
    pub fn as_str(self) -> &'static str {
        match self {
            FeatureScope::Tenant => "TENANT",
            FeatureScope::Shop => "SHOP",
        }
    }
}

impl fmt::Display for FeatureScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureFlagKey {
    OrderSync,
    OrderMerge,
    ShippingWrite,
    InventoryWrite,
    InventorySync,
    Printing,
    PrintLocalAgent,
    AftersalesWrite,
    AfterSalesSync,
    LogisticsPrice,
    TagSystem,
    WaybillAutoReserve,
    AutoFreeze,
    PackPerformance,
    WmsWave,
    WmsOutboundInspection,
    WmsShipmentCancel,
    PickRecommend,
    StockCountSuggest,
    CostManagement,
}

impl FeatureFlagKey {
    pub fn as_str(self) -> &'static str {
        match self {
            FeatureFlagKey::OrderSync => "ORDER_SYNC",
            FeatureFlagKey::OrderMerge => "ORDER_MERGE",
            FeatureFlagKey::ShippingWrite => "SHIPPING_WRITE",
            FeatureFlagKey::InventoryWrite => "INVENTORY_WRITE",
            FeatureFlagKey::InventorySync => "INVENTORY_SYNC",
            FeatureFlagKey::Printing => "PRINTING",
            FeatureFlagKey::PrintLocalAgent => "PRINT_LOCAL_AGENT",
            FeatureFlagKey::AftersalesWrite => "AFTERSALES_WRITE",
            FeatureFlagKey::AfterSalesSync => "AFTERSALES_SYNC",
            FeatureFlagKey::LogisticsPrice => "LOGISTICS_PRICE",
            FeatureFlagKey::TagSystem => "TAG_SYSTEM",
            FeatureFlagKey::WaybillAutoReserve => "WAYBILL_AUTO_RESERVE",
            FeatureFlagKey::AutoFreeze => "AUTO_FREEZE",
            FeatureFlagKey::PackPerformance => "PACK_PERFORMANCE",
            FeatureFlagKey::WmsWave => "WMS_WAVE",
            FeatureFlagKey::WmsOutboundInspection => "WMS_OUTBOUND_INSPECTION",
            FeatureFlagKey::WmsShipmentCancel => "WMS_SHIPMENT_CANCEL",
            FeatureFlagKey::PickRecommend => "PICK_RECOMMEND",
            FeatureFlagKey::StockCountSuggest => "STOCK_COUNT_SUGGEST",
            FeatureFlagKey::CostManagement => "COST_MANAGEMENT",
        }
    }
}

impl fmt::Display for FeatureFlagKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn pdd_only_mode_env() -> Option<bool> {
    let raw = std::env::var("PDD_ONLY_MODE").ok()?;
    let enabled = matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    );
    Some(enabled)
}
