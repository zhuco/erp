use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone)]
pub struct AfterSalesInspection {
    pub inspection_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub warehouse_id: Option<uuid::Uuid>,
    pub qty: i64,
    pub result_code: String,
    pub inspector_name: Option<String>,
    pub inspected_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AfterSalesInspectionListFilters {
    pub after_sales_id: Option<uuid::Uuid>,
    pub sku_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub result_code: Option<String>,
    pub inspected_from: Option<DateTime<Utc>>,
    pub inspected_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct AfterSalesInspectionListResult {
    pub items: Vec<AfterSalesInspection>,
    pub total: i64,
}

pub struct AfterSalesInspectionCreate {
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub warehouse_id: Option<uuid::Uuid>,
    pub qty: i64,
    pub result_code: String,
    pub inspector_name: Option<String>,
    pub inspected_at: Option<DateTime<Utc>>,
}

pub struct AfterSalesInspectionUpdate {
    pub warehouse_id: Option<uuid::Uuid>,
    pub qty: Option<i64>,
    pub result_code: Option<String>,
    pub inspector_name: Option<String>,
    pub inspected_at: Option<DateTime<Utc>>,
}

pub struct AfterSalesInspectionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AfterSalesInspectionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_inspections(
        &self,
        tenant_id: TenantId,
        filters: AfterSalesInspectionListFilters,
    ) -> anyhow::Result<AfterSalesInspectionListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                inspection_id, after_sales_id, sku_id, warehouse_id,
                qty, result_code, inspector_name, inspected_at,
                created_at, updated_at
            from after_sales_inspections
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(after_sales_id) = filters.after_sales_id {
            qb.push(" and after_sales_id = ");
            qb.push_bind(after_sales_id);
        }
        if let Some(sku_id) = filters.sku_id {
            qb.push(" and sku_id = ");
            qb.push_bind(sku_id);
        }
        if let Some(warehouse_id) = filters.warehouse_id {
            qb.push(" and warehouse_id = ");
            qb.push_bind(warehouse_id);
        }
        if let Some(result_code) = filters.result_code.as_ref() {
            qb.push(" and result_code = ");
            qb.push_bind(result_code);
        }
        if let Some(from) = filters.inspected_from {
            qb.push(" and inspected_at >= ");
            qb.push_bind(from);
        }
        if let Some(to) = filters.inspected_to {
            qb.push(" and inspected_at <= ");
            qb.push_bind(to);
        }
        qb.push(" order by inspected_at desc nulls last, created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<AfterSalesInspectionRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_inspection(tenant_id))
            .collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from after_sales_inspections where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(after_sales_id) = filters.after_sales_id {
            count_qb.push(" and after_sales_id = ");
            count_qb.push_bind(after_sales_id);
        }
        if let Some(sku_id) = filters.sku_id {
            count_qb.push(" and sku_id = ");
            count_qb.push_bind(sku_id);
        }
        if let Some(warehouse_id) = filters.warehouse_id {
            count_qb.push(" and warehouse_id = ");
            count_qb.push_bind(warehouse_id);
        }
        if let Some(result_code) = filters.result_code.as_ref() {
            count_qb.push(" and result_code = ");
            count_qb.push_bind(result_code);
        }
        if let Some(from) = filters.inspected_from {
            count_qb.push(" and inspected_at >= ");
            count_qb.push_bind(from);
        }
        if let Some(to) = filters.inspected_to {
            count_qb.push(" and inspected_at <= ");
            count_qb.push_bind(to);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(AfterSalesInspectionListResult { items, total })
    }

    pub async fn create_inspection(
        &self,
        input: AfterSalesInspectionCreate,
    ) -> anyhow::Result<AfterSalesInspection> {
        let row: AfterSalesInspectionRow = sqlx::query_as(
            r#"
            insert into after_sales_inspections(
                tenant_id, after_sales_id, sku_id, warehouse_id,
                qty, result_code, inspector_name, inspected_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                inspection_id, after_sales_id, sku_id, warehouse_id,
                qty, result_code, inspector_name, inspected_at,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.after_sales_id)
        .bind(input.sku_id)
        .bind(input.warehouse_id)
        .bind(input.qty)
        .bind(input.result_code)
        .bind(input.inspector_name)
        .bind(input.inspected_at)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_inspection(input.tenant_id))
    }

    pub async fn update_inspection(
        &self,
        tenant_id: TenantId,
        inspection_id: uuid::Uuid,
        update: AfterSalesInspectionUpdate,
    ) -> anyhow::Result<Option<AfterSalesInspection>> {
        let mut qb = QueryBuilder::<Postgres>::new("update after_sales_inspections set ");
        let mut separated = qb.separated(", ");
        let mut has_update = false;

        if let Some(warehouse_id) = update.warehouse_id {
            separated.push("warehouse_id = ");
            separated.push_bind(warehouse_id);
            has_update = true;
        }
        if let Some(qty) = update.qty {
            separated.push("qty = ");
            separated.push_bind(qty);
            has_update = true;
        }
        if let Some(result_code) = update.result_code {
            separated.push("result_code = ");
            separated.push_bind(result_code);
            has_update = true;
        }
        if let Some(inspector_name) = update.inspector_name {
            separated.push("inspector_name = ");
            separated.push_bind(inspector_name);
            has_update = true;
        }
        if let Some(inspected_at) = update.inspected_at {
            separated.push("inspected_at = ");
            separated.push_bind(inspected_at);
            has_update = true;
        }

        if !has_update {
            return Err(anyhow!("no fields to update"));
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and inspection_id = ");
        qb.push_bind(inspection_id);
        qb.push(
            " returning inspection_id, after_sales_id, sku_id, warehouse_id,
              qty, result_code, inspector_name, inspected_at, created_at, updated_at",
        );

        let row: Option<AfterSalesInspectionRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_inspection(tenant_id)))
    }
}

#[derive(sqlx::FromRow)]
struct AfterSalesInspectionRow {
    inspection_id: uuid::Uuid,
    after_sales_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    warehouse_id: Option<uuid::Uuid>,
    qty: i64,
    result_code: String,
    inspector_name: Option<String>,
    inspected_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AfterSalesInspectionRow {
    fn into_inspection(self, tenant_id: TenantId) -> AfterSalesInspection {
        AfterSalesInspection {
            inspection_id: self.inspection_id,
            tenant_id,
            after_sales_id: self.after_sales_id,
            sku_id: self.sku_id,
            warehouse_id: self.warehouse_id,
            qty: self.qty,
            result_code: self.result_code,
            inspector_name: self.inspector_name,
            inspected_at: self.inspected_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
