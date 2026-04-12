use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductBrand {
    pub brand_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProductBrandListFilters {
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ProductBrandListResult {
    pub items: Vec<ProductBrand>,
    pub total: i64,
}

pub struct ProductBrandCreate {
    pub tenant_id: TenantId,
    pub name: String,
}

pub struct ProductBrandUpdate {
    pub name: Option<String>,
}

pub struct ProductBrandsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductBrandsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_brands(
        &self,
        tenant_id: TenantId,
        filters: ProductBrandListFilters,
    ) -> anyhow::Result<ProductBrandListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let total: i64 = match &keyword {
            Some(keyword) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from product_brands
                    where tenant_id = $1
                      and name ilike $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from product_brands
                    where tenant_id = $1
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select brand_id, name, created_at, updated_at
            from product_brands
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }

        qb.push(" order by updated_at desc, brand_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            brand_id: uuid::Uuid,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProductBrand {
                brand_id: row.brand_id,
                tenant_id,
                name: row.name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProductBrandListResult { items, total })
    }

    pub async fn create_brand(&self, input: ProductBrandCreate) -> anyhow::Result<ProductBrand> {
        #[derive(sqlx::FromRow)]
        struct Row {
            brand_id: uuid::Uuid,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into product_brands (tenant_id, name)
            values ($1, $2)
            on conflict (tenant_id, name)
            do update set updated_at = now()
            returning brand_id, name, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .fetch_one(self.pool)
        .await?;

        Ok(ProductBrand {
            brand_id: row.brand_id,
            tenant_id: input.tenant_id,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_brand(
        &self,
        tenant_id: TenantId,
        brand_id: uuid::Uuid,
        input: ProductBrandUpdate,
    ) -> anyhow::Result<Option<ProductBrand>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct CurrentRow {
            name: String,
        }

        let current: Option<CurrentRow> = sqlx::query_as(
            r#"
            select name
            from product_brands
            where tenant_id = $1 and brand_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(brand_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(current) = current else {
            tx.rollback().await?;
            return Ok(None);
        };

        let next_name = input.name.unwrap_or_else(|| current.name.clone());

        #[derive(sqlx::FromRow)]
        struct UpdatedRow {
            brand_id: uuid::Uuid,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let updated: UpdatedRow = sqlx::query_as(
            r#"
            update product_brands
            set name = $3,
                updated_at = now()
            where tenant_id = $1 and brand_id = $2
            returning brand_id, name, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(brand_id)
        .bind(&next_name)
        .fetch_one(&mut *tx)
        .await?;

        if next_name != current.name {
            sqlx::query(
                r#"
                update products
                set brand = $3,
                    updated_at = now()
                where tenant_id = $1 and brand = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(&current.name)
            .bind(&next_name)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(Some(ProductBrand {
            brand_id: updated.brand_id,
            tenant_id,
            name: updated.name,
            created_at: updated.created_at,
            updated_at: updated.updated_at,
        }))
    }

    pub async fn delete_brand(
        &self,
        tenant_id: TenantId,
        brand_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductBrand>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            brand_id: uuid::Uuid,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select brand_id, name, created_at, updated_at
            from product_brands
            where tenant_id = $1 and brand_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(brand_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let used: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from products
            where tenant_id = $1
              and brand = $2
              and status <> 'DELETED'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&row.name)
        .fetch_one(self.pool)
        .await?;

        if used > 0 {
            return Err(anyhow!("brand has products"));
        }

        sqlx::query(
            r#"
            delete from product_brands
            where tenant_id = $1 and brand_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(brand_id)
        .execute(self.pool)
        .await?;

        Ok(Some(ProductBrand {
            brand_id: row.brand_id,
            tenant_id,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
