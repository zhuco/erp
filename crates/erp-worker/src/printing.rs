use super::*;

pub(super) async fn handle_print_job_queued(db: &Db, env: OutboxEnvelope) -> anyhow::Result<()> {
    let job_id = env
        .payload
        .get("job_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing job_id"))?;
    let job_id = uuid::Uuid::parse_str(job_id)?;

    let jobs_repo = PrintJobsRepo::new(db.pool());
    let templates_repo = PrintTemplatesRepo::new(db.pool());
    let audit_repo = AuditLogsRepo::new(db.pool());

    let Some(job) = jobs_repo.get_job(env.tenant_id, job_id).await? else {
        return Ok(());
    };

    if job.status != "PENDING" {
        return Ok(());
    }
    if job.device_fingerprint.is_some() {
        return Ok(());
    }

    let attempts = job.attempts.saturating_add(1);
    let render_result: anyhow::Result<(String, String)> = async {
        let template_id = job
            .template_id
            .ok_or_else(|| anyhow::anyhow!("template_id missing"))?;
        let template = templates_repo
            .get_template(env.tenant_id, template_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("template not found"))?;

        let template_body = match template.content {
            serde_json::Value::String(value) => value,
            other => other.to_string(),
        };
        let payload_json =
            serde_json::to_string_pretty(&job.payload).unwrap_or_else(|_| job.payload.to_string());
        let body = template_body.replace("{{payload}}", &payload_json);
        let format = template.content_format.trim().to_uppercase();
        let rendered = match format.as_str() {
            "HTML" => {
                let lower = body.to_lowercase();
                if lower.contains("<html") || lower.contains("<!doctype") {
                    body.clone()
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
                    template.content_format
                ))
            }
        };
        let output_url = format!("/v1/tenants/{}/print-jobs/{}/output", env.tenant_id, job_id);
        Ok((rendered, output_url))
    }
    .await;

    match render_result {
        Ok((html, output_url)) => {
            let updated = jobs_repo
                .update_job(
                    env.tenant_id,
                    job_id,
                    erp_repo::print_jobs::PrintJobUpdate {
                        status: Some("DONE".to_string()),
                        payload: None,
                        output_url: Some(output_url),
                        output_content: Some(html),
                        attempts: Some(attempts),
                        last_error: Some(String::new()),
                        actor_user_id: None,
                    },
                )
                .await?
                .ok_or_else(|| anyhow::anyhow!("print job not found"))?;

            let after_json = serde_json::to_value(&updated).unwrap_or(serde_json::json!({}));
            audit_repo
                .insert(AuditLogCreate {
                    tenant_id: env.tenant_id,
                    user_id: None,
                    action: "PRINT_JOB.RENDER".to_string(),
                    entity_type: "print_job".to_string(),
                    entity_id: Some(job_id.to_string()),
                    before_json: None,
                    after_json: Some(after_json),
                    ip: None,
                    ua: None,
                })
                .await?;

            metrics().inc_print_jobs_success();
        }
        Err(err) => {
            let message = err.to_string();
            let _ = jobs_repo
                .update_job(
                    env.tenant_id,
                    job_id,
                    erp_repo::print_jobs::PrintJobUpdate {
                        status: Some("FAILED".to_string()),
                        payload: None,
                        output_url: None,
                        output_content: None,
                        attempts: Some(attempts),
                        last_error: Some(message.clone()),
                        actor_user_id: None,
                    },
                )
                .await;

            let _ = audit_repo
                .insert(AuditLogCreate {
                    tenant_id: env.tenant_id,
                    user_id: None,
                    action: "PRINT_JOB.FAIL".to_string(),
                    entity_type: "print_job".to_string(),
                    entity_id: Some(job_id.to_string()),
                    before_json: None,
                    after_json: Some(serde_json::json!({"error": message})),
                    ip: None,
                    ua: None,
                })
                .await;
            metrics().inc_print_jobs_failed();
        }
    }

    Ok(())
}
