//! Immutable root requests and applied SKU components, separate from manufacture BOMs.
use anyhow::Result;
use erp_types::TenantId;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::inventory::{
    InventoryError, InventoryRepo, InventoryReservationInput, ReservationExpansion,
    ReservationItem, component_order_item_id, validate_reservation_input,
};

pub(crate) const MAX_ORDER_RESERVATION_COMPONENTS: usize = 1000;

pub(crate) struct FrozenReservationItems {
    pub(crate) source_id: Option<Uuid>,
    pub(crate) items: Vec<ReservationItem>,
}

#[derive(sqlx::FromRow)]
struct RootRow {
    source_id: Uuid,
    warehouse_id: Uuid,
    sku_id: Uuid,
    order_id: Option<Uuid>,
    original_qty: i64,
    component_count: i32,
    definition_kind: String,
}

#[derive(sqlx::FromRow)]
struct DirectRow {
    warehouse_id: Uuid,
    sku_id: Uuid,
    order_id: Option<Uuid>,
    qty: i64,
    reservation_source_id: Option<Uuid>,
}

impl InventoryRepo<'_> {
    pub(crate) async fn resolve_order_reservation_items(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: &InventoryReservationInput,
        creating: bool,
    ) -> Result<FrozenReservationItems> {
        self.resolve_prepared_order_reservation_items(tx, input, creating, None)
            .await
    }

    pub(crate) async fn frozen_order_expansion(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
        order_item_id: Uuid,
        sku_id: Uuid,
        qty: i64,
        order_id: Option<Uuid>,
    ) -> Result<Option<(Uuid, ReservationExpansion)>> {
        let root:Option<RootRow>=sqlx::query_as("select source_id,warehouse_id,sku_id,order_id,original_qty,
            component_count,definition_kind from inventory_order_reservation_sources where tenant_id=$1 and root_order_item_id=$2")
            .bind(tenant.0).bind(order_item_id).fetch_optional(&mut **tx).await?;
        let Some(root) = root else {
            let direct: Option<DirectRow> = sqlx::query_as(
                "select warehouse_id,sku_id,order_id,qty,reservation_source_id
                from inventory_reservations where tenant_id=$1 and order_item_id=$2",
            )
            .bind(tenant.0)
            .bind(order_item_id)
            .fetch_optional(&mut **tx)
            .await?;
            if let Some(direct) = direct {
                if direct.sku_id != sku_id
                    || direct.qty != qty
                    || direct.order_id != order_id
                    || direct.reservation_source_id.is_some()
                {
                    return Err(InventoryError::ReservationMismatch.into());
                }
                return Ok(Some((
                    direct.warehouse_id,
                    ReservationExpansion {
                        definition_kind: "DIRECT",
                        items: vec![ReservationItem {
                            sku_id,
                            order_item_id,
                            qty,
                            bom_version: None,
                        }],
                    },
                )));
            }
            refuse_unverified_legacy_children(tx, tenant, order_item_id).await?;
            return Ok(None);
        };
        if root.sku_id != sku_id || root.original_qty != qty || root.order_id != order_id {
            return Err(InventoryError::ReservationMismatch.into());
        }
        let items = read_components(tx, tenant, &root).await?;
        let definition_kind = match root.definition_kind.as_str() {
            "DIRECT" => "DIRECT",
            "ORDER_COMPONENTS" => "ORDER_COMPONENTS",
            "SKU_BOM" => "SKU_BOM",
            _ => return Err(InventoryError::ReservationSourceUnverified.into()),
        };
        Ok(Some((
            root.warehouse_id,
            ReservationExpansion {
                definition_kind,
                items,
            },
        )))
    }

    pub(crate) async fn resolve_prepared_order_reservation_items(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: &InventoryReservationInput,
        creating: bool,
        prepared: Option<ReservationExpansion>,
    ) -> Result<FrozenReservationItems> {
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(input.tenant_id.to_string())
            .bind(format!("order-reservation-root:{}", input.order_item_id))
            .execute(&mut **tx)
            .await?;
        let root: Option<RootRow> = sqlx::query_as(
            "select source_id,warehouse_id,sku_id,order_id,original_qty,component_count,definition_kind
             from inventory_order_reservation_sources where tenant_id=$1 and root_order_item_id=$2",
        )
        .bind(input.tenant_id.0)
        .bind(input.order_item_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(root) = root {
            if root.warehouse_id != input.warehouse_id
                || root.sku_id != input.sku_id
                || root.original_qty != input.qty
                || (creating && root.order_id != input.order_id)
            {
                return Err(InventoryError::ReservationMismatch.into());
            }
            let items = read_components(tx, input.tenant_id, &root).await?;
            return Ok(FrozenReservationItems {
                source_id: Some(root.source_id),
                items,
            });
        }

        // An old direct row provides its physical key and original whole qty.
        // Keep it unmodified; catalog reclassification cannot re-expand it.
        let direct: Option<DirectRow> = sqlx::query_as(
            "select warehouse_id,sku_id,order_id,qty,reservation_source_id from inventory_reservations
             where tenant_id=$1 and order_item_id=$2",
        )
        .bind(input.tenant_id.0)
        .bind(input.order_item_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(direct) = direct {
            if direct.warehouse_id != input.warehouse_id
                || direct.sku_id != input.sku_id
                || direct.qty != input.qty
                || direct.reservation_source_id.is_some()
                || (creating && direct.order_id != input.order_id)
            {
                return Err(InventoryError::ReservationMismatch.into());
            }
            return Ok(FrozenReservationItems {
                source_id: None,
                items: vec![ReservationItem {
                    sku_id: direct.sku_id,
                    order_item_id: input.order_item_id,
                    qty: direct.qty,
                    bom_version: None,
                }],
            });
        }
        refuse_unverified_legacy_children(tx, input.tenant_id, input.order_item_id).await?;
        if !creating {
            return Err(InventoryError::ReservationNotFound.into());
        }

        let mut expansion = if let Some(prepared) = prepared {
            prepared
        } else {
            self.expand_order_item(
                tx,
                input.tenant_id,
                input.sku_id,
                input.order_item_id,
                input.qty,
                input.order_id,
            )
            .await?
        };
        if expansion.items.is_empty() || expansion.items.len() > MAX_ORDER_RESERVATION_COMPONENTS {
            return Err(InventoryError::ReservationComponentsLimit.into());
        }
        expansion
            .items
            .sort_by_key(|item| (item.sku_id, item.order_item_id));
        for item in &expansion.items {
            validate_reservation_input(
                input.tenant_id,
                input.warehouse_id,
                item.sku_id,
                item.order_item_id,
                item.qty,
            )?;
        }
        let source_id: Uuid = sqlx::query_scalar(
            "insert into inventory_order_reservation_sources(
             tenant_id,root_order_item_id,warehouse_id,sku_id,order_id,original_qty,component_count,definition_kind)
             values($1,$2,$3,$4,$5,$6,$7,$8) returning source_id",
        )
        .bind(input.tenant_id.0)
        .bind(input.order_item_id)
        .bind(input.warehouse_id)
        .bind(input.sku_id)
        .bind(input.order_id)
        .bind(input.qty)
        .bind(i32::try_from(expansion.items.len())?)
        .bind(expansion.definition_kind)
        .fetch_one(&mut **tx)
        .await?;
        for item in &expansion.items {
            sqlx::query(
                "insert into inventory_order_reservation_components(
                 tenant_id,source_id,order_item_id,sku_id,qty,bom_version) values($1,$2,$3,$4,$5,$6)",
            )
            .bind(input.tenant_id.0)
            .bind(source_id)
            .bind(item.order_item_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .bind(item.bom_version)
            .execute(&mut **tx)
            .await?;
        }
        Ok(FrozenReservationItems {
            source_id: Some(source_id),
            items: expansion.items,
        })
    }
}

async fn read_components(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    root: &RootRow,
) -> Result<Vec<ReservationItem>> {
    let items: Vec<ReservationItem> = sqlx::query_as(
        "select sku_id,order_item_id,qty,bom_version
        from inventory_order_reservation_components where tenant_id=$1 and source_id=$2
        order by sku_id,order_item_id limit 1001",
    )
    .bind(tenant.0)
    .bind(root.source_id)
    .fetch_all(&mut **tx)
    .await?;
    if usize::try_from(root.component_count).ok() != Some(items.len())
        || items.is_empty()
        || items.len() > MAX_ORDER_RESERVATION_COMPONENTS
    {
        return Err(InventoryError::ReservationSourceUnverified.into());
    }
    Ok(items)
}

async fn refuse_unverified_legacy_children(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    root: Uuid,
) -> Result<()> {
    // A changed/deleted BOM cannot reveal all old components. Scan only the
    // indexed unmapped legacy subset, with bounded keyset pages, using each
    // stored SKU to reproduce its original UUIDv5 child identity. Never guess
    // the old parent qty/definition, attach it, or silently reserve it again.
    let mut cursor = Uuid::nil();
    loop {
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
            "select order_item_id,sku_id from inventory_reservations
             where tenant_id=$1 and reservation_source_id is null and order_item_id>$2
             order by order_item_id limit 500",
        )
        .bind(tenant.0)
        .bind(cursor)
        .fetch_all(&mut **tx)
        .await?;
        if rows
            .iter()
            .any(|(id, sku)| component_order_item_id(root, *sku) == *id)
        {
            return Err(InventoryError::ReservationSourceUnverified.into());
        }
        let Some((last, _)) = rows.last() else {
            return Ok(());
        };
        cursor = *last;
        if rows.len() < 500 {
            return Ok(());
        }
    }
}
