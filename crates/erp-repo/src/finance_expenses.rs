use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinanceExpense {
    pub expense_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub category: String,
    pub amount_cent: i64,
    pub currency: String,
    pub allocation_rule: Value,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct FinanceExpenseListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct FinanceExpenseListResult {
    pub items: Vec<FinanceExpense>,
    pub total: i64,
}

pub struct FinanceExpenseCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub category: String,
    pub amount_cent: i64,
    pub currency: String,
    pub allocation_rule: Value,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
    pub created_by: Option<String>,
}

pub struct FinanceExpenseUpdate {
    pub name: Option<String>,
    pub category: Option<String>,
    pub amount_cent: Option<i64>,
    pub currency: Option<String>,
    pub allocation_rule: Option<Value>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: Option<String>,
}

pub struct FinanceExpensesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> FinanceExpensesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_expenses(
        &self,
        tenant_id: TenantId,
        filters: FinanceExpenseListFilters,
    ) -> anyhow::Result<FinanceExpenseListResult> {
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
                    from finance_expenses
                    where tenant_id = $1
                      and name ilike $2
                      and ($3::text is null or status = $3)
                      and ($4::text is null or category = $4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .bind(filters.category.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from finance_expenses
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                      and ($3::text is null or category = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .bind(filters.category.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select expense_id, name, category, amount_cent, currency, allocation_rule,
                   period_start, period_end, status, created_by, created_at, updated_at
            from finance_expenses
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
        if let Some(category) = &filters.category {
            qb.push(" and category = ");
            qb.push_bind(category);
        }

        qb.push(" order by period_start desc nulls last, updated_at desc, expense_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            expense_id: uuid::Uuid,
            name: String,
            category: String,
            amount_cent: i64,
            currency: String,
            allocation_rule: Value,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| FinanceExpense {
                expense_id: row.expense_id,
                tenant_id,
                name: row.name,
                category: row.category,
                amount_cent: row.amount_cent,
                currency: row.currency,
                allocation_rule: row.allocation_rule,
                period_start: row.period_start,
                period_end: row.period_end,
                status: row.status,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(FinanceExpenseListResult { items, total })
    }

    pub async fn create_expense(
        &self,
        input: FinanceExpenseCreate,
    ) -> anyhow::Result<FinanceExpense> {
        #[derive(sqlx::FromRow)]
        struct Row {
            expense_id: uuid::Uuid,
            name: String,
            category: String,
            amount_cent: i64,
            currency: String,
            allocation_rule: Value,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into finance_expenses(
                tenant_id, name, category, amount_cent, currency, allocation_rule, period_start,
                period_end, status, created_by
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            returning expense_id, name, category, amount_cent, currency, allocation_rule,
                      period_start, period_end, status, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.category)
        .bind(input.amount_cent)
        .bind(&input.currency)
        .bind(&input.allocation_rule)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(&input.status)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(FinanceExpense {
            expense_id: row.expense_id,
            tenant_id: input.tenant_id,
            name: row.name,
            category: row.category,
            amount_cent: row.amount_cent,
            currency: row.currency,
            allocation_rule: row.allocation_rule,
            period_start: row.period_start,
            period_end: row.period_end,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_expense(
        &self,
        tenant_id: TenantId,
        expense_id: uuid::Uuid,
        input: FinanceExpenseUpdate,
    ) -> anyhow::Result<Option<FinanceExpense>> {
        let mut qb = QueryBuilder::<Postgres>::new("update finance_expenses set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(category) = input.category {
            separated.push("category = ");
            separated.push_bind(category);
        }
        if let Some(amount_cent) = input.amount_cent {
            separated.push("amount_cent = ");
            separated.push_bind(amount_cent);
        }
        if let Some(currency) = input.currency {
            separated.push("currency = ");
            separated.push_bind(currency);
        }
        if let Some(allocation_rule) = input.allocation_rule {
            separated.push("allocation_rule = ");
            separated.push_bind(allocation_rule);
        }
        if let Some(period_start) = input.period_start {
            separated.push("period_start = ");
            separated.push_bind(period_start);
        }
        if let Some(period_end) = input.period_end {
            separated.push("period_end = ");
            separated.push_bind(period_end);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and expense_id = ");
        qb.push_bind(expense_id);
        qb.push(
            r#"
            returning expense_id, name, category, amount_cent, currency, allocation_rule,
                      period_start, period_end, status, created_by, created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            expense_id: uuid::Uuid,
            name: String,
            category: String,
            amount_cent: i64,
            currency: String,
            allocation_rule: Value,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| FinanceExpense {
            expense_id: row.expense_id,
            tenant_id,
            name: row.name,
            category: row.category,
            amount_cent: row.amount_cent,
            currency: row.currency,
            allocation_rule: row.allocation_rule,
            period_start: row.period_start,
            period_end: row.period_end,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_expense(
        &self,
        tenant_id: TenantId,
        expense_id: uuid::Uuid,
    ) -> anyhow::Result<Option<FinanceExpense>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            expense_id: uuid::Uuid,
            name: String,
            category: String,
            amount_cent: i64,
            currency: String,
            allocation_rule: Value,
            period_start: Option<NaiveDate>,
            period_end: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from finance_expenses
            where tenant_id = $1 and expense_id = $2
            returning expense_id, name, category, amount_cent, currency, allocation_rule,
                      period_start, period_end, status, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(expense_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| FinanceExpense {
            expense_id: row.expense_id,
            tenant_id,
            name: row.name,
            category: row.category,
            amount_cent: row.amount_cent,
            currency: row.currency,
            allocation_rule: row.allocation_rule,
            period_start: row.period_start,
            period_end: row.period_end,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
