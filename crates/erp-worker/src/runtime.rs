use super::*;

pub(super) struct AdapterRegistry {
    taobao: erp_adapter_taobao::Adapter,
    jd: erp_adapter_jd::Adapter,
    pdd: erp_adapter_pdd::Adapter,
    douyin: erp_adapter_douyin::Adapter,
}

impl AdapterRegistry {
    pub(super) fn new() -> Self {
        Self {
            taobao: erp_adapter_taobao::Adapter::new(),
            jd: erp_adapter_jd::Adapter::new(),
            pdd: erp_adapter_pdd::Adapter::new(),
            douyin: erp_adapter_douyin::Adapter::new(),
        }
    }

    pub(super) fn platform_adapter(&self, platform: Platform) -> &dyn PlatformAdapter {
        match platform {
            Platform::Taobao => &self.taobao,
            Platform::Jd => &self.jd,
            Platform::Pdd => &self.pdd,
            Platform::Douyin => &self.douyin,
        }
    }
}

pub(super) async fn is_pdd_only_mode(db: &Db, tenant_id: TenantId) -> anyhow::Result<bool> {
    if let Some(enabled) = feature_flags::pdd_only_mode_env() {
        return Ok(enabled);
    }
    let tenants_repo = TenantsRepo::new(db.pool());
    tenants_repo.is_pdd_only_mode(tenant_id).await
}

pub(super) async fn feature_flag_enabled(
    db: &Db,
    tenant_id: TenantId,
    shop_id: Option<ShopId>,
    key: FeatureFlagKey,
) -> anyhow::Result<bool> {
    let repo = FeatureFlagsRepo::new(db.pool());
    let flag = FeatureFlag::from_common_key(key);
    repo.is_enabled(tenant_id, shop_id, flag).await
}

pub(super) async fn read_only_enabled(db: &Db, tenant_id: TenantId) -> anyhow::Result<bool> {
    let repo = FeatureFlagsRepo::new(db.pool());
    repo.is_read_only(tenant_id, None).await
}

pub(super) async fn metrics_handler() -> Response {
    let body = render_metrics();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, METRICS_CONTENT_TYPE)
        .body(Body::from(body))
        .unwrap()
}
