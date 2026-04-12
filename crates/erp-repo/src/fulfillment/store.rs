use super::models::*;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{OrderId, Platform, TenantId};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use crate::inventory::{InventoryDelta, InventoryService};

const ORDER_COGS_REF_SHIPMENT_ITEM: &str = "SHIPMENT_ITEM";
const SHIPMENT_STATUS_VERSION_CREATED: i32 = 10;
const SHIPMENT_STATUS_VERSION_SHIPPED: i32 = 30;

pub struct FulfillmentStore<'a> {
    pool: &'a PgPool,
}

impl<'a> FulfillmentStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
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
        allocate_order(self.pool, tenant_id, order_id, carrier_code).await
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
        )
        .await
    }

    pub async fn release_order_reservations(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<ReleaseResult> {
        release_order_reservations(self.pool, tenant_id, order_id, true).await
    }

    pub async fn release_order_reservations_keep_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<ReleaseResult> {
        release_order_reservations(self.pool, tenant_id, order_id, false).await
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
}

#[derive(Debug, Clone)]
struct ReservationGroup {
    order_id: uuid::Uuid,
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
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(shipment_id)
    .fetch_all(&mut **tx)
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
    )
    .await
}

async fn create_resend_shipments(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    carrier_code: Option<String>,
    origin_shipment_id: Option<uuid::Uuid>,
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
    )
    .await
}

async fn create_exchange_shipments(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    carrier_code: Option<String>,
    origin_shipment_id: Option<uuid::Uuid>,
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
    )
    .await
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
) -> anyhow::Result<AllocationResult> {
    if reuse_existing {
        let existing = list_oms_shipments(
            pool,
            tenant_id,
            OmsShipmentListFilters {
                order_id: Some(uuid::Uuid::from(order_id)),
                status: None,
                warehouse_id: None,
                shipment_types: vec![shipment_type.to_string()],
                limit: 200,
                offset: 0,
            },
        )
        .await?;
        if !existing.items.is_empty() {
            return Ok(AllocationResult {
                shipments: existing.items,
                shortages: vec![],
            });
        }
    }

    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

    let order = fetch_order_header(&mut tx, tenant_id, order_id).await?;
    let items = fetch_order_items(&mut tx, tenant_id, order_id).await?;
    if items.is_empty() {
        return Err(anyhow!("order items missing"));
    }

    let reservation_plan =
        build_reservation_plan(&mut tx, tenant_id, order_id, &items, resend_key).await?;
    if reservation_plan.is_empty() {
        return Err(anyhow!("order items missing sku mappings"));
    }

    let rules = fetch_warehouse_rules(&mut tx, tenant_id).await?;
    let candidates =
        resolve_candidates(&mut tx, tenant_id, &rules, &order, carrier_code.as_deref()).await?;

    let mut shortages = Vec::new();
    let mut allocations = Vec::new();

    for item in reservation_plan {
        let mut allocated = None;
        for (idx, candidate) in candidates.iter().enumerate() {
            let savepoint = format!("sp_alloc_{idx}");
            sqlx::query(&format!("savepoint {savepoint}"))
                .execute(&mut *tx)
                .await?;
            match reserve_item_group(pool, &mut tx, tenant_id, candidate.warehouse_id, &item).await
            {
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
            if shipment_type == SHIPMENT_TYPE_OMS {
                mark_order_shortage(&mut tx, tenant_id, order_id).await?;
            }
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
    tx.commit().await?;

    Ok(AllocationResult {
        shipments: shipment_rows,
        shortages,
    })
}

async fn ship_oms_shipment(
    pool: &PgPool,
    tenant_id: TenantId,
    shipment_id: uuid::Uuid,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    shipped_at: DateTime<Utc>,
    weight_gram: Option<i32>,
) -> anyhow::Result<(OmsShipment, bool)> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
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

    if shipment.status != SHIPMENT_STATUS_SHIPPED {
        shipment.carrier_code = carrier_code.clone().or(shipment.carrier_code);
        shipment.tracking_no = tracking_no.clone().or(shipment.tracking_no);
        shipment.weight_gram = weight_gram.or(shipment.weight_gram);
        shipment.shipped_at = Some(shipped_at);
        shipment.updated_at = shipped_at;
        shipment.status = SHIPMENT_STATUS_SHIPPED.to_string();
        shipped_now = true;

        sqlx::query(
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
        .execute(&mut *tx)
        .await?;

        let shipment_items = list_oms_shipment_items_tx(&mut tx, tenant_id, shipment_id).await?;
        let sku_ids: Vec<uuid::Uuid> = shipment_items.iter().map(|item| item.sku_id).collect();
        if !sku_ids.is_empty() {
            #[derive(sqlx::FromRow)]
            struct CostRow {
                sku_id: uuid::Uuid,
                cost_cent: Option<i64>,
            }

            let cost_rows: Vec<CostRow> = sqlx::query_as(
                r#"
                select sku_id, cost_cent
                from skus
                where tenant_id = $1 and sku_id = any($2)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(&sku_ids)
            .fetch_all(&mut *tx)
            .await?;

            let cost_map: HashMap<uuid::Uuid, i64> = cost_rows
                .into_iter()
                .map(|row| (row.sku_id, row.cost_cent.unwrap_or(0)))
                .collect();

            let mut qb = QueryBuilder::<Postgres>::new(
                r#"
                insert into order_cogs(
                    tenant_id, order_id, shipment_id, outbound_doc_id, order_item_id,
                    sku_id, qty, unit_cost_cent, total_cost_cent, ref_type, ref_id
                )
                "#,
            );
            qb.push_values(shipment_items.iter(), |mut b, item| {
                let unit_cost = *cost_map.get(&item.sku_id).unwrap_or(&0);
                let total_cost = unit_cost.saturating_mul(item.qty);
                b.push_bind(uuid::Uuid::from(tenant_id))
                    .push_bind(shipment.order_id)
                    .push_bind(shipment.shipment_id)
                    .push_bind(Option::<uuid::Uuid>::None)
                    .push_bind(item.order_item_id)
                    .push_bind(item.sku_id)
                    .push_bind(item.qty)
                    .push_bind(unit_cost)
                    .push_bind(total_cost)
                    .push_bind(ORDER_COGS_REF_SHIPMENT_ITEM)
                    .push_bind(item.shipment_item_id.to_string());
            });
            qb.push(" on conflict (tenant_id, ref_type, ref_id, sku_id) do nothing");
            qb.build().execute(&mut *tx).await?;
        }
        consume_reservations(
            pool,
            &mut tx,
            tenant_id,
            shipment.warehouse_id,
            shipment_items,
        )
        .await?;

        if shipment.shipment_type == SHIPMENT_TYPE_OMS {
            update_order_status_for_shipments(&mut tx, tenant_id, shipment.order_id).await?;
        }
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
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    order_id: OrderId,
    items: &[OrderItemRow],
    resend_key: Option<uuid::Uuid>,
) -> anyhow::Result<Vec<ReservationGroup>> {
    let mut sku_ids = HashSet::new();
    let mut parsed_items = Vec::new();
    let mut base_item_ids = Vec::new();

    for item in items {
        let Some(raw) = &item.sku_id else {
            continue;
        };
        let Ok(sku_id) = uuid::Uuid::parse_str(raw) else {
            continue;
        };
        sku_ids.insert(sku_id);
        let base_order_item_id = order_item_uuid(order_id, &item.line_id);
        parsed_items.push((item.line_id.clone(), sku_id, item.qty, base_order_item_id));
        base_item_ids.push(base_order_item_id);
    }

    if sku_ids.is_empty() {
        return Ok(vec![]);
    }

    let sku_id_list: Vec<uuid::Uuid> = sku_ids.iter().copied().collect();

    #[derive(sqlx::FromRow)]
    struct SkuRow {
        sku_id: uuid::Uuid,
        sku_kind: Option<String>,
        weight_gram: Option<i32>,
    }

    let sku_rows: Vec<SkuRow> = sqlx::query_as(
        r#"
        select sku_id, sku_kind, weight_gram
        from skus
        where tenant_id = $1 and sku_id = any($2)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_id_list)
    .fetch_all(&mut **tx)
    .await?;

    let mut sku_info = HashMap::new();
    for row in sku_rows {
        sku_info.insert(
            row.sku_id,
            (row.sku_kind.unwrap_or_default(), row.weight_gram),
        );
    }

    let kit_sku_ids: Vec<uuid::Uuid> = sku_info
        .iter()
        .filter_map(|(sku_id, (kind, _))| {
            if kind.eq_ignore_ascii_case("KIT") {
                Some(*sku_id)
            } else {
                None
            }
        })
        .collect();

    let mut kit_components: HashMap<uuid::Uuid, Vec<(uuid::Uuid, i64)>> = HashMap::new();
    if !kit_sku_ids.is_empty() {
        #[derive(sqlx::FromRow)]
        struct BomRow {
            kit_sku_id: uuid::Uuid,
            component_sku_id: uuid::Uuid,
            qty: i64,
        }

        let rows: Vec<BomRow> = sqlx::query_as(
            r#"
            select bundle_sku_id as kit_sku_id, component_sku_id, qty
            from sku_boms
            where tenant_id = $1 and bundle_sku_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(kit_sku_ids)
        .fetch_all(&mut **tx)
        .await?;

        for row in rows {
            kit_components
                .entry(row.kit_sku_id)
                .or_default()
                .push((row.component_sku_id, row.qty));
        }
    }

    let mut component_map: HashMap<uuid::Uuid, Vec<(uuid::Uuid, i64)>> = HashMap::new();
    if !base_item_ids.is_empty() {
        #[derive(sqlx::FromRow)]
        struct ComponentRow {
            order_item_id: uuid::Uuid,
            component_sku_id: uuid::Uuid,
            component_qty: i64,
        }

        let rows: Vec<ComponentRow> = sqlx::query_as(
            r#"
            select order_item_id, component_sku_id, component_qty
            from order_item_components
            where tenant_id = $1
              and order_id = $2
              and order_item_id = any($3)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(&base_item_ids)
        .fetch_all(&mut **tx)
        .await?;

        for row in rows {
            component_map
                .entry(row.order_item_id)
                .or_default()
                .push((row.component_sku_id, row.component_qty));
        }
    }

    let mut groups = Vec::new();
    for (_line_id, sku_id, qty, base_order_item_id) in parsed_items {
        let (sku_kind, weight_gram) = sku_info
            .get(&sku_id)
            .cloned()
            .unwrap_or_else(|| ("".to_string(), None));
        let order_item_id = match resend_key {
            Some(key) => resend_order_item_id(base_order_item_id, key),
            None => base_order_item_id,
        };
        let mut items = Vec::new();
        let special_split = is_special_sku(&sku_kind);

        if let Some(components) = component_map.get(&base_order_item_id) {
            for (component_sku_id, component_qty) in components {
                items.push(ReservationItem {
                    order_item_id: component_order_item_id(order_item_id, *component_sku_id),
                    sku_id: *component_sku_id,
                    qty: *component_qty,
                    special_split,
                    weight_gram,
                });
            }
        } else if sku_kind.eq_ignore_ascii_case("KIT") {
            let components = kit_components
                .get(&sku_id)
                .ok_or_else(|| anyhow!("kit components missing for sku {sku_id}"))?;
            for (component_sku_id, component_qty) in components {
                let total_qty = qty
                    .checked_mul(*component_qty)
                    .ok_or_else(|| anyhow!("qty overflow"))?;
                items.push(ReservationItem {
                    order_item_id: component_order_item_id(order_item_id, *component_sku_id),
                    sku_id: *component_sku_id,
                    qty: total_qty,
                    special_split,
                    weight_gram,
                });
            }
        } else {
            items.push(ReservationItem {
                order_item_id,
                sku_id,
                qty,
                special_split,
                weight_gram,
            });
        }

        groups.push(ReservationGroup {
            order_id: uuid::Uuid::from(order_id),
            items,
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
    if let Some(province) = &rule.province {
        if !address_lower.contains(&province.to_lowercase()) {
            return false;
        }
    }
    if let Some(city) = &rule.city {
        if !address_lower.contains(&city.to_lowercase()) {
            return false;
        }
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
    let service = InventoryService::new(pool);
    let mut allocations = Vec::new();
    for item in &group.items {
        let inserted: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            insert into inventory_reservations(
                tenant_id, warehouse_id, sku_id, order_id, order_item_id, qty, status
            ) values ($1,$2,$3,$4,$5,$6,'RESERVED')
            on conflict (tenant_id, order_item_id) do nothing
            returning reservation_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(item.sku_id)
        .bind(group.order_id)
        .bind(item.order_item_id)
        .bind(item.qty)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| ReservationFailure::Error(anyhow!(e)))?;

        if inserted.is_none() {
            #[derive(sqlx::FromRow)]
            struct ReservationRow {
                warehouse_id: uuid::Uuid,
                sku_id: uuid::Uuid,
                qty: i64,
                status: String,
            }

            let reservation: ReservationRow = sqlx::query_as(
                r#"
                select warehouse_id, sku_id, qty, status
                from inventory_reservations
                where tenant_id = $1 and order_item_id = $2
                for update
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(item.order_item_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| ReservationFailure::Error(anyhow!(e)))?;

            if reservation.status != "RESERVED" {
                return Err(ReservationFailure::Error(anyhow!("reservation not active")));
            }
            if reservation.warehouse_id != warehouse_id
                || reservation.sku_id != item.sku_id
                || reservation.qty != item.qty
            {
                return Err(ReservationFailure::Error(anyhow!("reservation mismatch")));
            }
        } else {
            #[derive(sqlx::FromRow)]
            struct BalanceRow {
                on_hand: i64,
                reserved: i64,
                frozen: i64,
            }

            let balance: Option<BalanceRow> = sqlx::query_as(
                r#"
                select on_hand, reserved, frozen
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .bind(item.sku_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| ReservationFailure::Error(anyhow!(e)))?;

            let Some(balance) = balance else {
                return Err(ReservationFailure::Insufficient {
                    shortage: AllocationShortage {
                        order_item_id: item.order_item_id,
                        sku_id: item.sku_id,
                        requested: item.qty,
                        available: 0,
                    },
                });
            };
            let available = balance.on_hand - balance.reserved - balance.frozen;
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

            service
                .apply_with_tx(
                    tx,
                    InventoryDelta {
                        tenant_id,
                        warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: 0,
                        delta_reserved: item.qty,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: "RESERVATION_CREATE".to_string(),
                        ref_type: Some("ORDER_ITEM".to_string()),
                        ref_id: Some(item.order_item_id.to_string()),
                        occurred_at: Utc::now(),
                    },
                )
                .await
                .map_err(|e| ReservationFailure::Error(anyhow!(e)))?;
        }

        let split_key = if item.special_split {
            format!("SPECIAL-{}", item.sku_id)
        } else {
            "NORMAL".to_string()
        };
        allocations.push(AllocationItem {
            order_item_id: item.order_item_id,
            sku_id: item.sku_id,
            qty: item.qty,
            warehouse_id,
            split_key,
            weight_gram: item.weight_gram,
        });
    }

    Ok(allocations)
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
    let mut index = 1;

    for ((warehouse_id, _split_key), items) in grouped {
        let shipment_id = uuid::Uuid::new_v4();
        let prefix = if shipment_type == SHIPMENT_TYPE_RESEND {
            "RSHP"
        } else if shipment_type == SHIPMENT_TYPE_EXCHANGE {
            "XCHG"
        } else {
            "SHP"
        };
        let shipment_no = Some(format!("{}-{}-{}", prefix, order.order_id, index));
        index += 1;

        let total_weight = items
            .iter()
            .filter_map(|item| item.weight_gram.map(|w| w as i64 * item.qty))
            .sum::<i64>();
        let weight_gram = if total_weight > 0 {
            Some(total_weight.min(i32::MAX as i64) as i32)
        } else {
            None
        };

        sqlx::query(
            r#"
            insert into shipments(
                shipment_id, tenant_id, shipment_type, shipment_no, origin_shipment_id,
                shop_id, order_id, platform, platform_order_id,
                warehouse_id, status, shipment_status, status_version, carrier_code, tracking_no,
                weight_gram, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,now(),now())
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
        .execute(&mut **tx)
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
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
    warehouse_id: uuid::Uuid,
    items: Vec<OmsShipmentItem>,
) -> anyhow::Result<()> {
    let service = InventoryService::new(pool);
    for item in items {
        #[derive(sqlx::FromRow)]
        struct ReservationRow {
            sku_id: uuid::Uuid,
            qty: i64,
            status: String,
        }

        let reservation: ReservationRow = sqlx::query_as(
            r#"
            select sku_id, qty, status
            from inventory_reservations
            where tenant_id = $1 and order_item_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(item.order_item_id)
        .fetch_one(&mut **tx)
        .await?;

        if reservation.status != "RESERVED" && reservation.status != "CONSUMED" {
            return Err(anyhow!("reservation not active"));
        }
        if reservation.sku_id != item.sku_id || reservation.qty != item.qty {
            return Err(anyhow!("reservation mismatch"));
        }

        if reservation.status != "CONSUMED" {
            sqlx::query(
                r#"
                update inventory_reservations
                set status = 'CONSUMED', updated_at = now()
                where tenant_id = $1 and order_item_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(item.order_item_id)
            .execute(&mut **tx)
            .await?;

            #[derive(sqlx::FromRow)]
            struct BalanceRow {
                on_hand: i64,
                reserved: i64,
            }

            let balance: BalanceRow = sqlx::query_as(
                r#"
                select on_hand, reserved
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .bind(item.sku_id)
            .fetch_one(&mut **tx)
            .await?;

            if balance.reserved < item.qty || balance.on_hand < item.qty {
                return Err(anyhow!("inventory balance mismatch"));
            }

            service
                .apply_with_tx(
                    tx,
                    InventoryDelta {
                        tenant_id,
                        warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: -item.qty,
                        delta_reserved: -item.qty,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: "SALES_SHIP".to_string(),
                        ref_type: Some("ORDER_ITEM".to_string()),
                        ref_id: Some(item.order_item_id.to_string()),
                        occurred_at: Utc::now(),
                    },
                )
                .await?;
        }
    }
    Ok(())
}

async fn release_order_reservations(
    pool: &PgPool,
    tenant_id: TenantId,
    order_id: OrderId,
    mark_order_cancelled: bool,
) -> anyhow::Result<ReleaseResult> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    let service = InventoryService::new(pool);

    #[derive(sqlx::FromRow)]
    struct ReservationRow {
        order_item_id: uuid::Uuid,
        warehouse_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        qty: i64,
        status: String,
    }

    let reservations: Vec<ReservationRow> = sqlx::query_as(
        r#"
        select order_item_id, warehouse_id, sku_id, qty, status
        from inventory_reservations
        where tenant_id = $1 and order_id = $2
        for update
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(uuid::Uuid::from(order_id))
    .fetch_all(&mut *tx)
    .await?;

    let mut released = 0_i64;

    #[derive(sqlx::FromRow)]
    struct BalanceRow {
        reserved: i64,
    }

    for reservation in reservations {
        if reservation.status == "RELEASED" {
            continue;
        }
        if reservation.status != "RESERVED" {
            continue;
        }

        let balance: BalanceRow = sqlx::query_as(
            r#"
            select reserved
            from inventory_balances
            where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(reservation.warehouse_id)
        .bind(reservation.sku_id)
        .fetch_one(&mut *tx)
        .await?;

        if balance.reserved < reservation.qty {
            return Err(anyhow!("reservation mismatch"));
        }

        sqlx::query(
            r#"
            update inventory_balances
            set reserved = reserved - $4,
                updated_at = now()
            where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(reservation.warehouse_id)
        .bind(reservation.sku_id)
        .bind(reservation.qty)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            update inventory_reservations
            set status = 'RELEASED', updated_at = now()
            where tenant_id = $1 and order_item_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(reservation.order_item_id)
        .execute(&mut *tx)
        .await?;

        service
            .apply_with_tx(
                &mut tx,
                InventoryDelta {
                    tenant_id,
                    warehouse_id: reservation.warehouse_id,
                    sku_id: reservation.sku_id,
                    delta_on_hand: 0,
                    delta_reserved: -reservation.qty,
                    delta_in_transit: 0,
                    delta_frozen: 0,
                    reason_type: "RESERVATION_RELEASE".to_string(),
                    ref_type: Some("ORDER_ITEM".to_string()),
                    ref_id: Some(reservation.order_item_id.to_string()),
                    occurred_at: Utc::now(),
                },
            )
            .await?;

        released += 1;
    }

    if mark_order_cancelled && released > 0 {
        mark_order_released(&mut tx, tenant_id, order_id).await?;
    }
    tx.commit().await?;

    Ok(ReleaseResult { released })
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
