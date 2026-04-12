use chrono::{DateTime, NaiveDate, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsSettlementRule {
    pub tenant_id: TenantId,
    pub weight_rounding: String,
    pub min_fee_cent: i64,
    pub oversize_fee_cent: i64,
    pub overtime_fee_cent: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LogisticsSettlementRuleUpsert {
    pub tenant_id: TenantId,
    pub weight_rounding: String,
    pub min_fee_cent: i64,
    pub oversize_fee_cent: i64,
    pub overtime_fee_cent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsSettlementBatch {
    pub batch_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub carrier: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_shipments: i32,
    pub expected_cent: i64,
    pub actual_cent: i64,
    pub diff_cent: i64,
    pub status: String,
    pub deductions: Vec<String>,
    pub payment_plan_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LogisticsSettlementBatchListFilters {
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct LogisticsSettlementBatchListResult {
    pub items: Vec<LogisticsSettlementBatch>,
    pub total: i64,
}

pub struct LogisticsSettlementRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsSettlementRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_rule(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Option<LogisticsSettlementRule>> {
        let row: Option<LogisticsSettlementRuleRow> = sqlx::query_as(
            r#"
            select weight_rounding, min_fee_cent, oversize_fee_cent, overtime_fee_cent,
                   created_at, updated_at
            from logistics_settlement_rules
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_rule(tenant_id)))
    }

    pub async fn upsert_rule(
        &self,
        input: LogisticsSettlementRuleUpsert,
    ) -> anyhow::Result<LogisticsSettlementRule> {
        let row: LogisticsSettlementRuleRow = sqlx::query_as(
            r#"
            insert into logistics_settlement_rules(
                tenant_id, weight_rounding, min_fee_cent, oversize_fee_cent, overtime_fee_cent
            ) values ($1, $2, $3, $4, $5)
            on conflict (tenant_id)
            do update set
                weight_rounding = excluded.weight_rounding,
                min_fee_cent = excluded.min_fee_cent,
                oversize_fee_cent = excluded.oversize_fee_cent,
                overtime_fee_cent = excluded.overtime_fee_cent,
                updated_at = now()
            returning weight_rounding, min_fee_cent, oversize_fee_cent, overtime_fee_cent,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.weight_rounding)
        .bind(input.min_fee_cent)
        .bind(input.oversize_fee_cent)
        .bind(input.overtime_fee_cent)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_rule(input.tenant_id))
    }
}

pub struct LogisticsSettlementBatchesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsSettlementBatchesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_batches(
        &self,
        tenant_id: TenantId,
        filters: LogisticsSettlementBatchListFilters,
    ) -> anyhow::Result<LogisticsSettlementBatchListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from logistics_settlement_batches where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                batch_id, carrier, period_start, period_end, total_shipments,
                expected_cent, actual_cent, diff_cent, status, deductions,
                payment_plan_id, created_at
            from logistics_settlement_batches
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        qb.push(" order by period_end desc, batch_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        let rows: Vec<LogisticsSettlementBatchRow> =
            qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_batch(tenant_id))
            .collect();

        Ok(LogisticsSettlementBatchListResult { items, total })
    }

    pub async fn confirm_batch(
        &self,
        tenant_id: TenantId,
        batch_id: uuid::Uuid,
        payment_plan_id: Option<String>,
    ) -> anyhow::Result<Option<LogisticsSettlementBatch>> {
        let row: Option<LogisticsSettlementBatchRow> = sqlx::query_as(
            r#"
            update logistics_settlement_batches
            set status = 'CONFIRMED',
                payment_plan_id = coalesce(payment_plan_id, $3)
            where tenant_id = $1 and batch_id = $2
            returning
                batch_id, carrier, period_start, period_end, total_shipments,
                expected_cent, actual_cent, diff_cent, status, deductions,
                payment_plan_id, created_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(batch_id)
        .bind(&payment_plan_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_batch(tenant_id)))
    }
}

#[derive(sqlx::FromRow)]
struct LogisticsSettlementRuleRow {
    weight_rounding: String,
    min_fee_cent: i64,
    oversize_fee_cent: i64,
    overtime_fee_cent: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl LogisticsSettlementRuleRow {
    fn into_rule(self, tenant_id: TenantId) -> LogisticsSettlementRule {
        LogisticsSettlementRule {
            tenant_id,
            weight_rounding: self.weight_rounding,
            min_fee_cent: self.min_fee_cent,
            oversize_fee_cent: self.oversize_fee_cent,
            overtime_fee_cent: self.overtime_fee_cent,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct LogisticsSettlementBatchRow {
    batch_id: uuid::Uuid,
    carrier: String,
    period_start: NaiveDate,
    period_end: NaiveDate,
    total_shipments: i32,
    expected_cent: i64,
    actual_cent: i64,
    diff_cent: i64,
    status: String,
    deductions: Vec<String>,
    payment_plan_id: Option<String>,
    created_at: DateTime<Utc>,
}

impl LogisticsSettlementBatchRow {
    fn into_batch(self, tenant_id: TenantId) -> LogisticsSettlementBatch {
        LogisticsSettlementBatch {
            batch_id: self.batch_id,
            tenant_id,
            carrier: self.carrier,
            period_start: self.period_start,
            period_end: self.period_end,
            total_shipments: self.total_shipments,
            expected_cent: self.expected_cent,
            actual_cent: self.actual_cent,
            diff_cent: self.diff_cent,
            status: self.status,
            deductions: self.deductions,
            payment_plan_id: self.payment_plan_id,
            created_at: self.created_at,
        }
    }
}
