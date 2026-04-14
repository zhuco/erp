use chrono::{DateTime, Utc};
use erp_types::{Platform, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub entity_type: String,
    pub version: i32,
    pub enabled: bool,
    pub rule_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MappingRuleListFilters {
    pub platform: Option<Platform>,
    pub entity_type: Option<String>,
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

pub struct MappingRuleListResult {
    pub items: Vec<MappingRule>,
    pub total: i64,
}

pub struct MappingRuleCreate {
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub entity_type: String,
    pub version: i32,
    pub enabled: Option<bool>,
    pub rule_json: Value,
}

pub struct MappingRuleUpdate {
    pub enabled: Option<bool>,
    pub rule_json: Option<Value>,
}

pub struct MappingRulesRepo<'a> {
    pool: &'a PgPool,
}

fn normalize_entity_type(value: &str) -> anyhow::Result<String> {
    let normalized = value.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        anyhow::bail!("entity_type is required");
    }
    Ok(normalized)
}

fn validate_version(version: i32) -> anyhow::Result<()> {
    if version <= 0 {
        anyhow::bail!("version must be greater than 0");
    }
    Ok(())
}

impl<'a> MappingRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: MappingRuleListFilters,
    ) -> anyhow::Result<MappingRuleListResult> {
        let mut total_qb =
            QueryBuilder::<Postgres>::new("select count(*) from mapping_rules where tenant_id = ");
        total_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(platform) = filters.platform {
            total_qb.push(" and platform = ");
            total_qb.push_bind(platform.to_string());
        }
        if let Some(entity_type) = &filters.entity_type {
            total_qb.push(" and entity_type = ");
            total_qb.push_bind(entity_type);
        }
        if let Some(enabled) = filters.enabled {
            total_qb.push(" and enabled = ");
            total_qb.push_bind(enabled);
        }
        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select rule_id, platform, entity_type, version, enabled, rule_json, created_at, updated_at
            from mapping_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(platform) = filters.platform {
            qb.push(" and platform = ");
            qb.push_bind(platform.to_string());
        }
        if let Some(entity_type) = filters.entity_type {
            qb.push(" and entity_type = ");
            qb.push_bind(entity_type);
        }
        if let Some(enabled) = filters.enabled {
            qb.push(" and enabled = ");
            qb.push_bind(enabled);
        }
        qb.push(" order by updated_at desc, rule_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            platform: String,
            entity_type: String,
            version: i32,
            enabled: bool,
            rule_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| {
                let platform = Platform::from_str(&row.platform)
                    .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
                Ok(MappingRule {
                    rule_id: row.rule_id,
                    tenant_id,
                    platform,
                    entity_type: row.entity_type,
                    version: row.version,
                    enabled: row.enabled,
                    rule_json: row.rule_json,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(MappingRuleListResult { items, total })
    }

    pub async fn create_rule(&self, input: MappingRuleCreate) -> anyhow::Result<MappingRule> {
        let entity_type = normalize_entity_type(&input.entity_type)?;
        validate_version(input.version)?;
        let enabled = input.enabled.unwrap_or(true);
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            platform: String,
            entity_type: String,
            version: i32,
            enabled: bool,
            rule_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let mut tx = self.pool.begin().await?;
        if enabled {
            sqlx::query(
                r#"
                update mapping_rules
                set enabled = false, updated_at = now()
                where tenant_id = $1 and platform = $2 and entity_type = $3 and enabled = true
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.platform.to_string())
            .bind(&entity_type)
            .execute(&mut *tx)
            .await?;
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into mapping_rules (
              tenant_id, platform, entity_type, version, enabled, rule_json
            ) values ($1,$2,$3,$4,$5,$6)
            returning rule_id, platform, entity_type, version, enabled, rule_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.platform.to_string())
        .bind(&entity_type)
        .bind(input.version)
        .bind(enabled)
        .bind(input.rule_json)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        let platform = Platform::from_str(&row.platform)
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
        Ok(MappingRule {
            rule_id: row.rule_id,
            tenant_id: input.tenant_id,
            platform,
            entity_type: row.entity_type,
            version: row.version,
            enabled: row.enabled,
            rule_json: row.rule_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: MappingRuleUpdate,
    ) -> anyhow::Result<Option<MappingRule>> {
        if input.enabled.is_none() && input.rule_json.is_none() {
            return Ok(None);
        }
        let mut tx = self.pool.begin().await?;
        if matches!(input.enabled, Some(true)) {
            #[derive(sqlx::FromRow)]
            struct RuleScope {
                platform: String,
                entity_type: String,
            }

            let scope: Option<RuleScope> = sqlx::query_as(
                r#"
                select platform, entity_type
                from mapping_rules
                where tenant_id = $1 and rule_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(rule_id)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(scope) = scope else {
                tx.rollback().await?;
                return Ok(None);
            };

            sqlx::query(
                r#"
                update mapping_rules
                set enabled = false, updated_at = now()
                where tenant_id = $1
                  and platform = $2
                  and entity_type = $3
                  and rule_id <> $4
                  and enabled = true
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(scope.platform)
            .bind(scope.entity_type)
            .bind(rule_id)
            .execute(&mut *tx)
            .await?;
        }
        let mut qb = QueryBuilder::<Postgres>::new("update mapping_rules set ");
        let mut separated = qb.separated(", ");
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
        }
        if let Some(rule_json) = input.rule_json {
            separated.push("rule_json = ");
            separated.push_bind(rule_json);
        }
        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(
            " returning rule_id, platform, entity_type, version, enabled, rule_json, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            platform: String,
            entity_type: String,
            version: i32,
            enabled: bool,
            rule_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(&mut *tx).await?;
        let Some(row) = row else {
            tx.rollback().await?;
            return Ok(None);
        };
        tx.commit().await?;
        let platform = Platform::from_str(&row.platform)
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
        Ok(Some(MappingRule {
            rule_id: row.rule_id,
            tenant_id,
            platform,
            entity_type: row.entity_type,
            version: row.version,
            enabled: row.enabled,
            rule_json: row.rule_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn latest_enabled_version(
        &self,
        tenant_id: TenantId,
        platform: Platform,
        entity_type: &str,
    ) -> anyhow::Result<Option<i32>> {
        let version: Option<i32> = sqlx::query_scalar(
            r#"
            select version
            from mapping_rules
            where tenant_id = $1 and platform = $2 and entity_type = $3 and enabled = true
            order by version desc, updated_at desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(platform.to_string())
        .bind(entity_type)
        .fetch_optional(self.pool)
            .await?;

        Ok(version)
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_entity_type, validate_version};

    #[test]
    fn normalize_entity_type_trims_and_uppercases() {
        let value = normalize_entity_type("  shop_product  ").expect("should normalize");
        assert_eq!(value, "SHOP_PRODUCT");
    }

    #[test]
    fn normalize_entity_type_rejects_empty() {
        let err = normalize_entity_type("   ").expect_err("empty entity_type should fail");
        assert!(err.to_string().contains("entity_type is required"));
    }

    #[test]
    fn validate_version_rejects_non_positive_values() {
        let err = validate_version(0).expect_err("zero version should fail");
        assert!(err.to_string().contains("greater than 0"));
    }
}
