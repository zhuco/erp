use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelFieldMapping {
    pub mapping_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub platform: String,
    pub entity_type: String,
    pub platform_field: String,
    pub internal_field: String,
    pub default_value: Option<String>,
    pub transform_rule: Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ChannelFieldMappingListFilters {
    pub platform: Option<String>,
    pub entity_type: Option<String>,
    pub platform_field: Option<String>,
    pub internal_field: Option<String>,
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ChannelFieldMappingListResult {
    pub items: Vec<ChannelFieldMapping>,
    pub total: i64,
}

pub struct ChannelFieldMappingCreate {
    pub tenant_id: TenantId,
    pub platform: String,
    pub entity_type: String,
    pub platform_field: String,
    pub internal_field: String,
    pub default_value: Option<String>,
    pub transform_rule: Value,
    pub enabled: bool,
}

pub struct ChannelFieldMappingUpdate {
    pub platform: Option<String>,
    pub entity_type: Option<String>,
    pub platform_field: Option<String>,
    pub internal_field: Option<String>,
    pub default_value: Option<Option<String>>,
    pub transform_rule: Option<Value>,
    pub enabled: Option<bool>,
}

pub struct ChannelFieldMappingsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ChannelFieldMappingsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_mappings(
        &self,
        tenant_id: TenantId,
        filters: ChannelFieldMappingListFilters,
    ) -> anyhow::Result<ChannelFieldMappingListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from channel_field_mappings where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                mapping_id, platform, entity_type, platform_field, internal_field,
                default_value, transform_rule, enabled, created_at, updated_at
            from channel_field_mappings
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by updated_at desc, mapping_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            platform: String,
            entity_type: String,
            platform_field: String,
            internal_field: String,
            default_value: Option<String>,
            transform_rule: Value,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ChannelFieldMapping {
                mapping_id: row.mapping_id,
                tenant_id,
                platform: row.platform,
                entity_type: row.entity_type,
                platform_field: row.platform_field,
                internal_field: row.internal_field,
                default_value: row.default_value,
                transform_rule: row.transform_rule,
                enabled: row.enabled,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ChannelFieldMappingListResult { items, total })
    }

    pub async fn create_mapping(
        &self,
        input: ChannelFieldMappingCreate,
    ) -> anyhow::Result<ChannelFieldMapping> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            platform: String,
            entity_type: String,
            platform_field: String,
            internal_field: String,
            default_value: Option<String>,
            transform_rule: Value,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into channel_field_mappings(
                tenant_id, platform, entity_type, platform_field, internal_field,
                default_value, transform_rule, enabled
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                mapping_id, platform, entity_type, platform_field, internal_field,
                default_value, transform_rule, enabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.platform)
        .bind(&input.entity_type)
        .bind(&input.platform_field)
        .bind(&input.internal_field)
        .bind(&input.default_value)
        .bind(&input.transform_rule)
        .bind(input.enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(ChannelFieldMapping {
            mapping_id: row.mapping_id,
            tenant_id: input.tenant_id,
            platform: row.platform,
            entity_type: row.entity_type,
            platform_field: row.platform_field,
            internal_field: row.internal_field,
            default_value: row.default_value,
            transform_rule: row.transform_rule,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_mapping(
        &self,
        tenant_id: TenantId,
        mapping_id: uuid::Uuid,
        input: ChannelFieldMappingUpdate,
    ) -> anyhow::Result<Option<ChannelFieldMapping>> {
        let mut qb = QueryBuilder::<Postgres>::new("update channel_field_mappings set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(platform) = input.platform {
            separated.push("platform = ");
            separated.push_bind(platform);
            has_updates = true;
        }
        if let Some(entity_type) = input.entity_type {
            separated.push("entity_type = ");
            separated.push_bind(entity_type);
            has_updates = true;
        }
        if let Some(platform_field) = input.platform_field {
            separated.push("platform_field = ");
            separated.push_bind(platform_field);
            has_updates = true;
        }
        if let Some(internal_field) = input.internal_field {
            separated.push("internal_field = ");
            separated.push_bind(internal_field);
            has_updates = true;
        }
        if let Some(default_value) = input.default_value {
            separated.push("default_value = ");
            separated.push_bind(default_value);
            has_updates = true;
        }
        if let Some(transform_rule) = input.transform_rule {
            separated.push("transform_rule = ");
            separated.push_bind(transform_rule);
            has_updates = true;
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
            has_updates = true;
        }

        if !has_updates {
            return self.get_mapping(tenant_id, mapping_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and mapping_id = ");
        qb.push_bind(mapping_id);
        qb.push(
            " returning mapping_id, platform, entity_type, platform_field, internal_field, default_value, transform_rule, enabled, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            platform: String,
            entity_type: String,
            platform_field: String,
            internal_field: String,
            default_value: Option<String>,
            transform_rule: Value,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ChannelFieldMapping {
            mapping_id: row.mapping_id,
            tenant_id,
            platform: row.platform,
            entity_type: row.entity_type,
            platform_field: row.platform_field,
            internal_field: row.internal_field,
            default_value: row.default_value,
            transform_rule: row.transform_rule,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_mapping(
        &self,
        tenant_id: TenantId,
        mapping_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ChannelFieldMapping>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            platform: String,
            entity_type: String,
            platform_field: String,
            internal_field: String,
            default_value: Option<String>,
            transform_rule: Value,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from channel_field_mappings
            where tenant_id = $1 and mapping_id = $2
            returning
                mapping_id, platform, entity_type, platform_field, internal_field,
                default_value, transform_rule, enabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(mapping_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ChannelFieldMapping {
            mapping_id: row.mapping_id,
            tenant_id,
            platform: row.platform,
            entity_type: row.entity_type,
            platform_field: row.platform_field,
            internal_field: row.internal_field,
            default_value: row.default_value,
            transform_rule: row.transform_rule,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn get_mapping(
        &self,
        tenant_id: TenantId,
        mapping_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ChannelFieldMapping>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            platform: String,
            entity_type: String,
            platform_field: String,
            internal_field: String,
            default_value: Option<String>,
            transform_rule: Value,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                mapping_id, platform, entity_type, platform_field, internal_field,
                default_value, transform_rule, enabled, created_at, updated_at
            from channel_field_mappings
            where tenant_id = $1 and mapping_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(mapping_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ChannelFieldMapping {
            mapping_id: row.mapping_id,
            tenant_id,
            platform: row.platform,
            entity_type: row.entity_type,
            platform_field: row.platform_field,
            internal_field: row.internal_field,
            default_value: row.default_value,
            transform_rule: row.transform_rule,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ChannelFieldMappingListFilters,
) {
    if let Some(platform) = &filters.platform {
        qb.push(" and platform = ");
        qb.push_bind(platform);
    }
    if let Some(entity_type) = &filters.entity_type {
        qb.push(" and entity_type = ");
        qb.push_bind(entity_type);
    }
    if let Some(platform_field) = &filters.platform_field {
        qb.push(" and platform_field = ");
        qb.push_bind(platform_field);
    }
    if let Some(internal_field) = &filters.internal_field {
        qb.push(" and internal_field = ");
        qb.push_bind(internal_field);
    }
    if let Some(enabled) = filters.enabled {
        qb.push(" and enabled = ");
        qb.push_bind(enabled);
    }
}
