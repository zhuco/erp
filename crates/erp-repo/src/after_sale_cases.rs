use crate::inventory::{InventoryDelta, InventoryService};
use crate::operation_logs::{OP_ACTION_RETURN_STOCKED, OperationLogCreate, OperationLogsRepo};
use anyhow::anyhow;
use chrono::{DateTime, NaiveDate, Utc};
use erp_domain::{
    AfterSaleCase, AfterSaleCaseDetail, AfterSaleCaseItem, aggregate_after_sale_state,
    aggregate_refund_state,
};
use erp_types::{
    AfterSaleCaseId, AfterSaleState, OrderId, Platform, RefundState, ShopId, TenantId,
};
use serde_json::json;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

const RETURN_REASON_TYPE: &str = "RETURN_IN";
const RETURN_REF_TYPE: &str = "AFTER_SALE_CASE";

#[derive(sqlx::FromRow, Clone)]
struct OrderItemRow {
    line_id: String,
    sku_id: Option<String>,
    title: Option<String>,
    qty: i32,
    refund_state: String,
    after_sale_state: String,
    refund_amount_cent: Option<i64>,
    after_sale_type: Option<String>,
    after_sale_id: Option<String>,
}

#[derive(Default)]
struct ReturnSku {
    qty: i64,
    should_stock: bool,
}

fn is_return_refund(after_sale_type: Option<&str>) -> bool {
    let Some(raw) = after_sale_type else {
        return false;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let normalized = trimmed.to_lowercase();
    trimmed.contains("退货") || normalized.contains("return")
}

pub struct AfterSaleCaseRepo<'a> {
    pool: &'a PgPool,
}

pub struct AfterSaleCaseListFilters {
    pub platform: Option<Platform>,
    pub shop_id: Option<ShopId>,
    pub refund_state: Option<RefundState>,
    pub after_sale_state: Option<AfterSaleState>,
    pub limit: i64,
    pub offset: i64,
}

impl<'a> AfterSaleCaseRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn sync_cases_for_order(
        &self,
        tenant_id: TenantId,
        platform: Platform,
        shop_id: ShopId,
        order_id: OrderId,
    ) -> anyhow::Result<()> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct LocRow {
            order_month: NaiveDate,
        }

        let loc = sqlx::query_as::<_, LocRow>(
            r#"
            select order_month
            from order_index
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(&mut *tx)
        .await?;

        let Some(loc) = loc else {
            tx.commit().await?;
            return Ok(());
        };

        #[derive(sqlx::FromRow)]
        struct CaseRow {
            case_id: uuid::Uuid,
        }

        let existing_case_rows: Vec<CaseRow> = sqlx::query_as::<_, CaseRow>(
            r#"
            select distinct case_id
            from after_sale_case_items
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            delete from after_sale_case_items
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .execute(&mut *tx)
        .await?;

        let order_items: Vec<OrderItemRow> = sqlx::query_as::<_, OrderItemRow>(
            r#"
            select
                line_id, sku_id, title, qty,
                refund_state, after_sale_state,
                refund_amount_cent, after_sale_type, after_sale_id
            from order_items
            where tenant_id = $1 and order_month = $2 and order_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(loc.order_month)
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(&mut *tx)
        .await?;

        let mut groups: HashMap<String, Vec<OrderItemRow>> = HashMap::new();
        for item in order_items {
            let Some(after_sale_id) = item.after_sale_id.as_ref() else {
                continue;
            };
            let trimmed = after_sale_id.trim();
            if trimmed.is_empty() {
                continue;
            }
            groups.entry(trimmed.to_string()).or_default().push(item);
        }

        let mut refresh_ids: HashSet<uuid::Uuid> =
            existing_case_rows.into_iter().map(|r| r.case_id).collect();

        for (after_sale_id, items) in groups {
            let case_type = items.iter().find_map(|it| it.after_sale_type.clone());
            let case_id: uuid::Uuid = sqlx::query_scalar(
                r#"
                insert into after_sale_cases(
                    tenant_id, platform, shop_id, after_sale_id, after_sale_type
                ) values ($1, $2, $3, $4, $5)
                on conflict (tenant_id, platform, shop_id, after_sale_id)
                do update set
                    after_sale_type = coalesce(excluded.after_sale_type, after_sale_cases.after_sale_type),
                    updated_at = now()
                returning case_id
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(platform.to_string())
            .bind(uuid::Uuid::from(shop_id))
            .bind(&after_sale_id)
            .bind(&case_type)
            .fetch_one(&mut *tx)
            .await?;

            for item in &items {
                sqlx::query(
                    r#"
                    insert into after_sale_case_items(
                        tenant_id, case_id, order_id, order_month, line_id,
                        sku_id, title, qty,
                        refund_state, after_sale_state,
                        refund_amount_cent, after_sale_type
                    ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
                    on conflict (tenant_id, case_id, order_id, line_id)
                    do update set
                        sku_id = excluded.sku_id,
                        title = excluded.title,
                        qty = excluded.qty,
                        refund_state = excluded.refund_state,
                        after_sale_state = excluded.after_sale_state,
                        refund_amount_cent = excluded.refund_amount_cent,
                        after_sale_type = excluded.after_sale_type,
                        updated_at = now()
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(case_id)
                .bind(uuid::Uuid::from(order_id))
                .bind(loc.order_month)
                .bind(&item.line_id)
                .bind(&item.sku_id)
                .bind(&item.title)
                .bind(item.qty)
                .bind(&item.refund_state)
                .bind(&item.after_sale_state)
                .bind(item.refund_amount_cent)
                .bind(&item.after_sale_type)
                .execute(&mut *tx)
                .await?;
            }

            self.process_refund_returns(&mut tx, tenant_id, order_id, case_id, &items)
                .await?;

            refresh_ids.insert(case_id);
        }

        for case_id in refresh_ids {
            self.refresh_case_summary(&mut tx, tenant_id, case_id)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_cases(
        &self,
        tenant_id: TenantId,
        filters: AfterSaleCaseListFilters,
    ) -> anyhow::Result<Vec<AfterSaleCase>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                case_id, platform, shop_id, after_sale_id, after_sale_type,
                refund_state, after_sale_state,
                refund_amount_cent, item_count, total_qty, order_count,
                created_at, updated_at
            from after_sale_cases
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(platform) = filters.platform {
            qb.push(" and platform = ");
            qb.push_bind(platform.to_string());
        }
        if let Some(shop_id) = filters.shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(uuid::Uuid::from(shop_id));
        }
        if let Some(refund_state) = filters.refund_state {
            qb.push(" and refund_state = ");
            qb.push_bind(refund_state.to_string());
        }
        if let Some(after_sale_state) = filters.after_sale_state {
            qb.push(" and after_sale_state = ");
            qb.push_bind(after_sale_state.to_string());
        }

        qb.push(" order by updated_at desc, case_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        #[derive(sqlx::FromRow)]
        struct Row {
            case_id: uuid::Uuid,
            platform: String,
            shop_id: uuid::Uuid,
            after_sale_id: String,
            after_sale_type: Option<String>,
            refund_state: String,
            after_sale_state: String,
            refund_amount_cent: i64,
            item_count: i32,
            total_qty: i32,
            order_count: i32,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;

        rows.into_iter()
            .map(|r| {
                let platform = Platform::from_str(&r.platform)
                    .map_err(|e| anyhow!("bad platform in db: {e}"))?;
                let refund_state =
                    RefundState::from_str(&r.refund_state).unwrap_or(RefundState::Unknown);
                let after_sale_state = AfterSaleState::from_str(&r.after_sale_state)
                    .unwrap_or(AfterSaleState::Unknown);
                Ok(AfterSaleCase {
                    tenant_id,
                    case_id: AfterSaleCaseId(r.case_id),
                    platform,
                    shop_id: ShopId(r.shop_id),
                    after_sale_id: r.after_sale_id,
                    after_sale_type: r.after_sale_type,
                    refund_state,
                    after_sale_state,
                    refund_amount_cent: r.refund_amount_cent,
                    item_count: r.item_count,
                    total_qty: r.total_qty,
                    order_count: r.order_count,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
            })
            .collect()
    }

    pub async fn get_case_detail(
        &self,
        tenant_id: TenantId,
        case_id: AfterSaleCaseId,
    ) -> anyhow::Result<Option<AfterSaleCaseDetail>> {
        #[derive(sqlx::FromRow)]
        struct CaseRow {
            platform: String,
            shop_id: uuid::Uuid,
            after_sale_id: String,
            after_sale_type: Option<String>,
            refund_state: String,
            after_sale_state: String,
            refund_amount_cent: i64,
            item_count: i32,
            total_qty: i32,
            order_count: i32,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let case_row = sqlx::query_as::<_, CaseRow>(
            r#"
            select
                platform, shop_id, after_sale_id, after_sale_type,
                refund_state, after_sale_state,
                refund_amount_cent, item_count, total_qty, order_count,
                created_at, updated_at
            from after_sale_cases
            where tenant_id = $1 and case_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(case_id))
        .fetch_optional(self.pool)
        .await?;

        let Some(case_row) = case_row else {
            return Ok(None);
        };

        let platform = Platform::from_str(&case_row.platform)
            .map_err(|e| anyhow!("bad platform in db: {e}"))?;
        let refund_state =
            RefundState::from_str(&case_row.refund_state).unwrap_or(RefundState::Unknown);
        let after_sale_state =
            AfterSaleState::from_str(&case_row.after_sale_state).unwrap_or(AfterSaleState::Unknown);

        let case = AfterSaleCase {
            tenant_id,
            case_id,
            platform,
            shop_id: ShopId(case_row.shop_id),
            after_sale_id: case_row.after_sale_id,
            after_sale_type: case_row.after_sale_type,
            refund_state,
            after_sale_state,
            refund_amount_cent: case_row.refund_amount_cent,
            item_count: case_row.item_count,
            total_qty: case_row.total_qty,
            order_count: case_row.order_count,
            created_at: case_row.created_at,
            updated_at: case_row.updated_at,
        };

        #[derive(sqlx::FromRow)]
        struct ItemRow {
            order_id: uuid::Uuid,
            order_month: NaiveDate,
            line_id: String,
            sku_id: Option<String>,
            title: Option<String>,
            qty: i32,
            refund_state: String,
            after_sale_state: String,
            refund_amount_cent: Option<i64>,
            after_sale_type: Option<String>,
        }

        let item_rows: Vec<ItemRow> = sqlx::query_as::<_, ItemRow>(
            r#"
            select
                order_id, order_month, line_id, sku_id, title, qty,
                refund_state, after_sale_state, refund_amount_cent, after_sale_type
            from after_sale_case_items
            where tenant_id = $1 and case_id = $2
            order by order_id asc, line_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(case_id))
        .fetch_all(self.pool)
        .await?;

        #[derive(sqlx::FromRow)]
        struct LedgerRow {
            sku_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            qty: i64,
            created_at: DateTime<Utc>,
        }

        let ledger_rows: Vec<LedgerRow> = sqlx::query_as(
            r#"
            select sku_id, warehouse_id, delta_on_hand as qty, occurred_at as created_at
            from inventory_ledgers
            where tenant_id = $1
              and ref_type = $2
              and ref_id = $3
              and reason_type = $4
            order by occurred_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(RETURN_REF_TYPE)
        .bind(uuid::Uuid::from(case_id).to_string())
        .bind(RETURN_REASON_TYPE)
        .fetch_all(self.pool)
        .await?;

        let mut ledger_map: HashMap<uuid::Uuid, LedgerRow> = HashMap::new();
        for row in ledger_rows {
            ledger_map.entry(row.sku_id).or_insert(row);
        }

        let items = item_rows
            .into_iter()
            .map(|row| {
                let refund_state =
                    RefundState::from_str(&row.refund_state).unwrap_or(RefundState::Unknown);
                let after_sale_state = AfterSaleState::from_str(&row.after_sale_state)
                    .unwrap_or(AfterSaleState::Unknown);
                let (return_stocked, return_warehouse_id, return_at) = row
                    .sku_id
                    .as_ref()
                    .and_then(|sku| {
                        let sku_id = uuid::Uuid::parse_str(sku).ok()?;
                        let ledger = ledger_map.get(&sku_id)?;
                        if ledger.qty <= 0 {
                            return Some((false, None, None));
                        }
                        Some((true, Some(ledger.warehouse_id), Some(ledger.created_at)))
                    })
                    .unwrap_or((false, None, None));
                AfterSaleCaseItem {
                    tenant_id,
                    case_id,
                    order_id: OrderId(row.order_id),
                    order_month: row.order_month,
                    line_id: row.line_id,
                    sku_id: row.sku_id,
                    title: row.title,
                    qty: row.qty,
                    refund_state,
                    after_sale_state,
                    refund_amount_cent: row.refund_amount_cent,
                    after_sale_type: row.after_sale_type,
                    return_stocked,
                    return_warehouse_id,
                    return_at,
                }
            })
            .collect();

        Ok(Some(AfterSaleCaseDetail { case, items }))
    }

    async fn process_refund_returns(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        order_id: OrderId,
        case_id: uuid::Uuid,
        items: &[OrderItemRow],
    ) -> anyhow::Result<()> {
        let mut sku_totals: HashMap<uuid::Uuid, ReturnSku> = HashMap::new();

        for item in items {
            let refund_state =
                RefundState::from_str(&item.refund_state).unwrap_or(RefundState::Unknown);
            if refund_state != RefundState::Success {
                continue;
            }
            let Some(sku_id) = item
                .sku_id
                .as_ref()
                .and_then(|raw| uuid::Uuid::parse_str(raw).ok())
            else {
                continue;
            };

            let entry = sku_totals.entry(sku_id).or_default();
            entry.qty = entry.qty.saturating_add(i64::from(item.qty));
            if is_return_refund(item.after_sale_type.as_deref()) {
                entry.should_stock = true;
            }
        }

        if sku_totals.is_empty() {
            return Ok(());
        }

        let warehouse_id = self
            .resolve_return_warehouse(tx, tenant_id, order_id)
            .await?;

        let inventory_service = InventoryService::new(self.pool);
        for (sku_id, entry) in sku_totals {
            let qty = if entry.should_stock { entry.qty } else { 0 };
            let exists: bool = sqlx::query_scalar(
                r#"
                select exists(
                    select 1
                    from inventory_ledgers
                    where tenant_id = $1 and ref_type = $2 and ref_id = $3 and sku_id = $4
                )
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(RETURN_REF_TYPE)
            .bind(case_id.to_string())
            .bind(sku_id)
            .fetch_one(&mut **tx)
            .await?;
            if exists {
                continue;
            }

            inventory_service
                .apply_with_tx(
                    &mut *tx,
                    InventoryDelta {
                        tenant_id,
                        warehouse_id,
                        sku_id,
                        delta_on_hand: qty,
                        delta_reserved: 0,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: RETURN_REASON_TYPE.to_string(),
                        ref_type: Some(RETURN_REF_TYPE.to_string()),
                        ref_id: Some(case_id.to_string()),
                        occurred_at: Utc::now(),
                    },
                )
                .await?;

            let metadata = json!({
                "case_id": case_id,
                "order_id": order_id.to_string(),
                "warehouse_id": warehouse_id,
                "sku_id": sku_id,
                "qty": qty,
            });
            OperationLogsRepo::insert_with_tx(
                &mut **tx,
                OperationLogCreate {
                    tenant_id,
                    actor_type: "SYSTEM".to_string(),
                    actor_user_id: None,
                    action: OP_ACTION_RETURN_STOCKED.to_string(),
                    ref_type: Some(RETURN_REF_TYPE.to_string()),
                    ref_id: Some(case_id.to_string()),
                    metadata: Some(metadata),
                },
            )
            .await?;
        }

        Ok(())
    }

    async fn resolve_return_warehouse(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<uuid::Uuid> {
        #[derive(sqlx::FromRow)]
        struct ShipmentRow {
            warehouse_id: Option<uuid::Uuid>,
        }

        let shipment: Option<ShipmentRow> = sqlx::query_as(
            r#"
            select warehouse_id
            from shipments
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(row) = shipment.and_then(|row| row.warehouse_id) {
            return Ok(row);
        }

        let default_warehouse: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select warehouse_id
            from warehouses
            where tenant_id = $1 and is_default = true
            order by created_at desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(default_id) = default_warehouse {
            return Ok(default_id);
        }

        let fallback: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select warehouse_id
            from warehouses
            where tenant_id = $1
            order by created_at desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(&mut **tx)
        .await?;

        fallback.ok_or_else(|| anyhow!("warehouse not found for return"))
    }

    async fn refresh_case_summary(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        case_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        #[derive(sqlx::FromRow)]
        struct SummaryRow {
            order_id: uuid::Uuid,
            refund_state: String,
            after_sale_state: String,
            refund_amount_cent: Option<i64>,
            qty: i32,
        }

        let rows: Vec<SummaryRow> = sqlx::query_as::<_, SummaryRow>(
            r#"
            select order_id, refund_state, after_sale_state, refund_amount_cent, qty
            from after_sale_case_items
            where tenant_id = $1 and case_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(case_id)
        .fetch_all(&mut **tx)
        .await?;

        if rows.is_empty() {
            sqlx::query(
                r#"
                delete from after_sale_cases
                where tenant_id = $1 and case_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(case_id)
            .execute(&mut **tx)
            .await?;
            return Ok(());
        }

        let refund_state = aggregate_refund_state(
            rows.iter()
                .map(|r| RefundState::from_str(&r.refund_state).unwrap_or(RefundState::Unknown)),
        );
        let after_sale_state = aggregate_after_sale_state(rows.iter().map(|r| {
            AfterSaleState::from_str(&r.after_sale_state).unwrap_or(AfterSaleState::Unknown)
        }));
        let refund_amount_cent = rows
            .iter()
            .map(|r| r.refund_amount_cent.unwrap_or(0))
            .sum::<i64>();
        let total_qty = rows.iter().map(|r| r.qty).sum::<i32>();
        let item_count = rows.len() as i32;
        let order_count = rows
            .iter()
            .map(|r| r.order_id)
            .collect::<HashSet<_>>()
            .len() as i32;

        sqlx::query(
            r#"
            update after_sale_cases
            set refund_state = $3,
                after_sale_state = $4,
                refund_amount_cent = $5,
                item_count = $6,
                total_qty = $7,
                order_count = $8,
                updated_at = now()
            where tenant_id = $1 and case_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(case_id)
        .bind(refund_state.to_string())
        .bind(after_sale_state.to_string())
        .bind(refund_amount_cent)
        .bind(item_count)
        .bind(total_qty)
        .bind(order_count)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
