use clap::Parser;
use erp_common::{settings::Settings, telemetry::init_tracing};
use erp_outbox::{dispatcher::OutboxDispatcher, publisher::NatsPublisher};
use erp_repo::db::Db;

#[derive(Parser, Debug)]
struct Args {
    /// Optional override NATS url (otherwise from env)
    #[arg(long)]
    nats_url: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing("erp-dispatcher");
    let _args = Args::parse();
    let settings = Settings::load()?;

    let db = Db::connect(&settings.database_url).await?;
    let nats = async_nats::connect(settings.nats_url.clone()).await?;
    let publisher = NatsPublisher::new(nats);

    let dispatcher = OutboxDispatcher {
        publisher,
        pool: db.pool().clone(),
        batch_size: settings.dispatch_batch_size as i64,
        poll_interval_ms: settings.dispatch_poll_interval_ms,
        max_tries: settings.dispatch_max_tries,
        dispatch_timeout: std::time::Duration::from_secs(settings.dispatch_timeout_secs),
    };

    dispatcher.run_forever().await
}
