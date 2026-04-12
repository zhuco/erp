use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintTemplate {
    pub template_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: uuid::Uuid,
    pub platform: String,
    pub template_type: String,
    pub platform_template_id: Option<String>,
    pub version: i32,
    pub content_format: String,
    pub content: Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PrintTemplateCursor {
    pub created_at: DateTime<Utc>,
    pub template_id: uuid::Uuid,
}

impl PrintTemplateCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.template_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let template_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            template_id,
        })
    }
}

pub struct PrintTemplateListFilters {
    pub shop_id: Option<uuid::Uuid>,
    pub platform: Option<String>,
    pub template_type: Option<String>,
    pub enabled: Option<bool>,
    pub cursor: Option<PrintTemplateCursor>,
    pub limit: i64,
}

pub struct PrintTemplateListResult {
    pub items: Vec<PrintTemplate>,
    pub next_cursor: Option<PrintTemplateCursor>,
}

pub struct PrintTemplateCreate {
    pub tenant_id: TenantId,
    pub shop_id: uuid::Uuid,
    pub platform: String,
    pub template_type: String,
    pub platform_template_id: Option<String>,
    pub version: i32,
    pub content_format: String,
    pub content: Value,
    pub enabled: bool,
}

pub struct PrintTemplateUpdate {
    pub platform_template_id: Option<String>,
    pub content_format: Option<String>,
    pub content: Option<Value>,
    pub enabled: Option<bool>,
}

pub struct PrintTemplatesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PrintTemplatesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintTemplate>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            template_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            template_type: String,
            platform_template_id: Option<String>,
            version: i32,
            content_format: String,
            content: Value,
            is_active: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                template_id, shop_id, platform, template_type, platform_template_id,
                version, content_format, content, is_active, created_at, updated_at
            from print_templates
            where tenant_id = $1 and template_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(template_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PrintTemplate {
            template_id: row.template_id,
            tenant_id,
            shop_id: row.shop_id,
            platform: row.platform,
            template_type: row.template_type,
            platform_template_id: row.platform_template_id,
            version: row.version,
            content_format: row.content_format,
            content: row.content,
            enabled: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_templates(
        &self,
        tenant_id: TenantId,
        filters: PrintTemplateListFilters,
    ) -> anyhow::Result<PrintTemplateListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                template_id, shop_id, platform, template_type, platform_template_id,
                version, content_format, content, is_active,
                created_at, updated_at
            from print_templates
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = filters.shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(platform) = filters.platform {
            qb.push(" and platform = ");
            qb.push_bind(platform);
        }
        if let Some(template_type) = filters.template_type {
            qb.push(" and template_type = ");
            qb.push_bind(template_type);
        }
        if let Some(enabled) = filters.enabled {
            qb.push(" and is_active = ");
            qb.push_bind(enabled);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, template_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.template_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, template_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            template_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            template_type: String,
            platform_template_id: Option<String>,
            version: i32,
            content_format: String,
            content: Value,
            is_active: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| PrintTemplateCursor {
            created_at: row.created_at,
            template_id: row.template_id,
        });

        let items = rows
            .into_iter()
            .map(|row| PrintTemplate {
                template_id: row.template_id,
                tenant_id,
                shop_id: row.shop_id,
                platform: row.platform,
                template_type: row.template_type,
                platform_template_id: row.platform_template_id,
                version: row.version,
                content_format: row.content_format,
                content: row.content,
                enabled: row.is_active,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(PrintTemplateListResult { items, next_cursor })
    }

    pub async fn create_template(
        &self,
        input: PrintTemplateCreate,
    ) -> anyhow::Result<PrintTemplate> {
        #[derive(sqlx::FromRow)]
        struct Row {
            template_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            template_type: String,
            platform_template_id: Option<String>,
            version: i32,
            content_format: String,
            content: Value,
            is_active: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into print_templates(
                tenant_id, shop_id, platform, template_type, platform_template_id,
                version, content_format, content, is_active
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            returning
                template_id, shop_id, platform, template_type, platform_template_id,
                version, content_format, content, is_active,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.shop_id)
        .bind(&input.platform)
        .bind(&input.template_type)
        .bind(&input.platform_template_id)
        .bind(input.version)
        .bind(&input.content_format)
        .bind(&input.content)
        .bind(input.enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(PrintTemplate {
            template_id: row.template_id,
            tenant_id: input.tenant_id,
            shop_id: row.shop_id,
            platform: row.platform,
            template_type: row.template_type,
            platform_template_id: row.platform_template_id,
            version: row.version,
            content_format: row.content_format,
            content: row.content,
            enabled: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
        input: PrintTemplateUpdate,
    ) -> anyhow::Result<Option<PrintTemplate>> {
        let mut qb = QueryBuilder::<Postgres>::new("update print_templates set ");
        let mut separated = qb.separated(", ");

        if let Some(platform_template_id) = input.platform_template_id {
            separated.push("platform_template_id = ");
            separated.push_bind(platform_template_id);
        }
        if let Some(content_format) = input.content_format {
            separated.push("content_format = ");
            separated.push_bind(content_format);
        }
        if let Some(content) = input.content {
            separated.push("content = ");
            separated.push_bind(content);
        }
        if let Some(enabled) = input.enabled {
            separated.push("is_active = ");
            separated.push_bind(enabled);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and template_id = ");
        qb.push_bind(template_id);
        qb.push(
            r#"
            returning
                template_id, shop_id, platform, template_type, platform_template_id,
                version, content_format, content, is_active,
                created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            template_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            template_type: String,
            platform_template_id: Option<String>,
            version: i32,
            content_format: String,
            content: Value,
            is_active: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(PrintTemplate {
            template_id: row.template_id,
            tenant_id,
            shop_id: row.shop_id,
            platform: row.platform,
            template_type: row.template_type,
            platform_template_id: row.platform_template_id,
            version: row.version,
            content_format: row.content_format,
            content: row.content,
            enabled: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn get_default_template(
        &self,
        tenant_id: TenantId,
        template_type: &str,
        shop_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<Option<PrintTemplate>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            template_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            template_type: String,
            platform_template_id: Option<String>,
            version: i32,
            content_format: String,
            content: Value,
            is_active: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                template_id, shop_id, platform, template_type, platform_template_id,
                version, content_format, content, is_active, created_at, updated_at
            from print_templates
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and template_type = ");
        qb.push_bind(template_type);
        qb.push(" and is_active = true");
        if let Some(shop_id) = shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(shop_id);
        }
        qb.push(" order by version desc, updated_at desc, template_id desc");
        qb.push(" limit 1");

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;

        Ok(row.map(|row| PrintTemplate {
            template_id: row.template_id,
            tenant_id,
            shop_id: row.shop_id,
            platform: row.platform,
            template_type: row.template_type,
            platform_template_id: row.platform_template_id,
            version: row.version,
            content_format: row.content_format,
            content: row.content,
            enabled: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_template(
        &self,
        tenant_id: TenantId,
        template_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PrintTemplate>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            template_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            template_type: String,
            platform_template_id: Option<String>,
            version: i32,
            content_format: String,
            content: Value,
            is_active: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from print_templates
            where tenant_id = $1 and template_id = $2
            returning
                template_id, shop_id, platform, template_type, platform_template_id,
                version, content_format, content, is_active, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(template_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PrintTemplate {
            template_id: row.template_id,
            tenant_id,
            shop_id: row.shop_id,
            platform: row.platform,
            template_type: row.template_type,
            platform_template_id: row.platform_template_id,
            version: row.version,
            content_format: row.content_format,
            content: row.content,
            enabled: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
