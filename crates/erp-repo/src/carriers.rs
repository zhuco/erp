use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Carrier {
    pub tenant_id: TenantId,
    pub carrier_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CarrierListFilters {
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct CarrierListResult {
    pub items: Vec<Carrier>,
    pub total: i64,
}

pub struct CarrierCreate {
    pub tenant_id: TenantId,
    pub carrier_code: String,
    pub name: String,
}

pub struct CarrierUpdate {
    pub name: Option<String>,
}

pub struct CarriersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> CarriersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_carriers(
        &self,
        tenant_id: TenantId,
        filters: CarrierListFilters,
    ) -> anyhow::Result<CarrierListResult> {
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
                    from carriers
                    where tenant_id = $1
                      and (carrier_code ilike $2 or name ilike $2)
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
                    from carriers
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
            select carrier_code, name, created_at, updated_at
            from carriers
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = keyword {
            qb.push(" and (carrier_code ilike ");
            qb.push_bind(keyword.clone());
            qb.push(" or name ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }

        qb.push(" order by updated_at desc, carrier_code asc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            carrier_code: String,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| Carrier {
                tenant_id,
                carrier_code: row.carrier_code,
                name: row.name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(CarrierListResult { items, total })
    }

    pub async fn create_carrier(&self, input: CarrierCreate) -> anyhow::Result<Carrier> {
        #[derive(sqlx::FromRow)]
        struct Row {
            carrier_code: String,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into carriers (tenant_id, carrier_code, name)
            values ($1, $2, $3)
            on conflict (tenant_id, carrier_code)
            do update set
                name = excluded.name,
                updated_at = now()
            returning carrier_code, name, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.carrier_code)
        .bind(&input.name)
        .fetch_one(self.pool)
        .await?;

        Ok(Carrier {
            tenant_id: input.tenant_id,
            carrier_code: row.carrier_code,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_carrier(
        &self,
        tenant_id: TenantId,
        carrier_code: &str,
        input: CarrierUpdate,
    ) -> anyhow::Result<Option<Carrier>> {
        let mut qb = QueryBuilder::<Postgres>::new("update carriers set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and carrier_code = ");
        qb.push_bind(carrier_code);
        qb.push(" returning carrier_code, name, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            carrier_code: String,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(Carrier {
            tenant_id,
            carrier_code: row.carrier_code,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_carrier(
        &self,
        tenant_id: TenantId,
        carrier_code: &str,
    ) -> anyhow::Result<Option<Carrier>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            carrier_code: String,
            name: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from carriers
            where tenant_id = $1 and carrier_code = $2
            returning carrier_code, name, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(carrier_code)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Carrier {
            tenant_id,
            carrier_code: row.carrier_code,
            name: row.name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
