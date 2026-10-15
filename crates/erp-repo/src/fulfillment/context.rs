//! Trusted caller identity and mandatory records for one OMS transaction.
use std::collections::HashSet;

use erp_types::{TenantId, UserId};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    operation_logs::{OperationLogCreate, OperationLogsRepo},
};

#[derive(Debug, thiserror::Error)]
pub enum FulfillmentAccessError {
    #[error("current OMS actor or warehouse access denied")]
    Forbidden,
    #[error("OMS warehouse scope exceeds 10000 grants")]
    ScopeLimit,
    #[error("current role does not hold OMS order, shipping, and inventory write")]
    RolePermissionDenied,
    #[error("session is not active for this OMS command")]
    SessionInactive,
    #[error("session credentials are stale for this OMS command")]
    CredentialsStale,
}

/// Supplied by authenticated application code, never deserialized from a body.
#[derive(Debug, Clone)]
pub struct FulfillmentUser {
    pub authentication_tenant: TenantId,
    pub user_id: UserId,
    pub is_admin: bool,
    pub request_id: String,
    pub trace_id: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    /// Opened login session. Stock shipment rereads this row in the same transaction.
    pub session_id: Option<Uuid>,
}

/// Internal workers have no user identity; they are recorded as SYSTEM.
/// Every HTTP caller must explicitly bind its authenticated user.
#[derive(Debug, Clone, Default)]
pub struct FulfillmentActor {
    user: Option<FulfillmentUser>,
}

pub(super) struct FulfillmentAuthority {
    warehouses: Option<HashSet<Uuid>>,
}

pub(super) struct FulfillmentRecord {
    pub action: &'static str,
    pub audit_action: &'static str,
    pub entity: &'static str,
    pub id: String,
    pub metadata: Value,
}

impl FulfillmentAuthority {
    pub(super) fn allows(&self, warehouse: Uuid) -> bool {
        self.warehouses
            .as_ref()
            .is_none_or(|scope| scope.contains(&warehouse))
    }

    pub(super) fn require(&self, warehouses: impl IntoIterator<Item = Uuid>) -> anyhow::Result<()> {
        if warehouses
            .into_iter()
            .all(|warehouse| self.allows(warehouse))
        {
            Ok(())
        } else {
            Err(FulfillmentAccessError::Forbidden.into())
        }
    }
}

impl FulfillmentActor {
    pub fn authenticated(user: FulfillmentUser) -> Self {
        Self { user: Some(user) }
    }

    pub(super) async fn authority(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
    ) -> anyhow::Result<FulfillmentAuthority> {
        let Some(user) = &self.user else {
            return Ok(FulfillmentAuthority { warehouses: None });
        };
        if user.user_id.0.is_nil() || user.authentication_tenant.0.is_nil() {
            return Err(FulfillmentAccessError::Forbidden.into());
        }
        // A foreign enterprise must be refused after the shipment row is locked,
        // not before the transaction starts. Role and session rows stay on the
        // authenticated tenant; warehouse grants stay on the path tenant.
        let authenticated = user.authentication_tenant;
        // Shared actor locks are compatible across devices/orders. The scope
        // replacement writer locks this same user before deleting/inserting
        // grants, including the empty-scope (all warehouses) policy.
        let status: Option<String> = sqlx::query_scalar(
            "select status from users where tenant_id=$1 and user_id=$2 for share",
        )
        .bind(user.authentication_tenant.0)
        .bind(user.user_id.0)
        .fetch_optional(&mut **tx)
        .await?;
        if status.as_deref() != Some("ACTIVE") {
            return Err(FulfillmentAccessError::Forbidden.into());
        }
        // Recount stored roles before warehouse scope. Admin does not skip this.
        // The JWT permission list remains an earlier HTTP gate, not this proof.
        let role_grants: i64 = sqlx::query_scalar(
            r#"
            select count(distinct rp.perm_code)
            from user_roles ur
            join role_permissions rp
              on rp.tenant_id = ur.tenant_id and rp.role_id = ur.role_id
            where ur.tenant_id = $1
              and ur.user_id = $2
              and rp.perm_code in ('OMS.ORDER.WRITE', 'OMS.SHIPPING.WRITE', 'WMS.INVENTORY.WRITE')
            "#,
        )
        .bind(user.authentication_tenant.0)
        .bind(user.user_id.0)
        .fetch_one(&mut **tx)
        .await?;
        if role_grants != 3 {
            return Err(FulfillmentAccessError::RolePermissionDenied.into());
        }
        let session_id = user
            .session_id
            .filter(|id| !id.is_nil())
            .ok_or(FulfillmentAccessError::SessionInactive)?;
        let session: Option<(String, bool, i32, i32)> = sqlx::query_as(
            r#"
            select s.status, s.expires_at > now(), s.credentials_epoch, t.credentials_epoch
            from auth_sessions s
            join tenants t on t.tenant_id = s.tenant_id
            where s.tenant_id = $1 and s.user_id = $2 and s.session_id = $3
            for share of s
            "#,
        )
        .bind(authenticated.0)
        .bind(user.user_id.0)
        .bind(session_id)
        .fetch_optional(&mut **tx)
        .await?;
        let Some((status, not_expired, token_epoch, current_epoch)) = session else {
            return Err(FulfillmentAccessError::SessionInactive.into());
        };
        match erp_domain::access::session_command_allowed(
            status == "ACTIVE",
            not_expired,
            token_epoch,
            current_epoch,
        ) {
            Ok(()) => {}
            Err(erp_domain::access::AccessError::CredentialsStale) => {
                return Err(FulfillmentAccessError::CredentialsStale.into());
            }
            Err(_) => return Err(FulfillmentAccessError::SessionInactive.into()),
        }
        if !user.is_admin && authenticated != tenant {
            return Err(FulfillmentAccessError::Forbidden.into());
        }
        if user.is_admin {
            return Ok(FulfillmentAuthority { warehouses: None });
        }
        let grants: Vec<Uuid> = sqlx::query_scalar(
            "select warehouse_id from user_warehouse_access where tenant_id=$1 and user_id=$2
             order by warehouse_id limit 10001 for share",
        )
        .bind(tenant.0)
        .bind(user.user_id.0)
        .fetch_all(&mut **tx)
        .await?;
        if grants.len() > 10000 {
            return Err(FulfillmentAccessError::ScopeLimit.into());
        }
        // Preserve the established product policy: no grants means unrestricted.
        Ok(FulfillmentAuthority {
            warehouses: (!grants.is_empty()).then(|| grants.into_iter().collect()),
        })
    }

    pub(super) async fn record(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
        record: FulfillmentRecord,
    ) -> anyhow::Result<()> {
        let FulfillmentRecord {
            action,
            audit_action,
            entity,
            id,
            mut metadata,
        } = record;
        let user_id = self.user.as_ref().map(|user| user.user_id);
        if let Some(user) = &self.user {
            metadata["request_id"] = Value::String(user.request_id.clone());
            metadata["trace_id"] = Value::String(user.trace_id.clone());
            metadata["authentication_tenant_id"] =
                serde_json::to_value(user.authentication_tenant)?;
        } else {
            metadata["actor_source"] = Value::String("internal.fulfillment".into());
        }
        self.operation(tx, tenant, action, entity, id.clone(), metadata.clone())
            .await?;
        AuditLogsRepo::insert_with_tx(
            &mut **tx,
            AuditLogCreate {
                tenant_id: tenant,
                user_id,
                action: audit_action.into(),
                entity_type: entity.into(),
                entity_id: Some(id),
                before_json: None,
                after_json: Some(metadata),
                ip: self.user.as_ref().and_then(|user| user.ip.clone()),
                ua: self.user.as_ref().and_then(|user| user.user_agent.clone()),
            },
        )
        .await?;
        Ok(())
    }

    pub(super) async fn operation(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
        action: &str,
        entity: &str,
        id: String,
        metadata: Value,
    ) -> anyhow::Result<()> {
        OperationLogsRepo::insert_with_tx(
            &mut **tx,
            OperationLogCreate {
                tenant_id: tenant,
                actor_type: if self.user.is_some() {
                    "USER"
                } else {
                    "SYSTEM"
                }
                .into(),
                actor_user_id: self.user.as_ref().map(|user| user.user_id),
                action: action.into(),
                ref_type: Some(entity.into()),
                ref_id: Some(id),
                metadata: Some(metadata),
            },
        )
        .await?;
        Ok(())
    }
}
