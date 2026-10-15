//! A location projection cites the same transaction's authoritative inventory effect.
use anyhow::Result;
use chrono::{DateTime, Utc};
use erp_domain::quantity::{QTY_SCALE, ScaledQty, format_qty, parse_qty};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    inventory::{InventoryBalance, InventoryService},
    inventory_movement_commands::{
        InventoryCommandError, InventoryMovementCommandInput, InventoryMovementKind,
    },
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey, require_owned_key},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryInboundLocation {
    pub location_id: Uuid,
    pub batch_code: String,
    pub quality_status: String,
}

impl InventoryInboundLocation {
    pub(crate) fn normalize(mut self) -> Result<Self> {
        self.batch_code = self.batch_code.trim().to_owned();
        self.quality_status = self.quality_status.trim().to_owned();
        if self.location_id.is_nil()
            || self.batch_code.is_empty()
            || self.batch_code.len() > 128
            || self.batch_code.chars().any(char::is_control)
            || !matches!(
                self.quality_status.as_str(),
                "GOOD" | "PENDING" | "FAILED" | "HOLD"
            )
        {
            return Err(InventoryCommandError::InvalidPayload(
                "invalid inbound location, batch or quality status",
            )
            .into());
        }
        Ok(self)
    }

    pub(crate) fn frozen_delta(&self, quantity_delta: i64) -> Result<i64> {
        if self.quality_status == "GOOD" {
            return Ok(0);
        }
        quantity_delta.checked_mul(QTY_SCALE).ok_or_else(|| {
            InventoryCommandError::InvalidPayload("inbound quantity exceeds exact range").into()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryLocationEffect {
    pub ledger_id: Uuid,
    /// Original location/batch/quality on-hand, never the later location balance.
    pub on_hand: String,
    pub created_at: DateTime<Utc>,
}

pub(crate) async fn apply_location_inbound(
    service: &InventoryService<'_>,
    tx: &mut Transaction<'_, Postgres>,
    input: &InventoryMovementCommandInput,
    location: &InventoryInboundLocation,
) -> Result<(InventoryBalance, InventoryLocationEffect)> {
    apply_location_movement(service, tx, input, location).await
}

pub(crate) async fn apply_location_outbound(
    service: &InventoryService<'_>,
    tx: &mut Transaction<'_, Postgres>,
    input: &InventoryMovementCommandInput,
    location: &InventoryInboundLocation,
) -> Result<(InventoryBalance, InventoryLocationEffect)> {
    apply_location_movement(service, tx, input, location).await
}

async fn apply_location_movement(
    service: &InventoryService<'_>,
    tx: &mut Transaction<'_, Postgres>,
    input: &InventoryMovementCommandInput,
    location: &InventoryInboundLocation,
) -> Result<(InventoryBalance, InventoryLocationEffect)> {
    let amounts = lock_location_quantities(tx, input, location, false).await?;
    let key = InventoryQuantityKey {
        tenant_id: input.tenant_id,
        warehouse_id: input.warehouse_id,
        sku_id: input.sku_id,
    };
    let before = amounts[0];
    let available = i128::from(before)
        - i128::from(amounts[1])
        - i128::from(amounts[2])
        - i128::from(amounts[3]);
    if available < 0 {
        return Err(InventoryCommandError::ReconciliationRequired.into());
    }
    let micros = input.quantity_delta.checked_mul(QTY_SCALE).ok_or(
        InventoryCommandError::InvalidPayload("movement quantity exceeds exact range"),
    )?;
    if input.kind == InventoryMovementKind::Outbound && -i128::from(micros) > available {
        return Err(InventoryCommandError::InsufficientLocationStock {
            available_micros: available,
            requested_micros: -i128::from(micros),
        }
        .into());
    }
    let next = before
        .checked_add(micros)
        .filter(|_| before >= 0)
        .ok_or(InventoryCommandError::ReconciliationRequired)?;
    let ledger_id = Uuid::new_v4();
    let balance = service
        .apply_precise_with_tx(
            tx,
            InventoryPreciseDelta {
                key,
                on_hand: ScaledQty::from_micros(micros),
                reserved: ScaledQty::ZERO,
                frozen: ScaledQty::from_micros(location.frozen_delta(input.quantity_delta)?),
                in_transit: ScaledQty::ZERO,
                reason_type: input.kind.reason_type().into(),
                ref_type: Some("shared_stock_ledger".into()),
                ref_id: Some(ledger_id.to_string()),
                occurred_at: Utc::now(),
            },
        )
        .await?
        .whole_projection()?;
    let created_at:DateTime<Utc>=sqlx::query_scalar("insert into shared_stock_ledgers(
        tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id)
        values($1,$2,$3,$4,$5,$6,$7,'ON_HAND',$8,'',$9) returning created_at")
        .bind(input.tenant_id.0).bind(ledger_id).bind(input.warehouse_id).bind(location.location_id)
        .bind(input.sku_id).bind(&location.batch_code).bind(&location.quality_status).bind(micros)
        .bind(&input.request_id).fetch_one(&mut **tx).await?;
    Ok((
        balance,
        InventoryLocationEffect {
            ledger_id,
            on_hand: format_qty(ScaledQty::from_micros(next)),
            created_at,
        },
    ))
}

pub(crate) async fn lock_location_quantities(
    tx: &mut Transaction<'_, Postgres>,
    input: &InventoryMovementCommandInput,
    location: &InventoryInboundLocation,
    allow_disabled: bool,
) -> Result<[i64; 4]> {
    let owned: Option<Uuid> = sqlx::query_scalar(
        "select location_id from warehouse_locations
        where tenant_id=$1 and warehouse_id=$2 and location_id=$3 and ($4 or not is_disabled) for share",
    )
    .bind(input.tenant_id.0)
    .bind(input.warehouse_id)
    .bind(location.location_id)
    .bind(allow_disabled)
    .fetch_optional(&mut **tx)
    .await?;
    if owned.is_none() {
        return Err(InventoryCommandError::InvalidPayload(
            "movement location is missing, disabled or outside warehouse",
        )
        .into());
    }
    // Preserve the shared writer's location-before-physical-key lock order.
    sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
        .bind(input.tenant_id.to_string())
        .bind(format!(
            "stock-ledger:{}:{}:{}:{}:{}:ON_HAND",
            input.warehouse_id,
            location.location_id,
            input.sku_id,
            location.batch_code,
            location.quality_status
        ))
        .execute(&mut **tx)
        .await?;
    let key = InventoryQuantityKey {
        tenant_id: input.tenant_id,
        warehouse_id: input.warehouse_id,
        sku_id: input.sku_id,
    };
    // All projection INSERT guards take this physical lock. Acquire it before
    // reading location reservations, so a different writer cannot pass between
    // this admission read and the authoritative stock effect.
    require_owned_key(tx, key).await?;
    let totals: (String, String, String, String) = sqlx::query_as(
        "select
         coalesce(sum(delta_micros) filter(where bucket='ON_HAND'),0)::text,
         coalesce(sum(delta_micros) filter(where bucket='RESERVED'),0)::text,
         coalesce(sum(delta_micros) filter(where bucket='FROZEN'),0)::text,
         coalesce(sum(delta_micros) filter(where bucket='UNAVAILABLE'),0)::text
         from shared_stock_ledgers where tenant_id=$1 and warehouse_id=$2 and location_id=$3
         and sku_id=$4 and batch_code=$5 and quality_status=$6
         and bucket in ('ON_HAND','RESERVED','FROZEN','UNAVAILABLE')",
    )
    .bind(input.tenant_id.0)
    .bind(input.warehouse_id)
    .bind(location.location_id)
    .bind(input.sku_id)
    .bind(&location.batch_code)
    .bind(&location.quality_status)
    .fetch_one(&mut **tx)
    .await?;
    let mut amounts = [0_i64; 4];
    for (index, text) in [&totals.0, &totals.1, &totals.2, &totals.3]
        .into_iter()
        .enumerate()
    {
        amounts[index] = text
            .parse::<i64>()
            .ok()
            .filter(|amount| *amount >= 0)
            .ok_or(InventoryCommandError::ReconciliationRequired)?;
    }
    Ok(amounts)
}

pub(crate) async fn verify_location_effect(
    tx: &mut Transaction<'_, Postgres>,
    input: &InventoryMovementCommandInput,
    location: &InventoryInboundLocation,
    effect: &InventoryLocationEffect,
) -> Result<bool> {
    let Ok(on_hand) = parse_qty(&effect.on_hand) else {
        return Ok(false);
    };
    let micros = input
        .quantity_delta
        .checked_mul(QTY_SCALE)
        .ok_or(InventoryCommandError::ReconciliationRequired)?;
    if effect.ledger_id.is_nil()
        || on_hand.as_micros() < 0
        || (input.kind == InventoryMovementKind::Inbound && on_hand.as_micros() < micros)
        || format_qty(on_hand) != effect.on_hand
    {
        return Ok(false);
    }
    Ok(sqlx::query_scalar(
        "select exists(select 1 from shared_stock_ledgers
        where tenant_id=$1 and ledger_id=$2 and warehouse_id=$3 and location_id=$4
        and sku_id=$5 and batch_code=$6 and quality_status=$7 and bucket='ON_HAND'
        and delta_micros=$8 and request_id=$9 and source_line='' and created_at=$10)",
    )
    .bind(input.tenant_id.0)
    .bind(effect.ledger_id)
    .bind(input.warehouse_id)
    .bind(location.location_id)
    .bind(input.sku_id)
    .bind(&location.batch_code)
    .bind(&location.quality_status)
    .bind(micros)
    .bind(&input.request_id)
    .bind(effect.created_at)
    .fetch_one(&mut **tx)
    .await?)
}
