use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderTagDefinition {
    pub tag_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub color: Option<String>,
    pub priority: i32,
    pub status: String,
    pub auto_apply: bool,
    pub conditions_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct OrderTagDefinitionListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct OrderTagDefinitionListResult {
    pub items: Vec<OrderTagDefinition>,
    pub total: i64,
}

pub struct OrderTagDefinitionCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub color: Option<String>,
    pub priority: i32,
    pub status: String,
    pub auto_apply: bool,
    pub conditions_json: Value,
}

pub struct OrderTagDefinitionUpdate {
    pub name: Option<String>,
    pub color: Option<String>,
    pub priority: Option<i32>,
    pub status: Option<String>,
    pub auto_apply: Option<bool>,
    pub conditions_json: Option<Value>,
}

pub struct OrderTagDefinitionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> OrderTagDefinitionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_tags(
        &self,
        tenant_id: TenantId,
        filters: OrderTagDefinitionListFilters,
    ) -> anyhow::Result<OrderTagDefinitionListResult> {
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
                    from order_tag_definitions
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
                    from order_tag_definitions
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
            select tag_id, name, color, priority, status, auto_apply, conditions_json, created_at, updated_at
            from order_tag_definitions
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
            auto_apply: bool,
            conditions_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| OrderTagDefinition {
                tag_id: row.tag_id,
                tenant_id,
                name: row.name,
                color: row.color,
                priority: row.priority,
                status: row.status,
                auto_apply: row.auto_apply,
                conditions_json: row.conditions_json,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(OrderTagDefinitionListResult { items, total })
    }

    pub async fn get_tag(
        &self,
        tenant_id: TenantId,
        tag_id: uuid::Uuid,
    ) -> anyhow::Result<Option<OrderTagDefinition>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            auto_apply: bool,
            conditions_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select tag_id, name, color, priority, status, auto_apply, conditions_json, created_at, updated_at
            from order_tag_definitions
            where tenant_id = $1 and tag_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(tag_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| OrderTagDefinition {
            tag_id: row.tag_id,
            tenant_id,
            name: row.name,
            color: row.color,
            priority: row.priority,
            status: row.status,
            auto_apply: row.auto_apply,
            conditions_json: row.conditions_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn count_usage(
        &self,
        tenant_id: TenantId,
        tag_id: uuid::Uuid,
    ) -> anyhow::Result<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from order_tags ot
            join order_tag_definitions td
              on ot.tenant_id = td.tenant_id and ot.tag = td.name
            where td.tenant_id = $1 and td.tag_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(tag_id)
        .fetch_one(self.pool)
        .await?;
        Ok(count)
    }

    pub async fn create_tag(
        &self,
        input: OrderTagDefinitionCreate,
    ) -> anyhow::Result<OrderTagDefinition> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            auto_apply: bool,
            conditions_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into order_tag_definitions(
              tenant_id, name, color, priority, status, auto_apply, conditions_json
            )
            values ($1, $2, $3, $4, $5, $6, $7)
            on conflict (tenant_id, name)
            do update set
                color = excluded.color,
                priority = excluded.priority,
                status = excluded.status,
                auto_apply = excluded.auto_apply,
                conditions_json = excluded.conditions_json,
                updated_at = now()
            returning tag_id, name, color, priority, status, auto_apply, conditions_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.color)
        .bind(input.priority)
        .bind(&input.status)
        .bind(input.auto_apply)
        .bind(input.conditions_json)
        .fetch_one(self.pool)
        .await?;

        Ok(OrderTagDefinition {
            tag_id: row.tag_id,
            tenant_id: input.tenant_id,
            name: row.name,
            color: row.color,
            priority: row.priority,
            status: row.status,
            auto_apply: row.auto_apply,
            conditions_json: row.conditions_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_tag(
        &self,
        tenant_id: TenantId,
        tag_id: uuid::Uuid,
        input: OrderTagDefinitionUpdate,
    ) -> anyhow::Result<Option<OrderTagDefinition>> {
        let mut qb = QueryBuilder::<Postgres>::new("update order_tag_definitions set ");
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
        if let Some(auto_apply) = input.auto_apply {
            separated.push("auto_apply = ");
            separated.push_bind(auto_apply);
        }
        if let Some(conditions_json) = input.conditions_json {
            separated.push("conditions_json = ");
            separated.push_bind(conditions_json);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and tag_id = ");
        qb.push_bind(tag_id);
        qb.push(" returning tag_id, name, color, priority, status, auto_apply, conditions_json, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            auto_apply: bool,
            conditions_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(OrderTagDefinition {
            tag_id: row.tag_id,
            tenant_id,
            name: row.name,
            color: row.color,
            priority: row.priority,
            status: row.status,
            auto_apply: row.auto_apply,
            conditions_json: row.conditions_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_tag(
        &self,
        tenant_id: TenantId,
        tag_id: uuid::Uuid,
    ) -> anyhow::Result<Option<OrderTagDefinition>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            auto_apply: bool,
            conditions_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from order_tag_definitions
            where tenant_id = $1 and tag_id = $2
            returning tag_id, name, color, priority, status, auto_apply, conditions_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(tag_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| OrderTagDefinition {
            tag_id: row.tag_id,
            tenant_id,
            name: row.name,
            color: row.color,
            priority: row.priority,
            status: row.status,
            auto_apply: row.auto_apply,
            conditions_json: row.conditions_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_auto_apply_tags(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<OrderTagDefinition>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            priority: i32,
            status: String,
            auto_apply: bool,
            conditions_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select tag_id, name, color, priority, status, auto_apply, conditions_json, created_at, updated_at
            from order_tag_definitions
            where tenant_id = $1 and auto_apply = true and status = 'ACTIVE'
            order by priority desc, updated_at desc, tag_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OrderTagDefinition {
                tag_id: row.tag_id,
                tenant_id,
                name: row.name,
                color: row.color,
                priority: row.priority,
                status: row.status,
                auto_apply: row.auto_apply,
                conditions_json: row.conditions_json,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }
}
