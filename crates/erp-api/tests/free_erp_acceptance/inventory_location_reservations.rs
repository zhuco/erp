//! Source reservations must share authoritative stock, identity and audit commit.
use super::inventory_location_commands::{Fixture, api, facts, movement};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use erp_repo::inventory::{InventoryRepo, InventoryReservationInput};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let f = super::inventory_location_commands::fixture(pool).await?;
    sqlx::query("update tenants set name='QA-INVENTORY-LOCATION-RESERVATIONS' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    eprintln!(
        "QA location reservation fixture tenant={} actor={} warehouse={} sku={} location={}",
        f.tenant, f.actor, f.warehouse, f.sku, f.location
    );
    let (status, body) = api(
        &f,
        "inbounds",
        movement(&f, &Uuid::new_v4().to_string(), 5),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "actual inbound: {body}");
    Ok(f)
}

fn reservation(f: &Fixture, id: &str, source: &str, quantity: i64) -> Value {
    json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"request_id":id,"quantity":quantity,
        "reference_id":source,"location_id":f.location,"batch_code":"QA-BATCH","quality_status":"GOOD"})
}

#[tokio::test]
async fn inventory_location_reservation_after_canonical_inbound() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let (status, body) = api(
        &f,
        "reservations",
        reservation(&f, &id, "SOURCE-X", 2),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "valid source reservation: {body}");
    let saved = facts(&independent, &f).await?;
    assert_eq!(saved["balances"][0]["on_hand_micros"], 5_000_000);
    assert_eq!(saved["balances"][0]["reserved_micros"], 2_000_000);
    assert_eq!(saved["ledgers"].as_array().map(Vec::len), Some(2));
    assert_eq!(saved["shared_ledgers"].as_array().map(Vec::len), Some(2));
    assert_eq!(saved["operations"].as_array().map(Vec::len), Some(2));
    assert_eq!(saved["audits"].as_array().map(Vec::len), Some(2));
    Ok(())
}

#[tokio::test]
async fn inventory_location_reservation_release_dimensions_require_identity() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let source = Uuid::new_v4();
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
    let before = facts(&independent, &f).await?;
    let mut body = reservation(&f, "unused", &source.to_string(), 1);
    body.as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("fixture body"))?
        .remove("request_id");
    let (status, result) = api(&f, "releases", body, None, None).await?;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "structured release needs original identity: {result}"
    );
    assert_eq!(
        facts(&independent, &f).await?,
        before,
        "invalid identity must not release legacy stock"
    );
    Ok(())
}
