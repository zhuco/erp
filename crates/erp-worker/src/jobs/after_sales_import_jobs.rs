use calamine::{open_workbook_auto, Data, Reader};
use csv::ReaderBuilder;
use erp_repo::{
    after_sales::{AfterSalesCreateInput, AfterSalesItemCreate, AfterSalesRepo},
    after_sales_import_jobs::{
        AfterSalesImportJob, AfterSalesImportJobUpdate, AfterSalesImportJobsRepo,
    },
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    db::Db,
};
use erp_types::{AfterSalesType, OrderId, TenantId};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

const DEFAULT_POLL_INTERVAL_MS: u64 = 2000;
const DEFAULT_BATCH_SIZE: i64 = 8;
const DEFAULT_MAX_ATTEMPTS: i32 = 3;

#[derive(Debug, Clone)]
pub struct AfterSalesImportJobConfig {
    pub poll_interval: Duration,
    pub batch_size: i64,
    pub max_attempts: i32,
}

impl AfterSalesImportJobConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            poll_interval: Duration::from_millis(parse_u64(
                "AFTER_SALES_IMPORT_JOB_POLL_INTERVAL_MS",
                DEFAULT_POLL_INTERVAL_MS,
            )?),
            batch_size: parse_env_i64("AFTER_SALES_IMPORT_JOB_BATCH_SIZE", DEFAULT_BATCH_SIZE)?,
            max_attempts: parse_env_i32(
                "AFTER_SALES_IMPORT_JOB_MAX_ATTEMPTS",
                DEFAULT_MAX_ATTEMPTS,
            )?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImportRow {
    after_sales_id: Option<String>,
    order_id: String,
    after_sales_type: String,
    reason_code: Option<String>,
    amount_cent: Option<i64>,
    sku_id: String,
    qty: i64,
    refund_cent: i64,
}

pub async fn run_loop(db: &Db, config: AfterSalesImportJobConfig) -> anyhow::Result<()> {
    info!(
        poll_interval_ms = config.poll_interval.as_millis(),
        batch_size = config.batch_size,
        max_attempts = config.max_attempts,
        "after-sales import job worker started"
    );

    loop {
        let repo = AfterSalesImportJobsRepo::new(db.pool());
        let jobs = repo
            .claim_jobs(config.batch_size, config.max_attempts)
            .await?;
        if jobs.is_empty() {
            sleep(config.poll_interval).await;
            continue;
        }

        for job in jobs {
            if let Err(err) = process_job(db.pool(), &repo, job).await {
                error!(error = %err, "after-sales import job failed");
            }
        }
    }
}

async fn process_job(
    pool: &sqlx::PgPool,
    repo: &AfterSalesImportJobsRepo<'_>,
    job: AfterSalesImportJob,
) -> anyhow::Result<()> {
    let rows = match load_rows(&job) {
        Ok(rows) => rows,
        Err(err) => {
            repo.update_job(
                job.tenant_id,
                job.job_id,
                AfterSalesImportJobUpdate {
                    status: Some("FAILED".to_string()),
                    progress: Some(0),
                    total_rows: Some(0),
                    success_rows: Some(0),
                    failed_rows: Some(0),
                    last_error: Some(err.to_string()),
                },
            )
            .await?;
            return Ok(());
        }
    };

    if rows.is_empty() {
        repo.update_job(
            job.tenant_id,
            job.job_id,
            AfterSalesImportJobUpdate {
                status: Some("DONE".to_string()),
                progress: Some(100),
                total_rows: Some(0),
                success_rows: Some(0),
                failed_rows: Some(0),
                last_error: Some(String::new()),
            },
        )
        .await?;
        return Ok(());
    }

    let mut grouped: HashMap<String, Vec<ImportRow>> = HashMap::new();
    for (index, row) in rows.into_iter().enumerate() {
        let key = row
            .after_sales_id
            .clone()
            .unwrap_or_else(|| format!("row-{index}"));
        grouped.entry(key).or_default().push(row);
    }

    let total_rows: i32 = grouped.values().map(|items| items.len() as i32).sum();
    let mut success_rows = 0_i32;
    let mut failed_rows = 0_i32;
    let mut last_error = String::new();

    let after_sales_repo = AfterSalesRepo::new(pool);
    let audit_repo = AuditLogsRepo::new(pool);

    for (group_key, items) in grouped.into_iter() {
        match create_after_sales_group(&after_sales_repo, job.tenant_id, &group_key, &items).await {
            Ok(detail) => {
                success_rows += items.len() as i32;
                let _ = audit_repo
                    .insert(AuditLogCreate {
                        tenant_id: job.tenant_id,
                        user_id: None,
                        action: "AFTERSALES.IMPORT".to_string(),
                        entity_type: "after_sales".to_string(),
                        entity_id: Some(detail.after_sales.id.to_string()),
                        before_json: None,
                        after_json: Some(serde_json::json!({
                            "order_id": detail.after_sales.order_id.to_string(),
                            "type": detail.after_sales.after_sales_type.to_string(),
                            "amount_cent": detail.after_sales.amount_cent,
                            "rows": items.len(),
                        })),
                        ip: None,
                        ua: None,
                    })
                    .await;
            }
            Err(err) => {
                failed_rows += items.len() as i32;
                last_error = err.to_string();
                warn!(
                    job_id = %job.job_id,
                    group = %group_key,
                    error = %last_error,
                    "after-sales import row failed"
                );
            }
        }

        let processed = success_rows + failed_rows;
        let progress = if total_rows > 0 {
            ((processed as f64 / total_rows as f64) * 100.0).round() as i32
        } else {
            100
        };
        repo.update_job(
            job.tenant_id,
            job.job_id,
            AfterSalesImportJobUpdate {
                status: Some("RUNNING".to_string()),
                progress: Some(progress.clamp(0, 100)),
                total_rows: Some(total_rows),
                success_rows: Some(success_rows),
                failed_rows: Some(failed_rows),
                last_error: Some(last_error.clone()),
            },
        )
        .await?;
    }

    let status = "DONE";
    repo.update_job(
        job.tenant_id,
        job.job_id,
        AfterSalesImportJobUpdate {
            status: Some(status.to_string()),
            progress: Some(100),
            total_rows: Some(total_rows),
            success_rows: Some(success_rows),
            failed_rows: Some(failed_rows),
            last_error: Some(last_error),
        },
    )
    .await?;

    Ok(())
}

async fn create_after_sales_group(
    repo: &AfterSalesRepo<'_>,
    tenant_id: TenantId,
    group_key: &str,
    rows: &[ImportRow],
) -> anyhow::Result<erp_repo::after_sales::AfterSalesDetail> {
    let first = rows.first().ok_or_else(|| anyhow::anyhow!("empty group"))?;

    let order_id = OrderId::from_str(&first.order_id)
        .map_err(|_| anyhow::anyhow!("invalid order_id: {}", first.order_id))?;
    let after_sales_type = AfterSalesType::from_str(&first.after_sales_type)
        .map_err(|_| anyhow::anyhow!("invalid after_sales_type: {}", first.after_sales_type))?;

    let after_sales_id = match first.after_sales_id.as_deref() {
        Some(raw) if !raw.trim().is_empty() => Some(
            uuid::Uuid::parse_str(raw)
                .map_err(|_| anyhow::anyhow!("invalid after_sales_id: {}", raw))?,
        ),
        _ => None,
    };

    let mut amount_cent = first.amount_cent.unwrap_or(0);
    let mut items = Vec::with_capacity(rows.len());
    let mut amount_sum = 0_i64;
    for row in rows {
        if row.qty <= 0 {
            return Err(anyhow::anyhow!("qty must be positive"));
        }
        let sku_id = uuid::Uuid::parse_str(&row.sku_id)
            .map_err(|_| anyhow::anyhow!("invalid sku_id: {}", row.sku_id))?;
        items.push(AfterSalesItemCreate {
            sku_id,
            qty: row.qty,
            refund_cent: row.refund_cent,
        });
        amount_sum = amount_sum.saturating_add(row.refund_cent);
        if amount_cent == 0 {
            amount_cent = row.amount_cent.unwrap_or(0);
        }
    }

    if amount_cent == 0 {
        amount_cent = amount_sum;
    }

    let raw_json = serde_json::json!({
        "group_key": group_key,
        "rows": rows,
    });

    repo.create_after_sales(AfterSalesCreateInput {
        after_sales_id,
        tenant_id,
        order_id,
        after_sales_type,
        reason_code: first.reason_code.clone(),
        amount_cent,
        raw_json: Some(raw_json),
        compensation_status: None,
        evidence_status: None,
        items,
    })
    .await
}

fn load_rows(job: &AfterSalesImportJob) -> anyhow::Result<Vec<ImportRow>> {
    let path = PathBuf::from(&job.file_path);
    if job.file_type.eq_ignore_ascii_case("CSV") {
        load_csv_rows(&path)
    } else {
        load_excel_rows(&path)
    }
}

fn load_csv_rows(path: &Path) -> anyhow::Result<Vec<ImportRow>> {
    let file = File::open(path)?;
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_reader(file);
    let headers = reader
        .headers()
        .map(|hdr| hdr.iter().map(normalize_header).collect::<Vec<_>>())?;
    let indexes = build_header_index(&headers)?;

    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result?;
        rows.push(parse_row(&record, &indexes)?);
    }
    Ok(rows)
}

fn load_excel_rows(path: &Path) -> anyhow::Result<Vec<ImportRow>> {
    let mut workbook = open_workbook_auto(path)?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing sheet"))?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|_| anyhow::anyhow!("invalid sheet"))?;

    let mut iter = range.rows();
    let header_row = iter.next().ok_or_else(|| anyhow::anyhow!("empty sheet"))?;
    let headers = header_row
        .iter()
        .map(|cell| normalize_header(&cell.to_string()))
        .collect::<Vec<_>>();
    let indexes = build_header_index(&headers)?;

    let mut rows = Vec::new();
    for row in iter {
        let values = row.iter().map(cell_string).collect::<Vec<_>>();
        rows.push(parse_row_values(&values, &indexes)?);
    }
    Ok(rows)
}

fn cell_string(cell: &Data) -> String {
    match cell {
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => {
            if value.fract() == 0.0 {
                format!("{:.0}", value)
            } else {
                value.to_string()
            }
        }
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        _ => cell.to_string(),
    }
}

fn parse_row(record: &csv::StringRecord, indexes: &HeaderIndex) -> anyhow::Result<ImportRow> {
    let get = |idx: Option<usize>| {
        idx.and_then(|i| record.get(i))
            .unwrap_or("")
            .trim()
            .to_string()
    };
    build_row(get, indexes)
}

fn parse_row_values(values: &[String], indexes: &HeaderIndex) -> anyhow::Result<ImportRow> {
    let get = |idx: Option<usize>| {
        idx.and_then(|i| values.get(i))
            .map(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    build_row(get, indexes)
}

fn build_row(
    get: impl Fn(Option<usize>) -> String,
    indexes: &HeaderIndex,
) -> anyhow::Result<ImportRow> {
    let order_id = get(Some(indexes.order_id));
    let after_sales_type = get(Some(indexes.after_sales_type));
    let sku_id = get(Some(indexes.sku_id));

    if order_id.is_empty() || after_sales_type.is_empty() || sku_id.is_empty() {
        return Err(anyhow::anyhow!("order_id/type/sku_id required"));
    }

    let after_sales_id = get(indexes.after_sales_id);
    let reason_code = get(indexes.reason_code);
    let amount_cent = get(indexes.amount_cent);
    let qty = get(indexes.qty);
    let refund_cent = get(indexes.refund_cent);

    Ok(ImportRow {
        after_sales_id: non_empty(&after_sales_id),
        order_id,
        after_sales_type,
        reason_code: non_empty(&reason_code),
        amount_cent: parse_i64_value(&amount_cent),
        sku_id,
        qty: parse_i64_value(&qty).unwrap_or(1),
        refund_cent: parse_i64_value(&refund_cent).unwrap_or(0),
    })
}

#[derive(Debug)]
struct HeaderIndex {
    after_sales_id: Option<usize>,
    order_id: usize,
    after_sales_type: usize,
    sku_id: usize,
    reason_code: Option<usize>,
    amount_cent: Option<usize>,
    qty: Option<usize>,
    refund_cent: Option<usize>,
}

fn build_header_index(headers: &[String]) -> anyhow::Result<HeaderIndex> {
    let idx = |keys: &[&str]| headers.iter().position(|h| keys.contains(&h.as_str()));
    let order_id = idx(&["order_id"]).ok_or_else(|| anyhow::anyhow!("missing order_id"))?;
    let after_sales_type =
        idx(&["type", "after_sales_type"]).ok_or_else(|| anyhow::anyhow!("missing type"))?;
    let sku_id = idx(&["sku_id", "sku"]).ok_or_else(|| anyhow::anyhow!("missing sku_id"))?;

    Ok(HeaderIndex {
        after_sales_id: idx(&["after_sales_id", "after_sales_no"]),
        order_id,
        after_sales_type,
        sku_id,
        reason_code: idx(&["reason_code"]),
        amount_cent: idx(&["amount_cent"]),
        qty: idx(&["qty", "quantity"]),
        refund_cent: idx(&["refund_cent"]),
    })
}

fn normalize_header(value: &str) -> String {
    value.trim().to_lowercase().replace(' ', "_")
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_i64_value(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<i64>().ok()
}

fn parse_u64(key: &str, default_value: u64) -> anyhow::Result<u64> {
    match std::env::var(key) {
        Ok(value) => Ok(value.parse()?),
        Err(_) => Ok(default_value),
    }
}

fn parse_env_i64(key: &str, default_value: i64) -> anyhow::Result<i64> {
    match std::env::var(key) {
        Ok(value) => Ok(value.parse()?),
        Err(_) => Ok(default_value),
    }
}

fn parse_env_i32(key: &str, default_value: i32) -> anyhow::Result<i32> {
    match std::env::var(key) {
        Ok(value) => Ok(value.parse()?),
        Err(_) => Ok(default_value),
    }
}
