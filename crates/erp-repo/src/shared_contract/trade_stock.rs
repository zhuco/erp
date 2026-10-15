//! A trade stock projection and its exact inventory ledger share the caller's transaction.
use crate::{
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use erp_domain::quantity::ScaledQty;
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) struct StockPosting<'a> {
    pub(super) tenant: TenantId,
    pub(super) warehouse: Uuid,
    pub(super) location: Uuid,
    pub(super) sku: Uuid,
    pub(super) batch: &'a str,
    pub(super) quality: &'a str,
    pub(super) bucket: &'a str,
    pub(super) delta_micros: i64,
    pub(super) source_line: &'a str,
    pub(super) request: &'a str,
    pub(super) reason: &'a str,
}

pub(super) async fn post(
    tx: &mut Transaction<'_, Postgres>,
    pool: &PgPool,
    input: StockPosting<'_>,
) -> anyhow::Result<Uuid> {
    // Preserve the existing shop/quality shadow contract. Those writers still
    // require their own source integration; the original DB guard remains active.
    if matches!(input.bucket, "UNAVAILABLE" | "HELD" | "WIP") {
        return super::insert_stock_row(
            tx,
            input.tenant,
            input.warehouse,
            input.location,
            input.sku,
            input.batch.trim(),
            input.quality.trim(),
            input.bucket,
            input.delta_micros,
            input.source_line.trim(),
            input.request.trim(),
        )
        .await;
    }
    let batch = input.batch.trim();
    let quality = input.quality.trim();
    if input.delta_micros == 0
        || batch.is_empty()
        || batch.len() > 512
        || !matches!(quality, "GOOD" | "PENDING" | "FAILED" | "HOLD")
    {
        anyhow::bail!(
            "trade stock requires a bounded batch, known quality and nonzero exact quantity"
        );
    }
    if input.bucket == "RESERVED" && input.delta_micros > 0 && quality != "GOOD" {
        anyhow::bail!("trade reservation requires GOOD stock");
    }
    let delta = ScaledQty::from_micros(input.delta_micros);
    let mut change = InventoryPreciseDelta {
        key: InventoryQuantityKey {
            tenant_id: input.tenant,
            warehouse_id: input.warehouse,
            sku_id: input.sku,
        },
        on_hand: ScaledQty::ZERO,
        reserved: ScaledQty::ZERO,
        frozen: ScaledQty::ZERO,
        in_transit: ScaledQty::ZERO,
        reason_type: input.reason.into(),
        ref_type: Some("shared_stock_ledger".into()),
        ref_id: None,
        occurred_at: chrono::Utc::now(),
    };
    match input.bucket {
        "ON_HAND" => {
            change.on_hand = delta;
            if quality != "GOOD" {
                change.frozen = delta;
            }
        }
        "RESERVED" => change.reserved = delta,
        "FROZEN" => change.frozen = delta,
        "IN_TRANSIT" => change.in_transit = delta,
        _ => anyhow::bail!("trade physical posting requires an inventory quantity bucket"),
    }
    let ledger = Uuid::new_v4();
    change.ref_id = Some(ledger.to_string());
    InventoryService::new(pool)
        .apply_precise_with_tx(tx, change)
        .await?;
    let written = sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
        .bind(input.tenant.0).bind(ledger).bind(input.warehouse).bind(input.location).bind(input.sku)
        .bind(batch).bind(quality).bind(input.bucket).bind(input.delta_micros)
        .bind(input.source_line.trim()).bind(input.request.trim()).execute(&mut **tx).await?.rows_affected();
    if written != 1 {
        anyhow::bail!("trade stock projection was not stored");
    }
    Ok(ledger)
}
