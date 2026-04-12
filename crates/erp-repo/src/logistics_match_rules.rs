use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsMatchRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub product_id: Option<uuid::Uuid>,
    pub conditions_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LogisticsMatchRuleCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub product_id: Option<uuid::Uuid>,
    pub conditions_json: Value,
}

#[derive(Debug, Clone)]
pub struct LogisticsMatchRuleUpdate {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub product_id: Option<Option<uuid::Uuid>>,
    pub conditions_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct LogisticsMatchRuleListFilters {
    pub keyword: Option<String>,
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct LogisticsMatchRuleListResult {
    pub items: Vec<LogisticsMatchRule>,
    pub total: i64,
}

pub struct LogisticsMatchRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsMatchRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: LogisticsMatchRuleListFilters,
    ) -> anyhow::Result<LogisticsMatchRuleListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from logistics_match_rules where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                rule_id, name, enabled, priority, product_id,
                conditions_json, created_at, updated_at
            from logistics_match_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by priority desc, updated_at desc, rule_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            name: String,
            enabled: bool,
            priority: i32,
            product_id: Option<uuid::Uuid>,
            conditions_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| LogisticsMatchRule {
                rule_id: row.rule_id,
                tenant_id,
                name: row.name,
                enabled: row.enabled,
                priority: row.priority,
                product_id: row.product_id,
                conditions_json: row.conditions_json,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(LogisticsMatchRuleListResult { items, total })
    }

    pub async fn list_enabled_rules(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<LogisticsMatchRule>> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                rule_id, name, enabled, priority, product_id,
                conditions_json, created_at, updated_at
            from logistics_match_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and enabled = true");
        qb.push(" order by priority desc, updated_at desc, rule_id desc");

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            name: String,
            enabled: bool,
            priority: i32,
            product_id: Option<uuid::Uuid>,
            conditions_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| LogisticsMatchRule {
                rule_id: row.rule_id,
                tenant_id,
                name: row.name,
                enabled: row.enabled,
                priority: row.priority,
                product_id: row.product_id,
                conditions_json: row.conditions_json,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn get_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<LogisticsMatchRule>> {
        let row = sqlx::query_as::<_, RuleRow>(
            r#"
            select
                rule_id, name, enabled, priority, product_id,
                conditions_json, created_at, updated_at
            from logistics_match_rules
            where tenant_id = $1 and rule_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_rule(tenant_id)))
    }

    pub async fn create_rule(
        &self,
        input: LogisticsMatchRuleCreate,
    ) -> anyhow::Result<LogisticsMatchRule> {
        let row = sqlx::query_as::<_, RuleRow>(
            r#"
            insert into logistics_match_rules(
                tenant_id, name, enabled, priority, product_id, conditions_json
            ) values ($1,$2,$3,$4,$5,$6)
            on conflict (tenant_id, name)
            do update set
                enabled = excluded.enabled,
                priority = excluded.priority,
                product_id = excluded.product_id,
                conditions_json = excluded.conditions_json,
                updated_at = now()
            returning
                rule_id, name, enabled, priority, product_id,
                conditions_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.enabled)
        .bind(input.priority)
        .bind(input.product_id)
        .bind(input.conditions_json)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_rule(input.tenant_id))
    }

    pub async fn update_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: LogisticsMatchRuleUpdate,
    ) -> anyhow::Result<Option<LogisticsMatchRule>> {
        let mut qb = QueryBuilder::<Postgres>::new("update logistics_match_rules set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
        }
        if let Some(priority) = input.priority {
            separated.push("priority = ");
            separated.push_bind(priority);
        }
        if let Some(product_id) = input.product_id {
            separated.push("product_id = ");
            separated.push_bind(product_id);
        }
        if let Some(conditions_json) = input.conditions_json {
            separated.push("conditions_json = ");
            separated.push_bind(conditions_json);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(
            " returning rule_id, name, enabled, priority, product_id, conditions_json, created_at, updated_at",
        );

        let row: Option<RuleRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_rule(tenant_id)))
    }

    pub async fn delete_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<LogisticsMatchRule>> {
        let row = sqlx::query_as::<_, RuleRow>(
            r#"
            delete from logistics_match_rules
            where tenant_id = $1 and rule_id = $2
            returning
                rule_id, name, enabled, priority, product_id,
                conditions_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_rule(tenant_id)))
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &LogisticsMatchRuleListFilters) {
    if let Some(enabled) = filters.enabled {
        qb.push(" and enabled = ");
        qb.push_bind(enabled);
    }
    if let Some(keyword) = filters.keyword.as_ref() {
        let like = format!("%{}%", keyword.trim());
        qb.push(" and name ilike ");
        qb.push_bind(like);
    }
}

#[derive(sqlx::FromRow)]
struct RuleRow {
    rule_id: uuid::Uuid,
    name: String,
    enabled: bool,
    priority: i32,
    product_id: Option<uuid::Uuid>,
    conditions_json: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl RuleRow {
    fn into_rule(self, tenant_id: TenantId) -> LogisticsMatchRule {
        LogisticsMatchRule {
            rule_id: self.rule_id,
            tenant_id,
            name: self.name,
            enabled: self.enabled,
            priority: self.priority,
            product_id: self.product_id,
            conditions_json: self.conditions_json,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
