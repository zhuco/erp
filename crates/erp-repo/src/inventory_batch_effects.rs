//! Owned exact batch changes and immutable children of the actual SKU ledger.
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use erp_domain::quantity::QTY_SCALE;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::inventory_precision::{
    InventoryPreciseEffect, InventoryPrecisionError, require_owned_key,
};

const MAX_BATCHES: usize = 1000;

#[derive(Debug, thiserror::Error)]
pub enum InventoryBatchError {
    #[error("batch not found for this enterprise and SKU")]
    NotFound,
    #[error("an incoming batch-managed SKU requires an explicit batch UUID")]
    IdentityRequired,
    #[error("batch is expired or has insufficient unreserved exact stock")]
    Unavailable,
}

#[derive(Debug, thiserror::Error)]
#[error("interactive batch consumption supports at most 1000 batches per warehouse and SKU")]
pub struct InventoryBatchLimit;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct InventoryBatchEffect {
    pub batch_movement_id: Uuid,
    pub inventory_ledger_id: Uuid,
    pub batch_id: Uuid,
    #[serde(with = "decimal_integer")]
    pub before_on_hand_micros: i64,
    #[serde(with = "decimal_integer")]
    pub after_on_hand_micros: i64,
    #[serde(with = "decimal_integer")]
    pub before_reserved_micros: i64,
    #[serde(with = "decimal_integer")]
    pub after_reserved_micros: i64,
    #[serde(with = "decimal_integer")]
    pub delta_on_hand_micros: i64,
    #[serde(with = "decimal_integer")]
    pub delta_reserved_micros: i64,
    pub decision_at: DateTime<Utc>,
    pub batch_metadata_snapshot: Value,
    pub created_at: DateTime<Utc>,
}

mod decimal_integer {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &i64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OriginalInteger {
            Text(String),
            Number(i64),
        }
        match OriginalInteger::deserialize(deserializer)? {
            OriginalInteger::Number(value) => Ok(value),
            OriginalInteger::Text(text) => {
                let value: i64 = text.parse().map_err(serde::de::Error::custom)?;
                if value.to_string() != text {
                    return Err(serde::de::Error::custom("noncanonical exact integer"));
                }
                Ok(value)
            }
        }
    }
}

#[derive(sqlx::FromRow)]
struct QuantityRow {
    batch_id: Uuid,
    on_hand: i64,
    reserved: i64,
    on_hand_micros: Option<i64>,
    reserved_micros: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct MetadataRow {
    batch_id: Uuid,
    expire_at: Option<DateTime<Utc>>,
    snapshot: Value,
}

struct Candidate {
    id: Uuid,
    hand: i64,
    held: i64,
    existed: bool,
    metadata: MetadataRow,
}

fn reconcile(reason: &'static str) -> anyhow::Error {
    InventoryPrecisionError::ReconciliationRequired(reason).into()
}

/// The caller already applied `effect` under the same physical key guard.
/// `batch=None` uses FEFO only for a negative physical change. Existing batch
/// holds are never spent without a real source-to-batch ownership mapping.
pub(crate) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    effect: &InventoryPreciseEffect,
    batch: Option<Uuid>,
) -> anyhow::Result<Vec<InventoryBatchEffect>> {
    let key = effect.before.key;
    require_owned_key(tx, key).await?;
    let delta = effect
        .balance
        .on_hand
        .try_sub(effect.before.on_hand)?
        .as_micros();
    let enabled: bool = sqlx::query_scalar(
        "select batch_enabled from skus where tenant_id=$1 and sku_id=$2 for share",
    )
    .bind(key.tenant_id.0)
    .bind(key.sku_id)
    .fetch_one(&mut **tx)
    .await?;
    if !enabled && batch.is_none() {
        return Ok(Vec::new());
    }
    if batch.is_none() && delta >= 0 {
        return Err(InventoryBatchError::IdentityRequired.into());
    }
    // The bound is checked before quantity admission or any batch mutation.
    let quantities: Vec<QuantityRow> = sqlx::query_as(
        "select batch_id,on_hand,reserved,on_hand_micros,reserved_micros
         from inventory_batch_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3
         order by batch_id limit 1001 for update",
    )
    .bind(key.tenant_id.0)
    .bind(key.warehouse_id)
    .bind(key.sku_id)
    .fetch_all(&mut **tx)
    .await?;
    if quantities.len() > MAX_BATCHES {
        return Err(InventoryBatchLimit.into());
    }
    let mut ids: Vec<Uuid> = quantities.iter().map(|q| q.batch_id).collect();
    if let Some(id) = batch
        && !ids.contains(&id)
    {
        ids.push(id);
    }
    ids.sort_unstable();
    let rows: Vec<MetadataRow> = sqlx::query_as(
        "select s.batch_id,s.expire_at,to_jsonb(s) as snapshot from sku_batches s
         where s.tenant_id=$1 and s.sku_id=$2 and s.batch_id=any($3)
         order by s.batch_id for share",
    )
    .bind(key.tenant_id.0)
    .bind(key.sku_id)
    .bind(&ids)
    .fetch_all(&mut **tx)
    .await?;
    let mut metadata: HashMap<Uuid, MetadataRow> =
        rows.into_iter().map(|r| (r.batch_id, r)).collect();
    if batch.is_some_and(|id| !metadata.contains_key(&id)) {
        return Err(InventoryBatchError::NotFound.into());
    }
    let mut candidates = Vec::with_capacity(ids.len());
    let mut total_hand = 0_i128;
    let mut total_held = 0_i128;
    for q in quantities {
        let hand = q
            .on_hand_micros
            .ok_or_else(|| reconcile("batch exact on-hand is unknown"))?;
        let held = q
            .reserved_micros
            .ok_or_else(|| reconcile("batch exact reservation is unknown"))?;
        if hand < 0
            || held < 0
            || held > hand
            || hand / QTY_SCALE != q.on_hand
            || held / QTY_SCALE != q.reserved
        {
            return Err(reconcile(
                "batch quantity or whole-unit projection is inconsistent",
            ));
        }
        let m = metadata
            .remove(&q.batch_id)
            .ok_or_else(|| reconcile("stored batch has no owned SKU metadata"))?;
        total_hand += i128::from(hand);
        total_held += i128::from(held);
        candidates.push(Candidate {
            id: q.batch_id,
            hand,
            held,
            existed: true,
            metadata: m,
        });
    }
    if total_hand != i128::from(effect.before.on_hand.as_micros())
        || total_held > i128::from(effect.before.reserved.as_micros())
        || total_held > i128::from(effect.balance.reserved.as_micros())
    {
        return Err(reconcile(
            "batch totals differ from the owned exact SKU effect",
        ));
    }
    if let Some(id) = batch
        && let Some(m) = metadata.remove(&id)
    {
        if candidates.len() == MAX_BATCHES {
            return Err(InventoryBatchLimit.into());
        }
        // Only an absent new owned batch has an explicit zero opening.
        // Unknown historical rows were rejected above, never activated here.
        candidates.push(Candidate {
            id,
            hand: 0,
            held: 0,
            existed: false,
            metadata: m,
        });
    }
    // Expiry is judged after all current metadata/quantity locks are held.
    let decision_at: DateTime<Utc> = sqlx::query_scalar("select clock_timestamp()")
        .fetch_one(&mut **tx)
        .await?;
    let mut changes = Vec::new();
    if let Some(id) = batch {
        let candidate = candidates
            .into_iter()
            .find(|c| c.id == id)
            .ok_or(InventoryBatchError::NotFound)?;
        if delta < 0
            && (candidate
                .metadata
                .expire_at
                .is_some_and(|e| e <= decision_at)
                || i128::from(candidate.hand) - i128::from(candidate.held) < -i128::from(delta))
        {
            return Err(InventoryBatchError::Unavailable.into());
        }
        let after = candidate
            .hand
            .checked_add(delta)
            .ok_or_else(|| reconcile("batch quantity overflows"))?;
        if after < candidate.held {
            return Err(InventoryBatchError::Unavailable.into());
        }
        changes.push((candidate, after));
    } else {
        let available: i128 = candidates
            .iter()
            .filter(|c| {
                c.metadata
                    .expire_at
                    .is_none_or(|expiry| expiry > decision_at)
            })
            .map(|c| i128::from(c.hand - c.held))
            .sum();
        if available < -i128::from(delta) {
            return Err(InventoryPrecisionError::InsufficientAvailable {
                available_micros: available,
                requested_micros: -i128::from(delta),
            }
            .into());
        }
        candidates.sort_by_key(|c| (c.metadata.expire_at.is_none(), c.metadata.expire_at, c.id));
        let mut remaining = -i128::from(delta);
        for candidate in candidates {
            if remaining == 0 {
                break;
            }
            if candidate
                .metadata
                .expire_at
                .is_some_and(|e| e <= decision_at)
            {
                continue;
            }
            let used = remaining.min(i128::from(candidate.hand - candidate.held));
            if used > 0 {
                let after = candidate.hand - i64::try_from(used)?;
                remaining -= used;
                changes.push((candidate, after));
            }
        }
        if remaining != 0 {
            return Err(InventoryBatchError::Unavailable.into());
        }
    }
    // At most two bounded balance statements plus one child INSERT, including
    // FEFO spanning many batches. Never one network round trip per batch.
    let existing = changes.iter().filter(|(c, _)| c.existed).count();
    if existing > 0 {
        let mut update = QueryBuilder::<Postgres>::new(
            "update inventory_batch_balances b set on_hand_micros=v.after_hand,reserved_micros=v.held,updated_at=now() from (",
        );
        update.push_values(
            changes.iter().filter(|(c, _)| c.existed),
            |mut row, (c, after)| {
                row.push_bind(c.id)
                    .push_bind(c.hand)
                    .push_bind(*after)
                    .push_bind(c.held);
            },
        );
        update.push(") as v(batch_id,before_hand,after_hand,held) where b.tenant_id=")
            .push_bind(key.tenant_id.0).push(" and b.warehouse_id=").push_bind(key.warehouse_id)
            .push(" and b.sku_id=").push_bind(key.sku_id)
            .push(" and b.batch_id=v.batch_id and b.on_hand_micros=v.before_hand and b.reserved_micros=v.held");
        if update.build().execute(&mut **tx).await?.rows_affected() != u64::try_from(existing)? {
            return Err(reconcile(
                "batch update did not affect every owned planned row",
            ));
        }
    }
    let new_rows = changes.len() - existing;
    if new_rows > 0 {
        let mut insert = QueryBuilder::<Postgres>::new(
            "insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros) ",
        );
        insert.push_values(
            changes.iter().filter(|(c, _)| !c.existed),
            |mut row, (c, after)| {
                row.push_bind(key.tenant_id.0)
                    .push_bind(key.warehouse_id)
                    .push_bind(key.sku_id)
                    .push_bind(c.id)
                    .push_bind(*after)
                    .push_bind(c.held);
            },
        );
        if insert.build().execute(&mut **tx).await?.rows_affected() != u64::try_from(new_rows)? {
            return Err(reconcile(
                "new batch insert did not affect every owned planned row",
            ));
        }
    }
    let mut insert = QueryBuilder::<Postgres>::new(
        "insert into inventory_batch_movements(tenant_id,warehouse_id,sku_id,batch_id,inventory_ledger_id,
         before_on_hand_micros,after_on_hand_micros,before_reserved_micros,after_reserved_micros,
         delta_on_hand_micros,delta_reserved_micros,decision_at,batch_metadata_snapshot,reason_type,ref_type,ref_id)
         select l.tenant_id,l.warehouse_id,l.sku_id,v.batch_id,l.ledger_id,
          v.before_hand,v.after_hand,v.held,v.held,v.delta,0,",
    );
    insert
        .push_bind(decision_at)
        .push(",v.metadata,l.reason_type,l.ref_type,l.ref_id from (");
    insert.push_values(&changes, |mut row, (c, after)| {
        row.push_bind(c.id)
            .push_bind(c.hand)
            .push_bind(*after)
            .push_bind(c.held)
            .push_bind(*after - c.hand)
            .push_bind(&c.metadata.snapshot);
    });
    insert.push(") as v(batch_id,before_hand,after_hand,held,delta,metadata) join inventory_ledgers l on l.tenant_id=")
        .push_bind(key.tenant_id.0).push(" and l.warehouse_id=").push_bind(key.warehouse_id)
        .push(" and l.sku_id=").push_bind(key.sku_id).push(" and l.ledger_id=").push_bind(effect.ledger_id)
        .push(" returning batch_movement_id,inventory_ledger_id,batch_id,before_on_hand_micros,after_on_hand_micros,
         before_reserved_micros,after_reserved_micros,delta_on_hand_micros,delta_reserved_micros,
         decision_at,batch_metadata_snapshot,created_at");
    let mut effects: Vec<InventoryBatchEffect> =
        insert.build_query_as().fetch_all(&mut **tx).await?;
    if effects.len() != changes.len() {
        return Err(reconcile("missing required batch provenance row"));
    }
    effects.sort_by_key(|e| e.batch_id);
    Ok(effects)
}

/// Replay binds only immutable original children, never today's metadata/balance.
pub(crate) async fn verify(
    tx: &mut Transaction<'_, Postgres>,
    input: &crate::inventory_movement_commands::InventoryMovementCommandInput,
    ledger: Uuid,
    original: &[InventoryBatchEffect],
) -> anyhow::Result<bool> {
    let saved: Vec<InventoryBatchEffect> = sqlx::query_as(
        "select batch_movement_id,inventory_ledger_id,batch_id,before_on_hand_micros,after_on_hand_micros,
         before_reserved_micros,after_reserved_micros,delta_on_hand_micros,delta_reserved_micros,
         decision_at,batch_metadata_snapshot,created_at from inventory_batch_movements
         where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 and inventory_ledger_id=$4 order by batch_id",
    ).bind(input.tenant_id.0).bind(input.warehouse_id).bind(input.sku_id).bind(ledger)
        .fetch_all(&mut **tx).await?;
    if saved != original || saved.len() > MAX_BATCHES {
        return Ok(false);
    }
    if let Some(batch) = input.batch_id
        && (saved.len() != 1 || saved[0].batch_id != batch)
    {
        return Ok(false);
    }
    Ok(true)
}
