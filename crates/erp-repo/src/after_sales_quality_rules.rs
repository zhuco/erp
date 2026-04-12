use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{AfterSalesType, TenantId};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct AfterSalesQualityRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub after_sales_type: AfterSalesType,
    pub result_code: String,
    pub status: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AfterSalesQualityRuleListFilters {
    pub after_sales_type: Option<AfterSalesType>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct AfterSalesQualityRuleListResult {
    pub items: Vec<AfterSalesQualityRule>,
    pub total: i64,
}

pub struct AfterSalesQualityRuleCreate {
    pub tenant_id: TenantId,
    pub after_sales_type: AfterSalesType,
    pub result_code: String,
    pub status: String,
    pub sort_order: i32,
}

pub struct AfterSalesQualityRuleUpdate {
    pub tenant_id: TenantId,
    pub rule_id: uuid::Uuid,
    pub after_sales_type: AfterSalesType,
    pub result_code: String,
    pub status: String,
    pub sort_order: i32,
}

pub struct AfterSalesQualityRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AfterSalesQualityRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: AfterSalesQualityRuleListFilters,
    ) -> anyhow::Result<AfterSalesQualityRuleListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                rule_id, after_sales_type, result_code, status, sort_order,
                created_at, updated_at
            from after_sales_quality_rules
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
        qb.push(" order by sort_order asc, updated_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<AfterSalesQualityRuleRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_rule(tenant_id))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from after_sales_quality_rules where tenant_id = ",
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

        Ok(AfterSalesQualityRuleListResult { items, total })
    }

    pub async fn get_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<AfterSalesQualityRule>> {
        let row = sqlx::query_as::<_, AfterSalesQualityRuleRow>(
            r#"
            select
                rule_id, after_sales_type, result_code, status, sort_order,
                created_at, updated_at
            from after_sales_quality_rules
            where tenant_id = $1 and rule_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(row.into_rule(tenant_id)?),
            None => None,
        })
    }

    pub async fn create_rule(
        &self,
        input: AfterSalesQualityRuleCreate,
    ) -> anyhow::Result<AfterSalesQualityRule> {
        if input.result_code.trim().is_empty() {
            return Err(anyhow!("result_code required"));
        }
        let row: AfterSalesQualityRuleRow = sqlx::query_as(
            r#"
            insert into after_sales_quality_rules(
                tenant_id, after_sales_type, result_code, status, sort_order
            ) values ($1,$2,$3,$4,$5)
            returning
                rule_id, after_sales_type, result_code, status, sort_order,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.after_sales_type.to_string())
        .bind(input.result_code.trim())
        .bind(input.status)
        .bind(input.sort_order)
        .fetch_one(self.pool)
        .await?;

        row.into_rule(input.tenant_id)
    }

    pub async fn update_rule(
        &self,
        input: AfterSalesQualityRuleUpdate,
    ) -> anyhow::Result<AfterSalesQualityRule> {
        if input.result_code.trim().is_empty() {
            return Err(anyhow!("result_code required"));
        }
        let row: AfterSalesQualityRuleRow = sqlx::query_as(
            r#"
            update after_sales_quality_rules
            set after_sales_type = $3,
                result_code = $4,
                status = $5,
                sort_order = $6,
                updated_at = now()
            where tenant_id = $1 and rule_id = $2
            returning
                rule_id, after_sales_type, result_code, status, sort_order,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.rule_id)
        .bind(input.after_sales_type.to_string())
        .bind(input.result_code.trim())
        .bind(input.status)
        .bind(input.sort_order)
        .fetch_one(self.pool)
        .await?;

        row.into_rule(input.tenant_id)
    }

    pub async fn delete_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            delete from after_sales_quality_rules
            where tenant_id = $1 and rule_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct AfterSalesQualityRuleRow {
    rule_id: uuid::Uuid,
    after_sales_type: String,
    result_code: String,
    status: String,
    sort_order: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AfterSalesQualityRuleRow {
    fn into_rule(self, tenant_id: TenantId) -> anyhow::Result<AfterSalesQualityRule> {
        let after_sales_type = AfterSalesType::from_str(&self.after_sales_type)
            .map_err(|e| anyhow!("bad after_sales_type: {e}"))?;
        Ok(AfterSalesQualityRule {
            rule_id: self.rule_id,
            tenant_id,
            after_sales_type,
            result_code: self.result_code,
            status: self.status,
            sort_order: self.sort_order,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
