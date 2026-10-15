//! Inventory export: exact snapshot, bounded CSV and durable publication receipt.
use erp_repo::{
    export_jobs::{ExportJob, ExportJobsRepo},
    inventory_export_cursor::InventoryExportCursor,
    inventory_export_jobs::{
        InventoryExportArtifact, InventoryExportSpec, MAX_EXPORT_BYTES, authorize_job, parse_spec,
    },
};
use erp_types::UserId;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

fn file_path(job: &ExportJob) -> PathBuf {
    let base = std::env::var("EXPORT_BASE_DIR").unwrap_or_else(|_| "exports".into());
    PathBuf::from(base)
        .join(job.tenant_id.to_string())
        .join(format!("{}.csv", job.job_id))
}

async fn regular_file(path: &Path) -> anyhow::Result<Option<u64>> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.is_file() && !is_link(&metadata) => Ok(Some(metadata.len())),
        Ok(_) => anyhow::bail!("inventory export path is not a regular file"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn is_link(metadata: &std::fs::Metadata) -> bool {
    let linked = metadata.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        linked || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        linked
    }
}

async fn check_directories(parent: &Path) -> anyhow::Result<()> {
    for directory in [
        parent,
        parent
            .parent()
            .ok_or_else(|| anyhow::anyhow!("missing export base"))?,
    ] {
        match tokio::fs::symlink_metadata(directory).await {
            Ok(metadata) if metadata.is_dir() && !is_link(&metadata) => {}
            Ok(_) => {
                anyhow::bail!("inventory export directory must not be a symlink/reparse point")
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

async fn verify_file(path: &Path, artifact: &InventoryExportArtifact) -> anyhow::Result<()> {
    artifact.validate()?;
    if regular_file(path).await? != Some(artifact.bytes) {
        anyhow::bail!("inventory export receipt length mismatch");
    }
    let mut file = tokio::fs::File::open(path).await?;
    let mut hash = Sha256::new();
    let mut bytes = 0u64;
    let mut chunk = vec![0u8; 65536];
    loop {
        let n = file.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        bytes = bytes
            .checked_add(n as u64)
            .ok_or_else(|| anyhow::anyhow!("file size overflow"))?;
        if bytes > artifact.bytes {
            anyhow::bail!("inventory export file grew");
        }
        hash.update(&chunk[..n]);
    }
    if bytes != artifact.bytes || hex::encode(hash.finalize()) != artifact.sha256 {
        anyhow::bail!("inventory export receipt hash mismatch");
    }
    Ok(())
}

async fn publish(
    pool: &PgPool,
    job: &ExportJob,
    spec: &InventoryExportSpec,
    temp: Option<&Path>,
) -> anyhow::Result<()> {
    let artifact = spec
        .artifact
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing prepared receipt"))?;
    let mut tx = pool.begin().await?;
    sqlx::query("set local statement_timeout='10s'")
        .execute(&mut *tx)
        .await?;
    let params: Option<serde_json::Value> = sqlx::query_scalar("select params_json from export_jobs where tenant_id=$1 and job_id=$2 and status='RUNNING' and attempts=$3 for update")
        .bind(Uuid::from(job.tenant_id)).bind(job.job_id).bind(job.attempts).fetch_optional(&mut *tx).await?;
    if params != Some(serde_json::to_value(spec)?) {
        anyhow::bail!("inventory export attempt or receipt changed");
    }
    authorize_job(pool, job, UserId(spec.actor_user_id), true).await?;
    let final_path = file_path(job);
    if let Some(temp) = temp {
        // Same-directory hard link publishes a complete file without overwrite.
        // The receipt is already durable, so a crash here can be verified on retry.
        tokio::fs::hard_link(temp, &final_path).await?;
    }
    if regular_file(&final_path).await? != Some(artifact.bytes) {
        anyhow::bail!("published inventory file length mismatch");
    }
    let changed = sqlx::query("update export_jobs set status='DONE',progress=100,last_error='',output_url=$4,updated_at=now() where tenant_id=$1 and job_id=$2 and status='RUNNING' and attempts=$3")
        .bind(Uuid::from(job.tenant_id)).bind(job.job_id).bind(job.attempts)
        .bind(format!("/exports/jobs/{}/file",job.job_id)).execute(&mut *tx).await?.rows_affected();
    if changed != 1 {
        anyhow::bail!("inventory export completion fence failed");
    }
    tx.commit().await?;
    Ok(())
}

async fn generate(
    pool: &PgPool,
    job: &ExportJob,
    page_size: u16,
    temp: &Path,
) -> anyhow::Result<()> {
    let mut spec = authorize_job(pool, job, UserId(parse_spec(job)?.actor_user_id), true).await?;
    let final_path = file_path(job);
    let parent = final_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("missing export directory"))?;
    check_directories(parent).await?;
    if regular_file(&final_path).await?.is_some() {
        let artifact = spec
            .artifact
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("published inventory file has no durable receipt"))?;
        verify_file(&final_path, artifact).await?;
        return publish(pool, job, &spec, None).await;
    }
    let mut cursor = InventoryExportCursor::open(pool, job.tenant_id, &spec).await?;
    tokio::fs::create_dir_all(parent).await?;
    check_directories(parent).await?;
    let total = cursor.total();
    let snapshot = cursor.snapshot_at();
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)
        .await?;
    let mut hash = Sha256::new();
    let mut byte_count = 0u64;
    let mut rows = 0u64;
    let mut first = true;
    loop {
        authorize_job(pool, job, UserId(spec.actor_user_id), true).await?;
        let page = cursor.fetch_next(page_size).await?;
        let done = page.is_empty();
        let mut csv = csv::WriterBuilder::new().from_writer(Vec::new());
        if first {
            csv.write_record([
                "sku_id",
                "sku_code",
                "warehouse_id",
                "on_hand",
                "reserved",
                "available",
                "frozen",
                "in_transit",
                "quantity_scale",
                "snapshot_at",
            ])?;
            first = false;
        }
        for row in page {
            if row.sku_code.as_ref().is_some_and(|code| code.len() > 1024) {
                anyhow::bail!("oversized SKU code");
            }
            let q = row.quantities()?;
            csv.write_record([
                row.sku_id.to_string(),
                super::export_jobs::csv_safe_cell(row.sku_code.as_deref().unwrap_or_default()),
                row.warehouse_id.to_string(),
                q.on_hand,
                q.reserved,
                q.available,
                q.frozen,
                q.in_transit,
                "6".into(),
                snapshot.to_rfc3339(),
            ])?;
            rows += 1;
        }
        let chunk = csv.into_inner()?;
        if chunk.len() > 8 * 1024 * 1024 {
            anyhow::bail!("inventory CSV chunk exceeds 8 MiB");
        }
        byte_count = byte_count
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| anyhow::anyhow!("CSV size overflow"))?;
        if byte_count > MAX_EXPORT_BYTES {
            anyhow::bail!("inventory CSV exceeds 1 GiB");
        }
        file.write_all(&chunk).await?;
        hash.update(&chunk);
        let progress = if total == 0 {
            0
        } else {
            i32::try_from(rows.saturating_mul(100) / total)?.min(99)
        };
        let changed = sqlx::query("update export_jobs set progress=$4,updated_at=now() where tenant_id=$1 and job_id=$2 and status='RUNNING' and attempts=$3")
            .bind(Uuid::from(job.tenant_id)).bind(job.job_id).bind(job.attempts).bind(progress).execute(pool).await?.rows_affected();
        if changed != 1 {
            anyhow::bail!("inventory export attempt lost");
        }
        if done {
            break;
        }
    }
    if rows != total {
        anyhow::bail!("inventory export snapshot row count mismatch");
    }
    cursor.finish().await?;
    file.flush().await?;
    file.sync_all().await?;
    drop(file);
    spec.artifact = Some(InventoryExportArtifact {
        schema: 2,
        quantity_scale: 6,
        snapshot_at: snapshot,
        rows,
        bytes: byte_count,
        sha256: hex::encode(hash.finalize()),
    });
    let changed = sqlx::query("update export_jobs set params_json=$4,updated_at=now() where tenant_id=$1 and job_id=$2 and status='RUNNING' and attempts=$3")
        .bind(Uuid::from(job.tenant_id)).bind(job.job_id).bind(job.attempts).bind(serde_json::to_value(&spec)?).execute(pool).await?.rows_affected();
    if changed != 1 {
        anyhow::bail!("inventory export prepared receipt fence failed");
    }
    publish(pool, job, &spec, Some(temp)).await
}

/// Called by the existing Worker dispatcher; public for registered QA acceptance.
/// RUNNING crash reclamation belongs to the existing scheduler and remains a
/// separate acceptance gate. This executor never claims other tenants' jobs.
pub async fn execute_inventory_job(
    pool: &PgPool,
    claimed: &ExportJob,
    page_size: i64,
) -> anyhow::Result<()> {
    let mut execution = pool.begin().await?;
    sqlx::query("set local statement_timeout='10s'")
        .execute(&mut *execution)
        .await?;
    sqlx::query("set local idle_in_transaction_session_timeout='330s'")
        .execute(&mut *execution)
        .await?;
    let acquired: bool =
        sqlx::query_scalar("select pg_try_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!("inventory-export:{}", claimed.job_id))
            .fetch_one(&mut *execution)
            .await?;
    if !acquired {
        anyhow::bail!("inventory export already has an executor");
    }
    let job = ExportJobsRepo::new(pool)
        .get_job(claimed.tenant_id, claimed.job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("inventory export job missing"))?;
    if job.status != "RUNNING"
        || job.attempts != claimed.attempts
        || job.attempts < 1
        || job.job_type != "INVENTORY_EXPORT"
    {
        anyhow::bail!("inventory export claim is stale");
    }
    let final_path = file_path(&job);
    let temp = final_path.with_file_name(format!(
        "{}.{}.{}.inventory.tmp",
        job.job_id,
        job.attempts,
        Uuid::new_v4().simple()
    ));
    let result = if let Ok(page_size) = u16::try_from(page_size)
        && (1..=1000).contains(&page_size)
    {
        match tokio::time::timeout(
            Duration::from_secs(300),
            generate(pool, &job, page_size, &temp),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!(
                "inventory export timed out after 300 seconds"
            )),
        }
    } else {
        Err(anyhow::anyhow!(
            "inventory export page size must be 1..1000"
        ))
    };
    let cleanup = tokio::fs::remove_file(&temp).await;
    if let Err(error) = cleanup
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(job_id=%job.job_id,error=%error,"inventory export temporary file cleanup failed");
    }
    if let Err(error) = &result {
        sqlx::query("update export_jobs set status='FAILED',progress=0,output_url=null,last_error=$4,updated_at=now() where tenant_id=$1 and job_id=$2 and status='RUNNING' and attempts=$3")
            .bind(Uuid::from(job.tenant_id)).bind(job.job_id).bind(job.attempts).bind(error.to_string()).execute(pool).await?;
    }
    execution.rollback().await?;
    result
}
