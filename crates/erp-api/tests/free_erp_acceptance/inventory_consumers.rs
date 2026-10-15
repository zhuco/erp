//! Remaining inventory readers and legacy write receipts use the exact stock.
use super::{Result, isolated_pool};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_domain::quantity::parse_qty;
use erp_repo::{
    db::Db,
    inventory::{InventoryDelta, InventoryRepo, InventoryService},
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use erp_types::TenantId;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    tenant: TenantId,
    actor: Uuid,
    warehouse: Uuid,
    denied: Uuid,
    sku: Uuid,
}

async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let f = Fixture {
        tenant: TenantId(Uuid::new_v4()),
        actor: Uuid::new_v4(),
        warehouse: Uuid::new_v4(),
        denied: Uuid::new_v4(),
        sku: Uuid::new_v4(),
    };
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-INVENTORY-CONSUMERS')")
        .bind(Uuid::from(f.tenant))
        .execute(pool)
        .await?;
    sqlx::query("insert into users(tenant_id,user_id,email,password_hash) values($1,$2,$3,'QA-synthetic-no-login')")
        .bind(Uuid::from(f.tenant)).bind(f.actor).bind(format!("{}@consumers.qa.invalid",f.actor)).execute(pool).await?;
    let product = Uuid::new_v4();
    sqlx::query(
        "insert into products(tenant_id,product_id,name) values($1,$2,'Inventory consumer')",
    )
    .bind(Uuid::from(f.tenant))
    .bind(product)
    .execute(pool)
    .await?;
    sqlx::query(
        "insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-CONSUMER',$3)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.sku)
    .bind(product)
    .execute(pool)
    .await?;
    for warehouse in [f.warehouse, f.denied] {
        sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Consumer QA',$3)")
            .bind(Uuid::from(f.tenant)).bind(warehouse).bind(warehouse.to_string()).execute(pool).await?;
        InventoryService::new(pool)
            .apply_precise(InventoryPreciseDelta {
                key: InventoryQuantityKey {
                    tenant_id: f.tenant,
                    warehouse_id: warehouse,
                    sku_id: f.sku,
                },
                on_hand: parse_qty("1.9")?,
                reserved: parse_qty("0.9")?,
                frozen: parse_qty("0.9")?,
                in_transit: parse_qty("2.3")?,
                reason_type: "QA_CONSUMER_OPENING".into(),
                ref_type: Some("QA_CONSUMER_OPENING".into()),
                ref_id: Some(Uuid::new_v4().to_string()),
                occurred_at: Utc::now(),
            })
            .await?;
    }
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.actor)
    .bind(f.warehouse)
    .execute(pool)
    .await?;
    eprintln!(
        "QA inventory consumer fixture tenant={} actor={} warehouse={} denied={} sku={}",
        f.tenant, f.actor, f.warehouse, f.denied, f.sku
    );
    Ok(f)
}

async fn facts(pool: &PgPool, f: &Fixture) -> Result<String> {
    Ok(sqlx::query_scalar("select jsonb_build_object('balances',coalesce((select jsonb_agg(to_jsonb(b) order by warehouse_id,sku_id) from inventory_balances b where tenant_id=$1),'[]'::jsonb),'ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from inventory_ledgers l where tenant_id=$1),'[]'::jsonb),'outbox',coalesce((select jsonb_agg(to_jsonb(o) order by outbox_id) from outbox_events o where tenant_id=$1),'[]'::jsonb))::text")
        .bind(Uuid::from(f.tenant)).fetch_one(pool).await?)
}

async fn http(pool: &PgPool, f: &Fixture, path: &str) -> Result<(StatusCode, Value)> {
    let audit = isolated_pool().await?;
    let before = facts(&audit, f).await?;
    let (status, body) = request(
        f,
        path,
        vec!["inventory.read".into(), "reports.read".into()],
        None,
    )
    .await?;
    assert_eq!(
        facts(&audit, f).await?,
        before,
        "read must preserve original stock facts"
    );
    assert_eq!(facts(pool, f).await?, before, "separate connections agree");
    Ok((status, body))
}

async fn request(
    f: &Fixture,
    path: &str,
    perms: Vec<String>,
    body: Option<Value>,
) -> Result<(StatusCode, Value)> {
    let jwt = JwtService::new("synthetic-inventory-consumer-key".into());
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        f.actor.to_string(),
        f.tenant.to_string(),
        perms,
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
                .uri(path)
                .method(if body.is_some() { "POST" } else { "GET" })
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA inventory consumer HTTP tenant={} path={path} status={status} body={body}",
        f.tenant
    );
    Ok((status, body))
}

#[tokio::test]
async fn consumer_turnover_excludes_warehouses_outside_the_actor_grant() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let (status, body) = http(&pool, &f, "/reports/inventory?warning_threshold=0").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["total"], 1,
        "report must not expose an ungranted warehouse: {body}"
    );
    assert_eq!(body["items"][0]["warehouse_id"], f.warehouse.to_string());
    let (status, body) = http(
        &pool,
        &f,
        &format!("/reports/inventory?warehouse_id={}", f.denied),
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    Ok(())
}

#[tokio::test]
async fn consumer_turnover_uses_exact_available_and_warning_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let (status, body) = http(
        &pool,
        &f,
        &format!(
            "/reports/inventory?warehouse_id={}&warning_threshold=0",
            f.warehouse
        ),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["items"][0]["available"], 0,
        "legacy projection cannot overpromise: {body}"
    );
    assert_eq!(body["items"][0]["quantities"]["available"], "0.1", "{body}");
    assert_eq!(
        body["items"][0]["warning"], false,
        "positive exact stock is above zero"
    );
    assert_eq!(body["items"][0]["out_of_stock"], false);
    Ok(())
}

#[tokio::test]
async fn consumer_legacy_single_balance_projects_after_exact_subtraction() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let audit = isolated_pool().await?;
    let before = facts(&audit, &f).await?;
    let balance = InventoryRepo::new(&pool)
        .get_balance(f.tenant, f.warehouse, f.sku)
        .await?
        .ok_or_else(|| anyhow::anyhow!("missing fixture balance"))?;
    let body = serde_json::to_value(&balance)?;
    eprintln!(
        "QA inventory consumer balance tenant={} available={} body={body}",
        f.tenant,
        balance.available()
    );
    assert_eq!(
        balance.available(),
        0,
        "precise available is 0.1, not one unit"
    );
    assert_eq!(body["quantities"]["available"], "0.1");
    assert_eq!(facts(&audit, &f).await?, before);
    Ok(())
}

async fn reserve_ready(pool: &PgPool, f: &Fixture) -> Result<Uuid> {
    InventoryService::new(pool)
        .apply(InventoryDelta {
            tenant_id: f.tenant,
            warehouse_id: f.warehouse,
            sku_id: f.sku,
            delta_on_hand: 2,
            delta_reserved: 0,
            delta_frozen: 0,
            delta_in_transit: 0,
            reason_type: "QA_CONSUMER_RESERVATION_STOCK".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;
    let item = Uuid::new_v4();
    let input = erp_repo::inventory::InventoryReservationInput {
        tenant_id: f.tenant,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        order_id: None,
        order_item_id: item,
        qty: 1,
    };
    let repo = InventoryRepo::new(pool);
    let first = repo.reserve_for_order_item(input.clone()).await?;
    assert_eq!(first.available(), 1);
    assert_eq!(first.quantities.available, "1.1");
    let before = facts(pool, f).await?;
    let retry = repo.reserve_for_order_item(input).await?;
    assert_eq!(retry.quantities.available, "1.1");
    assert_eq!(retry.available(), 1);
    assert_eq!(
        facts(pool, f).await?,
        before,
        "reserve replay adds no stock effect"
    );
    Ok(item)
}

#[tokio::test]
async fn consumer_legacy_reservation_release_and_replay_keep_exact_balance() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let item = reserve_ready(&pool, &f).await?;
    let action = erp_repo::inventory::InventoryReservationAction {
        tenant_id: f.tenant,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        order_item_id: item,
        qty: 1,
    };
    let repo = InventoryRepo::new(&pool);
    let first = repo.release_reservation(action.clone()).await?;
    assert_eq!(first.available(), 2);
    assert_eq!(first.quantities.available, "2.1");
    let before = facts(&pool, &f).await?;
    let replay = repo.release_reservation(action).await?;
    assert_eq!(replay.available(), 2);
    assert_eq!(replay.quantities.available, "2.1");
    assert_eq!(facts(&pool, &f).await?, before);
    eprintln!(
        "QA inventory consumer reservation tenant={} action=release body={}",
        f.tenant,
        serde_json::to_value(replay)?
    );
    Ok(())
}

#[tokio::test]
async fn consumer_legacy_reservation_consume_and_replay_keep_exact_balance() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let item = reserve_ready(&pool, &f).await?;
    let action = erp_repo::inventory::InventoryReservationAction {
        tenant_id: f.tenant,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        order_item_id: item,
        qty: 1,
    };
    let repo = InventoryRepo::new(&pool);
    let first = repo.consume_reservation(action.clone()).await?;
    assert_eq!(first.available(), 1);
    assert_eq!(first.quantities.on_hand, "2.9");
    assert_eq!(first.quantities.available, "1.1");
    let before = facts(&pool, &f).await?;
    let replay = repo.consume_reservation(action).await?;
    assert_eq!(replay.available(), 1);
    assert_eq!(replay.quantities.available, "1.1");
    assert_eq!(facts(&pool, &f).await?, before);
    eprintln!(
        "QA inventory consumer reservation tenant={} action=consume body={}",
        f.tenant,
        serde_json::to_value(replay)?
    );
    Ok(())
}

#[tokio::test]
async fn consumer_legacy_reservation_terminal_replay_validates_original_identity() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let item = reserve_ready(&pool, &f).await?;
    let action = erp_repo::inventory::InventoryReservationAction {
        tenant_id: f.tenant,
        warehouse_id: f.warehouse,
        sku_id: f.sku,
        order_item_id: item,
        qty: 1,
    };
    let repo = InventoryRepo::new(&pool);
    repo.release_reservation(action.clone()).await?;
    let before = facts(&pool, &f).await?;
    for changed in [
        erp_repo::inventory::InventoryReservationAction {
            qty: 2,
            ..action.clone()
        },
        erp_repo::inventory::InventoryReservationAction {
            warehouse_id: f.denied,
            ..action.clone()
        },
    ] {
        let result = repo.release_reservation(changed).await;
        eprintln!(
            "QA inventory consumer terminal replay tenant={} result={result:?}",
            f.tenant
        );
        let error = result
            .expect_err("a different original quantity or warehouse was accepted as a replay");
        assert!(matches!(
            error.downcast_ref::<erp_repo::inventory::InventoryError>(),
            Some(erp_repo::inventory::InventoryError::ReservationMismatch)
        ));
    }
    assert_eq!(facts(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn consumer_shared_single_balance_uses_the_same_precise_reader() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let before = facts(&pool, &f).await?;
    let balance = erp_repo::shared_contract::SharedContractRepo::new(&pool)
        .read_balance(f.tenant, f.warehouse, f.sku)
        .await?
        .ok_or_else(|| anyhow::anyhow!("missing balance"))?;
    assert_eq!(balance.available(), 0);
    assert_eq!(
        serde_json::to_value(balance)?["quantities"]["available"],
        "0.1"
    );
    assert_eq!(facts(&pool, &f).await?, before);
    Ok(())
}

async fn unknown_stock(pool: &PgPool, f: &Fixture, warehouse: Uuid) -> Result<Uuid> {
    let sku = Uuid::new_v4();
    sqlx::query("insert into skus(tenant_id,sku_id,sku_code,product_id) select tenant_id,$2,$2::text,product_id from skus where tenant_id=$1 and sku_id=$3")
        .bind(Uuid::from(f.tenant)).bind(sku).bind(f.sku).execute(pool).await?;
    sqlx::query(
        "insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand) values($1,$2,$3,7)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(warehouse)
    .bind(sku)
    .execute(pool)
    .await?;
    Ok(sku)
}

#[tokio::test]
async fn consumer_single_balance_rejects_unknown_history_without_activation() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let sku = unknown_stock(&pool, &f, f.warehouse).await?;
    let audit = isolated_pool().await?;
    let before = facts(&audit, &f).await?;
    for result in [
        InventoryRepo::new(&pool)
            .get_balance(f.tenant, f.warehouse, sku)
            .await,
        erp_repo::shared_contract::SharedContractRepo::new(&pool)
            .read_balance(f.tenant, f.warehouse, sku)
            .await,
    ] {
        let error = result.expect_err("unknown seven units were exposed as precise");
        assert!(matches!(
            error.downcast_ref::<erp_repo::inventory_precision::InventoryPrecisionError>(),
            Some(erp_repo::inventory_precision::InventoryPrecisionError::ReconciliationRequired(_))
        ));
    }
    assert_eq!(facts(&audit, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn consumer_turnover_rejects_unknown_stock_before_paging_but_excludes_other_warehouses()
-> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    unknown_stock(&pool, &f, f.denied).await?;
    let (status, body) = http(&pool, &f, "/reports/inventory?offset=1000").await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "outside unknown history cannot poison this scope: {body}"
    );
    assert_eq!(body["total"], 1);
    unknown_stock(&pool, &f, f.warehouse).await?;
    let (status, body) = http(&pool, &f, "/reports/inventory?offset=1000").await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "unknown stock must fail before an empty page: {body}"
    );
    assert_eq!(body["code"], "inventory_quantity_reconciliation_required");
    Ok(())
}

#[tokio::test]
async fn consumer_turnover_decimal_warning_and_invalid_fields_preserve_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    for (warning, expected) in [("0.099999", false), ("0.1", true), ("0.100001", true)] {
        let (status, body) = http(
            &pool,
            &f,
            &format!("/reports/inventory?warning_quantity={warning}"),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["items"][0]["warning"], expected, "{body}");
    }
    for query in [
        "warning_quantity=-1",
        "warning_quantity=0.0000001",
        "warning_quantity=no",
        "warning_threshold=-1",
        "warning_quantity=0.1&warning_threshold=1",
        "start_at=2026-10-01T00:00:00Z&end_at=2026-10-01T00:00:00Z",
    ] {
        let (status, body) = http(&pool, &f, &format!("/reports/inventory?{query}")).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
    }
    Ok(())
}

#[tokio::test]
async fn consumer_turnover_requires_both_report_and_inventory_permissions() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let before = facts(&pool, &f).await?;
    for permission in ["reports.read", "inventory.read"] {
        let (status, body) =
            request(&f, "/reports/inventory", vec![permission.into()], None).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{permission}: {body}");
    }
    assert_eq!(facts(&pool, &f).await?, before);
    Ok(())
}

async fn shipment(
    pool: &PgPool,
    f: &Fixture,
    warehouse: Uuid,
    quantity: &str,
    occurred: &str,
) -> Result<()> {
    InventoryService::new(pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: warehouse,
                sku_id: f.sku,
            },
            on_hand: parse_qty(quantity)?,
            reserved: parse_qty("0")?,
            frozen: parse_qty("0")?,
            in_transit: parse_qty("0")?,
            reason_type: "SALES_SHIP".into(),
            ref_type: Some("ORDER_ITEM".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: chrono::DateTime::parse_from_rfc3339(occurred)?.with_timezone(&Utc),
        })
        .await?;
    Ok(())
}

#[tokio::test]
async fn consumer_turnover_counts_posted_shipments_not_order_demand_or_other_warehouses()
-> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shop = Uuid::new_v4();
    let order = Uuid::new_v4();
    sqlx::query("insert into shops(tenant_id,shop_id,platform,platform_shop_id) values($1,$2,'taobao',$2::text)")
        .bind(Uuid::from(f.tenant)).bind(shop).execute(&pool).await?;
    sqlx::query("insert into orders(tenant_id,order_month,order_id,platform,shop_id,platform_order_id,status,order_created_at) values($1,'2026-10-01',$2,'taobao',$3,$2::text,'CANCELLED','2026-10-02T12:00:00Z')")
        .bind(Uuid::from(f.tenant)).bind(order).bind(shop).execute(&pool).await?;
    sqlx::query("insert into order_items(tenant_id,order_month,order_id,line_id,sku_id,qty) values($1,'2026-10-01',$2,'QA',$3,99)")
        .bind(Uuid::from(f.tenant)).bind(order).bind(f.sku.to_string()).execute(&pool).await?;
    shipment(&pool, &f, f.warehouse, "-0.05", "2026-10-02T12:00:00Z").await?;
    shipment(&pool, &f, f.denied, "-0.09", "2026-10-02T12:00:00Z").await?;
    let (status, body) = http(
        &pool,
        &f,
        "/reports/inventory?start_at=2026-10-01T00:00:00Z&end_at=2026-10-03T00:00:00Z",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["schema"], 2);
    assert_eq!(body["total"], 1);
    let row = &body["items"][0];
    assert_eq!(
        row["sold_qty"], 0,
        "fractional posted shipment cannot become a full unit"
    );
    assert_eq!(row["sold_quantity"], "0.05", "{body}");
    assert_eq!(row["quantities"]["on_hand"], "1.85");
    assert_eq!(row["quantities"]["available"], "0.05");
    assert!(
        (row["turnover_days"]
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("missing cover"))?
            - 74.0)
            .abs()
            < 1e-9
    );
    assert_eq!(body["sales_basis"], "gross_posted_sales_ship");
    assert_eq!(body["turnover_basis"], "closing_stock_cover");
    let (status, body) = http(
        &pool,
        &f,
        "/reports/inventory?start_at=2026-10-01T00:00:00Z&end_at=2026-10-02T00:00:00Z",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["items"][0]["sold_quantity"], "0");
    assert_eq!(body["items"][0]["turnover_days"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn consumer_turnover_unclassified_shared_effect_is_unknown_not_zero_sales() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let source = Uuid::new_v4();
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
            },
            on_hand: parse_qty("-0.05")?,
            reserved: parse_qty("0")?,
            frozen: parse_qty("0")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_SHARED_UNCLASSIFIED".into(),
            ref_type: Some("shared_stock_ledger".into()),
            ref_id: Some(source.to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros) values($1,$2,$3,$2,$4,'','GOOD','ON_HAND',-50000)")
        .bind(Uuid::from(f.tenant)).bind(source).bind(f.warehouse).bind(f.sku).execute(&pool).await?;
    let (status, body) = http(&pool, &f, "/reports/inventory").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let row = &body["items"][0];
    assert_eq!(row["quantities"]["available"], "0.05");
    assert_eq!(row["sales_state"], "source_classification_required");
    assert_eq!(row["sold_qty"], Value::Null);
    assert_eq!(row["sold_quantity"], Value::Null);
    assert_eq!(row["turnover_days"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn consumer_legacy_adjustment_http_returns_exact_committed_quantities() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    sqlx::query("insert into feature_flags(tenant_id,scope,key,enabled) values($1,'TENANT','INVENTORY_WRITE',true)")
        .bind(Uuid::from(f.tenant)).execute(&pool).await?;
    let (status, body) = request(
        &f,
        &format!("/v1/tenants/{}/inventory/adjustments", f.tenant),
        vec!["inventory.write".into()],
        Some(json!({"warehouse_id":f.warehouse,"sku_id":f.sku,
            "quantity_delta":1,"reason":"QA_CONSUMER_API_WRITE","reference_id":Uuid::new_v4()})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["record"]["available"], 1, "{body}");
    assert_eq!(body["record"]["quantities"]["on_hand"], "2.9");
    assert_eq!(body["record"]["quantities"]["available"], "1.1");
    assert_eq!(body["record"]["quantity_state"], "precise");
    let audit = isolated_pool().await?;
    let qty:i64=sqlx::query_scalar("select on_hand_micros from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).fetch_one(&audit).await?;
    assert_eq!(qty, 2_900_000);
    Ok(())
}

#[tokio::test]
async fn consumer_legacy_whole_writer_returns_its_exact_transaction_balance() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let balance = InventoryService::new(&pool)
        .apply(InventoryDelta {
            tenant_id: f.tenant,
            warehouse_id: f.warehouse,
            sku_id: f.sku,
            delta_on_hand: 1,
            delta_reserved: 0,
            delta_frozen: 0,
            delta_in_transit: 0,
            reason_type: "QA_CONSUMER_LEGACY_WRITE".into(),
            ref_type: Some("QA_CONSUMER_LEGACY_WRITE".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    let body = serde_json::to_value(&balance)?;
    eprintln!(
        "QA inventory consumer writer tenant={} available={} body={body}",
        f.tenant,
        balance.available()
    );
    assert_eq!(
        balance.available(),
        1,
        "exact 1.1 projects to one whole unit"
    );
    assert_eq!(body["quantities"]["on_hand"], "2.9");
    assert_eq!(body["quantities"]["available"], "1.1");
    let audit = isolated_pool().await?;
    let qty:i64=sqlx::query_scalar("select on_hand_micros from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).fetch_one(&audit).await?;
    assert_eq!(qty, 2_900_000, "old integer one still means one full unit");
    Ok(())
}
