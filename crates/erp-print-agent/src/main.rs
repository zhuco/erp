use clap::Parser;
use erp_common::telemetry::init_tracing;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        long,
        env = "PRINT_AGENT_API_BASE",
        default_value = "http://localhost:8000"
    )]
    api_base: String,
    #[arg(long, env = "PRINT_AGENT_DEVICE_TOKEN")]
    device_token: String,
    #[arg(long, env = "PRINT_AGENT_POLL_INTERVAL_MS", default_value_t = 2000)]
    poll_interval_ms: u64,
    #[arg(long, env = "PRINT_AGENT_BATCH_SIZE", default_value_t = 5)]
    batch_size: i64,
}

#[derive(Debug, Deserialize)]
struct PrintAgentJobListResponse {
    items: Vec<PrintAgentJob>,
}

#[derive(Debug, Deserialize)]
struct PrintAgentJob {
    job_id: String,
    tenant_id: String,
    job_type: Option<String>,
    ref_type: Option<String>,
    ref_id: Option<String>,
    template_id: String,
    template_version: i32,
    content_format: String,
    template_content: String,
    payload: Value,
    attempts: i32,
}

#[derive(Debug, Serialize)]
struct PrintAgentJobCompleteRequest {
    status: String,
    output_content: Option<String>,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    init_tracing("erp-print-agent");
    let args = Args::parse();
    let client = Client::new();

    loop {
        match fetch_jobs(&client, &args).await {
            Ok(jobs) => {
                if jobs.is_empty() {
                    sleep(Duration::from_millis(args.poll_interval_ms)).await;
                    continue;
                }
                for job in jobs {
                    if let Err(err) = process_job(&client, &args, job).await {
                        error!(error = %err, "print job processing failed");
                    }
                }
            }
            Err(err) => {
                warn!(error = %err, "print agent poll failed");
                sleep(Duration::from_millis(args.poll_interval_ms)).await;
            }
        }
    }
}

async fn fetch_jobs(client: &Client, args: &Args) -> anyhow::Result<Vec<PrintAgentJob>> {
    let url = format!(
        "{}/v1/print-agent/jobs?limit={}",
        args.api_base.trim_end_matches('/'),
        args.batch_size
    );
    let response = client
        .get(url)
        .header("x-device-token", &args.device_token)
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("fetch jobs failed: {status} {text}"));
    }
    let payload: PrintAgentJobListResponse = response.json().await?;
    Ok(payload.items)
}

async fn process_job(client: &Client, args: &Args, job: PrintAgentJob) -> anyhow::Result<()> {
    info!(
        job_id = %job.job_id,
        tenant_id = %job.tenant_id,
        job_type = ?job.job_type,
        ref_type = ?job.ref_type,
        ref_id = ?job.ref_id,
        template_id = %job.template_id,
        template_version = job.template_version,
        attempts = job.attempts,
        "rendering print job"
    );

    let render_result = render_job(&job);
    match render_result {
        Ok(output) => {
            complete_job(
                client,
                args,
                &job.job_id,
                PrintAgentJobCompleteRequest {
                    status: "DONE".to_string(),
                    output_content: Some(output),
                    error: None,
                },
            )
            .await?;
        }
        Err(err) => {
            error!(job_id = %job.job_id, error = %err, "render failed");
            complete_job(
                client,
                args,
                &job.job_id,
                PrintAgentJobCompleteRequest {
                    status: "FAILED".to_string(),
                    output_content: None,
                    error: Some(err.to_string()),
                },
            )
            .await?;
        }
    }
    Ok(())
}

fn render_job(job: &PrintAgentJob) -> anyhow::Result<String> {
    let payload_json =
        serde_json::to_string_pretty(&job.payload).unwrap_or_else(|_| job.payload.to_string());
    let body = job
        .template_content
        .replace("{{payload}}", payload_json.as_str());
    let format = job.content_format.trim().to_uppercase();
    let rendered = match format.as_str() {
        "HTML" => {
            let lower = body.to_lowercase();
            if lower.contains("<html") || lower.contains("<!doctype") {
                body
            } else {
                format!(
                    "<!doctype html><html><head><meta charset=\"utf-8\"/></head><body>{body}</body></html>"
                )
            }
        }
        "ZPL" | "TEXT" => body,
        _ => {
            return Err(anyhow::anyhow!(
                "unsupported template format: {}",
                job.content_format
            ));
        }
    };
    Ok(rendered)
}

async fn complete_job(
    client: &Client,
    args: &Args,
    job_id: &str,
    payload: PrintAgentJobCompleteRequest,
) -> anyhow::Result<()> {
    let url = format!(
        "{}/v1/print-agent/jobs/{}/complete",
        args.api_base.trim_end_matches('/'),
        job_id
    );
    let response = client
        .post(url)
        .header("x-device-token", &args.device_token)
        .json(&payload)
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("complete job failed: {status} {text}"));
    }
    Ok(())
}
