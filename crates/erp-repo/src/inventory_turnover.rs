//! Authorized stock and gross posted shipment cover from one read-only snapshot.
//! Order demand is not a shipment. Unclassified shared stock cannot become zero sales.
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use erp_grid_core::decimal_text;
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    inventory_reads::{
        ExactAmounts, InventoryReadError, ReadScope, begin_read, require_reconciled, source_filters,
    },
    reports::{InventoryReport, InventoryReportFilters, InventoryTurnoverItem},
};

#[derive(sqlx::FromRow)]
struct Row {
    sku_id: Uuid,
    sku_code: Option<String>,
    warehouse_id: Uuid,
    hand: String,
    reserved: String,
    frozen: String,
    transit: String,
    sold: String,
    unclassified: i64,
}

pub(crate) async fn read_report(
    pool: &PgPool,
    tenant: TenantId,
    filters: InventoryReportFilters,
) -> Result<InventoryReport> {
    if (filters.end_at - filters.start_at).num_milliseconds() < 1
        || filters.warning_threshold_micros < 0
        || filters.offset < 0
        || !(1..=200).contains(&filters.limit)
    {
        return Err(anyhow!("invalid inventory report bounds"));
    }
    let scope = ReadScope {
        warehouse: filters.warehouse_id,
        warehouses: filters.warehouse_ids.as_deref(),
        sku: None,
        code: None,
        barcode: None,
        tag: None,
        search: None,
    };
    let mut tx = begin_read(pool).await?;
    let snapshot_at: DateTime<Utc> = sqlx::query_scalar("select statement_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    require_reconciled(&mut tx, tenant, &scope).await?;
    let mut count = QueryBuilder::<Postgres>::new(
        "select count(*) from inventory_balances ib where ib.tenant_id=",
    );
    count.push_bind(Uuid::from(tenant));
    source_filters(&mut count, &scope);
    let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;

    let mut qb=QueryBuilder::<Postgres>::new("with shipments as (select ib.warehouse_id,ib.sku_id,
        coalesce(sum(case when ib.reason_type='SALES_SHIP' and coalesce(ib.delta_on_hand_micros::numeric,ib.delta_on_hand::numeric*1000000)<0
        then -coalesce(ib.delta_on_hand_micros::numeric,ib.delta_on_hand::numeric*1000000) else 0 end),0)::text as sold,
        count(*) filter (where (ib.reason_type='SALES_SHIP' and coalesce(ib.delta_on_hand_micros::numeric,ib.delta_on_hand::numeric*1000000)>=0)
        or (ib.ref_type='shared_stock_ledger' and coalesce(ib.delta_on_hand_micros::numeric,ib.delta_on_hand::numeric*1000000)<>0)) as unclassified
        from inventory_ledgers ib where ib.tenant_id=");
    qb.push_bind(Uuid::from(tenant))
        .push(" and ib.occurred_at>=")
        .push_bind(filters.start_at)
        .push(" and ib.occurred_at<=")
        .push_bind(filters.end_at);
    source_filters(&mut qb, &scope);
    qb.push(
        " group by ib.warehouse_id,ib.sku_id) select ib.sku_id,s.sku_code,ib.warehouse_id,
        ib.on_hand_micros::text as hand,ib.reserved_micros::text as reserved,
        ib.frozen_micros::text as frozen,ib.in_transit_micros::text as transit,
        coalesce(shipments.sold,'0') as sold,coalesce(shipments.unclassified,0) as unclassified
        from inventory_balances ib join skus s on s.tenant_id=ib.tenant_id and s.sku_id=ib.sku_id
        left join shipments on shipments.warehouse_id=ib.warehouse_id and shipments.sku_id=ib.sku_id
        where ib.tenant_id=",
    )
    .push_bind(Uuid::from(tenant));
    source_filters(&mut qb, &scope);
    qb.push(" order by s.sku_code nulls last,ib.sku_id,ib.warehouse_id limit ")
        .push_bind(filters.limit)
        .push(" offset ")
        .push_bind(filters.offset);
    let rows: Vec<Row> = qb.build_query_as().fetch_all(&mut *tx).await?;
    let range_days = (filters.end_at - filters.start_at).num_days().max(1);
    let exact_days = (filters.end_at - filters.start_at)
        .num_milliseconds()
        .max(1) as f64
        / 86_400_000.0;
    let items = rows
        .into_iter()
        .map(|row| {
            let amounts =
                ExactAmounts::from_raw(&row.hand, &row.reserved, &row.frozen, &row.transit)?;
            let quantities = amounts.wire();
            let [on_hand, reserved, frozen, in_transit, available] = amounts.whole()?;
            let available_micros = amounts.available_micros();
            let sold = row
                .sold
                .parse::<i128>()
                .map_err(|_| InventoryReadError::ValueOverflow)?;
            if sold < 0 {
                return Err(InventoryReadError::ValueOverflow.into());
            }
            let classified = row.unclassified == 0;
            let sold_qty = if classified {
                Some(
                    i64::try_from(sold / 1_000_000)
                        .map_err(|_| InventoryReadError::ValueOverflow)?,
                )
            } else {
                None
            };
            let turnover_days = if classified && sold > 0 {
                Some(exact_days * amounts.hand as f64 / sold as f64)
            } else {
                None
            };
            Ok(InventoryTurnoverItem {
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                warehouse_id: row.warehouse_id,
                on_hand,
                reserved,
                frozen,
                in_transit,
                available,
                quantities,
                sold_qty,
                sold_quantity: classified.then(|| decimal_text(sold, 6)),
                sales_state: if classified {
                    "gross_posted_shipments"
                } else {
                    "source_classification_required"
                }
                .into(),
                turnover_days,
                warning: available_micros <= filters.warning_threshold_micros,
                out_of_stock: available_micros <= 0,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    tx.rollback().await?;
    Ok(InventoryReport {
        items,
        total,
        range_days,
        schema: 2,
        snapshot_at,
        sales_basis: "gross_posted_sales_ship".into(),
        turnover_basis: "closing_stock_cover".into(),
    })
}
