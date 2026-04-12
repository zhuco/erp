use secrecy::SecretString;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub database_url: String,
    pub redis_url: String,
    pub nats_url: String,
    pub http_port_api: u16,
    pub http_port_channel: u16,
    pub jwt_secret: SecretString,

    pub dispatch_batch_size: usize,
    pub dispatch_poll_interval_ms: u64,
    pub dispatch_max_tries: i32,
    pub dispatch_timeout_secs: u64,

    pub worker_subject: String,
    pub worker_concurrency: usize,
    pub token_refresh_interval_secs: u64,
    pub token_refresh_threshold_secs: u64,
    pub token_refresh_batch_limit: i64,
    pub sync_job_interval_secs: u64,
    pub sync_job_lookback_minutes: i64,
    pub sync_job_batch_limit: i64,
    pub stock_sync_job_interval_secs: u64,
    pub stock_sync_job_batch_limit: i64,
    pub stock_sync_debounce_ms: i64,
    pub wave_run_interval_secs: u64,
    pub worker_retry_max_attempts: u32,
    pub worker_retry_base_delay_ms: u64,
    pub worker_retry_max_delay_ms: u64,
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        // Prefer explicit env parsing for predictability in production.
        // (config crate is still included for future extension)
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://erp:erp@localhost:5432/erp".to_string());
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let nats_url =
            std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());

        let http_port_api: u16 = std::env::var("HTTP_PORT_API")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()?;
        let http_port_channel: u16 = std::env::var("HTTP_PORT_CHANNEL")
            .unwrap_or_else(|_| "8081".to_string())
            .parse()?;

        let jwt_secret = SecretString::new(
            std::env::var("JWT_SECRET").unwrap_or_else(|_| "CHANGE_ME".to_string()),
        );

        let dispatch_batch_size: usize = std::env::var("DISPATCH_BATCH_SIZE")
            .unwrap_or_else(|_| "200".to_string())
            .parse()?;
        let dispatch_poll_interval_ms: u64 = std::env::var("DISPATCH_POLL_INTERVAL_MS")
            .unwrap_or_else(|_| "200".to_string())
            .parse()?;
        let dispatch_max_tries: i32 = std::env::var("DISPATCH_MAX_TRIES")
            .unwrap_or_else(|_| "20".to_string())
            .parse()?;
        let dispatch_timeout_secs: u64 = std::env::var("DISPATCH_TIMEOUT_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()?;

        let worker_subject =
            std::env::var("WORKER_SUBJECT").unwrap_or_else(|_| "erp.outbox.>".to_string());
        let worker_concurrency: usize = std::env::var("WORKER_CONCURRENCY")
            .unwrap_or_else(|_| "64".to_string())
            .parse()?;
        let token_refresh_interval_secs: u64 = std::env::var("TOKEN_REFRESH_INTERVAL_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()?;
        let token_refresh_threshold_secs: u64 = std::env::var("TOKEN_REFRESH_THRESHOLD_SECS")
            .unwrap_or_else(|_| "1800".to_string())
            .parse()?;
        let token_refresh_batch_limit: i64 = std::env::var("TOKEN_REFRESH_BATCH_LIMIT")
            .unwrap_or_else(|_| "200".to_string())
            .parse()?;
        let sync_job_interval_secs: u64 = std::env::var("SYNC_JOB_INTERVAL_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()?;
        let sync_job_lookback_minutes: i64 = std::env::var("SYNC_JOB_LOOKBACK_MINUTES")
            .unwrap_or_else(|_| "10".to_string())
            .parse()?;
        let sync_job_batch_limit: i64 = std::env::var("SYNC_JOB_BATCH_LIMIT")
            .unwrap_or_else(|_| "200".to_string())
            .parse()?;
        let stock_sync_job_interval_secs: u64 = std::env::var("STOCK_SYNC_JOB_INTERVAL_SECS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()?;
        let stock_sync_job_batch_limit: i64 = std::env::var("STOCK_SYNC_JOB_BATCH_LIMIT")
            .unwrap_or_else(|_| "200".to_string())
            .parse()?;
        let stock_sync_debounce_ms: i64 = std::env::var("STOCK_SYNC_DEBOUNCE_MS")
            .unwrap_or_else(|_| "2000".to_string())
            .parse()?;
        let wave_run_interval_secs: u64 = std::env::var("WAVE_RUN_INTERVAL_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()?;
        let worker_retry_max_attempts: u32 = std::env::var("WORKER_RETRY_MAX_ATTEMPTS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()?;
        let worker_retry_base_delay_ms: u64 = std::env::var("WORKER_RETRY_BASE_DELAY_MS")
            .unwrap_or_else(|_| "500".to_string())
            .parse()?;
        let worker_retry_max_delay_ms: u64 = std::env::var("WORKER_RETRY_MAX_DELAY_MS")
            .unwrap_or_else(|_| "30000".to_string())
            .parse()?;

        Ok(Self {
            database_url,
            redis_url,
            nats_url,
            http_port_api,
            http_port_channel,
            jwt_secret,
            dispatch_batch_size,
            dispatch_poll_interval_ms,
            dispatch_max_tries,
            dispatch_timeout_secs,
            worker_subject,
            worker_concurrency,
            token_refresh_interval_secs,
            token_refresh_threshold_secs,
            token_refresh_batch_limit,
            sync_job_interval_secs,
            sync_job_lookback_minutes,
            sync_job_batch_limit,
            stock_sync_job_interval_secs,
            stock_sync_job_batch_limit,
            stock_sync_debounce_ms,
            wave_run_interval_secs,
            worker_retry_max_attempts,
            worker_retry_base_delay_ms,
            worker_retry_max_delay_ms,
        })
    }
}
