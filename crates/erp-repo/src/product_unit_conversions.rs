use anyhow::anyhow;
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

const ROUNDING_ROUND: &str = "ROUND";
const ROUNDING_CEIL: &str = "CEIL";
const ROUNDING_FLOOR: &str = "FLOOR";
const STATUS_ACTIVE: &str = "ACTIVE";
const STATUS_INACTIVE: &str = "INACTIVE";

#[derive(Debug, Clone)]
struct NormalizedConversionState {
    base_unit: String,
    aux_unit: String,
    ratio: i32,
    rounding: String,
    scenario: String,
    status: String,
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

        let normalized = normalize_conversion_state(
            input.base_unit,
            input.aux_unit,
            input.ratio,
            input.rounding,
            input.scenario,
            input.status,
        )?;

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
        .bind(&normalized.base_unit)
        .bind(&normalized.aux_unit)
        .bind(normalized.ratio)
        .bind(&normalized.rounding)
        .bind(&normalized.scenario)
        .bind(&normalized.status)
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
        let current = match self.get_conversion(tenant_id, conversion_id).await? {
            Some(record) => record,
            None => return Ok(None),
        };

        let normalized = normalize_conversion_state(
            input
                .base_unit
                .as_ref()
                .cloned()
                .unwrap_or_else(|| current.base_unit.clone()),
            input
                .aux_unit
                .as_ref()
                .cloned()
                .unwrap_or_else(|| current.aux_unit.clone()),
            input.ratio.unwrap_or(current.ratio),
            input
                .rounding
                .as_ref()
                .cloned()
                .unwrap_or_else(|| current.rounding.clone()),
            input
                .scenario
                .as_ref()
                .cloned()
                .unwrap_or_else(|| current.scenario.clone()),
            input
                .status
                .as_ref()
                .cloned()
                .unwrap_or_else(|| current.status.clone()),
        )?;

        let mut qb = QueryBuilder::<Postgres>::new("update product_unit_conversions set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if input.base_unit.is_some() {
            separated.push("base_unit = ");
            separated.push_bind(&normalized.base_unit);
            has_updates = true;
        }
        if input.aux_unit.is_some() {
            separated.push("aux_unit = ");
            separated.push_bind(&normalized.aux_unit);
            has_updates = true;
        }
        if input.ratio.is_some() {
            separated.push("ratio = ");
            separated.push_bind(normalized.ratio);
            has_updates = true;
        }
        if input.rounding.is_some() {
            separated.push("rounding = ");
            separated.push_bind(&normalized.rounding);
            has_updates = true;
        }
        if input.scenario.is_some() {
            separated.push("scenario = ");
            separated.push_bind(&normalized.scenario);
            has_updates = true;
        }
        if input.status.is_some() {
            separated.push("status = ");
            separated.push_bind(&normalized.status);
            has_updates = true;
        }

        if !has_updates {
            return Ok(Some(current));
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

fn normalize_conversion_state(
    base_unit: String,
    aux_unit: String,
    ratio: i32,
    rounding: String,
    scenario: String,
    status: String,
) -> anyhow::Result<NormalizedConversionState> {
    let base_unit = base_unit.trim().to_string();
    let aux_unit = aux_unit.trim().to_string();
    let rounding = rounding.trim().to_ascii_uppercase();
    let scenario = scenario.trim().to_string();
    let status = status.trim().to_ascii_uppercase();

    if base_unit.is_empty() || aux_unit.is_empty() {
        return Err(anyhow!("base_unit and aux_unit are required"));
    }
    if base_unit == aux_unit {
        return Err(anyhow!("base_unit and aux_unit must be different"));
    }
    if ratio <= 0 {
        return Err(anyhow!("ratio must be greater than 0"));
    }
    if ![ROUNDING_ROUND, ROUNDING_CEIL, ROUNDING_FLOOR].contains(&rounding.as_str()) {
        return Err(anyhow!("invalid rounding: {rounding}"));
    }
    if scenario.is_empty() {
        return Err(anyhow!("scenario is required"));
    }
    if ![STATUS_ACTIVE, STATUS_INACTIVE].contains(&status.as_str()) {
        return Err(anyhow!("invalid status: {status}"));
    }

    Ok(NormalizedConversionState {
        base_unit,
        aux_unit,
        ratio,
        rounding,
        scenario,
        status,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_conversion_state() {
        assert!(normalize_conversion_state(
            "BOX".to_string(),
            "PCS".to_string(),
            12,
            "round".to_string(),
            "SALES".to_string(),
            "active".to_string(),
        )
        .is_ok());
        assert!(normalize_conversion_state(
            "PCS".to_string(),
            "PCS".to_string(),
            1,
            "ROUND".to_string(),
            "SALES".to_string(),
            "ACTIVE".to_string(),
        )
        .is_err());
        assert!(normalize_conversion_state(
            "BOX".to_string(),
            "PCS".to_string(),
            0,
            "ROUND".to_string(),
            "SALES".to_string(),
            "ACTIVE".to_string(),
        )
        .is_err());
    }
}
