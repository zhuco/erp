use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderReviewRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub conditions_json: Value,
    pub action: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrderReviewRuleCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub conditions_json: Value,
    pub action: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct OrderReviewRuleUpdate {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub conditions_json: Option<Value>,
    pub action: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OrderReviewRuleListFilters {
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct OrderReviewRuleListResult {
    pub items: Vec<OrderReviewRule>,
    pub total: i64,
}

pub struct OrderReviewRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> OrderReviewRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: OrderReviewRuleListFilters,
    ) -> anyhow::Result<OrderReviewRuleListResult> {
        let total: i64 = match filters.enabled {
            Some(enabled) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from order_review_rules
                    where tenant_id = $1 and enabled = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(enabled)
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from order_review_rules
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
            select
                rule_id, name, enabled, priority, conditions_json,
                action, reason, created_at, updated_at
            from order_review_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(enabled) = filters.enabled {
            qb.push(" and enabled = ");
            qb.push_bind(enabled);
        }
        qb.push(" order by priority desc, updated_at desc, rule_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<OrderReviewRuleRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_rule(tenant_id))
            .collect();

        Ok(OrderReviewRuleListResult { items, total })
    }

    pub async fn list_enabled_rules(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<OrderReviewRule>> {
        let rows: Vec<OrderReviewRuleRow> = sqlx::query_as(
            r#"
            select
                rule_id, name, enabled, priority, conditions_json,
                action, reason, created_at, updated_at
            from order_review_rules
            where tenant_id = $1 and enabled = true
            order by priority desc, updated_at desc, rule_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| row.into_rule(tenant_id))
            .collect())
    }

    pub async fn get_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<OrderReviewRule>> {
        let row: Option<OrderReviewRuleRow> = sqlx::query_as(
            r#"
            select
                rule_id, name, enabled, priority, conditions_json,
                action, reason, created_at, updated_at
            from order_review_rules
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
        input: OrderReviewRuleCreate,
    ) -> anyhow::Result<OrderReviewRule> {
        let row: OrderReviewRuleRow = sqlx::query_as(
            r#"
            insert into order_review_rules(
                tenant_id, name, enabled, priority, conditions_json, action, reason
            ) values ($1,$2,$3,$4,$5,$6,$7)
            on conflict (tenant_id, name)
            do update set
                enabled = excluded.enabled,
                priority = excluded.priority,
                conditions_json = excluded.conditions_json,
                action = excluded.action,
                reason = excluded.reason,
                updated_at = now()
            returning
                rule_id, name, enabled, priority, conditions_json,
                action, reason, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.enabled)
        .bind(input.priority)
        .bind(input.conditions_json)
        .bind(&input.action)
        .bind(&input.reason)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_rule(input.tenant_id))
    }

    pub async fn update_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: OrderReviewRuleUpdate,
    ) -> anyhow::Result<Option<OrderReviewRule>> {
        let mut qb = QueryBuilder::<Postgres>::new("update order_review_rules set ");
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
        if let Some(conditions_json) = input.conditions_json {
            separated.push("conditions_json = ");
            separated.push_bind(conditions_json);
        }
        if let Some(action) = input.action {
            separated.push("action = ");
            separated.push_bind(action);
        }
        if let Some(reason) = input.reason {
            separated.push("reason = ");
            separated.push_bind(reason);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(" returning rule_id, name, enabled, priority, conditions_json, action, reason, created_at, updated_at");

        let row: Option<OrderReviewRuleRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_rule(tenant_id)))
    }

    pub async fn delete_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<OrderReviewRule>> {
        let row: Option<OrderReviewRuleRow> = sqlx::query_as(
            r#"
            delete from order_review_rules
            where tenant_id = $1 and rule_id = $2
            returning rule_id, name, enabled, priority, conditions_json, action, reason, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_rule(tenant_id)))
    }
}

#[derive(sqlx::FromRow)]
struct OrderReviewRuleRow {
    rule_id: uuid::Uuid,
    name: String,
    enabled: bool,
    priority: i32,
    conditions_json: Value,
    action: String,
    reason: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl OrderReviewRuleRow {
    fn into_rule(self, tenant_id: TenantId) -> OrderReviewRule {
        OrderReviewRule {
            rule_id: self.rule_id,
            tenant_id,
            name: self.name,
            enabled: self.enabled,
            priority: self.priority,
            conditions_json: self.conditions_json,
            action: self.action,
            reason: self.reason,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
