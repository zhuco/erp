//! Manufacturing and trade must admit against the same exact physical balance.

use super::{Result, isolated_pool};
use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_domain::{
    manufacturing::{MaterialMove, MaterialMoveKind},
    quantity::parse_qty,
};
use erp_repo::{
    db::Db,
    inventory::{
        InventoryDelta, InventoryRepo, InventoryReservationAction, InventoryReservationInput,
        InventoryService,
    },
    inventory_precision::{InventoryPreciseDelta, InventoryPrecisionError, InventoryQuantityKey},
    shared_contract::{
        ManufacturingReservationInput, MaterialMoveInput, SharedContractRepo, SharedWrite,
    },
};
use erp_types::TenantId;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    tenant: TenantId,
    warehouse: Uuid,
    sku: Uuid,
    order: Uuid,
}

async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let opening = Uuid::new_v4();
    let f = Fixture {
        tenant: TenantId(Uuid::new_v4()),
        warehouse: Uuid::new_v4(),
        sku: Uuid::new_v4(),
        order: Uuid::new_v4(),
    };
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-CANONICAL-PRECISION')")
        .bind(Uuid::from(f.tenant))
        .execute(pool)
        .await?;
    sqlx::query(
        "insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Precision QA',$3)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.warehouse)
    .bind(f.warehouse.to_string())
    .execute(pool)
    .await?;
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-PREC',(select product_id from qa_parent))")
        .bind(Uuid::from(f.tenant))
        .bind(f.sku)
        .execute(pool)
        .await?;
    sqlx::query(
        "insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand) values($1,$2,$3,1)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.warehouse)
    .bind(f.sku)
    .execute(pool)
    .await?;
    sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_in_transit,delta_frozen,reason_type,ref_type,ref_id) values($1,$2,$3,1,0,0,0,'QA_OPENING','mfg_material_move',$4)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(opening.to_string()).execute(pool).await?;
    sqlx::query("insert into mfg_qty_balances(tenant_id,warehouse_id,sku_id,on_hand_micros) values($1,$2,$3,1000000)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).execute(pool).await?;
    sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,1000000,'QA_OPENING')")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(opening).execute(pool).await?;
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-PREC-WO','QA-PREC','RELEASED',1000000)")
        .bind(Uuid::from(f.tenant)).bind(f.order).execute(pool).await?;
    eprintln!(
        "QA inventory precision tenant={} order={}",
        Uuid::from(f.tenant),
        f.order
    );
    Ok(f)
}

fn whole_delta(f: &Fixture, quantity: i64) -> InventoryDelta {
    InventoryDelta {
        tenant_id: f.tenant,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        delta_on_hand: quantity,
        delta_reserved: 0,
        delta_in_transit: 0,
        delta_frozen: 0,
        reason_type: "QA_TRADE_SHIP".into(),
        ref_type: Some("QA_SHIP".into()),
        ref_id: Some(Uuid::new_v4().to_string()),
        occurred_at: Utc::now(),
    }
}

fn precise_delta(
    f: &Fixture,
    hand: &str,
    reserved: &str,
    frozen: &str,
) -> Result<InventoryPreciseDelta> {
    Ok(InventoryPreciseDelta {
        key: InventoryQuantityKey {
            tenant_id: f.tenant,
            warehouse_id: f.warehouse,
            sku_id: f.sku,
        },
        on_hand: parse_qty(hand)?,
        reserved: parse_qty(reserved)?,
        frozen: parse_qty(frozen)?,
        in_transit: parse_qty("0")?,
        reason_type: "QA_EXACT".into(),
        ref_type: Some("QA_EXACT".into()),
        ref_id: Some(Uuid::new_v4().to_string()),
        occurred_at: Utc::now(),
    })
}

async fn stock_readback(pool: &PgPool, f: &Fixture) -> Result<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("select on_hand_micros,reserved_micros,frozen_micros,(select coalesce(sum(delta_on_hand_micros),0)::bigint from inventory_ledgers where tenant_id=$1 and warehouse_id=$2 and sku_id=$3) from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).fetch_one(pool).await?)
}

#[tokio::test]
async fn precision_manufacturing_quarter_blocks_trade_whole_unit() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    SharedContractRepo::new(&pool)
        .post_material_move(
            f.tenant,
            "qa-issue-quarter",
            "qa-issue-quarter",
            MaterialMoveInput {
                work_order_id: f.order,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
                batch_id: None,
                original_issue_id: None,
                movement: MaterialMove {
                    kind: MaterialMoveKind::Issue,
                    qty: parse_qty("0.250")?,
                    supplement_reason_required: false,
                },
                reason: None,
            },
        )
        .await?
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let trade = InventoryService::new(&pool)
        .apply(whole_delta(&f, -1))
        .await;
    assert!(
        trade.is_err(),
        "trade sold a whole unit after manufacturing already issued a quarter"
    );
    assert!(matches!(
        trade
            .err()
            .and_then(|error| error.downcast::<InventoryPrecisionError>().ok()),
        Some(InventoryPrecisionError::InsufficientAvailable {
            available_micros: 750000,
            requested_micros: 1000000,
        })
    ));
    let sales: i64 = sqlx::query_scalar(
        "select count(*) from inventory_ledgers where tenant_id=$1 and reason_type='QA_TRADE_SHIP'",
    )
    .bind(Uuid::from(f.tenant))
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(sales, 0);
    assert_eq!(stock_readback(&pool, &f).await?, (750000, 0, 0, 750000));
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "-0.750", "0", "0")?)
        .await?;
    assert_eq!(stock_readback(&pool, &f).await?, (0, 0, 0, 0));
    Ok(())
}

#[tokio::test]
async fn precision_reserved_and_frozen_are_subtracted_before_rounding() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let service = InventoryService::new(&pool);
    let stock = service
        .apply_precise(precise_delta(&f, "0.900", "0.900", "0.900")?)
        .await?;
    assert_eq!(stock.available()?.as_micros(), 100000);
    assert!(
        service
            .apply_precise(precise_delta(&f, "0", "1.000", "0")?)
            .await
            .is_err()
    );
    assert_eq!(
        stock_readback(&pool, &f).await?,
        (1900000, 900000, 900000, 1900000)
    );
    Ok(())
}

async fn atp_http(f: &Fixture, sku: Uuid, quantity: &str) -> Result<(StatusCode, Value)> {
    // Exercise the registered route. This synthetic JWT does not certify a
    // password/session login; restricted warehouse authorization has its own test.
    let jwt = JwtService::new("synthetic-atp-precision-key".into());
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        Uuid::new_v4().to_string(),
        Uuid::from(f.tenant).to_string(),
        vec!["inventory.read".into()],
        false,
        now,
        now + 300,
    ))?;
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        Arc::new(jwt),
    ));
    let response = app.oneshot(Request::builder()
        .uri(format!("/v1/tenants/{}/inventory/atp?warehouse_id={}&sku_id={sku}&request_qty={quantity}", Uuid::from(f.tenant), f.warehouse))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())?).await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text": String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA precise ATP tenant={} request={quantity} status={status} body={body}",
        Uuid::from(f.tenant)
    );
    Ok((status, body))
}

#[tokio::test]
async fn precision_atp_subtracts_fractional_reservation_and_freeze_before_projection() -> Result<()>
{
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "0.900", "0.900", "0.900")?)
        .await?;
    let before = stock_readback(&pool, &f).await?;
    let (status, body) = atp_http(&f, f.sku, "1").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["available"], 0,
        "an old whole-unit caller must not promise one unit: {body}"
    );
    assert_eq!(body["atp_qty"], 0);
    assert_eq!(body["shortage_qty"], 1);
    for (field, expected) in [
        ("on_hand", "1.9"),
        ("reserved", "0.9"),
        ("frozen", "0.9"),
        ("available", "0.1"),
        ("request_qty", "1"),
        ("atp_qty", "0.1"),
        ("shortage_qty", "0.9"),
    ] {
        assert_eq!(body["quantities"][field], expected, "{body}");
    }
    assert_eq!(
        stock_readback(&pool, &f).await?,
        before,
        "a quote must not reserve or activate inventory"
    );
    Ok(())
}

#[tokio::test]
async fn precision_atp_decimal_requests_and_invalid_inputs_do_not_write_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "0.900", "0.900", "0.900")?)
        .await?;
    let before = stock_readback(&pool, &f).await?;
    for (requested, promised, shortage) in [
        ("0.075", "0.075", "0"),
        ("0.100001", "0.1", "0.000001"),
        ("0", "0", "0"),
        ("9223372036854.775807", "0.1", "9223372036854.675807"),
    ] {
        let (status, body) = atp_http(&f, f.sku, requested).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["quantities"]["request_qty"], requested);
        assert_eq!(body["quantities"]["atp_qty"], promised);
        assert_eq!(body["quantities"]["shortage_qty"], shortage);
    }
    for invalid in ["-1", "0.0000001", "1e3", "9223372036854.775808", "garbage"] {
        let (status, body) = atp_http(&f, f.sku, invalid).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    assert_eq!(stock_readback(&pool, &f).await?, before);
    let effects: i64 =
        sqlx::query_scalar("select count(*) from inventory_ledgers where tenant_id=$1")
            .bind(Uuid::from(f.tenant))
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        effects, 2,
        "quotes and rejected query input must not create stock effects"
    );
    Ok(())
}

#[tokio::test]
async fn precision_atp_unknown_history_and_foreign_sku_are_explicit() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let (status, body) = atp_http(&f, f.sku, "1").await?;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "inventory_reconciliation_required");
    let (status, body) = atp_http(&f, Uuid::new_v4(), "1").await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "inventory_scope_denied");
    let empty_sku = Uuid::new_v4();
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-PREC-EMPTY',(select product_id from qa_parent))")
        .bind(Uuid::from(f.tenant))
        .bind(empty_sku)
        .execute(&pool)
        .await?;
    let (status, body) = atp_http(&f, empty_sku, "0.25").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["quantities"]["available"], "0");
    assert_eq!(body["quantities"]["shortage_qty"], "0.25");
    let orphan = Uuid::new_v4();
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-PREC-ORPHAN',(select product_id from qa_parent))")
        .bind(Uuid::from(f.tenant))
        .bind(orphan)
        .execute(&pool)
        .await?;
    sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,reason_type,ref_type,ref_id) values($1,$2,$3,1,'QA_ORPHAN','QA_ORPHAN',$4)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(orphan).bind(Uuid::new_v4().to_string()).execute(&pool).await?;
    let (status, body) = atp_http(&f, orphan, "1").await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a missing balance with posted history cannot become a known zero: {body}"
    );
    assert_eq!(body["code"], "inventory_reconciliation_required");
    let facts: (Option<i64>, i64, i64) = sqlx::query_as("select on_hand_micros,(select count(*) from inventory_ledgers where tenant_id=$1),(select count(*) from inventory_balances where tenant_id=$1) from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).fetch_one(&pool).await?;
    assert_eq!(
        facts,
        (None, 2, 1),
        "GET must neither activate history nor create an empty key"
    );
    Ok(())
}

async fn atp_batch(
    pool: &PgPool,
    f: &Fixture,
    name: &str,
    expiry_days: Option<i64>,
    hand: Option<i64>,
    held: Option<i64>,
) -> Result<Uuid> {
    let batch = Uuid::new_v4();
    let expiry = expiry_days.map(|days| Utc::now() + chrono::Duration::days(days));
    sqlx::query("insert into sku_batches(tenant_id,sku_id,batch_id,batch_no,expire_at) values($1,$2,$3,$4,$5)")
        .bind(Uuid::from(f.tenant)).bind(f.sku).bind(batch).bind(name).bind(expiry).execute(pool).await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand,on_hand_micros,reserved_micros) values($1,$2,$3,$4,1,$5,$6)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(batch).bind(hand).bind(held).execute(pool).await?;
    Ok(batch)
}

#[tokio::test]
async fn precision_atp_fefo_preserves_fractions_and_excludes_expired_batches() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "0.900", "0.900", "0.900")?)
        .await?;
    sqlx::query("update skus set batch_enabled=true where tenant_id=$1 and sku_id=$2")
        .bind(Uuid::from(f.tenant))
        .bind(f.sku)
        .execute(&pool)
        .await?;
    let expired = atp_batch(&pool, &f, "EXPIRED", Some(-1), Some(1_600_000), Some(0)).await?;
    let early = atp_batch(&pool, &f, "EARLY", Some(1), Some(250_000), Some(200_000)).await?;
    let late = atp_batch(&pool, &f, "NO-EXPIRY", None, Some(50_000), Some(0)).await?;
    let before = stock_readback(&pool, &f).await?;
    let (status, body) = atp_http(&f, f.sku, "0.075").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["quantities"]["available"], "0.1");
    assert_eq!(body["quantities"]["atp_qty"], "0.075");
    assert_eq!(body["quantities"]["shortage_qty"], "0");
    let allocations = body["allocations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("{body}"))?;
    assert_eq!(allocations.len(), 2);
    assert_eq!(allocations[0]["batch_id"], early.to_string());
    assert_eq!(allocations[0]["quantity"], "0.05");
    assert_eq!(allocations[1]["batch_id"], late.to_string());
    assert_eq!(allocations[1]["quantity"], "0.025");
    assert!(
        allocations
            .iter()
            .all(|row| row["batch_id"] != expired.to_string())
    );
    assert_eq!(stock_readback(&pool, &f).await?, before);
    eprintln!(
        "QA ATP batches tenant={} expired={expired} early={early} late={late}",
        Uuid::from(f.tenant)
    );
    Ok(())
}

#[tokio::test]
async fn precision_atp_batch_missing_precision_and_mismatch_require_reconciliation() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "0", "0", "0")?)
        .await?;
    sqlx::query("update skus set batch_enabled=true where tenant_id=$1 and sku_id=$2")
        .bind(Uuid::from(f.tenant))
        .bind(f.sku)
        .execute(&pool)
        .await?;
    let batch = atp_batch(&pool, &f, "LEGACY", None, None, None).await?;
    let (status, body) = atp_http(&f, f.sku, "1").await?;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "inventory_reconciliation_required");
    let unactivated: (Option<i64>, Option<i64>) = sqlx::query_as("select on_hand_micros,reserved_micros from inventory_batch_balances where tenant_id=$1 and batch_id=$2")
        .bind(Uuid::from(f.tenant)).bind(batch).fetch_one(&pool).await?;
    assert_eq!(unactivated, (None, None));
    // Explicit QA fixture preparation, never performed by the GET route.
    sqlx::query("update inventory_batch_balances set on_hand_micros=1000000,reserved_micros=0 where tenant_id=$1 and batch_id=$2")
        .bind(Uuid::from(f.tenant)).bind(batch).execute(&pool).await?;
    let (status, body) = atp_http(&f, f.sku, "1").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    atp_batch(&pool, &f, "EXTRA", None, Some(i64::MAX), Some(0)).await?;
    let (status, body) = atp_http(&f, f.sku, "1").await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "wide batch sums must reject inconsistent facts without overflow: {body}"
    );
    assert_eq!(body["code"], "inventory_reconciliation_required");
    assert_eq!(
        stock_readback(&pool, &f).await?,
        (1_000_000, 0, 0, 1_000_000)
    );
    Ok(())
}

#[tokio::test]
async fn precision_atp_batch_budget_rejects_without_silent_truncation() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "1000", "0", "0")?)
        .await?;
    sqlx::query("update skus set batch_enabled=true where tenant_id=$1 and sku_id=$2")
        .bind(Uuid::from(f.tenant))
        .bind(f.sku)
        .execute(&pool)
        .await?;
    sqlx::query("insert into sku_batches(tenant_id,sku_id,batch_id,batch_no) select $1,$2,gen_random_uuid(),'B-'||i::text from generate_series(1,1001) i")
        .bind(Uuid::from(f.tenant)).bind(f.sku).execute(&pool).await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros) select $1,$2,$3,batch_id,1000000,0 from sku_batches where tenant_id=$1 and sku_id=$3")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).execute(&pool).await?;
    let (status, body) = atp_http(&f, f.sku, "1").await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "inventory_atp_batch_limit");
    let facts: (i64, i64, i64) = sqlx::query_as("select count(*),sum(on_hand_micros)::bigint,(select count(*) from inventory_ledgers where tenant_id=$1) from inventory_batch_balances where tenant_id=$1")
        .bind(Uuid::from(f.tenant)).fetch_one(&pool).await?;
    assert_eq!(facts, (1001, 1_001_000_000, 2));
    assert_eq!(
        stock_readback(&pool, &f).await?,
        (1_001_000_000, 0, 0, 1_001_000_000)
    );
    Ok(())
}

#[tokio::test]
async fn precision_http_whole_outbound_reports_conflict_without_stock_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "0.900", "0.900", "0.900")?)
        .await?;
    let mut reader = pool.acquire().await?;
    // This exercises the actual protected route with a synthetic bearer token.
    // It does not certify real password sessions or restricted warehouse scopes.
    let jwt = JwtService::new("synthetic-inventory-precision-key".into());
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        Uuid::new_v4().to_string(),
        Uuid::from(f.tenant).to_string(),
        vec!["inventory.write".into()],
        false,
        now,
        now + 300,
    ))?;
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        Arc::new(jwt),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/v1/tenants/{}/inventory/outbounds",
                    Uuid::from(f.tenant)
                ))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "warehouse_id": f.warehouse, "sku_id": f.sku, "quantity": 1
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let body: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(body["code"], "inventory_insufficient");
    let effects: i64 = sqlx::query_scalar(
        "select count(*) from inventory_ledgers where tenant_id=$1 and reason_type='TRANSFER_OUT'",
    )
    .bind(Uuid::from(f.tenant))
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(effects, 0);
    assert_eq!(
        stock_readback(&pool, &f).await?,
        (1900000, 900000, 900000, 1900000)
    );
    Ok(())
}

#[tokio::test]
async fn precision_direct_writers_cannot_bypass_sources_or_rewrite_history() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let foreign = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "-0.250", "0", "0")?)
        .await?;
    for (statement, expected) in [
        (
            "update inventory_balances set on_hand=2 where tenant_id=$1",
            "integer-only inventory write",
        ),
        (
            "update mfg_qty_balances set on_hand_micros=2000000 where tenant_id=$1",
            "manufacturing shadow balance",
        ),
        (
            "update inventory_ledgers set reason_type='QA_REWRITE' where tenant_id=$1",
            "source facts are immutable",
        ),
        (
            "update inventory_ledgers set delta_on_hand_micros=delta_on_hand_micros+1 where tenant_id=$1",
            "exact quantities are immutable",
        ),
        (
            "update inventory_balances set frozen_micros=-1 where tenant_id=$1",
            "negative physical",
        ),
        (
            "update inventory_balances set on_hand_micros=null,reserved_micros=null,frozen_micros=null,in_transit_micros=null where tenant_id=$1",
            "cannot return to an integer-only",
        ),
        (
            "insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros) select tenant_id,$1,warehouse_id,$1,sku_id,'','GOOD','ON_HAND',-750000 from inventory_balances where tenant_id=$1",
            "canonical precision service",
        ),
        (
            "insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) select tenant_id,warehouse_id,sku_id,$1,250000,'QA_OPENING' from inventory_balances where tenant_id=$1",
            "original exact inventory effect",
        ),
    ] {
        let error = sqlx::query(statement)
            .bind(Uuid::from(f.tenant))
            .execute(&pool)
            .await
            .expect_err("unsafe direct writer was accepted");
        assert!(
            error
                .as_database_error()
                .is_some_and(|value| value.message().contains(expected)),
            "unexpected guard: {error}"
        );
    }
    let error = sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,reason_type) values($1,$2,$3,1,0,0,0,'QA_FOREIGN')")
        .bind(Uuid::from(f.tenant)).bind(foreign.warehouse).bind(f.sku)
        .execute(&pool).await.expect_err("foreign warehouse source was accepted");
    assert!(
        error
            .as_database_error()
            .is_some_and(|value| value.message().contains("outside the enterprise"))
    );
    SharedContractRepo::new(&pool)
        .post_manufacturing_reservation(
            f.tenant,
            "guard-reservation",
            "guard-reservation",
            ManufacturingReservationInput {
                work_order_id: f.order,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
                qty: parse_qty("0.500")?,
            },
        )
        .await?
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    for (statement, expected) in [
        (
            "delete from inventory_ledgers where tenant_id=$1",
            "inventory ledger facts are immutable",
        ),
        (
            "update inventory_reservations set original_qty_micros=600000 where tenant_id=$1",
            "original reservation quantity is immutable",
        ),
        (
            "update inventory_reservations set qty_micros=600000 where tenant_id=$1",
            "exceeds its original quantity",
        ),
        (
            "update inventory_reservations set qty_micros=-1 where tenant_id=$1",
            "negative exact source reservation",
        ),
    ] {
        let error = sqlx::query(statement)
            .bind(Uuid::from(f.tenant))
            .execute(&pool)
            .await
            .expect_err("source reservation was rewritten");
        assert!(
            error
                .as_database_error()
                .is_some_and(|value| value.message().contains(expected)),
            "unexpected reservation guard: {error}"
        );
    }
    let mut reader = pool.acquire().await?;
    let facts: (i64, i64, i64, i64) = sqlx::query_as(
        "select (select count(*) from inventory_ledgers where tenant_id=$1),
          (select count(*) from shared_stock_ledgers where tenant_id=$1),
          (select qty_micros from inventory_reservations where tenant_id=$1),
          (select original_qty_micros from inventory_reservations where tenant_id=$1)",
    )
    .bind(Uuid::from(f.tenant))
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(facts, (3, 0, 500000, 500000));
    assert_eq!(
        stock_readback(&pool, &f).await?,
        (750000, 500000, 0, 750000)
    );
    Ok(())
}

#[tokio::test]
async fn precision_half_reserved_keeps_a_whole_trade_sale_available() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "0.500", "0.500", "0")?)
        .await?;
    let other = Uuid::new_v4();
    let own = Uuid::new_v4();
    sqlx::query("insert into inventory_reservations(tenant_id,warehouse_id,sku_id,order_item_id,qty,qty_micros,status,source_type,source_id) values($1,$2,$3,$4,0,500000,'RESERVED','QA_OTHER','qa-other')")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(other).execute(&pool).await?;
    let repo = InventoryRepo::new(&pool);
    repo.reserve_for_order_item(InventoryReservationInput {
        tenant_id: f.tenant,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        order_id: None,
        order_item_id: own,
        qty: 1,
    })
    .await?;
    repo.consume_reservation(InventoryReservationAction {
        tenant_id: f.tenant,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        order_item_id: own,
        qty: 1,
    })
    .await?;
    assert_eq!(
        stock_readback(&pool, &f).await?,
        (500000, 500000, 0, 500000)
    );
    let other_qty:(i64,String)=sqlx::query_as("select qty_micros,status from inventory_reservations where tenant_id=$1 and order_item_id=$2")
        .bind(Uuid::from(f.tenant)).bind(other).fetch_one(&pool).await?;
    assert_eq!(other_qty, (500000, "RESERVED".into()));
    Ok(())
}

#[tokio::test]
async fn precision_fractional_batch_issue_and_return_share_physical_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let batch = Uuid::new_v4();
    sqlx::query(
        "insert into sku_batches(tenant_id,sku_id,batch_id,batch_no) values($1,$2,$3,'QA-PREC-B')",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.sku)
    .bind(batch)
    .execute(&pool)
    .await?;
    // This success fixture explicitly declares an exact opening, never unknown history.
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros) values($1,$2,$3,$4,1000000,0)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(batch).execute(&pool).await?;
    let repo = SharedContractRepo::new(&pool);
    let mut input = MaterialMoveInput {
        work_order_id: f.order,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        batch_id: Some(batch),
        original_issue_id: None,
        movement: MaterialMove {
            kind: MaterialMoveKind::Issue,
            qty: parse_qty("0.250")?,
            supplement_reason_required: false,
        },
        reason: None,
    };
    let issue = repo
        .post_material_move(f.tenant, "batch-quarter", "batch-quarter", input)
        .await?
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    input.original_issue_id = Some(issue.move_id);
    input.movement.kind = MaterialMoveKind::Return;
    input.movement.qty = parse_qty("0.125")?;
    repo.post_material_move(f.tenant, "batch-return", "batch-return", input)
        .await?
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    assert_eq!(stock_readback(&pool, &f).await?, (875000, 0, 0, 875000));
    let amount: i64 = sqlx::query_scalar(
        "select on_hand_micros from inventory_batch_balances where tenant_id=$1 and batch_id=$2",
    )
    .bind(Uuid::from(f.tenant))
    .bind(batch)
    .fetch_one(&pool)
    .await?;
    assert_eq!(amount, 875000);
    assert!(
        InventoryService::new(&pool)
            .apply(whole_delta(&f, -1))
            .await
            .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn precision_unmirrored_manufacturing_history_requires_reconciliation() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    sqlx::query("update mfg_qty_balances set on_hand_micros=750000 where tenant_id=$1")
        .bind(Uuid::from(f.tenant))
        .execute(&pool)
        .await?;
    sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,-250000,'MFG_ISSUE')")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(Uuid::new_v4()).execute(&pool).await?;
    let error = InventoryService::new(&pool)
        .apply(whole_delta(&f, -1))
        .await
        .expect_err("unmapped history must not activate");
    assert!(error.to_string().contains("reconciliation required"));
    let values:(i64,Option<i64>,i64,i64)=sqlx::query_as("select on_hand,on_hand_micros,(select count(*) from inventory_quantity_activations where tenant_id=$1),(select count(*) from inventory_ledgers where tenant_id=$1) from inventory_balances where tenant_id=$1")
        .bind(Uuid::from(f.tenant)).fetch_one(&pool).await?;
    assert_eq!(values, (1, None, 0, 1));
    Ok(())
}

#[tokio::test]
async fn precision_mirrored_whole_history_is_mapped_without_double_counting() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let movement = Uuid::new_v4();
    sqlx::query("update inventory_balances set on_hand=0 where tenant_id=$1")
        .bind(Uuid::from(f.tenant))
        .execute(&pool)
        .await?;
    sqlx::query("update mfg_qty_balances set on_hand_micros=0 where tenant_id=$1")
        .bind(Uuid::from(f.tenant))
        .execute(&pool)
        .await?;
    sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,reason_type,ref_type,ref_id) values($1,$2,$3,-1,0,0,0,'MFG_ISSUE','mfg_material_move',$4)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(movement.to_string()).execute(&pool).await?;
    sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,-1000000,'MFG_ISSUE')")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(movement).execute(&pool).await?;
    InventoryService::new(&pool)
        .apply(whole_delta(&f, 1))
        .await?;
    assert_eq!(stock_readback(&pool, &f).await?, (1000000, 0, 0, 1000000));
    let maps:(i64,i64)=sqlx::query_as("select count(*),coalesce(sum(qty_micros),0)::bigint from inventory_quantity_source_maps where tenant_id=$1")
        .bind(Uuid::from(f.tenant)).fetch_one(&pool).await?;
    assert_eq!(maps, (2, 0));
    Ok(())
}

#[tokio::test]
async fn precision_twenty_fractional_outgoings_never_overdraw_one_balance() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(precise_delta(&f, "-0.500", "0", "0")?)
        .await?;
    let mut reader = pool.acquire().await?;
    let mut blocker = pool.begin().await?;
    let blocker_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("select on_hand_micros from inventory_balances where tenant_id=$1 for update")
        .bind(Uuid::from(f.tenant))
        .execute(&mut *blocker)
        .await?;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let pool = pool.clone();
        let input = precise_delta(&f, "-0.400", "0", "0")?;
        tasks.spawn(async move { InventoryService::new(&pool).apply_precise(input).await });
    }
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut waiters = 0_i64;
    while waiters < 2 && tokio::time::Instant::now() < deadline {
        waiters = sqlx::query_scalar(
            "with recursive blocked(pid) as (
               select pid from pg_stat_activity where $1=any(pg_blocking_pids(pid))
               union
               select activity.pid from pg_stat_activity activity
                 join blocked parent on parent.pid=any(pg_blocking_pids(activity.pid))
             ) select count(*) from blocked",
        )
        .bind(blocker_pid)
        .fetch_one(&mut *reader)
        .await?;
        if waiters < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    blocker.rollback().await?;
    assert_eq!(waiters, 2);
    let mut accepted = 0;
    let mut rejected = 0;
    while let Some(result) = tasks.join_next().await {
        match result? {
            Ok(_) => accepted += 1,
            Err(error) => {
                assert!(
                    matches!(
                        error.downcast_ref::<InventoryPrecisionError>(),
                        Some(InventoryPrecisionError::InsufficientAvailable {
                            available_micros: 100000,
                            requested_micros: 400000,
                        })
                    ),
                    "unexpected competing writer failure: {error}"
                );
                rejected += 1;
            }
        }
    }
    assert_eq!(accepted, 1);
    assert_eq!(rejected, 19);
    eprintln!(
        "QA precision contention tenant={} blocker_pid={blocker_pid} blocked_chain={waiters} accepted={accepted} rejected={rejected}",
        Uuid::from(f.tenant),
    );
    drop(reader);
    assert_eq!(stock_readback(&pool, &f).await?, (100000, 0, 0, 100000));
    Ok(())
}

#[tokio::test]
async fn precision_manufacturing_source_reservation_replays_original_quantity_after_spending()
-> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let other_order = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-OTHER-WO','QA-PREC','RELEASED',1000000)")
        .bind(Uuid::from(f.tenant)).bind(other_order).execute(&pool).await?;
    let repo = SharedContractRepo::new(&pool);
    repo.post_manufacturing_reservation(
        f.tenant,
        "other-reservation",
        "opaque-other",
        ManufacturingReservationInput {
            work_order_id: other_order,
            warehouse_id: f.warehouse,
            sku_id: f.sku,
            qty: parse_qty("0.200")?,
        },
    )
    .await?
    .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let input = ManufacturingReservationInput {
        work_order_id: f.order,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        qty: parse_qty("0.500")?,
    };
    let saved = repo
        .post_manufacturing_reservation(f.tenant, "own-reservation", "opaque-own", input)
        .await?
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
    let original_id = match saved {
        SharedWrite::Stored(receipt) => receipt.effect_id,
        SharedWrite::Replay(_) => anyhow::bail!("fresh reservation unexpectedly replayed"),
    };
    for command in ["issue-first-quarter", "issue-second-quarter"] {
        repo.post_material_move(
            f.tenant,
            command,
            command,
            MaterialMoveInput {
                work_order_id: f.order,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
                batch_id: None,
                original_issue_id: None,
                movement: MaterialMove {
                    kind: MaterialMoveKind::Issue,
                    qty: parse_qty("0.250")?,
                    supplement_reason_required: false,
                },
                reason: None,
            },
        )
        .await?
        .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        let replay = repo
            .post_manufacturing_reservation(f.tenant, "own-reservation", "opaque-own", input)
            .await?
            .map_err(|error| anyhow::anyhow!("{error:?}"))?;
        assert!(matches!(replay, SharedWrite::Replay(receipt) if receipt.effect_id == original_id));
    }
    assert!(
        repo.post_manufacturing_reservation(
            f.tenant,
            "own-reservation",
            "opaque-own",
            ManufacturingReservationInput {
                qty: parse_qty("0.600")?,
                ..input
            }
        )
        .await?
        .is_err()
    );
    let mut reader = pool.acquire().await?;
    let sources: Vec<(String, i64, i64, String)> = sqlx::query_as("select source_id,qty_micros,original_qty_micros,status from inventory_reservations where tenant_id=$1 order by source_id")
        .bind(Uuid::from(f.tenant)).fetch_all(&mut *reader).await?;
    assert_eq!(sources.len(), 2);
    assert!(sources.contains(&(f.order.to_string(), 0, 500000, "CONSUMED".into())));
    assert!(sources.contains(&(other_order.to_string(), 200000, 200000, "RESERVED".into())));
    let effects: (i64, i64) = sqlx::query_as("select (select count(*) from inventory_ledgers where tenant_id=$1 and reason_type='MFG_RESERVATION'),(select count(*) from command_receipts where tenant_id=$1)")
        .bind(Uuid::from(f.tenant)).fetch_one(&mut *reader).await?;
    assert_eq!(effects, (2, 4));
    assert_eq!(
        stock_readback(&pool, &f).await?,
        (500000, 200000, 0, 500000)
    );
    Ok(())
}

#[tokio::test]
async fn precision_unknown_offsetting_manufacturing_history_still_requires_sources() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    for quantity in [-250000_i64, 250000] {
        sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,$5,'UNRECOGNIZED_PHYSICAL')")
            .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).bind(Uuid::new_v4()).bind(quantity).execute(&pool).await?;
    }
    let error = InventoryService::new(&pool)
        .apply(whole_delta(&f, -1))
        .await
        .expect_err("matching balances cannot prove two unlinked physical effects");
    assert!(matches!(
        error.downcast_ref::<InventoryPrecisionError>(),
        Some(InventoryPrecisionError::ReconciliationRequired(_))
    ));
    let mut reader = pool.acquire().await?;
    let facts: (i64, Option<i64>, i64, i64) = sqlx::query_as("select on_hand,on_hand_micros,(select count(*) from inventory_quantity_activations where tenant_id=$1),(select count(*) from inventory_ledgers where tenant_id=$1) from inventory_balances where tenant_id=$1")
        .bind(Uuid::from(f.tenant)).fetch_one(&mut *reader).await?;
    assert_eq!(facts, (1, None, 0, 1));
    Ok(())
}

#[tokio::test]
async fn precision_qa_named_opening_without_proven_source_is_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    for delta in [-250000_i64, 250000] {
        sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,$5,'QA_OPENING')")
            .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku)
            .bind(Uuid::new_v4()).bind(delta).execute(&pool).await?;
    }
    let error = InventoryService::new(&pool)
        .apply(whole_delta(&f, -1))
        .await
        .expect_err("a QA label bypassed historical source proof");
    assert!(matches!(
        error.downcast_ref::<InventoryPrecisionError>(),
        Some(InventoryPrecisionError::ReconciliationRequired(_))
    ));
    let mut reader = pool.acquire().await?;
    let facts: (i64, Option<i64>, i64, i64) = sqlx::query_as(
        "select on_hand,on_hand_micros,
          (select count(*) from inventory_quantity_activations where tenant_id=$1),
          (select count(*) from inventory_ledgers where tenant_id=$1)
         from inventory_balances where tenant_id=$1",
    )
    .bind(Uuid::from(f.tenant))
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(facts, (1, None, 0, 1));
    Ok(())
}

#[tokio::test]
async fn precision_proven_fractional_history_activates_without_rounding_away_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    for (increments, expected_before, expected_after) in [
        (vec![250000_i64], 1250000_i64, 750000_i64),
        (vec![750000_i64, 750000], 2500000_i64, 2000000_i64),
    ] {
        let f = fixture(&pool).await?;
        for delta in increments {
            let movement = Uuid::new_v4();
            sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,ref_type,ref_id) values($1,$2,$3,0,0,0,0,$4,0,0,0,'MFG_RECEIPT','mfg_material_move',$5)")
                .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku)
                .bind(delta).bind(movement.to_string()).execute(&pool).await?;
            sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,$5,'MFG_RECEIPT')")
                .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku)
                .bind(movement).bind(delta).execute(&pool).await?;
        }
        sqlx::query("update inventory_balances set on_hand=$2 where tenant_id=$1")
            .bind(Uuid::from(f.tenant))
            .bind(expected_before / 1000000)
            .execute(&pool)
            .await?;
        sqlx::query("update mfg_qty_balances set on_hand_micros=$2 where tenant_id=$1")
            .bind(Uuid::from(f.tenant))
            .bind(expected_before)
            .execute(&pool)
            .await?;
        InventoryService::new(&pool)
            .apply_precise(precise_delta(&f, "-0.500", "0", "0")?)
            .await?;
        assert_eq!(
            stock_readback(&pool, &f).await?,
            (expected_after, 0, 0, expected_after)
        );
        let mut reader = pool.acquire().await?;
        let expected_sources = if expected_before == 1250000 {
            2_i64
        } else {
            3_i64
        };
        let maps: i64 = sqlx::query_scalar(
            "select count(*) from inventory_quantity_source_maps where tenant_id=$1",
        )
        .bind(Uuid::from(f.tenant))
        .fetch_one(&mut *reader)
        .await?;
        assert_eq!(maps, expected_sources);
    }
    Ok(())
}

#[tokio::test]
async fn precision_unproven_shared_history_requires_reconciliation() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros) values($1,$2,$3,$4,$5,'','GOOD','ON_HAND',-500000)")
        .bind(Uuid::from(f.tenant)).bind(Uuid::new_v4()).bind(f.warehouse)
        .bind(Uuid::new_v4()).bind(f.sku).execute(&pool).await?;
    let error = InventoryService::new(&pool)
        .apply(whole_delta(&f, -1))
        .await
        .expect_err("an unmirrored shared-stock history was ignored");
    assert!(matches!(
        error.downcast_ref::<InventoryPrecisionError>(),
        Some(InventoryPrecisionError::ReconciliationRequired(_))
    ));
    let mut reader = pool.acquire().await?;
    let facts: (Option<i64>, i64, i64, i64) = sqlx::query_as(
        "select on_hand_micros,(select count(*) from inventory_quantity_activations where tenant_id=$1),
         (select count(*) from inventory_ledgers where tenant_id=$1),
         (select count(*) from shared_stock_ledgers where tenant_id=$1)
         from inventory_balances where tenant_id=$1",
    ).bind(Uuid::from(f.tenant)).fetch_one(&mut *reader).await?;
    assert_eq!(facts, (None, 0, 1, 1));
    Ok(())
}

#[tokio::test]
async fn precision_shared_writer_waits_for_activation_then_cannot_bypass_it() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let mut canonical = pool.begin().await?;
    let blocker: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *canonical)
        .await?;
    InventoryService::new(&pool)
        .apply_precise_with_tx(&mut canonical, precise_delta(&f, "-0.250", "0", "0")?)
        .await?;
    let writer_pool = pool.clone();
    let tenant = Uuid::from(f.tenant);
    let warehouse = f.warehouse;
    let sku = f.sku;
    let writer = tokio::spawn(async move {
        sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros) values($1,$2,$3,$4,$5,'','GOOD','ON_HAND',-500000)")
            .bind(tenant).bind(Uuid::new_v4()).bind(warehouse)
            .bind(Uuid::new_v4()).bind(sku).execute(&writer_pool).await
    });
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut waiters = 0_i64;
    while waiters == 0 && tokio::time::Instant::now() < deadline {
        waiters = sqlx::query_scalar(
            "select count(*) from pg_stat_activity where $1=any(pg_blocking_pids(pid))",
        )
        .bind(blocker)
        .fetch_one(&mut *reader)
        .await?;
        if waiters == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    canonical.commit().await?;
    let result = writer.await?;
    assert_eq!(
        waiters, 1,
        "shared writer skipped the in-flight canonical activation"
    );
    let error = result.expect_err("legacy writer bypassed an activated stock key");
    assert!(
        error
            .as_database_error()
            .is_some_and(|value| value.message().contains("canonical precision service"))
    );
    eprintln!("QA precision shared fence tenant={tenant} blocked={waiters} bypass_effects=0");
    let facts: i64 =
        sqlx::query_scalar("select count(*) from shared_stock_ledgers where tenant_id=$1")
            .bind(tenant)
            .fetch_one(&mut *reader)
            .await?;
    assert_eq!(facts, 0);
    assert_eq!(stock_readback(&pool, &f).await?, (750000, 0, 0, 750000));
    Ok(())
}
