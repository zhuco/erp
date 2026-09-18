//! Bounded, read-only result pages for durable copy batches.
use crate::product_copy_batches::BatchError;
use erp_grid_core::{
    RowId,
    actions::{ActionCommand, ActionOutcome, ActionReceipt},
};
use erp_types::{TenantId, UserId};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct Item {
    pub ordinal: u64,
    pub source: Uuid,
    pub operation_id: Uuid,
    pub state: String,
    pub receipt: Option<ActionReceipt>,
}
#[derive(Debug, Serialize)]
pub struct Results {
    pub batch_id: Uuid,
    pub items: Vec<Item>,
    pub next_after: Option<u64>,
}
pub async fn page(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    batch: Uuid,
    after: Option<u64>,
    limit: u16,
    failed_only: bool,
) -> Result<Results, BatchError> {
    if batch.is_nil() || !(1..=100).contains(&limit) || after.is_some_and(|after| after >= 100_000)
    {
        return Err(BatchError::Invalid("invalid batch result page".into()));
    }
    let tenant = Uuid::from(tenant);
    let actor = Uuid::from(actor);
    let exists:bool=sqlx::query_scalar("select exists(select 1 from grid_product_copy_batches where tenant_id=$1 and actor_user_id=$2 and batch_id=$3)").bind(tenant).bind(actor).bind(batch).fetch_one(pool).await?;
    if !exists {
        return Err(BatchError::Unavailable);
    }
    let rows:Vec<(i64,Uuid,Uuid,String,serde_json::Value,Option<serde_json::Value>)>=sqlx::query_as("select ordinal,source_product_id,operation_id,state,command,receipt from grid_product_copy_batch_items where tenant_id=$1 and actor_user_id=$2 and batch_id=$3 and ordinal>$4 and (not $5 or state='REJECTED') order by ordinal limit $6")
        .bind(tenant).bind(actor).bind(batch).bind(after.map_or(-1,|after|after as i64)).bind(failed_only).bind(i64::from(limit)+1).fetch_all(pool).await?;
    let more = rows.len() > usize::from(limit);
    let mut items = Vec::with_capacity(rows.len().min(usize::from(limit)));
    for (ordinal, source, operation_id, state, command, receipt) in
        rows.into_iter().take(usize::from(limit))
    {
        let command: ActionCommand = serde_json::from_value(command)?;
        let receipt: Option<ActionReceipt> = receipt.map(serde_json::from_value).transpose()?;
        let valid = match (&*state, &receipt) {
            ("APPLIED", Some(receipt)) => matches!(receipt.outcome, ActionOutcome::Applied { .. }),
            ("REJECTED", Some(receipt)) => {
                matches!(receipt.outcome, ActionOutcome::Rejected { .. })
            }
            ("PENDING" | "IN_FLIGHT" | "CANCELLED", None) => true,
            _ => false,
        };
        if !valid
            || command.operation_id != RowId(operation_id.as_u128())
            || command.source != RowId(source.as_u128())
            || receipt.as_ref().is_some_and(|receipt| {
                receipt.operation_id != command.operation_id
                    || receipt.source != command.source
                    || receipt.command_fingerprint != command.fingerprint()
            })
        {
            return Err(BatchError::Invalid(
                "stored result identity mismatch".into(),
            ));
        }
        items.push(Item {
            ordinal: u64::try_from(ordinal)
                .map_err(|_| BatchError::Invalid("invalid stored ordinal".into()))?,
            source,
            operation_id,
            state,
            receipt,
        });
    }
    let next_after = if more {
        items.last().map(|item| item.ordinal)
    } else {
        None
    };
    Ok(Results {
        batch_id: batch,
        items,
        next_after,
    })
}
