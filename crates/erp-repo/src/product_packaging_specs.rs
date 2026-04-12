use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPackagingSpec {
    pub spec_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub level_code: String,
    pub unit_name: String,
    pub units_per_level: i64,
    pub length_mm: Option<i32>,
    pub width_mm: Option<i32>,
    pub height_mm: Option<i32>,
    pub weight_gram: Option<i32>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProductPackagingSpecUpsert {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub level_code: String,
    pub unit_name: String,
    pub units_per_level: i64,
    pub length_mm: Option<i32>,
    pub width_mm: Option<i32>,
    pub height_mm: Option<i32>,
    pub weight_gram: Option<i32>,
}

pub struct ProductPackagingSpecsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductPackagingSpecsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<ProductPackagingSpec>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            spec_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            level_code: String,
            unit_name: String,
            units_per_level: i64,
            length_mm: Option<i32>,
            width_mm: Option<i32>,
            height_mm: Option<i32>,
            weight_gram: Option<i32>,
            is_default: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select
                spec_id,
                sku_id,
                level_code,
                unit_name,
                units_per_level,
                length_mm,
                width_mm,
                height_mm,
                weight_gram,
                is_default,
                created_at,
                updated_at
            from product_packaging_specs
            where tenant_id = $1 and sku_id = $2
            order by is_default desc, updated_at desc, spec_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ProductPackagingSpec {
                spec_id: row.spec_id,
                tenant_id,
                sku_id: row.sku_id,
                level_code: row.level_code,
                unit_name: row.unit_name,
                units_per_level: row.units_per_level,
                length_mm: row.length_mm,
                width_mm: row.width_mm,
                height_mm: row.height_mm,
                weight_gram: row.weight_gram,
                is_default: row.is_default,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn upsert_default(
        &self,
        input: ProductPackagingSpecUpsert,
    ) -> anyhow::Result<ProductPackagingSpec> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            update product_packaging_specs
            set is_default = false, updated_at = now()
            where tenant_id = $1 and sku_id = $2 and is_default = true
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .execute(&mut *tx)
        .await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            spec_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            level_code: String,
            unit_name: String,
            units_per_level: i64,
            length_mm: Option<i32>,
            width_mm: Option<i32>,
            height_mm: Option<i32>,
            weight_gram: Option<i32>,
            is_default: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into product_packaging_specs(
                tenant_id,
                sku_id,
                level_code,
                unit_name,
                units_per_level,
                length_mm,
                width_mm,
                height_mm,
                weight_gram,
                is_default
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,true)
            on conflict (tenant_id, sku_id, level_code)
            do update set
                unit_name = excluded.unit_name,
                units_per_level = excluded.units_per_level,
                length_mm = excluded.length_mm,
                width_mm = excluded.width_mm,
                height_mm = excluded.height_mm,
                weight_gram = excluded.weight_gram,
                is_default = true,
                updated_at = now()
            returning
                spec_id,
                sku_id,
                level_code,
                unit_name,
                units_per_level,
                length_mm,
                width_mm,
                height_mm,
                weight_gram,
                is_default,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.level_code)
        .bind(&input.unit_name)
        .bind(input.units_per_level.max(1))
        .bind(input.length_mm)
        .bind(input.width_mm)
        .bind(input.height_mm)
        .bind(input.weight_gram)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(ProductPackagingSpec {
            spec_id: row.spec_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            level_code: row.level_code,
            unit_name: row.unit_name,
            units_per_level: row.units_per_level,
            length_mm: row.length_mm,
            width_mm: row.width_mm,
            height_mm: row.height_mm,
            weight_gram: row.weight_gram,
            is_default: row.is_default,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
