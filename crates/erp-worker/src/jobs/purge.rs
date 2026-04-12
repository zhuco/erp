use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::path::PathBuf;
use tokio::fs;

const DEFAULT_OUTBOX_RETENTION_DAYS: i64 = 30;
const DEFAULT_INBOX_RETENTION_DAYS: i64 = 30;
const DEFAULT_PRINT_JOB_RETENTION_DAYS: i64 = 90;
const DEFAULT_EXPORT_JOB_RETENTION_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct PurgeConfig {
    pub outbox_retention_days: i64,
    pub inbox_retention_days: i64,
    pub print_jobs_retention_days: i64,
    pub export_jobs_retention_days: i64,
}

impl PurgeConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            outbox_retention_days: parse_days(
                "OUTBOX_RETENTION_DAYS",
                DEFAULT_OUTBOX_RETENTION_DAYS,
            )?,
            inbox_retention_days: parse_days("INBOX_RETENTION_DAYS", DEFAULT_INBOX_RETENTION_DAYS)?,
            print_jobs_retention_days: parse_days(
                "PRINT_JOBS_RETENTION_DAYS",
                DEFAULT_PRINT_JOB_RETENTION_DAYS,
            )?,
            export_jobs_retention_days: parse_days(
                "EXPORT_JOBS_RETENTION_DAYS",
                DEFAULT_EXPORT_JOB_RETENTION_DAYS,
            )?,
        })
    }
}

#[derive(Debug, Default)]
pub struct PurgeSummary {
    pub outbox_deleted: u64,
    pub inbox_events_deleted: u64,
    pub inbox_payloads_deleted: u64,
    pub print_jobs_deleted: u64,
    pub export_jobs_deleted: u64,
    pub export_files_deleted: u64,
}

pub async fn run(pool: &PgPool, config: &PurgeConfig) -> anyhow::Result<PurgeSummary> {
    let now = Utc::now();
    let mut summary = PurgeSummary::default();

    if config.outbox_retention_days > 0 {
        let cutoff = now - Duration::days(config.outbox_retention_days);
        let result = sqlx::query(
            r#"
            delete from outbox_events
            where status in ('SENT','DEAD')
              and updated_at < $1
            "#,
        )
        .bind(cutoff)
        .execute(pool)
        .await?;
        summary.outbox_deleted = result.rows_affected();
    }

    if config.inbox_retention_days > 0 {
        let cutoff = now - Duration::days(config.inbox_retention_days);
        let mut tx = pool.begin().await?;
        let events = sqlx::query(
            r#"
            delete from inbox_events
            where status in ('PROCESSED','FAILED')
              and coalesce(processed_at, updated_at, received_at) < $1
            "#,
        )
        .bind(cutoff)
        .execute(&mut *tx)
        .await?;
        summary.inbox_events_deleted = events.rows_affected();

        let payloads = sqlx::query(
            r#"
            delete from inbox_raw_payloads p
            where p.received_at < $1
              and not exists (
                select 1 from inbox_events e where e.raw_id = p.raw_id
              )
            "#,
        )
        .bind(cutoff)
        .execute(&mut *tx)
        .await?;
        summary.inbox_payloads_deleted = payloads.rows_affected();
        tx.commit().await?;
    }

    if config.print_jobs_retention_days > 0 {
        let cutoff = now - Duration::days(config.print_jobs_retention_days);
        let result = sqlx::query(
            r#"
            delete from print_jobs
            where status in ('SUCCESS','FAILED')
              and created_at < $1
            "#,
        )
        .bind(cutoff)
        .execute(pool)
        .await?;
        summary.print_jobs_deleted = result.rows_affected();
    }

    if config.export_jobs_retention_days > 0 {
        let cutoff = now - Duration::days(config.export_jobs_retention_days);
        #[derive(sqlx::FromRow)]
        struct ExportRow {
            job_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
        }

        let rows: Vec<ExportRow> = sqlx::query_as(
            r#"
            select job_id, tenant_id
            from export_jobs
            where status in ('DONE','FAILED')
              and created_at < $1
            "#,
        )
        .bind(cutoff)
        .fetch_all(pool)
        .await?;

        for row in &rows {
            let file_path = export_job_file_path(row.tenant_id, row.job_id);
            if fs::remove_file(file_path).await.is_ok() {
                summary.export_files_deleted += 1;
            }
        }

        let result = sqlx::query(
            r#"
            delete from export_jobs
            where status in ('DONE','FAILED')
              and created_at < $1
            "#,
        )
        .bind(cutoff)
        .execute(pool)
        .await?;
        summary.export_jobs_deleted = result.rows_affected();
    }

    Ok(summary)
}

fn parse_days(key: &str, default: i64) -> anyhow::Result<i64> {
    let raw = std::env::var(key).unwrap_or_else(|_| default.to_string());
    let value: i64 = raw
        .parse()
        .map_err(|_| anyhow::anyhow!("{key} must be an integer"))?;
    if value < 0 {
        return Err(anyhow::anyhow!("{key} must be >= 0"));
    }
    Ok(value)
}

fn export_job_file_path(tenant_id: uuid::Uuid, job_id: uuid::Uuid) -> PathBuf {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, NaiveDate};
    use sqlx::{postgres::PgPoolOptions, PgPool};
    use std::time::Duration as StdDuration;

    fn month_start(dt: chrono::DateTime<Utc>) -> NaiveDate {
        let d = dt.date_naive();
        NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
    }

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
            create table if not exists shops (
              shop_id uuid primary key,
              tenant_id uuid not null,
              platform text not null,
              platform_shop_id text not null,
              name text not null default '',
              shop_name text not null default '',
              status text not null default 'AUTHORIZED',
              default_carrier_code text,
              credentials jsonb not null default '{}'::jsonb,
              auth_meta_json jsonb not null default '{}'::jsonb,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists inbox_raw_payloads (
              raw_id bigserial primary key,
              tenant_id uuid not null,
              platform text not null,
              shop_id uuid not null,
              headers jsonb not null default '{}'::jsonb,
              body bytea not null,
              received_at timestamptz not null default now()
            )
            "#,
            r#"
            create table if not exists inbox_events (
              tenant_id uuid not null,
              event_month date not null,
              inbox_event_id uuid not null,
              platform text not null,
              shop_id uuid not null,
              dedup_key text not null,
              raw_id bigint not null,
              status text not null,
              error text not null default '',
              received_at timestamptz not null default now(),
              processed_at timestamptz,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, event_month, inbox_event_id)
            )
            "#,
            r#"
            create table if not exists outbox_events (
              tenant_id uuid not null,
              event_month date not null,
              outbox_id uuid not null,
              event_type text not null,
              payload jsonb not null,
              trace_id text,
              dedup_key text,
              status text not null,
              attempts int not null default 0,
              next_retry_at timestamptz not null default now(),
              last_error text not null default '',
              last_attempt_at timestamptz not null default now(),
              published_at timestamptz,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              primary key (tenant_id, event_month, outbox_id)
            )
            "#,
            r#"
            create table if not exists print_jobs (
              job_id uuid primary key,
              tenant_id uuid not null,
              shop_id uuid not null,
              type text not null,
              ref_type text not null,
              ref_id uuid not null,
              payload jsonb not null default '{}'::jsonb,
              status text not null,
              target_printer_name text,
              device_fingerprint text,
              created_at timestamptz not null default now(),
              updated_at timestamptz not null default now(),
              printed_at timestamptz
            )
            "#,
        ];

        for stmt in statements {
            sqlx::query(stmt).execute(pool).await?;
        }
        Ok(())
    }

    async fn try_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
        PgPoolOptions::new()
            .acquire_timeout(StdDuration::from_secs(2))
            .connect(&url)
            .await
            .ok()
    }

    #[tokio::test]
    async fn purge_removes_expired_rows() -> anyhow::Result<()> {
        let Some(pool) = try_pool().await else {
            eprintln!("skipping purge_removes_expired_rows: db unavailable");
            return Ok(());
        };
        setup_schema(&pool).await?;

        let tenant_id = uuid::Uuid::new_v4();
        let shop_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            insert into tenants (tenant_id, name, status)
            values ($1, $2, 'ACTIVE')
            "#,
        )
        .bind(tenant_id)
        .bind("test-tenant")
        .execute(&pool)
        .await?;
        sqlx::query(
            r#"
            insert into shops (shop_id, tenant_id, platform, platform_shop_id, name, shop_name, status)
            values ($1, $2, $3, $4, $5, $5, 'AUTHORIZED')
            "#,
        )
        .bind(shop_id)
        .bind(tenant_id)
        .bind("taobao")
        .bind("SHOP-1")
        .bind("shop")
        .execute(&pool)
        .await?;

        let old_ts = Utc::now() - Duration::days(45);
        let fresh_ts = Utc::now() - Duration::days(5);

        let old_raw_id: i64 = sqlx::query_scalar(
            r#"
            insert into inbox_raw_payloads(tenant_id, platform, shop_id, headers, body, received_at)
            values ($1, $2, $3, $4, $5, $6)
            returning raw_id
            "#,
        )
        .bind(tenant_id)
        .bind("taobao")
        .bind(shop_id)
        .bind(serde_json::json!({}))
        .bind(Vec::<u8>::new())
        .bind(old_ts)
        .fetch_one(&pool)
        .await?;
        let old_failed_raw_id: i64 = sqlx::query_scalar(
            r#"
            insert into inbox_raw_payloads(tenant_id, platform, shop_id, headers, body, received_at)
            values ($1, $2, $3, $4, $5, $6)
            returning raw_id
            "#,
        )
        .bind(tenant_id)
        .bind("taobao")
        .bind(shop_id)
        .bind(serde_json::json!({}))
        .bind(Vec::<u8>::new())
        .bind(old_ts)
        .fetch_one(&pool)
        .await?;
        let fresh_raw_id: i64 = sqlx::query_scalar(
            r#"
            insert into inbox_raw_payloads(tenant_id, platform, shop_id, headers, body, received_at)
            values ($1, $2, $3, $4, $5, $6)
            returning raw_id
            "#,
        )
        .bind(tenant_id)
        .bind("taobao")
        .bind(shop_id)
        .bind(serde_json::json!({}))
        .bind(Vec::<u8>::new())
        .bind(fresh_ts)
        .fetch_one(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into inbox_events(
              tenant_id, event_month, inbox_event_id, platform, shop_id, dedup_key, raw_id,
              status, error, received_at, processed_at, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(tenant_id)
        .bind(month_start(old_ts))
        .bind(uuid::Uuid::new_v4())
        .bind("taobao")
        .bind(shop_id)
        .bind("dedup-old")
        .bind(old_raw_id)
        .bind("PROCESSED")
        .bind("")
        .bind(old_ts)
        .bind(old_ts)
        .bind(old_ts)
        .bind(old_ts)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into inbox_events(
              tenant_id, event_month, inbox_event_id, platform, shop_id, dedup_key, raw_id,
              status, error, received_at, processed_at, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(tenant_id)
        .bind(month_start(old_ts))
        .bind(uuid::Uuid::new_v4())
        .bind("taobao")
        .bind(shop_id)
        .bind("dedup-failed")
        .bind(old_failed_raw_id)
        .bind("FAILED")
        .bind("error")
        .bind(old_ts)
        .bind::<Option<chrono::DateTime<Utc>>>(None)
        .bind(old_ts)
        .bind(old_ts)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into inbox_events(
              tenant_id, event_month, inbox_event_id, platform, shop_id, dedup_key, raw_id,
              status, error, received_at, processed_at, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(tenant_id)
        .bind(month_start(fresh_ts))
        .bind(uuid::Uuid::new_v4())
        .bind("taobao")
        .bind(shop_id)
        .bind("dedup-fresh")
        .bind(fresh_raw_id)
        .bind("PROCESSED")
        .bind("")
        .bind(fresh_ts)
        .bind(fresh_ts)
        .bind(fresh_ts)
        .bind(fresh_ts)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into outbox_events(
              tenant_id, event_month, outbox_id, event_type, payload, status, attempts,
              next_retry_at, last_error, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(tenant_id)
        .bind(month_start(old_ts))
        .bind(uuid::Uuid::new_v4())
        .bind("TEST")
        .bind(serde_json::json!({}))
        .bind("SENT")
        .bind(1_i32)
        .bind(old_ts)
        .bind("")
        .bind(old_ts)
        .bind(old_ts)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into outbox_events(
              tenant_id, event_month, outbox_id, event_type, payload, status, attempts,
              next_retry_at, last_error, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(tenant_id)
        .bind(month_start(old_ts))
        .bind(uuid::Uuid::new_v4())
        .bind("TEST")
        .bind(serde_json::json!({}))
        .bind("PENDING")
        .bind(1_i32)
        .bind(old_ts)
        .bind("")
        .bind(old_ts)
        .bind(old_ts)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into outbox_events(
              tenant_id, event_month, outbox_id, event_type, payload, status, attempts,
              next_retry_at, last_error, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(tenant_id)
        .bind(month_start(fresh_ts))
        .bind(uuid::Uuid::new_v4())
        .bind("TEST")
        .bind(serde_json::json!({}))
        .bind("DEAD")
        .bind(1_i32)
        .bind(fresh_ts)
        .bind("")
        .bind(fresh_ts)
        .bind(fresh_ts)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into print_jobs(
              job_id, tenant_id, shop_id, type, ref_type, ref_id, payload, status, target_printer_name,
              created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(uuid::Uuid::new_v4())
        .bind(tenant_id)
        .bind(shop_id)
        .bind("LABEL")
        .bind("ORDER")
        .bind(uuid::Uuid::new_v4())
        .bind(serde_json::json!({}))
        .bind("SUCCESS")
        .bind(None::<String>)
        .bind(old_ts)
        .bind(old_ts)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into print_jobs(
              job_id, tenant_id, shop_id, type, ref_type, ref_id, payload, status, target_printer_name,
              created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(uuid::Uuid::new_v4())
        .bind(tenant_id)
        .bind(shop_id)
        .bind("LABEL")
        .bind("ORDER")
        .bind(uuid::Uuid::new_v4())
        .bind(serde_json::json!({}))
        .bind("FAILED")
        .bind(None::<String>)
        .bind(fresh_ts)
        .bind(fresh_ts)
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            insert into print_jobs(
              job_id, tenant_id, shop_id, type, ref_type, ref_id, payload, status, target_printer_name,
              created_at, updated_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(uuid::Uuid::new_v4())
        .bind(tenant_id)
        .bind(shop_id)
        .bind("LABEL")
        .bind("ORDER")
        .bind(uuid::Uuid::new_v4())
        .bind(serde_json::json!({}))
        .bind("QUEUED")
        .bind(None::<String>)
        .bind(old_ts)
        .bind(old_ts)
        .execute(&pool)
        .await?;

        let config = PurgeConfig {
            outbox_retention_days: 30,
            inbox_retention_days: 30,
            print_jobs_retention_days: 30,
            export_jobs_retention_days: 30,
        };
        let summary = run(&pool, &config).await?;

        assert_eq!(summary.outbox_deleted, 1);
        assert_eq!(summary.inbox_events_deleted, 2);
        assert_eq!(summary.inbox_payloads_deleted, 2);
        assert_eq!(summary.print_jobs_deleted, 1);
        assert_eq!(summary.export_jobs_deleted, 0);
        assert_eq!(summary.export_files_deleted, 0);

        let inbox_left: i64 = sqlx::query_scalar(
            r#"
            select count(*) from inbox_events where tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(inbox_left, 1);

        let raw_left: i64 = sqlx::query_scalar(
            r#"
            select count(*) from inbox_raw_payloads where tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(raw_left, 1);

        let outbox_left: i64 = sqlx::query_scalar(
            r#"
            select count(*) from outbox_events where tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(outbox_left, 2);

        let print_left: i64 = sqlx::query_scalar(
            r#"
            select count(*) from print_jobs where tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(print_left, 2);

        sqlx::query("delete from inbox_events where tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await?;
        sqlx::query("delete from inbox_raw_payloads where tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await?;
        sqlx::query("delete from outbox_events where tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await?;
        sqlx::query("delete from print_jobs where tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await?;
        sqlx::query("delete from shops where tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await?;
        sqlx::query("delete from tenants where tenant_id = $1")
            .bind(tenant_id)
            .execute(&pool)
            .await?;

        Ok(())
    }
}
