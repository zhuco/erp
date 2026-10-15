use super::context::FulfillmentRecord;
use super::costs::insert_shipment_costs;
use super::models::*;
use super::{FulfillmentAccessError, FulfillmentActor};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{OrderId, Platform, ShopId, TenantId};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use crate::inventory::{
    InventoryError, InventoryRepo, InventoryReservationInput, ReservationExpansion,
};
use crate::inventory_order_consumers::{
    AppliedOrderItem, MAX_APPLIED_ORDER_ITEMS, apply_order_terminal_items, lock_order_keys,
    lock_order_roots,
};
use crate::{
    operation_logs::{
        OP_ACTION_OMS_ALLOCATED, OP_ACTION_OMS_RESEND_CREATED, OP_ACTION_OMS_RESERVATION_RELEASED,
        OP_ACTION_OMS_SHIPMENT_CREATED, OP_ACTION_OMS_SHIPMENT_SHIPPED,
    },
    outbox::OutboxRepo,
    shipments::SHIPMENT_SHIPPED_EVENT,
};

const SHIPMENT_STATUS_VERSION_CREATED: i32 = 10;
const SHIPMENT_STATUS_VERSION_SHIPPED: i32 = 30;

pub struct FulfillmentStore<'a> {
    pool: &'a PgPool,
    actor: FulfillmentActor,
}

impl<'a> FulfillmentStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            pool,
            actor: FulfillmentActor::default(),
        }
    }

    pub fn with_actor(mut self, actor: FulfillmentActor) -> Self {
        self.actor = actor;
        self
    }

    pub async fn list_oms_shipments(
        &self,
        tenant_id: TenantId,
        filters: OmsShipmentListFilters,
    ) -> anyhow::Result<OmsShipmentListResult> {
        list_oms_shipments(self.pool, tenant_id, filters).await
    }

    pub async fn list_oms_shipment_items(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<OmsShipmentItem>> {
        list_oms_shipment_items(self.pool, tenant_id, shipment_id).await
    }

    pub async fn get_oms_shipment(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Option<OmsShipment>> {
        get_oms_shipment(self.pool, tenant_id, shipment_id).await
    }

    pub async fn allocate_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        carrier_code: Option<String>,
    ) -> anyhow::Result<AllocationResult> {
        allocate_order(self.pool, tenant_id, order_id, carrier_code, &self.actor).await
    }

    pub async fn create_resend_shipments(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        carrier_code: Option<String>,
        origin_shipment_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<AllocationResult> {
        create_resend_shipments(
            self.pool,
            tenant_id,
            order_id,
            carrier_code,
            origin_shipment_id,
            &self.actor,
        )
        .await
    }

    pub async fn create_exchange_shipments(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        carrier_code: Option<String>,
        origin_shipment_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<AllocationResult> {
        create_exchange_shipments(
            self.pool,
            tenant_id,
            order_id,
            carrier_code,
            origin_shipment_id,
            &self.actor,
        )
        .await
    }

    pub async fn release_order_reservations(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<ReleaseResult> {
        release_order_reservations(self.pool, tenant_id, order_id, true, &self.actor).await
    }

    pub async fn release_order_reservations_keep_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<ReleaseResult> {
        release_order_reservations(self.pool, tenant_id, order_id, false, &self.actor).await
    }

    pub async fn request_shipment_cancel(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        reason: Option<String>,
        requested_by: Option<uuid::Uuid>,
    ) -> anyhow::Result<ShipmentCancelRequest> {
        request_shipment_cancel(self.pool, tenant_id, shipment_id, reason, requested_by).await
    }

    pub async fn ship_oms_shipment(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        carrier_code: Option<String>,
        tracking_no: Option<String>,
        shipped_at: DateTime<Utc>,
        weight_gram: Option<i32>,
    ) -> anyhow::Result<(OmsShipment, bool)> {
        ship_oms_shipment(
            self.pool,
            tenant_id,
            shipment_id,
            carrier_code,
            tracking_no,
            shipped_at,
            weight_gram,
            &self.actor,
        )
        .await
    }
}

#[derive(Debug, Clone)]
struct OrderHeader {
    order_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    platform: Platform,
    platform_order_id: String,
    receiver_address: Option<String>,
}

#[derive(Debug, Clone)]
struct OrderItemRow {
    line_id: String,
    sku_id: Option<String>,
    qty: i64,
}

#[derive(Debug, Clone)]
struct ReservationItem {
    order_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    special_split: bool,
    weight_gram: Option<i32>,
    bom_version: Option<i32>,
}

#[derive(Debug, Clone)]
struct ReservationGroup {
    order_id: uuid::Uuid,
    root_order_item_id: uuid::Uuid,
    parent_sku_id: uuid::Uuid,
    original_qty: i64,
    definition_kind: &'static str,
    saved_warehouse_id: Option<uuid::Uuid>,
    items: Vec<ReservationItem>,
}

#[derive(Debug, Clone)]
struct WarehouseCandidate {
    warehouse_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
struct AllocationItem {
    order_item_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    warehouse_id: uuid::Uuid,
    split_key: String,
    weight_gram: Option<i32>,
}

#[derive(Debug)]
enum ReservationFailure {
    Insufficient { shortage: AllocationShortage },
    Error(anyhow::Error),
}

async fn list_oms_shipments(
    pool: &PgPool,
    tenant_id: TenantId,
    filters: OmsShipmentListFilters,
) -> anyhow::Result<OmsShipmentListResult> {
    let mut count_qb = QueryBuilder::<Postgres>::new(
        r#"
        select count(*)
        from shipments
        where tenant_id =
        "#,
    );
    count_qb.push_bind(uuid::Uuid::from(tenant_id));
    apply_oms_shipment_types(&mut count_qb, &filters);
    apply_oms_filters(&mut count_qb, &filters);
    let total: i64 = count_qb.build_query_scalar().fetch_one(pool).await?;

    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        select
            shipment_id,
            shipment_no,
            shipment_type,
            origin_shipment_id,
            order_id,
            shop_id,
            platform,
            platform_order_id,
            warehouse_id,
            carrier_code,
            tracking_no,
            status,
            weight_gram,
            shipped_at,
            created_at,
            updated_at
        from shipments
        where tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(tenant_id));
    apply_oms_shipment_types(&mut qb, &filters);
    apply_oms_filters(&mut qb, &filters);
    qb.push(" order by created_at desc, shipment_id desc");
    qb.push(" limit ");
    qb.push_bind(filters.limit);
    qb.push(" offset ");
    qb.push_bind(filters.offset);

    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct Row {
        shipment_id: uuid::Uuid,
        shipment_no: Option<String>,
        shipment_type: String,
        origin_shipment_id: Option<uuid::Uuid>,
        order_id: uuid::Uuid,
        shop_id: uuid::Uuid,
        platform: String,
        platform_order_id: String,
        warehouse_id: uuid::Uuid,
        carrier_code: Option<String>,
        tracking_no: Option<String>,
        status: String,
        weight_gram: Option<i32>,
        shipped_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let rows: Vec<Row> = qb.build_query_as().fetch_all(pool).await?;
    let items = rows
        .into_iter()
        .map(|row| OmsShipment {
            shipment_id: row.shipment_id,
            tenant_id,
            shipment_no: row.shipment_no,
            shipment_type: row.shipment_type,
            origin_shipment_id: row.origin_shipment_id,
            order_id: row.order_id,
            shop_id: row.shop_id,
            platform: row.platform,
            platform_order_id: row.platform_order_id,
            warehouse_id: row.warehouse_id,
            carrier_code: row.carrier_code,
            tracking_no: row.tracking_no,
            status: row.status,
            weight_gram: row.weight_gram,
            shipped_at: row.shipped_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect();

    Ok(OmsShipmentListResult { items, total })
}

async fn list_oms_shipment_items(
    pool: &PgPool,
    tenant_id: TenantId,
    shipment_id: uuid::Uuid,
) -> anyhow::Result<Vec<OmsShipmentItem>> {
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct Row {
        shipment_item_id: uuid::Uuid,
        shipment_id: uuid::Uuid,
        order_item_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        qty: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select shipment_item_id, shipment_id, order_item_id, sku_id, qty
        from shipment_items
        where tenant_id = $1 and shipment_id = $2
        order by shipment_item_id asc
        limit 4097
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OmsShipmentItem {
            shipment_item_id: row.shipment_item_id,
            shipment_id: row.shipment_id,
            order_item_id: row.order_item_id,
            sku_id: row.sku_id,
            qty: row.qty,
        })
        .collect())
}

async fn list_oms_shipment_items_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    shipment_id: uuid::Uuid,
) -> anyhow::Result<Vec<OmsShipmentItem>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        shipment_item_id: uuid::Uuid,
        shipment_id: uuid::Uuid,
        order_item_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        qty: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select shipment_item_id, shipment_id, order_item_id, sku_id, qty
        from shipment_items
        where tenant_id = $1 and shipment_id = $2
        order by shipment_item_id asc
        limit 4097
        for update
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .fetch_all(&mut **tx)
    .await?;

    if rows.len() > MAX_APPLIED_ORDER_ITEMS {
        return Err(InventoryError::ReservationComponentsLimit.into());
    }
    Ok(rows
        .into_iter()
        .map(|row| OmsShipmentItem {
            shipment_item_id: row.shipment_item_id,
            shipment_id: row.shipment_id,
            order_item_id: row.order_item_id,
            sku_id: row.sku_id,
            qty: row.qty,
        })
        .collect())
}

async fn get_oms_shipment(
    pool: &PgPool,
    tenant_id: TenantId,
    shipment_id: uuid::Uuid,
) -> anyhow::Result<Option<OmsShipment>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        shipment_id: uuid::Uuid,
        shipment_no: Option<String>,
        shipment_type: String,
        origin_shipment_id: Option<uuid::Uuid>,
        order_id: uuid::Uuid,
        shop_id: uuid::Uuid,
        platform: String,
        platform_order_id: String,
        warehouse_id: uuid::Uuid,
        carrier_code: Option<String>,
        tracking_no: Option<String>,
        status: String,
        weight_gram: Option<i32>,
        shipped_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let row: Option<Row> = sqlx::query_as(
        r#"
        select
            shipment_id,
            shipment_no,
            shipment_type,
            origin_shipment_id,
            order_id,
            shop_id,
            platform,
            platform_order_id,
            warehouse_id,
            carrier_code,
            tracking_no,
            status,
            weight_gram,
            shipped_at,
            created_at,
            updated_at
        from shipments
        where tenant_id = $1
          and shipment_id = $2
          and shipment_type = any($3)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .bind(vec![
        SHIPMENT_TYPE_OMS.to_string(),
        SHIPMENT_TYPE_EXCHANGE.to_string(),
        SHIPMENT_TYPE_RESEND.to_string(),
    ])
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| OmsShipment {
        shipment_id: row.shipment_id,
        tenant_id,
        shipment_no: row.shipment_no,
        shipment_type: row.shipment_type,
        origin_shipment_id: row.origin_shipment_id,
        order_id: row.order_id,
        shop_id: row.shop_id,
        platform: row.platform,
        platform_order_id: row.platform_order_id,
        warehouse_id: row.warehouse_id,
        carrier_code: row.carrier_code,
        tracking_no: row.tracking_no,
        status: row.status,
        weight_gram: row.weight_gram,
        shipped_at: row.shipped_at,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}

async fn allocate_order(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    carrier_code: Option<String>,
    actor: &FulfillmentActor,
) -> anyhow::Result<AllocationResult> {
    allocate_order_internal(
        pool,
        tenant_id,
        order_id,
        carrier_code,
        SHIPMENT_TYPE_OMS,
        None,
        None,
        true,
        actor,
    )
    .await
}

async fn create_resend_shipments(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    carrier_code: Option<String>,
    origin_shipment_id: Option<uuid::Uuid>,
    actor: &FulfillmentActor,
) -> anyhow::Result<AllocationResult> {
    let resend_key = uuid::Uuid::new_v4();
    allocate_order_internal(
        pool,
        tenant_id,
        order_id,
        carrier_code,
        SHIPMENT_TYPE_RESEND,
        origin_shipment_id,
        Some(resend_key),
        false,
        actor,
    )
    .await
}

async fn create_exchange_shipments(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    carrier_code: Option<String>,
    origin_shipment_id: Option<uuid::Uuid>,
    actor: &FulfillmentActor,
) -> anyhow::Result<AllocationResult> {
    let exchange_key = uuid::Uuid::new_v4();
    allocate_order_internal(
        pool,
        tenant_id,
        order_id,
        carrier_code,
        SHIPMENT_TYPE_EXCHANGE,
        origin_shipment_id,
        Some(exchange_key),
        false,
        actor,
    )
    .await
}

async fn existing_order_shipments(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: OrderId,
    shipment_type: &str,
) -> anyhow::Result<Vec<OmsShipment>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        shipment_id: uuid::Uuid,
        shipment_no: Option<String>,
        shipment_type: String,
        origin_shipment_id: Option<uuid::Uuid>,
        order_id: uuid::Uuid,
        shop_id: uuid::Uuid,
        platform: String,
        platform_order_id: String,
        warehouse_id: uuid::Uuid,
        carrier_code: Option<String>,
        tracking_no: Option<String>,
        status: String,
        weight_gram: Option<i32>,
        shipped_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "select shipment_id,shipment_no,shipment_type,origin_shipment_id,order_id,shop_id,
         platform,platform_order_id,warehouse_id,carrier_code,tracking_no,status,
         weight_gram,shipped_at,created_at,updated_at from shipments
         where tenant_id=$1 and order_id=$2 and shipment_type=$3
         order by created_at,shipment_id limit 4097",
    )
    .bind(tenant.0)
    .bind(order.0)
    .bind(shipment_type)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > MAX_APPLIED_ORDER_ITEMS {
        return Err(InventoryError::ReservationComponentsLimit.into());
    }
    Ok(rows
        .into_iter()
        .map(|row| OmsShipment {
            shipment_id: row.shipment_id,
            tenant_id: tenant,
            shipment_no: row.shipment_no,
            shipment_type: row.shipment_type,
            origin_shipment_id: row.origin_shipment_id,
            order_id: row.order_id,
            shop_id: row.shop_id,
            platform: row.platform,
            platform_order_id: row.platform_order_id,
            warehouse_id: row.warehouse_id,
            carrier_code: row.carrier_code,
            tracking_no: row.tracking_no,
            status: row.status,
            weight_gram: row.weight_gram,
            shipped_at: row.shipped_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
        .collect())
}

async fn validate_existing_allocation(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    groups: &[ReservationGroup],
    shipments: &[OmsShipment],
) -> anyhow::Result<()> {
    let mut expected = HashMap::new();
    for group in groups {
        for item in &group.items {
            if expected.insert(item.order_item_id, (group, item)).is_some()
                || expected.len() > MAX_APPLIED_ORDER_ITEMS
            {
                return Err(InventoryError::ReservationMismatch.into());
            }
        }
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        order_item_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        qty: i64,
        shipment_order: uuid::Uuid,
        source_order: Option<uuid::Uuid>,
        source_warehouse: uuid::Uuid,
        source_sku: uuid::Uuid,
        source_qty: i64,
        source_status: String,
        qty_micros: Option<i64>,
        original_qty_micros: Option<i64>,
    }
    for shipment in shipments {
        if !matches!(
            shipment.status.as_str(),
            SHIPMENT_STATUS_CREATED | SHIPMENT_STATUS_SHIPPED
        ) {
            return Err(InventoryError::ReservationNotActive.into());
        }
        let rows: Vec<Row> = sqlx::query_as(
            "select si.order_item_id,si.sku_id,si.qty,si.order_id as shipment_order,
             r.order_id as source_order,r.warehouse_id as source_warehouse,
             r.sku_id as source_sku,r.qty as source_qty,r.status as source_status,
             r.qty_micros,r.original_qty_micros from shipment_items si
             join inventory_reservations r on r.tenant_id=si.tenant_id and r.order_item_id=si.order_item_id
             where si.tenant_id=$1 and si.shipment_id=$2 order by si.order_item_id limit 4097",
        ).bind(tenant.0).bind(shipment.shipment_id).fetch_all(&mut **tx).await?;
        if rows.is_empty() || rows.len() > MAX_APPLIED_ORDER_ITEMS {
            return Err(InventoryError::ReservationMismatch.into());
        }
        for row in rows {
            let (group, item) = expected
                .remove(&row.order_item_id)
                .ok_or(InventoryError::ReservationMismatch)?;
            let micros =
                erp_domain::quantity::ScaledQty::from_legacy_whole_units(item.qty)?.as_micros();
            if row.sku_id != item.sku_id
                || row.qty != item.qty
                || row.shipment_order != group.order_id
                || shipment.order_id != group.order_id
                || row.source_order != Some(group.order_id)
                || row.source_warehouse != shipment.warehouse_id
                || group.saved_warehouse_id != Some(shipment.warehouse_id)
                || row.source_sku != item.sku_id
                || row.source_qty != item.qty
                || row.qty_micros.is_some_and(|value| value != micros)
                || row.original_qty_micros.is_some_and(|value| value != micros)
                || row.source_status
                    != if shipment.status == SHIPMENT_STATUS_SHIPPED {
                        "CONSUMED"
                    } else {
                        "RESERVED"
                    }
            {
                return Err(InventoryError::ReservationMismatch.into());
            }
        }
    }
    if !expected.is_empty() {
        return Err(InventoryError::ReservationMismatch.into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn allocate_order_internal(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    carrier_code: Option<String>,
    shipment_type: &str,
    origin_shipment_id: Option<uuid::Uuid>,
    resend_key: Option<uuid::Uuid>,
    reuse_existing: bool,
    actor: &FulfillmentActor,
) -> anyhow::Result<AllocationResult> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    lock_fulfillment_order(&mut tx, tenant_id, order_id.0).await?;
    let authority = actor.authority(&mut tx, tenant_id).await?;

    let order = fetch_order_header(&mut tx, tenant_id, order_id).await?;
    let items = fetch_order_items(&mut tx, tenant_id, order_id).await?;
    if items.is_empty() {
        return Err(anyhow!("order items missing"));
    }

    let reservation_plan =
        build_reservation_plan(pool, &mut tx, tenant_id, order_id, &items, resend_key).await?;
    if reservation_plan.is_empty() {
        return Err(anyhow!("order items missing sku mappings"));
    }

    if reuse_existing {
        let existing =
            existing_order_shipments(&mut tx, tenant_id, order_id, shipment_type).await?;
        if !existing.is_empty() {
            authority.require(existing.iter().map(|shipment| shipment.warehouse_id))?;
            validate_existing_allocation(&mut tx, tenant_id, &reservation_plan, &existing).await?;
            tx.commit().await?;
            return Ok(AllocationResult {
                shipments: existing,
                shortages: vec![],
            });
        }
    }

    let rules = fetch_warehouse_rules(&mut tx, tenant_id).await?;
    let mut candidates =
        resolve_candidates(&mut tx, tenant_id, &rules, &order, carrier_code.as_deref()).await?;
    authority.require(
        reservation_plan
            .iter()
            .filter_map(|group| group.saved_warehouse_id),
    )?;
    candidates.retain(|candidate| authority.allows(candidate.warehouse_id));
    if candidates.is_empty()
        && reservation_plan
            .iter()
            .any(|group| group.saved_warehouse_id.is_none())
    {
        return Err(FulfillmentAccessError::Forbidden.into());
    }

    // Acquire every possible physical key before any source or balance row.
    // A later group cannot invert the key order of an earlier group.
    let mut keys = Vec::new();
    for group in &reservation_plan {
        let warehouses: Vec<_> = group.saved_warehouse_id.map_or_else(
            || {
                candidates
                    .iter()
                    .map(|candidate| candidate.warehouse_id)
                    .collect()
            },
            |warehouse| vec![warehouse],
        );
        for warehouse in warehouses {
            keys.push((warehouse, group.parent_sku_id));
            keys.extend(group.items.iter().map(|item| (warehouse, item.sku_id)));
        }
    }
    lock_order_keys(&mut tx, tenant_id, &keys).await?;
    sqlx::query("savepoint sp_complete_order_reserve")
        .execute(&mut *tx)
        .await?;

    let mut shortages = Vec::new();
    let mut allocations = Vec::new();

    for item in reservation_plan {
        let mut allocated = None;
        let warehouses: Vec<_> = item.saved_warehouse_id.map_or_else(
            || {
                candidates
                    .iter()
                    .map(|candidate| candidate.warehouse_id)
                    .collect()
            },
            |warehouse| vec![warehouse],
        );
        for (idx, warehouse) in warehouses.into_iter().enumerate() {
            let savepoint = format!("sp_alloc_{idx}");
            sqlx::query(&format!("savepoint {savepoint}"))
                .execute(&mut *tx)
                .await?;
            match reserve_item_group(pool, &mut tx, tenant_id, warehouse, &item).await {
                Ok(allocation) => {
                    sqlx::query(&format!("release savepoint {savepoint}"))
                        .execute(&mut *tx)
                        .await?;
                    allocated = Some(allocation);
                    break;
                }
                Err(ReservationFailure::Insufficient { shortage }) => {
                    sqlx::query(&format!("rollback to savepoint {savepoint}"))
                        .execute(&mut *tx)
                        .await?;
                    shortages.push(shortage);
                }
                Err(ReservationFailure::Error(err)) => {
                    return Err(err);
                }
            }
        }

        if let Some(allocation) = allocated {
            allocations.push(allocation);
        } else {
            sqlx::query("rollback to savepoint sp_complete_order_reserve")
                .execute(&mut *tx)
                .await?;
            if shipment_type == SHIPMENT_TYPE_OMS {
                mark_order_shortage(&mut tx, tenant_id, order_id).await?;
            }
            record_allocation(
                actor,
                &mut tx,
                tenant_id,
                AllocationEvidence {
                    order_id,
                    shipment_type,
                    origin_shipment_id,
                    shipments: &[],
                    shortages: &shortages,
                },
            )
            .await?;
            tx.commit().await?;
            return Ok(AllocationResult {
                shipments: vec![],
                shortages,
            });
        }
    }

    let shipment_rows = create_shipments(
        &mut tx,
        tenant_id,
        &order,
        &allocations,
        shipment_type,
        origin_shipment_id,
    )
    .await?;
    if shipment_type == SHIPMENT_TYPE_OMS {
        mark_order_allocated(&mut tx, tenant_id, order_id).await?;
    }
    record_allocation(
        actor,
        &mut tx,
        tenant_id,
        AllocationEvidence {
            order_id,
            shipment_type,
            origin_shipment_id,
            shipments: &shipment_rows,
            shortages: &shortages,
        },
    )
    .await?;
    tx.commit().await?;

    Ok(AllocationResult {
        shipments: shipment_rows,
        shortages,
    })
}

struct AllocationEvidence<'a> {
    order_id: OrderId,
    shipment_type: &'a str,
    origin_shipment_id: Option<uuid::Uuid>,
    shipments: &'a [OmsShipment],
    shortages: &'a [AllocationShortage],
}

async fn record_allocation(
    actor: &FulfillmentActor,
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    evidence: AllocationEvidence<'_>,
) -> anyhow::Result<()> {
    let (action, audit_action) = match evidence.shipment_type {
        SHIPMENT_TYPE_RESEND => (OP_ACTION_OMS_RESEND_CREATED, "OMS.RESEND.CREATED"),
        SHIPMENT_TYPE_EXCHANGE => ("OMS.EXCHANGE_CREATED", "OMS.EXCHANGE.CREATED"),
        _ => (OP_ACTION_OMS_ALLOCATED, "OMS.ALLOCATED"),
    };
    let shortages: Vec<_> = evidence
        .shortages
        .iter()
        .map(|shortage| {
            serde_json::json!({
                "order_item_id": shortage.order_item_id, "sku_id": shortage.sku_id,
                "requested": shortage.requested, "available": shortage.available,
            })
        })
        .collect();
    actor.record(tx, tenant, FulfillmentRecord {
        action, audit_action, entity: "ORDER", id: evidence.order_id.to_string(),
        metadata: serde_json::json!({
            "order_id": evidence.order_id,
            "shipment_ids": evidence.shipments.iter().map(|shipment| shipment.shipment_id).collect::<Vec<_>>(),
            "origin_shipment_id": evidence.origin_shipment_id,
            "shortage_count": shortages.len(), "shortages": shortages,
        }),
    }).await?;
    for shipment in evidence.shipments {
        actor.operation(tx, tenant, OP_ACTION_OMS_SHIPMENT_CREATED, "OMS_SHIPMENT",
            shipment.shipment_id.to_string(), serde_json::json!({
                "shipment_id": shipment.shipment_id, "order_id": shipment.order_id,
                "warehouse_id": shipment.warehouse_id, "origin_shipment_id": shipment.origin_shipment_id,
            })).await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn ship_oms_shipment(
    pool: &PgPool,
    tenant_id: TenantId,
    shipment_id: uuid::Uuid,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    shipped_at: DateTime<Utc>,
    weight_gram: Option<i32>,
    actor: &FulfillmentActor,
) -> anyhow::Result<(OmsShipment, bool)> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    let locked_order: uuid::Uuid =
        sqlx::query_scalar("select order_id from shipments where tenant_id=$1 and shipment_id=$2")
            .bind(tenant_id.0)
            .bind(shipment_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(InventoryError::ReservationNotFound)?;
    lock_fulfillment_order(&mut tx, tenant_id, locked_order).await?;
    let authority = actor.authority(&mut tx, tenant_id).await?;
    let mut shipped_now = false;

    #[derive(sqlx::FromRow)]
    struct ShipmentRow {
        shipment_id: uuid::Uuid,
        shipment_no: Option<String>,
        shipment_type: String,
        origin_shipment_id: Option<uuid::Uuid>,
        order_id: uuid::Uuid,
        shop_id: uuid::Uuid,
        platform: String,
        platform_order_id: String,
        warehouse_id: uuid::Uuid,
        carrier_code: Option<String>,
        tracking_no: Option<String>,
        status: String,
        weight_gram: Option<i32>,
        shipped_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let mut shipment: ShipmentRow = sqlx::query_as(
        r#"
        select
            shipment_id,
            shipment_no,
            shipment_type,
            origin_shipment_id,
            order_id,
            shop_id,
            platform,
            platform_order_id,
            warehouse_id,
            carrier_code,
            tracking_no,
            status,
            weight_gram,
            shipped_at,
            created_at,
            updated_at
        from shipments
        where tenant_id = $1
          and shipment_id = $2
          and shipment_type = any($3)
        for update
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .bind(vec![
        SHIPMENT_TYPE_OMS.to_string(),
        SHIPMENT_TYPE_EXCHANGE.to_string(),
        SHIPMENT_TYPE_RESEND.to_string(),
    ])
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow!("shipment not found"))?;

    if shipment.order_id != locked_order {
        return Err(InventoryError::ReservationMismatch.into());
    }
    authority.require([shipment.warehouse_id])?;

    if shipment.status != SHIPMENT_STATUS_SHIPPED {
        shipment.carrier_code = carrier_code.clone().or(shipment.carrier_code);
        shipment.tracking_no = tracking_no.clone().or(shipment.tracking_no);
        shipment.weight_gram = weight_gram.or(shipment.weight_gram);
        shipment.shipped_at = Some(shipped_at);
        shipment.updated_at = shipped_at;
        shipment.status = SHIPMENT_STATUS_SHIPPED.to_string();
        shipped_now = true;

        let persisted: (Option<DateTime<Utc>>, DateTime<Utc>) = sqlx::query_as(
            r#"
            update shipments
            set status = $3,
                shipment_status = $3,
                status_version = $4,
                carrier_code = $5,
                tracking_no = $6,
                shipped_at = $7,
                weight_gram = $8,
                updated_at = now()
            where tenant_id = $1
              and shipment_id = $2
              and shipment_type = $9
              and status_version <= $4
            returning shipped_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(&shipment.status)
        .bind(SHIPMENT_STATUS_VERSION_SHIPPED)
        .bind(&shipment.carrier_code)
        .bind(&shipment.tracking_no)
        .bind(shipped_at)
        .bind(shipment.weight_gram)
        .bind(&shipment.shipment_type)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(InventoryError::ReservationMismatch)?;
        // PostgreSQL stores microseconds. Both the first response and the
        // event cite the returned database fact, just like a later replay.
        shipment.shipped_at = Some(persisted.0.ok_or(InventoryError::ReservationMismatch)?);
        shipment.updated_at = persisted.1;

        let shipment_items = list_oms_shipment_items_tx(&mut tx, tenant_id, shipment_id).await?;
        consume_reservations(
            pool,
            &mut tx,
            tenant_id,
            shipment.order_id,
            shipment.warehouse_id,
            &shipment_items,
        )
        .await?;
        insert_shipment_costs(
            &mut tx,
            tenant_id,
            shipment.order_id,
            shipment_id,
            &shipment_items,
        )
        .await?;

        if shipment.shipment_type == SHIPMENT_TYPE_OMS {
            update_order_status_for_shipments(&mut tx, tenant_id, shipment.order_id).await?;
        }
        let platform = Platform::from_str(&shipment.platform)
            .map_err(|_| InventoryError::ReservationMismatch)?;
        let event = serde_json::to_value(erp_domain::ShipmentShippedPayload {
            tenant_id,
            shipment_id: shipment.shipment_id,
            order_id: OrderId(shipment.order_id),
            shop_id: ShopId(shipment.shop_id),
            platform,
            platform_order_id: shipment.platform_order_id.clone(),
            warehouse_id: shipment.warehouse_id,
            carrier_code: shipment.carrier_code.clone(),
            tracking_no: shipment.tracking_no.clone(),
            shipped_at: shipment.shipped_at,
        })?;
        OutboxRepo::enqueue_with_tx(
            &mut *tx,
            tenant_id,
            SHIPMENT_SHIPPED_EVENT,
            &event,
            Utc::now(),
        )
        .await?;
        actor
            .record(
                &mut tx,
                tenant_id,
                FulfillmentRecord {
                    action: OP_ACTION_OMS_SHIPMENT_SHIPPED,
                    audit_action: "OMS.SHIPMENT.SHIPPED",
                    entity: "OMS_SHIPMENT",
                    id: shipment.shipment_id.to_string(),
                    metadata: event,
                },
            )
            .await?;
    }

    tx.commit().await?;

    Ok((
        OmsShipment {
            shipment_id: shipment.shipment_id,
            tenant_id,
            shipment_no: shipment.shipment_no,
            shipment_type: shipment.shipment_type,
            origin_shipment_id: shipment.origin_shipment_id,
            order_id: shipment.order_id,
            shop_id: shipment.shop_id,
            platform: shipment.platform,
            platform_order_id: shipment.platform_order_id,
            warehouse_id: shipment.warehouse_id,
            carrier_code: shipment.carrier_code,
            tracking_no: shipment.tracking_no,
            status: shipment.status,
            weight_gram: shipment.weight_gram,
            shipped_at: shipment.shipped_at,
            created_at: shipment.created_at,
            updated_at: shipment.updated_at,
        },
        shipped_now,
    ))
}

fn apply_oms_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a OmsShipmentListFilters) {
    if let Some(order_id) = filters.order_id {
        qb.push(" and order_id = ");
        qb.push_bind(order_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
}

fn apply_oms_shipment_types(qb: &mut QueryBuilder<Postgres>, filters: &OmsShipmentListFilters) {
    let shipment_types = if filters.shipment_types.is_empty() {
        vec![
            SHIPMENT_TYPE_OMS.to_string(),
            SHIPMENT_TYPE_EXCHANGE.to_string(),
            SHIPMENT_TYPE_RESEND.to_string(),
        ]
    } else {
        filters.shipment_types.clone()
    };
    qb.push(" and shipment_type = any(");
    qb.push_bind(shipment_types);
    qb.push(")");
}

async fn fetch_order_header(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<OrderHeader> {
    #[derive(sqlx::FromRow)]
    struct Row {
        order_id: uuid::Uuid,
        shop_id: uuid::Uuid,
        platform: String,
        platform_order_id: String,
        receiver_address: Option<String>,
    }

    let row: Row = sqlx::query_as(
        r#"
        select o.order_id,
               o.shop_id,
               o.platform,
               o.platform_order_id,
               o.receiver_address
        from orders o
        join order_index i
          on i.tenant_id = o.tenant_id
         and i.order_id = o.order_id
         and i.order_month = o.order_month
        where i.tenant_id = $1
          and i.order_id = $2
        for update of o
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| anyhow!("order not found"))?;

    let platform =
        Platform::from_str(&row.platform).map_err(|e| anyhow!("bad platform in db: {e}"))?;

    Ok(OrderHeader {
        order_id: row.order_id,
        shop_id: row.shop_id,
        platform,
        platform_order_id: row.platform_order_id,
        receiver_address: row.receiver_address,
    })
}

async fn fetch_order_items(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<Vec<OrderItemRow>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        line_id: String,
        sku_id: Option<String>,
        qty: i32,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select oi.line_id, oi.sku_id, oi.qty
        from order_items oi
        join order_index i
          on i.tenant_id = oi.tenant_id
         and i.order_id = oi.order_id
         and i.order_month = oi.order_month
        where i.tenant_id = $1
          and i.order_id = $2
        order by oi.line_id
        limit 1001
        for update of oi
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| OrderItemRow {
            line_id: row.line_id,
            sku_id: row.sku_id,
            qty: row.qty as i64,
        })
        .collect())
}

async fn build_reservation_plan(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: OrderId,
    items: &[OrderItemRow],
    resend_key: Option<uuid::Uuid>,
) -> anyhow::Result<Vec<ReservationGroup>> {
    if items.is_empty() || items.len() > 1000 {
        return Err(InventoryError::ReservationComponentsLimit.into());
    }
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();
    for item in items {
        let sku = uuid::Uuid::parse_str(
            item.sku_id
                .as_deref()
                .ok_or(InventoryError::ReservationMismatch)?,
        )
        .map_err(|_| InventoryError::ReservationMismatch)?;
        if sku.is_nil() || item.qty <= 0 {
            return Err(InventoryError::ReservationMismatch.into());
        }
        erp_domain::quantity::ScaledQty::from_legacy_whole_units(item.qty)?;
        let base = order_item_uuid(order_id, &item.line_id);
        let root = match resend_key {
            Some(key) => resend_order_item_id(base, key),
            None => base,
        };
        if !seen.insert(root) {
            return Err(InventoryError::ReservationMismatch.into());
        }
        parsed.push((sku, item.qty, base, root));
    }
    let roots: Vec<_> = parsed.iter().map(|(_, _, _, root)| *root).collect();
    lock_order_roots(tx, tenant_id, &roots).await?;
    let repo = InventoryRepo::new(pool);
    let mut groups = Vec::new();
    for (sku, qty, base, root) in parsed {
        let kind: Option<String> =
            sqlx::query_scalar("select sku_kind from skus where tenant_id=$1 and sku_id=$2")
                .bind(tenant_id.0)
                .bind(sku)
                .fetch_optional(&mut **tx)
                .await?;
        let kind = kind.ok_or(InventoryError::ReservationMismatch)?;
        let frozen = repo
            .frozen_order_expansion(tx, tenant_id, root, sku, qty, Some(order_id.0))
            .await?;
        let (saved_warehouse_id, mut expansion) = if let Some((warehouse, expansion)) = frozen {
            (Some(warehouse), expansion)
        } else {
            // Resend namespaces are new stock intents, but their component
            // identity comes from the original applied/order definition.
            let original = if resend_key.is_some() {
                repo.frozen_order_expansion(tx, tenant_id, base, sku, qty, Some(order_id.0))
                    .await?
                    .map(|(_, expansion)| expansion)
            } else {
                None
            };
            let expansion = if let Some(original) = original {
                original
            } else {
                repo.expand_order_item(tx, tenant_id, sku, base, qty, Some(order_id.0))
                    .await?
            };
            (None, expansion)
        };
        if expansion.items.is_empty() || expansion.items.len() > 1000 {
            return Err(InventoryError::ReservationComponentsLimit.into());
        }
        if resend_key.is_some() && saved_warehouse_id.is_none() {
            for item in &mut expansion.items {
                item.order_item_id = if expansion.definition_kind == "DIRECT" {
                    root
                } else {
                    component_order_item_id(root, item.sku_id)
                };
            }
        }
        let special_split = is_special_sku(&kind);
        let mut physical = Vec::new();
        for item in expansion.items {
            let weight: Option<i32> =
                sqlx::query_scalar("select weight_gram from skus where tenant_id=$1 and sku_id=$2")
                    .bind(tenant_id.0)
                    .bind(item.sku_id)
                    .fetch_optional(&mut **tx)
                    .await?
                    .flatten();
            physical.push(ReservationItem {
                order_item_id: item.order_item_id,
                sku_id: item.sku_id,
                qty: item.qty,
                special_split,
                weight_gram: weight,
                bom_version: item.bom_version,
            });
        }
        physical.sort_by_key(|item| (item.sku_id, item.order_item_id));
        groups.push(ReservationGroup {
            order_id: order_id.0,
            root_order_item_id: root,
            parent_sku_id: sku,
            original_qty: qty,
            definition_kind: expansion.definition_kind,
            saved_warehouse_id,
            items: physical,
        });
    }
    Ok(groups)
}

async fn fetch_warehouse_rules(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
) -> anyhow::Result<Vec<WarehouseRule>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        rule_id: uuid::Uuid,
        warehouse_id: uuid::Uuid,
        province: Option<String>,
        city: Option<String>,
        carrier_code: Option<String>,
        priority: i32,
        is_active: bool,
        updated_at: DateTime<Utc>,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select rule_id, warehouse_id, province, city, carrier_code, priority, is_active, updated_at
        from warehouse_rules
        where tenant_id = $1 and is_active = true
        order by priority desc, updated_at desc, rule_id desc
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| WarehouseRule {
            rule_id: row.rule_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            province: row.province,
            city: row.city,
            carrier_code: row.carrier_code,
            priority: row.priority,
            is_active: row.is_active,
            updated_at: row.updated_at,
        })
        .collect())
}

async fn resolve_candidates(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    rules: &[WarehouseRule],
    order: &OrderHeader,
    carrier_code: Option<&str>,
) -> anyhow::Result<Vec<WarehouseCandidate>> {
    let address = order.receiver_address.clone().unwrap_or_default();
    let mut candidates = Vec::new();

    for rule in rules {
        if !rule_matches(rule, &address, carrier_code) {
            continue;
        }
        candidates.push(WarehouseCandidate {
            warehouse_id: rule.warehouse_id,
        });
    }

    if candidates.is_empty() {
        let default: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select warehouse_id
            from warehouses
            where tenant_id = $1 and is_default = true
            order by updated_at desc, warehouse_id desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(&mut **tx)
        .await?;

        let Some(warehouse_id) = default else {
            return Err(anyhow!("default warehouse missing"));
        };
        candidates.push(WarehouseCandidate { warehouse_id });
    }

    Ok(candidates)
}

fn rule_matches(rule: &WarehouseRule, address: &str, carrier_code: Option<&str>) -> bool {
    let address_lower = address.to_lowercase();
    if let Some(province) = &rule.province
        && !address_lower.contains(&province.to_lowercase())
    {
        return false;
    }
    if let Some(city) = &rule.city
        && !address_lower.contains(&city.to_lowercase())
    {
        return false;
    }
    if let Some(rule_carrier) = &rule.carrier_code {
        let Some(input_carrier) = carrier_code else {
            return false;
        };
        if !rule_carrier.eq_ignore_ascii_case(input_carrier) {
            return false;
        }
    }
    true
}

async fn reserve_item_group(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    group: &ReservationGroup,
) -> Result<Vec<AllocationItem>, ReservationFailure> {
    let service = crate::inventory::InventoryService::new(pool);
    // All candidate physical keys are already guarded in global order. Check
    // exact available whole units; a 0.9 residual is neither zero nor one.
    for item in &group.items {
        let existing:bool=sqlx::query_scalar("select exists(select 1 from inventory_reservations where tenant_id=$1 and order_item_id=$2)")
            .bind(tenant_id.0).bind(item.order_item_id).fetch_one(&mut **tx).await
            .map_err(|e|ReservationFailure::Error(e.into()))?;
        if !existing {
            let balance = service
                .get_precise_balance_with_tx(
                    tx,
                    crate::inventory_precision::InventoryQuantityKey {
                        tenant_id,
                        warehouse_id,
                        sku_id: item.sku_id,
                    },
                )
                .await
                .map_err(ReservationFailure::Error)?;
            let available = match balance {
                Some(balance) => balance
                    .whole_projection()
                    .map_err(ReservationFailure::Error)?
                    .available(),
                None => 0,
            };
            if available < item.qty {
                return Err(ReservationFailure::Insufficient {
                    shortage: AllocationShortage {
                        order_item_id: item.order_item_id,
                        sku_id: item.sku_id,
                        requested: item.qty,
                        available,
                    },
                });
            }
        }
    }
    let prepared = ReservationExpansion {
        definition_kind: group.definition_kind,
        items: group
            .items
            .iter()
            .map(|item| crate::inventory::ReservationItem {
                sku_id: item.sku_id,
                order_item_id: item.order_item_id,
                qty: item.qty,
                bom_version: item.bom_version,
            })
            .collect(),
    };
    InventoryRepo::new(pool)
        .reserve_frozen_order_item_with_tx(
            tx,
            InventoryReservationInput {
                tenant_id,
                warehouse_id,
                sku_id: group.parent_sku_id,
                order_item_id: group.root_order_item_id,
                order_id: Some(group.order_id),
                qty: group.original_qty,
            },
            Some(prepared),
        )
        .await
        .map_err(ReservationFailure::Error)?;
    Ok(group
        .items
        .iter()
        .map(|item| AllocationItem {
            order_item_id: item.order_item_id,
            sku_id: item.sku_id,
            qty: item.qty,
            warehouse_id,
            split_key: if item.special_split {
                format!("SPECIAL-{}", item.sku_id)
            } else {
                "NORMAL".into()
            },
            weight_gram: item.weight_gram,
        })
        .collect())
}

async fn create_shipments(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order: &OrderHeader,
    allocations: &[Vec<AllocationItem>],
    shipment_type: &str,
    origin_shipment_id: Option<uuid::Uuid>,
) -> anyhow::Result<Vec<OmsShipment>> {
    let mut grouped: HashMap<(uuid::Uuid, String), Vec<AllocationItem>> = HashMap::new();
    for group in allocations {
        for item in group {
            grouped
                .entry((item.warehouse_id, item.split_key.clone()))
                .or_default()
                .push(item.clone());
        }
    }

    let mut shipments = Vec::new();
    for (index, ((warehouse_id, _split_key), items)) in (1..).zip(grouped) {
        let shipment_id = uuid::Uuid::new_v4();
        let prefix = if shipment_type == SHIPMENT_TYPE_RESEND {
            "RSHP"
        } else if shipment_type == SHIPMENT_TYPE_EXCHANGE {
            "XCHG"
        } else {
            "SHP"
        };
        let shipment_no = Some(format!("{}-{}-{}", prefix, order.order_id, index));

        let total_weight = items
            .iter()
            .filter_map(|item| item.weight_gram.map(|w| w as i64 * item.qty))
            .sum::<i64>();
        let weight_gram = if total_weight > 0 {
            Some(total_weight.min(i32::MAX as i64) as i32)
        } else {
            None
        };

        let (created_at, updated_at): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
            r#"
            insert into shipments(
                shipment_id, tenant_id, shipment_type, shipment_no, origin_shipment_id,
                shop_id, order_id, platform, platform_order_id,
                warehouse_id, status, shipment_status, status_version, carrier_code, tracking_no,
                weight_gram, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,now(),now())
            returning created_at, updated_at
            "#,
        )
        .bind(shipment_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_type)
        .bind(&shipment_no)
        .bind(origin_shipment_id)
        .bind(order.shop_id)
        .bind(order.order_id)
        .bind(order.platform.to_string())
        .bind(&order.platform_order_id)
        .bind(warehouse_id)
        .bind(SHIPMENT_STATUS_CREATED)
        .bind(SHIPMENT_STATUS_CREATED)
        .bind(SHIPMENT_STATUS_VERSION_CREATED)
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .bind(weight_gram)
        .fetch_one(&mut **tx)
        .await?;

        for item in items {
            sqlx::query(
                r#"
                insert into shipment_items(
                    tenant_id, shipment_id, order_id, order_item_id, sku_id, qty
                ) values ($1,$2,$3,$4,$5,$6)
                on conflict (tenant_id, shipment_id, order_item_id) do nothing
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(shipment_id)
            .bind(order.order_id)
            .bind(item.order_item_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .execute(&mut **tx)
            .await?;
        }

        shipments.push(OmsShipment {
            shipment_id,
            tenant_id,
            shipment_no,
            shipment_type: shipment_type.to_string(),
            origin_shipment_id,
            order_id: order.order_id,
            shop_id: order.shop_id,
            platform: order.platform.to_string(),
            platform_order_id: order.platform_order_id.clone(),
            warehouse_id,
            carrier_code: None,
            tracking_no: None,
            status: SHIPMENT_STATUS_CREATED.to_string(),
            weight_gram,
            shipped_at: None,
            created_at,
            updated_at,
        });
    }

    Ok(shipments)
}

async fn mark_order_allocated(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        update orders o
        set order_status = 'PENDING_OUTBOUND',
            fulfillment_status = 'ALLOCATED',
            inventory_shortage = false,
            exception_flags = (
                select coalesce(jsonb_agg(value), '[]'::jsonb)
                from jsonb_array_elements_text(o.exception_flags) as value
                where value <> 'INVENTORY_SHORTAGE'
            ),
            exception_status = case
                when (
                    select jsonb_array_length(
                        coalesce(jsonb_agg(value), '[]'::jsonb)
                    )
                    from jsonb_array_elements_text(o.exception_flags) as value
                    where value <> 'INVENTORY_SHORTAGE'
                ) = 0 then 'NONE'
                else o.exception_status
            end,
            allocated_at = now(),
            updated_at = now()
        from order_index i
        where o.tenant_id = i.tenant_id
          and o.order_id = i.order_id
          and o.order_month = i.order_month
          and i.tenant_id = $1
          and i.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn mark_order_shortage(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        update orders o
        set order_status = 'EXCEPTION_HOLD',
            fulfillment_status = 'NOT_ALLOCATED',
            exception_status = 'OUT_OF_STOCK',
            inventory_shortage = true,
            exception_flags = case
                when o.exception_flags ? 'INVENTORY_SHORTAGE' then o.exception_flags
                else o.exception_flags || '["INVENTORY_SHORTAGE"]'::jsonb
            end,
            updated_at = now()
        from order_index i
        where o.tenant_id = i.tenant_id
          and o.order_id = i.order_id
          and o.order_month = i.order_month
          and i.tenant_id = $1
          and i.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn mark_order_released(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: OrderId,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        update orders o
        set order_status = 'CANCELLED',
            fulfillment_status = 'NOT_ALLOCATED',
            exception_status = 'NONE',
            inventory_shortage = false,
            updated_at = now()
        from order_index i
        where o.tenant_id = i.tenant_id
          and o.order_id = i.order_id
          and o.order_month = i.order_month
          and i.tenant_id = $1
          and i.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn consume_reservations(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    items: &[OmsShipmentItem],
) -> anyhow::Result<()> {
    if items.is_empty() {
        return Err(InventoryError::ReservationMismatch.into());
    }
    let source_ids: Vec<_> = items.iter().map(|item| item.order_item_id).collect();
    let shipment_id = items[0].shipment_id;
    let count: i64 = sqlx::query_scalar(
        "select count(*) from shipment_items where tenant_id=$1 and shipment_id=$2
         and order_id=$3 and order_item_id=any($4)",
    )
    .bind(tenant_id.0)
    .bind(shipment_id)
    .bind(order_id)
    .bind(&source_ids)
    .fetch_one(&mut **tx)
    .await?;
    if usize::try_from(count)? != items.len() {
        return Err(InventoryError::ReservationMismatch.into());
    }
    let applied: Vec<_> = items
        .iter()
        .map(|item| AppliedOrderItem {
            warehouse_id,
            sku_id: item.sku_id,
            order_item_id: item.order_item_id,
            order_id,
            qty: item.qty,
        })
        .collect();
    apply_order_terminal_items(pool, tx, tenant_id, &applied, true).await?;
    Ok(())
}

async fn release_order_reservations(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    mark_order_cancelled: bool,
    actor: &FulfillmentActor,
) -> anyhow::Result<ReleaseResult> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    lock_fulfillment_order(&mut tx, tenant_id, order_id.0).await?;
    let authority = actor.authority(&mut tx, tenant_id).await?;
    #[derive(sqlx::FromRow)]
    struct Row {
        order_item_id: uuid::Uuid,
        warehouse_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        qty: i64,
        status: String,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "select order_item_id,warehouse_id,sku_id,qty,status
        from inventory_reservations where tenant_id=$1 and order_id=$2
        order by warehouse_id,sku_id,order_item_id limit 4097",
    )
    .bind(tenant_id.0)
    .bind(order_id.0)
    .fetch_all(&mut *tx)
    .await?;
    if rows.len() > MAX_APPLIED_ORDER_ITEMS {
        return Err(InventoryError::ReservationComponentsLimit.into());
    }
    authority.require(rows.iter().map(|row| row.warehouse_id))?;
    let consumed = rows.iter().any(|row| row.status == "CONSUMED");
    let items: Vec<_> = rows
        .into_iter()
        .map(|row| AppliedOrderItem {
            warehouse_id: row.warehouse_id,
            sku_id: row.sku_id,
            order_item_id: row.order_item_id,
            order_id: order_id.0,
            qty: row.qty,
        })
        .collect();
    let released = apply_order_terminal_items(pool, &mut tx, tenant_id, &items, false).await?;
    if mark_order_cancelled && released > 0 && !consumed {
        mark_order_released(&mut tx, tenant_id, order_id).await?;
    }
    if released > 0 {
        actor
            .record(
                &mut tx,
                tenant_id,
                FulfillmentRecord {
                    action: OP_ACTION_OMS_RESERVATION_RELEASED,
                    audit_action: "OMS.RESERVATION.RELEASED",
                    entity: "ORDER",
                    id: order_id.to_string(),
                    metadata: serde_json::json!({"order_id": order_id, "released": released}),
                },
            )
            .await?;
    }
    tx.commit().await?;
    Ok(ReleaseResult { released })
}

async fn lock_fulfillment_order(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: uuid::Uuid,
) -> anyhow::Result<()> {
    if tenant.0.is_nil() || order.is_nil() {
        return Err(InventoryError::ReservationMismatch.into());
    }
    sqlx::query(
        "select set_config('statement_timeout','10s',true),set_config('lock_timeout','5s',true)",
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
        .bind(tenant.to_string())
        .bind(format!("fulfillment-order:{order}"))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn request_shipment_cancel(
    pool: &PgPool,
    tenant_id: TenantId,
    shipment_id: uuid::Uuid,
    reason: Option<String>,
    requested_by: Option<uuid::Uuid>,
) -> anyhow::Result<ShipmentCancelRequest> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    #[derive(sqlx::FromRow)]
    struct ShipmentRow {
        order_id: uuid::Uuid,
    }

    let shipment: ShipmentRow = sqlx::query_as(
        r#"
        select order_id
        from shipments
        where tenant_id = $1
          and shipment_id = $2
          and shipment_type = any($3)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .bind(vec![
        SHIPMENT_TYPE_OMS.to_string(),
        SHIPMENT_TYPE_EXCHANGE.to_string(),
        SHIPMENT_TYPE_RESEND.to_string(),
    ])
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow!("shipment not found"))?;

    #[derive(sqlx::FromRow)]
    struct Row {
        cancel_request_id: uuid::Uuid,
        shipment_id: uuid::Uuid,
        order_id: uuid::Uuid,
        status: String,
        reason: Option<String>,
        requested_by: Option<uuid::Uuid>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let row: Row = sqlx::query_as(
        r#"
        insert into shipment_cancel_requests(
            tenant_id, shipment_id, order_id, reason, requested_by, status, created_at, updated_at
        ) values ($1,$2,$3,$4,$5,'PENDING',now(),now())
        on conflict (tenant_id, shipment_id)
        do update set reason = excluded.reason, updated_at = now()
        returning cancel_request_id, shipment_id, order_id, status, reason, requested_by, created_at, updated_at
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .bind(shipment.order_id)
    .bind(reason.clone())
    .bind(requested_by)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(ShipmentCancelRequest {
        cancel_request_id: row.cancel_request_id,
        tenant_id,
        shipment_id: row.shipment_id,
        order_id: row.order_id,
        status: row.status,
        reason: row.reason,
        requested_by: row.requested_by,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

async fn update_order_status_for_shipments(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: uuid::Uuid,
) -> anyhow::Result<()> {
    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct Row {
        total: i64,
        shipped: i64,
    }

    let row: Row = sqlx::query_as(
        r#"
        select
            count(*) as total,
            sum(case when status = $3 then 1 else 0 end) as shipped
        from shipments
        where tenant_id = $1
          and order_id = $2
          and shipment_type = $4
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .bind(SHIPMENT_STATUS_SHIPPED)
    .bind(SHIPMENT_TYPE_OMS)
    .fetch_one(&mut **tx)
    .await?;

    let (fulfillment_status, order_status) = if row.shipped > 0 {
        ("SHIPPED", "SHIPPED")
    } else {
        return Ok(());
    };

    sqlx::query(
        r#"
        update orders o
        set fulfillment_status = $3,
            order_status = $4,
            ship_at = case when $3 = 'SHIPPED' then now() else ship_at end,
            updated_at = now()
        from order_index i
        where o.tenant_id = i.tenant_id
          and o.order_id = i.order_id
          and o.order_month = i.order_month
          and i.tenant_id = $1
          and i.order_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(order_id)
    .bind(fulfillment_status)
    .bind(order_status)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn order_item_uuid(order_id: OrderId, line_id: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(line_id)
        .unwrap_or_else(|_| uuid::Uuid::new_v5(&uuid::Uuid::from(order_id), line_id.as_bytes()))
}

fn resend_order_item_id(order_item_id: uuid::Uuid, resend_key: uuid::Uuid) -> uuid::Uuid {
    uuid::Uuid::new_v5(&order_item_id, resend_key.as_bytes())
}

fn component_order_item_id(order_item_id: uuid::Uuid, component_sku_id: uuid::Uuid) -> uuid::Uuid {
    uuid::Uuid::new_v5(&order_item_id, component_sku_id.as_bytes())
}

fn is_special_sku(kind: &str) -> bool {
    matches!(
        kind.to_ascii_uppercase().as_str(),
        "PRESALE" | "BULKY" | "COLD_CHAIN"
    )
}
