use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUnitConversion {
    pub conversion_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub sku_name: Option<String>,
    pub base_unit: String,
    pub aux_unit: String,
    pub ratio: i32,
    pub rounding: String,
    pub scenario: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProductUnitConversionListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub scenario: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ProductUnitConversionListResult {
    pub items: Vec<ProductUnitConversion>,
    pub total: i64,
}

pub struct ProductUnitConversionCreate {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub base_unit: String,
    pub aux_unit: String,
    pub ratio: i32,
    pub rounding: String,
    pub scenario: String,
    pub status: String,
}

pub struct ProductUnitConversionUpdate {
    pub base_unit: Option<String>,
    pub aux_unit: Option<String>,
    pub ratio: Option<i32>,
    pub rounding: Option<String>,
    pub scenario: Option<String>,
    pub status: Option<String>,
}

pub struct ProductUnitConversionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductUnitConversionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_conversions(
        &self,
        tenant_id: TenantId,
        filters: ProductUnitConversionListFilters,
    ) -> anyhow::Result<ProductUnitConversionListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from product_unit_conversions puc where puc.tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                puc.conversion_id,
                puc.sku_id,
                s.sku_code,
                s.name as sku_name,
                puc.base_unit,
                puc.aux_unit,
                puc.ratio,
                puc.rounding,
                puc.scenario,
                puc.status,
                puc.created_at,
                puc.updated_at
            from product_unit_conversions puc
            join skus s
              on s.sku_id = puc.sku_id
             and s.tenant_id = puc.tenant_id
            where puc.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by puc.updated_at desc, puc.conversion_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            conversion_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            base_unit: String,
            aux_unit: String,
            ratio: i32,
            rounding: String,
            scenario: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProductUnitConversion {
                conversion_id: row.conversion_id,
                tenant_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                sku_name: row.sku_name,
                base_unit: row.base_unit,
                aux_unit: row.aux_unit,
                ratio: row.ratio,
                rounding: row.rounding,
                scenario: row.scenario,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProductUnitConversionListResult { items, total })
    }

    pub async fn get_conversion(
        &self,
        tenant_id: TenantId,
        conversion_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductUnitConversion>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            conversion_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            base_unit: String,
            aux_unit: String,
            ratio: i32,
            rounding: String,
            scenario: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                puc.conversion_id,
                puc.sku_id,
                s.sku_code,
                s.name as sku_name,
                puc.base_unit,
                puc.aux_unit,
                puc.ratio,
                puc.rounding,
                puc.scenario,
                puc.status,
                puc.created_at,
                puc.updated_at
            from product_unit_conversions puc
            join skus s
              on s.sku_id = puc.sku_id
             and s.tenant_id = puc.tenant_id
            where puc.tenant_id = $1 and puc.conversion_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(conversion_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductUnitConversion {
            conversion_id: row.conversion_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            base_unit: row.base_unit,
            aux_unit: row.aux_unit,
            ratio: row.ratio,
            rounding: row.rounding,
            scenario: row.scenario,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_conversion(
        &self,
        input: ProductUnitConversionCreate,
    ) -> anyhow::Result<ProductUnitConversion> {
        #[derive(sqlx::FromRow)]
        struct Row {
            conversion_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            base_unit: String,
            aux_unit: String,
            ratio: i32,
            rounding: String,
            scenario: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into product_unit_conversions(
                tenant_id, sku_id, base_unit, aux_unit, ratio, rounding, scenario, status
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                conversion_id,
                sku_id,
                (select sku_code from skus where sku_id = $2) as sku_code,
                (select name from skus where sku_id = $2) as sku_name,
                base_unit,
                aux_unit,
                ratio,
                rounding,
                scenario,
                status,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.base_unit)
        .bind(&input.aux_unit)
        .bind(input.ratio)
        .bind(&input.rounding)
        .bind(&input.scenario)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(ProductUnitConversion {
            conversion_id: row.conversion_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            base_unit: row.base_unit,
            aux_unit: row.aux_unit,
            ratio: row.ratio,
            rounding: row.rounding,
            scenario: row.scenario,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_conversion(
        &self,
        tenant_id: TenantId,
        conversion_id: uuid::Uuid,
        input: ProductUnitConversionUpdate,
    ) -> anyhow::Result<Option<ProductUnitConversion>> {
        let mut qb = QueryBuilder::<Postgres>::new("update product_unit_conversions set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(base_unit) = input.base_unit {
            separated.push("base_unit = ");
            separated.push_bind(base_unit);
            has_updates = true;
        }
        if let Some(aux_unit) = input.aux_unit {
            separated.push("aux_unit = ");
            separated.push_bind(aux_unit);
            has_updates = true;
        }
        if let Some(ratio) = input.ratio {
            separated.push("ratio = ");
            separated.push_bind(ratio);
            has_updates = true;
        }
        if let Some(rounding) = input.rounding {
            separated.push("rounding = ");
            separated.push_bind(rounding);
            has_updates = true;
        }
        if let Some(scenario) = input.scenario {
            separated.push("scenario = ");
            separated.push_bind(scenario);
            has_updates = true;
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
            has_updates = true;
        }

        if !has_updates {
            return self.get_conversion(tenant_id, conversion_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and conversion_id = ");
        qb.push_bind(conversion_id);
        qb.push(
            r#"
            returning
                conversion_id,
                sku_id,
                (select sku_code from skus where sku_id = product_unit_conversions.sku_id) as sku_code,
                (select name from skus where sku_id = product_unit_conversions.sku_id) as sku_name,
                base_unit,
                aux_unit,
                ratio,
                rounding,
                scenario,
                status,
                created_at,
                updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            conversion_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            base_unit: String,
            aux_unit: String,
            ratio: i32,
            rounding: String,
            scenario: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ProductUnitConversion {
            conversion_id: row.conversion_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            base_unit: row.base_unit,
            aux_unit: row.aux_unit,
            ratio: row.ratio,
            rounding: row.rounding,
            scenario: row.scenario,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_conversion(
        &self,
        tenant_id: TenantId,
        conversion_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductUnitConversion>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            conversion_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            base_unit: String,
            aux_unit: String,
            ratio: i32,
            rounding: String,
            scenario: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from product_unit_conversions
            where tenant_id = $1 and conversion_id = $2
            returning
                conversion_id,
                sku_id,
                (select sku_code from skus where sku_id = product_unit_conversions.sku_id) as sku_code,
                (select name from skus where sku_id = product_unit_conversions.sku_id) as sku_name,
                base_unit,
                aux_unit,
                ratio,
                rounding,
                scenario,
                status,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(conversion_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductUnitConversion {
            conversion_id: row.conversion_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            base_unit: row.base_unit,
            aux_unit: row.aux_unit,
            ratio: row.ratio,
            rounding: row.rounding,
            scenario: row.scenario,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProductUnitConversionListFilters,
) {
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and puc.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(scenario) = filters.scenario.as_ref() {
        qb.push(" and puc.scenario = ");
        qb.push_bind(scenario);
    }
    if let Some(status) = filters.status.as_ref() {
        qb.push(" and puc.status = ");
        qb.push_bind(status);
    }
}
