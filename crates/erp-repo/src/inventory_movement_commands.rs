//! Original identity, exact stock, required logs and immutable result share one commit.
use anyhow::Result;
use erp_domain::quantity::{QTY_SCALE, parse_qty};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    inventory::{InventoryAdjustment, InventoryBalance, InventoryRepo, InventoryService},
    inventory_location_commands::{
        apply_location_inbound, apply_location_outbound, verify_location_effect,
    },
    inventory_location_reservations::{
        InventoryReservationEffect, InventoryReservationSource, apply_location_reservation,
        verify_reservation_effect,
    },
    operation_logs::{OperationLogCreate, OperationLogsRepo},
};

const LEGACY_RESULT_VERSION: u8 = 1;
const RESULT_VERSION: u8 = 2;
const REF_TYPE: &str = "inventory_movement_command";

pub use crate::inventory_location_commands::{InventoryInboundLocation, InventoryLocationEffect};

#[derive(Debug, thiserror::Error)]
pub enum InventoryCommandError {
    #[error("{0}")]
    InvalidPayload(&'static str),
    #[error("original inventory command identity conflicts")]
    IdentityConflict,
    #[error("original inventory command requires reconciliation")]
    ReconciliationRequired,
    #[error(
        "this location has only {available_micros} free micro-units, requested {requested_micros}"
    )]
    InsufficientLocationStock {
        available_micros: i128,
        requested_micros: i128,
    },
    #[error("only GOOD location stock can be issued")]
    QualityNotIssuable,
    #[error("reservation source is missing, reused or bound to another physical key")]
    ReservationSourceConflict,
    #[error("source has {remaining_micros} remaining micro-units, requested {requested_micros}")]
    ReservationRemainingInsufficient {
        remaining_micros: i64,
        requested_micros: i64,
    },
    #[error("current role does not hold inventory write")]
    RolePermissionDenied,
    #[error("session is not active for this inventory command")]
    SessionInactive,
    #[error("session credentials are stale for this inventory command")]
    CredentialsStale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InventoryMovementKind {
    Inbound,
    Outbound,
    Adjustment,
    Reserve,
    Release,
}

impl InventoryMovementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inbound => "INBOUND",
            Self::Outbound => "OUTBOUND",
            Self::Adjustment => "ADJUSTMENT",
            Self::Reserve => "RESERVE",
            Self::Release => "RELEASE",
        }
    }

    pub(crate) fn reason_type(self) -> &'static str {
        match self {
            Self::Inbound => "PURCHASE_RECEIPT",
            Self::Outbound => "TRANSFER_OUT",
            Self::Adjustment => "STOCK_COUNT_ADJUST",
            Self::Reserve => "RESERVATION_CREATE",
            Self::Release => "RESERVATION_RELEASE",
        }
    }

    fn action(self) -> &'static str {
        match self {
            Self::Inbound => "INVENTORY_INBOUND",
            Self::Outbound => "INVENTORY_OUTBOUND",
            Self::Adjustment => "INVENTORY_ADJUSTED",
            Self::Reserve => "INVENTORY_RESERVED",
            Self::Release => "INVENTORY_RELEASED",
        }
    }
}

/// `quantity_delta` is a whole base unit. The ledger still uses six-digit micros.
/// Optional location metadata shares the original inbound/outbound identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryMovementCommandInput {
    pub tenant_id: TenantId,
    pub actor_user_id: UserId,
    /// Opened login session. Required so the stock transaction can reread it.
    /// Absent only means the caller did not bind a session; the command then stores nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<Uuid>,
    pub request_id: String,
    pub kind: InventoryMovementKind,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub quantity_delta: i64,
    pub reason: Option<String>,
    pub reference_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<InventoryInboundLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reservation_source: Option<InventoryReservationSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<Uuid>,
}

impl InventoryMovementCommandInput {
    fn normalize(self) -> Result<Self> {
        if self.session_id.is_none_or(|id| id.is_nil()) {
            return Err(InventoryCommandError::InvalidPayload("nil command identity").into());
        }
        self.normalize_business()
    }

    /// Historical v1 effects may predate session binding; new commands never do.
    fn normalize_business(mut self) -> Result<Self> {
        self.request_id = normalize_inventory_request_id(&self.request_id)?;
        if self.batch_id.is_some_and(|id| id.is_nil())
            || (self.batch_id.is_some()
                && (self.location.is_some() || self.reservation_source.is_some()))
        {
            return Err(InventoryCommandError::InvalidPayload(
                "batch UUID requires a valid identity and cannot be combined with location batch code",
            ).into());
        }
        if self.tenant_id.0.is_nil()
            || self.actor_user_id.0.is_nil()
            || self.session_id.is_some_and(|id| id.is_nil())
            || self.warehouse_id.is_nil()
            || self.sku_id.is_nil()
        {
            return Err(InventoryCommandError::InvalidPayload("nil command identity").into());
        }
        if self.quantity_delta == 0
            || (self.kind == InventoryMovementKind::Inbound && self.quantity_delta < 0)
            || (self.kind == InventoryMovementKind::Outbound && self.quantity_delta > 0)
            || (self.kind == InventoryMovementKind::Reserve && self.quantity_delta < 0)
            || (self.kind == InventoryMovementKind::Release && self.quantity_delta > 0)
        {
            return Err(InventoryCommandError::InvalidPayload("invalid movement quantity").into());
        }
        self.quantity_delta
            .checked_mul(QTY_SCALE)
            .ok_or(InventoryCommandError::InvalidPayload(
                "movement quantity exceeds exact range",
            ))?;
        self.reason = normalize_optional(self.reason, 512)?;
        self.reference_id = normalize_optional(self.reference_id, 128)?;
        self.location = self
            .location
            .take()
            .map(InventoryInboundLocation::normalize)
            .transpose()?;
        self.reservation_source = self
            .reservation_source
            .take()
            .map(InventoryReservationSource::normalize)
            .transpose()?;
        let reservation = matches!(
            self.kind,
            InventoryMovementKind::Reserve | InventoryMovementKind::Release
        );
        if reservation != self.reservation_source.is_some()
            || (reservation && self.location.is_none())
        {
            return Err(InventoryCommandError::InvalidPayload(
                "reservation requires source and location dimensions",
            )
            .into());
        }
        if self.location.is_some() && self.kind == InventoryMovementKind::Adjustment {
            return Err(InventoryCommandError::InvalidPayload(
                "location dimensions require inbound or outbound",
            )
            .into());
        }
        if (self.kind == InventoryMovementKind::Outbound || reservation)
            && self
                .location
                .as_ref()
                .is_some_and(|location| location.quality_status != "GOOD")
        {
            return Err(InventoryCommandError::QualityNotIssuable.into());
        }
        Ok(self)
    }
}

pub fn normalize_inventory_request_id(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|c| c.is_ascii_graphic()) {
        return Err(InventoryCommandError::InvalidPayload(
            "request_id must contain 1 to 128 visible ASCII bytes without spaces",
        )
        .into());
    }
    Ok(value.to_owned())
}

fn normalize_optional(value: Option<String>, limit: usize) -> Result<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim();
    if value.len() > limit || value.chars().any(char::is_control) {
        return Err(
            InventoryCommandError::InvalidPayload("invalid movement reference or reason").into(),
        );
    }
    Ok((!value.is_empty()).then(|| value.to_owned()))
}

fn payload_hash(input: &InventoryMovementCommandInput) -> Result<String> {
    // A session authorizes this attempt, while the user's business intent
    // retains its identity across devices. The original session remains in
    // the snapshot and the mandatory v2 operation/audit metadata.
    let mut business = input.clone();
    business.session_id = None;
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&("inventory-movement-v2", business,))?)
    ))
}

fn snapshot_payload_hash(input: &InventoryMovementCommandInput, version: u8) -> Result<String> {
    match version {
        LEGACY_RESULT_VERSION => Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&("inventory-movement-v1", input,))?)
        )),
        RESULT_VERSION => payload_hash(input),
        _ => Err(InventoryCommandError::ReconciliationRequired.into()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryCommandSnapshot {
    pub version: u8,
    pub input: InventoryMovementCommandInput,
    pub balance: InventoryBalance,
    pub ledger_id: Uuid,
    pub operation_log_id: Uuid,
    pub audit_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_effect: Option<InventoryLocationEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reservation_effect: Option<InventoryReservationEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub batch_effects: Vec<crate::inventory_batch_effects::InventoryBatchEffect>,
}

pub struct InventoryCommandResult {
    pub snapshot: InventoryCommandSnapshot,
    pub payload_hash: String,
    pub replay: bool,
}

#[derive(sqlx::FromRow)]
struct ReceiptRow {
    request_id: String,
    payload_hash: String,
    status: String,
    effect_id: String,
    movement_kind: Option<String>,
    actor_user_id: Option<Uuid>,
    warehouse_id: Option<Uuid>,
    sku_id: Option<Uuid>,
    original_result: Option<Value>,
}

const READ_RECEIPT: &str = "select request_id,payload_hash,status,effect_id,movement_kind,
 actor_user_id,warehouse_id,sku_id,original_result from command_receipts
 where tenant_id=$1 and request_id=$2";

pub struct InventoryMovementCommandsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InventoryMovementCommandsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn execute(
        &self,
        input: InventoryMovementCommandInput,
    ) -> Result<InventoryCommandResult> {
        let input = input.normalize()?;
        let hash = payload_hash(&input)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("set local statement_timeout='10s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local lock_timeout='5s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(input.tenant_id.to_string())
            .bind(format!("inventory-command:{}", input.request_id))
            .execute(&mut *tx)
            .await?;

        // Recount the stored role and reread the opened session before claiming a receipt.
        // A refused role or session must not leave a RUNNING receipt or a stock row.
        // The JWT permission list and the login-layer lookup stay outside this transaction.
        // Admin does not skip this recount. A missing session row is not a bound session.
        let role_grants: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from users u
            join user_roles ur
              on ur.tenant_id = u.tenant_id and ur.user_id = u.user_id
            join role_permissions rp
              on rp.tenant_id = ur.tenant_id and rp.role_id = ur.role_id
            where u.tenant_id = $1
              and u.user_id = $2
              and u.status = 'ACTIVE'
              and rp.perm_code = 'WMS.INVENTORY.WRITE'
            "#,
        )
        .bind(input.tenant_id.0)
        .bind(input.actor_user_id.0)
        .fetch_one(&mut *tx)
        .await?;
        if role_grants == 0 {
            return Err(InventoryCommandError::RolePermissionDenied.into());
        }
        let session: Option<(String, bool, i32, i32)> = sqlx::query_as(
            r#"
            select s.status, s.expires_at > now(), s.credentials_epoch, t.credentials_epoch
            from auth_sessions s
            join tenants t on t.tenant_id = s.tenant_id
            where s.tenant_id = $1 and s.user_id = $2 and s.session_id = $3
            for update of s
            "#,
        )
        .bind(input.tenant_id.0)
        .bind(input.actor_user_id.0)
        .bind(
            input
                .session_id
                .ok_or(InventoryCommandError::SessionInactive)?,
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some((status, not_expired, token_epoch, current_epoch)) = session else {
            return Err(InventoryCommandError::SessionInactive.into());
        };
        match erp_domain::access::session_command_allowed(
            status == "ACTIVE",
            not_expired,
            token_epoch,
            current_epoch,
        ) {
            Ok(()) => {}
            Err(erp_domain::access::AccessError::CredentialsStale) => {
                return Err(InventoryCommandError::CredentialsStale.into());
            }
            Err(_) => return Err(InventoryCommandError::SessionInactive.into()),
        }

        // Claim the shared namespace before acquiring stock locks. Losing a
        // collision never permits another inventory effect, including old writers.
        let claimed: Option<String> = sqlx::query_scalar(
            "insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id,
             movement_kind,actor_user_id,warehouse_id,sku_id)
             values($1,$2,$3,'RUNNING','',$4,$5,$6,$7)
             on conflict(tenant_id,request_id) do nothing returning request_id",
        )
        .bind(input.tenant_id.0)
        .bind(&input.request_id)
        .bind(&hash)
        .bind(input.kind.as_str())
        .bind(input.actor_user_id.0)
        .bind(input.warehouse_id)
        .bind(input.sku_id)
        .fetch_optional(&mut *tx)
        .await?;
        if claimed.is_none() {
            let row: ReceiptRow = sqlx::query_as(READ_RECEIPT)
                .bind(input.tenant_id.0)
                .bind(&input.request_id)
                .fetch_one(&mut *tx)
                .await?;
            if row.actor_user_id != Some(input.actor_user_id.0)
                || row.movement_kind.as_deref() != Some(input.kind.as_str())
                || row.warehouse_id != Some(input.warehouse_id)
                || row.sku_id != Some(input.sku_id)
            {
                return Err(InventoryCommandError::IdentityConflict.into());
            }
            let result = verified_snapshot(&mut tx, input.tenant_id, row).await?;
            // Verify the immutable old hash against its original version first.
            // Comparison uses current business semantics without rewriting v1.
            if payload_hash(&result.snapshot.input)? != hash {
                return Err(InventoryCommandError::IdentityConflict.into());
            }
            tx.commit().await?;
            return Ok(result);
        }

        if input.location.is_some() {
            let batch_managed: Option<bool> = sqlx::query_scalar(
                "select batch_enabled from skus where tenant_id=$1 and sku_id=$2 for share",
            )
            .bind(input.tenant_id.0)
            .bind(input.sku_id)
            .fetch_optional(&mut *tx)
            .await?;
            if batch_managed == Some(true) {
                return Err(InventoryCommandError::InvalidPayload(
                    "batch-managed SKU requires a proven mapping between batch UUID and location batch code",
                ).into());
            }
        }
        let (balance, location_effect, reservation_effect, batch_effects, actual_ledger_id) =
            if let (Some(location), Some(source)) = (&input.location, &input.reservation_source) {
                let (balance, effect) = apply_location_reservation(
                    &InventoryService::new(self.pool),
                    &mut tx,
                    &input,
                    location,
                    source,
                )
                .await?;
                (balance, None, Some(effect), Vec::new(), None)
            } else if let Some(location) = &input.location {
                let service = InventoryService::new(self.pool);
                let (balance, effect) = if input.kind == InventoryMovementKind::Inbound {
                    apply_location_inbound(&service, &mut tx, &input, location).await?
                } else {
                    apply_location_outbound(&service, &mut tx, &input, location).await?
                };
                (balance, Some(effect), None, Vec::new(), None)
            } else {
                let effect = InventoryRepo::new(self.pool)
                    .apply_adjustment_effect_with_tx(
                        &mut tx,
                        InventoryAdjustment {
                            tenant_id: input.tenant_id,
                            warehouse_id: input.warehouse_id,
                            sku_id: input.sku_id,
                            batch_id: input.batch_id,
                            qty_delta: input.quantity_delta,
                            ref_type: Some(REF_TYPE.into()),
                            ref_id: Some(input.request_id.clone()),
                        },
                        input.kind.reason_type(),
                    )
                    .await?;
                (
                    effect.stock.balance.whole_projection()?,
                    None,
                    None,
                    effect.batch_effects,
                    Some(effect.stock.ledger_id),
                )
            };
        let (ref_type, ref_id) = match (&location_effect, &reservation_effect) {
            (Some(effect), None) => ("shared_stock_ledger", effect.ledger_id.to_string()),
            (None, Some(effect)) => ("shared_stock_ledger", effect.ledger_id.to_string()),
            (None, None) => (REF_TYPE, input.request_id.clone()),
            _ => return Err(InventoryCommandError::ReconciliationRequired.into()),
        };
        let ledger_id: Uuid = if let Some(id) = actual_ledger_id {
            id
        } else {
            sqlx::query_scalar(
                "select ledger_id from inventory_ledgers where tenant_id=$1 and warehouse_id=$2
             and sku_id=$3 and ref_type=$4 and ref_id=$5 and reason_type=$6",
            )
            .bind(input.tenant_id.0)
            .bind(input.warehouse_id)
            .bind(input.sku_id)
            .bind(ref_type)
            .bind(&ref_id)
            .bind(input.kind.reason_type())
            .fetch_one(&mut *tx)
            .await?
        };
        let metadata = movement_metadata(&input, &hash, &balance, RESULT_VERSION);
        let operation = OperationLogsRepo::insert_with_tx(
            &mut *tx,
            OperationLogCreate {
                tenant_id: input.tenant_id,
                actor_type: "USER".into(),
                actor_user_id: Some(input.actor_user_id),
                action: input.kind.action().into(),
                ref_type: Some("INVENTORY".into()),
                ref_id: Some(input.sku_id.to_string()),
                metadata: Some(metadata.clone()),
            },
        )
        .await?;
        let audit = AuditLogsRepo::insert_with_tx(
            &mut *tx,
            AuditLogCreate {
                tenant_id: input.tenant_id,
                user_id: Some(input.actor_user_id),
                action: format!("INVENTORY.{}", input.kind.reason_type()),
                entity_type: "INVENTORY_BALANCE".into(),
                entity_id: Some(input.sku_id.to_string()),
                before_json: None,
                after_json: Some(metadata),
                ip: None,
                ua: None,
            },
        )
        .await?;
        let snapshot = InventoryCommandSnapshot {
            version: RESULT_VERSION,
            input,
            balance,
            ledger_id,
            operation_log_id: operation.log_id,
            audit_id: audit.audit_id,
            location_effect,
            reservation_effect,
            batch_effects,
        };
        let updated = sqlx::query(
            "update command_receipts set status='SUCCEEDED',effect_id=$3,original_result=$4
             where tenant_id=$1 and request_id=$2 and status='RUNNING' and payload_hash=$5",
        )
        .bind(snapshot.input.tenant_id.0)
        .bind(&snapshot.input.request_id)
        .bind(ledger_id.to_string())
        .bind(serde_json::to_value(&snapshot)?)
        .bind(&hash)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(InventoryCommandError::ReconciliationRequired.into());
        }
        tx.commit().await?;
        Ok(InventoryCommandResult {
            snapshot,
            payload_hash: hash,
            replay: false,
        })
    }

    /// Fresh readback resolves a lost HTTP response without consulting today's stock.
    /// Missing/invisible commands are indistinguishable to another user.
    pub async fn find(
        &self,
        tenant: TenantId,
        actor: UserId,
        request_id: &str,
    ) -> Result<Option<InventoryCommandResult>> {
        let request_id = normalize_inventory_request_id(request_id)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read read only")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local statement_timeout='10s'")
            .execute(&mut *tx)
            .await?;
        let row: Option<ReceiptRow> = sqlx::query_as(
            "select request_id,payload_hash,status,effect_id,movement_kind,actor_user_id,
             warehouse_id,sku_id,original_result from command_receipts
             where tenant_id=$1 and request_id=$2 and actor_user_id=$3",
        )
        .bind(tenant.0)
        .bind(request_id)
        .bind(actor.0)
        .fetch_optional(&mut *tx)
        .await?;
        let result = match row {
            Some(row) => Some(verified_snapshot(&mut tx, tenant, row).await?),
            None => None,
        };
        tx.commit().await?;
        Ok(result)
    }
}

fn movement_metadata(
    input: &InventoryMovementCommandInput,
    hash: &str,
    balance: &InventoryBalance,
    version: u8,
) -> Value {
    let mut metadata = json!({"request_id":input.request_id,"payload_hash":hash,"movement_kind":input.kind,
        "warehouse_id":input.warehouse_id,"sku_id":input.sku_id,"batch_id":input.batch_id,
        "qty_delta":input.quantity_delta,"reason":input.reason,"reference_id":input.reference_id,
        "reason_type":input.kind.reason_type(),"quantities":balance.quantities});
    if version == RESULT_VERSION {
        // The fingerprint intentionally excludes this field. Required logs
        // separately anchor the actual original session so snapshot tampering
        // cannot substitute the session used for a later replay.
        metadata["submitter_session_id"] = json!(input.session_id);
    }
    if let Some(location) = &input.location {
        metadata["location"] = json!(location);
    }
    if let Some(source) = &input.reservation_source {
        metadata["reservation_source"] = json!(source);
    }
    metadata
}

async fn verified_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    row: ReceiptRow,
) -> Result<InventoryCommandResult> {
    let reconcile = || InventoryCommandError::ReconciliationRequired;
    if row.status != "SUCCEEDED" {
        return Err(reconcile().into());
    }
    let snapshot: InventoryCommandSnapshot =
        serde_json::from_value(row.original_result.ok_or_else(reconcile)?)
            .map_err(|_| reconcile())?;
    let input = match snapshot.version {
        LEGACY_RESULT_VERSION => snapshot.input.clone().normalize_business(),
        RESULT_VERSION => snapshot.input.clone().normalize(),
        _ => return Err(reconcile().into()),
    }
    .map_err(|_| reconcile())?;
    if serde_json::to_value(&input)? != serde_json::to_value(&snapshot.input)?
        || input.tenant_id != tenant
        || input.request_id != row.request_id
        || input.actor_user_id.0 != row.actor_user_id.ok_or_else(reconcile)?
        || Some(input.kind.as_str()) != row.movement_kind.as_deref()
        || Some(input.warehouse_id) != row.warehouse_id
        || Some(input.sku_id) != row.sku_id
        || snapshot_payload_hash(&input, snapshot.version)? != row.payload_hash
        || snapshot.ledger_id.to_string() != row.effect_id
        || snapshot.balance.tenant_id != tenant
        || snapshot.balance.warehouse_id != input.warehouse_id
        || snapshot.balance.sku_id != input.sku_id
        || !valid_balance(&snapshot.balance)
    {
        return Err(reconcile().into());
    }
    // A syntactically valid receipt is insufficient: all original effect IDs and
    // exact quantities must still bind to the same required stock/log facts.
    let micros = input
        .quantity_delta
        .checked_mul(QTY_SCALE)
        .ok_or_else(reconcile)?;
    let (ref_type, ref_id, hand_delta, reserved_delta, frozen_delta) = match (
        &input.location,
        &snapshot.location_effect,
        &input.reservation_source,
        &snapshot.reservation_effect,
    ) {
        (Some(location), Some(effect), None, None) => {
            if !verify_location_effect(tx, &input, location, effect).await? {
                return Err(reconcile().into());
            }
            (
                "shared_stock_ledger",
                effect.ledger_id.to_string(),
                micros,
                0,
                location.frozen_delta(input.quantity_delta)?,
            )
        }
        (Some(location), None, Some(source), Some(effect)) => {
            if !verify_reservation_effect(tx, &input, location, source, effect).await? {
                return Err(reconcile().into());
            }
            (
                "shared_stock_ledger",
                effect.ledger_id.to_string(),
                0,
                micros,
                0,
            )
        }
        (None, None, None, None) => (REF_TYPE, input.request_id.clone(), micros, 0, 0),
        _ => return Err(reconcile().into()),
    };
    let matches: bool = sqlx::query_scalar(
        "select exists(select 1 from inventory_ledgers l
         join operation_logs o on o.tenant_id=l.tenant_id and o.log_id=$3
         join audit_logs a on a.tenant_id=l.tenant_id and a.audit_id=$4
         where l.tenant_id=$1 and l.ledger_id=$2 and l.warehouse_id=$5 and l.sku_id=$6
         and l.ref_type=$7 and l.ref_id=$8 and l.reason_type=$9
         and l.delta_on_hand_micros=$10 and l.delta_reserved_micros=$17
         and l.delta_frozen_micros=$16 and l.delta_in_transit_micros=0
         and o.actor_type='USER' and o.actor_user_id=$11 and o.action=$12
         and o.ref_type='INVENTORY' and o.ref_id=$13 and o.metadata=$14
         and a.user_id=$11 and a.action=$15 and a.entity_type='INVENTORY_BALANCE'
         and a.entity_id=$13 and a.after_json=$14)",
    )
    .bind(tenant.0)
    .bind(snapshot.ledger_id)
    .bind(snapshot.operation_log_id)
    .bind(snapshot.audit_id)
    .bind(input.warehouse_id)
    .bind(input.sku_id)
    .bind(ref_type)
    .bind(&ref_id)
    .bind(input.kind.reason_type())
    .bind(hand_delta)
    .bind(input.actor_user_id.0)
    .bind(input.kind.action())
    .bind(input.sku_id.to_string())
    .bind(movement_metadata(
        &input,
        &row.payload_hash,
        &snapshot.balance,
        snapshot.version,
    ))
    .bind(format!("INVENTORY.{}", input.kind.reason_type()))
    .bind(frozen_delta)
    .bind(reserved_delta)
    .fetch_one(&mut **tx)
    .await?;
    if !matches {
        return Err(reconcile().into());
    }
    if !crate::inventory_batch_effects::verify(
        tx,
        &input,
        snapshot.ledger_id,
        &snapshot.batch_effects,
    )
    .await?
    {
        return Err(reconcile().into());
    }
    Ok(InventoryCommandResult {
        snapshot,
        payload_hash: row.payload_hash,
        replay: true,
    })
}

fn valid_balance(balance: &InventoryBalance) -> bool {
    let q = &balance.quantities;
    let values = [
        &q.on_hand,
        &q.reserved,
        &q.frozen,
        &q.in_transit,
        &q.available,
    ];
    let projected = [
        balance.on_hand,
        balance.reserved,
        balance.frozen,
        balance.in_transit,
        balance.available(),
    ];
    let mut micros = [0_i64; 5];
    for (index, text) in values.into_iter().enumerate() {
        let Ok(value) = parse_qty(text) else {
            return false;
        };
        micros[index] = value.as_micros();
        if micros[index] < 0 || micros[index] / QTY_SCALE != projected[index] {
            return false;
        }
    }
    micros[0]
        .checked_sub(micros[1])
        .and_then(|v| v.checked_sub(micros[2]))
        == Some(micros[4])
}
