use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuTagDefinition {
    pub tag_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub color: Option<String>,
    pub priority: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SkuTagDefinitionListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct SkuTagDefinitionListResult {
    pub items: Vec<SkuTagDefinition>,
    pub total: i64,
}

pub struct SkuTagDefinitionCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub color: Option<String>,
    pub priority: i32,
    pub status: String,
}

pub struct SkuTagDefinitionUpdate {
    pub name: Option<String>,
    pub color: Option<String>,
    pub priority: Option<i32>,
    pub status: Option<String>,
}

pub struct SkuTagDefinitionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SkuTagDefinitionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_tags(
        &self,
        tenant_id: TenantId,
        filters: SkuTagDefinitionListFilters,
    ) -> anyhow::Result<SkuTagDefinitionListResult> {
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
                    from sku_tag_definitions
                    where tenant_id = $1
                      and name ilike $2
                      and ($3::text is null or status = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from sku_tag_definitions
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select tag_id, name, color, priority, status, created_at, updated_at
            from sku_tag_definitions
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        qb.push(" order by priority desc, updated_at desc, tag_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| SkuTagDefinition {
                tag_id: row.tag_id,
                tenant_id,
                name: row.name,
                color: row.color,
                priority: row.priority,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SkuTagDefinitionListResult { items, total })
    }

    pub async fn create_tag(
        &self,
        input: SkuTagDefinitionCreate,
    ) -> anyhow::Result<SkuTagDefinition> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into sku_tag_definitions(tenant_id, name, color, priority, status)
            values ($1, $2, $3, $4, $5)
            on conflict (tenant_id, name)
            do update set
                color = excluded.color,
                priority = excluded.priority,
                status = excluded.status,
                updated_at = now()
            returning tag_id, name, color, priority, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.color)
        .bind(input.priority)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(SkuTagDefinition {
            tag_id: row.tag_id,
            tenant_id: input.tenant_id,
            name: row.name,
            color: row.color,
            priority: row.priority,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_tag(
        &self,
        tenant_id: TenantId,
        tag_id: uuid::Uuid,
        input: SkuTagDefinitionUpdate,
    ) -> anyhow::Result<Option<SkuTagDefinition>> {
        let mut qb = QueryBuilder::<Postgres>::new("update sku_tag_definitions set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(color) = input.color {
            separated.push("color = ");
            separated.push_bind(color);
        }
        if let Some(priority) = input.priority {
            separated.push("priority = ");
            separated.push_bind(priority);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and tag_id = ");
        qb.push_bind(tag_id);
        qb.push(" returning tag_id, name, color, priority, status, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(SkuTagDefinition {
            tag_id: row.tag_id,
            tenant_id,
            name: row.name,
            color: row.color,
            priority: row.priority,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_tag(
        &self,
        tenant_id: TenantId,
        tag_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuTagDefinition>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from sku_tag_definitions
            where tenant_id = $1 and tag_id = $2
            returning tag_id, name, color, priority, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(tag_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SkuTagDefinition {
            tag_id: row.tag_id,
            tenant_id,
            name: row.name,
            color: row.color,
            priority: row.priority,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
