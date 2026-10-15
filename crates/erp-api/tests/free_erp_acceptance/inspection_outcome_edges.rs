//! Actual append/review/release history, immutable commands and source competition.
use super::{
    Result,
    inspection_outcomes::{frozen, frozen_range, original_record, record},
    isolated_pool,
    production_receipt_release::{Receipt, facts, http, rejected, release},
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
use tokio::{
    sync::Barrier,
    task::JoinSet,
    time::{Duration, Instant},
};
use tower::ServiceExt;
use uuid::Uuid;

pub(super) async fn scoped_http(
    r: &Receipt,
    method: &str,
    path: &str,
    body: Value,
    permissions: Vec<String>,
) -> Result<(StatusCode, Value)> {
    let jwt = Arc::new(JwtService::new("synthetic-inspection-outcome-key".into()));
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        r.f.actor.to_string(),
        r.f.tenant.to_string(),
        permissions,
        false,
        now,
        now + 300,
    ))?;
    let database = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let response = build_app(AppState::new(Db::connect_lazy(&database)?, jwt))
        .oneshot(
            Request::builder()
                .method(method)
                .uri(if path.starts_with('/') {
                    path.to_string()
                } else {
                    format!("/v1/tenants/{}/{path}", r.f.tenant)
                })
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let response = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA production_release_http tenant={} path={path} input={body} status={} response={response}",
        r.f.tenant,
        status.as_u16()
    );
    Ok((status, response))
}

pub(super) async fn owned_facts(pool: &PgPool, r: &Receipt) -> Result<Value> {
    let mut result = facts(pool, r).await?;
    for table in [
        "shared_inspection_reading_events",
        "shared_inspection_reading_reviews",
    ] {
        result[table] = sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from {table} x where tenant_id=$1"))
            .bind(r.f.tenant.0).fetch_one(pool).await?;
    }
    Ok(result)
}

pub(super) fn effect(result: &Value) -> Result<Uuid> {
    Ok(Uuid::parse_str(result["effect_id"].as_str().ok_or_else(
        || anyhow::anyhow!("original effect id missing"),
    )?)?)
}

async fn reinspect(
    r: &Receipt,
    freeze: Uuid,
    previous: Uuid,
    request: &str,
    reading: Option<&str>,
    reason: &str,
) -> Result<(StatusCode, Value)> {
    http(&r.f,&format!("quality/inspection-freezes/{freeze}/reinspections"),json!({
        "request_id":request,"item_code":"WIDTH","reading_qty":reading,"inspector":"QA-INSPECTOR",
        "reviewer":"Declared reviewer, not an account","previous_item_id":previous,"reason":reason
    })).await
}

async fn review(
    r: &Receipt,
    freeze: Uuid,
    item: Uuid,
    request: &str,
    decision: &str,
    reason: &str,
) -> Result<(StatusCode, Value)> {
    http(
        &r.f,
        &format!("quality/inspection-freezes/{freeze}/readings/{item}/reviews"),
        json!({"request_id":request,"decision":decision,"reason":reason}),
    )
    .await
}

pub(super) async fn failed(r: &Receipt, freeze: Uuid) -> Result<Uuid> {
    let (status, result) = record(r, freeze, &Uuid::new_v4().to_string(), Some("3")).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["reading"]["verdict"], "FAILED");
    effect(&result)
}

pub(super) async fn passing_reinspection(
    r: &Receipt,
    freeze: Uuid,
    previous: Uuid,
) -> Result<Uuid> {
    let (status, result) = reinspect(
        r,
        freeze,
        previous,
        &Uuid::new_v4().to_string(),
        Some("1.5"),
        "Reworked and measured again",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["reading"]["verdict"], "PASSED");
    assert_eq!(result["reading"]["review_required"], true);
    effect(&result)
}

#[tokio::test]
async fn inspection_outcome_reinspection_waits_for_actual_review_and_release_keeps_failed_history()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-review-release").await?;
    let first = failed(&r, freeze).await?;
    let original = original_record(&pool, &r, first).await?;
    let current = passing_reinspection(&r, freeze, first).await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await?;
    let before = owned_facts(&pool, &r).await?;
    let review_request = Uuid::new_v4().to_string();
    let (status, result) = review(&r, freeze, current, &review_request, "approve", "").await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["review"]["reviewer_user_id"], r.f.actor.to_string());
    let after = owned_facts(&pool, &r).await?;
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "shared_stock_ledgers",
        "mfg_receipts",
    ] {
        assert_eq!(before[table], after[table], "review cannot change {table}");
    }
    let (status, replay) = review(&r, freeze, current, &review_request, "APPROVE", "").await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["replay"], true);
    assert_eq!(replay["review"], result["review"]);
    let (status, result) = release(&r, &Uuid::new_v4().to_string(), r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let proof:Value=sqlx::query_scalar("select snapshot->'quality' from mfg_receipt_release_commands where tenant_id=$1 and receipt_id=$2")
        .bind(r.f.tenant.0).bind(r.receipt).fetch_one(&pool).await?;
    assert_eq!(proof["readings"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        proof["effective_readings"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        proof["effective_readings"][0]["item_id"],
        current.to_string()
    );
    assert_eq!(proof["reading_events"].as_array().map(Vec::len), Some(2));
    assert_eq!(proof["reading_reviews"].as_array().map(Vec::len), Some(1));
    assert_eq!(original_record(&pool, &r, first).await?, original);
    eprintln!(
        "QA inspection_outcome_release_proof tenant={} proof={proof}",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_failed_measurement_cannot_be_approved() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-failed-approval").await?;
    let item = failed(&r, freeze).await?;
    let before = owned_facts(&pool, &r).await?;
    let (status, result) =
        review(&r, freeze, item, &Uuid::new_v4().to_string(), "APPROVE", "").await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
    assert_eq!(before, owned_facts(&pool, &r).await?);
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn inspection_outcome_zero_is_present_and_judged_against_original_range() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen_range(&pool, "outcome-real-zero", "0", "2").await?;
    let (status, result) = record(&r, freeze, &Uuid::new_v4().to_string(), Some("0")).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["reading"]["reading_present"], true);
    assert_eq!(result["reading"]["reading_micros"], "0");
    assert_eq!(result["reading"]["verdict"], "PASSED");
    let (status, result) = release(&r, &Uuid::new_v4().to_string(), r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_v1_payload_and_original_retry_survive_review_changes() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-original-v1").await?;
    let request = Uuid::new_v4().to_string();
    let (status, first) = record(&r, freeze, &request, Some("1.5")).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let payload: String = sqlx::query_scalar(
        "select payload_hash from command_receipts where tenant_id=$1 and request_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(&request)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        serde_json::from_str::<Value>(&payload)?,
        json!([
            1,
            freeze,
            "WIDTH",
            1_500_000,
            "QA-INSPECTOR",
            "Declared reviewer, not an account"
        ])
    );
    let (status, result) = review(
        &r,
        freeze,
        effect(&first)?,
        &Uuid::new_v4().to_string(),
        "REJECT",
        "Second sample required",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let before = owned_facts(&pool, &r).await?;
    let (status, replay) = record(&r, freeze, &request, Some("+01.500000")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["replay"], true);
    assert_eq!(first["reading"], replay["reading"]);
    let (status, result) = record(&r, freeze, &request, Some("1.6")).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_stale_previous_and_stale_review_have_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-stale-attempt").await?;
    let first = failed(&r, freeze).await?;
    let (status, second) = reinspect(
        &r,
        freeze,
        first,
        &Uuid::new_v4().to_string(),
        Some("4"),
        "Still failed",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{second}");
    let before = owned_facts(&pool, &r).await?;
    let (status, result) = reinspect(
        &r,
        freeze,
        first,
        &Uuid::new_v4().to_string(),
        Some("1.5"),
        "Stale screen",
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
    let (status, result) = review(
        &r,
        freeze,
        first,
        &Uuid::new_v4().to_string(),
        "REJECT",
        "Stale review",
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_rejection_preserves_passed_value_and_allows_explicit_reinspection()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-reject-pass").await?;
    let (status, first) = record(&r, freeze, &Uuid::new_v4().to_string(), Some("1.5")).await?;
    assert_eq!(status, StatusCode::OK);
    let first = effect(&first)?;
    let original = original_record(&pool, &r, first).await?;
    let (status, result) = review(
        &r,
        freeze,
        first,
        &Uuid::new_v4().to_string(),
        "REJECT",
        "Repeat measurement required",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await?;
    let second = passing_reinspection(&r, freeze, first).await?;
    let (status, result) = review(
        &r,
        freeze,
        second,
        &Uuid::new_v4().to_string(),
        "APPROVE",
        "",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let (status, result) = release(&r, &Uuid::new_v4().to_string(), r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(original_record(&pool, &r, first).await?, original);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_completed_stock_source_refuses_new_review_but_preserves_retry()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-closed-source").await?;
    let request = Uuid::new_v4().to_string();
    let (status, first) = record(&r, freeze, &request, Some("1.5")).await?;
    assert_eq!(status, StatusCode::OK);
    let item = effect(&first)?;
    let (status, result) = release(&r, &Uuid::new_v4().to_string(), r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let before = owned_facts(&pool, &r).await?;
    let (status, result) = review(
        &r,
        freeze,
        item,
        &Uuid::new_v4().to_string(),
        "REJECT",
        "After physical release",
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    let (status, replay) = record(&r, freeze, &request, Some("1.5")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["reading"], replay["reading"]);
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_history_is_read_only_even_while_its_owner_is_locked() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-read-only-history").await?;
    let first = failed(&r, freeze).await?;
    let second = passing_reinspection(&r, freeze, first).await?;
    let (status, result) = review(
        &r,
        freeze,
        second,
        &Uuid::new_v4().to_string(),
        "APPROVE",
        "",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let before = owned_facts(&pool, &r).await?;
    let mut owner = pool.begin().await?;
    sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
        .bind(r.f.tenant.0).bind(r.work_order).fetch_one(&mut *owner).await?;
    let answer = tokio::time::timeout(
        Duration::from_secs(3),
        scoped_http(
            &r,
            "GET",
            &format!("quality/inspection-freezes/{freeze}/readings?item_code=WIDTH"),
            json!({}),
            vec!["quality.read".into()],
        ),
    )
    .await;
    owner.rollback().await?;
    let (status, result) = answer
        .map_err(|_| anyhow::anyhow!("read-only history waited for the owner write lock"))??;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["history"].as_array().map(Vec::len), Some(2));
    assert_eq!(result["history"][0]["reading"]["verdict"], "FAILED");
    assert_eq!(
        result["history"][1]["review"]["reviewer_user_id"],
        r.f.actor.to_string()
    );
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_wrong_owner_and_cross_tenant_previous_have_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let (foreign, foreign_freeze) = frozen(&pool, "outcome-foreign-reading").await?;
    let foreign_item = failed(&foreign, foreign_freeze).await?;
    let (r, freeze) = frozen(&pool, "outcome-owned-reading").await?;
    let before = owned_facts(&pool, &r).await?;
    let foreign_before = owned_facts(&pool, &foreign).await?;
    let (status, result) = reinspect(
        &r,
        freeze,
        foreign_item,
        &Uuid::new_v4().to_string(),
        Some("1.5"),
        "Wrong source",
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    let (status, result) = review(
        &r,
        foreign_freeze,
        foreign_item,
        &Uuid::new_v4().to_string(),
        "REJECT",
        "Foreign record",
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
    let (status, result) = scoped_http(
        &r,
        "GET",
        &format!(
            "/v1/tenants/{}/quality/inspection-freezes/{foreign_freeze}/readings?item_code=WIDTH",
            foreign.f.tenant
        ),
        json!({}),
        vec!["quality.read".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{result}");
    assert_eq!(before, owned_facts(&pool, &r).await?);
    assert_eq!(foreign_before, owned_facts(&pool, &foreign).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_permissions_and_current_actor_status_are_enforced() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-actor-authority").await?;
    let before = owned_facts(&pool, &r).await?;
    let input = json!({"request_id":Uuid::new_v4().to_string(),"item_code":"WIDTH","reading_qty":"1.5","inspector":"QA-INSPECTOR","reviewer":"DECLARED"});
    let (status, result) = scoped_http(
        &r,
        "POST",
        &format!("quality/inspection-freezes/{freeze}/readings"),
        input,
        vec!["quality.read".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{result}");
    let (status, result) = scoped_http(
        &r,
        "GET",
        &format!("quality/inspection-freezes/{freeze}/readings?item_code=WIDTH"),
        json!({}),
        vec!["quality.write".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{result}");
    assert_eq!(before, owned_facts(&pool, &r).await?);
    let (status, first) = scoped_http(
        &r,
        "POST",
        &format!("quality/inspection-freezes/{freeze}/readings"),
        json!({"request_id":Uuid::new_v4().to_string(),"item_code":"WIDTH","reading_qty":"3","inspector":"QA-INSPECTOR","reviewer":"DECLARED"}),
        vec!["quality.write".into()],
    ).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (status, second) = scoped_http(
        &r,
        "POST",
        &format!("quality/inspection-freezes/{freeze}/reinspections"),
        json!({"request_id":Uuid::new_v4().to_string(),"item_code":"WIDTH","reading_qty":"1.5","inspector":"QA-INSPECTOR","reviewer":"DECLARED","previous_item_id":effect(&first)?,"reason":"Actual quality.write authority"}),
        vec!["quality.write".into()],
    ).await?;
    assert_eq!(status, StatusCode::OK, "{second}");
    let (status, result) = scoped_http(
        &r,
        "POST",
        &format!(
            "quality/inspection-freezes/{freeze}/readings/{}/reviews",
            effect(&second)?
        ),
        json!({"request_id":Uuid::new_v4().to_string(),"decision":"APPROVE","reason":""}),
        vec!["quality.write".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    sqlx::query("update users set status='DISABLED' where tenant_id=$1 and user_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.f.actor)
        .execute(&pool)
        .await?;
    let before = owned_facts(&pool, &r).await?;
    let (status, result) = record(&r, freeze, &Uuid::new_v4().to_string(), Some("3")).await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{result}");
    let (status, result) = scoped_http(
        &r,
        "GET",
        &format!("quality/inspection-freezes/{freeze}/readings?item_code=WIDTH"),
        json!({}),
        vec!["quality.read".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{result}");
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_precision_and_forged_reviewer_fields_are_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-invalid-fields").await?;
    let before = owned_facts(&pool, &r).await?;
    let (status, result) =
        record(&r, freeze, &Uuid::new_v4().to_string(), Some("1.0000001")).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
    let (status, result) = http(
        &r.f,
        &format!("quality/inspection-freezes/{freeze}/readings"),
        json!({
            "request_id":Uuid::new_v4().to_string(),"item_code":"WIDTH","reading_qty":"1.5",
            "inspector":"QA-INSPECTOR","reviewer":"LABEL","actor_user_id":r.f.actor
        }),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_twenty_attempt_limit_preserves_complete_history() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-attempt-budget").await?;
    let mut previous = failed(&r, freeze).await?;
    for index in 2..=20 {
        let (status, result) = reinspect(
            &r,
            freeze,
            previous,
            &Uuid::new_v4().to_string(),
            Some("3"),
            &format!("Actual failed attempt {index}"),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(result["reading"]["attempt_no"], index);
        previous = effect(&result)?;
    }
    let before = owned_facts(&pool, &r).await?;
    let (status, result) = reinspect(
        &r,
        freeze,
        previous,
        &Uuid::new_v4().to_string(),
        Some("1.5"),
        "Over the explicit budget",
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
    let (status, result) = scoped_http(
        &r,
        "GET",
        &format!("quality/inspection-freezes/{freeze}/readings?item_code=WIDTH"),
        json!({}),
        vec!["quality.read".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["history"].as_array().map(Vec::len), Some(20));
    assert_eq!(before, owned_facts(&pool, &r).await?);
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn inspection_outcome_original_rows_reviews_and_commands_refuse_sql_mutation() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-immutable-facts").await?;
    let first = failed(&r, freeze).await?;
    let second = passing_reinspection(&r, freeze, first).await?;
    let (status, result) = review(
        &r,
        freeze,
        second,
        &Uuid::new_v4().to_string(),
        "APPROVE",
        "",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let before = owned_facts(&pool, &r).await?;
    for statement in [
        "update mfg_quality_items set reading_micros=1500000,passed=true where tenant_id=$1",
        "update shared_inspection_reading_events set verdict='PASSED' where tenant_id=$1",
        "delete from shared_inspection_reading_events where tenant_id=$1",
        "update shared_inspection_reading_reviews set decision='REJECT',reason='Rewrite' where tenant_id=$1",
        "delete from shared_inspection_reading_reviews where tenant_id=$1",
        "update command_receipts set payload_hash='rewrite' where tenant_id=$1 and request_id in(select request_id from shared_inspection_reading_events where tenant_id=$1)",
        "delete from command_receipts where tenant_id=$1 and request_id in(select request_id from shared_inspection_reading_reviews where tenant_id=$1)",
    ] {
        let outcome = sqlx::query(statement)
            .bind(r.f.tenant.0)
            .execute(&pool)
            .await;
        eprintln!(
            "QA inspection_outcome_sql_guard tenant={} statement={statement} refused={}",
            r.f.tenant,
            outcome.is_err()
        );
        assert!(
            outcome.is_err(),
            "original fact mutation must fail: {statement}"
        );
    }
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}

#[derive(Clone, Copy)]
enum Competition {
    DifferentReinspection,
    SameReinspection,
    DifferentReview,
    MixedReviewRelease,
}

async fn competition(mode: Competition, case: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, case).await?;
    let first = failed(&r, freeze).await?;
    let prior = if matches!(
        mode,
        Competition::DifferentReview | Competition::MixedReviewRelease
    ) {
        passing_reinspection(&r, freeze, first).await?
    } else {
        first
    };
    let r = Arc::new(r);
    let barrier = Arc::new(Barrier::new(21));
    let mut tasks = JoinSet::new();
    let mut owner = pool.begin().await?;
    sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
        .bind(r.f.tenant.0).bind(r.work_order).fetch_one(&mut *owner).await?;
    let blocker: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *owner)
        .await?;
    let same_request = Uuid::new_v4().to_string();
    for index in 0..20 {
        let r = r.clone();
        let barrier = barrier.clone();
        let request = if matches!(mode, Competition::SameReinspection) {
            same_request.clone()
        } else {
            Uuid::new_v4().to_string()
        };
        tasks.spawn(async move {
            barrier.wait().await;
            if matches!(mode, Competition::MixedReviewRelease) && index % 4 == 2 {
                release(&r, &request, r.work_order).await
            } else if matches!(mode, Competition::DifferentReview)
                || (matches!(mode, Competition::MixedReviewRelease) && index % 4 < 2)
            {
                review(&r, freeze, prior, &request, "APPROVE", "").await
            } else {
                reinspect(
                    &r,
                    freeze,
                    prior,
                    &request,
                    Some("1.5"),
                    "Concurrent reinspection",
                )
                .await
            }
        });
    }
    barrier.wait().await;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut chain;
    loop {
        chain=sqlx::query_scalar::<_,Value>("select coalesce(jsonb_agg(jsonb_build_object('pid',pid,'blocked_by',pg_blocking_pids(pid)) order by pid),'[]'::jsonb) from pg_stat_activity where datname=current_database() and application_name=$1 and cardinality(pg_blocking_pids(pid))>0")
            .bind(format!("qa_release_{}",r.f.tenant)).fetch_one(&pool).await?;
        if chain.as_array().is_some_and(|rows| rows.len() >= 20) || Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    owner.commit().await?;
    let mut responses = Vec::new();
    while let Some(response) = tasks.join_next().await {
        responses.push(response??);
    }
    eprintln!(
        "QA inspection_outcome_competition tenant={} case={case} blocker={blocker} chain={chain} responses={}",
        r.f.tenant,
        responses.len()
    );
    assert_eq!(
        chain.as_array().map(Vec::len),
        Some(20),
        "real PG waiting chain: {chain}"
    );
    assert_eq!(responses.len(), 20);
    let success = responses
        .iter()
        .filter(|(status, _)| *status == StatusCode::OK)
        .count();
    if matches!(mode, Competition::SameReinspection) {
        assert_eq!(success, 20);
        assert_eq!(
            responses
                .iter()
                .filter(|(_, body)| body["replay"] == false)
                .count(),
            1
        );
        let original = &responses[0].1["reading"];
        assert!(
            responses
                .iter()
                .all(|(_, body)| &body["reading"] == original)
        );
    } else if matches!(mode, Competition::MixedReviewRelease) {
        assert_eq!(
            responses
                .iter()
                .filter(
                    |(status, result)| *status == StatusCode::OK && result.get("review").is_some()
                )
                .count(),
            1
        );
        assert!(
            responses
                .iter()
                .filter(|(status, _)| *status != StatusCode::OK)
                .all(|(status, _)| matches!(
                    *status,
                    StatusCode::UNPROCESSABLE_ENTITY | StatusCode::CONFLICT
                ))
        );
        let already_released: bool = sqlx::query_scalar(
            "select status='RELEASED' from mfg_receipts where tenant_id=$1 and receipt_id=$2",
        )
        .bind(r.f.tenant.0)
        .bind(r.receipt)
        .fetch_one(&pool)
        .await?;
        if !already_released {
            let (status, result) = release(&r, &Uuid::new_v4().to_string(), r.work_order).await?;
            assert_eq!(status, StatusCode::OK, "{result}");
        }
    } else {
        assert_eq!(success, 1);
        assert!(
            responses
                .iter()
                .filter(|(status, _)| *status != StatusCode::OK)
                .all(|(status, _)| *status == StatusCode::UNPROCESSABLE_ENTITY)
        );
    }
    let state = owned_facts(&pool, &r).await?;
    assert_eq!(
        state["shared_inspection_reading_events"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        state["shared_inspection_reading_reviews"]
            .as_array()
            .map(Vec::len),
        Some(
            if matches!(
                mode,
                Competition::DifferentReview | Competition::MixedReviewRelease
            ) {
                1
            } else {
                0
            }
        )
    );
    assert_eq!(
        state["mfg_receipts"][0]["status"],
        if matches!(mode, Competition::MixedReviewRelease) {
            "RELEASED"
        } else {
            "HOLD"
        }
    );
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_twenty_different_reinspections_append_only_one_effect() -> Result<()> {
    competition(
        Competition::DifferentReinspection,
        "outcome-concurrent-different",
    )
    .await
}
#[tokio::test]
async fn inspection_outcome_twenty_identical_reinspections_return_one_original_result() -> Result<()>
{
    competition(Competition::SameReinspection, "outcome-concurrent-same").await
}
#[tokio::test]
async fn inspection_outcome_twenty_actual_reviews_commit_one_review() -> Result<()> {
    competition(Competition::DifferentReview, "outcome-concurrent-review").await
}

#[tokio::test]
async fn inspection_outcome_twenty_mixed_reading_review_release_requests_keep_one_physical_effect()
-> Result<()> {
    competition(Competition::MixedReviewRelease, "outcome-concurrent-mixed").await
}

async fn write_fault(table: &str, case: &str, reviewing: bool) -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, case).await?;
    let item = if reviewing {
        let first = failed(&r, freeze).await?;
        Some(passing_reinspection(&r, freeze, first).await?)
    } else {
        None
    };
    let before = owned_facts(&pool, &r).await?;
    let name = format!("qa_io_{}", Uuid::new_v4().simple());
    assert!(matches!(
        table,
        "shared_inspection_reading_events" | "shared_inspection_reading_reviews" | "audit_logs"
    ));
    sqlx::query(&format!("create sequence {name}_seq"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("create function {name}_fn() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then perform nextval('{name}_seq'); raise exception 'QA_INSPECTION_OUTCOME_INSERT_FAILURE'; end if; return new; end $$",r.f.tenant)).execute(&pool).await?;
    sqlx::query(&format!("create trigger {name}_tr before insert on {table} for each row execute function {name}_fn()")).execute(&pool).await?;
    let request = Uuid::new_v4().to_string();
    let outcome = if let Some(item) = item {
        review(&r, freeze, item, &request, "APPROVE", "").await
    } else {
        record(&r, freeze, &request, Some("3")).await
    };
    let fired: (i64, bool) =
        sqlx::query_as(&format!("select last_value,is_called from {name}_seq"))
            .fetch_one(&pool)
            .await?;
    sqlx::query(&format!("drop trigger {name}_tr on {table}"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop function {name}_fn()"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop sequence {name}_seq"))
        .execute(&pool)
        .await?;
    let (status, result) = outcome?;
    eprintln!(
        "QA inspection_outcome_actual_fault tenant={} table={table} request={request} marker={} called={} status={} response={result}",
        r.f.tenant,
        fired.0,
        fired.1,
        status.as_u16()
    );
    assert_eq!(fired, (1, true));
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{result}");
    assert!(
        !result
            .to_string()
            .contains("QA_INSPECTION_OUTCOME_INSERT_FAILURE")
    );
    assert_eq!(before, owned_facts(&pool, &r).await?);
    let (status, original) = if let Some(item) = item {
        review(&r, freeze, item, &request, "APPROVE", "").await?
    } else {
        record(&r, freeze, &request, Some("3")).await?
    };
    assert_eq!(status, StatusCode::OK, "{original}");
    let after = owned_facts(&pool, &r).await?;
    let (status, replay) = if let Some(item) = item {
        review(&r, freeze, item, &request, "APPROVE", "").await?
    } else {
        record(&r, freeze, &request, Some("3")).await?
    };
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["replay"], true);
    assert_eq!(
        original[if reviewing { "review" } else { "reading" }],
        replay[if reviewing { "review" } else { "reading" }]
    );
    assert_eq!(after, owned_facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_reading_event_insert_fault_rolls_back_and_retries_original_id()
-> Result<()> {
    write_fault(
        "shared_inspection_reading_events",
        "outcome-reading-fault",
        false,
    )
    .await
}
#[tokio::test]
async fn inspection_outcome_review_insert_fault_rolls_back_and_retries_original_id() -> Result<()> {
    write_fault(
        "shared_inspection_reading_reviews",
        "outcome-review-fault",
        true,
    )
    .await
}
#[tokio::test]
async fn inspection_outcome_mandatory_audit_insert_fault_rolls_back_and_retries_original_id()
-> Result<()> {
    write_fault("audit_logs", "outcome-audit-fault", false).await
}
