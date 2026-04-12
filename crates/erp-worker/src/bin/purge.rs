use clap::Parser;
use erp_common::telemetry::init_tracing;
use erp_repo::db::Db;
use erp_worker::jobs::purge::{self, PurgeConfig};
use tracing::info;

#[derive(Parser, Debug)]
struct Args {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing("erp-purge");
    let _args = Args::parse();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://erp:erp@localhost:5432/erp".to_string());
    let db = Db::connect(&database_url).await?;
    let config = PurgeConfig::from_env()?;
    let summary = purge::run(db.pool(), &config).await?;

    info!(
        outbox_deleted = summary.outbox_deleted,
        inbox_events_deleted = summary.inbox_events_deleted,
        inbox_payloads_deleted = summary.inbox_payloads_deleted,
        print_jobs_deleted = summary.print_jobs_deleted,
        export_jobs_deleted = summary.export_jobs_deleted,
        export_files_deleted = summary.export_files_deleted,
        "purge completed"
    );

    Ok(())
}
