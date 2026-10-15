//! Existing real OMS routes must cite the original physical source and commit whole orders.
use super::inventory_legacy_reservation_atomicity::fixture as stock_fixture;
use super::inventory_location_commands::Fixture;
use super::{Result, isolated_pool};
use axum::{
    Router,
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
use tokio::{sync::Barrier, task::JoinSet};
use tower::ServiceExt;
use uuid::Uuid;

pub(super) struct Fulfillment {
    pub(super) f: Fixture,
    pub(super) order: Uuid,
    pub(super) root: Uuid,
    pub(super) shop: Uuid,
    pub(super) session_id: Uuid,
    pub(super) app: Router,
    pub(super) token: String,
}

pub(super) async fn fixture(pool: &PgPool) -> Result<Fulfillment> {
    let (f, root) = stock_fixture(pool).await?;
    let order = Uuid::new_v4();
    let shop = Uuid::new_v4();
    sqlx::query("update tenants set name='QA-ERP-FULFILLMENT-SOURCES' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    sqlx::query("update warehouses set is_default=true,updated_at=now() where tenant_id=$1 and warehouse_id=$2")
        .bind(f.tenant.0).bind(f.warehouse).execute(pool).await?;
    sqlx::query("update skus set cost_cent=2500 where tenant_id=$1 and sku_id=$2")
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(pool)
        .await?;
    sqlx::query("insert into shops(tenant_id,shop_id,platform,platform_shop_id,shop_name) values($1,$2,'TAOBAO',$3,'Real OMS QA')")
        .bind(f.tenant.0).bind(shop).bind(shop.to_string()).execute(pool).await?;
    for key in ["INVENTORY_WRITE", "SHIPPING_WRITE"] {
        sqlx::query(
            "insert into feature_flags(tenant_id,scope,key,enabled) values($1,'GLOBAL',$2,true)",
        )
        .bind(f.tenant.0)
        .bind(key)
        .execute(pool)
        .await?;
    }
    let role_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'qa-oms-ship')")
        .bind(role_id)
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(f.tenant.0)
        .bind(f.actor)
        .bind(role_id)
        .execute(pool)
        .await?;
    for code in [
        "OMS.ORDER.WRITE",
        "OMS.SHIPPING.WRITE",
        "WMS.INVENTORY.WRITE",
    ] {
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, $3)",
        )
        .bind(f.tenant.0)
        .bind(role_id)
        .bind(code)
        .execute(pool)
        .await?;
    }
    sqlx::query("update tenants set credentials_epoch = 1 where tenant_id = $1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    sqlx::query(
        "insert into auth_sessions(session_id, tenant_id, user_id, status, client_kind, expires_at, credentials_epoch) values ($1, $2, $3, 'ACTIVE', 'WEB', now() + interval '3600 seconds', 1)",
    )
    .bind(session_id)
    .bind(f.tenant.0)
    .bind(f.actor)
    .execute(pool)
    .await?;
    let jwt = Arc::new(JwtService::new("synthetic-fulfillment-source-key".into()));
    let now = Utc::now().timestamp() as usize;
    let mut claims = Claims::bearer(
        f.actor.to_string(),
        f.tenant.to_string(),
        vec![
            "inventory.read".into(),
            "inventory.write".into(),
            "orders.read".into(),
            "orders.write".into(),
            "shipping.write".into(),
            "users.write".into(),
        ],
        false,
        now,
        now + 300,
    );
    claims.sid = Some(session_id.to_string());
    claims.cep = Some(1);
    let token = jwt.encode(&claims)?;
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let result = Fulfillment {
        f,
        order,
        root,
        shop,
        session_id,
        app,
        token,
    };
    add_order(pool, &result, order, root, result.f.sku).await?;
    eprintln!(
        "QA fulfillment fixture tenant={} actor={} order={} root={} shop={} warehouse={} sku={}",
        result.f.tenant, result.f.actor, order, root, shop, result.f.warehouse, result.f.sku
    );
    Ok(result)
}

pub(super) async fn add_order(
    pool: &PgPool,
    f: &Fulfillment,
    order: Uuid,
    root: Uuid,
    sku: Uuid,
) -> Result<()> {
    sqlx::query("insert into orders(tenant_id,order_month,order_id,platform,shop_id,platform_order_id,status,
        order_created_at,order_status,fulfillment_status,exception_status,exception_flags,inventory_shortage)
        values($1,'2026-10-01',$2,'TAOBAO',$3,$4,'PAID',now(),'PENDING_ALLOCATE','NOT_ALLOCATED','NONE','[]',false)")
        .bind(f.f.tenant.0).bind(order).bind(f.shop).bind(order.to_string()).execute(pool).await?;
    sqlx::query("insert into order_index(tenant_id,order_id,order_month,platform,platform_order_id,shop_id,order_created_at)
        values($1,$2,'2026-10-01','TAOBAO',$3,$4,now())")
        .bind(f.f.tenant.0).bind(order).bind(order.to_string()).bind(f.shop).execute(pool).await?;
    sqlx::query(
        "insert into order_items(tenant_id,order_month,order_id,line_id,sku_id,qty)
        values($1,'2026-10-01',$2,$3,$4,1)",
    )
    .bind(f.f.tenant.0)
    .bind(order)
    .bind(root.to_string())
    .bind(sku.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn request_as(
    app: &Router,
    tenant: Uuid,
    token: &str,
    suffix: String,
    payload: Value,
) -> Result<(StatusCode, Value)> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/tenants/{tenant}/oms/{suffix}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body =
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes)));
    eprintln!(
        "QA fulfillment_http tenant={tenant} suffix={suffix} status={} body={body}",
        status.as_u16()
    );
    Ok((status, body))
}

pub(super) async fn request(
    f: &Fulfillment,
    suffix: String,
    payload: Value,
) -> Result<(StatusCode, Value)> {
    request_as(&f.app, f.f.tenant.0, &f.token, suffix, payload).await
}

pub(super) async fn allocated(f: &Fulfillment) -> Result<Uuid> {
    let (status, result) = request(f, format!("orders/{}/allocate", f.order), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "real allocation: {result}");
    Ok(Uuid::parse_str(
        result["shipments"][0]["shipment_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("actual shipment identity: {result}"))?,
    )?)
}

pub(super) async fn facts(pool: &PgPool, f: &Fulfillment) -> Result<Value> {
    let mut values = serde_json::Map::new();
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "inventory_reservations",
        "inventory_order_reservation_sources",
        "inventory_order_reservation_components",
        "inventory_quantity_activations",
        "inventory_quantity_source_maps",
        "shared_stock_ledgers",
        "inventory_batch_balances",
        "shipments",
        "shipment_items",
        "order_cogs",
        "outbox_events",
        "operation_logs",
        "audit_logs",
    ] {
        let rows:Value=sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from {table} x where tenant_id=$1"))
            .bind(f.f.tenant.0).fetch_one(pool).await?;
        values.insert(table.into(), rows);
    }
    Ok(Value::Object(values))
}

async fn kit_order(pool: &PgPool, f: &Fulfillment) -> Result<(Uuid, Uuid)> {
    let parent = Uuid::new_v4();
    let second = Uuid::new_v4();
    for (sku, code, kind) in [
        (parent, "QA-OMS-KIT", "KIT"),
        (second, "QA-OMS-SECOND", "NORMAL"),
    ] {
        sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,sku_kind,cost_cent,product_id) values($1,$2,$3,$4,2500,(select product_id from qa_parent))")
            .bind(f.f.tenant.0).bind(sku).bind(code).bind(kind).execute(pool).await?;
    }
    erp_repo::inventory::InventoryService::new(pool)
        .apply_precise(erp_repo::inventory_precision::InventoryPreciseDelta {
            key: erp_repo::inventory_precision::InventoryQuantityKey {
                tenant_id: f.f.tenant,
                warehouse_id: f.f.warehouse,
                sku_id: second,
            },
            on_hand: erp_domain::quantity::parse_qty("2.9")?,
            reserved: erp_domain::quantity::parse_qty("0.9")?,
            frozen: erp_domain::quantity::parse_qty("0.9")?,
            in_transit: erp_domain::quantity::parse_qty("2.3")?,
            reason_type: "QA_OMS_SECOND_OPENING".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;
    for sku in [f.f.sku, second] {
        sqlx::query(
            "insert into sku_boms(tenant_id,bundle_sku_id,component_sku_id,qty) values($1,$2,$3,1)",
        )
        .bind(f.f.tenant.0)
        .bind(parent)
        .bind(sku)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        "update order_items set sku_id=$3 where tenant_id=$1 and order_id=$2 and line_id=$4",
    )
    .bind(f.f.tenant.0)
    .bind(f.order)
    .bind(parent.to_string())
    .bind(f.root.to_string())
    .execute(pool)
    .await?;
    Ok((parent, second))
}

#[tokio::test]
async fn fulfillment_snapshot_cached_allocation_rejects_changed_parent_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    allocated(&f).await?;
    sqlx::query("update order_items set qty=2 where tenant_id=$1 and order_id=$2 and line_id=$3")
        .bind(f.f.tenant.0)
        .bind(f.order)
        .bind(f.root.to_string())
        .execute(&pool)
        .await?;
    let before = facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("orders/{}/allocate", f.order), json!({})).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "changed real parent quantity: {body}"
    );
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_shipping_rejects_changed_shipment_item_order() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = allocated(&f).await?;
    sqlx::query("update shipment_items set order_id=$3 where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0)
        .bind(shipment)
        .bind(Uuid::new_v4())
        .execute(&pool)
        .await?;
    let before = facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "item order must agree with its header: {body}"
    );
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_deleted_bom_shipping_and_replay_use_original_components() -> Result<()>
{
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let (parent, second) = kit_order(&pool, &f).await?;
    let shipment = allocated(&f).await?;
    sqlx::query("delete from sku_boms where tenant_id=$1 and bundle_sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(parent)
        .execute(&pool)
        .await?;
    sqlx::query("update skus set sku_kind='NORMAL' where tenant_id=$1 and sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(parent)
        .execute(&pool)
        .await?;
    assert_eq!(allocated(&f).await?, shipment);
    let (status, body) = request(
        &f,
        format!("shipments/{shipment}/ship"),
        json!({"tracking_no":"QA-ORIGINAL"}),
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "deleted catalog still consumes the applied sources: {body}"
    );
    let after = facts(&independent, &f).await?;
    assert_eq!(after["order_cogs"].as_array().map(Vec::len), Some(2));
    for sku in [f.f.sku, second] {
        let balance = after["inventory_balances"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["sku_id"] == json!(sku)))
            .ok_or_else(|| anyhow::anyhow!("missing actual component balance"))?;
        assert_eq!(balance["on_hand_micros"], 1_900_000);
        assert_eq!(balance["reserved_micros"], 900_000);
        assert_eq!(balance["frozen_micros"], 900_000);
        assert_eq!(balance["in_transit_micros"], 2_300_000);
    }
    let (status, replay) = request(
        &f,
        format!("shipments/{shipment}/ship"),
        json!({"tracking_no":"QA-LATER"}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "original shipment replay: {replay}");
    assert_eq!(replay["tracking_no"], "QA-ORIGINAL");
    assert_eq!(facts(&independent, &f).await?, after);
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_deleted_bom_release_keeps_original_components() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let (parent, _) = kit_order(&pool, &f).await?;
    allocated(&f).await?;
    sqlx::query("delete from sku_boms where tenant_id=$1 and bundle_sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(parent)
        .execute(&pool)
        .await?;
    let (status, body) = request(&f, format!("orders/{}/release", f.order), json!({})).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "release uses original physical rows: {body}"
    );
    assert_eq!(body["released"], 2);
    let after = facts(&independent, &f).await?;
    assert!(
        after["inventory_reservations"].as_array().is_some_and(
            |rows| rows.len() == 2 && rows.iter().all(|row| row["status"] == "RELEASED")
        )
    );
    assert!(after["inventory_balances"].as_array().is_some_and(|rows| {
        rows.len() == 2
            && rows
                .iter()
                .all(|row| row["reserved_micros"] == 900_000 && row["on_hand_micros"] == 2_900_000)
    }));
    let (status, retry) = request(&f, format!("orders/{}/release", f.order), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "terminal release replay: {retry}");
    assert_eq!(retry["released"], 0);
    let replay = facts(&independent, &f).await?;
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "inventory_reservations",
        "inventory_order_reservation_sources",
        "inventory_order_reservation_components",
        "outbox_events",
        "order_cogs",
    ] {
        assert_eq!(
            replay[table], after[table],
            "release replay changed {table}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_ship_and_release_serialize_without_double_effects() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(fixture(&pool).await?);
    let shipment = allocated(&f).await?;
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for index in 0..20 {
        let f = f.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            let suffix = if index % 2 == 0 {
                format!("shipments/{shipment}/ship")
            } else {
                format!("orders/{}/release", f.order)
            };
            request(&f, suffix, json!({})).await
        });
    }
    let mut responses = Vec::new();
    while let Some(result) = tasks.join_next().await {
        responses.push(result??);
    }
    assert_eq!(responses.len(), 20);
    for (status, body) in responses {
        assert!(
            matches!(status, StatusCode::OK | StatusCode::CONFLICT),
            "terminal race: {status} {body}"
        );
    }
    let after = facts(&independent, &f).await?;
    let source = &after["inventory_reservations"][0];
    let consumed = source["status"] == "CONSUMED";
    assert!(consumed || source["status"] == "RELEASED");
    assert_eq!(after["inventory_balances"][0]["reserved_micros"], 900_000);
    assert_eq!(
        after["inventory_balances"][0]["on_hand_micros"],
        if consumed { 1_900_000 } else { 2_900_000 }
    );
    assert_eq!(
        after["order_cogs"].as_array().map(Vec::len),
        Some(usize::from(consumed))
    );
    let terminals = after["inventory_ledgers"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("terminal ledgers"))?
        .iter()
        .filter(|row| {
            row["reason_type"] == "SALES_SHIP" || row["reason_type"] == "RESERVATION_RELEASE"
        })
        .count();
    assert_eq!(terminals, 1);
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_ship_rejects_another_reservation_warehouse() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = allocated(&f).await?;
    let other = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Wrong source WH',$3)")
        .bind(f.f.tenant.0).bind(other).bind(other.to_string()).execute(&pool).await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.f.tenant.0)
    .bind(f.f.actor)
    .bind(other)
    .execute(&pool)
    .await?;
    erp_repo::inventory::InventoryService::new(&pool)
        .apply_precise(erp_repo::inventory_precision::InventoryPreciseDelta {
            key: erp_repo::inventory_precision::InventoryQuantityKey {
                tenant_id: f.f.tenant,
                warehouse_id: other,
                sku_id: f.f.sku,
            },
            on_hand: erp_domain::quantity::parse_qty("2.9")?,
            reserved: erp_domain::quantity::parse_qty("1.9")?,
            frozen: erp_domain::quantity::parse_qty("0.9")?,
            in_transit: erp_domain::quantity::ScaledQty::ZERO,
            reason_type: "QA_FULFILLMENT_OTHER_WH".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;
    sqlx::query("update shipments set warehouse_id=$3 where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0)
        .bind(shipment)
        .bind(other)
        .execute(&pool)
        .await?;
    let before = facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a different WH must not borrow the source: {body}"
    );
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_ship_rejects_another_order_source() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = allocated(&f).await?;
    let other = Uuid::new_v4();
    add_order(&pool, &f, other, Uuid::new_v4(), f.f.sku).await?;
    sqlx::query("update shipments set order_id=$3 where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0)
        .bind(shipment)
        .bind(other)
        .execute(&pool)
        .await?;
    sqlx::query("update shipment_items set order_id=$3 where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0)
        .bind(shipment)
        .bind(other)
        .execute(&pool)
        .await?;
    let before = facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a different order must not borrow the source: {body}"
    );
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_second_shipment_cannot_claim_consumed_source() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = allocated(&f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let duplicate = Uuid::new_v4();
    sqlx::query("insert into shipments(shipment_id,tenant_id,shop_id,order_id,platform,platform_order_id,warehouse_id,
        status,shipment_type,status_version,shipment_status) select $3,tenant_id,shop_id,order_id,platform,
        platform_order_id,warehouse_id,'CREATED',shipment_type,10,'CREATED' from shipments where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0).bind(shipment).bind(duplicate).execute(&pool).await?;
    sqlx::query("insert into shipment_items(tenant_id,shipment_id,order_id,order_item_id,sku_id,qty)
        select tenant_id,$3,order_id,order_item_id,sku_id,qty from shipment_items where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0).bind(shipment).bind(duplicate).execute(&pool).await?;
    let before = facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{duplicate}/ship"), json!({})).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "consumed does not prove this new shipment succeeded: {body}"
    );
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_later_shortage_leaves_no_partial_order_reserve() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let missing = Uuid::new_v4();
    sqlx::query(
        "with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-FULFILLMENT-SHORTAGE',(select product_id from qa_parent))",
    )
    .bind(f.f.tenant.0)
    .bind(missing)
    .execute(&pool)
    .await?;
    sqlx::query("insert into order_items(tenant_id,order_month,order_id,line_id,sku_id,qty) values($1,'2026-10-01',$2,'ZZ-SHORTAGE',$3,1)")
        .bind(f.f.tenant.0).bind(f.order).bind(missing.to_string()).execute(&pool).await?;
    let before = facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("orders/{}/allocate", f.order), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "shortage result: {body}");
    assert_eq!(body["shipments"], json!([]));
    let after = facts(&independent, &f).await?;
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "inventory_reservations",
        "inventory_order_reservation_sources",
        "inventory_order_reservation_components",
        "inventory_quantity_activations",
        "inventory_quantity_source_maps",
        "shared_stock_ledgers",
        "outbox_events",
        "shipments",
        "shipment_items",
        "order_cogs",
    ] {
        assert_eq!(
            after[table], before[table],
            "later shortage changed {table}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn fulfillment_snapshot_twenty_allocations_return_one_original_shipment() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(fixture(&pool).await?);
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let f = f.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            request(&f, format!("orders/{}/allocate", f.order), json!({})).await
        });
    }
    // Drain every submitted request before evaluating any failure. The
    // evidence must include all 20 responses even when one request fails.
    let mut responses = Vec::new();
    while let Some(result) = tasks.join_next().await {
        responses.push(result??);
    }
    assert_eq!(responses.len(), 20);
    let mut first = None;
    for (status, body) in responses {
        assert_eq!(
            status,
            StatusCode::OK,
            "same natural allocation must be recoverable: {body}"
        );
        let id = body["shipments"][0]["shipment_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("shipment response: {body}"))?
            .to_owned();
        if let Some(first) = &first {
            assert_eq!(&id, first);
        } else {
            first = Some(id);
        }
    }
    let after = facts(&independent, &f).await?;
    assert_eq!(after["shipments"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        after["inventory_reservations"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        after["inventory_ledgers"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("ledger facts"))?
            .iter()
            .filter(|row| row["reason_type"] == "RESERVATION_CREATE")
            .count(),
        1
    );
    Ok(())
}
