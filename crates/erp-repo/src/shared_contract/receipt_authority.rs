//! Current receipt authority is locked in the business transaction and retained as evidence.
use erp_types::{TenantId, UserId};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct ReceiptValueActor {
    pub authentication_tenant: TenantId,
    pub user_id: UserId,
    pub session_id: Uuid,
    pub credential_epoch: i32,
    pub warehouse_id: Uuid,
}
#[derive(Debug, thiserror::Error)]
pub enum ReceiptValueAuthorizationError {
    #[error("a current business session is required")]
    Unauthorized,
    #[error("current business write authority is required")]
    Forbidden,
    #[error("receipt is unavailable in the requested warehouse scope")]
    Unavailable,
}
impl ReceiptValueActor {
    pub(super) fn validate(self, tenant: TenantId) -> Result<(), ReceiptValueAuthorizationError> {
        if self.authentication_tenant != tenant {
            return Err(ReceiptValueAuthorizationError::Forbidden);
        }
        if self.user_id.0.is_nil() || self.session_id.is_nil() || self.credential_epoch < 0 {
            return Err(ReceiptValueAuthorizationError::Unauthorized);
        }
        if self.warehouse_id.is_nil() {
            return Err(ReceiptValueAuthorizationError::Unavailable);
        }
        Ok(())
    }
}
pub(super) struct Permit {
    actor: ReceiptValueActor,
    role_id: Uuid,
    permission: String,
}

pub(super) async fn lock(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: ReceiptValueActor,
) -> anyhow::Result<Permit> {
    lock_permissions(
        tx,
        tenant,
        actor,
        &["purchase_orders.write", "PROCUREMENT.ORDER.WRITE"],
    )
    .await
}

pub(super) async fn lock_permissions(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: ReceiptValueActor,
    permissions: &[&str],
) -> anyhow::Result<Permit> {
    lock_current(tx, tenant, actor, permissions, true).await
}

pub(super) async fn lock_identity(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: ReceiptValueActor,
    permissions: &[&str],
) -> anyhow::Result<Permit> {
    lock_current(tx, tenant, actor, permissions, false).await
}

async fn lock_current(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: ReceiptValueActor,
    permissions: &[&str],
    warehouse_required: bool,
) -> anyhow::Result<Permit> {
    if warehouse_required {
        actor.validate(tenant)?;
    } else {
        if actor.authentication_tenant != tenant {
            return Err(ReceiptValueAuthorizationError::Forbidden.into());
        }
        if actor.user_id.0.is_nil() || actor.session_id.is_nil() || actor.credential_epoch < 0 {
            return Err(ReceiptValueAuthorizationError::Unauthorized.into());
        }
    }
    // Password/status writers lock user before tenant. Keep that order; a login
    // takes tenant but only reads user, so this order introduces no reverse wait.
    let active: Option<String> =
        sqlx::query_scalar("select status from users where tenant_id=$1 and user_id=$2 for share")
            .bind(tenant.0)
            .bind(actor.user_id.0)
            .fetch_optional(&mut **tx)
            .await?;
    if active.as_deref() != Some("ACTIVE") {
        return Err(ReceiptValueAuthorizationError::Unauthorized.into());
    }
    let epoch: Option<i32> = sqlx::query_scalar(
        "select coalesce(credentials_epoch,0) from tenants where tenant_id=$1 for share",
    )
    .bind(tenant.0)
    .fetch_optional(&mut **tx)
    .await?;
    if epoch != Some(actor.credential_epoch) {
        return Err(ReceiptValueAuthorizationError::Unauthorized.into());
    }
    let session: Option<Uuid> = sqlx::query_scalar(
        "select session_id from auth_sessions where tenant_id=$1 and user_id=$2 and session_id=$3
        and status='ACTIVE' and credentials_epoch=$4 and expires_at>clock_timestamp() for share",
    )
    .bind(tenant.0)
    .bind(actor.user_id.0)
    .bind(actor.session_id)
    .bind(actor.credential_epoch)
    .fetch_optional(&mut **tx)
    .await?;
    if session.is_none() {
        return Err(ReceiptValueAuthorizationError::Unauthorized.into());
    }
    let grant: Option<(Uuid, String)> = sqlx::query_as(
        "select ur.role_id,rp.perm_code from user_roles ur
        join role_permissions rp on rp.tenant_id=ur.tenant_id and rp.role_id=ur.role_id
        where ur.tenant_id=$1 and ur.user_id=$2 and rp.perm_code=any($3::text[])
        order by ur.role_id,rp.perm_code limit 1 for share of ur,rp",
    )
    .bind(tenant.0)
    .bind(actor.user_id.0)
    .bind(permissions)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((role_id, permission)) = grant else {
        return Err(ReceiptValueAuthorizationError::Forbidden.into());
    };
    if warehouse_required {
        let warehouse: Option<Uuid> = sqlx::query_scalar(
            "select a.warehouse_id from user_warehouse_access a
        join warehouses w on w.tenant_id=a.tenant_id and w.warehouse_id=a.warehouse_id
        where a.tenant_id=$1 and a.user_id=$2 and a.warehouse_id=$3 for share of a,w",
        )
        .bind(tenant.0)
        .bind(actor.user_id.0)
        .bind(actor.warehouse_id)
        .fetch_optional(&mut **tx)
        .await?;
        if warehouse.is_none() {
            return Err(ReceiptValueAuthorizationError::Unavailable.into());
        }
    }
    Ok(Permit {
        actor,
        role_id,
        permission,
    })
}
impl Permit {
    pub(super) fn snapshot(&self) -> serde_json::Value {
        serde_json::json!({"user_id":self.actor.user_id,"session_id":self.actor.session_id,
            "credential_epoch":self.actor.credential_epoch,"warehouse_id":self.actor.warehouse_id,
            "role_id":self.role_id,"permission":self.permission})
    }
    pub(super) async fn recheck(&self, tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<()> {
        let a = self.actor;
        // Locks protect status, epoch and grants. Time itself is not row-locked.
        let live: bool = sqlx::query_scalar(
            "select exists(select 1 from users u
              join tenants t on t.tenant_id=u.tenant_id
              join auth_sessions s on s.tenant_id=u.tenant_id and s.user_id=u.user_id
              where u.tenant_id=$1 and u.user_id=$2 and u.status='ACTIVE'
                and s.session_id=$3 and s.status='ACTIVE' and s.expires_at>clock_timestamp()
                and s.credentials_epoch=$4 and t.credentials_epoch=$4)",
        )
        .bind(a.authentication_tenant.0)
        .bind(a.user_id.0)
        .bind(a.session_id)
        .bind(a.credential_epoch)
        .fetch_one(&mut **tx)
        .await?;
        if !live {
            return Err(ReceiptValueAuthorizationError::Unauthorized.into());
        }
        Ok(())
    }
    pub(super) async fn retain(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: &str,
    ) -> anyhow::Result<()> {
        self.recheck(tx).await?;
        let a = self.actor;
        sqlx::query("insert into shared_receipt_value_actors(tenant_id,request_id,authentication_tenant,user_id,
            session_id,warehouse_id,credential_epoch,role_id,permission_code) values($1,$2,$1,$3,$4,$5,$6,$7,$8)")
            .bind(a.authentication_tenant.0).bind(request).bind(a.user_id.0).bind(a.session_id)
            .bind(a.warehouse_id).bind(a.credential_epoch).bind(self.role_id).bind(&self.permission)
            .execute(&mut **tx).await?;
        Ok(())
    }
}
