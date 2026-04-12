use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use super::models::*;

pub struct ProcurementRequestsStore<'a> {
    pool: &'a PgPool,
}

fn apply_procurement_request_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProcurementRequestListFilters,
) {
    if let Some(department) = filters
        .department
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        qb.push(" and pr.department ilike ");
        qb.push_bind(format!("%{}%", department));
    }
    if let Some(requester) = filters
        .requester
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        qb.push(" and pr.requester ilike ");
        qb.push_bind(format!("%{}%", requester));
    }
    if let Some(status) = &filters.status {
        qb.push(" and pr.status = ");
        qb.push_bind(status);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and pr.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and pr.created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_procurement_plan_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProcurementPlanListFilters,
) {
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and pp.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and pp.status = ");
        qb.push_bind(status);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and pp.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and pp.created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_procurement_sourcing_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProcurementSourcingListFilters,
) {
    if let Some(keyword) = filters
        .keyword
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        qb.push(" and (ps.title ilike ");
        qb.push_bind(format!("%{}%", keyword));
        qb.push(" or ps.category ilike ");
        qb.push_bind(format!("%{}%", keyword));
        qb.push(")");
    }
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and ps.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and ps.status = ");
        qb.push_bind(status);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and ps.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and ps.created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_procurement_inspection_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProcurementInspectionListFilters,
) {
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and pi.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and pi.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and pi.status = ");
        qb.push_bind(status);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and pi.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and pi.created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_procurement_reconciliation_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProcurementReconciliationListFilters,
) {
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and pr.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and pr.status = ");
        qb.push_bind(status);
    }
    if let Some(period_from) = filters.period_from {
        qb.push(" and pr.period_start >= ");
        qb.push_bind(period_from);
    }
    if let Some(period_to) = filters.period_to {
        qb.push(" and pr.period_end <= ");
        qb.push_bind(period_to);
    }
}

fn apply_supplier_performance_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a SupplierPerformanceListFilters,
) {
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and sp.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and sp.status = ");
        qb.push_bind(status);
    }
    if let Some(period_from) = filters.period_from {
        qb.push(" and sp.period_start >= ");
        qb.push_bind(period_from);
    }
    if let Some(period_to) = filters.period_to {
        qb.push(" and sp.period_end <= ");
        qb.push_bind(period_to);
    }
}

async fn insert_request_items(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    request_id: uuid::Uuid,
    items: &[ProcurementRequestItemInput],
) -> anyhow::Result<()> {
    for item in items {
        if item.qty <= 0 {
            return Err(anyhow!("qty must be positive"));
        }
        if item.price_cent < 0 {
            return Err(anyhow!("price must be non-negative"));
        }
        sqlx::query(
            r#"
            insert into procurement_request_items(
                tenant_id, request_id, sku_id, qty, price_cent
            ) values ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(request_id)
        .bind(item.sku_id)
        .bind(item.qty)
        .bind(item.price_cent)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn fetch_request_items(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    request_id: uuid::Uuid,
) -> anyhow::Result<Vec<ProcurementRequestItemInput>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        sku_id: uuid::Uuid,
        qty: i64,
        price_cent: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select sku_id, qty, price_cent
        from procurement_request_items
        where tenant_id = $1 and request_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(request_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ProcurementRequestItemInput {
            sku_id: row.sku_id,
            qty: row.qty,
            price_cent: row.price_cent,
        })
        .collect())
}

async fn insert_plan_items(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    plan_id: uuid::Uuid,
    items: &[ProcurementPlanItemInput],
) -> anyhow::Result<()> {
    for item in items {
        if item.qty <= 0 {
            return Err(anyhow!("qty must be positive"));
        }
        if item.price_cent < 0 {
            return Err(anyhow!("price must be non-negative"));
        }
        sqlx::query(
            r#"
            insert into procurement_plan_items(
                tenant_id, plan_id, sku_id, qty, price_cent
            ) values ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(plan_id)
        .bind(item.sku_id)
        .bind(item.qty)
        .bind(item.price_cent)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn fetch_plan_items(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    plan_id: uuid::Uuid,
) -> anyhow::Result<Vec<ProcurementPlanItemInput>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        sku_id: uuid::Uuid,
        qty: i64,
        price_cent: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select sku_id, qty, price_cent
        from procurement_plan_items
        where tenant_id = $1 and plan_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(plan_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ProcurementPlanItemInput {
            sku_id: row.sku_id,
            qty: row.qty,
            price_cent: row.price_cent,
        })
        .collect())
}

async fn insert_inspection_items(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    inspection_id: uuid::Uuid,
    items: &[ProcurementInspectionItemInput],
) -> anyhow::Result<()> {
    for item in items {
        if item.qty <= 0 {
            return Err(anyhow!("qty must be positive"));
        }
        if item.passed_qty < 0 || item.failed_qty < 0 {
            return Err(anyhow!("inspection qty must be non-negative"));
        }
        if item.passed_qty + item.failed_qty > item.qty {
            return Err(anyhow!("inspection qty exceeds total qty"));
        }
        sqlx::query(
            r#"
            insert into procurement_inspection_items(
                tenant_id, inspection_id, sku_id, qty, passed_qty, failed_qty, result
            ) values ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inspection_id)
        .bind(item.sku_id)
        .bind(item.qty)
        .bind(item.passed_qty)
        .bind(item.failed_qty)
        .bind(item.result.as_deref())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

fn compute_item_total<I>(items: I) -> anyhow::Result<i64>
where
    I: IntoIterator<Item = (i64, i64)>,
{
    let mut total: i64 = 0;
    for (qty, price_cent) in items {
        let line_total = qty
            .checked_mul(price_cent)
            .ok_or_else(|| anyhow!("total overflow"))?;
        total = total
            .checked_add(line_total)
            .ok_or_else(|| anyhow!("total overflow"))?;
    }
    Ok(total)
}

fn compute_performance_score(delivery: i32, quality: i32, price: i32) -> i32 {
    (delivery + quality + price) / 3
}

pub struct ProcurementReconciliationsStore<'a> {
    pool: &'a PgPool,
}

impl<'a> ProcurementReconciliationsStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_reconciliations(
        &self,
        tenant_id: TenantId,
        filters: ProcurementReconciliationListFilters,
    ) -> anyhow::Result<ProcurementReconciliationListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from procurement_reconciliations pr
            where pr.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_reconciliation_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pr.reconciliation_id,
                pr.supplier_id,
                s.name as supplier_name,
                pr.period_start,
                pr.period_end,
                pr.amount_cent,
                pr.status,
                pr.remark,
                pr.created_at,
                pr.updated_at
            from procurement_reconciliations pr
            join suppliers s
              on s.supplier_id = pr.supplier_id
             and s.tenant_id = pr.tenant_id
            where pr.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_reconciliation_filters(&mut qb, &filters);
        qb.push(" order by pr.created_at desc, pr.reconciliation_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            reconciliation_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            period_start: chrono::NaiveDate,
            period_end: chrono::NaiveDate,
            amount_cent: i64,
            status: String,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProcurementReconciliation {
                reconciliation_id: row.reconciliation_id,
                tenant_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                period_start: row.period_start,
                period_end: row.period_end,
                amount_cent: row.amount_cent,
                status: row.status,
                remark: row.remark,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProcurementReconciliationListResult { items, total })
    }

    pub async fn create_reconciliation(
        &self,
        input: ProcurementReconciliationCreate,
    ) -> anyhow::Result<ProcurementReconciliation> {
        let status = input
            .status
            .unwrap_or_else(|| PROCUREMENT_RECONCILIATION_STATUS_OPEN.to_string());
        let reconciliation_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            insert into procurement_reconciliations(
                reconciliation_id, tenant_id, supplier_id, period_start,
                period_end, amount_cent, status, remark
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(reconciliation_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.supplier_id)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(input.amount_cent)
        .bind(&status)
        .bind(input.remark.as_deref())
        .execute(self.pool)
        .await?;

        Ok(ProcurementReconciliation {
            reconciliation_id,
            tenant_id: input.tenant_id,
            supplier_id: input.supplier_id,
            supplier_name: None,
            period_start: input.period_start,
            period_end: input.period_end,
            amount_cent: input.amount_cent,
            status,
            remark: input.remark,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub async fn update_reconciliation(
        &self,
        tenant_id: TenantId,
        reconciliation_id: uuid::Uuid,
        input: ProcurementReconciliationUpdate,
    ) -> anyhow::Result<Option<ProcurementReconciliation>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            reconciliation_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            period_start: chrono::NaiveDate,
            period_end: chrono::NaiveDate,
            amount_cent: i64,
            status: String,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update procurement_reconciliations pr
            set supplier_id = coalesce($3, pr.supplier_id),
                period_start = coalesce($4, pr.period_start),
                period_end = coalesce($5, pr.period_end),
                amount_cent = coalesce($6, pr.amount_cent),
                status = coalesce($7, pr.status),
                remark = coalesce($8, pr.remark),
                updated_at = now()
            from suppliers s
            where pr.tenant_id = $1
              and pr.reconciliation_id = $2
              and s.supplier_id = pr.supplier_id
              and s.tenant_id = pr.tenant_id
            returning pr.reconciliation_id, pr.supplier_id, s.name as supplier_name,
                      pr.period_start, pr.period_end, pr.amount_cent, pr.status,
                      pr.remark, pr.created_at, pr.updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(reconciliation_id)
        .bind(input.supplier_id)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(input.amount_cent)
        .bind(input.status)
        .bind(input.remark)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProcurementReconciliation {
            reconciliation_id: row.reconciliation_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            period_start: row.period_start,
            period_end: row.period_end,
            amount_cent: row.amount_cent,
            status: row.status,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_reconciliation(
        &self,
        tenant_id: TenantId,
        reconciliation_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementReconciliation>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            reconciliation_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            period_start: chrono::NaiveDate,
            period_end: chrono::NaiveDate,
            amount_cent: i64,
            status: String,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from procurement_reconciliations pr
            using suppliers s
            where pr.tenant_id = $1
              and pr.reconciliation_id = $2
              and s.supplier_id = pr.supplier_id
              and s.tenant_id = pr.tenant_id
            returning pr.reconciliation_id, pr.supplier_id, s.name as supplier_name,
                      pr.period_start, pr.period_end, pr.amount_cent, pr.status,
                      pr.remark, pr.created_at, pr.updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(reconciliation_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProcurementReconciliation {
            reconciliation_id: row.reconciliation_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            period_start: row.period_start,
            period_end: row.period_end,
            amount_cent: row.amount_cent,
            status: row.status,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

pub struct SupplierPerformanceStore<'a> {
    pool: &'a PgPool,
}

impl<'a> SupplierPerformanceStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_performance(
        &self,
        tenant_id: TenantId,
        filters: SupplierPerformanceListFilters,
    ) -> anyhow::Result<SupplierPerformanceListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from supplier_performances sp
            where sp.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_supplier_performance_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                sp.performance_id,
                sp.supplier_id,
                s.name as supplier_name,
                sp.period_start,
                sp.period_end,
                sp.delivery_score,
                sp.quality_score,
                sp.price_score,
                sp.total_score,
                sp.status,
                sp.remark,
                sp.created_at,
                sp.updated_at
            from supplier_performances sp
            join suppliers s
              on s.supplier_id = sp.supplier_id
             and s.tenant_id = sp.tenant_id
            where sp.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_supplier_performance_filters(&mut qb, &filters);
        qb.push(" order by sp.created_at desc, sp.performance_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            performance_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            period_start: chrono::NaiveDate,
            period_end: chrono::NaiveDate,
            delivery_score: i32,
            quality_score: i32,
            price_score: i32,
            total_score: i32,
            status: String,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| SupplierPerformance {
                performance_id: row.performance_id,
                tenant_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                period_start: row.period_start,
                period_end: row.period_end,
                delivery_score: row.delivery_score,
                quality_score: row.quality_score,
                price_score: row.price_score,
                total_score: row.total_score,
                status: row.status,
                remark: row.remark,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SupplierPerformanceListResult { items, total })
    }

    pub async fn create_performance(
        &self,
        input: SupplierPerformanceCreate,
    ) -> anyhow::Result<SupplierPerformance> {
        let status = input
            .status
            .unwrap_or_else(|| SUPPLIER_PERFORMANCE_STATUS_ACTIVE.to_string());
        let total_score =
            compute_performance_score(input.delivery_score, input.quality_score, input.price_score);
        let performance_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            insert into supplier_performances(
                performance_id, tenant_id, supplier_id, period_start, period_end,
                delivery_score, quality_score, price_score, total_score, status, remark
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(performance_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.supplier_id)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(input.delivery_score)
        .bind(input.quality_score)
        .bind(input.price_score)
        .bind(total_score)
        .bind(&status)
        .bind(input.remark.as_deref())
        .execute(self.pool)
        .await?;

        Ok(SupplierPerformance {
            performance_id,
            tenant_id: input.tenant_id,
            supplier_id: input.supplier_id,
            supplier_name: None,
            period_start: input.period_start,
            period_end: input.period_end,
            delivery_score: input.delivery_score,
            quality_score: input.quality_score,
            price_score: input.price_score,
            total_score,
            status,
            remark: input.remark,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub async fn update_performance(
        &self,
        tenant_id: TenantId,
        performance_id: uuid::Uuid,
        input: SupplierPerformanceUpdate,
    ) -> anyhow::Result<Option<SupplierPerformance>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            performance_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            period_start: chrono::NaiveDate,
            period_end: chrono::NaiveDate,
            delivery_score: i32,
            quality_score: i32,
            price_score: i32,
            total_score: i32,
            status: String,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let updated_scores = if input.delivery_score.is_some()
            || input.quality_score.is_some()
            || input.price_score.is_some()
        {
            Some((
                input.delivery_score.unwrap_or(0),
                input.quality_score.unwrap_or(0),
                input.price_score.unwrap_or(0),
            ))
        } else {
            None
        };

        let total_score = updated_scores
            .map(|(delivery, quality, price)| compute_performance_score(delivery, quality, price));

        let row: Option<Row> = sqlx::query_as(
            r#"
            update supplier_performances sp
            set supplier_id = coalesce($3, sp.supplier_id),
                period_start = coalesce($4, sp.period_start),
                period_end = coalesce($5, sp.period_end),
                delivery_score = coalesce($6, sp.delivery_score),
                quality_score = coalesce($7, sp.quality_score),
                price_score = coalesce($8, sp.price_score),
                total_score = coalesce($9, sp.total_score),
                status = coalesce($10, sp.status),
                remark = coalesce($11, sp.remark),
                updated_at = now()
            from suppliers s
            where sp.tenant_id = $1
              and sp.performance_id = $2
              and s.supplier_id = sp.supplier_id
              and s.tenant_id = sp.tenant_id
            returning sp.performance_id, sp.supplier_id, s.name as supplier_name,
                      sp.period_start, sp.period_end, sp.delivery_score,
                      sp.quality_score, sp.price_score, sp.total_score,
                      sp.status, sp.remark, sp.created_at, sp.updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(performance_id)
        .bind(input.supplier_id)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(input.delivery_score)
        .bind(input.quality_score)
        .bind(input.price_score)
        .bind(total_score)
        .bind(input.status)
        .bind(input.remark)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SupplierPerformance {
            performance_id: row.performance_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            period_start: row.period_start,
            period_end: row.period_end,
            delivery_score: row.delivery_score,
            quality_score: row.quality_score,
            price_score: row.price_score,
            total_score: row.total_score,
            status: row.status,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_performance(
        &self,
        tenant_id: TenantId,
        performance_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SupplierPerformance>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            performance_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            period_start: chrono::NaiveDate,
            period_end: chrono::NaiveDate,
            delivery_score: i32,
            quality_score: i32,
            price_score: i32,
            total_score: i32,
            status: String,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from supplier_performances sp
            using suppliers s
            where sp.tenant_id = $1
              and sp.performance_id = $2
              and s.supplier_id = sp.supplier_id
              and s.tenant_id = sp.tenant_id
            returning sp.performance_id, sp.supplier_id, s.name as supplier_name,
                      sp.period_start, sp.period_end, sp.delivery_score,
                      sp.quality_score, sp.price_score, sp.total_score,
                      sp.status, sp.remark, sp.created_at, sp.updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(performance_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SupplierPerformance {
            performance_id: row.performance_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            period_start: row.period_start,
            period_end: row.period_end,
            delivery_score: row.delivery_score,
            quality_score: row.quality_score,
            price_score: row.price_score,
            total_score: row.total_score,
            status: row.status,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

pub struct ProcurementInspectionsStore<'a> {
    pool: &'a PgPool,
}

impl<'a> ProcurementInspectionsStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_inspections(
        &self,
        tenant_id: TenantId,
        filters: ProcurementInspectionListFilters,
    ) -> anyhow::Result<ProcurementInspectionListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from procurement_inspections pi
            where pi.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_inspection_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pi.inspection_id,
                pi.receipt_id,
                pi.supplier_id,
                s.name as supplier_name,
                pi.warehouse_id,
                pi.status,
                pi.inspected_at,
                pi.remark,
                pi.created_at,
                pi.updated_at
            from procurement_inspections pi
            left join suppliers s
              on s.supplier_id = pi.supplier_id
             and s.tenant_id = pi.tenant_id
            where pi.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_inspection_filters(&mut qb, &filters);
        qb.push(" order by pi.created_at desc, pi.inspection_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            inspection_id: uuid::Uuid,
            receipt_id: Option<uuid::Uuid>,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            warehouse_id: Option<uuid::Uuid>,
            status: String,
            inspected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProcurementInspection {
                inspection_id: row.inspection_id,
                tenant_id,
                receipt_id: row.receipt_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                warehouse_id: row.warehouse_id,
                status: row.status,
                inspected_at: row.inspected_at,
                remark: row.remark,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProcurementInspectionListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        inspection_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementInspectionDetail>> {
        let inspection = self
            .get_inspection(tenant_id, inspection_id, self.pool)
            .await?;
        let Some(inspection) = inspection else {
            return Ok(None);
        };

        let items: Vec<ProcurementInspectionItem> = sqlx::query_as(
            r#"
            select i.sku_id,
                   s.sku_code,
                   i.qty,
                   i.passed_qty,
                   i.failed_qty,
                   i.result
            from procurement_inspection_items i
            left join skus s
              on s.sku_id = i.sku_id
             and s.tenant_id = i.tenant_id
            where i.tenant_id = $1 and i.inspection_id = $2
            order by i.sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inspection_id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(ProcurementInspectionDetail { inspection, items }))
    }

    pub async fn create_inspection(
        &self,
        input: ProcurementInspectionCreate,
    ) -> anyhow::Result<ProcurementInspectionDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("inspection items required"));
        }
        let inspection_id = uuid::Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            insert into procurement_inspections(
                inspection_id, tenant_id, receipt_id, supplier_id, warehouse_id,
                status, inspected_at, remark
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(inspection_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.receipt_id)
        .bind(input.supplier_id)
        .bind(input.warehouse_id)
        .bind(PROCUREMENT_INSPECTION_STATUS_PENDING)
        .bind(input.inspected_at)
        .bind(input.remark.as_deref())
        .execute(&mut *tx)
        .await?;

        insert_inspection_items(&mut tx, input.tenant_id, inspection_id, &input.items).await?;
        tx.commit().await?;
        self.get_detail(input.tenant_id, inspection_id)
            .await?
            .ok_or_else(|| anyhow!("procurement inspection not found"))
    }

    pub async fn update_inspection(
        &self,
        tenant_id: TenantId,
        inspection_id: uuid::Uuid,
        input: ProcurementInspectionUpdate,
    ) -> anyhow::Result<Option<ProcurementInspectionDetail>> {
        let mut tx = self.pool.begin().await?;
        #[derive(sqlx::FromRow)]
        struct Row {
            status: String,
            inspected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select status, inspected_at, remark
            from procurement_inspections
            where tenant_id = $1 and inspection_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inspection_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let status = input.status.unwrap_or(row.status);
        let inspected_at = input.inspected_at.or(row.inspected_at);
        let remark = input.remark.or(row.remark);

        if let Some(items) = input.items {
            if items.is_empty() {
                return Err(anyhow!("inspection items required"));
            }
            sqlx::query(
                r#"
                delete from procurement_inspection_items
                where tenant_id = $1 and inspection_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(inspection_id)
            .execute(&mut *tx)
            .await?;
            insert_inspection_items(&mut tx, tenant_id, inspection_id, &items).await?;
        }

        sqlx::query(
            r#"
            update procurement_inspections
            set status = $3,
                inspected_at = $4,
                remark = $5,
                updated_at = now()
            where tenant_id = $1 and inspection_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inspection_id)
        .bind(status)
        .bind(inspected_at)
        .bind(remark.as_deref())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_detail(tenant_id, inspection_id).await
    }

    pub async fn delete_inspection(
        &self,
        tenant_id: TenantId,
        inspection_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementInspection>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            inspection_id: uuid::Uuid,
            receipt_id: Option<uuid::Uuid>,
            supplier_id: Option<uuid::Uuid>,
            warehouse_id: Option<uuid::Uuid>,
            status: String,
            inspected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            supplier_name: Option<String>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from procurement_inspections
            where tenant_id = $1 and inspection_id = $2
            returning inspection_id,
                      receipt_id,
                      supplier_id,
                      warehouse_id,
                      status,
                      inspected_at,
                      remark,
                      created_at,
                      updated_at,
                      (
                        select name
                        from suppliers s
                        where s.supplier_id = procurement_inspections.supplier_id
                          and s.tenant_id = procurement_inspections.tenant_id
                      ) as supplier_name
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inspection_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProcurementInspection {
            inspection_id: row.inspection_id,
            tenant_id,
            receipt_id: row.receipt_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            warehouse_id: row.warehouse_id,
            status: row.status,
            inspected_at: row.inspected_at,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn get_inspection<'e, E>(
        &self,
        tenant_id: TenantId,
        inspection_id: uuid::Uuid,
        executor: E,
    ) -> anyhow::Result<Option<ProcurementInspection>>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        #[derive(sqlx::FromRow)]
        struct Row {
            inspection_id: uuid::Uuid,
            receipt_id: Option<uuid::Uuid>,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            warehouse_id: Option<uuid::Uuid>,
            status: String,
            inspected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select pi.inspection_id,
                   pi.receipt_id,
                   pi.supplier_id,
                   s.name as supplier_name,
                   pi.warehouse_id,
                   pi.status,
                   pi.inspected_at,
                   pi.remark,
                   pi.created_at,
                   pi.updated_at
            from procurement_inspections pi
            left join suppliers s
              on s.supplier_id = pi.supplier_id
             and s.tenant_id = pi.tenant_id
            where pi.tenant_id = $1 and pi.inspection_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(inspection_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.map(|row| ProcurementInspection {
            inspection_id: row.inspection_id,
            tenant_id,
            receipt_id: row.receipt_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            warehouse_id: row.warehouse_id,
            status: row.status,
            inspected_at: row.inspected_at,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

pub struct ProcurementSourcingStore<'a> {
    pool: &'a PgPool,
}

impl<'a> ProcurementSourcingStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_sourcing(
        &self,
        tenant_id: TenantId,
        filters: ProcurementSourcingListFilters,
    ) -> anyhow::Result<ProcurementSourcingListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from procurement_sourcing ps
            where ps.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_sourcing_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                ps.sourcing_id,
                ps.title,
                ps.category,
                ps.supplier_id,
                s.name as supplier_name,
                ps.status,
                ps.target_price_cent,
                ps.remark,
                ps.created_at,
                ps.updated_at
            from procurement_sourcing ps
            left join suppliers s
              on s.supplier_id = ps.supplier_id
             and s.tenant_id = ps.tenant_id
            where ps.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_sourcing_filters(&mut qb, &filters);
        qb.push(" order by ps.created_at desc, ps.sourcing_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            sourcing_id: uuid::Uuid,
            title: String,
            category: Option<String>,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            status: String,
            target_price_cent: i64,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProcurementSourcing {
                sourcing_id: row.sourcing_id,
                tenant_id,
                title: row.title,
                category: row.category,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                status: row.status,
                target_price_cent: row.target_price_cent,
                remark: row.remark,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProcurementSourcingListResult { items, total })
    }

    pub async fn create_sourcing(
        &self,
        input: ProcurementSourcingCreate,
    ) -> anyhow::Result<ProcurementSourcing> {
        let status = input
            .status
            .unwrap_or_else(|| PROCUREMENT_SOURCING_STATUS_OPEN.to_string());
        let sourcing_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            insert into procurement_sourcing(
                sourcing_id, tenant_id, title, category, supplier_id, status, target_price_cent, remark
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(sourcing_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.title)
        .bind(input.category.as_deref())
        .bind(input.supplier_id)
        .bind(&status)
        .bind(input.target_price_cent)
        .bind(input.remark.as_deref())
        .execute(self.pool)
        .await?;

        self.get_sourcing(input.tenant_id, sourcing_id)
            .await?
            .ok_or_else(|| anyhow!("procurement sourcing not found"))
    }

    pub async fn update_sourcing(
        &self,
        tenant_id: TenantId,
        sourcing_id: uuid::Uuid,
        input: ProcurementSourcingUpdate,
    ) -> anyhow::Result<Option<ProcurementSourcing>> {
        let result = sqlx::query(
            r#"
            update procurement_sourcing
            set title = coalesce($3, title),
                category = coalesce($4, category),
                supplier_id = coalesce($5, supplier_id),
                status = coalesce($6, status),
                target_price_cent = coalesce($7, target_price_cent),
                remark = coalesce($8, remark),
                updated_at = now()
            where tenant_id = $1 and sourcing_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sourcing_id)
        .bind(input.title)
        .bind(input.category)
        .bind(input.supplier_id)
        .bind(input.status)
        .bind(input.target_price_cent)
        .bind(input.remark)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_sourcing(tenant_id, sourcing_id).await
    }

    pub async fn delete_sourcing(
        &self,
        tenant_id: TenantId,
        sourcing_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementSourcing>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            sourcing_id: uuid::Uuid,
            title: String,
            category: Option<String>,
            supplier_id: Option<uuid::Uuid>,
            status: String,
            target_price_cent: i64,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            supplier_name: Option<String>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from procurement_sourcing
            where tenant_id = $1 and sourcing_id = $2
            returning sourcing_id,
                      title,
                      category,
                      supplier_id,
                      status,
                      target_price_cent,
                      remark,
                      created_at,
                      updated_at,
                      (
                        select name
                        from suppliers s
                        where s.supplier_id = procurement_sourcing.supplier_id
                          and s.tenant_id = procurement_sourcing.tenant_id
                      ) as supplier_name
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sourcing_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProcurementSourcing {
            sourcing_id: row.sourcing_id,
            tenant_id,
            title: row.title,
            category: row.category,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            status: row.status,
            target_price_cent: row.target_price_cent,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn get_sourcing(
        &self,
        tenant_id: TenantId,
        sourcing_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementSourcing>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            sourcing_id: uuid::Uuid,
            title: String,
            category: Option<String>,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            status: String,
            target_price_cent: i64,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select ps.sourcing_id,
                   ps.title,
                   ps.category,
                   ps.supplier_id,
                   s.name as supplier_name,
                   ps.status,
                   ps.target_price_cent,
                   ps.remark,
                   ps.created_at,
                   ps.updated_at
            from procurement_sourcing ps
            left join suppliers s
              on s.supplier_id = ps.supplier_id
             and s.tenant_id = ps.tenant_id
            where ps.tenant_id = $1 and ps.sourcing_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sourcing_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProcurementSourcing {
            sourcing_id: row.sourcing_id,
            tenant_id,
            title: row.title,
            category: row.category,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            status: row.status,
            target_price_cent: row.target_price_cent,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

pub struct ProcurementPlansStore<'a> {
    pool: &'a PgPool,
}

impl<'a> ProcurementPlansStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_plans(
        &self,
        tenant_id: TenantId,
        filters: ProcurementPlanListFilters,
    ) -> anyhow::Result<ProcurementPlanListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from procurement_plans pp
            where pp.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_plan_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pp.plan_id,
                pp.request_id,
                pp.supplier_id,
                s.name as supplier_name,
                pp.status,
                pp.total_cent,
                pp.planned_at,
                pp.expected_at,
                pp.remark,
                pp.created_at,
                pp.updated_at
            from procurement_plans pp
            left join suppliers s
              on s.supplier_id = pp.supplier_id
             and s.tenant_id = pp.tenant_id
            where pp.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_plan_filters(&mut qb, &filters);
        qb.push(" order by pp.created_at desc, pp.plan_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            plan_id: uuid::Uuid,
            request_id: Option<uuid::Uuid>,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            status: String,
            total_cent: i64,
            planned_at: Option<DateTime<Utc>>,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProcurementPlan {
                plan_id: row.plan_id,
                tenant_id,
                request_id: row.request_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                status: row.status,
                total_cent: row.total_cent,
                planned_at: row.planned_at,
                expected_at: row.expected_at,
                remark: row.remark,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProcurementPlanListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementPlanDetail>> {
        let plan = self.get_plan(tenant_id, plan_id, self.pool).await?;
        let Some(plan) = plan else {
            return Ok(None);
        };

        let items: Vec<ProcurementPlanItem> = sqlx::query_as(
            r#"
            select i.sku_id, s.sku_code, i.qty, i.price_cent
            from procurement_plan_items i
            left join skus s
              on s.sku_id = i.sku_id
             and s.tenant_id = i.tenant_id
            where i.tenant_id = $1 and i.plan_id = $2
            order by i.sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(plan_id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(ProcurementPlanDetail { plan, items }))
    }

    pub async fn create_plan(
        &self,
        input: ProcurementPlanCreate,
    ) -> anyhow::Result<ProcurementPlanDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("plan items required"));
        }
        let total_cent =
            compute_item_total(input.items.iter().map(|item| (item.qty, item.price_cent)))?;
        let plan_id = uuid::Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            insert into procurement_plans(
                plan_id, tenant_id, request_id, supplier_id, status, total_cent,
                planned_at, expected_at, remark
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(plan_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.request_id)
        .bind(input.supplier_id)
        .bind(PROCUREMENT_PLAN_STATUS_DRAFT)
        .bind(total_cent)
        .bind(input.planned_at)
        .bind(input.expected_at)
        .bind(input.remark.as_deref())
        .execute(&mut *tx)
        .await?;

        insert_plan_items(&mut tx, input.tenant_id, plan_id, &input.items).await?;
        tx.commit().await?;
        self.get_detail(input.tenant_id, plan_id)
            .await?
            .ok_or_else(|| anyhow!("procurement plan not found"))
    }

    pub async fn update_plan(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
        input: ProcurementPlanUpdate,
    ) -> anyhow::Result<Option<ProcurementPlanDetail>> {
        let mut tx = self.pool.begin().await?;
        #[derive(sqlx::FromRow)]
        struct Row {
            supplier_id: Option<uuid::Uuid>,
            status: String,
            planned_at: Option<DateTime<Utc>>,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select supplier_id, status, planned_at, expected_at, remark
            from procurement_plans
            where tenant_id = $1 and plan_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(plan_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let supplier_id = input.supplier_id.or(row.supplier_id);
        let status = input.status.unwrap_or(row.status);
        let planned_at = input.planned_at.or(row.planned_at);
        let expected_at = input.expected_at.or(row.expected_at);
        let remark = input.remark.or(row.remark);

        let items = match input.items {
            Some(items) => {
                if items.is_empty() {
                    return Err(anyhow!("plan items required"));
                }
                sqlx::query(
                    r#"
                    delete from procurement_plan_items
                    where tenant_id = $1 and plan_id = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(plan_id)
                .execute(&mut *tx)
                .await?;
                insert_plan_items(&mut tx, tenant_id, plan_id, &items).await?;
                items
            }
            None => fetch_plan_items(&mut tx, tenant_id, plan_id).await?,
        };

        let total_cent = compute_item_total(items.iter().map(|item| (item.qty, item.price_cent)))?;
        sqlx::query(
            r#"
            update procurement_plans
            set supplier_id = $3,
                status = $4,
                total_cent = $5,
                planned_at = $6,
                expected_at = $7,
                remark = $8,
                updated_at = now()
            where tenant_id = $1 and plan_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(plan_id)
        .bind(supplier_id)
        .bind(status)
        .bind(total_cent)
        .bind(planned_at)
        .bind(expected_at)
        .bind(remark.as_deref())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_detail(tenant_id, plan_id).await
    }

    pub async fn delete_plan(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementPlan>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            plan_id: uuid::Uuid,
            request_id: Option<uuid::Uuid>,
            supplier_id: Option<uuid::Uuid>,
            status: String,
            total_cent: i64,
            planned_at: Option<DateTime<Utc>>,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from procurement_plans
            where tenant_id = $1 and plan_id = $2
            returning plan_id, request_id, supplier_id, status, total_cent,
                      planned_at, expected_at, remark, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(plan_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProcurementPlan {
            plan_id: row.plan_id,
            tenant_id,
            request_id: row.request_id,
            supplier_id: row.supplier_id,
            supplier_name: None,
            status: row.status,
            total_cent: row.total_cent,
            planned_at: row.planned_at,
            expected_at: row.expected_at,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn get_plan<'e, E>(
        &self,
        tenant_id: TenantId,
        plan_id: uuid::Uuid,
        executor: E,
    ) -> anyhow::Result<Option<ProcurementPlan>>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        #[derive(sqlx::FromRow)]
        struct Row {
            plan_id: uuid::Uuid,
            request_id: Option<uuid::Uuid>,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
            status: String,
            total_cent: i64,
            planned_at: Option<DateTime<Utc>>,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select pp.plan_id,
                   pp.request_id,
                   pp.supplier_id,
                   s.name as supplier_name,
                   pp.status,
                   pp.total_cent,
                   pp.planned_at,
                   pp.expected_at,
                   pp.remark,
                   pp.created_at,
                   pp.updated_at
            from procurement_plans pp
            left join suppliers s
              on s.supplier_id = pp.supplier_id
             and s.tenant_id = pp.tenant_id
            where pp.tenant_id = $1 and pp.plan_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(plan_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.map(|row| ProcurementPlan {
            plan_id: row.plan_id,
            tenant_id,
            request_id: row.request_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            status: row.status,
            total_cent: row.total_cent,
            planned_at: row.planned_at,
            expected_at: row.expected_at,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

impl<'a> ProcurementRequestsStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_requests(
        &self,
        tenant_id: TenantId,
        filters: ProcurementRequestListFilters,
    ) -> anyhow::Result<ProcurementRequestListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from procurement_requests pr
            where pr.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_request_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pr.request_id,
                pr.department,
                pr.requester,
                pr.status,
                pr.budget_cent,
                pr.expected_at,
                pr.remark,
                pr.created_at,
                pr.updated_at
            from procurement_requests pr
            where pr.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_procurement_request_filters(&mut qb, &filters);
        qb.push(" order by pr.created_at desc, pr.request_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            request_id: uuid::Uuid,
            department: Option<String>,
            requester: Option<String>,
            status: String,
            budget_cent: i64,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProcurementRequest {
                request_id: row.request_id,
                tenant_id,
                department: row.department,
                requester: row.requester,
                status: row.status,
                budget_cent: row.budget_cent,
                expected_at: row.expected_at,
                remark: row.remark,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProcurementRequestListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementRequestDetail>> {
        let request = self.get_request(tenant_id, request_id, self.pool).await?;
        let Some(request) = request else {
            return Ok(None);
        };

        let items: Vec<ProcurementRequestItem> = sqlx::query_as(
            r#"
            select i.sku_id, s.sku_code, i.qty, i.price_cent
            from procurement_request_items i
            left join skus s
              on s.sku_id = i.sku_id
             and s.tenant_id = i.tenant_id
            where i.tenant_id = $1 and i.request_id = $2
            order by i.sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(request_id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(ProcurementRequestDetail { request, items }))
    }

    pub async fn create_request(
        &self,
        input: ProcurementRequestCreate,
    ) -> anyhow::Result<ProcurementRequestDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("request items required"));
        }
        let budget_cent =
            compute_item_total(input.items.iter().map(|item| (item.qty, item.price_cent)))?;
        let request_id = uuid::Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            insert into procurement_requests(
                request_id, tenant_id, department, requester, status, budget_cent, expected_at, remark
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(request_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.department.as_deref())
        .bind(input.requester.as_deref())
        .bind(PROCUREMENT_REQUEST_STATUS_DRAFT)
        .bind(budget_cent)
        .bind(input.expected_at)
        .bind(input.remark.as_deref())
        .execute(&mut *tx)
        .await?;

        insert_request_items(&mut tx, input.tenant_id, request_id, &input.items).await?;
        tx.commit().await?;
        self.get_detail(input.tenant_id, request_id)
            .await?
            .ok_or_else(|| anyhow!("procurement request not found"))
    }

    pub async fn update_request(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
        input: ProcurementRequestUpdate,
    ) -> anyhow::Result<Option<ProcurementRequestDetail>> {
        let mut tx = self.pool.begin().await?;
        #[derive(sqlx::FromRow)]
        struct Row {
            department: Option<String>,
            requester: Option<String>,
            status: String,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select department, requester, status, expected_at, remark
            from procurement_requests
            where tenant_id = $1 and request_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(request_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let department = input.department.or(row.department);
        let requester = input.requester.or(row.requester);
        let status = input.status.unwrap_or(row.status);
        let expected_at = input.expected_at.or(row.expected_at);
        let remark = input.remark.or(row.remark);

        let items = match input.items {
            Some(items) => {
                if items.is_empty() {
                    return Err(anyhow!("request items required"));
                }
                sqlx::query(
                    r#"
                    delete from procurement_request_items
                    where tenant_id = $1 and request_id = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(request_id)
                .execute(&mut *tx)
                .await?;
                insert_request_items(&mut tx, tenant_id, request_id, &items).await?;
                items
            }
            None => fetch_request_items(&mut tx, tenant_id, request_id).await?,
        };

        let budget_cent = compute_item_total(items.iter().map(|item| (item.qty, item.price_cent)))?;
        sqlx::query(
            r#"
            update procurement_requests
            set department = $3,
                requester = $4,
                status = $5,
                budget_cent = $6,
                expected_at = $7,
                remark = $8,
                updated_at = now()
            where tenant_id = $1 and request_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(request_id)
        .bind(department.as_deref())
        .bind(requester.as_deref())
        .bind(status)
        .bind(budget_cent)
        .bind(expected_at)
        .bind(remark.as_deref())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_detail(tenant_id, request_id).await
    }

    pub async fn delete_request(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProcurementRequest>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            request_id: uuid::Uuid,
            department: Option<String>,
            requester: Option<String>,
            status: String,
            budget_cent: i64,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from procurement_requests
            where tenant_id = $1 and request_id = $2
            returning request_id, department, requester, status, budget_cent, expected_at,
                      remark, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(request_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProcurementRequest {
            request_id: row.request_id,
            tenant_id,
            department: row.department,
            requester: row.requester,
            status: row.status,
            budget_cent: row.budget_cent,
            expected_at: row.expected_at,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn get_request<'e, E>(
        &self,
        tenant_id: TenantId,
        request_id: uuid::Uuid,
        executor: E,
    ) -> anyhow::Result<Option<ProcurementRequest>>
    where
        E: sqlx::Executor<'e, Database = Postgres>,
    {
        #[derive(sqlx::FromRow)]
        struct Row {
            request_id: uuid::Uuid,
            department: Option<String>,
            requester: Option<String>,
            status: String,
            budget_cent: i64,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select request_id, department, requester, status, budget_cent, expected_at,
                   remark, created_at, updated_at
            from procurement_requests
            where tenant_id = $1 and request_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(request_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.map(|row| ProcurementRequest {
            request_id: row.request_id,
            tenant_id,
            department: row.department,
            requester: row.requester,
            status: row.status,
            budget_cent: row.budget_cent,
            expected_at: row.expected_at,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
