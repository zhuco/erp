use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Courier {
    pub courier_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub code: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CourierListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct CourierListResult {
    pub items: Vec<Courier>,
    pub total: i64,
}

pub struct CourierCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub code: String,
    pub status: String,
}

pub struct CourierUpdate {
    pub name: Option<String>,
    pub code: Option<String>,
    pub status: Option<String>,
}

pub struct CouriersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> CouriersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_couriers(
        &self,
        tenant_id: TenantId,
        filters: CourierListFilters,
    ) -> anyhow::Result<CourierListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let status = filters
            .status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());

        let total: i64 = match (&keyword, &status) {
            (Some(keyword), Some(status)) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from couriers
                    where tenant_id = $1
                      and status = $2
                      and (name ilike $3 or code ilike $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(status)
                .bind(keyword)
                .fetch_one(self.pool)
                .await?
            }
            (Some(keyword), None) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from couriers
                    where tenant_id = $1
                      and (name ilike $2 or code ilike $2)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .fetch_one(self.pool)
                .await?
            }
            (None, Some(status)) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from couriers
                    where tenant_id = $1
                      and status = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(status)
                .fetch_one(self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from couriers
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
            select courier_id, name, code, status, created_at, updated_at
            from couriers
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and (name ilike ");
            qb.push_bind(keyword);
            qb.push(" or code ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }

        if let Some(status) = &status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        qb.push(" order by case when status = 'ACTIVE' then 0 else 1 end, updated_at desc, courier_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            courier_id: uuid::Uuid,
            name: String,
            code: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| Courier {
                courier_id: row.courier_id,
                tenant_id,
                name: row.name,
                code: row.code,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(CourierListResult { items, total })
    }

    pub async fn create_courier(&self, input: CourierCreate) -> anyhow::Result<Courier> {
        #[derive(sqlx::FromRow)]
        struct Row {
            courier_id: uuid::Uuid,
            name: String,
            code: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into couriers (tenant_id, name, code, status)
            values ($1, $2, $3, $4)
            on conflict (tenant_id, code)
            do update set
                name = excluded.name,
                status = excluded.status,
                updated_at = now()
            returning courier_id, name, code, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.code)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(Courier {
            courier_id: row.courier_id,
            tenant_id: input.tenant_id,
            name: row.name,
            code: row.code,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_courier(
        &self,
        tenant_id: TenantId,
        courier_id: uuid::Uuid,
        input: CourierUpdate,
    ) -> anyhow::Result<Option<Courier>> {
        let mut qb = QueryBuilder::<Postgres>::new("update couriers set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(code) = input.code {
            separated.push("code = ");
            separated.push_bind(code);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and courier_id = ");
        qb.push_bind(courier_id);
        qb.push(" returning courier_id, name, code, status, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            courier_id: uuid::Uuid,
            name: String,
            code: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(Courier {
            courier_id: row.courier_id,
            tenant_id,
            name: row.name,
            code: row.code,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_courier(
        &self,
        tenant_id: TenantId,
        courier_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Courier>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            courier_id: uuid::Uuid,
            name: String,
            code: String,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from couriers
            where tenant_id = $1 and courier_id = $2
            returning courier_id, name, code, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(courier_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Courier {
            courier_id: row.courier_id,
            tenant_id,
            name: row.name,
            code: row.code,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
