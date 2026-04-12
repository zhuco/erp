use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUnit {
    pub unit_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProductUnitListFilters {
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ProductUnitListResult {
    pub items: Vec<ProductUnit>,
    pub total: i64,
}

pub struct ProductUnitCreate {
    pub tenant_id: TenantId,
    pub name: String,
}

pub struct ProductUnitUpdate {
    pub name: Option<String>,
}

pub struct ProductUnitsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductUnitsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_units(
        &self,
        tenant_id: TenantId,
        filters: ProductUnitListFilters,
    ) -> anyhow::Result<ProductUnitListResult> {
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
                    from product_units
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
                    from product_units
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
            select unit_id, name, created_at, updated_at
            from product_units
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }

        qb.push(" order by updated_at desc, unit_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            unit_id: uuid::Uuid,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProductUnit {
                unit_id: row.unit_id,
                tenant_id,
                name: row.name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProductUnitListResult { items, total })
    }

    pub async fn create_unit(&self, input: ProductUnitCreate) -> anyhow::Result<ProductUnit> {
        #[derive(sqlx::FromRow)]
        struct Row {
            unit_id: uuid::Uuid,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into product_units (tenant_id, name)
            values ($1, $2)
            returning unit_id, name, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .fetch_one(self.pool)
        .await?;

        Ok(ProductUnit {
            unit_id: row.unit_id,
            tenant_id: input.tenant_id,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_unit(
        &self,
        tenant_id: TenantId,
        unit_id: uuid::Uuid,
        input: ProductUnitUpdate,
    ) -> anyhow::Result<Option<ProductUnit>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct CurrentRow {
            name: String,
        }

        let current: Option<CurrentRow> = sqlx::query_as(
            r#"
            select name
            from product_units
            where tenant_id = $1 and unit_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(unit_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(current) = current else {
            tx.rollback().await?;
            return Ok(None);
        };

        let next_name = input.name.unwrap_or_else(|| current.name.clone());

        #[derive(sqlx::FromRow)]
        struct UpdatedRow {
            unit_id: uuid::Uuid,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let updated: UpdatedRow = sqlx::query_as(
            r#"
            update product_units
            set name = $3,
                updated_at = now()
            where tenant_id = $1 and unit_id = $2
            returning unit_id, name, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(unit_id)
        .bind(&next_name)
        .fetch_one(&mut *tx)
        .await?;

        if next_name != current.name {
            sqlx::query(
                r#"
                update products
                set base_unit = $3,
                    updated_at = now()
                where tenant_id = $1 and base_unit = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(&current.name)
            .bind(&next_name)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                update products
                set second_unit = $3,
                    updated_at = now()
                where tenant_id = $1 and second_unit = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(&current.name)
            .bind(&next_name)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(Some(ProductUnit {
            unit_id: updated.unit_id,
            tenant_id,
            name: updated.name,
            created_at: updated.created_at,
            updated_at: updated.updated_at,
        }))
    }
}
