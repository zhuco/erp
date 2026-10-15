//! Legacy callers retain their route while stock, source and logs share one commit.
use super::inventory_location_commands::{Fixture, api, facts};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use chrono::Utc;
use erp_domain::quantity::parse_qty;
use erp_repo::{
    inventory::{InventoryRepo, InventoryReservationInput, InventoryService},
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn fixture(pool: &PgPool) -> Result<(Fixture, Uuid)> {
    let f = super::inventory_location_commands::fixture(pool).await?;
    let source = Uuid::new_v4();
    sqlx::query(
        "update tenants set name='QA-INVENTORY-LEGACY-RESERVATION-ATOMICITY' where tenant_id=$1",
    )
    .bind(f.tenant.0)
    .execute(pool)
    .await?;
    InventoryService::new(pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
            },
            on_hand: parse_qty("2.9")?,
            reserved: parse_qty("0.9")?,
            frozen: parse_qty("0.9")?,
            in_transit: parse_qty("2.3")?,
            reason_type: "QA_LEGACY_RESERVATION_OPENING".into(),
            ref_type: Some("QA_LEGACY_RESERVATION_OPENING".into()),
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;
    eprintln!(
        "QA legacy reservation fixture tenant={} actor={} warehouse={} sku={} source={}",
        f.tenant, f.actor, f.warehouse, f.sku, source
    );
    Ok((f, source))
}

pub(super) fn body(f: &Fixture, source: Uuid) -> Value {
    json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"quantity":1,"order_item_id":source})
}

pub(super) async fn saved(pool: &PgPool, f: &Fixture) -> Result<Value> {
    let reservations: Value=sqlx::query_scalar("select coalesce(jsonb_agg(to_jsonb(r) order by reservation_id),'[]'::jsonb) from inventory_reservations r where tenant_id=$1")
        .bind(f.tenant.0).fetch_one(pool).await?;
    Ok(json!({"stock":facts(pool,f).await?,"reservations":reservations}))
}

async fn log_failure(action: &str, table: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, source) = fixture(&pool).await?;
    if action == "releases" {
        InventoryRepo::new(&pool)
            .reserve_for_order_item(InventoryReservationInput {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
                order_id: None,
                order_item_id: source,
                qty: 1,
            })
            .await?;
    }
    sqlx::query("with qa_shop as (insert into shops(tenant_id,shop_id,platform,platform_shop_id,shop_name) values($1,gen_random_uuid(),'qa',gen_random_uuid()::text,'QA mapping shop') returning shop_id) insert into channel_sku_mappings(tenant_id,sku_id,status,shop_id,platform,platform_sku_id) values($1,$2,'ACTIVE',(select shop_id from qa_shop),'qa',$2::text)")
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(&pool)
        .await?;
    let schema = format!("qa_legacy_reservation_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("create schema {schema}"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("create view {schema}.{table} as select * from public.{table} where false with local check option"))
        .execute(&pool).await?;
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let separator = if url.contains('?') { '&' } else { '?' };
    let url = format!("{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog");
    assert!(super::approved_database_url(&url)?);
    eprintln!(
        "QA legacy reservation failure tenant={} action={action} schema={schema} table={table}",
        f.tenant
    );
    let before = saved(&independent, &f).await?;
    let (status, result) = api(&f, action, body(&f, source), None, Some(url)).await?;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{result}");
    let message = result["message"]
        .as_str()
        .or_else(|| result["text"].as_str())
        .ok_or_else(|| anyhow::anyhow!("fault message"))?;
    assert!(
        message.contains("check option") && message.contains(table),
        "{message}"
    );
    let after = saved(&independent, &f).await?;
    eprintln!(
        "QA legacy reservation rollback tenant={} action={action} table={table} before={before} after={after}",
        f.tenant
    );
    assert_eq!(
        after, before,
        "late log failure must not commit stock, source, Outbox or either log"
    );
    let (status, result) = api(&f, action, body(&f, source), None, None).await?;
    assert_eq!(status, StatusCode::OK, "after proved rollback: {result}");
    let now = saved(&independent, &f).await?;
    assert_eq!(now["stock"]["operations"].as_array().map(Vec::len), Some(1));
    assert_eq!(now["stock"]["audits"].as_array().map(Vec::len), Some(1));
    assert_eq!(now["stock"]["outbox"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        result["record"]["quantities"]["available"],
        if action == "reservations" {
            "0.1"
        } else {
            "1.1"
        }
    );
    Ok(())
}

#[tokio::test]
async fn legacy_reservation_create_operation_log_failure_rolls_back() -> Result<()> {
    log_failure("reservations", "operation_logs").await
}
#[tokio::test]
async fn legacy_reservation_create_audit_log_failure_rolls_back() -> Result<()> {
    log_failure("reservations", "audit_logs").await
}
#[tokio::test]
async fn legacy_reservation_release_operation_log_failure_rolls_back() -> Result<()> {
    log_failure("releases", "operation_logs").await
}
#[tokio::test]
async fn legacy_reservation_release_audit_log_failure_rolls_back() -> Result<()> {
    log_failure("releases", "audit_logs").await
}
#[tokio::test]
async fn legacy_reservation_retries_add_no_duplicate_logs() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, source) = fixture(&pool).await?;
    for action in ["reservations", "releases"] {
        let (status, result) = api(&f, action, body(&f, source), None, None).await?;
        assert_eq!(status, StatusCode::OK, "{result}");
        let before = saved(&independent, &f).await?;
        let (status, result) = api(&f, action, body(&f, source), None, None).await?;
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(
            saved(&independent, &f).await?,
            before,
            "same source retry must not add either log"
        );
    }
    Ok(())
}
#[tokio::test]
async fn legacy_reservation_retry_rejects_changed_order_context() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, source) = fixture(&pool).await?;
    let mut original = body(&f, source);
    original["order_id"] = json!(Uuid::new_v4());
    let (status, result) = api(&f, "reservations", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let before = saved(&independent, &f).await?;
    original["order_id"] = json!(Uuid::new_v4());
    let (status, result) = api(&f, "reservations", original, None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    assert_eq!(saved(&independent, &f).await?, before);
    Ok(())
}
#[tokio::test]
async fn legacy_reservation_conflicting_source_aliases_are_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, source) = fixture(&pool).await?;
    let before = saved(&independent, &f).await?;
    let mut original = body(&f, source);
    original["reference_id"] = json!(Uuid::new_v4());
    let (status, result) = api(&f, "reservations", original, None, None).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
    assert_eq!(saved(&independent, &f).await?, before);
    Ok(())
}
