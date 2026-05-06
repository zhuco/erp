use super::*;

#[derive(Clone, Debug)]
pub(crate) struct AuthContext {
    pub(crate) tenant_id: TenantId,
    #[allow(dead_code)]
    pub(crate) user_id: UserId,
    pub(crate) perms: Vec<String>,
    pub(crate) is_admin: bool,
    pub(crate) shop_ids: Vec<ShopId>,
    pub(crate) warehouse_ids: Vec<uuid::Uuid>,
    pub(crate) merchant_ids: Vec<uuid::Uuid>,
    pub(crate) organization_ids: Vec<uuid::Uuid>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct DeviceAuthContext {
    pub(crate) tenant_id: TenantId,
    pub(crate) device_id: uuid::Uuid,
    pub(crate) fingerprint: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug)]
pub(crate) struct RequestContext {
    #[allow(dead_code)]
    pub(crate) request_id: String,
    pub(crate) trace_id: String,
    pub(crate) ip: Option<String>,
    pub(crate) user_agent: Option<String>,
}
