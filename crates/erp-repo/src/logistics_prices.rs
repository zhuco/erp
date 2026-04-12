use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsPrice {
    pub price_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub carrier_code: String,
    pub product_id: Option<uuid::Uuid>,
    pub region_code: String,
    pub weight_min_kg: f64,
    pub weight_max_kg: f64,
    pub price_cent: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct LogisticsPriceListFilters {
    pub carrier_code: Option<String>,
    pub product_id: Option<uuid::Uuid>,
    pub region_code: Option<String>,
    pub weight_kg: Option<f64>,
    pub offset: i64,
    pub limit: i64,
}

pub struct LogisticsPriceListResult {
    pub items: Vec<LogisticsPrice>,
    pub total: i64,
}

pub struct LogisticsPriceCreate {
    pub tenant_id: TenantId,
    pub carrier_code: String,
    pub product_id: Option<uuid::Uuid>,
    pub region_code: String,
    pub weight_min_kg: f64,
    pub weight_max_kg: f64,
    pub price_cent: i64,
}

pub struct LogisticsPriceUpdate {
    pub carrier_code: Option<String>,
    pub product_id: Option<uuid::Uuid>,
    pub region_code: Option<String>,
    pub weight_min_kg: Option<f64>,
    pub weight_max_kg: Option<f64>,
    pub price_cent: Option<i64>,
}

pub struct LogisticsPricesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsPricesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_price(
        &self,
        tenant_id: TenantId,
        carrier_code: &str,
        product_id: Option<uuid::Uuid>,
        region_code: &str,
        weight_kg: f64,
    ) -> anyhow::Result<Option<LogisticsPrice>> {
        if let Some(product_id) = product_id {
            if let Some(price) = self
                .fetch_price(
                    tenant_id,
                    carrier_code,
                    Some(product_id),
                    region_code,
                    weight_kg,
                )
                .await?
            {
                return Ok(Some(price));
            }
        }

        self.fetch_price(tenant_id, carrier_code, None, region_code, weight_kg)
            .await
    }

    pub async fn list_prices(
        &self,
        tenant_id: TenantId,
        filters: LogisticsPriceListFilters,
    ) -> anyhow::Result<LogisticsPriceListResult> {
        let total: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from logistics_prices
            where tenant_id = $1
              and ($2::text is null or carrier_code = $2)
              and ($3::uuid is null or product_id = $3)
              and ($4::text is null or region_code = $4)
              and (
                $5::double precision is null
                or (weight_min_kg <= $5 and (weight_max_kg = 0 or weight_max_kg >= $5))
              )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(filters.carrier_code.as_deref())
        .bind(filters.product_id)
        .bind(filters.region_code.as_deref())
        .bind(filters.weight_kg)
        .fetch_one(self.pool)
        .await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select price_id, carrier_code, product_id, region_code, weight_min_kg, weight_max_kg,
                   price_cent, created_at, updated_at
            from logistics_prices
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(carrier_code) = &filters.carrier_code {
            qb.push(" and carrier_code = ");
            qb.push_bind(carrier_code);
        }
        if let Some(product_id) = filters.product_id {
            qb.push(" and product_id = ");
            qb.push_bind(product_id);
        }
        if let Some(region_code) = &filters.region_code {
            qb.push(" and region_code = ");
            qb.push_bind(region_code);
        }
        if let Some(weight_kg) = filters.weight_kg {
            qb.push(" and weight_min_kg <= ");
            qb.push_bind(weight_kg);
            qb.push(" and (weight_max_kg = 0 or weight_max_kg >= ");
            qb.push_bind(weight_kg);
            qb.push(")");
        }

        qb.push(" order by updated_at desc, price_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            price_id: uuid::Uuid,
            carrier_code: String,
            product_id: Option<uuid::Uuid>,
            region_code: String,
            weight_min_kg: f64,
            weight_max_kg: f64,
            price_cent: i64,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| LogisticsPrice {
                price_id: row.price_id,
                tenant_id,
                carrier_code: row.carrier_code,
                product_id: row.product_id,
                region_code: row.region_code,
                weight_min_kg: row.weight_min_kg,
                weight_max_kg: row.weight_max_kg,
                price_cent: row.price_cent,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(LogisticsPriceListResult { items, total })
    }

    async fn fetch_price(
        &self,
        tenant_id: TenantId,
        carrier_code: &str,
        product_id: Option<uuid::Uuid>,
        region_code: &str,
        weight_kg: f64,
    ) -> anyhow::Result<Option<LogisticsPrice>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            price_id: uuid::Uuid,
            carrier_code: String,
            product_id: Option<uuid::Uuid>,
            region_code: String,
            weight_min_kg: f64,
            weight_max_kg: f64,
            price_cent: i64,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select price_id, carrier_code, product_id, region_code, weight_min_kg, weight_max_kg,
                   price_cent, created_at, updated_at
            from logistics_prices
            where tenant_id = $1
              and carrier_code = $2
              and region_code = $3
              and weight_min_kg <= $4
              and (weight_max_kg = 0 or weight_max_kg >= $4)
              and (($5::uuid is null and product_id is null) or product_id = $5)
            order by weight_min_kg desc, weight_max_kg asc, updated_at desc, price_id desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(carrier_code)
        .bind(region_code)
        .bind(weight_kg)
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| LogisticsPrice {
            price_id: row.price_id,
            tenant_id,
            carrier_code: row.carrier_code,
            product_id: row.product_id,
            region_code: row.region_code,
            weight_min_kg: row.weight_min_kg,
            weight_max_kg: row.weight_max_kg,
            price_cent: row.price_cent,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_price(
        &self,
        input: LogisticsPriceCreate,
    ) -> anyhow::Result<LogisticsPrice> {
        #[derive(sqlx::FromRow)]
        struct Row {
            price_id: uuid::Uuid,
            carrier_code: String,
            product_id: Option<uuid::Uuid>,
            region_code: String,
            weight_min_kg: f64,
            weight_max_kg: f64,
            price_cent: i64,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into logistics_prices(
                tenant_id, carrier_code, product_id, region_code, weight_min_kg, weight_max_kg,
                price_cent
            ) values ($1, $2, $3, $4, $5, $6, $7)
            on conflict (tenant_id, carrier_code, region_code, weight_min_kg, weight_max_kg)
            do update set
                product_id = excluded.product_id,
                price_cent = excluded.price_cent,
                updated_at = now()
            returning price_id, carrier_code, product_id, region_code, weight_min_kg, weight_max_kg,
                      price_cent, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.carrier_code)
        .bind(input.product_id)
        .bind(&input.region_code)
        .bind(input.weight_min_kg)
        .bind(input.weight_max_kg)
        .bind(input.price_cent)
        .fetch_one(self.pool)
        .await?;

        Ok(LogisticsPrice {
            price_id: row.price_id,
            tenant_id: input.tenant_id,
            carrier_code: row.carrier_code,
            product_id: row.product_id,
            region_code: row.region_code,
            weight_min_kg: row.weight_min_kg,
            weight_max_kg: row.weight_max_kg,
            price_cent: row.price_cent,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_price(
        &self,
        tenant_id: TenantId,
        price_id: uuid::Uuid,
        input: LogisticsPriceUpdate,
    ) -> anyhow::Result<Option<LogisticsPrice>> {
        let mut qb = QueryBuilder::<Postgres>::new("update logistics_prices set ");
        let mut separated = qb.separated(", ");

        if let Some(carrier_code) = input.carrier_code {
            separated.push("carrier_code = ");
            separated.push_bind(carrier_code);
        }
        if let Some(product_id) = input.product_id {
            separated.push("product_id = ");
            separated.push_bind(product_id);
        }
        if let Some(region_code) = input.region_code {
            separated.push("region_code = ");
            separated.push_bind(region_code);
        }
        if let Some(weight_min_kg) = input.weight_min_kg {
            separated.push("weight_min_kg = ");
            separated.push_bind(weight_min_kg);
        }
        if let Some(weight_max_kg) = input.weight_max_kg {
            separated.push("weight_max_kg = ");
            separated.push_bind(weight_max_kg);
        }
        if let Some(price_cent) = input.price_cent {
            separated.push("price_cent = ");
            separated.push_bind(price_cent);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and price_id = ");
        qb.push_bind(price_id);
        qb.push(
            " returning price_id, carrier_code, product_id, region_code, weight_min_kg,
              weight_max_kg, price_cent, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            price_id: uuid::Uuid,
            carrier_code: String,
            product_id: Option<uuid::Uuid>,
            region_code: String,
            weight_min_kg: f64,
            weight_max_kg: f64,
            price_cent: i64,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(LogisticsPrice {
            price_id: row.price_id,
            tenant_id,
            carrier_code: row.carrier_code,
            product_id: row.product_id,
            region_code: row.region_code,
            weight_min_kg: row.weight_min_kg,
            weight_max_kg: row.weight_max_kg,
            price_cent: row.price_cent,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_price(
        &self,
        tenant_id: TenantId,
        price_id: uuid::Uuid,
    ) -> anyhow::Result<Option<LogisticsPrice>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            price_id: uuid::Uuid,
            carrier_code: String,
            product_id: Option<uuid::Uuid>,
            region_code: String,
            weight_min_kg: f64,
            weight_max_kg: f64,
            price_cent: i64,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from logistics_prices
            where tenant_id = $1 and price_id = $2
            returning price_id, carrier_code, product_id, region_code, weight_min_kg, weight_max_kg,
                      price_cent, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(price_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| LogisticsPrice {
            price_id: row.price_id,
            tenant_id,
            carrier_code: row.carrier_code,
            product_id: row.product_id,
            region_code: row.region_code,
            weight_min_kg: row.weight_min_kg,
            weight_max_kg: row.weight_max_kg,
            price_cent: row.price_cent,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
