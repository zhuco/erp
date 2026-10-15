//! Lost-response reads must never execute caller-described stock or invent completion.
use super::{
    Result,
    inspection_outcome_edges::owned_facts,
    isolated_pool,
    production_receipt_release::{Receipt, http as manufacturing_http, setup},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

async fn before_read(pool: &PgPool, r: &Receipt) -> Result<Value> {
    let value = facts(pool, r).await?;
    eprintln!(
        "QA command_readback_before tenant={} facts={value}",
        r.f.tenant
    );
    Ok(value)
}

async fn finish_read(
    pool: &PgPool,
    r: &Receipt,
    case: &str,
    request: &str,
    before: &Value,
    response: (StatusCode, Value),
    expected: StatusCode,
) -> Result<Value> {
    let after = facts(pool, r).await?;
    let (status, body) = response;
    eprintln!(
        "QA command_readback_result tenant={} case={case} request={request} status={} unchanged={} response_status={}",
        r.f.tenant,
        status.as_u16(),
        before == &after,
        body["status"]
    );
    assert_eq!(status, expected, "{case}: actual HTTP {body}");
    assert_eq!(
        before, &after,
        "{case}: no selected effect or receipt changes"
    );
    Ok(body)
}

async fn context(pool: &PgPool, case: &str) -> Result<Receipt> {
    let r = setup(pool, case).await?;
    sqlx::query("update tenants set name='QA-ERP-COMMAND-READBACK' where tenant_id=$1")
        .bind(r.f.tenant.0)
        .execute(pool)
        .await?;
    eprintln!(
        "QA command_readback_fixture tenant={} case={case} actor={} warehouse={} sku={} work_order={}",
        r.f.tenant, r.f.actor, r.f.warehouse, r.f.sku, r.work_order
    );
    Ok(r)
}

async fn facts(pool: &PgPool, r: &Receipt) -> Result<Value> {
    let mut value = owned_facts(pool, r).await?;
    for table in [
        "command_objects",
        "browser_sessions",
        "atomic_source_documents",
        "atomic_posting_events",
        "mfg_material_moves",
        "mfg_qty_balances",
        "inventory_reservations",
    ] {
        value[table]=sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from {table} x where tenant_id=$1"))
            .bind(r.f.tenant.0).fetch_one(pool).await?;
    }
    Ok(value)
}

async fn http(r: &Receipt, method: &str, path: &str, input: Value) -> Result<(StatusCode, Value)> {
    let jwt = Arc::new(JwtService::new("synthetic-command-readback-key".into()));
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        r.f.actor.to_string(),
        r.f.tenant.to_string(),
        vec![
            "manufacturing.write".into(),
            "manufacturing.read".into(),
            "inventory.read".into(),
            "inventory.write".into(),
        ],
        false,
        now,
        now + 300,
    ))?;
    let database = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let response = build_app(AppState::new(Db::connect_lazy(&database)?, jwt))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(format!("/v1/tenants/{}/{path}", r.f.tenant))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(input.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA command_readback_http tenant={} method={method} path={path} input={input} status={} response={body}",
        r.f.tenant,
        status.as_u16()
    );
    Ok((status, body))
}

async fn opened(pool: &PgPool, r: &Receipt) -> Result<(String, String)> {
    let request = Uuid::new_v4().to_string();
    let (status,body)=http(r,"POST","commands",json!({"request_id":request,"object_type":"WORK_ORDER","object_id":r.work_order.to_string(),"version_no":1})).await?;
    assert_eq!(status, StatusCode::OK, "actual open {body}");
    let hash = sqlx::query_scalar(
        "select payload_hash from command_receipts where tenant_id=$1 and request_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(&request)
    .fetch_one(pool)
    .await?;
    Ok((request, hash))
}

async fn sessions(pool: &PgPool, r: &Receipt) -> Result<(Uuid, Uuid)> {
    let revoked = Uuid::new_v4();
    let current = Uuid::new_v4();
    for (id, status) in [(revoked, "REVOKED"), (current, "ACTIVE")] {
        sqlx::query("insert into browser_sessions(tenant_id,session_id,actor_user_id,mode,http_only,secure,same_site,csrf_token,credential_epoch,status,expires_at) values($1,$2,$3,'BEARER',false,false,'','',0,$4,now()+interval '1 hour')")
            .bind(r.f.tenant.0).bind(id).bind(r.f.actor.to_string()).bind(status).execute(pool).await?;
    }
    Ok((revoked, current))
}

async fn lookup(
    r: &Receipt,
    request: &str,
    hash: &str,
    revoked: Uuid,
    current: Uuid,
    unknown: bool,
) -> Result<(StatusCode, Value)> {
    http(
        r,
        "POST",
        &format!("commands/{request}/lost-lookups"),
        json!({"payload_hash":hash,
        "revoked_session_id":revoked,"current_session_id":current,"client_unknown":unknown,
        "warehouse_id":r.f.warehouse,"sku_id":r.f.sku,"qty_units":3}),
    )
    .await
}

#[tokio::test]
async fn command_readback_accepted_lost_lookup_does_not_post_client_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "accepted-lookup").await?;
    let (request, hash) = opened(&pool, &r).await?;
    let (revoked, current) = sessions(&pool, &r).await?;
    let before = facts(&independent, &r).await?;
    eprintln!(
        "QA command_readback_before tenant={} facts={before}",
        r.f.tenant
    );
    let (status, body) = lookup(&r, &request, &hash, revoked, current, false).await?;
    let after = facts(&independent, &r).await?;
    eprintln!(
        "QA command_readback_result tenant={} case=accepted-lookup request={request} status={} unchanged={} response_status={}",
        r.f.tenant,
        status.as_u16(),
        before == after,
        body["status"]
    );
    assert_eq!(status, StatusCode::OK, "actual lookup {body}");
    assert_eq!(
        body["status"], "ACCEPTED",
        "lookup does not execute a caller-described effect"
    );
    assert_eq!(
        before, after,
        "lookup is read-only across all selected effects"
    );
    Ok(())
}

#[tokio::test]
async fn command_readback_client_success_cannot_invent_a_committed_result() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "client-success").await?;
    let (request, hash) = opened(&pool, &r).await?;
    let before = facts(&independent, &r).await?;
    eprintln!(
        "QA command_readback_before tenant={} facts={before}",
        r.f.tenant
    );
    let (status,body)=http(&r,"POST",&format!("commands/{request}"),json!({"reported_status":"SUCCEEDED","error_text":"","recoverable":false,"payload_hash":hash})).await?;
    let after = facts(&independent, &r).await?;
    eprintln!(
        "QA command_readback_result tenant={} case=client-success request={request} status={} unchanged={} response_status={}",
        r.f.tenant,
        status.as_u16(),
        before == after,
        body["status"]
    );
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a client has no server completion authority {body}"
    );
    assert_eq!(
        before, after,
        "refused client completion has no receipt or business effect"
    );
    Ok(())
}

async fn real_material(pool: &PgPool, r: &Receipt) -> Result<(String, String, String)> {
    let request = Uuid::new_v4().to_string();
    let (status,body)=manufacturing_http(&r.f,&format!("work-orders/{}/materials",r.work_order),json!({
        "request_id":request,"kind":"ISSUE","qty":"0.1","warehouse_id":r.f.warehouse,"sku_id":r.f.sku
    })).await?;
    assert_eq!(status, StatusCode::OK, "real material effect {body}");
    let effect = body["effect_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("real effect missing"))?
        .to_string();
    let hash = sqlx::query_scalar(
        "select payload_hash from command_receipts where tenant_id=$1 and request_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(&request)
    .fetch_one(pool)
    .await?;
    eprintln!(
        "QA command_readback_material tenant={} request={request} effect={effect} qty_micros=100000",
        r.f.tenant
    );
    Ok((request, hash, effect))
}

#[tokio::test]
async fn command_readback_actual_material_command_is_visible_without_fake_open_metadata()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "material-get").await?;
    let (request, _, effect) = real_material(&pool, &r).await?;
    let before = facts(&independent, &r).await?;
    eprintln!(
        "QA command_readback_before tenant={} facts={before}",
        r.f.tenant
    );
    let (status, body) = http(&r, "GET", &format!("commands/{request}"), Value::Null).await?;
    let after = facts(&independent, &r).await?;
    eprintln!(
        "QA command_readback_result tenant={} case=material-get request={request} status={} unchanged={} response_status={}",
        r.f.tenant,
        status.as_u16(),
        before == after,
        body["status"]
    );
    assert_eq!(status, StatusCode::OK, "actual committed command {body}");
    assert_eq!(body["status"], "SUCCEEDED");
    assert_eq!(body["object_id"], r.work_order.to_string());
    assert_eq!(body["effect_id"], effect);
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn command_readback_actual_material_lost_lookup_returns_original_effect_without_stock()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "material-lost").await?;
    let (request, hash, effect) = real_material(&pool, &r).await?;
    let (revoked, current) = sessions(&pool, &r).await?;
    let before = facts(&independent, &r).await?;
    eprintln!(
        "QA command_readback_before tenant={} facts={before}",
        r.f.tenant
    );
    let (status, body) = lookup(&r, &request, &hash, revoked, current, false).await?;
    let after = facts(&independent, &r).await?;
    eprintln!(
        "QA command_readback_result tenant={} case=material-lost request={request} status={} unchanged={} response_status={}",
        r.f.tenant,
        status.as_u16(),
        before == after,
        body["status"]
    );
    assert_eq!(status, StatusCode::OK, "original material lookup {body}");
    assert_eq!(body["status"], "SUCCEEDED");
    assert_eq!(body["effect_id"], effect);
    assert_eq!(body["replay"], true);
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn command_readback_accepted_observations_preserve_server_metadata() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "accepted-observations").await?;
    let (request, hash) = opened(&pool, &r).await?;
    let before = before_read(&independent, &r).await?;
    for reported in ["UNKNOWN", "ACCEPTED"] {
        let response=http(&r,"POST",&format!("commands/{request}"),json!({"payload_hash":hash,
            "reported_status":reported,"error_text":"fabricated client failure","recoverable":false})).await?;
        assert_eq!(response.0, StatusCode::OK);
        assert_eq!(response.1["status"], "ACCEPTED");
        assert_eq!(response.1["replay"], true);
        assert_eq!(response.1["effect_id"], "");
    }
    let response = http(&r, "GET", &format!("commands/{request}"), Value::Null).await?;
    let body = finish_read(
        &independent,
        &r,
        "accepted-observations",
        &request,
        &before,
        response,
        StatusCode::OK,
    )
    .await?;
    assert_eq!(body["status"], "ACCEPTED");
    assert_eq!(body["error_text"], "");
    assert_eq!(body["recoverable"], true);
    Ok(())
}

#[tokio::test]
async fn command_readback_all_client_status_transitions_are_refused() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "client-transitions").await?;
    let (request, hash) = opened(&pool, &r).await?;
    let before = before_read(&independent, &r).await?;
    for reported in ["RUNNING", "SUCCEEDED", "FAILED", "RECONCILING"] {
        let response = http(
            &r,
            "POST",
            &format!("commands/{request}"),
            json!({"payload_hash":hash,
            "reported_status":reported,"error_text":"untrusted client result","recoverable":false}),
        )
        .await?;
        assert_eq!(response.0, StatusCode::CONFLICT);
        assert_eq!(response.1["code"], "command_client_transition_denied");
    }
    let response = http(
        &r,
        "POST",
        &format!("commands/{request}"),
        json!({"payload_hash":hash,
        "reported_status":"FAILED","error_text":"fake failure","recoverable":true}),
    )
    .await?;
    finish_read(
        &independent,
        &r,
        "client-transitions",
        &request,
        &before,
        response,
        StatusCode::CONFLICT,
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn command_readback_missing_unknown_does_not_occupy_business_identity() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "missing-unknown").await?;
    let request = Uuid::new_v4().to_string();
    let (revoked, current) = sessions(&pool, &r).await?;
    let before = before_read(&independent, &r).await?;
    let response = http(
        &r,
        "POST",
        &format!("commands/{request}"),
        json!({"payload_hash":"unknown-original-payload",
        "reported_status":"UNKNOWN","error_text":"timeout","recoverable":true}),
    )
    .await?;
    assert_eq!(response.0, StatusCode::NOT_FOUND);
    assert_eq!(response.1["code"], "command_not_found");
    let response = http(&r, "GET", &format!("commands/{request}"), Value::Null).await?;
    assert_eq!(response.0, StatusCode::NOT_FOUND);
    let response = lookup(
        &r,
        &request,
        "unknown-original-payload",
        revoked,
        current,
        true,
    )
    .await?;
    finish_read(
        &independent,
        &r,
        "missing-unknown",
        &request,
        &before,
        response,
        StatusCode::NOT_FOUND,
    )
    .await?;
    let count: i64 = sqlx::query_scalar(
        "select count(*) from command_receipts where tenant_id=$1 and request_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(&request)
    .fetch_one(&independent)
    .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn command_readback_actual_material_observations_and_restart_return_same_effect() -> Result<()>
{
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "material-observe").await?;
    let (request, hash, effect) = real_material(&pool, &r).await?;
    let before = before_read(&independent, &r).await?;
    for reported in ["UNKNOWN", "SUCCEEDED"] {
        let response = http(
            &r,
            "POST",
            &format!("commands/{request}"),
            json!({"payload_hash":hash,
            "reported_status":reported,"error_text":"invented retry error","recoverable":true}),
        )
        .await?;
        assert_eq!(response.0, StatusCode::OK);
        assert_eq!(response.1["status"], "SUCCEEDED");
        assert_eq!(response.1["effect_id"], effect);
        assert_eq!(response.1["replay"], true);
    }
    let response = http(
        &r,
        "GET",
        &format!("commands/{request}/restarts?restarted=true"),
        Value::Null,
    )
    .await?;
    let body = finish_read(
        &independent,
        &r,
        "material-observe",
        &request,
        &before,
        response,
        StatusCode::OK,
    )
    .await?;
    assert_eq!(body["effect_id"], effect);
    assert_eq!(body["found_after_restart"], true);
    assert_eq!(body["version_no"], 0);
    Ok(())
}

#[tokio::test]
async fn command_readback_payload_conflict_keeps_real_material_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "material-payload").await?;
    let (request, hash, _) = real_material(&pool, &r).await?;
    let (revoked, current) = sessions(&pool, &r).await?;
    let before = before_read(&independent, &r).await?;
    let response = http(
        &r,
        "POST",
        &format!("commands/{request}"),
        json!({"payload_hash":format!("{hash}-changed"),
        "reported_status":"UNKNOWN","error_text":"timeout","recoverable":true}),
    )
    .await?;
    assert_eq!(response.0, StatusCode::CONFLICT);
    let response = lookup(
        &r,
        &request,
        &format!("{hash}-changed"),
        revoked,
        current,
        false,
    )
    .await?;
    finish_read(
        &independent,
        &r,
        "material-payload",
        &request,
        &before,
        response,
        StatusCode::CONFLICT,
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn command_readback_compatibility_quantity_and_stock_ids_cannot_execute() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "material-client-fields").await?;
    let (request, hash, effect) = real_material(&pool, &r).await?;
    let (revoked, current) = sessions(&pool, &r).await?;
    let before = before_read(&independent, &r).await?;
    let mut response = (StatusCode::OK, Value::Null);
    for unknown in [false, true] {
        response = http(
            &r,
            "POST",
            &format!("commands/{request}/lost-lookups"),
            json!({"payload_hash":hash,
            "revoked_session_id":revoked,"current_session_id":current,"client_unknown":unknown,
            "warehouse_id":Uuid::new_v4(),"sku_id":Uuid::new_v4(),"qty_units":i64::MAX}),
        )
        .await?;
        assert_eq!(response.0, StatusCode::OK);
        assert_eq!(response.1["effect_id"], effect);
    }
    finish_read(
        &independent,
        &r,
        "material-client-fields",
        &request,
        &before,
        response,
        StatusCode::OK,
    )
    .await?;
    Ok(())
}

async fn invalid_session(case: &str, alteration: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, case).await?;
    let (request, hash) = opened(&pool, &r).await?;
    let (revoked, current) = sessions(&pool, &r).await?;
    sqlx::query(alteration)
        .bind(r.f.tenant.0)
        .bind(current)
        .execute(&pool)
        .await?;
    let before = before_read(&independent, &r).await?;
    let response = lookup(&r, &request, &hash, revoked, current, false).await?;
    let body = finish_read(
        &independent,
        &r,
        case,
        &request,
        &before,
        response,
        StatusCode::FORBIDDEN,
    )
    .await?;
    assert_eq!(body["code"], "command_session_forbidden");
    Ok(())
}

#[tokio::test]
async fn command_readback_expired_active_session_is_not_current() -> Result<()> {
    invalid_session("expired-session","update browser_sessions set expires_at=now()-interval '1 second' where tenant_id=$1 and session_id=$2").await
}

#[tokio::test]
async fn command_readback_stale_credential_epoch_is_not_current() -> Result<()> {
    invalid_session(
        "stale-session",
        "update browser_sessions set credential_epoch=1 where tenant_id=$1 and session_id=$2",
    )
    .await
}

#[tokio::test]
async fn command_readback_another_actor_session_is_not_current() -> Result<()> {
    invalid_session("foreign-session","update browser_sessions set actor_user_id='different-actor' where tenant_id=$1 and session_id=$2").await
}

#[tokio::test]
async fn command_readback_historical_client_completion_requires_reconciliation() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "legacy-generic-terminal").await?;
    let (request, hash) = opened(&pool, &r).await?;
    let (revoked, current) = sessions(&pool, &r).await?;
    // Deliberately corrupt only this owned QA prototype, matching the historical client-write shape.
    sqlx::query("update command_receipts set status='SUCCEEDED',effect_id=$3 where tenant_id=$1 and request_id=$2")
        .bind(r.f.tenant.0).bind(&request).bind(r.work_order.to_string()).execute(&pool).await?;
    sqlx::query("update command_objects set line_status='SUCCEEDED',completed_at=now() where tenant_id=$1 and request_id=$2")
        .bind(r.f.tenant.0).bind(&request).execute(&pool).await?;
    let before = before_read(&independent, &r).await?;
    let response = http(&r, "GET", &format!("commands/{request}"), Value::Null).await?;
    assert_eq!(response.0, StatusCode::CONFLICT);
    assert_eq!(response.1["code"], "command_reconciliation_required");
    let response = http(
        &r,
        "POST",
        &format!("commands/{request}"),
        json!({"payload_hash":hash,
        "reported_status":"UNKNOWN","error_text":"timeout","recoverable":true}),
    )
    .await?;
    assert_eq!(response.0, StatusCode::CONFLICT);
    let response = lookup(&r, &request, &hash, revoked, current, false).await?;
    finish_read(
        &independent,
        &r,
        "legacy-generic-terminal",
        &request,
        &before,
        response,
        StatusCode::CONFLICT,
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn command_readback_twenty_lookups_do_not_wait_for_stock_writer_or_repost() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = Arc::new(context(&pool, "material-concurrent").await?);
    let (request, hash, effect) = real_material(&pool, &r).await?;
    let (revoked, current) = sessions(&pool, &r).await?;
    let before = before_read(&independent, &r).await?;
    let mut writer = pool.begin().await?;
    sqlx::query("select tenant_id from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 for update")
        .bind(r.f.tenant.0).bind(r.f.warehouse).bind(r.f.sku).fetch_one(&mut *writer).await?;
    let mut tasks = tokio::task::JoinSet::new();
    let started = tokio::time::Instant::now();
    for index in 0..20 {
        let r = Arc::clone(&r);
        let request = request.clone();
        let hash = hash.clone();
        tasks.spawn(
            async move { lookup(&r, &request, &hash, revoked, current, index % 2 == 0).await },
        );
    }
    let responses = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        let mut responses = Vec::new();
        while let Some(response) = tasks.join_next().await {
            responses.push(response??);
        }
        Ok::<_, anyhow::Error>(responses)
    })
    .await??;
    assert_eq!(responses.len(), 20);
    for (status, body) in &responses {
        assert_eq!(*status, StatusCode::OK);
        assert_eq!(body["status"], "SUCCEEDED");
        assert_eq!(body["effect_id"], effect);
        assert_eq!(body["replay"], true);
    }
    eprintln!(
        "QA command_readback_concurrency tenant={} returned=20 stock_lock_held=true elapsed_ms={}",
        r.f.tenant,
        started.elapsed().as_millis()
    );
    writer.rollback().await?;
    let response = http(&r, "GET", &format!("commands/{request}"), Value::Null).await?;
    finish_read(
        &independent,
        &r,
        "material-concurrent",
        &request,
        &before,
        response,
        StatusCode::OK,
    )
    .await?;
    Ok(())
}
