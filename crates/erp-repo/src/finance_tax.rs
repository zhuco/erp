use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub tax_code: String,
    pub tax_rate: f64,
    pub effective_from: Option<NaiveDate>,
    pub effective_to: Option<NaiveDate>,
    pub status: String,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct TaxRuleListFilters {
    pub tax_code: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct TaxRuleListResult {
    pub items: Vec<TaxRule>,
    pub total: i64,
}

pub struct TaxRuleCreate {
    pub tenant_id: TenantId,
    pub tax_code: String,
    pub tax_rate: f64,
    pub effective_from: Option<NaiveDate>,
    pub effective_to: Option<NaiveDate>,
    pub status: String,
    pub created_by: Option<String>,
}

pub struct TaxRuleUpdate {
    pub tax_code: Option<String>,
    pub tax_rate: Option<f64>,
    pub effective_from: Option<NaiveDate>,
    pub effective_to: Option<NaiveDate>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceAdjustment {
    pub adjustment_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub invoice_id: uuid::Uuid,
    pub adjustment_type: String,
    pub reason: Option<String>,
    pub status: String,
    pub applied_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct InvoiceAdjustmentListFilters {
    pub status: Option<String>,
    pub invoice_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

pub struct InvoiceAdjustmentListResult {
    pub items: Vec<InvoiceAdjustment>,
    pub total: i64,
}

pub struct InvoiceAdjustmentCreate {
    pub tenant_id: TenantId,
    pub invoice_id: uuid::Uuid,
    pub adjustment_type: String,
    pub reason: Option<String>,
    pub status: String,
    pub created_by: Option<String>,
}

pub struct InvoiceAdjustmentsRepo<'a> {
    pool: &'a PgPool,
}

pub struct TaxRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> TaxRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: TaxRuleListFilters,
    ) -> anyhow::Result<TaxRuleListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from tax_rules
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_tax_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select rule_id, tax_code, tax_rate, effective_from, effective_to, status, created_by,
                   created_at, updated_at
            from tax_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_tax_filters(&mut qb, &filters);
        qb.push(" order by effective_from desc nulls last, created_at desc, rule_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            tax_code: String,
            tax_rate: f64,
            effective_from: Option<NaiveDate>,
            effective_to: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| TaxRule {
                rule_id: row.rule_id,
                tenant_id,
                tax_code: row.tax_code,
                tax_rate: row.tax_rate,
                effective_from: row.effective_from,
                effective_to: row.effective_to,
                status: row.status,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(TaxRuleListResult { items, total })
    }

    pub async fn create_rule(&self, input: TaxRuleCreate) -> anyhow::Result<TaxRule> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            tax_code: String,
            tax_rate: f64,
            effective_from: Option<NaiveDate>,
            effective_to: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into tax_rules(
                tenant_id, tax_code, tax_rate, effective_from, effective_to, status, created_by
            ) values ($1,$2,$3,$4,$5,$6,$7)
            returning rule_id, tax_code, tax_rate, effective_from, effective_to, status, created_by,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.tax_code)
        .bind(input.tax_rate)
        .bind(input.effective_from)
        .bind(input.effective_to)
        .bind(&input.status)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(TaxRule {
            rule_id: row.rule_id,
            tenant_id: input.tenant_id,
            tax_code: row.tax_code,
            tax_rate: row.tax_rate,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: TaxRuleUpdate,
    ) -> anyhow::Result<Option<TaxRule>> {
        let mut qb = QueryBuilder::<Postgres>::new("update tax_rules set ");
        let mut separated = qb.separated(", ");

        if let Some(tax_code) = input.tax_code {
            separated.push("tax_code = ");
            separated.push_bind(tax_code);
        }
        if let Some(tax_rate) = input.tax_rate {
            separated.push("tax_rate = ");
            separated.push_bind(tax_rate);
        }
        if let Some(effective_from) = input.effective_from {
            separated.push("effective_from = ");
            separated.push_bind(effective_from);
        }
        if let Some(effective_to) = input.effective_to {
            separated.push("effective_to = ");
            separated.push_bind(effective_to);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(
            r#"
            returning rule_id, tax_code, tax_rate, effective_from, effective_to, status, created_by,
                      created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            tax_code: String,
            tax_rate: f64,
            effective_from: Option<NaiveDate>,
            effective_to: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| TaxRule {
            rule_id: row.rule_id,
            tenant_id,
            tax_code: row.tax_code,
            tax_rate: row.tax_rate,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<TaxRule>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            tax_code: String,
            tax_rate: f64,
            effective_from: Option<NaiveDate>,
            effective_to: Option<NaiveDate>,
            status: String,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from tax_rules
            where tenant_id = $1 and rule_id = $2
            returning rule_id, tax_code, tax_rate, effective_from, effective_to, status, created_by,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| TaxRule {
            rule_id: row.rule_id,
            tenant_id,
            tax_code: row.tax_code,
            tax_rate: row.tax_rate,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            status: row.status,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

impl<'a> InvoiceAdjustmentsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_adjustments(
        &self,
        tenant_id: TenantId,
        filters: InvoiceAdjustmentListFilters,
    ) -> anyhow::Result<InvoiceAdjustmentListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from invoice_adjustments
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_adjustment_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select adjustment_id, invoice_id, adjustment_type, reason, status, applied_at,
                   created_by, created_at, updated_at
            from invoice_adjustments
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_adjustment_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, adjustment_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            adjustment_id: uuid::Uuid,
            invoice_id: uuid::Uuid,
            adjustment_type: String,
            reason: Option<String>,
            status: String,
            applied_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| InvoiceAdjustment {
                adjustment_id: row.adjustment_id,
                tenant_id,
                invoice_id: row.invoice_id,
                adjustment_type: row.adjustment_type,
                reason: row.reason,
                status: row.status,
                applied_at: row.applied_at,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(InvoiceAdjustmentListResult { items, total })
    }

    pub async fn create_adjustment(
        &self,
        input: InvoiceAdjustmentCreate,
    ) -> anyhow::Result<InvoiceAdjustment> {
        #[derive(sqlx::FromRow)]
        struct Row {
            adjustment_id: uuid::Uuid,
            invoice_id: uuid::Uuid,
            adjustment_type: String,
            reason: Option<String>,
            status: String,
            applied_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into invoice_adjustments(
                tenant_id, invoice_id, adjustment_type, reason, status, created_by
            ) values ($1,$2,$3,$4,$5,$6)
            returning adjustment_id, invoice_id, adjustment_type, reason, status, applied_at,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.invoice_id)
        .bind(&input.adjustment_type)
        .bind(&input.reason)
        .bind(&input.status)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(InvoiceAdjustment {
            adjustment_id: row.adjustment_id,
            tenant_id: input.tenant_id,
            invoice_id: row.invoice_id,
            adjustment_type: row.adjustment_type,
            reason: row.reason,
            status: row.status,
            applied_at: row.applied_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn mark_applied(
        &self,
        tenant_id: TenantId,
        adjustment_id: uuid::Uuid,
    ) -> anyhow::Result<Option<InvoiceAdjustment>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            adjustment_id: uuid::Uuid,
            invoice_id: uuid::Uuid,
            adjustment_type: String,
            reason: Option<String>,
            status: String,
            applied_at: Option<DateTime<Utc>>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update invoice_adjustments
            set status = $3,
                applied_at = now(),
                updated_at = now()
            where tenant_id = $1 and adjustment_id = $2
            returning adjustment_id, invoice_id, adjustment_type, reason, status, applied_at,
                      created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(adjustment_id)
        .bind("APPLIED")
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| InvoiceAdjustment {
            adjustment_id: row.adjustment_id,
            tenant_id,
            invoice_id: row.invoice_id,
            adjustment_type: row.adjustment_type,
            reason: row.reason,
            status: row.status,
            applied_at: row.applied_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_tax_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a TaxRuleListFilters) {
    if let Some(tax_code) = &filters.tax_code {
        qb.push(" and tax_code = ");
        qb.push_bind(tax_code);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
}

fn apply_adjustment_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a InvoiceAdjustmentListFilters,
) {
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
    if let Some(invoice_id) = filters.invoice_id {
        qb.push(" and invoice_id = ");
        qb.push_bind(invoice_id);
    }
}
