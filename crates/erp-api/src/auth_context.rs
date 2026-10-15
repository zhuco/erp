use super::*;

/// Captured only by the verified bearer middleware, never from a business body.
#[derive(Clone, Copy, Debug)]
pub(crate) struct VerifiedCredentialEpoch(pub(crate) Option<i32>);

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
    /// Present only when the token named an opened session. Stock commands reread that row.
    pub(crate) session_id: Option<uuid::Uuid>,
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

impl AuthContext {
    pub(crate) fn from_claims(
        tenant_id: TenantId,
        user_id: UserId,
        perms: Vec<String>,
        is_admin: bool,
        shop_ids: Vec<ShopId>,
        warehouse_ids: Vec<uuid::Uuid>,
        merchant_ids: Vec<uuid::Uuid>,
        organization_ids: Vec<uuid::Uuid>,
        session_id: Option<uuid::Uuid>,
    ) -> Self {
        Self {
            tenant_id,
            user_id,
            perms,
            is_admin,
            shop_ids,
            warehouse_ids,
            merchant_ids,
            organization_ids,
            session_id,
        }
    }

    pub(crate) fn production_release_actor(
        &self,
        request: &RequestContext,
    ) -> erp_repo::shared_contract::ProductionReleaseActor {
        erp_repo::shared_contract::ProductionReleaseActor::authenticated(
            erp_repo::shared_contract::ProductionReleaseUser {
                authentication_tenant: self.tenant_id,
                user_id: self.user_id,
                is_admin: self.is_admin,
                request_id: request.request_id.clone(),
                trace_id: request.trace_id.clone(),
                ip: request.ip.clone(),
                user_agent: request.user_agent.clone(),
            },
        )
    }

    pub(crate) fn fulfillment_actor(
        &self,
        request: &RequestContext,
    ) -> erp_repo::fulfillment::FulfillmentActor {
        erp_repo::fulfillment::FulfillmentActor::authenticated(
            erp_repo::fulfillment::FulfillmentUser {
                authentication_tenant: self.tenant_id,
                user_id: self.user_id,
                is_admin: self.is_admin,
                request_id: request.request_id.clone(),
                trace_id: request.trace_id.clone(),
                ip: request.ip.clone(),
                user_agent: request.user_agent.clone(),
                session_id: self.session_id,
            },
        )
    }
}
