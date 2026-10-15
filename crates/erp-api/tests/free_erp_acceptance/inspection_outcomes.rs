//! Measured failures remain facts; reinspection and review must preserve their history.
use super::{
    Result, isolated_pool,
    production_receipt_release::{Receipt, facts, http, rejected, setup},
};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn frozen(pool: &PgPool, case: &str) -> Result<(Receipt, Uuid)> {
    frozen_range(pool, case, "1", "2").await
}

pub(super) async fn frozen_range(
    pool: &PgPool,
    case: &str,
    minimum: &str,
    maximum: &str,
) -> Result<(Receipt, Uuid)> {
    let r = setup(pool, case).await?;
    let (status, result) = http(&r.f, "quality/inspection-templates", json!({
        "request_id":Uuid::new_v4().to_string(),"template_code":"QA-OUTCOME-STANDARD","kind":"FINAL",
        "lines":[{"item_code":"WIDTH","required":true,"min_qty":minimum,"max_qty":maximum,"sample_count":1}]
    })).await?;
    assert_eq!(status, StatusCode::OK, "template fixture: {result}");
    let template = result["effect_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("template effect"))?;
    let (status, result) = http(&r.f, "quality/inspection-freezes", json!({
        "request_id":Uuid::new_v4().to_string(),"template_id":template,"source_kind":"PRODUCTION_RECEIPT",
        "source_id":r.receipt,"item_code":"QA-P","batch_code":"QA-PRODUCTION"
    })).await?;
    assert_eq!(status, StatusCode::OK, "freeze fixture: {result}");
    let freeze = Uuid::parse_str(
        result["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("freeze effect"))?,
    )?;
    eprintln!(
        "QA inspection_outcome_fixture tenant={} case={case} actor={} work_order={} receipt={} freeze={freeze}",
        r.f.tenant, r.f.actor, r.work_order, r.receipt
    );
    Ok((r, freeze))
}

pub(super) async fn record(
    r: &Receipt,
    freeze: Uuid,
    request: &str,
    reading: Option<&str>,
) -> Result<(StatusCode, Value)> {
    http(
        &r.f,
        &format!("quality/inspection-freezes/{freeze}/readings"),
        json!({
            "request_id":request,"item_code":"WIDTH","reading_qty":reading,
            "inspector":"QA-INSPECTOR","reviewer":"Declared reviewer, not an account"
        }),
    )
    .await
}

pub(super) async fn original_record(pool: &PgPool, r: &Receipt, item: Uuid) -> Result<Value> {
    Ok(sqlx::query_scalar(
        "select to_jsonb(q) from mfg_quality_items q where tenant_id=$1 and item_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(item)
    .fetch_one(pool)
    .await?)
}

async fn event(pool: &PgPool, r: &Receipt, item: Uuid) -> Result<Value> {
    let present: bool = sqlx::query_scalar(
        "select to_regclass('public.shared_inspection_reading_events') is not null",
    )
    .fetch_one(pool)
    .await?;
    if !present {
        return Ok(Value::Null);
    }
    Ok(sqlx::query_scalar("select to_jsonb(e) from shared_inspection_reading_events e where tenant_id=$1 and item_id=$2")
        .bind(r.f.tenant.0).bind(item).fetch_optional(pool).await?.unwrap_or(Value::Null))
}

#[tokio::test]
async fn inspection_outcome_failed_signed_measurement_is_saved_and_blocks_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-failed-signed").await?;
    let before = facts(&pool, &r).await?;
    let request = Uuid::new_v4().to_string();
    let (status, result) = record(&r, freeze, &request, Some("-0.25")).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "a measured failure is a fact: {result}"
    );
    assert_eq!(result["reading"]["verdict"], "FAILED");
    let item = Uuid::parse_str(
        result["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("reading effect"))?,
    )?;
    let stored = original_record(&pool, &r, item).await?;
    assert_eq!(stored["reading_micros"], -250_000);
    assert_eq!(stored["reading_present"], true);
    assert_eq!(stored["passed"], false);
    let after = facts(&pool, &r).await?;
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "shared_stock_ledgers",
        "mfg_receipts",
    ] {
        assert_eq!(
            before[table], after[table],
            "recording a failure cannot change {table}"
        );
    }
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await?;
    let (status, replay) = record(&r, freeze, &request, Some("-0.25")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["replay"], true);
    assert_eq!(replay["reading"], result["reading"]);
    assert_eq!(original_record(&pool, &r, item).await?, stored);
    Ok(())
}

#[tokio::test]
async fn inspection_outcome_missing_required_value_is_saved_as_missing() -> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-required-missing").await?;
    let (status, result) = record(&r, freeze, &Uuid::new_v4().to_string(), None).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "a missing value must remain distinguishable: {result}"
    );
    assert_eq!(result["reading"]["verdict"], "MISSING");
    let item = Uuid::parse_str(
        result["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("reading effect"))?,
    )?;
    let stored = original_record(&pool, &r, item).await?;
    assert_eq!(stored["reading_present"], false);
    assert_eq!(stored["passed"], false);
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn inspection_outcome_declared_reviewer_and_authenticated_submitter_are_persisted()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (r, freeze) = frozen(&pool, "outcome-reviewer-provenance").await?;
    let request = Uuid::new_v4().to_string();
    let (status, result) = record(&r, freeze, &request, Some("1.5")).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let item = Uuid::parse_str(
        result["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("reading effect"))?,
    )?;
    let event = event(&pool, &r, item).await?;
    assert!(
        !event.is_null(),
        "the original request must have durable provenance"
    );
    assert_eq!(
        event["declared_reviewer"],
        "Declared reviewer, not an account"
    );
    assert_eq!(event["actor_user_id"], r.f.actor.to_string());
    assert_eq!(event["actor_source"], "USER");
    assert_eq!(event["request_id"], request);
    assert_eq!(event["attempt_no"], 1);
    assert_eq!(event["previous_item_id"], Value::Null);
    assert_eq!(event["review_required"], false);
    eprintln!(
        "QA inspection_outcome_provenance tenant={} item={item} event={event}",
        r.f.tenant
    );
    Ok(())
}
