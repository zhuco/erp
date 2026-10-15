//! Canonical public stock reads. Legacy integer columns are projections only.
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use erp_grid_core::decimal_text;
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::inventory::*;

pub const MAX_READ_PAGE: i64 = 2_000;
const SCALE: i128 = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReadQuantities {
    pub on_hand: String,
    pub reserved: String,
    pub frozen: String,
    pub in_transit: String,
    pub available: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReadDelta {
    pub source: String,
    pub delta_on_hand: String,
    pub delta_reserved: String,
    pub delta_frozen: String,
    pub delta_in_transit: String,
}

#[derive(Debug, thiserror::Error)]
pub enum InventoryReadError {
    #[error("{0} inventory keys require quantity reconciliation")]
    ReconciliationRequired(i64),
    #[error("inventory read value exceeds the supported range")]
    ValueOverflow,
}

/// Amounts remain wide until after aggregation; availability is projected last.
#[derive(Debug)]
pub(crate) struct ExactAmounts {
    pub(crate) hand: i128,
    reserved: i128,
    frozen: i128,
    transit: i128,
    available: i128,
}

impl ExactAmounts {
    pub(crate) fn available_micros(&self) -> i128 {
        self.available
    }

    fn parse(row: &StockRow) -> Result<Self> {
        Self::from_raw(&row.hand, &row.reserved, &row.frozen, &row.transit)
    }

    pub(crate) fn from_raw(
        hand: &str,
        reserved: &str,
        frozen: &str,
        transit: &str,
    ) -> Result<Self> {
        let read = |s: &str| {
            s.parse::<i128>()
                .map_err(|_| InventoryReadError::ValueOverflow)
        };
        let hand = read(hand)?;
        let reserved = read(reserved)?;
        let frozen = read(frozen)?;
        let transit = read(transit)?;
        let available = hand
            .checked_sub(reserved)
            .and_then(|v| v.checked_sub(frozen))
            .ok_or(InventoryReadError::ValueOverflow)?;
        if [hand, reserved, frozen, transit, available]
            .iter()
            .any(|value| *value < 0)
        {
            return Err(InventoryReadError::ReconciliationRequired(1).into());
        }
        Ok(Self {
            hand,
            reserved,
            frozen,
            transit,
            available,
        })
    }

    pub(crate) fn whole(&self) -> Result<[i64; 5]> {
        let values = [
            self.hand,
            self.reserved,
            self.frozen,
            self.transit,
            self.available,
        ];
        let mut whole = [0; 5];
        for (output, value) in whole.iter_mut().zip(values) {
            *output =
                i64::try_from(value / SCALE).map_err(|_| InventoryReadError::ValueOverflow)?;
        }
        Ok(whole)
    }

    pub(crate) fn wire(&self) -> InventoryReadQuantities {
        InventoryReadQuantities {
            on_hand: decimal_text(self.hand, 6),
            reserved: decimal_text(self.reserved, 6),
            frozen: decimal_text(self.frozen, 6),
            in_transit: decimal_text(self.transit, 6),
            available: decimal_text(self.available, 6),
        }
    }
}

pub(crate) struct ReadScope<'a> {
    pub(crate) warehouse: Option<Uuid>,
    pub(crate) warehouses: Option<&'a [Uuid]>,
    pub(crate) sku: Option<Uuid>,
    pub(crate) code: Option<&'a str>,
    pub(crate) barcode: Option<&'a str>,
    pub(crate) tag: Option<&'a str>,
    pub(crate) search: Option<&'a str>,
}

pub(crate) fn exact_quantities(
    hand: &str,
    reserved: &str,
    frozen: &str,
    transit: &str,
) -> Result<InventoryReadQuantities> {
    Ok(ExactAmounts::from_raw(hand, reserved, frozen, transit)?.wire())
}

pub(crate) async fn begin_read(pool: &PgPool) -> Result<Transaction<'_, Postgres>> {
    let mut tx = pool.begin().await?;
    sqlx::query("set transaction isolation level repeatable read read only")
        .execute(&mut *tx)
        .await?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

pub(crate) fn valid_quantity_sql(alias: &str) -> String {
    format!(
        "{alias}.on_hand_micros is not null and {alias}.reserved_micros is not null and {alias}.frozen_micros is not null and {alias}.in_transit_micros is not null and {alias}.on_hand_micros >= 0 and {alias}.reserved_micros >= 0 and {alias}.frozen_micros >= 0 and {alias}.in_transit_micros >= 0 and {alias}.on_hand_micros::numeric - {alias}.reserved_micros::numeric - {alias}.frozen_micros::numeric >= 0"
    )
}

/// A SKU master/BOM estimate, not posted inventory accounting cost. Never
/// replace missing component cost, absent BOM, or invalid cost with zero.
pub(crate) fn estimated_unit_cost_sql() -> &'static str {
    "case when s.sku_kind = 'KIT' then (select case when count(*) = 0 or count(*) filter (where c.cost_cent is null or c.cost_cent < 0 or bom.qty <= 0) > 0 then null else sum(bom.qty::numeric * c.cost_cent::numeric) end from sku_boms bom left join skus c on c.tenant_id = bom.tenant_id and c.sku_id = bom.component_sku_id where bom.tenant_id = s.tenant_id and bom.bundle_sku_id = s.sku_id) else case when s.cost_cent >= 0 then s.cost_cent::numeric else null end end"
}

pub(crate) fn source_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, scope: &ReadScope<'a>) {
    if let Some(id) = scope.warehouse {
        qb.push(" and ib.warehouse_id = ").push_bind(id);
    }
    // Some(empty) explicitly grants no warehouse; None is the legacy all-scope policy.
    if let Some(ids) = scope.warehouses {
        qb.push(" and ib.warehouse_id = any(")
            .push_bind(ids)
            .push(")");
    }
    if let Some(id) = scope.sku {
        qb.push(" and ib.sku_id = ").push_bind(id);
    }
    if let Some(code) = scope.code {
        qb.push(" and s.sku_code = ").push_bind(code);
    }
    if let Some(barcode) = scope.barcode {
        qb.push(" and s.barcode = ").push_bind(barcode);
    }
    if let Some(tag) = scope.tag {
        qb.push(" and exists(select 1 from unnest(s.tags) tag where tag ilike ")
            .push_bind(format!("%{}%", literal_text(tag)))
            .push(" escape '!')");
    }
    if let Some(search) = scope.search.map(str::trim).filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", literal_text(search));
        qb.push(" and (coalesce(s.sku_code, '') ilike ")
            .push_bind(pattern.clone())
            .push(" escape '!' or coalesce(p.name, '') ilike ")
            .push_bind(pattern)
            .push(" escape '!')");
    }
}

fn literal_text(text: &str) -> String {
    text.replace('!', "!!")
        .replace('%', "!%")
        .replace('_', "!_")
}

/// Validate the entire non-numeric scope before applying quantity predicates or
/// pagination, including history whose physical balance is absent.
pub(crate) async fn require_reconciled(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    scope: &ReadScope<'_>,
) -> Result<()> {
    let mut qb = QueryBuilder::<Postgres>::new("with stock_keys as (");
    for (index, table) in [
        "inventory_balances",
        "inventory_ledgers",
        "mfg_qty_ledgers",
        "shared_stock_ledgers",
    ]
    .iter()
    .enumerate()
    {
        if index != 0 {
            qb.push(" union ");
        }
        qb.push("select warehouse_id, sku_id from ")
            .push(*table)
            .push(" where tenant_id = ")
            .push_bind(Uuid::from(tenant));
    }
    qb.push(") select count(*) from stock_keys ib left join inventory_balances b on b.warehouse_id = ib.warehouse_id and b.sku_id = ib.sku_id and b.tenant_id = ")
        .push_bind(Uuid::from(tenant))
        .push(" left join skus s on s.sku_id = ib.sku_id and s.tenant_id = ").push_bind(Uuid::from(tenant))
        .push(" left join warehouses w on w.warehouse_id = ib.warehouse_id and w.tenant_id = ").push_bind(Uuid::from(tenant))
        .push(" left join products p on p.tenant_id = s.tenant_id and p.product_id = s.product_id where 1=1");
    source_filters(&mut qb, scope);
    qb.push(" and not (")
        .push(valid_quantity_sql("b"))
        .push(" and s.sku_id is not null and w.warehouse_id is not null)");
    let count: i64 = qb.build_query_scalar().fetch_one(&mut **tx).await?;
    if count != 0 {
        return Err(InventoryReadError::ReconciliationRequired(count).into());
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) struct QuantityBounds {
    negative: bool,
    below: Option<i128>,
    min: Option<i128>,
    max: Option<i128>,
}
impl QuantityBounds {
    pub(crate) fn new(
        negative: bool,
        below: Option<i128>,
        min: Option<i128>,
        max: Option<i128>,
    ) -> Self {
        Self {
            negative,
            below,
            min,
            max,
        }
    }
}

pub(crate) fn quantity_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    hand: &str,
    available: &str,
    bounds: QuantityBounds,
) -> Result<()> {
    let QuantityBounds {
        negative,
        below,
        min,
        max,
    } = bounds;
    if min.zip(max).is_some_and(|(min, max)| min > max) {
        return Err(anyhow!("invalid available range"));
    }
    if negative {
        qb.push(" and ").push(hand).push(" < 0");
    }
    // API settings are converted once to exact micros before repository admission.
    if let Some(value) = below {
        qb.push(" and ")
            .push(available)
            .push(" < ")
            .push_bind(value.to_string())
            .push("::numeric");
    }
    for (operator, value) in [(" >= ", min), (" <= ", max)] {
        if let Some(value) = value {
            qb.push(" and ")
                .push(available)
                .push(operator)
                .push_bind(value.to_string())
                .push("::numeric");
        }
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct StockRow {
    warehouse_id: Option<Uuid>,
    sku_id: Uuid,
    sku_code: Option<String>,
    barcode: Option<String>,
    cost: Option<String>,
    hand: String,
    reserved: String,
    frozen: String,
    transit: String,
    updated_at: DateTime<Utc>,
}

fn cost_value(row: &StockRow) -> Result<Option<i128>> {
    row.cost
        .as_ref()
        .map(|cost| {
            cost.parse::<i128>()
                .map_err(|_| InventoryReadError::ValueOverflow.into())
        })
        .transpose()
}

fn summary_cte<'a>(tenant: TenantId, scope: &ReadScope<'a>) -> QueryBuilder<'a, Postgres> {
    let mut qb = QueryBuilder::new(
        "with summary as (select ib.sku_id, sum(ib.on_hand_micros::numeric) as hand, sum(ib.reserved_micros::numeric) as reserved, sum(ib.frozen_micros::numeric) as frozen, sum(ib.in_transit_micros::numeric) as transit, max(ib.updated_at) as updated_at",
    );
    qb.push(" from inventory_balances ib left join skus s on s.tenant_id = ib.tenant_id and s.sku_id = ib.sku_id where ib.tenant_id = ").push_bind(Uuid::from(tenant));
    source_filters(&mut qb, scope);
    qb.push(" group by ib.sku_id)");
    qb
}

impl InventoryRepo<'_> {
    pub async fn list_inventory(
        &self,
        tenant: TenantId,
        filters: InventoryListFilters,
    ) -> Result<InventoryListResult> {
        let scope = ReadScope {
            warehouse: filters.warehouse_id,
            warehouses: filters.warehouse_ids.as_deref(),
            sku: filters.sku_id,
            code: filters.sku_code.as_deref(),
            barcode: filters.barcode.as_deref(),
            tag: filters.tag.as_deref(),
            search: None,
        };
        let mut tx = begin_read(self.pool).await?;
        require_reconciled(&mut tx, tenant, &scope).await?;
        let mut qb = QueryBuilder::<Postgres>::new(
            "select ib.warehouse_id, ib.sku_id, s.sku_code, s.barcode, (",
        );
        qb.push(estimated_unit_cost_sql()).push(")::text as cost, ib.on_hand_micros::text as hand, ib.reserved_micros::text as reserved, ib.frozen_micros::text as frozen, ib.in_transit_micros::text as transit, ib.updated_at");
        // Append with the same builder so bind placeholders remain correct.
        qb.push(" from inventory_balances ib left join skus s on s.tenant_id = ib.tenant_id and s.sku_id = ib.sku_id where ib.tenant_id = ").push_bind(Uuid::from(tenant));
        source_filters(&mut qb, &scope);
        quantity_filters(
            &mut qb,
            "ib.on_hand_micros",
            "(ib.on_hand_micros::numeric - ib.reserved_micros::numeric - ib.frozen_micros::numeric)",
            QuantityBounds::new(
                filters.on_hand_negative,
                filters.available_below_micros,
                filters.available_min,
                filters.available_max,
            ),
        )?;
        if let Some(cursor) = &filters.cursor {
            qb.push(" and (ib.updated_at, ib.warehouse_id, ib.sku_id) < (")
                .push_bind(cursor.updated_at)
                .push(", ")
                .push_bind(cursor.warehouse_id)
                .push(", ")
                .push_bind(cursor.sku_id)
                .push(")");
        }
        let limit = filters.limit.clamp(1, MAX_READ_PAGE);
        qb.push(" order by ib.updated_at desc, ib.warehouse_id desc, ib.sku_id desc");
        if filters.cursor.is_none() {
            qb.push(" offset ")
                .push_bind(filters.offset.unwrap_or(0).max(0));
        }
        qb.push(" limit ").push_bind(limit + 1);
        let mut rows: Vec<StockRow> = qb.build_query_as().fetch_all(&mut *tx).await?;
        let has_more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        let next_cursor = if has_more {
            rows.last().and_then(|row| {
                row.warehouse_id.map(|warehouse_id| InventoryCursor {
                    updated_at: row.updated_at,
                    warehouse_id,
                    sku_id: row.sku_id,
                })
            })
        } else {
            None
        };
        let items = rows
            .into_iter()
            .map(|row| {
                let exact = ExactAmounts::parse(&row)?;
                let [on_hand, reserved, frozen, in_transit, available_whole] = exact.whole()?;
                let cost_cent = cost_value(&row)?.and_then(|value| i64::try_from(value).ok());
                Ok(InventoryRecord {
                    warehouse_id: row
                        .warehouse_id
                        .ok_or_else(|| anyhow!("missing warehouse"))?,
                    sku_id: row.sku_id,
                    sku_code: row.sku_code,
                    barcode: row.barcode,
                    cost_cent,
                    on_hand,
                    reserved,
                    frozen,
                    in_transit,
                    updated_at: row.updated_at,
                    quantities: exact.wire(),
                    available_whole,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        tx.commit().await?;
        Ok(InventoryListResult { items, next_cursor })
    }

    pub async fn list_inventory_summary(
        &self,
        tenant: TenantId,
        filters: InventorySummaryListFilters,
    ) -> Result<InventorySummaryListResult> {
        let scope = ReadScope {
            warehouse: filters.warehouse_id,
            warehouses: filters.warehouse_ids.as_deref(),
            sku: filters.sku_id,
            code: filters.sku_code.as_deref(),
            barcode: filters.barcode.as_deref(),
            tag: filters.tag.as_deref(),
            search: None,
        };
        let mut tx = begin_read(self.pool).await?;
        require_reconciled(&mut tx, tenant, &scope).await?;
        let (rows, total) = summary_rows(
            &mut tx,
            tenant,
            &scope,
            QuantityBounds::new(
                filters.on_hand_negative,
                filters.available_below_micros,
                filters.available_min,
                filters.available_max,
            ),
            (filters.offset, filters.limit),
        )
        .await?;
        let items = rows
            .into_iter()
            .map(|row| {
                let exact = ExactAmounts::parse(&row)?;
                let [on_hand, reserved, frozen, in_transit, available_whole] = exact.whole()?;
                Ok(InventorySummaryRecord {
                    sku_id: row.sku_id,
                    sku_code: row.sku_code,
                    barcode: row.barcode,
                    on_hand,
                    reserved,
                    frozen,
                    in_transit,
                    updated_at: row.updated_at,
                    quantities: exact.wire(),
                    available_whole,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        tx.commit().await?;
        Ok(InventorySummaryListResult { items, total })
    }

    pub async fn list_inventory_costs(
        &self,
        tenant: TenantId,
        filters: InventoryCostListFilters,
    ) -> Result<InventoryCostListResult> {
        let scope = ReadScope {
            warehouse: filters.warehouse_id,
            warehouses: filters.warehouse_ids.as_deref(),
            sku: filters.sku_id,
            code: filters.sku_code.as_deref(),
            barcode: filters.barcode.as_deref(),
            tag: filters.tag.as_deref(),
            search: None,
        };
        let mut tx = begin_read(self.pool).await?;
        require_reconciled(&mut tx, tenant, &scope).await?;
        let (rows, total) = summary_rows(
            &mut tx,
            tenant,
            &scope,
            QuantityBounds::new(
                filters.on_hand_negative,
                filters.available_below_micros,
                filters.available_min,
                filters.available_max,
            ),
            (filters.offset, filters.limit),
        )
        .await?;
        let items = rows
            .into_iter()
            .map(|row| {
                let exact = ExactAmounts::parse(&row)?;
                let [on_hand, reserved, frozen, in_transit, available_whole] = exact.whole()?;
                let cost = cost_value(&row)?;
                let micro_cents = cost
                    .map(|value| {
                        value
                            .checked_mul(exact.hand)
                            .ok_or(InventoryReadError::ValueOverflow)
                    })
                    .transpose()?;
                Ok(InventoryCostRecord {
                    sku_id: row.sku_id,
                    sku_code: row.sku_code,
                    barcode: row.barcode,
                    on_hand,
                    reserved,
                    frozen,
                    in_transit,
                    updated_at: row.updated_at,
                    unit_cost_cent: cost.and_then(|value| i64::try_from(value).ok()),
                    total_cost_cent: micro_cents
                        .and_then(|value| i64::try_from(value / SCALE).ok()),
                    estimated_value: micro_cents.map(|value| decimal_text(value, 8)),
                    quantities: exact.wire(),
                    available_whole,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        tx.commit().await?;
        Ok(InventoryCostListResult { items, total })
    }

    pub async fn list_ledgers(
        &self,
        tenant: TenantId,
        filters: InventoryLedgerListFilters,
    ) -> Result<InventoryLedgerListResult> {
        let mut tx = begin_read(self.pool).await?;
        let mut count = ledger_source(tenant, &filters, "select count(*)");
        let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;
        let mut qb = ledger_source(
            tenant,
            &filters,
            "select l.ledger_id, l.warehouse_id, l.sku_id, s.sku_code, l.delta_on_hand, l.delta_reserved, l.delta_frozen, l.delta_in_transit, l.delta_on_hand_micros, l.delta_reserved_micros, l.delta_frozen_micros, l.delta_in_transit_micros, l.reason_type, l.ref_type, l.ref_id, l.occurred_at",
        );
        qb.push(" order by l.occurred_at desc, l.ledger_id desc offset ")
            .push_bind(filters.offset.max(0))
            .push(" limit ")
            .push_bind(filters.limit.clamp(1, MAX_READ_PAGE));
        let rows: Vec<LedgerRow> = qb.build_query_as().fetch_all(&mut *tx).await?;
        let items = rows
            .into_iter()
            .map(LedgerRow::record)
            .collect::<Result<Vec<_>>>()?;
        tx.commit().await?;
        Ok(InventoryLedgerListResult { items, total })
    }
}

async fn summary_rows(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    scope: &ReadScope<'_>,
    bounds: QuantityBounds,
    page: (i64, i64),
) -> Result<(Vec<StockRow>, i64)> {
    let (offset, limit) = page;
    let mut count = summary_cte(tenant, scope);
    count.push(" select count(*) from summary where 1=1");
    quantity_filters(&mut count, "hand", "(hand - reserved - frozen)", bounds)?;
    let total: i64 = count.build_query_scalar().fetch_one(&mut **tx).await?;
    let mut qb = summary_cte(tenant, scope);
    qb.push(" select null::uuid as warehouse_id, a.sku_id, s.sku_code, s.barcode, (")
        .push(estimated_unit_cost_sql()).push(")::text as cost, a.hand::text, a.reserved::text, a.frozen::text, a.transit::text, a.updated_at from summary a left join skus s on s.sku_id = a.sku_id and s.tenant_id = ")
        .push_bind(Uuid::from(tenant)).push(" where 1=1");
    quantity_filters(&mut qb, "hand", "(hand - reserved - frozen)", bounds)?;
    qb.push(" order by a.updated_at desc, a.sku_id desc offset ")
        .push_bind(offset.max(0))
        .push(" limit ")
        .push_bind(limit.clamp(1, MAX_READ_PAGE));
    Ok((qb.build_query_as().fetch_all(&mut **tx).await?, total))
}

#[derive(sqlx::FromRow)]
struct LedgerRow {
    ledger_id: Uuid,
    warehouse_id: Uuid,
    sku_id: Uuid,
    sku_code: Option<String>,
    delta_on_hand: i64,
    delta_reserved: i64,
    delta_frozen: i64,
    delta_in_transit: i64,
    delta_on_hand_micros: Option<i64>,
    delta_reserved_micros: Option<i64>,
    delta_frozen_micros: Option<i64>,
    delta_in_transit_micros: Option<i64>,
    reason_type: String,
    ref_type: Option<String>,
    ref_id: Option<String>,
    occurred_at: DateTime<Utc>,
}

impl LedgerRow {
    fn record(self) -> Result<InventoryLedgerRecord> {
        let exact = [
            self.delta_on_hand_micros,
            self.delta_reserved_micros,
            self.delta_frozen_micros,
            self.delta_in_transit_micros,
        ];
        let whole = [
            self.delta_on_hand,
            self.delta_reserved,
            self.delta_frozen,
            self.delta_in_transit,
        ];
        let mut values = [0; 4];
        let count = exact.iter().flatten().count();
        if count != 0 && count != 4 {
            return Err(InventoryReadError::ReconciliationRequired(1).into());
        }
        for (index, value) in values.iter_mut().enumerate() {
            *value = exact[index].map_or(i128::from(whole[index]) * SCALE, i128::from);
        }
        Ok(InventoryLedgerRecord {
            ledger_id: self.ledger_id,
            warehouse_id: self.warehouse_id,
            sku_id: self.sku_id,
            sku_code: self.sku_code,
            delta_on_hand: self.delta_on_hand,
            delta_reserved: self.delta_reserved,
            delta_frozen: self.delta_frozen,
            delta_in_transit: self.delta_in_transit,
            reason_type: self.reason_type,
            ref_type: self.ref_type,
            ref_id: self.ref_id,
            occurred_at: self.occurred_at,
            quantities: InventoryReadDelta {
                source: if count == 4 {
                    "canonical_micros"
                } else {
                    "legacy_whole_units"
                }
                .into(),
                delta_on_hand: decimal_text(values[0], 6),
                delta_reserved: decimal_text(values[1], 6),
                delta_frozen: decimal_text(values[2], 6),
                delta_in_transit: decimal_text(values[3], 6),
            },
        })
    }
}

fn ledger_source<'a>(
    tenant: TenantId,
    filters: &'a InventoryLedgerListFilters,
    select: &str,
) -> QueryBuilder<'a, Postgres> {
    let mut qb = QueryBuilder::new(select);
    qb.push(" from inventory_ledgers l left join skus s on s.tenant_id = l.tenant_id and s.sku_id = l.sku_id where l.tenant_id = ")
        .push_bind(Uuid::from(tenant));
    if let Some(id) = filters.warehouse_id {
        qb.push(" and l.warehouse_id = ").push_bind(id);
    }
    if let Some(ids) = filters.warehouse_ids.as_deref() {
        qb.push(" and l.warehouse_id = any(")
            .push_bind(ids)
            .push(")");
    }
    if let Some(id) = filters.sku_id {
        qb.push(" and l.sku_id = ").push_bind(id);
    }
    if let Some(code) = filters.sku_code.as_deref() {
        qb.push(" and s.sku_code = ").push_bind(code);
    }
    if let Some(date) = filters.occurred_from {
        qb.push(" and l.occurred_at >= ").push_bind(date);
    }
    if let Some(date) = filters.occurred_to {
        qb.push(" and l.occurred_at <= ").push_bind(date);
    }
    qb
}
