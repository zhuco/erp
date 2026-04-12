use chrono::{DateTime, Duration, Utc};
use erp_types::{Platform, ShopId, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone)]
pub struct DashboardReportFilters {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub platform: Option<Platform>,
    pub shop_id: Option<ShopId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetric {
    pub group_key: String,
    pub order_count: i64,
    pub gmv_cent: i64,
    pub refund_cent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardReport {
    pub by_shop: Vec<DashboardMetric>,
    pub by_platform: Vec<DashboardMetric>,
    pub by_date: Vec<DashboardMetric>,
}

#[derive(Debug, Clone)]
pub struct FulfillmentReportFilters {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub overdue_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillmentBucket {
    pub bucket: String,
    pub order_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillmentReport {
    pub buckets: Vec<FulfillmentBucket>,
    pub overdue_count: i64,
    pub overdue_hours: i64,
}

#[derive(Debug, Clone)]
pub struct InventoryReportFilters {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub warning_threshold: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTurnoverItem {
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub warehouse_id: uuid::Uuid,
    pub on_hand: i64,
    pub reserved: i64,
    pub available: i64,
    pub sold_qty: i64,
    pub turnover_days: Option<f64>,
    pub warning: bool,
    pub out_of_stock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReport {
    pub items: Vec<InventoryTurnoverItem>,
    pub total: i64,
    pub range_days: i64,
}

#[derive(Debug, Clone)]
pub struct FinanceReportFilters {
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub platform: Option<Platform>,
    pub shop_id: Option<ShopId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinanceReport {
    pub order_count: i64,
    pub gmv_cent: i64,
    pub items_amount_cent: i64,
    pub refund_cent: i64,
    pub discrepancy_cent: i64,
}

pub struct ReportsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ReportsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn dashboard_report(
        &self,
        tenant_id: TenantId,
        filters: DashboardReportFilters,
    ) -> anyhow::Result<DashboardReport> {
        let by_shop = self
            .fetch_grouped_metrics(tenant_id, &filters, "o.shop_id::text", "order_count desc")
            .await?;
        let by_platform = self
            .fetch_grouped_metrics(tenant_id, &filters, "o.platform", "order_count desc")
            .await?;
        let by_date = self
            .fetch_grouped_metrics(
                tenant_id,
                &filters,
                "to_char(date_trunc('day', o.order_created_at), 'YYYY-MM-DD')",
                "group_key asc",
            )
            .await?;

        Ok(DashboardReport {
            by_shop,
            by_platform,
            by_date,
        })
    }

    pub async fn fulfillment_report(
        &self,
        tenant_id: TenantId,
        filters: FulfillmentReportFilters,
    ) -> anyhow::Result<FulfillmentReport> {
        #[derive(sqlx::FromRow)]
        struct BucketRow {
            bucket: String,
            order_count: i64,
        }

        let rows: Vec<BucketRow> = sqlx::query_as(
            r#"
            select
                case
                    when extract(epoch from (o.ship_at - o.order_created_at)) / 3600 <= 6
                        then '0-6h'
                    when extract(epoch from (o.ship_at - o.order_created_at)) / 3600 <= 24
                        then '6-24h'
                    when extract(epoch from (o.ship_at - o.order_created_at)) / 3600 <= 48
                        then '24-48h'
                    when extract(epoch from (o.ship_at - o.order_created_at)) / 3600 <= 72
                        then '48-72h'
                    else '72h+'
                end as bucket,
                count(*) as order_count
            from orders o
            where o.tenant_id = $1
              and o.ship_at is not null
              and o.order_created_at >= $2
              and o.order_created_at <= $3
            group by bucket
            order by
                min(
                    case
                        when extract(epoch from (o.ship_at - o.order_created_at)) / 3600 <= 6
                            then 1
                        when extract(epoch from (o.ship_at - o.order_created_at)) / 3600 <= 24
                            then 2
                        when extract(epoch from (o.ship_at - o.order_created_at)) / 3600 <= 48
                            then 3
                        when extract(epoch from (o.ship_at - o.order_created_at)) / 3600 <= 72
                            then 4
                        else 5
                    end
                )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(filters.start_at)
        .bind(filters.end_at)
        .fetch_all(self.pool)
        .await?;

        let overdue_cutoff = Utc::now() - Duration::hours(filters.overdue_hours.max(1));
        let overdue_count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from orders o
            where o.tenant_id = $1
              and o.ship_at is null
              and o.order_created_at < $2
              and o.status not in ('CANCELLED', 'CLOSED', 'REFUNDED', 'REFUNDING')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(overdue_cutoff)
        .fetch_one(self.pool)
        .await?;

        Ok(FulfillmentReport {
            buckets: rows
                .into_iter()
                .map(|row| FulfillmentBucket {
                    bucket: row.bucket,
                    order_count: row.order_count,
                })
                .collect(),
            overdue_count,
            overdue_hours: filters.overdue_hours,
        })
    }

    pub async fn inventory_report(
        &self,
        tenant_id: TenantId,
        filters: InventoryReportFilters,
    ) -> anyhow::Result<InventoryReport> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from inventory_balances where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(warehouse_id) = filters.warehouse_id {
            count_qb.push(" and warehouse_id = ");
            count_qb.push_bind(warehouse_id);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            warehouse_id: uuid::Uuid,
            on_hand: i64,
            reserved: i64,
            sold_qty: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            with sales as (
                select oi.sku_id, sum(oi.qty)::bigint as sold_qty
                from order_items oi
                join orders o
                  on o.tenant_id = oi.tenant_id
                 and o.order_id = oi.order_id
                 and o.order_month = oi.order_month
                where o.tenant_id = $1
                  and o.order_created_at >= $2
                  and o.order_created_at <= $3
                group by oi.sku_id
            )
            select
                b.sku_id,
                s.sku_code,
                b.warehouse_id,
                b.on_hand,
                b.reserved,
                coalesce(sales.sold_qty, 0) as sold_qty
            from inventory_balances b
            join skus s
              on s.tenant_id = b.tenant_id
             and s.sku_id = b.sku_id
            left join sales on sales.sku_id = b.sku_id::text
            where b.tenant_id = $1
              and ($4::uuid is null or b.warehouse_id = $4)
            order by s.sku_code nulls last, b.sku_id
            limit $5 offset $6
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(filters.start_at)
        .bind(filters.end_at)
        .bind(filters.warehouse_id)
        .bind(filters.limit)
        .bind(filters.offset)
        .fetch_all(self.pool)
        .await?;

        let range_days = (filters.end_at - filters.start_at).num_days().max(1);

        let items = rows
            .into_iter()
            .map(|row| {
                let available = row.on_hand - row.reserved;
                let turnover_days = if row.sold_qty > 0 {
                    Some(range_days as f64 * row.on_hand as f64 / row.sold_qty as f64)
                } else {
                    None
                };
                let warning = available <= filters.warning_threshold;
                let out_of_stock = available <= 0;
                InventoryTurnoverItem {
                    sku_id: row.sku_id,
                    sku_code: row.sku_code,
                    warehouse_id: row.warehouse_id,
                    on_hand: row.on_hand,
                    reserved: row.reserved,
                    available,
                    sold_qty: row.sold_qty,
                    turnover_days,
                    warning,
                    out_of_stock,
                }
            })
            .collect();

        Ok(InventoryReport {
            items,
            total,
            range_days,
        })
    }

    pub async fn finance_report(
        &self,
        tenant_id: TenantId,
        filters: FinanceReportFilters,
    ) -> anyhow::Result<FinanceReport> {
        #[derive(sqlx::FromRow)]
        struct Row {
            order_count: i64,
            gmv_cent: i64,
            items_amount_cent: i64,
            refund_cent: i64,
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                count(distinct o.order_id) as order_count,
                coalesce(sum(o.total_amount_cent), 0) as gmv_cent,
                coalesce(sum(oi.amount_cent), 0) as items_amount_cent,
                coalesce(sum(oi.refund_amount_cent), 0) as refund_cent
            from orders o
            left join order_items oi
              on oi.tenant_id = o.tenant_id
             and oi.order_id = o.order_id
             and oi.order_month = o.order_month
            where o.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and o.order_created_at >= ");
        qb.push_bind(filters.start_at);
        qb.push(" and o.order_created_at <= ");
        qb.push_bind(filters.end_at);
        if let Some(platform) = filters.platform {
            qb.push(" and o.platform = ");
            qb.push_bind(platform.to_string());
        }
        if let Some(shop_id) = filters.shop_id {
            qb.push(" and o.shop_id = ");
            qb.push_bind(uuid::Uuid::from(shop_id));
        }

        let row: Row = qb.build_query_as().fetch_one(self.pool).await?;
        let discrepancy_cent = row.gmv_cent - row.items_amount_cent;

        Ok(FinanceReport {
            order_count: row.order_count,
            gmv_cent: row.gmv_cent,
            items_amount_cent: row.items_amount_cent,
            refund_cent: row.refund_cent,
            discrepancy_cent,
        })
    }

    async fn fetch_grouped_metrics(
        &self,
        tenant_id: TenantId,
        filters: &DashboardReportFilters,
        group_expr: &str,
        order_by: &str,
    ) -> anyhow::Result<Vec<DashboardMetric>> {
        let mut qb = QueryBuilder::<Postgres>::new("select ");
        qb.push(group_expr);
        qb.push(
            r#"
            as group_key,
            count(distinct o.order_id) as order_count,
            coalesce(sum(o.total_amount_cent), 0) as gmv_cent,
            coalesce(sum(oi.refund_amount_cent), 0) as refund_cent
            from orders o
            left join order_items oi
              on oi.tenant_id = o.tenant_id
             and oi.order_id = o.order_id
             and oi.order_month = o.order_month
            where o.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and o.order_created_at >= ");
        qb.push_bind(filters.start_at);
        qb.push(" and o.order_created_at <= ");
        qb.push_bind(filters.end_at);
        if let Some(platform) = filters.platform {
            qb.push(" and o.platform = ");
            qb.push_bind(platform.to_string());
        }
        if let Some(shop_id) = filters.shop_id {
            qb.push(" and o.shop_id = ");
            qb.push_bind(uuid::Uuid::from(shop_id));
        }
        qb.push(" group by group_key");
        qb.push(" order by ");
        qb.push(order_by);

        #[derive(sqlx::FromRow)]
        struct Row {
            group_key: String,
            order_count: i64,
            gmv_cent: i64,
            refund_cent: i64,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| DashboardMetric {
                group_key: row.group_key,
                order_count: row.order_count,
                gmv_cent: row.gmv_cent,
                refund_cent: row.refund_cent,
            })
            .collect())
    }
}
