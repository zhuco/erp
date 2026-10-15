//! Formal source reservations and immutable per-command location results.
use anyhow::Result;
use chrono::{DateTime, Utc};
use erp_domain::quantity::{QTY_SCALE, ScaledQty, format_qty, parse_qty};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    inventory::{InventoryBalance, InventoryService},
    inventory_location_commands::{InventoryInboundLocation, lock_location_quantities},
    inventory_movement_commands::{
        InventoryCommandError, InventoryMovementCommandInput, InventoryMovementKind,
    },
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryReservationSource {
    pub source_line: String,
    /// Opaque source context; this does not confirm or transition a sales order.
    pub order_id: Option<Uuid>,
}

impl InventoryReservationSource {
    pub(crate) fn normalize(mut self) -> Result<Self> {
        self.source_line = self.source_line.trim().to_owned();
        if self.source_line.is_empty()
            || self.source_line.len() > 128
            || self.source_line.chars().any(char::is_control)
            || self.order_id.is_some_and(|id| id.is_nil())
        {
            return Err(InventoryCommandError::InvalidPayload("invalid reservation source").into());
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryReservationEffect {
    pub reservation_id: Uuid,
    pub ledger_id: Uuid,
    pub source_line: String,
    pub original_quantity: String,
    pub remaining_quantity: String,
    pub released_quantity: String,
    pub location_reserved: String,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct SourceRow {
    reservation_id: Uuid,
    warehouse_id: Uuid,
    sku_id: Uuid,
    location_id: Uuid,
    batch_code: String,
    quality_status: String,
    source_context: Value,
    original_qty_micros: i64,
    qty_micros: i64,
    released_micros: i64,
}

pub(crate) async fn apply_location_reservation(
    service: &InventoryService<'_>,
    tx: &mut Transaction<'_, Postgres>,
    input: &InventoryMovementCommandInput,
    location: &InventoryInboundLocation,
    source: &InventoryReservationSource,
) -> Result<(InventoryBalance, InventoryReservationEffect)> {
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
        .bind(input.tenant_id.to_string())
        .bind(format!(
            "inventory-reservation-source:{}",
            source.source_line
        ))
        .execute(&mut **tx)
        .await?;
    let amounts = lock_location_quantities(
        tx,
        input,
        location,
        input.kind == InventoryMovementKind::Release,
    )
    .await?;
    let micros = input.quantity_delta.checked_mul(QTY_SCALE).ok_or(
        InventoryCommandError::InvalidPayload("reservation exceeds exact range"),
    )?;
    let context = serde_json::to_value(source)?;
    let row: Option<SourceRow> = sqlx::query_as("select reservation_id,warehouse_id,sku_id,
        location_id,batch_code,quality_status,source_context,original_qty_micros,qty_micros,released_micros
        from inventory_reservations where tenant_id=$1 and source_type='LOCATION_COMMAND'
        and source_id=$2 for update")
        .bind(input.tenant_id.0).bind(&source.source_line).fetch_optional(&mut **tx).await?;
    let (reservation_id, original, remaining, released) = match input.kind {
        InventoryMovementKind::Reserve => {
            if row.is_some() {
                return Err(InventoryCommandError::ReservationSourceConflict.into());
            }
            let available = i128::from(amounts[0])
                - i128::from(amounts[1])
                - i128::from(amounts[2])
                - i128::from(amounts[3]);
            if available < 0 {
                return Err(InventoryCommandError::ReconciliationRequired.into());
            }
            if i128::from(micros) > available {
                return Err(InventoryCommandError::InsufficientLocationStock {
                    available_micros: available,
                    requested_micros: i128::from(micros),
                }
                .into());
            }
            (Uuid::new_v4(), micros, micros, 0)
        }
        InventoryMovementKind::Release => {
            let row = row.ok_or(InventoryCommandError::ReservationSourceConflict)?;
            if row.warehouse_id != input.warehouse_id
                || row.sku_id != input.sku_id
                || row.location_id != location.location_id
                || row.batch_code != location.batch_code
                || row.quality_status != location.quality_status
                || row.source_context != context
            {
                return Err(InventoryCommandError::ReservationSourceConflict.into());
            }
            if row.qty_micros < 0
                || row.released_micros < 0
                || row.original_qty_micros <= 0
                || row.qty_micros.checked_add(row.released_micros) != Some(row.original_qty_micros)
            {
                return Err(InventoryCommandError::ReconciliationRequired.into());
            }
            let requested = micros
                .checked_neg()
                .ok_or(InventoryCommandError::ReconciliationRequired)?;
            if requested > row.qty_micros {
                return Err(InventoryCommandError::ReservationRemainingInsufficient {
                    remaining_micros: row.qty_micros,
                    requested_micros: requested,
                }
                .into());
            }
            if requested > amounts[1] {
                return Err(InventoryCommandError::ReconciliationRequired.into());
            }
            (
                row.reservation_id,
                row.original_qty_micros,
                row.qty_micros - requested,
                row.released_micros
                    .checked_add(requested)
                    .ok_or(InventoryCommandError::ReconciliationRequired)?,
            )
        }
        _ => {
            return Err(InventoryCommandError::InvalidPayload("invalid reservation action").into());
        }
    };
    let next_reserved = amounts[1]
        .checked_add(micros)
        .filter(|value| *value >= 0)
        .ok_or(InventoryCommandError::ReconciliationRequired)?;
    let ledger_id = Uuid::new_v4();
    let balance = service
        .apply_precise_with_tx(
            tx,
            InventoryPreciseDelta {
                key: InventoryQuantityKey {
                    tenant_id: input.tenant_id,
                    warehouse_id: input.warehouse_id,
                    sku_id: input.sku_id,
                },
                on_hand: ScaledQty::ZERO,
                reserved: ScaledQty::from_micros(micros),
                frozen: ScaledQty::ZERO,
                in_transit: ScaledQty::ZERO,
                reason_type: input.kind.reason_type().into(),
                ref_type: Some("shared_stock_ledger".into()),
                ref_id: Some(ledger_id.to_string()),
                occurred_at: Utc::now(),
            },
        )
        .await?
        .whole_projection()?;
    let created_at: DateTime<Utc> = sqlx::query_scalar(
        "insert into shared_stock_ledgers(
        tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,
        delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,$6,$7,'RESERVED',$8,$9,$10)
        returning created_at",
    )
    .bind(input.tenant_id.0)
    .bind(ledger_id)
    .bind(input.warehouse_id)
    .bind(location.location_id)
    .bind(input.sku_id)
    .bind(&location.batch_code)
    .bind(&location.quality_status)
    .bind(micros)
    .bind(&source.source_line)
    .bind(&input.request_id)
    .fetch_one(&mut **tx)
    .await?;
    if input.kind == InventoryMovementKind::Reserve {
        sqlx::query(
            "insert into inventory_reservations(reservation_id,tenant_id,warehouse_id,sku_id,
            qty,status,qty_micros,original_qty_micros,source_type,source_id,
            location_id,batch_code,quality_status,source_context,source_request_id,released_micros)
            values($1,$2,$3,$4,$5,'RESERVED',$6,$6,'LOCATION_COMMAND',$7,$8,$9,$10,$11,$12,0)",
        )
        .bind(reservation_id)
        .bind(input.tenant_id.0)
        .bind(input.warehouse_id)
        .bind(input.sku_id)
        .bind(micros / QTY_SCALE)
        .bind(micros)
        .bind(&source.source_line)
        .bind(location.location_id)
        .bind(&location.batch_code)
        .bind(&location.quality_status)
        .bind(context)
        .bind(&input.request_id)
        .execute(&mut **tx)
        .await?;
    } else {
        let updated = sqlx::query("update inventory_reservations set qty_micros=$3,qty=$3/1000000,
            released_micros=$4,status=case when $3=0 then 'RELEASED' else 'RESERVED' end,updated_at=now()
            where tenant_id=$1 and reservation_id=$2 and source_type='LOCATION_COMMAND'")
            .bind(input.tenant_id.0).bind(reservation_id).bind(remaining).bind(released)
            .execute(&mut **tx).await?;
        if updated.rows_affected() != 1 {
            return Err(InventoryCommandError::ReconciliationRequired.into());
        }
    }
    sqlx::query(
        "insert into inventory_location_reservation_events(tenant_id,request_id,reservation_id,
        ledger_id,delta_micros,remaining_micros,released_micros,location_reserved_micros,created_at)
        values($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(input.tenant_id.0)
    .bind(&input.request_id)
    .bind(reservation_id)
    .bind(ledger_id)
    .bind(micros)
    .bind(remaining)
    .bind(released)
    .bind(next_reserved)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;
    Ok((
        balance,
        InventoryReservationEffect {
            reservation_id,
            ledger_id,
            source_line: source.source_line.clone(),
            original_quantity: format_qty(ScaledQty::from_micros(original)),
            remaining_quantity: format_qty(ScaledQty::from_micros(remaining)),
            released_quantity: format_qty(ScaledQty::from_micros(released)),
            location_reserved: format_qty(ScaledQty::from_micros(next_reserved)),
            created_at,
        },
    ))
}

pub(crate) async fn verify_reservation_effect(
    tx: &mut Transaction<'_, Postgres>,
    input: &InventoryMovementCommandInput,
    location: &InventoryInboundLocation,
    source: &InventoryReservationSource,
    effect: &InventoryReservationEffect,
) -> Result<bool> {
    let mut amounts = [0_i64; 4];
    for (index, text) in [
        &effect.original_quantity,
        &effect.remaining_quantity,
        &effect.released_quantity,
        &effect.location_reserved,
    ]
    .into_iter()
    .enumerate()
    {
        let Ok(quantity) = parse_qty(text) else {
            return Ok(false);
        };
        if quantity.as_micros() < 0 || format_qty(quantity) != *text {
            return Ok(false);
        }
        amounts[index] = quantity.as_micros();
    }
    if amounts[0] <= 0
        || amounts[1].checked_add(amounts[2]) != Some(amounts[0])
        || effect.source_line != source.source_line
    {
        return Ok(false);
    }
    let micros = input
        .quantity_delta
        .checked_mul(QTY_SCALE)
        .ok_or(InventoryCommandError::ReconciliationRequired)?;
    Ok(sqlx::query_scalar("select exists(select 1 from inventory_location_reservation_events e
        join inventory_reservations r on r.reservation_id=e.reservation_id and r.tenant_id=e.tenant_id
        join shared_stock_ledgers s on s.ledger_id=e.ledger_id and s.tenant_id=e.tenant_id
        where e.tenant_id=$1 and e.request_id=$2 and e.reservation_id=$3 and e.ledger_id=$4
        and e.delta_micros=$5 and e.remaining_micros=$6 and e.released_micros=$7
        and e.location_reserved_micros=$8 and e.created_at=$9
        and r.source_type='LOCATION_COMMAND' and r.source_id=$10 and r.original_qty_micros=$11
        and r.source_context=$12 and r.warehouse_id=$13 and r.sku_id=$14
        and r.location_id=$15 and r.batch_code=$16 and r.quality_status=$17
        and s.warehouse_id=$13 and s.sku_id=$14 and s.location_id=$15
        and s.batch_code=$16 and s.quality_status=$17 and s.bucket='RESERVED'
        and s.delta_micros=$5 and s.request_id=$2 and s.source_line=$10 and s.created_at=$9)")
        .bind(input.tenant_id.0).bind(&input.request_id).bind(effect.reservation_id).bind(effect.ledger_id)
        .bind(micros).bind(amounts[1]).bind(amounts[2]).bind(amounts[3]).bind(effect.created_at)
        .bind(&source.source_line).bind(amounts[0]).bind(serde_json::to_value(source)?)
        .bind(input.warehouse_id).bind(input.sku_id).bind(location.location_id)
        .bind(&location.batch_code).bind(&location.quality_status).fetch_one(&mut **tx).await?)
}
