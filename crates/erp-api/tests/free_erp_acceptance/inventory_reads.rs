//! Public inventory reads use canonical stock, and never activate unknown stock.
use super::{Result, isolated_pool};
use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_domain::quantity::parse_qty;
use erp_repo::{
    db::Db,
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
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
}

async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let f = Fixture {
        tenant: TenantId(Uuid::new_v4()),
        warehouse: Uuid::new_v4(),
        sku: Uuid::new_v4(),
    };
    let product = Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-CANONICAL-READ')")
        .bind(Uuid::from(f.tenant))
        .execute(pool)
        .await?;
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Inventory read QA',$3)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.warehouse.to_string()).execute(pool).await?;
    sqlx::query("insert into products(tenant_id,product_id,name,category) values($1,$2,'Precision read','Read category')")
        .bind(Uuid::from(f.tenant)).bind(product).execute(pool).await?;
    sqlx::query("insert into skus(tenant_id,sku_id,sku_code,product_id,cost_cent,barcode,tags) values($1,$2,'QA-READ',$3,7,'QA-READ-BARCODE',array['QA-READ-TAG'])")
        .bind(Uuid::from(f.tenant)).bind(f.sku).bind(product).execute(pool).await?;
    InventoryService::new(pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
            },
            on_hand: parse_qty("1.9")?,
            reserved: parse_qty("0.9")?,
            frozen: parse_qty("0.9")?,
            in_transit: parse_qty("2.3")?,
            reason_type: "QA_READ_OPENING".into(),
            ref_type: Some("QA_READ_OPENING".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    eprintln!(
        "QA inventory read tenant={} warehouse={} sku={}",
        Uuid::from(f.tenant),
        f.warehouse,
        f.sku
    );
    Ok(f)
}

async fn http(f: &Fixture, path: &str, body: Option<Value>) -> Result<(StatusCode, Value)> {
    http_as(f, path, body, Uuid::new_v4()).await
}

async fn read_facts(pool: &PgPool, f: &Fixture) -> Result<String> {
    Ok(sqlx::query_scalar("select jsonb_build_object('balances',coalesce((select jsonb_agg(to_jsonb(b) order by warehouse_id,sku_id) from inventory_balances b where tenant_id=$1),'[]'::jsonb),'ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from inventory_ledgers l where tenant_id=$1),'[]'::jsonb),'outbox',coalesce((select jsonb_agg(to_jsonb(o) order by outbox_id) from outbox_events o where tenant_id=$1),'[]'::jsonb))::text")
        .bind(Uuid::from(f.tenant)).fetch_one(pool).await?)
}

async fn http_as(
    f: &Fixture,
    path: &str,
    body: Option<Value>,
    user: Uuid,
) -> Result<(StatusCode, Value)> {
    let audit_pool = isolated_pool().await?;
    let before = read_facts(&audit_pool, f).await?;
    // A registered real HTTP route with synthetic JWT: not real session acceptance.
    let jwt = JwtService::new("synthetic-inventory-read-key".into());
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        user.to_string(),
        Uuid::from(f.tenant).to_string(),
        vec!["inventory.read".into(), "OPS.REPORT.READ".into()],
        false,
        now,
        now + 300,
    ))?;
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        Arc::new(jwt),
    ));
    let request = Request::builder()
        .uri(format!("/v1/tenants/{}{path}", Uuid::from(f.tenant)))
        .method(if body.is_some() { "POST" } else { "GET" })
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))?;
    let response = app.oneshot(request).await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text": String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA inventory HTTP tenant={} path={path} status={status} body={body}",
        Uuid::from(f.tenant)
    );
    assert_eq!(
        read_facts(&audit_pool, f).await?,
        before,
        "inventory read must leave all balances, original ledgers and outbox unchanged"
    );
    Ok((status, body))
}

#[tokio::test]
async fn precision_read_public_list_does_not_overpromise_available_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let (status, body) = http(&f, "/inventory?limit=1", None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["data"][0]["available"], 0,
        "legacy callers cannot promise one whole unit: {body}"
    );
    assert_eq!(body["data"][0]["quantities"]["available"], "0.1", "{body}");
    Ok(())
}

#[tokio::test]
async fn precision_read_public_summary_subtracts_exact_buckets_before_projection() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let (status, body) = http(&f, "/inventory/summary?limit=1", None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["items"][0]["available"], 0, "{body}");
    assert_eq!(body["items"][0]["quantities"]["available"], "0.1", "{body}");
    Ok(())
}

async fn warehouse(pool: &PgPool, f: &Fixture) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Inventory read QA',$3)")
        .bind(Uuid::from(f.tenant)).bind(id).bind(id.to_string()).execute(pool).await?;
    Ok(id)
}

async fn sku(pool: &PgPool, f: &Fixture, cost: Option<i64>, kind: &str) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("insert into skus(tenant_id,sku_id,sku_code,product_id,cost_cent,sku_kind) select tenant_id,$2,$3,product_id,$4,$5 from skus where tenant_id=$1 and sku_id=$6")
        .bind(Uuid::from(f.tenant)).bind(id).bind(id.to_string()).bind(cost).bind(kind).bind(f.sku).execute(pool).await?;
    Ok(id)
}

async fn opening(pool: &PgPool, f: &Fixture, wh: Uuid, sku: Uuid, hand: &str) -> Result<()> {
    InventoryService::new(pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: wh,
                sku_id: sku,
            },
            on_hand: parse_qty(hand)?,
            reserved: parse_qty("0")?,
            frozen: parse_qty("0")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_READ_OPENING".into(),
            ref_type: Some("QA_READ_OPENING".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    Ok(())
}

fn aggregate_query(schema: u32, measure: &str) -> Value {
    json!({"schema":schema,"by_category":true,"measure":measure})
}

#[tokio::test]
async fn precision_read_cross_warehouse_summary_and_cost_preserve_fraction() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let second = warehouse(&pool, &f).await?;
    opening(&pool, &f, second, f.sku, "0.2").await?;
    let (status, summary) = http(&f, "/inventory/summary?limit=1", None).await?;
    assert_eq!(status, StatusCode::OK, "{summary}");
    assert_eq!(summary["total"], 1);
    assert_eq!(summary["items"][0]["on_hand"], 2);
    assert_eq!(summary["items"][0]["available"], 0);
    assert_eq!(summary["items"][0]["quantities"]["on_hand"], "2.1");
    assert_eq!(summary["items"][0]["quantities"]["available"], "0.3");
    let (status, cost) = http(&f, "/inventory/costs?limit=1", None).await?;
    assert_eq!(status, StatusCode::OK, "{cost}");
    assert_eq!(cost["items"][0]["estimated_value"], "0.147");
    assert_eq!(cost["items"][0]["valuation_source"], "sku_master_estimate");
    assert_eq!(cost["items"][0]["total_cost_cent"], 14);
    Ok(())
}

#[tokio::test]
async fn precision_read_decimal_range_and_legacy_safety_apply_before_page() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let second = sku(&pool, &f, Some(3), "NORMAL").await?;
    opening(&pool, &f, f.warehouse, second, "2").await?;
    sqlx::query("insert into settings(tenant_id,scope_type,key,value_json) values($1,'TENANT','inventory.safety_stock','{\"value\":1}'::jsonb)")
        .bind(Uuid::from(f.tenant)).execute(&pool).await?;
    for suffix in [
        "available_min=0.1&available_max=0.1",
        "available_below_safety=true",
        "barcode=QA-READ-BARCODE&tag=QA-READ-TAG",
    ] {
        for (path, field) in [
            ("/inventory", "data"),
            ("/inventory/summary", "items"),
            ("/inventory/costs", "items"),
        ] {
            let (status, body) = http(&f, &format!("{path}?limit=1&{suffix}"), None).await?;
            assert_eq!(status, StatusCode::OK, "{body}");
            assert_eq!(body[field].as_array().expect("rows").len(), 1, "{body}");
            assert_eq!(body[field][0]["sku_id"], f.sku.to_string(), "{body}");
            assert_eq!(body[field][0]["quantities"]["available"], "0.1");
            if field == "items" {
                assert_eq!(body["total"], 1);
            }
        }
    }
    let (status, first) = http(&f, "/inventory?limit=1", None).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let cursor = first["next_cursor"]
        .as_str()
        .expect("another page")
        .replace('+', "%2B")
        .replace('|', "%7C");
    let (status, last) = http(&f, &format!("/inventory?limit=1&cursor={cursor}"), None).await?;
    assert_eq!(status, StatusCode::OK, "{last}");
    assert_ne!(first["data"][0]["sku_id"], last["data"][0]["sku_id"]);
    assert!(
        last["next_cursor"].is_null(),
        "no phantom empty page: {last}"
    );
    Ok(())
}

#[tokio::test]
async fn precision_read_decimal_safety_and_invalid_policy_are_explicit() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    sqlx::query("insert into settings(tenant_id,scope_type,key,value_json) values($1,'TENANT','inventory.safety_stock','{\"value\":\"0.100001\"}'::jsonb)")
        .bind(Uuid::from(f.tenant)).execute(&pool).await?;
    for path in ["/inventory", "/inventory/summary", "/inventory/costs"] {
        let (status, body) = http(&f, &format!("{path}?available_below_safety=true"), None).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        let field = if path == "/inventory" {
            "data"
        } else {
            "items"
        };
        assert_eq!(body[field].as_array().expect("rows").len(), 1, "{body}");
        assert_eq!(body[field][0]["quantities"]["available"], "0.1");
    }
    for invalid in [
        json!({"value":"0.0000001"}),
        json!({"value":"-1"}),
        json!({"value":0.1}),
        json!({"value":"bad"}),
    ] {
        sqlx::query(
            "update settings set value_json=$2 where tenant_id=$1 and key='inventory.safety_stock'",
        )
        .bind(Uuid::from(f.tenant))
        .bind(invalid)
        .execute(&pool)
        .await?;
        for path in ["/inventory", "/inventory/summary", "/inventory/costs"] {
            let (status, body) =
                http(&f, &format!("{path}?available_below_safety=true"), None).await?;
            assert_eq!(
                status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid policy must not silently become zero: {body}"
            );
            assert_eq!(body["code"], "inventory_safety_stock_invalid");
        }
    }
    Ok(())
}

#[tokio::test]
async fn precision_read_invalid_ranges_fail_without_stock_effects() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    for suffix in [
        "available_min=2&available_max=1",
        "available_min=0.0000001",
        "available_max=1e2",
        "available_min=NaN",
        "available_max=170141183460469231731687303715884105728",
    ] {
        for path in ["/inventory", "/inventory/summary", "/inventory/costs"] {
            let (status, body) = http(&f, &format!("{path}?{suffix}"), None).await?;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
        }
    }
    let (status, body) = http(&f, "/inventory?available_min=&available_max=", None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["data"][0]["quantities"]["available"], "0.1");
    Ok(())
}

#[tokio::test]
async fn precision_read_unknown_history_cannot_hide_behind_filter_or_page() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let unknown = sku(&pool, &f, Some(1), "NORMAL").await?;
    sqlx::query(
        "insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand) values($1,$2,$3,2)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.warehouse)
    .bind(unknown)
    .execute(&pool)
    .await?;
    for path in [
        "/inventory?limit=1&available_max=0",
        "/inventory/summary?limit=1",
        "/inventory/costs?limit=1",
    ] {
        let (status, body) = http(&f, path, None).await?;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["code"], "inventory_quantity_reconciliation_required");
        assert_eq!(body["unreconciled_count"], 1);
    }
    let (status, body) = http(&f, &format!("/inventory?sku_id={}", f.sku), None).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "an unknown key outside the query cannot invalidate the known SKU: {body}"
    );
    let (status, body) = http(
        &f,
        "/reports/inventory/aggregate",
        Some(aggregate_query(2, "quantity")),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let mut query = aggregate_query(2, "quantity");
    query["search"] = json!("QA-READ");
    let (status, body) = http(&f, "/reports/inventory/aggregate", Some(query)).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "unknown SKU outside literal search must stay outside: {body}"
    );
    assert_eq!(body["total"]["sum"], "1900000");
    let orphan = sku(&pool, &f, Some(1), "NORMAL").await?;
    sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,reason_type) values($1,$2,$3,1,0,0,0,'QA_READ_ORPHAN')")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(orphan).execute(&pool).await?;
    let (status, body) = http(&f, &format!("/inventory?sku_id={orphan}"), None).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "history without a balance is not empty stock: {body}"
    );
    assert_eq!(body["unreconciled_count"], 1);
    Ok(())
}

#[tokio::test]
async fn precision_read_wide_aggregate_exceeds_per_key_i64_without_narrowing() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let large = sku(&pool, &f, Some(7), "NORMAL").await?;
    let second = warehouse(&pool, &f).await?;
    for wh in [f.warehouse, second] {
        opening(&pool, &f, wh, large, "9223372036854.775807").await?;
    }
    for path in ["/inventory/summary", "/inventory/costs"] {
        let (status, body) = http(&f, &format!("{path}?sku_id={large}&available_min=18446744073709.551614&available_max=18446744073709.551614"), None).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["total"], 1);
        assert_eq!(
            body["items"][0]["quantities"]["on_hand"],
            "18446744073709.551614"
        );
        assert_eq!(body["items"][0]["on_hand"], 18_446_744_073_709i64);
        if path.ends_with("costs") {
            assert_eq!(
                body["items"][0]["estimated_value"],
                "1291272085159.66861298"
            );
        }
    }
    let mut query = aggregate_query(2, "quantity");
    query["search"] = json!(large.to_string());
    let (status, body) = http(&f, "/reports/inventory/aggregate", Some(query)).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"]["sum"], "18446744073709551614");
    assert_eq!(body["total"]["count"], 2);
    Ok(())
}

#[tokio::test]
async fn precision_read_cost_overflow_is_explicit_and_does_not_round_or_wrap() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let expensive = sku(&pool, &f, Some(i64::MAX), "NORMAL").await?;
    let second = warehouse(&pool, &f).await?;
    let third = warehouse(&pool, &f).await?;
    for wh in [f.warehouse, second, third] {
        opening(&pool, &f, wh, expensive, "9223372036854.775807").await?;
    }
    let (status, body) = http(&f, &format!("/inventory/costs?sku_id={expensive}"), None).await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "inventory_read_value_overflow");
    let mut query = aggregate_query(2, "cost");
    query["search"] = json!(expensive.to_string());
    let (status, body) = http(&f, "/reports/inventory/aggregate", Some(query)).await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["code"], "inventory_read_value_overflow");
    Ok(())
}

#[tokio::test]
async fn precision_read_aggregate_scales_and_legacy_schema_are_explicit() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let second = warehouse(&pool, &f).await?;
    opening(&pool, &f, second, f.sku, "0.2").await?;
    for (measure, scale, sum) in [("quantity", 6, "2100000"), ("cost", 8, "14700000")] {
        let (status, body) = http(
            &f,
            "/reports/inventory/aggregate",
            Some(aggregate_query(2, measure)),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["schema"], 2);
        assert_eq!(body["amount_scale"], scale);
        assert_eq!(body["total"]["sum"], sum);
        assert_eq!(body["total"]["count"], 2);
        assert_eq!(body["buckets"].as_array().expect("buckets").len(), 2);
        let (status, body) = http(
            &f,
            "/reports/inventory/aggregate",
            Some(aggregate_query(1, measure)),
        )
        .await?;
        assert_eq!(
            status,
            StatusCode::CONFLICT,
            "no silent projection of a schema 1 report: {body}"
        );
        assert_eq!(body["code"], "inventory_aggregate_precision_required");
    }
    let whole_sku = sku(&pool, &f, Some(7), "NORMAL").await?;
    opening(&pool, &f, f.warehouse, whole_sku, "2").await?;
    let mut query = aggregate_query(1, "quantity");
    query["search"] = json!(whole_sku.to_string());
    let (status, body) = http(&f, "/reports/inventory/aggregate", Some(query)).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["amount_scale"], 0);
    assert_eq!(
        body["total"]["sum"], "2",
        "the original integer 2 still means two whole units"
    );
    Ok(())
}

#[tokio::test]
async fn precision_read_missing_sku_and_kit_cost_never_become_zero() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let missing = sku(&pool, &f, None, "NORMAL").await?;
    let kit = sku(&pool, &f, Some(999), "KIT").await?;
    opening(&pool, &f, f.warehouse, missing, "0.4").await?;
    opening(&pool, &f, f.warehouse, kit, "0.25").await?;
    sqlx::query(
        "insert into sku_boms(tenant_id,bundle_sku_id,component_sku_id,qty) values($1,$2,$3,1)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(kit)
    .bind(missing)
    .execute(&pool)
    .await?;
    for id in [missing, kit] {
        let (status, body) = http(&f, &format!("/inventory?sku_id={id}"), None).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["data"][0]["cost_cent"].is_null());
        let (status, body) = http(&f, &format!("/inventory/costs?sku_id={id}"), None).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        for field in ["unit_cost_cent", "total_cost_cent", "estimated_value"] {
            assert!(body["items"][0][field].is_null(), "{field}: {body}");
        }
    }
    let (status, body) = http(
        &f,
        "/reports/inventory/aggregate",
        Some(aggregate_query(2, "cost")),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["missing_count"], 2);
    let (status, body) = http(
        &f,
        "/reports/inventory/aggregate",
        Some(aggregate_query(2, "quantity")),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"]["sum"], "2550000");
    Ok(())
}

#[tokio::test]
async fn precision_read_ledger_preserves_exact_and_original_whole_units() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let legacy = Uuid::new_v4();
    let legacy_sku = sku(&pool, &f, Some(1), "NORMAL").await?;
    sqlx::query("insert into inventory_ledgers(ledger_id,tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,reason_type) values($1,$2,$3,$4,1,0,0,0,'QA_READ_LEGACY')")
        .bind(legacy).bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(legacy_sku).execute(&pool).await?;
    let (status, body) = http(&f, "/inventory/ledgers?limit=1", None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"], 2);
    assert_eq!(body["items"][0]["ledger_id"], legacy.to_string());
    assert_eq!(body["items"][0]["quantities"]["delta_on_hand"], "1");
    let (status, body) = http(&f, "/inventory/ledgers?limit=1&offset=1", None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    for (field, expected) in [
        ("delta_on_hand", "1.9"),
        ("delta_reserved", "0.9"),
        ("delta_frozen", "0.9"),
        ("delta_in_transit", "2.3"),
    ] {
        assert_eq!(body["items"][0]["quantities"][field], expected, "{body}");
    }
    Ok(())
}

#[tokio::test]
async fn precision_read_restricted_warehouse_excludes_unreconciled_outside_grant() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let user = Uuid::new_v4();
    sqlx::query("insert into users(tenant_id,user_id,email,password_hash) values($1,$2,$3,'synthetic-not-for-password-login')")
        .bind(Uuid::from(f.tenant)).bind(user).bind(format!("{user}@inventory-read.invalid")).execute(&pool).await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(user)
    .bind(f.warehouse)
    .execute(&pool)
    .await?;
    let outside = warehouse(&pool, &f).await?;
    sqlx::query(
        "insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand) values($1,$2,$3,99)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(outside)
    .bind(f.sku)
    .execute(&pool)
    .await?;
    for path in [
        "/inventory",
        "/inventory/summary",
        "/inventory/costs",
        "/inventory/ledgers",
    ] {
        let (status, body) = http_as(&f, path, None, user).await?;
        assert_eq!(
            status,
            StatusCode::OK,
            "unknown stock outside the grant cannot poison or leak into the query: {body}"
        );
        let (status, body) =
            http_as(&f, &format!("{path}?warehouse_id={outside}"), None, user).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    }
    let (status, body) = http_as(
        &f,
        "/reports/inventory/aggregate",
        Some(aggregate_query(2, "quantity")),
        user,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["total"]["sum"], "1900000");
    Ok(())
}
