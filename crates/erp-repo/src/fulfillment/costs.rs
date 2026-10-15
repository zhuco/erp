//! Mandatory shipment costs cite known catalog facts and never saturate arithmetic.
use std::collections::HashMap;

use erp_types::TenantId;
use sqlx::{Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use super::models::OmsShipmentItem;
use crate::inventory::InventoryError;

#[derive(Debug, thiserror::Error)]
pub enum FulfillmentCostError {
    #[error("shipment SKU cost is unknown")]
    Unavailable,
    #[error("shipment SKU cost is negative")]
    Invalid,
    #[error("shipment total cost exceeds the supported range")]
    Overflow,
    #[error("shipment already has incompatible cost facts")]
    Conflict,
}

const REFERENCE: &str = "SHIPMENT_ITEM";

pub(super) async fn insert_shipment_costs(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order_id: Uuid,
    shipment_id: Uuid,
    items: &[OmsShipmentItem],
) -> anyhow::Result<()> {
    if items.is_empty() {
        return Err(InventoryError::ReservationMismatch.into());
    }
    #[derive(sqlx::FromRow)]
    struct CostRow {
        sku_id: Uuid,
        cost_cent: Option<i64>,
    }
    // Called after the frozen root and physical inventory guards. Keep catalog
    // cost writers outside the stock transaction until these facts commit.
    let sku_ids: Vec<_> = items.iter().map(|item| item.sku_id).collect();
    let rows: Vec<CostRow> = sqlx::query_as(
        "select sku_id,cost_cent from skus where tenant_id=$1 and sku_id=any($2) order by sku_id for share",
    ).bind(tenant.0).bind(&sku_ids).fetch_all(&mut **tx).await?;
    let costs: HashMap<_, _> = rows
        .into_iter()
        .map(|row| (row.sku_id, row.cost_cent))
        .collect();
    let mut prepared = Vec::with_capacity(items.len());
    for item in items {
        let unit = costs
            .get(&item.sku_id)
            .copied()
            .flatten()
            .ok_or(FulfillmentCostError::Unavailable)?;
        if unit < 0 {
            return Err(FulfillmentCostError::Invalid.into());
        }
        if item.qty <= 0 {
            return Err(InventoryError::ReservationMismatch.into());
        }
        let total = unit
            .checked_mul(item.qty)
            .ok_or(FulfillmentCostError::Overflow)?;
        prepared.push((item, unit, total));
    }
    let references: Vec<_> = items
        .iter()
        .map(|item| item.shipment_item_id.to_string())
        .collect();
    let exists: bool = sqlx::query_scalar("select exists(select 1 from order_cogs where tenant_id=$1 and ref_type=$2 and ref_id=any($3))")
        .bind(tenant.0).bind(REFERENCE).bind(&references).fetch_one(&mut **tx).await?;
    // A committed replay was handled by the SHIPPED header. No existing COGS
    // row can authorize a new shipment transition or be silently skipped.
    if exists {
        return Err(FulfillmentCostError::Conflict.into());
    }
    let mut query = QueryBuilder::<Postgres>::new(
        "insert into order_cogs(tenant_id,order_id,shipment_id,outbound_doc_id,order_item_id,sku_id,qty,unit_cost_cent,total_cost_cent,ref_type,ref_id) ",
    );
    query.push_values(prepared.iter(), |mut row, (item, unit, total)| {
        row.push_bind(tenant.0)
            .push_bind(order_id)
            .push_bind(shipment_id)
            .push_bind(Option::<Uuid>::None)
            .push_bind(item.order_item_id)
            .push_bind(item.sku_id)
            .push_bind(item.qty)
            .push_bind(*unit)
            .push_bind(*total)
            .push_bind(REFERENCE)
            .push_bind(item.shipment_item_id.to_string());
    });
    let inserted = query.build().execute(&mut **tx).await?;
    if inserted.rows_affected() != u64::try_from(items.len())? {
        return Err(FulfillmentCostError::Conflict.into());
    }
    Ok(())
}
