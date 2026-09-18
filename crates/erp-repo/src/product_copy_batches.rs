//! Bounded, durable execution for all-matching product copies.
use crate::product_actions;
use erp_grid_core::{
    RowId,
    actions::{ActionCommand, ActionOutcome, ActionReceipt},
};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
use uuid::Uuid;

const MAX_EXCLUSIONS: usize = 10_000;
const MAX_TARGETS: u64 = 100_000;
const MAX_ACTIVE_BATCHES: i64 = 4;
const ADVANCE_BUDGET: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartRequest {
    pub batch_id: Uuid,
    pub token: Uuid,
    pub excluded: Vec<Uuid>,
    pub confirmed_count: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Status {
    pub batch_id: Uuid,
    pub total: u64,
    pub applied: u64,
    pub rejected: u64,
    pub cancelled: u64,
    pub pending: u64,
    pub unknown: u64,
    pub cancel_requested: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum BatchError {
    #[error("invalid product copy batch: {0}")]
    Invalid(String),
    #[error("product copy batch or frozen selection is unavailable")]
    Unavailable,
    #[error("batch identity was already used for a different request")]
    IdentityConflict,
    #[error("product copy batch capacity exceeded")]
    Capacity,
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
    #[error(transparent)]
    Action(#[from] product_actions::ActionError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

pub async fn start(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    request: &StartRequest,
) -> Result<Status, BatchError> {
    start_inner(pool, tenant, actor, request).await
}

async fn start_inner(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    request: &StartRequest,
) -> Result<Status, BatchError> {
    let canonical = canonical_request(request)?;
    let tenant_id = Uuid::from(tenant);
    let actor_id = Uuid::from(actor);
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level serializable")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout='30s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("select pg_advisory_xact_lock(hashtextextended($1::text || ':' || $2::text || ':product-copy-batch',0))")
        .bind(tenant_id.to_string()).bind(actor_id.to_string()).execute(&mut *tx).await?;

    let previous: Option<Value> = sqlx::query_scalar("select request from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 for update")
        .bind(tenant_id).bind(actor_id).bind(request.batch_id).fetch_optional(&mut *tx).await?;
    if let Some(previous) = previous {
        if previous != canonical {
            return Err(BatchError::IdentityConflict);
        }
        let result = status_tx(&mut tx, tenant_id, actor_id, request.batch_id).await?;
        tx.commit().await?;
        return Ok(result);
    }

    let (active, active_targets): (i64, i64) = sqlx::query_as("select count(*),coalesce(sum(total),0)::bigint from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and completed_at is null")
        .bind(tenant_id).bind(actor_id).fetch_one(&mut *tx).await?;
    if active >= MAX_ACTIVE_BATCHES
        || u64::try_from(active_targets)
            .ok()
            .and_then(|n| n.checked_add(request.confirmed_count))
            .is_none_or(|n| n > MAX_TARGETS)
    {
        return Err(BatchError::Capacity);
    }
    let selection_total: Option<i64> = sqlx::query_scalar("select total from grid_product_selections where token=$1 and tenant_id=$2 and actor_user_id=$3 and expires_at>clock_timestamp() for share")
        .bind(request.token).bind(tenant_id).bind(actor_id).fetch_optional(&mut *tx).await?;
    let Some(selection_total) = selection_total else {
        return Err(BatchError::Unavailable);
    };
    let selection_total = u64::try_from(selection_total)
        .map_err(|_| BatchError::Invalid("invalid frozen selection count".into()))?;
    let excluded_count: i64 = if request.excluded.is_empty() {
        0
    } else {
        sqlx::query_scalar("select count(*) from grid_product_selection_items where selection_token=$1 and product_id=any($2)")
            .bind(request.token).bind(&request.excluded).fetch_one(&mut *tx).await?
    };
    if usize::try_from(excluded_count).ok() != Some(request.excluded.len()) {
        return Err(BatchError::Invalid(
            "an excluded product is outside the frozen selection".into(),
        ));
    }
    let affected = selection_total
        .checked_sub(request.excluded.len() as u64)
        .ok_or_else(|| BatchError::Invalid("exclusions exceed selection".into()))?;
    if affected != request.confirmed_count {
        return Err(BatchError::Invalid(
            "confirmed count does not match frozen membership".into(),
        ));
    }
    sqlx::query("insert into grid_product_copy_batches(tenant_id,actor_user_id,batch_id,request,total,pending,completed_at) values($1,$2,$3,$4,$5,$5,case when $5=0 then clock_timestamp() end)")
        .bind(tenant_id).bind(actor_id).bind(request.batch_id).bind(&canonical).bind(affected as i64).execute(&mut *tx).await?;
    let inserted = sqlx::query("with targets as (select i.ordinal,i.product_id,i.product_version,gen_random_uuid() operation_id from grid_product_selection_items i where i.selection_token=$1 and not(i.product_id=any($2)) order by i.ordinal) insert into grid_product_copy_batch_items(tenant_id,actor_user_id,batch_id,ordinal,source_product_id,operation_id,command) select $3,$4,$5,ordinal,product_id,operation_id,jsonb_build_object('operation_id',operation_id,'action','products.copy','source',product_id,'expected_version',to_char(product_version at time zone 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),'values','{}'::jsonb) from targets")
        .bind(request.token).bind(&request.excluded).bind(tenant_id).bind(actor_id).bind(request.batch_id).execute(&mut *tx).await?.rows_affected();
    if inserted != affected {
        return Err(BatchError::Storage(sqlx::Error::Protocol(
            "frozen batch membership changed while starting".into(),
        )));
    }
    let result = status_tx(&mut tx, tenant_id, actor_id, request.batch_id).await?;
    tx.commit().await?;
    Ok(result)
}

fn canonical_request(request: &StartRequest) -> Result<Value, BatchError> {
    if request.batch_id.is_nil()
        || request.token.is_nil()
        || request.confirmed_count == 0
        || request.confirmed_count > MAX_TARGETS
        || request.excluded.len() > MAX_EXCLUSIONS
    {
        return Err(BatchError::Invalid(
            "invalid batch identity or request bounds".into(),
        ));
    }
    let unique: BTreeSet<_> = request.excluded.iter().copied().collect();
    if unique.len() != request.excluded.len() || unique.iter().any(Uuid::is_nil) {
        return Err(BatchError::Invalid(
            "exclusions must be unique non-nil product IDs".into(),
        ));
    }
    let normalized = StartRequest {
        excluded: unique.into_iter().collect(),
        ..request.clone()
    };
    Ok(serde_json::to_value(normalized)?)
}

pub async fn status(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    batch_id: Uuid,
) -> Result<Status, BatchError> {
    if batch_id.is_nil() {
        return Err(BatchError::Invalid("invalid batch identity".into()));
    }
    let mut tx = pool.begin().await?;
    let result = status_tx(&mut tx, Uuid::from(tenant), Uuid::from(actor), batch_id).await?;
    tx.commit().await?;
    Ok(result)
}

async fn status_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant: Uuid,
    actor: Uuid,
    batch: Uuid,
) -> Result<Status, BatchError> {
    let row: Option<(i64,i64,i64,i64,i64,i64,bool)> = sqlx::query_as("select total,applied,rejected,cancelled,pending,unknown,cancel_requested from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and batch_id=$3")
        .bind(tenant).bind(actor).bind(batch).fetch_optional(&mut **tx).await?;
    let Some((total, applied, rejected, cancelled, pending, unknown, cancel_requested)) = row
    else {
        return Err(BatchError::Unavailable);
    };
    Ok(Status {
        batch_id: batch,
        total: count(total)?,
        applied: count(applied)?,
        rejected: count(rejected)?,
        cancelled: count(cancelled)?,
        pending: count(pending)?,
        unknown: count(unknown)?,
        cancel_requested,
    })
}

fn count(value: i64) -> Result<u64, BatchError> {
    u64::try_from(value).map_err(|_| BatchError::Invalid("invalid stored batch count".into()))
}

struct Claim {
    lease: Uuid,
    command: ActionCommand,
    was_unknown: bool,
}

async fn claim(
    pool: &PgPool,
    tenant: Uuid,
    actor: Uuid,
    batch: Uuid,
) -> Result<Option<Claim>, BatchError> {
    let mut tx = pool.begin().await?;
    sqlx::query("set local lock_timeout='3s'")
        .execute(&mut *tx)
        .await?;
    let exists: Option<bool> = sqlx::query_scalar("select cancel_requested from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 for update")
        .bind(tenant).bind(actor).bind(batch).fetch_optional(&mut *tx).await?;
    if exists.is_none() {
        return Err(BatchError::Unavailable);
    }
    let row: Option<(i64,Uuid,Uuid,String,Value)> = sqlx::query_as("select ordinal,source_product_id,operation_id,state,command from grid_product_copy_batch_items where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 and ((state='IN_FLIGHT' and lease_until<=clock_timestamp()) or (state='PENDING' and not (select cancel_requested from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and batch_id=$3))) order by case when state='IN_FLIGHT' then 0 else 1 end,ordinal for update skip locked limit 1")
        .bind(tenant).bind(actor).bind(batch).fetch_optional(&mut *tx).await?;
    let Some((ordinal, source_product_id, operation_id, state, encoded)) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let command: ActionCommand = serde_json::from_value(encoded)?;
    if command.action != product_actions::COPY
        || command.operation_id != RowId(operation_id.as_u128())
        || command.source != RowId(source_product_id.as_u128())
        || command.expected_version.is_none()
        || !command.values.is_empty()
    {
        return Err(BatchError::Invalid(
            "stored batch command does not match its immutable item".into(),
        ));
    }
    product_actions::validate(&command)?;
    let lease = Uuid::new_v4();
    let changed = sqlx::query("update grid_product_copy_batch_items set state='IN_FLIGHT',lease_id=$5,lease_until=clock_timestamp()+interval '60 seconds',updated_at=clock_timestamp() where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 and ordinal=$4")
        .bind(tenant).bind(actor).bind(batch).bind(ordinal).bind(lease).execute(&mut *tx).await?.rows_affected();
    if changed != 1 {
        return Err(BatchError::Unavailable);
    }
    if state == "PENDING" {
        sqlx::query("update grid_product_copy_batches set pending=pending-1,unknown=unknown+1,updated_at=clock_timestamp() where tenant_id=$1 and actor_user_id=$2 and batch_id=$3")
            .bind(tenant).bind(actor).bind(batch).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(Some(Claim {
        lease,
        command,
        was_unknown: state == "IN_FLIGHT",
    }))
}

async fn settle(
    pool: &PgPool,
    tenant: Uuid,
    actor: Uuid,
    batch: Uuid,
    claim: &Claim,
    receipt: &ActionReceipt,
) -> Result<(), BatchError> {
    if receipt.operation_id != claim.command.operation_id
        || receipt.source != claim.command.source
        || receipt.command_fingerprint != claim.command.fingerprint()
    {
        return Err(BatchError::Invalid(
            "product action receipt does not match the claimed command".into(),
        ));
    }
    let (state, counter) = match &receipt.outcome {
        ActionOutcome::Applied { .. } => ("APPLIED", "applied"),
        ActionOutcome::Rejected { .. } => ("REJECTED", "rejected"),
    };
    let mut tx = pool.begin().await?;
    let exists: Option<bool> = sqlx::query_scalar("select cancel_requested from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 for update")
        .bind(tenant).bind(actor).bind(batch).fetch_optional(&mut *tx).await?;
    if exists.is_none() {
        return Err(BatchError::Unavailable);
    }
    let changed = sqlx::query("update grid_product_copy_batch_items set state=$6,receipt=$7,lease_id=null,lease_until=null,updated_at=clock_timestamp() where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 and operation_id=$4 and state='IN_FLIGHT' and lease_id=$5")
        .bind(tenant).bind(actor).bind(batch).bind(Uuid::from_u128(claim.command.operation_id.0)).bind(claim.lease).bind(state).bind(serde_json::to_value(receipt)?).execute(&mut *tx).await?.rows_affected();
    if changed == 1 {
        let sql = format!(
            "update grid_product_copy_batches set unknown=unknown-1,{counter}={counter}+1,updated_at=clock_timestamp(),completed_at=case when applied+rejected+cancelled+1=total then clock_timestamp() else completed_at end where tenant_id=$1 and actor_user_id=$2 and batch_id=$3"
        );
        sqlx::query(&sql)
            .bind(tenant)
            .bind(actor)
            .bind(batch)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn advance(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    batch_id: Uuid,
    max_items: u16,
) -> Result<Status, BatchError> {
    if !(1..=25).contains(&max_items) || batch_id.is_nil() {
        return Err(BatchError::Invalid(
            "advance requires 1-25 items and a valid batch identity".into(),
        ));
    }
    let tenant_id = Uuid::from(tenant);
    let actor_id = Uuid::from(actor);
    let started = Instant::now();
    for index in 0..max_items {
        if index > 0 && started.elapsed() >= ADVANCE_BUDGET {
            break;
        }
        let Some(claimed) = claim(pool, tenant_id, actor_id, batch_id).await? else {
            break;
        };
        let receipt = if claimed.was_unknown {
            match product_actions::receipt(pool, tenant, actor, claimed.command.operation_id)
                .await?
            {
                Some(receipt) => receipt,
                None => product_actions::execute(pool, tenant, actor, &claimed.command).await?,
            }
        } else {
            product_actions::execute(pool, tenant, actor, &claimed.command).await?
        };
        settle(pool, tenant_id, actor_id, batch_id, &claimed, &receipt).await?;
    }
    status(pool, tenant, actor, batch_id).await
}

pub async fn cancel(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    batch_id: Uuid,
) -> Result<Status, BatchError> {
    if batch_id.is_nil() {
        return Err(BatchError::Invalid("invalid batch identity".into()));
    }
    let tenant_id = Uuid::from(tenant);
    let actor_id = Uuid::from(actor);
    let mut tx = pool.begin().await?;
    let locked: Option<(bool, bool)> = sqlx::query_as("select cancel_requested,completed_at is not null from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 for update")
        .bind(tenant_id).bind(actor_id).bind(batch_id).fetch_optional(&mut *tx).await?;
    let Some((_, completed)) = locked else {
        return Err(BatchError::Unavailable);
    };
    if completed {
        let result = status_tx(&mut tx, tenant_id, actor_id, batch_id).await?;
        tx.commit().await?;
        return Ok(result);
    }
    let cancelled = sqlx::query("update grid_product_copy_batch_items set state='CANCELLED',updated_at=clock_timestamp() where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 and state='PENDING'")
        .bind(tenant_id).bind(actor_id).bind(batch_id).execute(&mut *tx).await?.rows_affected();
    sqlx::query("update grid_product_copy_batches set cancel_requested=true,cancelled=cancelled+$4,pending=pending-$4,updated_at=clock_timestamp(),completed_at=case when unknown=0 then clock_timestamp() else completed_at end where tenant_id=$1 and actor_user_id=$2 and batch_id=$3")
        .bind(tenant_id).bind(actor_id).bind(batch_id).bind(cancelled as i64).execute(&mut *tx).await?;
    let result = status_tx(&mut tx, tenant_id, actor_id, batch_id).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn list(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
) -> Result<Vec<Status>, BatchError> {
    let rows: Vec<(Uuid, i64, i64, i64, i64, i64, i64, bool)> = sqlx::query_as(
        "select batch_id,total,applied,rejected,cancelled,pending,unknown,cancel_requested from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 order by (completed_at is null) desc,coalesce(completed_at,updated_at) desc,batch_id limit 20",
    )
    .bind(Uuid::from(tenant))
    .bind(Uuid::from(actor))
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(
            |(
                batch_id,
                total,
                applied,
                rejected,
                cancelled,
                pending,
                unknown,
                cancel_requested,
            )| {
                Ok(Status {
                    batch_id,
                    total: count(total)?,
                    applied: count(applied)?,
                    rejected: count(rejected)?,
                    cancelled: count(cancelled)?,
                    pending: count(pending)?,
                    unknown: count(unknown)?,
                    cancel_requested,
                })
            },
        )
        .collect()
}

#[cfg(test)]
mod tests;
