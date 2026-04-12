use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsProduct {
    pub product_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub carrier_code: String,
    pub code: String,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct LogisticsProductListFilters {
    pub keyword: Option<String>,
    pub carrier_code: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct LogisticsProductListResult {
    pub items: Vec<LogisticsProduct>,
    pub total: i64,
}

pub struct LogisticsProductCreate {
    pub tenant_id: TenantId,
    pub carrier_code: String,
    pub code: String,
    pub name: String,
    pub status: String,
}

pub struct LogisticsProductUpdate {
    pub carrier_code: Option<String>,
    pub code: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
}

pub struct LogisticsProductsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsProductsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Option<LogisticsProduct>> {
        let row = sqlx::query_as::<_, ProductRow>(
            r#"
            select product_id, carrier_code, code, name, status, created_at, updated_at
            from logistics_products
            where tenant_id = $1 and product_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_product(tenant_id)))
    }

    pub async fn list_products(
        &self,
        tenant_id: TenantId,
        filters: LogisticsProductListFilters,
    ) -> anyhow::Result<LogisticsProductListResult> {
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
                    from logistics_products
                    where tenant_id = $1
                      and (code ilike $2 or name ilike $2)
                      and ($3::text is null or carrier_code = $3)
                      and ($4::text is null or status = $4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.carrier_code.as_deref())
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from logistics_products
                    where tenant_id = $1
                      and ($2::text is null or carrier_code = $2)
                      and ($3::text is null or status = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.carrier_code.as_deref())
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select product_id, carrier_code, code, name, status, created_at, updated_at
            from logistics_products
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = keyword {
            qb.push(" and (code ilike ");
            qb.push_bind(keyword.clone());
            qb.push(" or name ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }
        if let Some(carrier_code) = &filters.carrier_code {
            qb.push(" and carrier_code = ");
            qb.push_bind(carrier_code);
        }
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        qb.push(" order by updated_at desc, product_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<ProductRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_product(tenant_id))
            .collect();

        Ok(LogisticsProductListResult { items, total })
    }

    pub async fn create_product(
        &self,
        input: LogisticsProductCreate,
    ) -> anyhow::Result<LogisticsProduct> {
        let row: ProductRow = sqlx::query_as(
            r#"
            insert into logistics_products(tenant_id, carrier_code, code, name, status)
            values ($1, $2, $3, $4, $5)
            on conflict (tenant_id, carrier_code, code)
            do update set
                name = excluded.name,
                status = excluded.status,
                updated_at = now()
            returning product_id, carrier_code, code, name, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.carrier_code)
        .bind(&input.code)
        .bind(&input.name)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_product(input.tenant_id))
    }

    pub async fn update_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        input: LogisticsProductUpdate,
    ) -> anyhow::Result<Option<LogisticsProduct>> {
        let mut qb = QueryBuilder::<Postgres>::new("update logistics_products set ");
        let mut separated = qb.separated(", ");

        if let Some(code) = input.code {
            separated.push("code = ");
            separated.push_bind(code);
        }
        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(carrier_code) = input.carrier_code {
            separated.push("carrier_code = ");
            separated.push_bind(carrier_code);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and product_id = ");
        qb.push_bind(product_id);
        qb.push(" returning product_id, carrier_code, code, name, status, created_at, updated_at");

        let row: Option<ProductRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(row.into_product(tenant_id)))
    }

    pub async fn delete_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Option<LogisticsProduct>> {
        let row: Option<ProductRow> = sqlx::query_as(
            r#"
            delete from logistics_products
            where tenant_id = $1 and product_id = $2
            returning product_id, carrier_code, code, name, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_product(tenant_id)))
    }
}

#[derive(sqlx::FromRow)]
struct ProductRow {
    product_id: uuid::Uuid,
    carrier_code: String,
    code: String,
    name: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProductRow {
    fn into_product(self, tenant_id: TenantId) -> LogisticsProduct {
        LogisticsProduct {
            product_id: self.product_id,
            tenant_id,
            carrier_code: self.carrier_code,
            code: self.code,
            name: self.name,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
