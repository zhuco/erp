use anyhow::anyhow;
use chrono::{DateTime, NaiveDate, Utc};
use erp_types::{OrderId, Platform, ShopId, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::str::FromStr;

pub const PLATFORM_STATEMENT_STATUS_IMPORTED: &str = "IMPORTED";
pub const PLATFORM_STATEMENT_STATUS_RECONCILING: &str = "RECONCILING";
pub const PLATFORM_STATEMENT_STATUS_RECONCILED: &str = "RECONCILED";
pub const PLATFORM_STATEMENT_STATUS_FAILED: &str = "FAILED";

pub const PAYMENT_STATEMENT_STATUS_IMPORTED: &str = "IMPORTED";
pub const PAYMENT_STATEMENT_STATUS_RECONCILING: &str = "RECONCILING";
pub const PAYMENT_STATEMENT_STATUS_RECONCILED: &str = "RECONCILED";
pub const PAYMENT_STATEMENT_STATUS_FAILED: &str = "FAILED";

pub const RECONCILE_JOB_TYPE_PLATFORM: &str = "PLATFORM";
pub const RECONCILE_JOB_TYPE_PAYMENT: &str = "PAYMENT";
pub const RECONCILE_JOB_TYPE_SHIPPING: &str = "SHIPPING";

pub const RECONCILE_JOB_STATUS_RUNNING: &str = "RUNNING";
pub const RECONCILE_JOB_STATUS_DONE: &str = "DONE";
pub const RECONCILE_JOB_STATUS_FAILED: &str = "FAILED";

pub const RECONCILE_REF_TYPE_ORDER: &str = "ORDER";
pub const RECONCILE_REF_TYPE_SHIPMENT: &str = "SHIPMENT";

pub const RECONCILE_REASON_ORDER_MISSING: &str = "ORDER_NOT_FOUND";
pub const RECONCILE_REASON_AMOUNT_MISMATCH: &str = "AMOUNT_MISMATCH";
pub const RECONCILE_REASON_SHIPMENT_MISSING: &str = "SHIPMENT_NOT_FOUND";
pub const RECONCILE_REASON_SHIPPING_MISMATCH: &str = "FEE_MISMATCH";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformStatement {
    pub statement_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub shop_id: ShopId,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformStatementLine {
    pub tenant_id: TenantId,
    pub statement_id: uuid::Uuid,
    pub platform_order_id: String,
    pub gross_cent: i64,
    pub fee_cent: i64,
    pub refund_cent: i64,
    pub net_cent: i64,
    pub occurred_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PlatformStatementCreate {
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub shop_id: ShopId,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct PlatformStatementLineInput {
    pub platform_order_id: String,
    pub gross_cent: i64,
    pub fee_cent: i64,
    pub refund_cent: i64,
    pub net_cent: i64,
    pub occurred_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
}

#[derive(Debug, Clone)]
pub struct PlatformStatementImportResult {
    pub statement: PlatformStatement,
    pub inserted_lines: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStatement {
    pub statement_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub channel: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStatementLine {
    pub tenant_id: TenantId,
    pub statement_id: uuid::Uuid,
    pub platform_order_id: String,
    pub paid_cent: i64,
    pub fee_cent: i64,
    pub net_cent: i64,
    pub occurred_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PaymentStatementCreate {
    pub tenant_id: TenantId,
    pub channel: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct PaymentStatementLineInput {
    pub platform_order_id: String,
    pub paid_cent: i64,
    pub fee_cent: i64,
    pub net_cent: i64,
    pub occurred_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
}

#[derive(Debug, Clone)]
pub struct PaymentStatementImportResult {
    pub statement: PaymentStatement,
    pub inserted_lines: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReconcileJob {
    pub job_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub statement_id: Option<uuid::Uuid>,
    pub status: String,
    pub diff_count: i64,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PaymentReconcileJobCreate {
    pub tenant_id: TenantId,
    pub statement_id: Option<uuid::Uuid>,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PaymentReconcileJobUpdate {
    pub status: String,
    pub diff_count: i64,
    pub finished_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PaymentReconcileJobListFilters {
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReconcileJobListResult {
    pub items: Vec<PaymentReconcileJob>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReconcileItem {
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub platform_order_id: String,
    pub order_id: Option<OrderId>,
    pub expected_cent: i64,
    pub actual_cent: i64,
    pub diff_cent: i64,
    pub diff_reason: Option<String>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PaymentReconcileItemCreate {
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub platform_order_id: String,
    pub order_id: Option<OrderId>,
    pub expected_cent: i64,
    pub actual_cent: i64,
    pub diff_cent: i64,
    pub diff_reason: Option<String>,
    pub raw_json: Value,
}

#[derive(Debug, Clone)]
pub struct PaymentReconcileItemListFilters {
    pub job_id: uuid::Uuid,
    pub diff_only: bool,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentReconcileItemListResult {
    pub items: Vec<PaymentReconcileItem>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteoffRecord {
    pub writeoff_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub ref_type: String,
    pub ref_id: String,
    pub amount_cent: i64,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WriteoffRecordListFilters {
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct WriteoffRecordListResult {
    pub items: Vec<WriteoffRecord>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingStatementLine {
    pub line_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub tracking_no: String,
    pub fee_cent: i64,
    pub weight: Option<f64>,
    pub occurred_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ShippingStatementLineInput {
    pub tracking_no: String,
    pub fee_cent: i64,
    pub weight: Option<f64>,
    pub occurred_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileJob {
    pub job_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub job_type: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ReconcileJobCreate {
    pub tenant_id: TenantId,
    pub job_type: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ReconcileJobUpdate {
    pub status: String,
    pub finished_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReconcileJobListFilters {
    pub job_type: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileJobListResult {
    pub items: Vec<ReconcileJob>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileJobItem {
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub ref_type: String,
    pub ref_id: String,
    pub expected_cent: i64,
    pub actual_cent: i64,
    pub diff_cent: i64,
    pub diff_reason: Option<String>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ReconcileJobItemCreate {
    pub tenant_id: TenantId,
    pub job_id: uuid::Uuid,
    pub ref_type: String,
    pub ref_id: String,
    pub expected_cent: i64,
    pub actual_cent: i64,
    pub diff_cent: i64,
    pub diff_reason: Option<String>,
    pub raw_json: Value,
}

#[derive(Debug, Clone)]
pub struct ReconcileJobItemListFilters {
    pub job_id: uuid::Uuid,
    pub ref_type: Option<String>,
    pub diff_only: bool,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileJobItemListResult {
    pub items: Vec<ReconcileJobItem>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub invoice_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub invoice_title: String,
    pub tax_no: Option<String>,
    pub status: String,
    pub issued_at: Option<DateTime<Utc>>,
    pub result_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InvoiceCreate {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub invoice_title: String,
    pub tax_no: Option<String>,
    pub status: String,
    pub result_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct InvoiceListFilters {
    pub status: Option<String>,
    pub order_id: Option<OrderId>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceListResult {
    pub items: Vec<Invoice>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitSnapshot {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub revenue_cent: i64,
    pub cost_cent: i64,
    pub platform_fee_cent: i64,
    pub shipping_fee_cent: i64,
    pub gross_profit_cent: i64,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProfitSnapshotUpsert {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub revenue_cent: i64,
    pub cost_cent: i64,
    pub platform_fee_cent: i64,
    pub shipping_fee_cent: i64,
    pub gross_profit_cent: i64,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProfitSnapshotListFilters {
    pub shop_id: Option<ShopId>,
    pub sku_id: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitSnapshotSummary {
    pub order_id: OrderId,
    pub shop_id: ShopId,
    pub platform_order_id: String,
    pub order_created_at: DateTime<Utc>,
    pub revenue_cent: i64,
    pub cost_cent: i64,
    pub platform_fee_cent: i64,
    pub shipping_fee_cent: i64,
    pub gross_profit_cent: i64,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitSnapshotListResult {
    pub items: Vec<ProfitSnapshotSummary>,
    pub total: i64,
}

pub struct PlatformStatementsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PlatformStatementsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_statement_with_lines(
        &self,
        input: PlatformStatementCreate,
        lines: &[PlatformStatementLineInput],
    ) -> anyhow::Result<PlatformStatementImportResult> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let row: PlatformStatementRow = sqlx::query_as(
            r#"
            insert into platform_statements(
                tenant_id, platform, shop_id, period_start, period_end, status
            ) values ($1,$2,$3,$4,$5,$6)
            on conflict (tenant_id, platform, shop_id, period_start, period_end)
            do update set updated_at = now()
            returning statement_id, tenant_id, platform, shop_id, period_start, period_end,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.platform.to_string())
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(&input.status)
        .fetch_one(&mut *tx)
        .await?;

        let mut inserted_lines = 0i64;
        if !lines.is_empty() {
            let mut qb = QueryBuilder::<Postgres>::new(
                r#"
                insert into platform_statement_lines(
                    tenant_id, statement_id, platform_order_id,
                    gross_cent, fee_cent, refund_cent, net_cent, occurred_at, raw_json
                )
                "#,
            );
            qb.push_values(lines, |mut b, line| {
                b.push_bind(uuid::Uuid::from(input.tenant_id))
                    .push_bind(row.statement_id)
                    .push_bind(&line.platform_order_id)
                    .push_bind(line.gross_cent)
                    .push_bind(line.fee_cent)
                    .push_bind(line.refund_cent)
                    .push_bind(line.net_cent)
                    .push_bind(line.occurred_at)
                    .push_bind(&line.raw_json);
            });
            qb.push(
                r#"
                on conflict (tenant_id, statement_id, platform_order_id)
                do update set
                    gross_cent = excluded.gross_cent,
                    fee_cent = excluded.fee_cent,
                    refund_cent = excluded.refund_cent,
                    net_cent = excluded.net_cent,
                    occurred_at = excluded.occurred_at,
                    raw_json = excluded.raw_json
                "#,
            );
            let result = qb.build().execute(&mut *tx).await?;
            inserted_lines = result.rows_affected() as i64;
        }

        tx.commit().await?;
        Ok(PlatformStatementImportResult {
            statement: row.into_statement()?,
            inserted_lines,
        })
    }

    pub async fn update_statement_status(
        &self,
        tenant_id: TenantId,
        statement_id: uuid::Uuid,
        status: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update platform_statements
            set status = $3, updated_at = now()
            where tenant_id = $1 and statement_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(statement_id)
        .bind(status)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_statement(
        &self,
        tenant_id: TenantId,
        statement_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PlatformStatement>> {
        let row: Option<PlatformStatementRow> = sqlx::query_as(
            r#"
            select statement_id, tenant_id, platform, shop_id, period_start, period_end,
                   status, created_at, updated_at
            from platform_statements
            where tenant_id = $1 and statement_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(statement_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(match row {
            Some(row) => Some(row.into_statement()?),
            None => None,
        })
    }

    pub async fn list_statement_lines(
        &self,
        tenant_id: TenantId,
        statement_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<PlatformStatementLine>> {
        let rows: Vec<PlatformStatementLineRow> = sqlx::query_as(
            r#"
            select tenant_id, statement_id, platform_order_id, gross_cent, fee_cent,
                   refund_cent, net_cent, occurred_at, raw_json, created_at
            from platform_statement_lines
            where tenant_id = $1 and statement_id = $2
            order by platform_order_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(statement_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(PlatformStatementLineRow::into_line)
            .collect())
    }
}

pub struct PaymentStatementsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PaymentStatementsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_statement_with_lines(
        &self,
        input: PaymentStatementCreate,
        lines: &[PaymentStatementLineInput],
    ) -> anyhow::Result<PaymentStatementImportResult> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let row: PaymentStatementRow = sqlx::query_as(
            r#"
            insert into payment_statements(
                tenant_id, channel, period_start, period_end, status
            ) values ($1,$2,$3,$4,$5)
            on conflict (tenant_id, channel, period_start, period_end)
            do update set updated_at = now()
            returning statement_id, tenant_id, channel, period_start, period_end,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.channel)
        .bind(input.period_start)
        .bind(input.period_end)
        .bind(&input.status)
        .fetch_one(&mut *tx)
        .await?;

        let mut inserted_lines = 0i64;
        if !lines.is_empty() {
            let mut qb = QueryBuilder::<Postgres>::new(
                r#"
                insert into payment_statement_lines(
                    tenant_id, statement_id, platform_order_id,
                    paid_cent, fee_cent, net_cent, occurred_at, raw_json
                )
                "#,
            );
            qb.push_values(lines, |mut b, line| {
                b.push_bind(uuid::Uuid::from(input.tenant_id))
                    .push_bind(row.statement_id)
                    .push_bind(&line.platform_order_id)
                    .push_bind(line.paid_cent)
                    .push_bind(line.fee_cent)
                    .push_bind(line.net_cent)
                    .push_bind(line.occurred_at)
                    .push_bind(&line.raw_json);
            });
            qb.push(
                r#"
                on conflict (tenant_id, statement_id, platform_order_id)
                do update set
                    paid_cent = excluded.paid_cent,
                    fee_cent = excluded.fee_cent,
                    net_cent = excluded.net_cent,
                    occurred_at = excluded.occurred_at,
                    raw_json = excluded.raw_json
                "#,
            );
            let result = qb.build().execute(&mut *tx).await?;
            inserted_lines = result.rows_affected() as i64;
        }

        tx.commit().await?;
        Ok(PaymentStatementImportResult {
            statement: row.into_statement(),
            inserted_lines,
        })
    }

    pub async fn update_statement_status(
        &self,
        tenant_id: TenantId,
        statement_id: uuid::Uuid,
        status: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update payment_statements
            set status = $3, updated_at = now()
            where tenant_id = $1 and statement_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(statement_id)
        .bind(status)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_statement(
        &self,
        tenant_id: TenantId,
        statement_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PaymentStatement>> {
        let row: Option<PaymentStatementRow> = sqlx::query_as(
            r#"
            select statement_id, tenant_id, channel, period_start, period_end,
                   status, created_at, updated_at
            from payment_statements
            where tenant_id = $1 and statement_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(statement_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(PaymentStatementRow::into_statement))
    }

    pub async fn list_statement_lines(
        &self,
        tenant_id: TenantId,
        statement_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<PaymentStatementLine>> {
        let rows: Vec<PaymentStatementLineRow> = sqlx::query_as(
            r#"
            select tenant_id, statement_id, platform_order_id, paid_cent, fee_cent,
                   net_cent, occurred_at, raw_json, created_at
            from payment_statement_lines
            where tenant_id = $1 and statement_id = $2
            order by platform_order_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(statement_id)
        .fetch_all(self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(PaymentStatementLineRow::into_line)
            .collect())
    }
}

pub struct ShippingStatementsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ShippingStatementsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_lines(
        &self,
        tenant_id: TenantId,
        lines: &[ShippingStatementLineInput],
    ) -> anyhow::Result<i64> {
        if lines.is_empty() {
            return Ok(0);
        }
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into shipping_statement_lines(
                tenant_id, tracking_no, fee_cent, weight, occurred_at, raw_json
            )
            "#,
        );
        qb.push_values(lines, |mut b, line| {
            b.push_bind(uuid::Uuid::from(tenant_id))
                .push_bind(&line.tracking_no)
                .push_bind(line.fee_cent)
                .push_bind(line.weight)
                .push_bind(line.occurred_at)
                .push_bind(&line.raw_json);
        });
        let result = qb.build().execute(self.pool).await?;
        Ok(result.rows_affected() as i64)
    }
}

pub struct ReconcileJobsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ReconcileJobsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_job(&self, input: ReconcileJobCreate) -> anyhow::Result<ReconcileJob> {
        let row: ReconcileJobRow = sqlx::query_as(
            r#"
            insert into reconcile_jobs(
                tenant_id, job_type, status, started_at
            ) values ($1,$2,$3,$4)
            returning job_id, tenant_id, job_type, status, started_at, finished_at,
                      last_error, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.job_type)
        .bind(&input.status)
        .bind(input.started_at)
        .fetch_one(self.pool)
        .await?;
        Ok(row.into_job())
    }

    pub async fn update_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        update: ReconcileJobUpdate,
    ) -> anyhow::Result<Option<ReconcileJob>> {
        let row: Option<ReconcileJobRow> = sqlx::query_as(
            r#"
            update reconcile_jobs
            set status = $3,
                finished_at = $4,
                last_error = $5
            where tenant_id = $1 and job_id = $2
            returning job_id, tenant_id, job_type, status, started_at, finished_at,
                      last_error, created_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .bind(&update.status)
        .bind(update.finished_at)
        .bind(&update.last_error)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(ReconcileJobRow::into_job))
    }

    pub async fn insert_job_items(&self, items: &[ReconcileJobItemCreate]) -> anyhow::Result<i64> {
        if items.is_empty() {
            return Ok(0);
        }
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into reconcile_job_items(
                tenant_id, job_id, ref_type, ref_id, expected_cent, actual_cent,
                diff_cent, diff_reason, raw_json
            )
            "#,
        );
        qb.push_values(items, |mut b, item| {
            b.push_bind(uuid::Uuid::from(item.tenant_id))
                .push_bind(item.job_id)
                .push_bind(&item.ref_type)
                .push_bind(&item.ref_id)
                .push_bind(item.expected_cent)
                .push_bind(item.actual_cent)
                .push_bind(item.diff_cent)
                .push_bind(&item.diff_reason)
                .push_bind(&item.raw_json);
        });
        let result = qb.build().execute(self.pool).await?;
        let diff_count = result.rows_affected() as i64;

        let mut diff_qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into reconcile_diffs(
                tenant_id, job_id, ref_type, ref_id, expected_cent, actual_cent,
                diff_cent, diff_reason, status, raw_json
            )
            "#,
        );
        diff_qb.push_values(items, |mut b, item| {
            b.push_bind(uuid::Uuid::from(item.tenant_id))
                .push_bind(item.job_id)
                .push_bind(&item.ref_type)
                .push_bind(&item.ref_id)
                .push_bind(item.expected_cent)
                .push_bind(item.actual_cent)
                .push_bind(item.diff_cent)
                .push_bind(&item.diff_reason)
                .push_bind("PENDING")
                .push_bind(&item.raw_json);
        });
        diff_qb.push(
            r#"
            on conflict (tenant_id, job_id, ref_type, ref_id)
            do update set
                expected_cent = excluded.expected_cent,
                actual_cent = excluded.actual_cent,
                diff_cent = excluded.diff_cent,
                diff_reason = excluded.diff_reason,
                raw_json = excluded.raw_json,
                updated_at = now()
            "#,
        );
        diff_qb.build().execute(self.pool).await?;
        Ok(diff_count)
    }

    pub async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: ReconcileJobListFilters,
    ) -> anyhow::Result<ReconcileJobListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select job_id, tenant_id, job_type, status, started_at, finished_at,
                   last_error, created_at
            from reconcile_jobs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(job_type) = filters.job_type.clone() {
            qb.push(" and job_type = ");
            qb.push_bind(job_type);
        }
        if let Some(status) = filters.status.clone() {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<ReconcileJobRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows.into_iter().map(ReconcileJobRow::into_job).collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)::bigint
            from reconcile_jobs
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(job_type) = filters.job_type {
            count_qb.push(" and job_type = ");
            count_qb.push_bind(job_type);
        }
        if let Some(status) = filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(ReconcileJobListResult { items, total })
    }

    pub async fn list_job_items(
        &self,
        tenant_id: TenantId,
        filters: ReconcileJobItemListFilters,
    ) -> anyhow::Result<ReconcileJobItemListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select tenant_id, job_id, ref_type, ref_id, expected_cent, actual_cent,
                   diff_cent, diff_reason, raw_json, created_at
            from reconcile_job_items
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and job_id = ");
        qb.push_bind(filters.job_id);
        if let Some(ref_type) = filters.ref_type.clone() {
            qb.push(" and ref_type = ");
            qb.push_bind(ref_type);
        }
        if filters.diff_only {
            qb.push(" and diff_cent <> 0");
        }
        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<ReconcileJobItemRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(ReconcileJobItemRow::into_item)
            .collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)::bigint
            from reconcile_job_items
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        count_qb.push(" and job_id = ");
        count_qb.push_bind(filters.job_id);
        if let Some(ref_type) = filters.ref_type {
            count_qb.push(" and ref_type = ");
            count_qb.push_bind(ref_type);
        }
        if filters.diff_only {
            count_qb.push(" and diff_cent <> 0");
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(ReconcileJobItemListResult { items, total })
    }
}

pub struct PaymentReconcileJobsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PaymentReconcileJobsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_job(
        &self,
        input: PaymentReconcileJobCreate,
    ) -> anyhow::Result<PaymentReconcileJob> {
        let row: PaymentReconcileJobRow = sqlx::query_as(
            r#"
            insert into payment_reconcile_jobs(
                tenant_id, statement_id, status, diff_count, started_at
            ) values ($1,$2,$3,$4,$5)
            returning job_id, tenant_id, statement_id, status, diff_count, started_at, finished_at,
                      last_error, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.statement_id)
        .bind(&input.status)
        .bind(0i64)
        .bind(input.started_at)
        .fetch_one(self.pool)
        .await?;
        Ok(row.into_job())
    }

    pub async fn update_job(
        &self,
        tenant_id: TenantId,
        job_id: uuid::Uuid,
        update: PaymentReconcileJobUpdate,
    ) -> anyhow::Result<Option<PaymentReconcileJob>> {
        let row: Option<PaymentReconcileJobRow> = sqlx::query_as(
            r#"
            update payment_reconcile_jobs
            set status = $3,
                diff_count = $4,
                finished_at = $5,
                last_error = $6
            where tenant_id = $1 and job_id = $2
            returning job_id, tenant_id, statement_id, status, diff_count, started_at, finished_at,
                      last_error, created_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(job_id)
        .bind(&update.status)
        .bind(update.diff_count)
        .bind(update.finished_at)
        .bind(&update.last_error)
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(PaymentReconcileJobRow::into_job))
    }

    pub async fn insert_items(&self, items: &[PaymentReconcileItemCreate]) -> anyhow::Result<i64> {
        if items.is_empty() {
            return Ok(0);
        }
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into payment_reconcile_items(
                tenant_id, job_id, platform_order_id, order_id, expected_cent, actual_cent,
                diff_cent, diff_reason, raw_json
            )
            "#,
        );
        qb.push_values(items, |mut b, item| {
            b.push_bind(uuid::Uuid::from(item.tenant_id))
                .push_bind(item.job_id)
                .push_bind(&item.platform_order_id)
                .push_bind(item.order_id.map(uuid::Uuid::from))
                .push_bind(item.expected_cent)
                .push_bind(item.actual_cent)
                .push_bind(item.diff_cent)
                .push_bind(&item.diff_reason)
                .push_bind(&item.raw_json);
        });
        qb.push(
            r#"
            on conflict (tenant_id, job_id, platform_order_id)
            do update set
                order_id = excluded.order_id,
                expected_cent = excluded.expected_cent,
                actual_cent = excluded.actual_cent,
                diff_cent = excluded.diff_cent,
                diff_reason = excluded.diff_reason,
                raw_json = excluded.raw_json
            "#,
        );
        let result = qb.build().execute(self.pool).await?;
        Ok(result.rows_affected() as i64)
    }

    pub async fn list_jobs(
        &self,
        tenant_id: TenantId,
        filters: PaymentReconcileJobListFilters,
    ) -> anyhow::Result<PaymentReconcileJobListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select job_id, tenant_id, statement_id, status, diff_count, started_at, finished_at,
                   last_error, created_at
            from payment_reconcile_jobs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status.clone() {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<PaymentReconcileJobRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(PaymentReconcileJobRow::into_job)
            .collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)::bigint
            from payment_reconcile_jobs
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;
        Ok(PaymentReconcileJobListResult { items, total })
    }

    pub async fn list_items(
        &self,
        tenant_id: TenantId,
        filters: PaymentReconcileItemListFilters,
    ) -> anyhow::Result<PaymentReconcileItemListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select tenant_id, job_id, platform_order_id, order_id, expected_cent, actual_cent,
                   diff_cent, diff_reason, raw_json, created_at
            from payment_reconcile_items
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and job_id = ");
        qb.push_bind(filters.job_id);
        if filters.diff_only {
            qb.push(" and diff_cent <> 0");
        }
        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<PaymentReconcileItemRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(PaymentReconcileItemRow::into_item)
            .collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)::bigint
            from payment_reconcile_items
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        count_qb.push(" and job_id = ");
        count_qb.push_bind(filters.job_id);
        if filters.diff_only {
            count_qb.push(" and diff_cent <> 0");
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;
        Ok(PaymentReconcileItemListResult { items, total })
    }
}

pub struct WriteoffRecordsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WriteoffRecordsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_records(
        &self,
        tenant_id: TenantId,
        filters: WriteoffRecordListFilters,
    ) -> anyhow::Result<WriteoffRecordListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select writeoff_id, tenant_id, ref_type, ref_id, amount_cent, reason, created_by,
                   created_at
            from writeoff_records
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(ref_type) = filters.ref_type.clone() {
            qb.push(" and ref_type = ");
            qb.push_bind(ref_type);
        }
        if let Some(ref_id) = filters.ref_id.clone() {
            qb.push(" and ref_id = ");
            qb.push_bind(ref_id);
        }
        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<WriteoffRecordRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(WriteoffRecordRow::into_record)
            .collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)::bigint
            from writeoff_records
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(ref_type) = filters.ref_type {
            count_qb.push(" and ref_type = ");
            count_qb.push_bind(ref_type);
        }
        if let Some(ref_id) = filters.ref_id {
            count_qb.push(" and ref_id = ");
            count_qb.push_bind(ref_id);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;
        Ok(WriteoffRecordListResult { items, total })
    }
}

pub struct InvoicesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InvoicesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_invoice(&self, input: InvoiceCreate) -> anyhow::Result<Invoice> {
        let result_json = input.result_json.unwrap_or_else(|| serde_json::json!({}));
        let row: InvoiceRow = sqlx::query_as(
            r#"
            insert into invoices(
                tenant_id, order_id, invoice_title, tax_no, status, result_json
            ) values ($1,$2,$3,$4,$5,$6)
            on conflict (tenant_id, order_id)
            do update set
                invoice_title = excluded.invoice_title,
                tax_no = excluded.tax_no,
                status = excluded.status,
                result_json = excluded.result_json,
                updated_at = now()
            returning invoice_id, tenant_id, order_id, invoice_title, tax_no, status,
                      issued_at, result_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.order_id))
        .bind(&input.invoice_title)
        .bind(&input.tax_no)
        .bind(&input.status)
        .bind(result_json)
        .fetch_one(self.pool)
        .await?;
        Ok(row.into_invoice())
    }

    pub async fn list_invoices(
        &self,
        tenant_id: TenantId,
        filters: InvoiceListFilters,
    ) -> anyhow::Result<InvoiceListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select invoice_id, tenant_id, order_id, invoice_title, tax_no, status, issued_at,
                   result_json, created_at, updated_at
            from invoices
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status.clone() {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(order_id) = filters.order_id {
            qb.push(" and order_id = ");
            qb.push_bind(uuid::Uuid::from(order_id));
        }
        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<InvoiceRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows.into_iter().map(InvoiceRow::into_invoice).collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)::bigint
            from invoices
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        if let Some(order_id) = filters.order_id {
            count_qb.push(" and order_id = ");
            count_qb.push_bind(uuid::Uuid::from(order_id));
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(InvoiceListResult { items, total })
    }
}

pub struct ProfitSnapshotsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProfitSnapshotsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_snapshot(
        &self,
        input: ProfitSnapshotUpsert,
    ) -> anyhow::Result<ProfitSnapshot> {
        let row: ProfitSnapshotRow = sqlx::query_as(
            r#"
            insert into profit_snapshots(
                tenant_id, order_id, revenue_cent, cost_cent, platform_fee_cent,
                shipping_fee_cent, gross_profit_cent, computed_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            on conflict (tenant_id, order_id)
            do update set
                revenue_cent = excluded.revenue_cent,
                cost_cent = excluded.cost_cent,
                platform_fee_cent = excluded.platform_fee_cent,
                shipping_fee_cent = excluded.shipping_fee_cent,
                gross_profit_cent = excluded.gross_profit_cent,
                computed_at = excluded.computed_at
            returning tenant_id, order_id, revenue_cent, cost_cent, platform_fee_cent,
                      shipping_fee_cent, gross_profit_cent, computed_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.order_id))
        .bind(input.revenue_cent)
        .bind(input.cost_cent)
        .bind(input.platform_fee_cent)
        .bind(input.shipping_fee_cent)
        .bind(input.gross_profit_cent)
        .bind(input.computed_at)
        .fetch_one(self.pool)
        .await?;
        Ok(row.into_snapshot())
    }

    pub async fn list_snapshots(
        &self,
        tenant_id: TenantId,
        filters: ProfitSnapshotListFilters,
    ) -> anyhow::Result<ProfitSnapshotListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                p.order_id,
                o.shop_id,
                o.platform_order_id,
                o.order_created_at,
                p.revenue_cent,
                p.cost_cent,
                p.platform_fee_cent,
                p.shipping_fee_cent,
                p.gross_profit_cent,
                p.computed_at
            from profit_snapshots p
            join order_index idx
              on idx.tenant_id = p.tenant_id and idx.order_id = p.order_id
            join orders o
              on o.tenant_id = p.tenant_id and o.order_id = idx.order_id and o.order_month = idx.order_month
            where p.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(shop_id) = filters.shop_id {
            qb.push(" and o.shop_id = ");
            qb.push_bind(uuid::Uuid::from(shop_id));
        }
        if let Some(created_from) = filters.created_from {
            qb.push(" and o.order_created_at >= ");
            qb.push_bind(created_from);
        }
        if let Some(created_to) = filters.created_to {
            qb.push(" and o.order_created_at <= ");
            qb.push_bind(created_to);
        }
        if let Some(sku_id) = filters.sku_id.clone() {
            qb.push(
                r#"
                and exists (
                    select 1 from order_items oi
                    where oi.tenant_id = p.tenant_id
                      and oi.order_id = p.order_id
                      and oi.order_month = idx.order_month
                      and oi.sku_id = 
                "#,
            );
            qb.push_bind(sku_id);
            qb.push(")");
        }

        qb.push(" order by p.computed_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<ProfitSnapshotSummaryRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(ProfitSnapshotSummaryRow::into_summary)
            .collect();

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)::bigint
            from profit_snapshots p
            join order_index idx
              on idx.tenant_id = p.tenant_id and idx.order_id = p.order_id
            join orders o
              on o.tenant_id = p.tenant_id and o.order_id = idx.order_id and o.order_month = idx.order_month
            where p.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(shop_id) = filters.shop_id {
            count_qb.push(" and o.shop_id = ");
            count_qb.push_bind(uuid::Uuid::from(shop_id));
        }
        if let Some(created_from) = filters.created_from {
            count_qb.push(" and o.order_created_at >= ");
            count_qb.push_bind(created_from);
        }
        if let Some(created_to) = filters.created_to {
            count_qb.push(" and o.order_created_at <= ");
            count_qb.push_bind(created_to);
        }
        if let Some(sku_id) = filters.sku_id {
            count_qb.push(
                r#"
                and exists (
                    select 1 from order_items oi
                    where oi.tenant_id = p.tenant_id
                      and oi.order_id = p.order_id
                      and oi.order_month = idx.order_month
                      and oi.sku_id = 
                "#,
            );
            count_qb.push_bind(sku_id);
            count_qb.push(")");
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(ProfitSnapshotListResult { items, total })
    }

    pub async fn get_snapshot_summary(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Option<ProfitSnapshotSummary>> {
        let row: Option<ProfitSnapshotSummaryRow> = sqlx::query_as(
            r#"
            select
                p.order_id,
                o.shop_id,
                o.platform_order_id,
                o.order_created_at,
                p.revenue_cent,
                p.cost_cent,
                p.platform_fee_cent,
                p.shipping_fee_cent,
                p.gross_profit_cent,
                p.computed_at
            from profit_snapshots p
            join order_index idx
              on idx.tenant_id = p.tenant_id and idx.order_id = p.order_id
            join orders o
              on o.tenant_id = p.tenant_id and o.order_id = idx.order_id and o.order_month = idx.order_month
            where p.tenant_id = $1 and p.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;
        Ok(row.map(ProfitSnapshotSummaryRow::into_summary))
    }

    pub async fn compute_profit_inputs(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<(i64, i64, i64, i64)> {
        let order_month: Option<NaiveDate> = sqlx::query_scalar(
            r#"
            select order_month
            from order_index
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;
        let Some(order_month) = order_month else {
            return Err(anyhow!("order not found"));
        };

        #[derive(sqlx::FromRow)]
        struct OrderAmountRow {
            total_amount_cent: i64,
            discount_amount_cent: Option<i64>,
        }

        let order_row: OrderAmountRow = sqlx::query_as(
            r#"
            select total_amount_cent, discount_amount_cent
            from orders
            where tenant_id = $1 and order_id = $2 and order_month = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(order_month)
        .fetch_one(self.pool)
        .await?;

        let order_revenue_cent =
            order_row.total_amount_cent - order_row.discount_amount_cent.unwrap_or(0);

        let item_revenue_cent: i64 = sqlx::query_scalar(
            r#"
            select coalesce(sum(coalesce(pay_amount_cent, amount_cent, 0)), 0)
            from order_items
            where tenant_id = $1 and order_id = $2 and order_month = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(order_month)
        .fetch_one(self.pool)
        .await?;

        let revenue_cent = if item_revenue_cent > 0 {
            item_revenue_cent
        } else {
            order_revenue_cent
        };

        let cost_cent: i64 = sqlx::query_scalar(
            r#"
            select coalesce(sum(total_cost_cent), 0)
            from order_cogs
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_one(self.pool)
        .await?;

        let platform_fee_cent: i64 = sqlx::query_scalar(
            r#"
            select coalesce(sum(fee_cent), 0)
            from platform_statement_lines
            where tenant_id = $1 and platform_order_id in (
                select platform_order_id
                from order_key_index
                where tenant_id = $1 and order_id = $2
            )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_one(self.pool)
        .await?;

        let shipping_fee_cent: i64 = sqlx::query_scalar(
            r#"
            select coalesce(sum(l.fee_cent), 0)
            from shipping_statement_lines l
            join shipments s
              on s.tenant_id = l.tenant_id and s.tracking_no = l.tracking_no
            where s.tenant_id = $1 and s.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_one(self.pool)
        .await?;

        Ok((
            revenue_cent,
            cost_cent,
            platform_fee_cent,
            shipping_fee_cent,
        ))
    }
}

#[derive(sqlx::FromRow)]
struct PlatformStatementRow {
    statement_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    platform: String,
    shop_id: uuid::Uuid,
    period_start: NaiveDate,
    period_end: NaiveDate,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PlatformStatementRow {
    fn into_statement(self) -> anyhow::Result<PlatformStatement> {
        Ok(PlatformStatement {
            statement_id: self.statement_id,
            tenant_id: TenantId(self.tenant_id),
            platform: Platform::from_str(&self.platform)?,
            shop_id: ShopId(self.shop_id),
            period_start: self.period_start,
            period_end: self.period_end,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct PlatformStatementLineRow {
    tenant_id: uuid::Uuid,
    statement_id: uuid::Uuid,
    platform_order_id: String,
    gross_cent: i64,
    fee_cent: i64,
    refund_cent: i64,
    net_cent: i64,
    occurred_at: Option<DateTime<Utc>>,
    raw_json: Value,
    created_at: DateTime<Utc>,
}

impl PlatformStatementLineRow {
    fn into_line(self) -> PlatformStatementLine {
        PlatformStatementLine {
            tenant_id: TenantId(self.tenant_id),
            statement_id: self.statement_id,
            platform_order_id: self.platform_order_id,
            gross_cent: self.gross_cent,
            fee_cent: self.fee_cent,
            refund_cent: self.refund_cent,
            net_cent: self.net_cent,
            occurred_at: self.occurred_at,
            raw_json: self.raw_json,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PaymentStatementRow {
    statement_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    channel: String,
    period_start: Option<NaiveDate>,
    period_end: Option<NaiveDate>,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PaymentStatementRow {
    fn into_statement(self) -> PaymentStatement {
        PaymentStatement {
            statement_id: self.statement_id,
            tenant_id: TenantId(self.tenant_id),
            channel: self.channel,
            period_start: self.period_start,
            period_end: self.period_end,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PaymentStatementLineRow {
    tenant_id: uuid::Uuid,
    statement_id: uuid::Uuid,
    platform_order_id: String,
    paid_cent: i64,
    fee_cent: i64,
    net_cent: i64,
    occurred_at: Option<DateTime<Utc>>,
    raw_json: Value,
    created_at: DateTime<Utc>,
}

impl PaymentStatementLineRow {
    fn into_line(self) -> PaymentStatementLine {
        PaymentStatementLine {
            tenant_id: TenantId(self.tenant_id),
            statement_id: self.statement_id,
            platform_order_id: self.platform_order_id,
            paid_cent: self.paid_cent,
            fee_cent: self.fee_cent,
            net_cent: self.net_cent,
            occurred_at: self.occurred_at,
            raw_json: self.raw_json,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ReconcileJobRow {
    job_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    job_type: String,
    status: String,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
}

impl ReconcileJobRow {
    fn into_job(self) -> ReconcileJob {
        ReconcileJob {
            job_id: self.job_id,
            tenant_id: TenantId(self.tenant_id),
            job_type: self.job_type,
            status: self.status,
            started_at: self.started_at,
            finished_at: self.finished_at,
            last_error: self.last_error,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ReconcileJobItemRow {
    tenant_id: uuid::Uuid,
    job_id: uuid::Uuid,
    ref_type: String,
    ref_id: String,
    expected_cent: i64,
    actual_cent: i64,
    diff_cent: i64,
    diff_reason: Option<String>,
    raw_json: Value,
    created_at: DateTime<Utc>,
}

impl ReconcileJobItemRow {
    fn into_item(self) -> ReconcileJobItem {
        ReconcileJobItem {
            tenant_id: TenantId(self.tenant_id),
            job_id: self.job_id,
            ref_type: self.ref_type,
            ref_id: self.ref_id,
            expected_cent: self.expected_cent,
            actual_cent: self.actual_cent,
            diff_cent: self.diff_cent,
            diff_reason: self.diff_reason,
            raw_json: self.raw_json,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PaymentReconcileJobRow {
    job_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    statement_id: Option<uuid::Uuid>,
    status: String,
    diff_count: i64,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
}

impl PaymentReconcileJobRow {
    fn into_job(self) -> PaymentReconcileJob {
        PaymentReconcileJob {
            job_id: self.job_id,
            tenant_id: TenantId(self.tenant_id),
            statement_id: self.statement_id,
            status: self.status,
            diff_count: self.diff_count,
            started_at: self.started_at,
            finished_at: self.finished_at,
            last_error: self.last_error,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PaymentReconcileItemRow {
    tenant_id: uuid::Uuid,
    job_id: uuid::Uuid,
    platform_order_id: String,
    order_id: Option<uuid::Uuid>,
    expected_cent: i64,
    actual_cent: i64,
    diff_cent: i64,
    diff_reason: Option<String>,
    raw_json: Value,
    created_at: DateTime<Utc>,
}

impl PaymentReconcileItemRow {
    fn into_item(self) -> PaymentReconcileItem {
        PaymentReconcileItem {
            tenant_id: TenantId(self.tenant_id),
            job_id: self.job_id,
            platform_order_id: self.platform_order_id,
            order_id: self.order_id.map(OrderId),
            expected_cent: self.expected_cent,
            actual_cent: self.actual_cent,
            diff_cent: self.diff_cent,
            diff_reason: self.diff_reason,
            raw_json: self.raw_json,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WriteoffRecordRow {
    writeoff_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    ref_type: String,
    ref_id: String,
    amount_cent: i64,
    reason: Option<String>,
    created_by: Option<String>,
    created_at: DateTime<Utc>,
}

impl WriteoffRecordRow {
    fn into_record(self) -> WriteoffRecord {
        WriteoffRecord {
            writeoff_id: self.writeoff_id,
            tenant_id: TenantId(self.tenant_id),
            ref_type: self.ref_type,
            ref_id: self.ref_id,
            amount_cent: self.amount_cent,
            reason: self.reason,
            created_by: self.created_by,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct InvoiceRow {
    invoice_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    order_id: uuid::Uuid,
    invoice_title: String,
    tax_no: Option<String>,
    status: String,
    issued_at: Option<DateTime<Utc>>,
    result_json: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl InvoiceRow {
    fn into_invoice(self) -> Invoice {
        Invoice {
            invoice_id: self.invoice_id,
            tenant_id: TenantId(self.tenant_id),
            order_id: OrderId(self.order_id),
            invoice_title: self.invoice_title,
            tax_no: self.tax_no,
            status: self.status,
            issued_at: self.issued_at,
            result_json: self.result_json,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ProfitSnapshotRow {
    tenant_id: uuid::Uuid,
    order_id: uuid::Uuid,
    revenue_cent: i64,
    cost_cent: i64,
    platform_fee_cent: i64,
    shipping_fee_cent: i64,
    gross_profit_cent: i64,
    computed_at: DateTime<Utc>,
}

impl ProfitSnapshotRow {
    fn into_snapshot(self) -> ProfitSnapshot {
        ProfitSnapshot {
            tenant_id: TenantId(self.tenant_id),
            order_id: OrderId(self.order_id),
            revenue_cent: self.revenue_cent,
            cost_cent: self.cost_cent,
            platform_fee_cent: self.platform_fee_cent,
            shipping_fee_cent: self.shipping_fee_cent,
            gross_profit_cent: self.gross_profit_cent,
            computed_at: self.computed_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ProfitSnapshotSummaryRow {
    order_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    platform_order_id: String,
    order_created_at: DateTime<Utc>,
    revenue_cent: i64,
    cost_cent: i64,
    platform_fee_cent: i64,
    shipping_fee_cent: i64,
    gross_profit_cent: i64,
    computed_at: DateTime<Utc>,
}

impl ProfitSnapshotSummaryRow {
    fn into_summary(self) -> ProfitSnapshotSummary {
        ProfitSnapshotSummary {
            order_id: OrderId(self.order_id),
            shop_id: ShopId(self.shop_id),
            platform_order_id: self.platform_order_id,
            order_created_at: self.order_created_at,
            revenue_cent: self.revenue_cent,
            cost_cent: self.cost_cent,
            platform_fee_cent: self.platform_fee_cent,
            shipping_fee_cent: self.shipping_fee_cent,
            gross_profit_cent: self.gross_profit_cent,
            computed_at: self.computed_at,
        }
    }
}
