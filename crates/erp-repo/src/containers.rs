use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub container_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub code: String,
    pub container_type: String,
    pub capacity: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ContainerListFilters {
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ContainerListResult {
    pub items: Vec<Container>,
    pub total: i64,
}

pub struct ContainerCreate {
    pub tenant_id: TenantId,
    pub code: String,
    pub container_type: String,
    pub capacity: Option<f64>,
}

pub struct ContainerUpdate {
    pub container_type: Option<String>,
    pub capacity: Option<Option<f64>>,
}

pub struct ContainersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ContainersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_containers(
        &self,
        tenant_id: TenantId,
        filters: ContainerListFilters,
    ) -> anyhow::Result<ContainerListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from containers
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_container_filters(&mut count_qb, keyword.as_deref());
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select container_id, code, container_type, capacity, created_at, updated_at
            from containers
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_container_filters(&mut qb, keyword.as_deref());
        qb.push(" order by updated_at desc, container_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            container_id: uuid::Uuid,
            code: String,
            container_type: String,
            capacity: Option<f64>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| Container {
                container_id: row.container_id,
                tenant_id,
                code: row.code,
                container_type: row.container_type,
                capacity: row.capacity,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ContainerListResult { items, total })
    }

    pub async fn create_container(&self, input: ContainerCreate) -> anyhow::Result<Container> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let container = self.create_container_with_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(container)
    }

    pub async fn update_container(
        &self,
        tenant_id: TenantId,
        container_id: uuid::Uuid,
        input: ContainerUpdate,
    ) -> anyhow::Result<Option<Container>> {
        let mut qb = QueryBuilder::<Postgres>::new("update containers set ");
        let mut separated = qb.separated(", ");

        if let Some(container_type) = input.container_type {
            separated.push("container_type = ");
            separated.push_bind(container_type);
        }
        if let Some(capacity) = input.capacity {
            separated.push("capacity = ");
            separated.push_bind(capacity);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and container_id = ");
        qb.push_bind(container_id);
        qb.push(" returning container_id, code, container_type, capacity, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            container_id: uuid::Uuid,
            code: String,
            container_type: String,
            capacity: Option<f64>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| Container {
            container_id: row.container_id,
            tenant_id,
            code: row.code,
            container_type: row.container_type,
            capacity: row.capacity,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_container(
        &self,
        tenant_id: TenantId,
        container_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Container>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            container_id: uuid::Uuid,
            code: String,
            container_type: String,
            capacity: Option<f64>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from containers
            where tenant_id = $1 and container_id = $2
            returning container_id, code, container_type, capacity, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(container_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Container {
            container_id: row.container_id,
            tenant_id,
            code: row.code,
            container_type: row.container_type,
            capacity: row.capacity,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn create_container_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: ContainerCreate,
    ) -> anyhow::Result<Container> {
        #[derive(sqlx::FromRow)]
        struct Row {
            container_id: uuid::Uuid,
            code: String,
            container_type: String,
            capacity: Option<f64>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into containers(tenant_id, code, container_type, capacity)
            values ($1,$2,$3,$4)
            on conflict (tenant_id, code)
            do update set
                container_type = excluded.container_type,
                capacity = excluded.capacity,
                updated_at = now()
            returning container_id, code, container_type, capacity, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.code)
        .bind(&input.container_type)
        .bind(input.capacity)
        .fetch_one(&mut **tx)
        .await?;

        Ok(Container {
            container_id: row.container_id,
            tenant_id: input.tenant_id,
            code: row.code,
            container_type: row.container_type,
            capacity: row.capacity,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn apply_container_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, keyword: Option<&'a str>) {
    if let Some(keyword) = keyword {
        qb.push(" and (code ilike ");
        qb.push_bind(keyword);
        qb.push(" or container_type ilike ");
        qb.push_bind(keyword);
        qb.push(")");
    }
}
