use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde_json::{Map, Value, json};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::HashMap;

use crate::inventory::{InventoryDelta, InventoryService};

#[derive(Debug, Clone)]
pub struct StockCount {
    pub count_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub status: String,
    pub strategy: String,
    pub dimension: String,
    pub filters_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockCountItem {
    pub count_item_id: uuid::Uuid,
    pub count_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub location_id: Option<uuid::Uuid>,
    pub system_qty: i64,
    pub counted_qty: i64,
    pub diff_qty: i64,
    pub diff_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockCountDetail {
    pub count: StockCount,
    pub items: Vec<StockCountItem>,
}

#[derive(Debug, Clone)]
pub struct StockCountCreateInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub strategy: String,
    pub dimension: String,
    pub recent_days: Option<i64>,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub scope_type: Option<String>,
    pub scope_ids: Option<Vec<uuid::Uuid>>,
    pub count_mode: Option<String>,
    pub count_type: Option<String>,
    pub freeze_stock: Option<bool>,
    pub created_by: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct StockCountCreateWithItemsInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_ids: Vec<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct StockCountItemUpdate {
    pub count_item_id: uuid::Uuid,
    pub counted_qty: i64,
    pub diff_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StockCountListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub count_type: Option<String>,
    pub scope_type: Option<String>,
    pub keyword: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct StockCountImportItemInput {
    pub sku_id: uuid::Uuid,
    pub location_id: Option<uuid::Uuid>,
    pub counted_qty: i64,
    pub diff_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StockCountImportInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub strategy: String,
    pub dimension: String,
    pub filters_json: Value,
    pub items: Vec<StockCountImportItemInput>,
}

pub struct StockCountListResult {
    pub items: Vec<StockCount>,
    pub total: i64,
}

pub struct StockCountsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> StockCountsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_count(&self, input: StockCountCreateInput) -> anyhow::Result<StockCount> {
        let StockCountCreateInput {
            tenant_id,
            warehouse_id,
            strategy,
            dimension,
            recent_days,
            brand,
            category,
            scope_type,
            scope_ids,
            count_mode,
            count_type,
            freeze_stock,
            created_by,
        } = input;
        let scope_ids = scope_ids.unwrap_or_default();
        let scope_type = scope_type.map(|value| value.to_string());
        let needs_scope_ids = matches!(
            scope_type.as_deref(),
            Some("AREA") | Some("LOCATION") | Some("SKU")
        );
        if needs_scope_ids && scope_ids.is_empty() {
            return Err(anyhow!("scope_ids required"));
        }
        let filters_json = json!({
            "recent_days": recent_days,
            "brand": brand,
            "category": category,
            "scope_type": scope_type,
            "scope_ids": scope_ids,
            "count_mode": count_mode,
            "count_type": count_type,
            "freeze_stock": freeze_stock,
            "created_by": created_by,
        });
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockCountRow = sqlx::query_as(
            r#"
            insert into stock_counts(
                tenant_id, warehouse_id, status, strategy, dimension, filters_json
            ) values ($1,$2,'COUNTING',$3,$4,$5)
            returning count_id, tenant_id, warehouse_id, status,
                      strategy, dimension, filters_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(&strategy)
        .bind(&dimension)
        .bind(filters_json)
        .fetch_one(&mut *tx)
        .await?;

        let tenant_uuid = uuid::Uuid::from(tenant_id);
        let mut qb = QueryBuilder::<Postgres>::new(
            "insert into stock_count_items(tenant_id, count_id, sku_id, location_id, system_qty, counted_qty, diff_qty) ",
        );
        if dimension == "LOCATION" {
            qb.push("select ");
            qb.push_bind(tenant_uuid);
            qb.push(", ");
            qb.push_bind(row.count_id);
            qb.push(
                ", ilb.sku_id, ilb.location_id, ilb.on_hand, 0, 0 from inventory_location_balances ilb ",
            );
            if matches!(scope_type.as_deref(), Some("AREA")) {
                qb.push("left join warehouse_locations wl on wl.location_id = ilb.location_id ");
                qb.push("and wl.tenant_id = ");
                qb.push_bind(tenant_uuid);
                qb.push(" ");
            }
            qb.push("left join skus s on s.sku_id = ilb.sku_id ");
            qb.push("left join products p on p.product_id = s.product_id ");
            qb.push("where ilb.tenant_id = ");
            qb.push_bind(tenant_uuid);
            qb.push(" and ilb.warehouse_id = ");
            qb.push_bind(warehouse_id);
            if let Some(scope) = scope_type.as_deref() {
                match scope {
                    "LOCATION" => {
                        qb.push(" and ilb.location_id = any(");
                        qb.push_bind(&scope_ids);
                        qb.push(")");
                    }
                    "AREA" => {
                        qb.push(" and wl.area_id = any(");
                        qb.push_bind(&scope_ids);
                        qb.push(")");
                    }
                    "SKU" => {
                        qb.push(" and ilb.sku_id = any(");
                        qb.push_bind(&scope_ids);
                        qb.push(")");
                    }
                    _ => {}
                }
            }
            match strategy.as_str() {
                "RECENT_DAYS" => {
                    if let Some(days) = recent_days {
                        let cutoff = Utc::now() - chrono::Duration::days(days);
                        qb.push(
                            " and exists (select 1 from inventory_ledgers l where l.tenant_id = ",
                        );
                        qb.push_bind(tenant_uuid);
                        qb.push(" and l.warehouse_id = ");
                        qb.push_bind(warehouse_id);
                        qb.push(" and l.sku_id = ilb.sku_id and l.occurred_at >= ");
                        qb.push_bind(cutoff);
                        qb.push(")");
                    }
                }
                "BRAND" => {
                    if let Some(value) = row.filters_json.get("brand").and_then(|v| v.as_str()) {
                        qb.push(" and p.brand = ");
                        qb.push_bind(value);
                    }
                }
                "CATEGORY" => {
                    if let Some(value) = row.filters_json.get("category").and_then(|v| v.as_str()) {
                        qb.push(" and p.category = ");
                        qb.push_bind(value);
                    }
                }
                _ => {}
            }
        } else {
            qb.push("select ");
            qb.push_bind(tenant_uuid);
            qb.push(", ");
            qb.push_bind(row.count_id);
            qb.push(", ib.sku_id, null, ib.on_hand, 0, 0 from inventory_balances ib ");
            qb.push("left join skus s on s.sku_id = ib.sku_id ");
            qb.push("left join products p on p.product_id = s.product_id ");
            qb.push("where ib.tenant_id = ");
            qb.push_bind(tenant_uuid);
            qb.push(" and ib.warehouse_id = ");
            qb.push_bind(warehouse_id);
            if matches!(scope_type.as_deref(), Some("SKU")) {
                qb.push(" and ib.sku_id = any(");
                qb.push_bind(&scope_ids);
                qb.push(")");
            }
            match strategy.as_str() {
                "RECENT_DAYS" => {
                    if let Some(days) = recent_days {
                        let cutoff = Utc::now() - chrono::Duration::days(days);
                        qb.push(
                            " and exists (select 1 from inventory_ledgers l where l.tenant_id = ",
                        );
                        qb.push_bind(tenant_uuid);
                        qb.push(" and l.warehouse_id = ");
                        qb.push_bind(warehouse_id);
                        qb.push(" and l.sku_id = ib.sku_id and l.occurred_at >= ");
                        qb.push_bind(cutoff);
                        qb.push(")");
                    }
                }
                "BRAND" => {
                    if let Some(value) = row.filters_json.get("brand").and_then(|v| v.as_str()) {
                        qb.push(" and p.brand = ");
                        qb.push_bind(value);
                    }
                }
                "CATEGORY" => {
                    if let Some(value) = row.filters_json.get("category").and_then(|v| v.as_str()) {
                        qb.push(" and p.category = ");
                        qb.push_bind(value);
                    }
                }
                _ => {}
            }
        }

        qb.build().execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(row.into_count())
    }

    pub async fn create_count_with_items(
        &self,
        input: StockCountCreateWithItemsInput,
    ) -> anyhow::Result<StockCount> {
        if input.sku_ids.is_empty() {
            return Err(anyhow!("sku_ids required"));
        }
        let strategy = "SUGGESTION";
        let dimension = "SKU";
        let filters_json = json!({ "source": "SUGGESTION" });
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockCountRow = sqlx::query_as(
            r#"
            insert into stock_counts(
                tenant_id, warehouse_id, status, strategy, dimension, filters_json
            ) values ($1,$2,'COUNTING',$3,$4,$5)
            returning count_id, tenant_id, warehouse_id, status,
                      strategy, dimension, filters_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(strategy)
        .bind(dimension)
        .bind(filters_json)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            insert into stock_count_items(
                tenant_id, count_id, sku_id, system_qty, counted_qty, diff_qty
            )
            select $1, $2, sku_id, coalesce(sum(b.on_hand), 0), 0, 0
            from unnest($3::uuid[]) as sku_id
            left join inventory_balances b
              on b.tenant_id = $1
             and b.warehouse_id = $4
             and b.sku_id = sku_id
            group by sku_id
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(row.count_id)
        .bind(&input.sku_ids)
        .bind(input.warehouse_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row.into_count())
    }

    pub async fn create_count_with_import_items(
        &self,
        input: StockCountImportInput,
    ) -> anyhow::Result<StockCountDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("import items required"));
        }
        let mut items_map: HashMap<(uuid::Uuid, Option<uuid::Uuid>), i64> = HashMap::new();
        for item in input.items {
            if item.counted_qty < 0 {
                return Err(anyhow!("counted_qty must be non-negative"));
            }
            if input.dimension == "LOCATION" && item.location_id.is_none() {
                return Err(anyhow!("location_id required"));
            }
            let key = (item.sku_id, item.location_id);
            *items_map.entry(key).or_insert(0) += item.counted_qty;
        }
        let mut sku_ids: Vec<uuid::Uuid> = Vec::with_capacity(items_map.len());
        let mut location_ids: Vec<uuid::Uuid> = Vec::with_capacity(items_map.len());
        let mut counted_qtys: Vec<i64> = Vec::with_capacity(items_map.len());
        for ((sku_id, location_id), counted_qty) in items_map {
            sku_ids.push(sku_id);
            if input.dimension == "LOCATION" {
                location_ids.push(location_id.ok_or_else(|| anyhow!("location_id required"))?);
            }
            counted_qtys.push(counted_qty);
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockCountRow = sqlx::query_as(
            r#"
            insert into stock_counts(
                tenant_id, warehouse_id, status, strategy, dimension, filters_json
            ) values ($1,$2,'COUNTING',$3,$4,$5)
            returning count_id, tenant_id, warehouse_id, status,
                      strategy, dimension, filters_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(&input.strategy)
        .bind(&input.dimension)
        .bind(input.filters_json)
        .fetch_one(&mut *tx)
        .await?;

        if input.dimension == "LOCATION" {
            sqlx::query(
                r#"
                with input_items as (
                    select unnest($3::uuid[]) as sku_id,
                           unnest($4::uuid[]) as location_id,
                           unnest($5::bigint[]) as counted_qty
                )
                insert into stock_count_items(
                    tenant_id, count_id, sku_id, location_id, system_qty, counted_qty, diff_qty
                )
                select $1, $2, input_items.sku_id, input_items.location_id,
                       coalesce(ilb.on_hand, 0),
                       input_items.counted_qty,
                       input_items.counted_qty - coalesce(ilb.on_hand, 0)
                from input_items
                left join inventory_location_balances ilb
                  on ilb.tenant_id = $1
                 and ilb.warehouse_id = $6
                 and ilb.sku_id = input_items.sku_id
                 and ilb.location_id = input_items.location_id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(row.count_id)
            .bind(&sku_ids)
            .bind(&location_ids)
            .bind(&counted_qtys)
            .bind(input.warehouse_id)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                r#"
                with input_items as (
                    select unnest($3::uuid[]) as sku_id,
                           unnest($4::bigint[]) as counted_qty
                )
                insert into stock_count_items(
                    tenant_id, count_id, sku_id, system_qty, counted_qty, diff_qty
                )
                select $1, $2, input_items.sku_id,
                       coalesce(ib.on_hand, 0),
                       input_items.counted_qty,
                       input_items.counted_qty - coalesce(ib.on_hand, 0)
                from input_items
                left join inventory_balances ib
                  on ib.tenant_id = $1
                 and ib.warehouse_id = $5
                 and ib.sku_id = input_items.sku_id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(row.count_id)
            .bind(&sku_ids)
            .bind(&counted_qtys)
            .bind(input.warehouse_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        let items = self.list_items(input.tenant_id, row.count_id).await?;
        Ok(StockCountDetail {
            count: row.into_count(),
            items,
        })
    }

    pub async fn list_counts(
        &self,
        tenant_id: TenantId,
        filters: StockCountListFilters,
    ) -> anyhow::Result<StockCountListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from stock_counts
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_count_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select count_id, tenant_id, warehouse_id, status,
                   strategy, dimension, filters_json, created_at, updated_at
            from stock_counts
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_count_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, count_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<StockCountRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows.into_iter().map(StockCountRow::into_count).collect();

        Ok(StockCountListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        count_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockCountDetail>> {
        let row: Option<StockCountRow> = sqlx::query_as(
            r#"
            select count_id, tenant_id, warehouse_id, status,
                   strategy, dimension, filters_json, created_at, updated_at
            from stock_counts
            where tenant_id = $1 and count_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let items = self.list_items(tenant_id, count_id).await?;
        Ok(Some(StockCountDetail {
            count: row.into_count(),
            items,
        }))
    }

    pub async fn list_items(
        &self,
        tenant_id: TenantId,
        count_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<StockCountItem>> {
        let rows: Vec<StockCountItemRow> = sqlx::query_as(
            r#"
            select
                count_item_id, count_id, sku_id, location_id,
                system_qty, counted_qty, diff_qty, diff_reason, created_at, updated_at
            from stock_count_items
            where tenant_id = $1 and count_id = $2
            order by count_item_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(StockCountItemRow::into_item).collect())
    }

    pub async fn update_items(
        &self,
        tenant_id: TenantId,
        count_id: uuid::Uuid,
        updates: Vec<StockCountItemUpdate>,
    ) -> anyhow::Result<Vec<StockCountItem>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let status: Option<String> = sqlx::query_scalar(
            r#"
            select status
            from stock_counts
            where tenant_id = $1 and count_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(status) = status else {
            return Err(anyhow!("stock count not found"));
        };
        ensure_count_editable(&status)?;

        let mut updated_items = Vec::with_capacity(updates.len());

        for update in updates {
            let row: StockCountItemRow = sqlx::query_as(
                r#"
                update stock_count_items
                set counted_qty = $4,
                    diff_qty = $4 - system_qty,
                    diff_reason = coalesce($5, diff_reason),
                    updated_at = now()
                where tenant_id = $1 and count_id = $2 and count_item_id = $3
                returning
                    count_item_id, count_id, sku_id, location_id,
                    system_qty, counted_qty, diff_qty, diff_reason, created_at, updated_at
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(count_id)
            .bind(update.count_item_id)
            .bind(update.counted_qty)
            .bind(update.diff_reason.as_deref())
            .fetch_one(&mut *tx)
            .await?;
            updated_items.push(row.into_item());
        }

        tx.commit().await?;
        Ok(updated_items)
    }

    pub async fn confirm_diff(
        &self,
        tenant_id: TenantId,
        count_id: uuid::Uuid,
    ) -> anyhow::Result<StockCount> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let status: Option<String> = sqlx::query_scalar(
            r#"
            select status
            from stock_counts
            where tenant_id = $1 and count_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(status) = status else {
            return Err(anyhow!("stock count not found"));
        };
        match resolve_confirm_diff_action(&status)? {
            StockCountConfirmDiffAction::Noop => {
                let row: StockCountRow = sqlx::query_as(
                    r#"
                    select count_id, tenant_id, warehouse_id, status,
                           strategy, dimension, filters_json, created_at, updated_at
                    from stock_counts
                    where tenant_id = $1 and count_id = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(count_id)
                .fetch_one(&mut *tx)
                .await?;
                tx.commit().await?;
                return Ok(row.into_count());
            }
            StockCountConfirmDiffAction::Recalculate => {}
        }

        sqlx::query(
            r#"
            update stock_count_items
            set diff_qty = counted_qty - system_qty,
                updated_at = now()
            where tenant_id = $1 and count_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            insert into inventory_reconciliation_center(
                tenant_id,
                warehouse_id,
                sku_id,
                expected_on_hand,
                actual_on_hand,
                diff_qty,
                status,
                source_ref
            )
            select
                sc.tenant_id,
                sc.warehouse_id,
                sci.sku_id,
                sci.system_qty,
                sci.counted_qty,
                sci.diff_qty,
                case when sci.diff_qty = 0 then 'MATCH' else 'MISMATCH' end,
                concat('STOCK_COUNT:', sc.count_id::text)
            from stock_count_items sci
            join stock_counts sc
              on sc.tenant_id = sci.tenant_id
             and sc.count_id = sci.count_id
            where sci.tenant_id = $1
              and sci.count_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .execute(&mut *tx)
        .await?;

        let row: StockCountRow = sqlx::query_as(
            r#"
            update stock_counts
            set status = 'DIFF_CONFIRMED',
                updated_at = now()
            where tenant_id = $1 and count_id = $2
            returning count_id, tenant_id, warehouse_id, status,
                      strategy, dimension, filters_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row.into_count())
    }

    pub async fn apply_count(
        &self,
        tenant_id: TenantId,
        count_id: uuid::Uuid,
    ) -> anyhow::Result<StockCount> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockCountRow = sqlx::query_as(
            r#"
            select count_id, tenant_id, warehouse_id, status,
                   strategy, dimension, filters_json, created_at, updated_at
            from stock_counts
            where tenant_id = $1 and count_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("stock count not found"))?;

        if row.status == "APPLIED" {
            return Ok(row.into_count());
        }
        if row.status != "DIFF_CONFIRMED" {
            return Err(anyhow!("stock count not ready"));
        }

        #[derive(sqlx::FromRow)]
        struct DiffRow {
            sku_id: uuid::Uuid,
            location_id: Option<uuid::Uuid>,
            diff_qty: i64,
        }

        let diff_rows: Vec<DiffRow> = sqlx::query_as(
            r#"
            select sku_id, location_id, diff_qty
            from stock_count_items
            where tenant_id = $1 and count_id = $2 and diff_qty <> 0
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_all(&mut *tx)
        .await?;

        let mut gain_items: HashMap<uuid::Uuid, i64> = HashMap::new();
        let mut scrap_items: HashMap<uuid::Uuid, i64> = HashMap::new();
        for diff in &diff_rows {
            if diff.diff_qty > 0 {
                *gain_items.entry(diff.sku_id).or_insert(0) += diff.diff_qty;
            } else if diff.diff_qty < 0 {
                *scrap_items.entry(diff.sku_id).or_insert(0) += -diff.diff_qty;
            }
        }
        let mut gain_adjust_id: Option<uuid::Uuid> = None;
        let mut scrap_adjust_id: Option<uuid::Uuid> = None;
        if !gain_items.is_empty() {
            let gain_id: uuid::Uuid = sqlx::query_scalar(
                r#"
                insert into stock_adjust_docs(
                    tenant_id, warehouse_id, reason, status
                ) values ($1,$2,'GAIN','APPLIED')
                returning adjust_id
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.warehouse_id)
            .fetch_one(&mut *tx)
            .await?;
            gain_adjust_id = Some(gain_id);
            for (sku_id, qty) in &gain_items {
                sqlx::query(
                    r#"
                    insert into stock_adjust_items(
                        tenant_id, adjust_id, sku_id, qty
                    ) values ($1,$2,$3,$4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(gain_id)
                .bind(*sku_id)
                .bind(*qty)
                .execute(&mut *tx)
                .await?;
            }
        }
        if !scrap_items.is_empty() {
            let scrap_id: uuid::Uuid = sqlx::query_scalar(
                r#"
                insert into stock_adjust_docs(
                    tenant_id, warehouse_id, reason, status
                ) values ($1,$2,'SCRAP','APPLIED')
                returning adjust_id
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.warehouse_id)
            .fetch_one(&mut *tx)
            .await?;
            scrap_adjust_id = Some(scrap_id);
            for (sku_id, qty) in &scrap_items {
                sqlx::query(
                    r#"
                    insert into stock_adjust_items(
                        tenant_id, adjust_id, sku_id, qty
                    ) values ($1,$2,$3,$4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(scrap_id)
                .bind(*sku_id)
                .bind(*qty)
                .execute(&mut *tx)
                .await?;
            }
        }

        let service = InventoryService::new(self.pool);
        let occurred_at = Utc::now();
        for diff in diff_rows {
            if row.dimension == "LOCATION" {
                let location_id = diff
                    .location_id
                    .ok_or_else(|| anyhow!("missing location_id"))?;
                sqlx::query(
                    r#"
                    insert into inventory_location_balances(
                        tenant_id, warehouse_id, location_id, sku_id, on_hand
                    ) values ($1,$2,$3,$4,$5)
                    on conflict (tenant_id, warehouse_id, location_id, sku_id)
                    do update set
                        on_hand = inventory_location_balances.on_hand + excluded.on_hand,
                        updated_at = now()
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(row.warehouse_id)
                .bind(location_id)
                .bind(diff.sku_id)
                .bind(diff.diff_qty)
                .execute(&mut *tx)
                .await?;
            }
            service
                .apply_with_tx(
                    &mut tx,
                    InventoryDelta {
                        tenant_id,
                        warehouse_id: row.warehouse_id,
                        sku_id: diff.sku_id,
                        delta_on_hand: diff.diff_qty,
                        delta_reserved: 0,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: "STOCK_COUNT_APPLY".to_string(),
                        ref_type: Some("STOCK_COUNT".to_string()),
                        ref_id: Some(count_id.to_string()),
                        occurred_at,
                    },
                )
                .await?;
        }

        let filters_json =
            merge_adjustment_linkage(row.filters_json, gain_adjust_id, scrap_adjust_id);
        let updated: StockCountRow = sqlx::query_as(
            r#"
            update stock_counts
            set status = 'APPLIED',
                filters_json = $3,
                updated_at = now()
            where tenant_id = $1 and count_id = $2
            returning count_id, tenant_id, warehouse_id, status,
                      strategy, dimension, filters_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .bind(filters_json)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated.into_count())
    }

    pub async fn cancel_count(
        &self,
        tenant_id: TenantId,
        count_id: uuid::Uuid,
    ) -> anyhow::Result<StockCount> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockCountRow = sqlx::query_as(
            r#"
            select count_id, tenant_id, warehouse_id, status,
                   strategy, dimension, filters_json, created_at, updated_at
            from stock_counts
            where tenant_id = $1 and count_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("stock count not found"))?;

        if row.status == "CANCELLED" {
            return Ok(row.into_count());
        }
        ensure_cancel_allowed(&row.status)?;

        let updated: StockCountRow = sqlx::query_as(
            r#"
            update stock_counts
            set status = 'CANCELLED',
                updated_at = now()
            where tenant_id = $1 and count_id = $2
            returning count_id, tenant_id, warehouse_id, status,
                      strategy, dimension, filters_json, created_at, updated_at
        "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(count_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated.into_count())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StockCountConfirmDiffAction {
    Recalculate,
    Noop,
}

fn ensure_count_editable(status: &str) -> anyhow::Result<()> {
    if status == "COUNTING" {
        Ok(())
    } else {
        Err(anyhow!("stock count not editable"))
    }
}

fn resolve_confirm_diff_action(status: &str) -> anyhow::Result<StockCountConfirmDiffAction> {
    match status {
        "COUNTING" => Ok(StockCountConfirmDiffAction::Recalculate),
        "DIFF_CONFIRMED" => Ok(StockCountConfirmDiffAction::Noop),
        "CANCELLED" => Err(anyhow!("stock count cancelled")),
        "APPLIED" => Err(anyhow!("stock count already applied")),
        _ => Err(anyhow!("stock count not ready")),
    }
}

fn ensure_cancel_allowed(status: &str) -> anyhow::Result<()> {
    if status == "COUNTING" || status == "DIFF_CONFIRMED" {
        Ok(())
    } else {
        Err(anyhow!("stock count cannot be cancelled in current status"))
    }
}

fn merge_adjustment_linkage(
    filters_json: Value,
    gain_adjust_id: Option<uuid::Uuid>,
    scrap_adjust_id: Option<uuid::Uuid>,
) -> Value {
    let mut root = match filters_json {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    let mut linkage = Map::new();
    if let Some(gain_id) = gain_adjust_id {
        linkage.insert(
            "gain_adjust_id".to_string(),
            Value::String(gain_id.to_string()),
        );
    }
    if let Some(scrap_id) = scrap_adjust_id {
        linkage.insert(
            "scrap_adjust_id".to_string(),
            Value::String(scrap_id.to_string()),
        );
    }
    linkage.insert(
        "transfer_recheck_required".to_string(),
        Value::Bool(gain_adjust_id.is_some() && scrap_adjust_id.is_some()),
    );
    linkage.insert(
        "linked_at".to_string(),
        Value::String(Utc::now().to_rfc3339()),
    );
    root.insert("adjustment_linkage".to_string(), Value::Object(linkage));

    Value::Object(root)
}

fn apply_count_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a StockCountListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
    if let Some(count_type) = &filters.count_type {
        qb.push(" and filters_json ->> 'count_type' = ");
        qb.push_bind(count_type);
    }
    if let Some(scope_type) = &filters.scope_type {
        qb.push(" and filters_json ->> 'scope_type' = ");
        qb.push_bind(scope_type);
    }
    if let Some(keyword) = &filters.keyword {
        let like = format!("%{}%", keyword);
        qb.push(" and count_id::text ilike ");
        qb.push_bind(like);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and created_at <= ");
        qb.push_bind(created_to);
    }
}

#[derive(sqlx::FromRow)]
struct StockCountRow {
    count_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    status: String,
    strategy: String,
    dimension: String,
    filters_json: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockCountRow {
    fn into_count(self) -> StockCount {
        StockCount {
            count_id: self.count_id,
            tenant_id: TenantId(self.tenant_id),
            warehouse_id: self.warehouse_id,
            status: self.status,
            strategy: self.strategy,
            dimension: self.dimension,
            filters_json: self.filters_json,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StockCountItemRow {
    count_item_id: uuid::Uuid,
    count_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    location_id: Option<uuid::Uuid>,
    system_qty: i64,
    counted_qty: i64,
    diff_qty: i64,
    diff_reason: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockCountItemRow {
    fn into_item(self) -> StockCountItem {
        StockCountItem {
            count_item_id: self.count_item_id,
            count_id: self.count_id,
            sku_id: self.sku_id,
            location_id: self.location_id,
            system_qty: self.system_qty,
            counted_qty: self.counted_qty,
            diff_qty: self.diff_qty,
            diff_reason: self.diff_reason,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_confirm_diff_action_respects_closed_loop() {
        assert!(matches!(
            resolve_confirm_diff_action("COUNTING").expect("counting should continue"),
            StockCountConfirmDiffAction::Recalculate
        ));
        assert!(matches!(
            resolve_confirm_diff_action("DIFF_CONFIRMED").expect("confirmed should be idempotent"),
            StockCountConfirmDiffAction::Noop
        ));
        assert!(resolve_confirm_diff_action("APPLIED").is_err());
        assert!(resolve_confirm_diff_action("CANCELLED").is_err());
    }

    #[test]
    fn ensure_count_editable_only_for_counting() {
        ensure_count_editable("COUNTING").expect("counting should be editable");
        assert!(ensure_count_editable("DIFF_CONFIRMED").is_err());
        assert!(ensure_count_editable("APPLIED").is_err());
    }

    #[test]
    fn ensure_cancel_allowed_for_counting_or_diff_confirmed() {
        ensure_cancel_allowed("COUNTING").expect("counting can cancel");
        ensure_cancel_allowed("DIFF_CONFIRMED").expect("diff confirmed can cancel");
        assert!(ensure_cancel_allowed("APPLIED").is_err());
    }

    #[test]
    fn merge_adjustment_linkage_sets_transfer_recheck_flag() {
        let merged = merge_adjustment_linkage(
            json!({"scope_type":"FULL"}),
            Some(uuid::Uuid::nil()),
            Some(uuid::Uuid::max()),
        );
        let linkage = merged
            .get("adjustment_linkage")
            .expect("should include linkage");
        assert_eq!(
            linkage.get("transfer_recheck_required"),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            linkage.get("gain_adjust_id"),
            Some(&Value::String(uuid::Uuid::nil().to_string()))
        );
    }
}
