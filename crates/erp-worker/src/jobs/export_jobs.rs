use calamine::{Data, Reader, open_workbook_auto};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use csv::{ReaderBuilder, WriterBuilder};
use erp_grid_core::paged_query::PageQuery;
use erp_repo::{
    db::Db,
    export_jobs::{ExportJob, ExportJobUpdate, ExportJobsRepo},
    feature_flags::{FeatureFlag, FeatureFlagsRepo},
    product_grid_export::ProductExportCursor,
    products::{ProductCreate, ProductsRepo},
    sku_suppliers::SkuSuppliersRepo,
    skus::{SkuCreate, SkuUpdate, SkusRepo},
};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

const DEFAULT_POLL_INTERVAL_MS: u64 = 2000;
const DEFAULT_BATCH_SIZE: i64 = 10;
const DEFAULT_PAGE_SIZE: i64 = 1000;
const DEFAULT_MAX_ATTEMPTS: i32 = 5;
const MAX_PRODUCT_EXPORT_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ExportJobConfig {
    pub poll_interval: Duration,
    pub batch_size: i64,
    pub page_size: i64,
    pub max_attempts: i32,
}

impl ExportJobConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            poll_interval: Duration::from_millis(parse_u64(
                "EXPORT_JOB_POLL_INTERVAL_MS",
                DEFAULT_POLL_INTERVAL_MS,
            )?),
            batch_size: parse_i64("EXPORT_JOB_BATCH_SIZE", DEFAULT_BATCH_SIZE)?,
            page_size: parse_i64("EXPORT_JOB_PAGE_SIZE", DEFAULT_PAGE_SIZE)?,
            max_attempts: parse_i32("EXPORT_JOB_MAX_ATTEMPTS", DEFAULT_MAX_ATTEMPTS)?,
        })
    }
}

pub async fn run_loop(db: &Db, config: ExportJobConfig) -> anyhow::Result<()> {
    info!(
        poll_interval_ms = config.poll_interval.as_millis(),
        batch_size = config.batch_size,
        page_size = config.page_size,
        max_attempts = config.max_attempts,
        "export job worker started"
    );

    loop {
        let repo = ExportJobsRepo::new(db.pool());
        let jobs = repo
            .claim_jobs(config.batch_size, config.max_attempts)
            .await?;
        if jobs.is_empty() {
            sleep(config.poll_interval).await;
            continue;
        }

        for job in jobs {
            if let Err(err) = process_job(db.pool(), &repo, &config, job).await {
                error!(error = %err, "export job failed");
            }
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct ExportParams {
    start_at: Option<String>,
    end_at: Option<String>,
    platform: Option<String>,
    shop_id: Option<String>,
    warehouse_id: Option<String>,
    job_id: Option<String>,
    diff_only: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct SkuImportParams {
    file_path: Option<String>,
    file_type: Option<String>,
    file_name: Option<String>,
    duplicate_strategy: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProductExportParams {
    query: Option<PageQuery>,
    selection: Option<serde_json::Value>,
    checked: Option<serde_json::Value>,
}

async fn process_job(
    pool: &PgPool,
    repo: &ExportJobsRepo<'_>,
    config: &ExportJobConfig,
    job: ExportJob,
) -> anyhow::Result<()> {
    let params: ExportParams = serde_json::from_value(job.params_json.clone()).unwrap_or_default();

    let result = match job.job_type.as_str() {
        "ORDER_EXPORT" => match parse_range(&params) {
            Ok((start_at, end_at)) => {
                export_orders(pool, repo, config, &job, &params, start_at, end_at).await
            }
            Err(err) => Err(err),
        },
        "RECONCILE_EXPORT" => match parse_range(&params) {
            Ok((start_at, end_at)) => {
                export_reconcile(pool, repo, config, &job, &params, start_at, end_at).await
            }
            Err(err) => Err(err),
        },
        "INVENTORY_EXPORT" => export_inventory(pool, repo, config, &job, &params).await,
        "PRODUCT_EXPORT" => export_products(pool, repo, &job).await,
        "SKU_IMPORT" => import_skus(pool, repo, &job).await,
        other => Err(anyhow::anyhow!("unsupported export type: {other}")),
    };

    if let Err(err) = result {
        warn!(job_id = %job.job_id, error = %err, "export job processing failed");
        repo.update_job(
            job.tenant_id,
            job.job_id,
            ExportJobUpdate {
                status: Some("FAILED".to_string()),
                output_url: Some(None),
                progress: Some(0),
                last_error: Some(err.to_string()),
                params_json: None,
            },
        )
        .await?;
    }

    Ok(())
}

async fn export_orders(
    pool: &PgPool,
    repo: &ExportJobsRepo<'_>,
    config: &ExportJobConfig,
    job: &ExportJob,
    params: &ExportParams,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let shop_id = parse_uuid(params.shop_id.as_deref())?;
    let platform = params
        .platform
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());

    let total = count_orders(pool, job, start_at, end_at, platform.as_deref(), shop_id).await?;

    let mut writer = build_writer(job)?;
    writer.write_record([
        "order_id",
        "platform",
        "shop_id",
        "platform_order_id",
        "status",
        "total_amount_cent",
        "currency",
        "order_created_at",
        "ship_at",
    ])?;

    let mut offset = 0_i64;
    let mut processed = 0_i64;

    loop {
        let rows = fetch_orders(
            pool,
            job,
            start_at,
            end_at,
            platform.as_deref(),
            shop_id,
            config.page_size,
            offset,
        )
        .await?;
        if rows.is_empty() {
            break;
        }
        for row in &rows {
            writer.write_record([
                row.order_id.to_string(),
                row.platform.clone(),
                row.shop_id.to_string(),
                row.platform_order_id.clone(),
                row.status.clone(),
                row.total_amount_cent.to_string(),
                row.currency.clone(),
                row.order_created_at.to_rfc3339(),
                row.ship_at.map(|v| v.to_rfc3339()).unwrap_or_default(),
            ])?;
        }
        processed += rows.len() as i64;
        offset += rows.len() as i64;
        update_progress(repo, job, processed, total).await?;
    }

    finalize_job(repo, job).await?;
    Ok(())
}

async fn export_reconcile(
    pool: &PgPool,
    repo: &ExportJobsRepo<'_>,
    config: &ExportJobConfig,
    job: &ExportJob,
    params: &ExportParams,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    let reconcile_job_id = parse_uuid(params.job_id.as_deref())?;
    if let Some(job_id) = reconcile_job_id {
        let exists: Option<i32> = sqlx::query_scalar(
            r#"
            select 1
            from reconcile_jobs
            where tenant_id = $1 and job_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(job.tenant_id))
        .bind(job_id)
        .fetch_optional(pool)
        .await?;
        if exists.is_none() {
            return Err(anyhow::anyhow!("reconcile job not found"));
        }

        let diff_only = params.diff_only.unwrap_or(true);
        let total = count_reconcile_items(pool, job, job_id, diff_only).await?;
        let mut writer = build_writer(job)?;
        writer.write_record([
            "job_id",
            "ref_type",
            "ref_id",
            "expected_cent",
            "actual_cent",
            "diff_cent",
            "diff_reason",
            "created_at",
        ])?;

        let mut offset = 0_i64;
        let mut processed = 0_i64;
        loop {
            let rows =
                fetch_reconcile_items(pool, job, job_id, diff_only, config.page_size, offset)
                    .await?;
            if rows.is_empty() {
                break;
            }
            for row in &rows {
                writer.write_record([
                    job_id.to_string(),
                    row.ref_type.clone(),
                    row.ref_id.clone(),
                    row.expected_cent.to_string(),
                    row.actual_cent.to_string(),
                    row.diff_cent.to_string(),
                    row.diff_reason.clone().unwrap_or_default(),
                    row.created_at.to_rfc3339(),
                ])?;
            }
            processed += rows.len() as i64;
            offset += rows.len() as i64;
            update_progress(repo, job, processed, total).await?;
        }

        finalize_job(repo, job).await?;
        return Ok(());
    }

    let shop_id = parse_uuid(params.shop_id.as_deref())?;
    let platform = params
        .platform
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());
    let total = count_orders(pool, job, start_at, end_at, platform.as_deref(), shop_id).await?;

    let mut writer = build_writer(job)?;
    writer.write_record([
        "order_id",
        "platform",
        "platform_order_id",
        "order_total_cent",
        "items_amount_cent",
        "refund_amount_cent",
        "diff_cent",
    ])?;

    let mut offset = 0_i64;
    let mut processed = 0_i64;
    loop {
        let rows = fetch_reconcile_rows(
            pool,
            job,
            start_at,
            end_at,
            platform.as_deref(),
            shop_id,
            config.page_size,
            offset,
        )
        .await?;
        if rows.is_empty() {
            break;
        }
        for row in &rows {
            let diff = row.order_total_cent - row.items_amount_cent;
            writer.write_record([
                row.order_id.to_string(),
                row.platform.clone(),
                row.platform_order_id.clone(),
                row.order_total_cent.to_string(),
                row.items_amount_cent.to_string(),
                row.refund_amount_cent.to_string(),
                diff.to_string(),
            ])?;
        }
        processed += rows.len() as i64;
        offset += rows.len() as i64;
        update_progress(repo, job, processed, total).await?;
    }

    finalize_job(repo, job).await?;
    Ok(())
}

async fn export_inventory(
    pool: &PgPool,
    repo: &ExportJobsRepo<'_>,
    config: &ExportJobConfig,
    job: &ExportJob,
    params: &ExportParams,
) -> anyhow::Result<()> {
    let warehouse_id = parse_uuid(params.warehouse_id.as_deref())?;
    let total = count_inventory(pool, job, warehouse_id).await?;

    let mut writer = build_writer(job)?;
    writer.write_record([
        "sku_id",
        "sku_code",
        "warehouse_id",
        "on_hand",
        "reserved",
        "available",
    ])?;

    let mut offset = 0_i64;
    let mut processed = 0_i64;
    loop {
        let rows = fetch_inventory_rows(pool, job, warehouse_id, config.page_size, offset).await?;
        if rows.is_empty() {
            break;
        }
        for row in &rows {
            let available = row.on_hand - row.reserved;
            writer.write_record([
                row.sku_id.to_string(),
                row.sku_code.clone().unwrap_or_default(),
                row.warehouse_id.to_string(),
                row.on_hand.to_string(),
                row.reserved.to_string(),
                available.to_string(),
            ])?;
        }
        processed += rows.len() as i64;
        offset += rows.len() as i64;
        update_progress(repo, job, processed, total).await?;
    }

    finalize_job(repo, job).await?;
    Ok(())
}

async fn export_products(
    pool: &PgPool,
    repo: &ExportJobsRepo<'_>,
    job: &ExportJob,
) -> anyhow::Result<()> {
    let params: ProductExportParams = serde_json::from_value(job.params_json.clone())
        .map_err(|err| anyhow::anyhow!("invalid PRODUCT_EXPORT params: {err}"))?;
    let final_path = export_job_file_path(job.tenant_id, job.job_id);
    let temporary_path = product_export_temporary_path(job);
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if final_path.try_exists()? {
        let metadata = fs::symlink_metadata(&final_path)?;
        if !metadata.file_type().is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() == 0
            || metadata.len() > MAX_PRODUCT_EXPORT_BYTES
        {
            anyhow::bail!("Existing product export artifact is not a valid regular CSV file");
        }
        finalize_job(repo, job).await?;
        return Ok(());
    }
    let result = tokio::time::timeout(Duration::from_secs(300), async {
        let mut cursor = match (&params.query, &params.selection, &params.checked) {
            (Some(query), None, None) => {
                ProductExportCursor::open(pool, job.tenant_id, query).await?
            }
            (None, Some(_), None) | (None, None, Some(_)) => {
                ProductExportCursor::open_selection(pool, job.tenant_id, job.job_id).await?
            }
            _ => anyhow::bail!("Product export must have exactly one source"),
        };
        let total = cursor.total();
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        let mut writer = WriterBuilder::new()
            .has_headers(false)
            .from_writer(ByteLimitWriter::new(
                BufWriter::new(file),
                MAX_PRODUCT_EXPORT_BYTES,
            ));
        writer.write_record([
            "product_id",
            "product_code",
            "name",
            "brand",
            "category",
            "status",
            "created_at",
            "updated_at",
        ])?;

        let mut processed = 0_u64;
        loop {
            let rows = cursor.fetch_next(1_000).await?;
            if rows.is_empty() {
                break;
            }
            for row in &rows {
                writer.write_record([
                    row.product_id.to_string(),
                    csv_safe_cell(&row.product_code.clone().unwrap_or_default()),
                    csv_safe_cell(&row.name),
                    csv_safe_cell(&row.brand.clone().unwrap_or_default()),
                    csv_safe_cell(&row.category.clone().unwrap_or_default()),
                    csv_safe_cell(&row.status),
                    row.created_at.to_rfc3339(),
                    row.updated_at.to_rfc3339(),
                ])?;
            }
            processed = processed.saturating_add(rows.len() as u64);
            update_progress_u64(repo, job, processed, total).await?;
        }

        cursor.finish().await?;
        writer.flush()?;
        let bounded = writer
            .into_inner()
            .map_err(|err| anyhow::anyhow!("failed to finish product CSV: {err}"))?;
        let file = bounded
            .into_inner()
            .into_inner()
            .map_err(|err| anyhow::anyhow!("failed to flush product CSV: {err}"))?;
        file.sync_all()?;
        drop(file);

        fs::hard_link(&temporary_path, &final_path)?;
        remove_file_if_present(&temporary_path)?;
        if let Err(err) = finalize_job(repo, job).await {
            let done = repo
                .get_job(job.tenant_id, job.job_id)
                .await?
                .is_some_and(|current| current.status == "DONE");
            if done {
                return Ok(());
            }
            return Err(err);
        }
        Ok(())
    })
    .await
    .unwrap_or_else(|_| {
        Err(anyhow::anyhow!(
            "Product export exceeded 300 second time limit"
        ))
    });

    if result.is_err() {
        remove_file_if_present(&temporary_path)?;
    }
    result
}

struct ByteLimitWriter<W> {
    inner: W,
    written: u64,
    limit: u64,
}

impl<W> ByteLimitWriter<W> {
    fn new(inner: W, limit: u64) -> Self {
        Self {
            inner,
            written: 0,
            limit,
        }
    }

    fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for ByteLimitWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let requested = u64::try_from(buffer.len()).unwrap_or(u64::MAX);
        if self.written.saturating_add(requested) > self.limit {
            return Err(std::io::Error::other(format!(
                "Product export exceeds {} byte CSV limit",
                self.limit
            )));
        }
        let written = self.inner.write(buffer)?;
        self.written = self.written.saturating_add(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn csv_safe_cell(value: &str) -> String {
    let trimmed = value.trim_start_matches([' ', '\t', '\r', '\n']);
    let begins_with_control = matches!(value.as_bytes().first(), Some(b'\t' | b'\r' | b'\n'));
    let numeric_identifier = trimmed.bytes().all(|byte| byte.is_ascii_digit())
        && (trimmed.len() >= 15 || (trimmed.len() > 1 && trimmed.starts_with('0')));
    if begins_with_control
        || numeric_identifier
        || matches!(trimmed.as_bytes().first(), Some(b'=' | b'+' | b'-' | b'@'))
    {
        format!("'{value}")
    } else {
        value.to_string()
    }
}

fn product_export_temporary_path(job: &ExportJob) -> PathBuf {
    let mut path = export_job_file_path(job.tenant_id, job.job_id);
    path.set_extension(format!("{}.csv.part", uuid::Uuid::new_v4().simple()));
    path
}

fn remove_file_if_present(path: &Path) -> anyhow::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

async fn import_skus(
    pool: &PgPool,
    repo: &ExportJobsRepo<'_>,
    job: &ExportJob,
) -> anyhow::Result<()> {
    let params: SkuImportParams =
        serde_json::from_value(job.params_json.clone()).unwrap_or_default();
    let duplicate_strategy = params
        .duplicate_strategy
        .as_deref()
        .unwrap_or("skip")
        .trim()
        .to_lowercase();
    let file_path = params
        .file_path
        .ok_or_else(|| anyhow::anyhow!("file_path missing"))?;
    let path = PathBuf::from(file_path);
    let file_type = resolve_import_file_type(
        params.file_type.as_deref(),
        params.file_name.as_deref(),
        &path,
    );

    let (header_index, raw_rows) = load_sku_import_rows(&path, &file_type)?;
    let total = raw_rows.len() as i64;
    if total == 0 {
        repo.update_job(
            job.tenant_id,
            job.job_id,
            ExportJobUpdate {
                status: Some("DONE".to_string()),
                output_url: Some(None),
                progress: Some(100),
                last_error: Some(String::new()),
                params_json: None,
            },
        )
        .await?;
        return Ok(());
    }

    let cost_enabled = FeatureFlagsRepo::new(pool)
        .is_enabled(job.tenant_id, None, FeatureFlag::CostManagement)
        .await
        .unwrap_or(true);

    let products_repo = ProductsRepo::new(pool);
    let skus_repo = SkusRepo::new(pool);
    let sku_suppliers_repo = SkuSuppliersRepo::new(pool);
    let mut product_cache: HashMap<String, (uuid::Uuid, Option<String>)> = HashMap::new();
    let mut product_id_cache: HashMap<uuid::Uuid, Option<String>> = HashMap::new();
    let existing_categories = fetch_existing_product_categories(pool, job.tenant_id).await?;
    let mut seen_sku_codes: HashSet<String> = HashSet::new();

    let mut processed = 0_i64;
    let mut success = 0_i64;
    let mut failed = 0_i64;
    let mut last_error = String::new();
    let mut failures: Vec<SkuImportFailure> = Vec::new();

    for raw in raw_rows {
        processed += 1;
        let row = match parse_sku_import_row(&raw, &header_index) {
            Ok(v) => v,
            Err(err) => {
                failed += 1;
                last_error = err.message.clone();
                failures.push(err);
                update_progress(repo, job, processed, total).await?;
                continue;
            }
        };

        let normalized_code = row.sku_code.trim().to_uppercase();
        if !seen_sku_codes.insert(normalized_code) {
            failed += 1;
            let failure = SkuImportFailure {
                row: row.row,
                sku_code: Some(row.sku_code.clone()),
                field: Some("sku_code".to_string()),
                message: "duplicate sku_code in file".to_string(),
            };
            last_error = failure.message.clone();
            failures.push(failure);
            update_progress(repo, job, processed, total).await?;
            continue;
        }
        if !existing_categories.contains(row.category.trim()) {
            failed += 1;
            let failure = SkuImportFailure {
                row: row.row,
                sku_code: Some(row.sku_code.clone()),
                field: Some("category".to_string()),
                message: "category not found".to_string(),
            };
            last_error = failure.message.clone();
            failures.push(failure);
            update_progress(repo, job, processed, total).await?;
            continue;
        }

        let product_id = match resolve_product_id(
            pool,
            &products_repo,
            job.tenant_id,
            &row,
            &mut product_cache,
            &mut product_id_cache,
        )
        .await
        {
            Ok(v) => v,
            Err(err) => {
                failed += 1;
                last_error = err.message.clone();
                failures.push(err);
                update_progress(repo, job, processed, total).await?;
                continue;
            }
        };

        let cost_cent = if cost_enabled { row.cost_cent } else { None };

        let existing_sku = fetch_sku_by_code(pool, job.tenant_id, &row.sku_code).await?;
        let sku = if let Some((sku_id, existing_product_id)) = existing_sku {
            if duplicate_strategy == "overwrite" {
                if existing_product_id != product_id {
                    failed += 1;
                    let failure = SkuImportFailure {
                        row: row.row,
                        sku_code: Some(row.sku_code.clone()),
                        field: Some("sku_code".to_string()),
                        message: "sku belongs to different product".to_string(),
                    };
                    last_error = failure.message.clone();
                    failures.push(failure);
                    update_progress(repo, job, processed, total).await?;
                    continue;
                }
                let updated = skus_repo
                    .update_sku(
                        job.tenant_id,
                        sku_id,
                        SkuUpdate {
                            sku_code: None,
                            barcode: row.barcode.clone(),
                            name: row.name.clone(),
                            note: None,
                            spec: Some(row.spec.clone()),
                            tags: Some(row.tags.clone()),
                            image_urls: Some(row.image_urls.clone()),
                            weight_gram: row.weight_gram,
                            length_mm: row.length_mm,
                            width_mm: row.width_mm,
                            height_mm: row.height_mm,
                            volume_cm3: row.volume_cm3,
                            sku_kind: Some(row.sku_kind.clone()),
                            cost_cent,
                            is_active: None,
                            status: None,
                            batch_enabled: None,
                            serial_enabled: None,
                        },
                    )
                    .await;
                match updated {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        failed += 1;
                        let failure = SkuImportFailure {
                            row: row.row,
                            sku_code: Some(row.sku_code.clone()),
                            field: Some("sku_code".to_string()),
                            message: "sku not found for overwrite".to_string(),
                        };
                        last_error = failure.message.clone();
                        failures.push(failure);
                        update_progress(repo, job, processed, total).await?;
                        continue;
                    }
                    Err(err) => {
                        failed += 1;
                        let failure = SkuImportFailure {
                            row: row.row,
                            sku_code: Some(row.sku_code.clone()),
                            field: Some("sku_code".to_string()),
                            message: err.to_string(),
                        };
                        last_error = failure.message.clone();
                        failures.push(failure);
                        update_progress(repo, job, processed, total).await?;
                        continue;
                    }
                }
            } else {
                failed += 1;
                let failure = SkuImportFailure {
                    row: row.row,
                    sku_code: Some(row.sku_code.clone()),
                    field: Some("sku_code".to_string()),
                    message: "sku_code already exists".to_string(),
                };
                last_error = failure.message.clone();
                failures.push(failure);
                update_progress(repo, job, processed, total).await?;
                continue;
            }
        } else {
            let created = skus_repo
                .create_sku(SkuCreate {
                    tenant_id: job.tenant_id,
                    product_id,
                    sku_code: row.sku_code.clone(),
                    barcode: row.barcode.clone(),
                    name: row.name.clone(),
                    note: None,
                    spec: row.spec.clone(),
                    tags: row.tags.clone(),
                    image_urls: row.image_urls.clone(),
                    weight_gram: row.weight_gram,
                    length_mm: row.length_mm,
                    width_mm: row.width_mm,
                    height_mm: row.height_mm,
                    volume_cm3: row.volume_cm3,
                    sku_kind: row.sku_kind.clone(),
                    cost_cent,
                    is_active: true,
                    status: None,
                    batch_enabled: false,
                    serial_enabled: false,
                })
                .await;

            match created {
                Ok(v) => v,
                Err(err) => {
                    failed += 1;
                    let failure = SkuImportFailure {
                        row: row.row,
                        sku_code: Some(row.sku_code.clone()),
                        field: Some("sku_code".to_string()),
                        message: err.to_string(),
                    };
                    last_error = failure.message.clone();
                    failures.push(failure);
                    update_progress(repo, job, processed, total).await?;
                    continue;
                }
            }
        };

        let mut row_failed = false;
        if !row.supplier_ids.is_empty() {
            if let Err(err) = sku_suppliers_repo
                .replace_sku_suppliers(job.tenant_id, sku.sku_id, row.supplier_ids.clone(), None)
                .await
            {
                failed += 1;
                let failure = SkuImportFailure {
                    row: row.row,
                    sku_code: Some(row.sku_code.clone()),
                    field: Some("supplier_ids".to_string()),
                    message: err.to_string(),
                };
                last_error = failure.message.clone();
                failures.push(failure);
                row_failed = true;
            }
        }

        if !row_failed {
            success += 1;
        }
        update_progress(repo, job, processed, total).await?;
    }

    if !failures.is_empty() {
        let _ = write_sku_import_failures(job.tenant_id, job.job_id, &failures);
    }

    let mut params_json = job.params_json.clone();
    if let Some(object) = params_json.as_object_mut() {
        object.insert("total_rows".to_string(), serde_json::json!(total));
        object.insert("success_rows".to_string(), serde_json::json!(success));
        object.insert("failed_rows".to_string(), serde_json::json!(failed));
    } else {
        params_json = serde_json::json!({
            "total_rows": total,
            "success_rows": success,
            "failed_rows": failed,
        });
    }

    let status = "DONE";
    repo.update_job(
        job.tenant_id,
        job.job_id,
        ExportJobUpdate {
            status: Some(status.to_string()),
            output_url: Some(None),
            progress: Some(100),
            last_error: Some(last_error),
            params_json: Some(params_json),
        },
    )
    .await?;

    Ok(())
}

#[derive(Debug)]
struct SkuImportRawRow {
    row: i64,
    values: Vec<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct SkuImportRow {
    row: i64,
    product_id: Option<uuid::Uuid>,
    product_name: Option<String>,
    sku_code: String,
    barcode: Option<String>,
    aux_barcode: Option<String>,
    name: Option<String>,
    spec: serde_json::Value,
    unit: String,
    category: String,
    sku_kind: String,
    tags: Vec<String>,
    image_urls: Vec<String>,
    cost_cent: Option<i64>,
    supplier_ids: Vec<uuid::Uuid>,
    weight_gram: Option<i32>,
    length_mm: Option<i32>,
    width_mm: Option<i32>,
    height_mm: Option<i32>,
    volume_cm3: Option<i32>,
}

#[derive(Default)]
struct SkuImportHeaderIndex {
    product_id: Option<usize>,
    product_name: Option<usize>,
    sku_code: Option<usize>,
    barcode: Option<usize>,
    aux_barcode: Option<usize>,
    name: Option<usize>,
    spec: Option<usize>,
    unit: Option<usize>,
    category_id: Option<usize>,
    category_name: Option<usize>,
    sku_kind: Option<usize>,
    tags: Option<usize>,
    image_urls: Option<usize>,
    cost_cent: Option<usize>,
    supplier_ids: Option<usize>,
    weight_gram: Option<usize>,
    length_mm: Option<usize>,
    width_mm: Option<usize>,
    height_mm: Option<usize>,
    volume_cm3: Option<usize>,
}

#[derive(Debug)]
struct SkuImportFailure {
    row: i64,
    sku_code: Option<String>,
    field: Option<String>,
    message: String,
}

fn load_sku_import_rows(
    path: &Path,
    file_type: &str,
) -> anyhow::Result<(SkuImportHeaderIndex, Vec<SkuImportRawRow>)> {
    if file_type.eq_ignore_ascii_case("CSV") {
        return load_sku_import_csv_rows(path);
    }
    load_sku_import_excel_rows(path)
}

fn load_sku_import_csv_rows(
    path: &Path,
) -> anyhow::Result<(SkuImportHeaderIndex, Vec<SkuImportRawRow>)> {
    let file = File::open(path)?;
    let mut reader = ReaderBuilder::new().trim(csv::Trim::All).from_reader(file);
    let headers = reader
        .headers()
        .map(|hdr| hdr.iter().map(normalize_import_header).collect::<Vec<_>>())?;
    let indexes = build_sku_import_header_index(&headers)?;

    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record = record?;
        let values = record.iter().map(|value| value.to_string()).collect();
        rows.push(SkuImportRawRow {
            row: index as i64 + 2,
            values,
        });
    }
    Ok((indexes, rows))
}

fn load_sku_import_excel_rows(
    path: &Path,
) -> anyhow::Result<(SkuImportHeaderIndex, Vec<SkuImportRawRow>)> {
    let mut workbook = open_workbook_auto(path)?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing sheet"))?;
    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|_| anyhow::anyhow!("invalid sheet"))?;

    let mut rows_iter = range.rows();
    let header_row = rows_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty sheet"))?;
    let headers = header_row
        .iter()
        .map(|cell| normalize_import_header(&cell.to_string()))
        .collect::<Vec<_>>();
    let indexes = build_sku_import_header_index(&headers)?;

    let mut rows = Vec::new();
    for (index, row) in rows_iter.enumerate() {
        let values = row.iter().map(cell_string).collect();
        rows.push(SkuImportRawRow {
            row: index as i64 + 2,
            values,
        });
    }
    Ok((indexes, rows))
}

fn build_import_failure(
    row: i64,
    sku_code: Option<&str>,
    field: Option<&str>,
    message: &str,
) -> SkuImportFailure {
    SkuImportFailure {
        row,
        sku_code: sku_code.map(|value| value.to_string()),
        field: field.map(|value| value.to_string()),
        message: message.to_string(),
    }
}

fn parse_optional_i64_field(
    raw: &str,
    row: i64,
    sku_code: Option<&str>,
    field: &str,
) -> Result<Option<i64>, SkuImportFailure> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<i64>()
        .map(Some)
        .map_err(|_| build_import_failure(row, sku_code, Some(field), &format!("invalid {field}")))
}

fn parse_optional_i32_field(
    raw: &str,
    row: i64,
    sku_code: Option<&str>,
    field: &str,
) -> Result<Option<i32>, SkuImportFailure> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<i32>()
        .map(Some)
        .map_err(|_| build_import_failure(row, sku_code, Some(field), &format!("invalid {field}")))
}

fn build_spec_value(raw: Option<&str>, unit: &str, aux_barcode: Option<&str>) -> serde_json::Value {
    let trimmed = raw.and_then(|value| {
        let value = value.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    });
    let parsed = trimmed
        .as_deref()
        .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok());

    let mut map = match parsed {
        Some(serde_json::Value::Object(map)) => map,
        Some(other) => {
            let mut map = serde_json::Map::new();
            map.insert("spec_value".to_string(), other);
            map
        }
        None => {
            let mut map = serde_json::Map::new();
            if let Some(value) = trimmed {
                map.insert("spec_text".to_string(), serde_json::Value::String(value));
            }
            map
        }
    };
    map.insert(
        "unit".to_string(),
        serde_json::Value::String(unit.to_string()),
    );
    if let Some(value) = aux_barcode {
        if !value.trim().is_empty() {
            map.insert(
                "aux_barcode".to_string(),
                serde_json::Value::String(value.trim().to_string()),
            );
        }
    }
    serde_json::Value::Object(map)
}

fn parse_sku_import_row(
    raw: &SkuImportRawRow,
    indexes: &SkuImportHeaderIndex,
) -> Result<SkuImportRow, SkuImportFailure> {
    let get = |idx: Option<usize>| {
        idx.and_then(|i| raw.values.get(i))
            .map(|v| v.as_str())
            .unwrap_or("")
    };
    let sku_code = get(indexes.sku_code).trim().to_string();
    if sku_code.is_empty() {
        return Err(build_import_failure(
            raw.row,
            None,
            Some("sku_code"),
            "sku_code required",
        ));
    }
    let product_id = match optional_string(get(indexes.product_id)) {
        Some(value) => Some(uuid::Uuid::parse_str(value.trim()).map_err(|_| {
            build_import_failure(
                raw.row,
                Some(&sku_code),
                Some("product_id"),
                "invalid product_id",
            )
        })?),
        None => None,
    };
    let product_name = optional_string(get(indexes.product_name));
    if product_id.is_none() && product_name.is_none() {
        return Err(build_import_failure(
            raw.row,
            Some(&sku_code),
            Some("product_id"),
            "product_id or product_name required",
        ));
    }

    let unit = optional_string(get(indexes.unit)).unwrap_or_default();
    if unit.trim().is_empty() {
        return Err(build_import_failure(
            raw.row,
            Some(&sku_code),
            Some("unit"),
            "unit required",
        ));
    }

    let category_id = optional_string(get(indexes.category_id));
    let category_name = optional_string(get(indexes.category_name));
    let category = match (category_id.as_deref(), category_name.as_deref()) {
        (Some(id), Some(name)) if id != name => {
            return Err(build_import_failure(
                raw.row,
                Some(&sku_code),
                Some("category"),
                "category_id and category_name mismatch",
            ));
        }
        (Some(id), _) => id.trim().to_string(),
        (None, Some(name)) => name.trim().to_string(),
        (None, None) => {
            return Err(build_import_failure(
                raw.row,
                Some(&sku_code),
                Some("category"),
                "category required",
            ));
        }
    };
    let aux_barcode = optional_string(get(indexes.aux_barcode));
    let spec = build_spec_value(
        optional_string(get(indexes.spec)).as_deref(),
        unit.trim(),
        aux_barcode.as_deref(),
    );
    let sku_kind = optional_string(get(indexes.sku_kind))
        .map(|value| value.trim().to_uppercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "NORMAL".to_string());
    if sku_kind != "NORMAL" && sku_kind != "KIT" {
        return Err(build_import_failure(
            raw.row,
            Some(&sku_code),
            Some("sku_kind"),
            "invalid sku_kind",
        ));
    }

    let cost_cent = parse_optional_i64_field(
        get(indexes.cost_cent),
        raw.row,
        Some(&sku_code),
        "cost_cent",
    )?;
    let weight_gram = parse_optional_i32_field(
        get(indexes.weight_gram),
        raw.row,
        Some(&sku_code),
        "weight_gram",
    )?;
    let length_mm = parse_optional_i32_field(
        get(indexes.length_mm),
        raw.row,
        Some(&sku_code),
        "length_mm",
    )?;
    let width_mm =
        parse_optional_i32_field(get(indexes.width_mm), raw.row, Some(&sku_code), "width_mm")?;
    let height_mm = parse_optional_i32_field(
        get(indexes.height_mm),
        raw.row,
        Some(&sku_code),
        "height_mm",
    )?;
    let volume_cm3 = parse_optional_i32_field(
        get(indexes.volume_cm3),
        raw.row,
        Some(&sku_code),
        "volume_cm3",
    )?;

    let supplier_ids = split_import_list(get(indexes.supplier_ids))
        .into_iter()
        .map(|value| {
            uuid::Uuid::parse_str(&value).map_err(|_| {
                build_import_failure(
                    raw.row,
                    Some(&sku_code),
                    Some("supplier_ids"),
                    "invalid supplier_id",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SkuImportRow {
        row: raw.row,
        product_id,
        product_name,
        sku_code,
        barcode: optional_string(get(indexes.barcode)),
        aux_barcode,
        name: optional_string(get(indexes.name)),
        spec,
        unit: unit.trim().to_string(),
        category,
        sku_kind,
        tags: normalize_string_list(split_import_list(get(indexes.tags))),
        image_urls: normalize_string_list(split_import_list(get(indexes.image_urls))),
        cost_cent,
        supplier_ids,
        weight_gram,
        length_mm,
        width_mm,
        height_mm,
        volume_cm3,
    })
}

async fn resolve_product_id(
    pool: &PgPool,
    repo: &ProductsRepo<'_>,
    tenant_id: erp_types::TenantId,
    row: &SkuImportRow,
    cache: &mut HashMap<String, (uuid::Uuid, Option<String>)>,
    id_cache: &mut HashMap<uuid::Uuid, Option<String>>,
) -> Result<uuid::Uuid, SkuImportFailure> {
    #[derive(sqlx::FromRow)]
    struct ProductRow {
        product_id: uuid::Uuid,
        category: Option<String>,
    }

    let ensure_category_match = |category: &Option<String>| -> Result<(), SkuImportFailure> {
        match category.as_deref().map(str::trim) {
            Some(value) if !value.is_empty() && value == row.category.as_str() => Ok(()),
            Some(_) => Err(build_import_failure(
                row.row,
                Some(&row.sku_code),
                Some("category"),
                "product category mismatch",
            )),
            None => Err(build_import_failure(
                row.row,
                Some(&row.sku_code),
                Some("category"),
                "product category missing",
            )),
        }
    };

    if let Some(product_id) = row.product_id {
        if let Some(category) = id_cache.get(&product_id) {
            ensure_category_match(category)?;
            return Ok(product_id);
        }
        let existing: Option<ProductRow> = sqlx::query_as(
            r#"
            select product_id, category
            from products
            where tenant_id = $1 and product_id = $2 and status <> 'DELETED'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(product_id)
        .fetch_optional(pool)
        .await
        .map_err(|err| {
            build_import_failure(
                row.row,
                Some(&row.sku_code),
                Some("product_id"),
                &err.to_string(),
            )
        })?;
        let existing = match existing {
            Some(value) => value,
            None => {
                return Err(build_import_failure(
                    row.row,
                    Some(&row.sku_code),
                    Some("product_id"),
                    "product not found",
                ));
            }
        };
        ensure_category_match(&existing.category)?;
        id_cache.insert(existing.product_id, existing.category.clone());
        return Ok(existing.product_id);
    }

    let name = row
        .product_name
        .as_ref()
        .ok_or_else(|| {
            build_import_failure(
                row.row,
                Some(&row.sku_code),
                Some("product_name"),
                "product_name required",
            )
        })?
        .trim()
        .to_string();
    if let Some((cached_id, cached_category)) = cache.get(&name) {
        ensure_category_match(cached_category)?;
        return Ok(*cached_id);
    }

    let existing: Option<ProductRow> = sqlx::query_as(
        r#"
        select product_id, category
        from products
        where tenant_id = $1 and name = $2 and status <> 'DELETED'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(&name)
    .fetch_optional(pool)
    .await
    .map_err(|err| {
        build_import_failure(
            row.row,
            Some(&row.sku_code),
            Some("product_name"),
            &err.to_string(),
        )
    })?;

    let product = if let Some(existing) = existing {
        ensure_category_match(&existing.category)?;
        existing
    } else {
        let created = repo
            .create_product(ProductCreate {
                tenant_id,
                product_code: None,
                name: name.clone(),
                alias: None,
                brand: None,
                category: Some(row.category.clone()),
                base_unit: None,
                second_unit: None,
                default_supplier_id: None,
                default_warehouse_id: None,
                primary_barcode: None,
                product_tags: Vec::new(),
                remark: None,
                warning_stock: serde_json::json!({}),
                batch_enabled: false,
                shelf_life_days: None,
                forbid_sale_days: None,
                serial_enabled: false,
                serial_push_platform: false,
                attrs: serde_json::json!({}),
                allow_negative_stock: false,
            })
            .await
            .map_err(|err| {
                build_import_failure(
                    row.row,
                    Some(&row.sku_code),
                    Some("product_name"),
                    &err.to_string(),
                )
            })?;
        ProductRow {
            product_id: created.product_id,
            category: created.category,
        }
    };

    cache.insert(name, (product.product_id, product.category.clone()));
    id_cache.insert(product.product_id, product.category.clone());
    Ok(product.product_id)
}

async fn fetch_existing_product_categories(
    pool: &PgPool,
    tenant_id: erp_types::TenantId,
) -> anyhow::Result<HashSet<String>> {
    let rows: Vec<Option<String>> = sqlx::query_scalar(
        r#"
        select distinct category
        from products
        where tenant_id = $1
          and status <> 'DELETED'
          and category is not null
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect())
}

async fn fetch_sku_by_code(
    pool: &PgPool,
    tenant_id: erp_types::TenantId,
    sku_code: &str,
) -> anyhow::Result<Option<(uuid::Uuid, uuid::Uuid)>> {
    let row: Option<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
        r#"
        select sku_id, product_id
        from skus
        where tenant_id = $1 and sku_code = $2 and status <> 'DELETED'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(sku_code)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

fn write_sku_import_failures(
    tenant_id: erp_types::TenantId,
    job_id: uuid::Uuid,
    failures: &[SkuImportFailure],
) -> anyhow::Result<PathBuf> {
    let file_path = sku_import_failure_file_path(tenant_id, job_id);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(&file_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all("\u{FEFF}".as_bytes())?;
    let mut csv_writer = WriterBuilder::new().from_writer(writer);
    csv_writer.write_record(["row", "sku_code", "field", "message"])?;
    for failure in failures {
        csv_writer.write_record([
            failure.row.to_string(),
            failure.sku_code.clone().unwrap_or_default(),
            failure.field.clone().unwrap_or_default(),
            failure.message.clone(),
        ])?;
    }
    csv_writer.flush()?;
    Ok(file_path)
}

fn resolve_import_file_type(
    file_type: Option<&str>,
    file_name: Option<&str>,
    path: &Path,
) -> String {
    if let Some(raw) = file_type {
        let normalized = raw.trim().to_uppercase();
        if matches!(normalized.as_str(), "CSV" | "XLS" | "XLSX") {
            return normalized;
        }
    }
    if let Some(name) = file_name {
        let lower = name.trim().to_lowercase();
        if lower.ends_with(".csv") {
            return "CSV".to_string();
        }
        if lower.ends_with(".xls") || lower.ends_with(".xlsx") {
            return "XLSX".to_string();
        }
    }
    if let Some(ext) = path.extension().and_then(|v| v.to_str()) {
        let lower = ext.to_lowercase();
        if lower == "csv" {
            return "CSV".to_string();
        }
        if lower == "xls" || lower == "xlsx" {
            return "XLSX".to_string();
        }
    }
    "CSV".to_string()
}

fn sku_import_failure_file_path(tenant_id: erp_types::TenantId, job_id: uuid::Uuid) -> PathBuf {
    let mut base = sku_import_failure_base_dir();
    base.push(tenant_id.to_string());
    base.push(format!("{job_id}.csv"));
    base
}

fn sku_import_failure_base_dir() -> PathBuf {
    std::env::var("SKU_IMPORT_FAILURE_BASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("imports/sku_failures"))
}

fn normalize_import_header(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace([' ', '-', '/', '.', '(', ')', '（', '）'], "_")
        .replace("__", "_")
        .trim_matches('_')
        .to_string()
}

fn build_sku_import_header_index(headers: &[String]) -> anyhow::Result<SkuImportHeaderIndex> {
    let mut index = SkuImportHeaderIndex::default();
    for (idx, header) in headers.iter().enumerate() {
        match header.as_str() {
            "product_id" | "productid" | "货品编号" | "商品编号" | "产品编号" => {
                index.product_id = Some(idx)
            }
            "product_name" | "product" | "productname" | "货品名称" | "商品名称" | "产品名称" => {
                index.product_name = Some(idx)
            }
            "sku_code" | "sku" | "skucode" | "规格编号" | "规格编码" | "sku编号" | "sku编码" => {
                index.sku_code = Some(idx)
            }
            "barcode" | "bar_code" | "条码" => index.barcode = Some(idx),
            "aux_barcode" | "assistant_barcode" | "辅助条码" | "辅助码" => {
                index.aux_barcode = Some(idx)
            }
            "name" | "sku_name" | "sku名称" | "规格名称" => index.name = Some(idx),
            "spec" | "spec_info" | "规格信息" | "规格" => index.spec = Some(idx),
            "unit" | "单位" => index.unit = Some(idx),
            "category_id" | "categoryid" | "分类编号" | "分类id" | "分类编码" => {
                index.category_id = Some(idx)
            }
            "category_name" | "category" | "分类名称" | "分类" => {
                index.category_name = Some(idx)
            }
            "sku_kind" | "kind" | "type" | "规格类型" | "sku类型" => {
                index.sku_kind = Some(idx)
            }
            "tags" | "tag" | "标签" => index.tags = Some(idx),
            "image_urls" | "images" | "image_url" | "图片" | "图片链接" => {
                index.image_urls = Some(idx)
            }
            "cost_cent" | "cost" | "成本" => index.cost_cent = Some(idx),
            "supplier_ids" | "suppliers" | "supplier_id" | "供应商" | "供应商id" => {
                index.supplier_ids = Some(idx)
            }
            "weight_gram" | "weight" | "weight_g" | "重量" | "重量g" | "重量_g" => {
                index.weight_gram = Some(idx)
            }
            "length_mm" | "length" | "长度" | "长度_mm" => index.length_mm = Some(idx),
            "width_mm" | "width" | "宽度" | "宽度_mm" => index.width_mm = Some(idx),
            "height_mm" | "height" | "高度" | "高度_mm" => index.height_mm = Some(idx),
            "volume_cm3" | "volume" | "体积" | "体积_cm3" => index.volume_cm3 = Some(idx),
            _ => {}
        }
    }
    if index.sku_code.is_none() {
        return Err(anyhow::anyhow!("missing sku_code column"));
    }
    if index.product_id.is_none() && index.product_name.is_none() {
        return Err(anyhow::anyhow!("missing product_id or product_name column"));
    }
    if index.unit.is_none() {
        return Err(anyhow::anyhow!("missing unit column"));
    }
    if index.category_id.is_none() && index.category_name.is_none() {
        return Err(anyhow::anyhow!("missing category column"));
    }
    Ok(index)
}

fn split_import_list(raw: &str) -> Vec<String> {
    raw.split([',', ';', '|'])
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            result.push(trimmed.to_string());
        }
    }
    result
}

fn optional_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[allow(dead_code)]
fn parse_optional_i64(raw: &str) -> Result<Option<i64>, anyhow::Error> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let value = trimmed.parse::<i64>()?;
    Ok(Some(value))
}

#[allow(dead_code)]
fn parse_optional_i32(raw: &str) -> Result<Option<i32>, anyhow::Error> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let value = trimmed.parse::<i32>()?;
    Ok(Some(value))
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

#[derive(Debug, sqlx::FromRow)]
struct OrderRow {
    order_id: uuid::Uuid,
    platform: String,
    shop_id: uuid::Uuid,
    platform_order_id: String,
    status: String,
    total_amount_cent: i64,
    currency: String,
    order_created_at: DateTime<Utc>,
    ship_at: Option<DateTime<Utc>>,
}

async fn count_orders(
    pool: &PgPool,
    job: &ExportJob,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    platform: Option<&str>,
    shop_id: Option<uuid::Uuid>,
) -> anyhow::Result<i64> {
    let mut qb = QueryBuilder::<Postgres>::new("select count(*) from orders where tenant_id = ");
    qb.push_bind(uuid::Uuid::from(job.tenant_id));
    qb.push(" and order_created_at >= ");
    qb.push_bind(start_at);
    qb.push(" and order_created_at <= ");
    qb.push_bind(end_at);
    if let Some(platform) = platform {
        qb.push(" and platform = ");
        qb.push_bind(platform);
    }
    if let Some(shop_id) = shop_id {
        qb.push(" and shop_id = ");
        qb.push_bind(shop_id);
    }
    let total: i64 = qb.build_query_scalar().fetch_one(pool).await?;
    Ok(total)
}

#[allow(clippy::too_many_arguments)]
async fn fetch_orders(
    pool: &PgPool,
    job: &ExportJob,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    platform: Option<&str>,
    shop_id: Option<uuid::Uuid>,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<OrderRow>> {
    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        select
            order_id,
            platform,
            shop_id,
            platform_order_id,
            status,
            total_amount_cent,
            currency,
            order_created_at,
            ship_at
        from orders
        where tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(job.tenant_id));
    qb.push(" and order_created_at >= ");
    qb.push_bind(start_at);
    qb.push(" and order_created_at <= ");
    qb.push_bind(end_at);
    if let Some(platform) = platform {
        qb.push(" and platform = ");
        qb.push_bind(platform);
    }
    if let Some(shop_id) = shop_id {
        qb.push(" and shop_id = ");
        qb.push_bind(shop_id);
    }
    qb.push(" order by order_created_at desc, order_id desc");
    qb.push(" limit ");
    qb.push_bind(limit);
    qb.push(" offset ");
    qb.push_bind(offset);

    let rows: Vec<OrderRow> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows)
}

#[derive(Debug, sqlx::FromRow)]
struct ReconcileRow {
    order_id: uuid::Uuid,
    platform: String,
    platform_order_id: String,
    order_total_cent: i64,
    items_amount_cent: i64,
    refund_amount_cent: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ReconcileItemRow {
    ref_type: String,
    ref_id: String,
    expected_cent: i64,
    actual_cent: i64,
    diff_cent: i64,
    diff_reason: Option<String>,
    created_at: DateTime<Utc>,
}

#[allow(clippy::too_many_arguments)]
async fn fetch_reconcile_rows(
    pool: &PgPool,
    job: &ExportJob,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    platform: Option<&str>,
    shop_id: Option<uuid::Uuid>,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<ReconcileRow>> {
    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        select
            o.order_id,
            o.platform,
            o.platform_order_id,
            o.total_amount_cent as order_total_cent,
            coalesce(sum(oi.amount_cent), 0) as items_amount_cent,
            coalesce(sum(oi.refund_amount_cent), 0) as refund_amount_cent
        from orders o
        left join order_items oi
          on oi.tenant_id = o.tenant_id
         and oi.order_id = o.order_id
         and oi.order_month = o.order_month
        where o.tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(job.tenant_id));
    qb.push(" and o.order_created_at >= ");
    qb.push_bind(start_at);
    qb.push(" and o.order_created_at <= ");
    qb.push_bind(end_at);
    if let Some(platform) = platform {
        qb.push(" and o.platform = ");
        qb.push_bind(platform);
    }
    if let Some(shop_id) = shop_id {
        qb.push(" and o.shop_id = ");
        qb.push_bind(shop_id);
    }
    qb.push(" group by o.order_id, o.platform, o.platform_order_id, o.total_amount_cent");
    qb.push(" order by max(o.order_created_at) desc, o.order_id desc");
    qb.push(" limit ");
    qb.push_bind(limit);
    qb.push(" offset ");
    qb.push_bind(offset);

    let rows: Vec<ReconcileRow> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows)
}

async fn count_reconcile_items(
    pool: &PgPool,
    job: &ExportJob,
    job_id: uuid::Uuid,
    diff_only: bool,
) -> anyhow::Result<i64> {
    let mut qb = QueryBuilder::<Postgres>::new(
        "select count(*) from reconcile_job_items where tenant_id = ",
    );
    qb.push_bind(uuid::Uuid::from(job.tenant_id));
    qb.push(" and job_id = ");
    qb.push_bind(job_id);
    if diff_only {
        qb.push(" and diff_cent != 0");
    }
    let total: i64 = qb.build_query_scalar().fetch_one(pool).await?;
    Ok(total)
}

async fn fetch_reconcile_items(
    pool: &PgPool,
    job: &ExportJob,
    job_id: uuid::Uuid,
    diff_only: bool,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<ReconcileItemRow>> {
    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        select
            ref_type,
            ref_id,
            expected_cent,
            actual_cent,
            diff_cent,
            diff_reason,
            created_at
        from reconcile_job_items
        where tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(job.tenant_id));
    qb.push(" and job_id = ");
    qb.push_bind(job_id);
    if diff_only {
        qb.push(" and diff_cent != 0");
    }
    qb.push(" order by created_at desc, ref_type, ref_id");
    qb.push(" limit ");
    qb.push_bind(limit);
    qb.push(" offset ");
    qb.push_bind(offset);

    let rows: Vec<ReconcileItemRow> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows)
}

#[derive(Debug, sqlx::FromRow)]
struct InventoryRow {
    sku_id: uuid::Uuid,
    sku_code: Option<String>,
    warehouse_id: uuid::Uuid,
    on_hand: i64,
    reserved: i64,
}

async fn count_inventory(
    pool: &PgPool,
    job: &ExportJob,
    warehouse_id: Option<uuid::Uuid>,
) -> anyhow::Result<i64> {
    let mut qb =
        QueryBuilder::<Postgres>::new("select count(*) from inventory_balances where tenant_id = ");
    qb.push_bind(uuid::Uuid::from(job.tenant_id));
    if let Some(warehouse_id) = warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    let total: i64 = qb.build_query_scalar().fetch_one(pool).await?;
    Ok(total)
}

async fn fetch_inventory_rows(
    pool: &PgPool,
    job: &ExportJob,
    warehouse_id: Option<uuid::Uuid>,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<InventoryRow>> {
    let mut qb = QueryBuilder::<Postgres>::new(
        r#"
        select
            b.sku_id,
            s.sku_code,
            b.warehouse_id,
            b.on_hand,
            b.reserved
        from inventory_balances b
        join skus s
          on s.tenant_id = b.tenant_id
         and s.sku_id = b.sku_id
        where b.tenant_id =
        "#,
    );
    qb.push_bind(uuid::Uuid::from(job.tenant_id));
    if let Some(warehouse_id) = warehouse_id {
        qb.push(" and b.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    qb.push(" order by s.sku_code nulls last, b.sku_id");
    qb.push(" limit ");
    qb.push_bind(limit);
    qb.push(" offset ");
    qb.push_bind(offset);

    let rows: Vec<InventoryRow> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows)
}

async fn update_progress(
    repo: &ExportJobsRepo<'_>,
    job: &ExportJob,
    processed: i64,
    total: i64,
) -> anyhow::Result<()> {
    let progress = if total == 0 {
        100
    } else {
        ((processed * 100) / total).clamp(0, 100) as i32
    };
    repo.update_job(
        job.tenant_id,
        job.job_id,
        ExportJobUpdate {
            status: None,
            output_url: None,
            progress: Some(progress),
            last_error: None,
            params_json: None,
        },
    )
    .await?;
    Ok(())
}

async fn update_progress_u64(
    repo: &ExportJobsRepo<'_>,
    job: &ExportJob,
    processed: u64,
    total: u64,
) -> anyhow::Result<()> {
    let progress = if total == 0 {
        100
    } else {
        ((processed.saturating_mul(100)) / total).min(100) as i32
    };
    repo.update_job(
        job.tenant_id,
        job.job_id,
        ExportJobUpdate {
            status: None,
            output_url: None,
            progress: Some(progress),
            last_error: None,
            params_json: None,
        },
    )
    .await?;
    Ok(())
}

async fn finalize_job(repo: &ExportJobsRepo<'_>, job: &ExportJob) -> anyhow::Result<()> {
    let output_url = format!("/exports/jobs/{}/file", job.job_id);
    repo.update_job(
        job.tenant_id,
        job.job_id,
        ExportJobUpdate {
            status: Some("DONE".to_string()),
            output_url: Some(Some(output_url)),
            progress: Some(100),
            last_error: Some(String::new()),
            params_json: None,
        },
    )
    .await?;
    Ok(())
}

fn build_writer(job: &ExportJob) -> anyhow::Result<csv::Writer<BufWriter<File>>> {
    let path = export_job_file_path(job.tenant_id, job.job_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    Ok(WriterBuilder::new()
        .has_headers(false)
        .from_writer(BufWriter::new(file)))
}

fn export_job_file_path(tenant_id: erp_types::TenantId, job_id: uuid::Uuid) -> PathBuf {
    let mut base = export_base_dir();
    base.push(tenant_id.to_string());
    base.push(format!("{job_id}.csv"));
    base
}

fn export_base_dir() -> PathBuf {
    std::env::var("EXPORT_BASE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("exports"))
}

fn parse_range(params: &ExportParams) -> anyhow::Result<(DateTime<Utc>, DateTime<Utc>)> {
    let end_at = match params.end_at.as_deref() {
        Some(raw) => DateTime::parse_from_rfc3339(raw)
            .map_err(|_| anyhow::anyhow!("invalid end_at"))?
            .with_timezone(&Utc),
        None => Utc::now(),
    };
    let start_at = match params.start_at.as_deref() {
        Some(raw) => DateTime::parse_from_rfc3339(raw)
            .map_err(|_| anyhow::anyhow!("invalid start_at"))?
            .with_timezone(&Utc),
        None => end_at - ChronoDuration::days(7),
    };
    if start_at > end_at {
        return Err(anyhow::anyhow!("start_at after end_at"));
    }
    Ok((start_at, end_at))
}

fn parse_uuid(raw: Option<&str>) -> anyhow::Result<Option<uuid::Uuid>> {
    match raw {
        Some(value) if !value.trim().is_empty() => Ok(Some(uuid::Uuid::parse_str(value)?)),
        _ => Ok(None),
    }
}

fn parse_u64(key: &str, default: u64) -> anyhow::Result<u64> {
    let raw = std::env::var(key).unwrap_or_else(|_| default.to_string());
    let value = raw
        .parse()
        .map_err(|_| anyhow::anyhow!("{key} must be an integer"))?;
    Ok(value)
}

fn parse_i64(key: &str, default: i64) -> anyhow::Result<i64> {
    let raw = std::env::var(key).unwrap_or_else(|_| default.to_string());
    let value = raw
        .parse()
        .map_err(|_| anyhow::anyhow!("{key} must be an integer"))?;
    Ok(value)
}

fn parse_i32(key: &str, default: i32) -> anyhow::Result<i32> {
    let raw = std::env::var(key).unwrap_or_else(|_| default.to_string());
    let value = raw
        .parse()
        .map_err(|_| anyhow::anyhow!("{key} must be an integer"))?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, TimeZone};
    use erp_repo::db::Db;
    use erp_types::TenantId;
    use sqlx::PgPool;
    use std::io::{BufRead, BufReader};

    async fn setup_schema(pool: &PgPool) -> anyhow::Result<()> {
        let statements = [
            "create extension if not exists pgcrypto",
            r#"
            create table if not exists tenants (
              tenant_id uuid primary key,
              name text not null,
              status text not null default 'ACTIVE',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists orders (
              tenant_id uuid not null,
              order_month date not null,
              order_id uuid not null,
                platform text not null,
                shop_id uuid not null,
                platform_order_id text not null,
                status text not null,
                oms_status text not null default 'NEW',
                order_status text not null default 'PENDING_REVIEW',
                fulfillment_status text not null default 'NOT_ALLOCATED',
                exception_status text not null default 'NONE',
                review_status text not null default 'NOT_REVIEWED',
                exception_flags jsonb not null default '[]'::jsonb,
                total_amount_cent bigint not null default 0,
              currency text not null default 'CNY',
              order_created_at timestamptz not null,
              ship_at timestamptz,
              primary key (tenant_id, order_month, order_id)
            )
            "#,
            r#"
            create table if not exists export_jobs (
              job_id uuid primary key default gen_random_uuid(),
              tenant_id uuid not null references tenants(tenant_id),
              job_type text not null,
              params_json jsonb not null default '{}'::jsonb,
              status text not null default 'PENDING',
              output_url text,
              progress int not null default 0,
              attempts int not null default 0,
              last_error text not null default '',
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
        ];

        for stmt in statements {
            sqlx::query(stmt).execute(pool).await?;
        }
        Ok(())
    }

    async fn insert_tenant(pool: &PgPool, tenant_id: uuid::Uuid) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(tenant_id)
        .bind("export-test")
        .execute(pool)
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn export_orders_handles_large_dataset() -> anyhow::Result<()> {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return Ok(());
        };

        let db = Db::connect(&database_url).await?;
        setup_schema(db.pool()).await?;

        let tenant_id = uuid::Uuid::new_v4();
        let shop_id = uuid::Uuid::new_v4();
        insert_tenant(db.pool(), tenant_id).await?;

        let now = Utc::now();
        let order_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
        sqlx::query(
            r#"
            insert into orders (
              tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
              total_amount_cent, currency, order_created_at, ship_at
            )
            select
              $1, $2, gen_random_uuid(), $3, $4, concat('ORDER-', g), 'SHIPPED',
              1200, 'CNY', $5, $5
            from generate_series(1, 100000) g
            "#,
        )
        .bind(tenant_id)
        .bind(order_month)
        .bind("taobao")
        .bind(shop_id)
        .bind(now)
        .execute(db.pool())
        .await?;

        let repo = ExportJobsRepo::new(db.pool());
        let start_at = now - ChronoDuration::days(1);
        let end_at = now + ChronoDuration::days(1);
        let job = repo
            .create_job(erp_repo::export_jobs::ExportJobCreate {
                tenant_id: TenantId(tenant_id),
                job_type: "ORDER_EXPORT".to_string(),
                params_json: serde_json::json!({
                    "start_at": start_at.to_rfc3339(),
                    "end_at": end_at.to_rfc3339()
                }),
            })
            .await?;

        let config = ExportJobConfig {
            poll_interval: Duration::from_millis(10),
            batch_size: 1,
            page_size: 1000,
            max_attempts: 1,
        };

        export_orders(
            db.pool(),
            &repo,
            &config,
            &job,
            &ExportParams::default(),
            start_at,
            end_at,
        )
        .await?;

        let updated = repo
            .get_job(TenantId(tenant_id), job.job_id)
            .await?
            .expect("job exists");
        assert_eq!(updated.status, "DONE");
        assert_eq!(updated.progress, 100);
        assert!(updated.output_url.is_some());

        let file_path = export_job_file_path(job.tenant_id, job.job_id);
        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);
        let line_count = reader.lines().count();
        assert_eq!(line_count, 100001);

        std::fs::remove_file(file_path).ok();

        Ok(())
    }

    #[tokio::test]
    async fn export_orders_paginates_stably() -> anyhow::Result<()> {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            return Ok(());
        };

        let db = Db::connect(&database_url).await?;
        setup_schema(db.pool()).await?;

        let tenant_id = uuid::Uuid::new_v4();
        let shop_id = uuid::Uuid::new_v4();
        insert_tenant(db.pool(), tenant_id).await?;

        let created_at = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let order_month =
            chrono::NaiveDate::from_ymd_opt(created_at.year(), created_at.month(), 1).unwrap();

        let order_ids = [
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
        ];

        for order_id in order_ids {
            sqlx::query(
                r#"
                insert into orders (
                  tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
                  total_amount_cent, currency, order_created_at, ship_at
                ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
                "#,
            )
            .bind(tenant_id)
            .bind(order_month)
            .bind(order_id)
            .bind("taobao")
            .bind(shop_id)
            .bind(format!("ORDER-{order_id}"))
            .bind("SHIPPED")
            .bind(100_i64)
            .bind("CNY")
            .bind(created_at)
            .bind(created_at)
            .execute(db.pool())
            .await?;
        }

        let repo = ExportJobsRepo::new(db.pool());
        let job = repo
            .create_job(erp_repo::export_jobs::ExportJobCreate {
                tenant_id: TenantId(tenant_id),
                job_type: "ORDER_EXPORT".to_string(),
                params_json: serde_json::json!({}),
            })
            .await?;

        let start_at = created_at - ChronoDuration::hours(1);
        let end_at = created_at + ChronoDuration::hours(1);

        let page_one = fetch_orders(db.pool(), &job, start_at, end_at, None, None, 2, 0).await?;
        let page_two = fetch_orders(db.pool(), &job, start_at, end_at, None, None, 2, 2).await?;

        let ordered_ids: Vec<uuid::Uuid> = page_one
            .into_iter()
            .chain(page_two.into_iter())
            .map(|row| row.order_id)
            .collect();

        let expected = vec![
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
        ];
        assert_eq!(ordered_ids, expected);

        Ok(())
    }

    #[test]
    fn product_csv_cells_neutralize_spreadsheet_formulas() {
        for dangerous in [
            "=1+1",
            "+SUM(A1:A2)",
            "-2+3",
            "@cmd",
            "  =HYPERLINK(\"https://example.invalid\")",
            "\tformula",
            "\rformula",
            "\nformula",
        ] {
            assert_eq!(csv_safe_cell(dangerous), format!("'{dangerous}"));
        }
        assert_eq!(csv_safe_cell("ordinary product"), "ordinary product");
        assert_eq!(csv_safe_cell("12345"), "12345");
        assert_eq!(csv_safe_cell("0012345"), "'0012345");
        assert_eq!(csv_safe_cell("123456789012345"), "'123456789012345");
        for ordinary in ["product", "123", "", "  product"] {
            assert_eq!(csv_safe_cell(ordinary), ordinary);
        }
    }

    #[test]
    fn product_csv_formula_protection_survives_csv_round_trip() -> anyhow::Result<()> {
        let mut writer = WriterBuilder::new().from_writer(Vec::new());
        writer.write_record([
            csv_safe_cell("=HYPERLINK(\"https://example.invalid\")"),
            csv_safe_cell("normal, quoted"),
        ])?;
        let bytes = writer.into_inner()?;
        let record = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(bytes.as_slice())
            .records()
            .next()
            .transpose()?
            .expect("one CSV record");
        assert_eq!(
            record.get(0),
            Some("'=HYPERLINK(\"https://example.invalid\")")
        );
        assert_eq!(record.get(1), Some("normal, quoted"));
        Ok(())
    }

    #[test]
    fn product_byte_limit_writer_rejects_record_without_writing_past_limit() {
        let mut writer = ByteLimitWriter::new(Vec::new(), 4);
        writer.write_all(b"1234").expect("within limit");
        let err = writer.write_all(b"5").expect_err("over limit");
        assert_eq!(err.kind(), std::io::ErrorKind::Other);
        assert_eq!(writer.into_inner(), b"1234");
    }
}
