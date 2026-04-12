use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettingScope {
    Global,
    Tenant,
    Shop,
    Warehouse,
}

impl SettingScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            SettingScope::Global => "GLOBAL",
            SettingScope::Tenant => "TENANT",
            SettingScope::Shop => "SHOP",
            SettingScope::Warehouse => "WAREHOUSE",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.to_uppercase().as_str() {
            "GLOBAL" => Some(SettingScope::Global),
            "TENANT" => Some(SettingScope::Tenant),
            "SHOP" => Some(SettingScope::Shop),
            "WAREHOUSE" => Some(SettingScope::Warehouse),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub setting_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub scope_type: String,
    pub scope_id: Option<uuid::Uuid>,
    pub key: String,
    pub value_json: Value,
    pub version: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SettingListFilters {
    pub scope_type: Option<String>,
    pub scope_id: Option<uuid::Uuid>,
    pub key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SettingUpsert {
    pub tenant_id: TenantId,
    pub scope_type: String,
    pub scope_id: Option<uuid::Uuid>,
    pub key: String,
    pub value_json: Value,
    pub expected_version: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct SettingConflict {
    pub message: String,
}

impl std::fmt::Display for SettingConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SettingConflict {}

pub struct SettingsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SettingsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_settings(
        &self,
        tenant_id: TenantId,
        filters: SettingListFilters,
    ) -> anyhow::Result<Vec<Setting>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select setting_id, tenant_id, scope_type, scope_id, key, value_json, version, updated_at
            from settings
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(scope_type) = filters.scope_type {
            qb.push(" and scope_type = ");
            qb.push_bind(scope_type);
        }
        if let Some(scope_id) = filters.scope_id {
            qb.push(" and scope_id = ");
            qb.push_bind(scope_id);
        }
        if let Some(key) = filters.key {
            qb.push(" and key = ");
            qb.push_bind(key);
        }

        qb.push(" order by updated_at desc");
        let rows: Vec<SettingRow> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(SettingRow::into_setting).collect())
    }

    pub async fn upsert_setting(&self, input: SettingUpsert) -> anyhow::Result<Setting> {
        let expected_version = input.expected_version;
        let row: Option<SettingRow> = sqlx::query_as(
            r#"
            insert into settings(tenant_id, scope_type, scope_id, key, value_json, version)
            values ($1, $2, $3, $4, $5, 1)
            on conflict (tenant_id, scope_type, scope_id, key)
            do update set
                value_json = excluded.value_json,
                version = settings.version + 1,
                updated_at = now()
            where ($6::int is null or settings.version = $6)
            returning setting_id, tenant_id, scope_type, scope_id, key, value_json, version, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.scope_type)
        .bind(input.scope_id)
        .bind(&input.key)
        .bind(input.value_json)
        .bind(expected_version)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Err(anyhow!(SettingConflict {
                message: "settings version conflict".to_string(),
            }));
        };
        Ok(row.into_setting())
    }
}

#[derive(sqlx::FromRow)]
struct SettingRow {
    setting_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    scope_type: String,
    scope_id: Option<uuid::Uuid>,
    key: String,
    value_json: Value,
    version: i32,
    updated_at: DateTime<Utc>,
}

impl SettingRow {
    fn into_setting(self) -> Setting {
        Setting {
            setting_id: self.setting_id,
            tenant_id: TenantId(self.tenant_id),
            scope_type: self.scope_type,
            scope_id: self.scope_id,
            key: self.key,
            value_json: self.value_json,
            version: self.version,
            updated_at: self.updated_at,
        }
    }
}
