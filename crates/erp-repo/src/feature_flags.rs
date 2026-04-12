use erp_common::feature_flags::FeatureScope;
use erp_types::{ShopId, TenantId};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureFlag {
    ReadOnly,
    OrderSync,
    OrderMerge,
    ShippingWrite,
    InventoryWrite,
    InventorySync,
    Printing,
    PrintLocalAgent,
    AfterSalesWrite,
    AfterSalesSync,
    LogisticsPrice,
    TagSystem,
    WaybillAutoReserve,
    AutoFreeze,
    PackPerformance,
    WmsWave,
    WmsOutboundInspection,
    WmsShipmentCancel,
    PddOnly,
    PickRecommend,
    StockCountSuggest,
    CostManagement,
}

impl FeatureFlag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "READ_ONLY",
            Self::OrderSync => "ORDER_SYNC",
            Self::OrderMerge => "ORDER_MERGE",
            Self::ShippingWrite => "SHIPPING_WRITE",
            Self::InventoryWrite => "INVENTORY_WRITE",
            Self::InventorySync => "INVENTORY_SYNC",
            Self::Printing => "PRINTING",
            Self::PrintLocalAgent => "PRINT_LOCAL_AGENT",
            Self::AfterSalesWrite => "AFTERSALES_WRITE",
            Self::AfterSalesSync => "AFTERSALES_SYNC",
            Self::LogisticsPrice => "LOGISTICS_PRICE",
            Self::TagSystem => "TAG_SYSTEM",
            Self::WaybillAutoReserve => "WAYBILL_AUTO_RESERVE",
            Self::AutoFreeze => "AUTO_FREEZE",
            Self::PackPerformance => "PACK_PERFORMANCE",
            Self::WmsWave => "WMS_WAVE",
            Self::WmsOutboundInspection => "WMS_OUTBOUND_INSPECTION",
            Self::WmsShipmentCancel => "WMS_SHIPMENT_CANCEL",
            Self::PddOnly => "PDD_ONLY",
            Self::PickRecommend => "PICK_RECOMMEND",
            Self::StockCountSuggest => "STOCK_COUNT_SUGGEST",
            Self::CostManagement => "COST_MANAGEMENT",
        }
    }

    /// Convert from erp-common FeatureFlagKey to FeatureFlag
    pub fn from_common_key(key: erp_common::feature_flags::FeatureFlagKey) -> Self {
        use erp_common::feature_flags::FeatureFlagKey;
        match key {
            FeatureFlagKey::OrderSync => Self::OrderSync,
            FeatureFlagKey::OrderMerge => Self::OrderMerge,
            FeatureFlagKey::ShippingWrite => Self::ShippingWrite,
            FeatureFlagKey::InventoryWrite => Self::InventoryWrite,
            FeatureFlagKey::InventorySync => Self::InventorySync,
            FeatureFlagKey::Printing => Self::Printing,
            FeatureFlagKey::PrintLocalAgent => Self::PrintLocalAgent,
            FeatureFlagKey::AftersalesWrite => Self::AfterSalesWrite,
            FeatureFlagKey::AfterSalesSync => Self::AfterSalesSync,
            FeatureFlagKey::LogisticsPrice => Self::LogisticsPrice,
            FeatureFlagKey::TagSystem => Self::TagSystem,
            FeatureFlagKey::WaybillAutoReserve => Self::WaybillAutoReserve,
            FeatureFlagKey::AutoFreeze => Self::AutoFreeze,
            FeatureFlagKey::PackPerformance => Self::PackPerformance,
            FeatureFlagKey::WmsWave => Self::WmsWave,
            FeatureFlagKey::WmsOutboundInspection => Self::WmsOutboundInspection,
            FeatureFlagKey::WmsShipmentCancel => Self::WmsShipmentCancel,
            FeatureFlagKey::PickRecommend => Self::PickRecommend,
            FeatureFlagKey::StockCountSuggest => Self::StockCountSuggest,
            FeatureFlagKey::CostManagement => Self::CostManagement,
        }
    }
}

pub struct FeatureFlagsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> FeatureFlagsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Check if a feature flag is enabled for the given tenant/shop.
    /// Checks in order: SHOP scope -> TENANT scope -> GLOBAL scope
    ///
    /// Special semantics:
    /// - READ_ONLY: if enabled, all write operations are blocked
    /// - PDD_ONLY: if enabled, only PDD platform requests are allowed
    /// - Other flags: if disabled, specific operation is blocked (default enabled)
    pub async fn is_enabled(
        &self,
        tenant_id: TenantId,
        shop_id: Option<ShopId>,
        flag: FeatureFlag,
    ) -> anyhow::Result<bool> {
        let key = flag.as_str();

        // Check SHOP scope if shop_id provided
        if let Some(sid) = shop_id {
            let shop_enabled: Option<bool> = sqlx::query_scalar(
                r#"
                SELECT enabled
                FROM feature_flags
                WHERE tenant_id = $1
                  AND scope = 'SHOP'
                  AND shop_id = $2
                  AND key = $3
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(sid))
            .bind(key)
            .fetch_optional(self.pool)
            .await?;

            if let Some(enabled) = shop_enabled {
                return Ok(self.apply_flag_semantics(flag, enabled));
            }
        }

        // Check TENANT scope
        let tenant_enabled: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT enabled
            FROM feature_flags
            WHERE tenant_id = $1
              AND scope = 'TENANT'
              AND shop_id IS NULL
              AND key = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(key)
        .fetch_optional(self.pool)
        .await?;

        if let Some(enabled) = tenant_enabled {
            return Ok(self.apply_flag_semantics(flag, enabled));
        }

        // Check GLOBAL scope
        let global_enabled: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT enabled
            FROM feature_flags
            WHERE scope = 'GLOBAL'
              AND shop_id IS NULL
              AND key = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(key)
        .fetch_optional(self.pool)
        .await?;
        if let Some(enabled) = global_enabled {
            return Ok(self.apply_flag_semantics(flag, enabled));
        }

        Ok(!matches!(
            flag,
            FeatureFlag::ReadOnly | FeatureFlag::PddOnly
        ))
    }

    fn apply_flag_semantics(&self, flag: FeatureFlag, db_value: bool) -> bool {
        match flag {
            // READ_ONLY and PDD_ONLY: enabled in DB means feature is active (blocking)
            FeatureFlag::ReadOnly | FeatureFlag::PddOnly => db_value,
            // Other flags: enabled in DB means operation is allowed
            _ => db_value,
        }
    }

    /// Check if READ_ONLY mode is active (blocking all writes)
    pub async fn is_read_only(
        &self,
        tenant_id: TenantId,
        shop_id: Option<ShopId>,
    ) -> anyhow::Result<bool> {
        self.is_enabled(tenant_id, shop_id, FeatureFlag::ReadOnly)
            .await
    }

    /// Check if PDD_ONLY mode is active (blocking non-PDD platforms)
    pub async fn is_pdd_only(&self, tenant_id: TenantId) -> anyhow::Result<bool> {
        self.is_enabled(tenant_id, None, FeatureFlag::PddOnly).await
    }

    pub async fn upsert_flag(
        &self,
        tenant_id: TenantId,
        scope: FeatureScope,
        shop_id: Option<ShopId>,
        flag: FeatureFlag,
        enabled: bool,
    ) -> anyhow::Result<()> {
        let tenant_uuid = uuid::Uuid::from(tenant_id);
        let shop_uuid = shop_id.map(uuid::Uuid::from);
        let updated = sqlx::query(
            r#"
            UPDATE feature_flags
               SET enabled = $5,
                   updated_at = now()
             WHERE tenant_id = $1
               AND scope = $2
               AND shop_id IS NOT DISTINCT FROM $3
               AND key = $4
            "#,
        )
        .bind(tenant_uuid)
        .bind(scope.as_str())
        .bind(shop_uuid)
        .bind(flag.as_str())
        .bind(enabled)
        .execute(self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            sqlx::query(
                r#"
                INSERT INTO feature_flags (tenant_id, scope, shop_id, key, enabled)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(tenant_uuid)
            .bind(scope.as_str())
            .bind(shop_uuid)
            .bind(flag.as_str())
            .bind(enabled)
            .execute(self.pool)
            .await?;
        }

        Ok(())
    }
}
