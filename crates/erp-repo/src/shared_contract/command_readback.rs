//! Client observations and lost-response recovery read committed facts; they never execute.
use super::{SharedContractRepo, SharedWrite};
use erp_domain::receipt::{CommandReceipt, ReceiptError, admit_retry, command_status};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub type CommandReadResult = (String, String, String, i64, String, bool, String);

#[derive(Debug, thiserror::Error)]
pub enum CommandReadbackError {
    #[error("command does not exist; an unknown client response is not a server failure")]
    NotFound,
    #[error("a client observation cannot change the stored command status")]
    ClientTransitionDenied,
    #[error("command status is invalid")]
    InvalidStatus,
    #[error("command readback requires reconciliation: {0}")]
    ReconciliationRequired(&'static str),
    #[error("command lookup requires owned revoked and currently valid sessions")]
    SessionForbidden,
}

#[derive(sqlx::FromRow)]
struct ReceiptRow {
    request_id: String,
    payload_hash: String,
    status: String,
    effect_id: String,
}

async fn begin_read(pool: &PgPool) -> anyhow::Result<Transaction<'_, Postgres>> {
    let mut tx = pool.begin().await?;
    // A consistent receipt/session/metadata snapshot without FOR UPDATE or advisory write locks.
    sqlx::query("set transaction isolation level repeatable read, read only")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

async fn stored_receipt(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    request: &str,
) -> anyhow::Result<Option<CommandReceipt>> {
    let row: Option<ReceiptRow> = sqlx::query_as(
        "select request_id,payload_hash,status,effect_id from command_receipts where tenant_id=$1 and request_id=$2",
    )
    .bind(tenant.0)
    .bind(request.trim())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let status = command_status(&row.status).map_err(|_| {
        CommandReadbackError::ReconciliationRequired("unrecognized stored command status")
    })?;
    if status.is_complete() {
        let has_generic_metadata: bool = sqlx::query_scalar(
            "select exists(select 1 from command_objects where tenant_id=$1 and request_id=$2)",
        )
        .bind(tenant.0)
        .bind(request.trim())
        .fetch_one(&mut **tx)
        .await?;
        // This historical prototype allowed client-made terminal states and stock posted by lookup.
        // Do not silently bless, rewrite or repair those rows while answering a read.
        if has_generic_metadata || (row.status == "SUCCEEDED" && row.effect_id.trim().is_empty()) {
            return Err(CommandReadbackError::ReconciliationRequired(
                "legacy generic completion has no authoritative business result",
            )
            .into());
        }
    }
    Ok(Some(CommandReceipt {
        request_id: row.request_id,
        payload_hash: row.payload_hash,
        status,
        effect_id: row.effect_id,
    }))
}

pub(super) struct LostCommandRead<'a> {
    pub tenant: TenantId,
    pub request: &'a str,
    pub payload_hash: &'a str,
    pub actor: &'a str,
    pub revoked_session: Uuid,
    pub current_session: Uuid,
}

impl SharedContractRepo<'_> {
    pub(super) async fn observe_command_status(
        &self,
        tenant: TenantId,
        request: &str,
        payload_hash: &str,
        reported_status: &str,
    ) -> anyhow::Result<Result<SharedWrite, ReceiptError>> {
        if request.trim().is_empty() {
            return Ok(Err(ReceiptError::EmptyRequest));
        }
        let reported = if reported_status == "UNKNOWN" {
            None
        } else {
            Some(command_status(reported_status).map_err(|_| CommandReadbackError::InvalidStatus)?)
        };
        let mut tx = begin_read(self.pool).await?;
        let stored = stored_receipt(&mut tx, tenant, request)
            .await?
            .ok_or(CommandReadbackError::NotFound)?;
        if let Err(error) = admit_retry(Some(&stored), request, payload_hash) {
            return Ok(Err(error));
        }
        if reported.is_some_and(|reported| reported != stored.status) {
            return Err(CommandReadbackError::ClientTransitionDenied.into());
        }
        tx.rollback().await?;
        Ok(Ok(SharedWrite::Replay(stored)))
    }

    pub(super) async fn lookup_lost_command_readonly(
        &self,
        input: LostCommandRead<'_>,
    ) -> anyhow::Result<Result<SharedWrite, ReceiptError>> {
        if input.request.trim().is_empty() {
            return Ok(Err(ReceiptError::EmptyRequest));
        }
        if input.actor.trim().is_empty() || input.revoked_session == input.current_session {
            return Err(CommandReadbackError::SessionForbidden.into());
        }
        let mut tx = begin_read(self.pool).await?;
        let stored = stored_receipt(&mut tx, input.tenant, input.request)
            .await?
            .ok_or(CommandReadbackError::NotFound)?;
        if let Err(error) = admit_retry(Some(&stored), input.request, input.payload_hash) {
            return Ok(Err(error));
        }
        let sessions_valid: bool = sqlx::query_scalar(
            "select exists(select 1 from browser_sessions old
             join browser_sessions current on current.tenant_id=old.tenant_id and current.actor_user_id=old.actor_user_id
             join users u on u.tenant_id=current.tenant_id and u.user_id::text=current.actor_user_id and u.status='ACTIVE'
             left join credential_epochs e on e.tenant_id=current.tenant_id and e.actor_user_id=current.actor_user_id
             where old.tenant_id=$1 and old.session_id=$2 and current.session_id=$3
             and old.actor_user_id=$4 and old.status='REVOKED' and current.status='ACTIVE'
             and current.expires_at>now() and current.credential_epoch=coalesce(e.epoch,0))",
        )
        .bind(input.tenant.0)
        .bind(input.revoked_session)
        .bind(input.current_session)
        .bind(input.actor.trim())
        .fetch_one(&mut *tx)
        .await?;
        if !sessions_valid {
            return Err(CommandReadbackError::SessionForbidden.into());
        }
        tx.rollback().await?;
        Ok(Ok(SharedWrite::Replay(stored)))
    }

    pub(super) async fn read_command_result(
        &self,
        tenant: TenantId,
        request: &str,
    ) -> anyhow::Result<Option<CommandReadResult>> {
        let mut tx = begin_read(self.pool).await?;
        let Some(receipt) = stored_receipt(&mut tx, tenant, request).await? else {
            tx.rollback().await?;
            return Ok(None);
        };
        let metadata: Option<(String, i64, bool, String)> = sqlx::query_as(
            "select object_id,version_no,recoverable,error_text from command_objects where tenant_id=$1 and request_id=$2 and line_no=1",
        )
        .bind(tenant.0)
        .bind(request.trim())
        .fetch_optional(&mut *tx)
        .await?;
        let (object, version, recoverable, error) = if let Some(metadata) = metadata {
            metadata
        } else {
            let object = if let Ok(effect) = Uuid::parse_str(&receipt.effect_id) {
                sqlx::query_scalar::<_, String>(
                    "select work_order_id::text from mfg_material_moves where tenant_id=$1 and move_id=$2",
                )
                .bind(tenant.0)
                .bind(effect)
                .fetch_optional(&mut *tx)
                .await?
                .unwrap_or_default()
            } else {
                String::new()
            };
            // Zero explicitly means that this compatibility response has no source-version metadata.
            (object, 0, !receipt.status.is_complete(), String::new())
        };
        tx.rollback().await?;
        Ok(Some((
            object,
            receipt.effect_id,
            receipt.status.as_str().into(),
            version,
            receipt.status.as_str().into(),
            recoverable,
            error,
        )))
    }
}
