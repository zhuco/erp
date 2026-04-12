use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{AfterSalesType, TenantId};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct AfterSalesSlaRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub after_sales_type: AfterSalesType,
    pub response_hours: i32,
    pub resolution_hours: i32,
    pub escalation_role: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AfterSalesSlaRuleListFilters {
    pub after_sales_type: Option<AfterSalesType>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct AfterSalesSlaRuleListResult {
    pub items: Vec<AfterSalesSlaRule>,
    pub total: i64,
}

pub struct AfterSalesSlaRuleCreate {
    pub tenant_id: TenantId,
    pub after_sales_type: AfterSalesType,
    pub response_hours: i32,
    pub resolution_hours: i32,
    pub escalation_role: Option<String>,
    pub status: String,
}

pub struct AfterSalesSlaRuleUpdate {
    pub after_sales_type: Option<AfterSalesType>,
    pub response_hours: Option<i32>,
    pub resolution_hours: Option<i32>,
    pub escalation_role: Option<String>,
    pub status: Option<String>,
}

pub struct AfterSalesSlaRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AfterSalesSlaRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: AfterSalesSlaRuleListFilters,
    ) -> anyhow::Result<AfterSalesSlaRuleListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                rule_id, after_sales_type, response_hours, resolution_hours,
                escalation_role, status, created_at, updated_at
            from after_sales_sla_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(after_sales_type) = filters.after_sales_type {
            qb.push(" and after_sales_type = ");
            qb.push_bind(after_sales_type.to_string());
        }
        if let Some(status) = filters.status.as_ref() {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        qb.push(" order by updated_at desc, rule_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<AfterSalesSlaRuleRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_rule(tenant_id))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from after_sales_sla_rules where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(after_sales_type) = filters.after_sales_type {
            count_qb.push(" and after_sales_type = ");
            count_qb.push_bind(after_sales_type.to_string());
        }
        if let Some(status) = filters.status.as_ref() {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(AfterSalesSlaRuleListResult { items, total })
    }

    pub async fn create_rule(
        &self,
        input: AfterSalesSlaRuleCreate,
    ) -> anyhow::Result<AfterSalesSlaRule> {
        let row: AfterSalesSlaRuleRow = sqlx::query_as(
            r#"
            insert into after_sales_sla_rules(
                tenant_id, after_sales_type, response_hours, resolution_hours,
                escalation_role, status
            ) values ($1,$2,$3,$4,$5,$6)
            returning
                rule_id, after_sales_type, response_hours, resolution_hours,
                escalation_role, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.after_sales_type.to_string())
        .bind(input.response_hours)
        .bind(input.resolution_hours)
        .bind(input.escalation_role)
        .bind(input.status)
        .fetch_one(self.pool)
        .await?;

        row.into_rule(input.tenant_id)
    }

    pub async fn update_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        update: AfterSalesSlaRuleUpdate,
    ) -> anyhow::Result<Option<AfterSalesSlaRule>> {
        let mut qb = QueryBuilder::<Postgres>::new("update after_sales_sla_rules set ");
        let mut separated = qb.separated(", ");
        let mut has_update = false;

        if let Some(after_sales_type) = update.after_sales_type {
            separated.push("after_sales_type = ");
            separated.push_bind(after_sales_type.to_string());
            has_update = true;
        }
        if let Some(response_hours) = update.response_hours {
            separated.push("response_hours = ");
            separated.push_bind(response_hours);
            has_update = true;
        }
        if let Some(resolution_hours) = update.resolution_hours {
            separated.push("resolution_hours = ");
            separated.push_bind(resolution_hours);
            has_update = true;
        }
        if let Some(escalation_role) = update.escalation_role {
            separated.push("escalation_role = ");
            separated.push_bind(escalation_role);
            has_update = true;
        }
        if let Some(status) = update.status {
            separated.push("status = ");
            separated.push_bind(status);
            has_update = true;
        }

        if !has_update {
            return Err(anyhow!("no fields to update"));
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(
            " returning rule_id, after_sales_type, response_hours, resolution_hours,
              escalation_role, status, created_at, updated_at",
        );

        let row: Option<AfterSalesSlaRuleRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        match row {
            Some(row) => Ok(Some(row.into_rule(tenant_id)?)),
            None => Ok(None),
        }
    }

    pub async fn delete_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<AfterSalesSlaRule>> {
        let row: Option<AfterSalesSlaRuleRow> = sqlx::query_as(
            r#"
            delete from after_sales_sla_rules
            where tenant_id = $1 and rule_id = $2
            returning
                rule_id, after_sales_type, response_hours, resolution_hours,
                escalation_role, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row.into_rule(tenant_id)?)),
            None => Ok(None),
        }
    }
}

#[derive(sqlx::FromRow)]
struct AfterSalesSlaRuleRow {
    rule_id: uuid::Uuid,
    after_sales_type: String,
    response_hours: i32,
    resolution_hours: i32,
    escalation_role: Option<String>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AfterSalesSlaRuleRow {
    fn into_rule(self, tenant_id: TenantId) -> anyhow::Result<AfterSalesSlaRule> {
        let after_sales_type = AfterSalesType::from_str(&self.after_sales_type)
            .map_err(|e| anyhow!("bad after_sales_type: {e}"))?;
        Ok(AfterSalesSlaRule {
            rule_id: self.rule_id,
            tenant_id,
            after_sales_type,
            response_hours: self.response_hours,
            resolution_hours: self.resolution_hours,
            escalation_role: self.escalation_role,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
