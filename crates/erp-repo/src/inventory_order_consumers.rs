//! Terminal actions on applied physical order sources; never re-expand a catalog.
use anyhow::Result;
use chrono::Utc;
use erp_domain::quantity::ScaledQty;
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::inventory::{
    InventoryError, InventoryRepo, InventoryService, validate_reservation_input,
};
use crate::inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey, require_owned_key};

pub(crate) const MAX_APPLIED_ORDER_ITEMS: usize = 4096;

#[derive(Clone)]
pub(crate) struct AppliedOrderItem {
    pub(crate) warehouse_id: Uuid,
    pub(crate) sku_id: Uuid,
    pub(crate) order_item_id: Uuid,
    pub(crate) order_id: Uuid,
    pub(crate) qty: i64,
}

#[derive(sqlx::FromRow)]
struct PhysicalSource {
    warehouse_id: Uuid,
    sku_id: Uuid,
    order_id: Option<Uuid>,
    qty: i64,
    status: String,
    qty_micros: Option<i64>,
    original_qty_micros: Option<i64>,
    location_id: Option<Uuid>,
    quality_status: Option<String>,
    batch_code: Option<String>,
    source_type: Option<String>,
    source_id: Option<String>,
    reservation_source_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct FrozenChild {
    root_order_item_id: Uuid,
    warehouse_id: Uuid,
    sku_id: Uuid,
    order_id: Option<Uuid>,
    mapped_sku: Uuid,
    mapped_qty: i64,
}

pub(crate) async fn lock_order_roots(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    roots: &[Uuid],
) -> Result<()> {
    let mut roots = roots.to_vec();
    roots.sort_unstable();
    roots.dedup();
    for root in roots {
        if root.is_nil() {
            return Err(InventoryError::ReservationMismatch.into());
        }
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(tenant.to_string())
            .bind(format!("order-reservation-root:{root}"))
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

pub(crate) async fn lock_order_keys(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    keys: &[(Uuid, Uuid)],
) -> Result<()> {
    let mut keys = keys.to_vec();
    keys.sort_unstable();
    keys.dedup();
    if keys.len() > MAX_APPLIED_ORDER_ITEMS {
        return Err(InventoryError::ReservationComponentsLimit.into());
    }
    for (warehouse, sku) in keys {
        require_owned_key(
            tx,
            InventoryQuantityKey {
                tenant_id: tenant,
                warehouse_id: warehouse,
                sku_id: sku,
            },
        )
        .await?;
    }
    Ok(())
}

fn matches(source: &PhysicalSource, item: &AppliedOrderItem) -> Result<()> {
    let micros = ScaledQty::from_legacy_whole_units(item.qty)?.as_micros();
    if source.warehouse_id != item.warehouse_id
        || source.sku_id != item.sku_id
        || source.order_id != Some(item.order_id)
        || source.qty != item.qty
        || source.qty_micros.is_some_and(|value| value != micros)
        || source
            .original_qty_micros
            .is_some_and(|value| value != micros)
        || source.location_id.is_some()
        || source.quality_status.is_some()
        || source.batch_code.is_some()
        || source.source_type.is_some()
        || source.source_id.is_some()
    {
        return Err(InventoryError::ReservationMismatch.into());
    }
    Ok(())
}

pub(crate) async fn apply_order_terminal_items(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    items: &[AppliedOrderItem],
    consume: bool,
) -> Result<i64> {
    if items.len() > MAX_APPLIED_ORDER_ITEMS {
        return Err(InventoryError::ReservationComponentsLimit.into());
    }
    let mut ordered = items.to_vec();
    ordered.sort_by_key(|item| (item.warehouse_id, item.sku_id, item.order_item_id));
    let mut ids = std::collections::HashSet::new();
    let mut roots = Vec::new();
    let mut keys = Vec::new();
    let mut expected_attachments = std::collections::HashMap::new();
    for item in &ordered {
        validate_reservation_input(
            tenant,
            item.warehouse_id,
            item.sku_id,
            item.order_item_id,
            item.qty,
        )?;
        if item.order_id.is_nil() || !ids.insert(item.order_item_id) {
            return Err(InventoryError::ReservationMismatch.into());
        }
        let source:PhysicalSource=sqlx::query_as("select warehouse_id,sku_id,order_id,qty,status,
            qty_micros,original_qty_micros,location_id,quality_status,batch_code,source_type,source_id,
            reservation_source_id from inventory_reservations where tenant_id=$1 and order_item_id=$2")
            .bind(tenant.0).bind(item.order_item_id).fetch_optional(&mut **tx).await?
            .ok_or(InventoryError::ReservationNotFound)?;
        matches(&source, item)?;
        if let Some(id) = source.reservation_source_id {
            let child:FrozenChild=sqlx::query_as("select s.root_order_item_id,s.warehouse_id,s.sku_id,s.order_id,
                c.sku_id as mapped_sku,c.qty as mapped_qty from inventory_order_reservation_sources s
                join inventory_order_reservation_components c on c.tenant_id=s.tenant_id and c.source_id=s.source_id
                where s.tenant_id=$1 and s.source_id=$2 and c.order_item_id=$3")
                .bind(tenant.0).bind(id).bind(item.order_item_id).fetch_optional(&mut **tx).await?
                .ok_or(InventoryError::ReservationSourceUnverified)?;
            if child.warehouse_id != item.warehouse_id
                || child.order_id != Some(item.order_id)
                || child.mapped_sku != item.sku_id
                || child.mapped_qty != item.qty
            {
                return Err(InventoryError::ReservationMismatch.into());
            }
            roots.push(child.root_order_item_id);
            keys.push((child.warehouse_id, child.sku_id));
        } else {
            // The actual old physical row is sufficient for its own order,
            // warehouse, SKU and qty. Do not invent a parent or attach it.
            roots.push(item.order_item_id);
        }
        expected_attachments.insert(item.order_item_id, source.reservation_source_id);
        keys.push((item.warehouse_id, item.sku_id));
    }
    // Shared root and physical guards precede source rows and balances. Public
    // root actions use the same order; complete batches cannot invert keys.
    lock_order_roots(tx, tenant, &roots).await?;
    lock_order_keys(tx, tenant, &keys).await?;
    let service = InventoryService::new(pool);
    let new_status = if consume { "CONSUMED" } else { "RELEASED" };
    let mut applied = 0_i64;
    for item in ordered {
        let source:PhysicalSource=sqlx::query_as("select warehouse_id,sku_id,order_id,qty,status,
            qty_micros,original_qty_micros,location_id,quality_status,batch_code,source_type,source_id,
            reservation_source_id from inventory_reservations where tenant_id=$1 and order_item_id=$2 for update")
            .bind(tenant.0).bind(item.order_item_id).fetch_optional(&mut **tx).await?
            .ok_or(InventoryError::ReservationNotFound)?;
        matches(&source, &item)?;
        if expected_attachments.get(&item.order_item_id) != Some(&source.reservation_source_id) {
            return Err(InventoryError::ReservationMismatch.into());
        }
        if !consume && matches!(source.status.as_str(), "RELEASED" | "CONSUMED") {
            continue;
        }
        // A different new shipment cannot treat CONSUMED as its own success.
        // Same-shipment replay is identified by that shipment's committed row.
        if source.status != "RESERVED" {
            return Err(InventoryError::ReservationNotActive.into());
        }
        let quantity = ScaledQty::from_legacy_whole_units(item.qty)?;
        let effect = service
            .apply_precise_effect_with_tx(
                tx,
                InventoryPreciseDelta {
                    key: InventoryQuantityKey {
                        tenant_id: tenant,
                        warehouse_id: item.warehouse_id,
                        sku_id: item.sku_id,
                    },
                    on_hand: if consume {
                        ScaledQty::from_micros(-quantity.as_micros())
                    } else {
                        ScaledQty::ZERO
                    },
                    reserved: ScaledQty::from_micros(-quantity.as_micros()),
                    frozen: ScaledQty::ZERO,
                    in_transit: ScaledQty::ZERO,
                    reason_type: if consume {
                        "SALES_SHIP"
                    } else {
                        "RESERVATION_RELEASE"
                    }
                    .into(),
                    ref_type: Some("ORDER_ITEM".into()),
                    ref_id: Some(item.order_item_id.to_string()),
                    occurred_at: Utc::now(),
                },
            )
            .await?;
        if consume {
            InventoryRepo::new(pool)
                .consume_batches_by_fefo(tx, &effect)
                .await?;
        }
        sqlx::query("update inventory_reservations set status=$3,updated_at=now() where tenant_id=$1 and order_item_id=$2")
            .bind(tenant.0).bind(item.order_item_id).bind(new_status).execute(&mut **tx).await?;
        applied += 1;
    }
    Ok(applied)
}
