//! Concurrency and actual multi-component rollback on the legacy business routes.
use super::inventory_legacy_reservation_atomicity::{body, fixture, saved};
use super::inventory_location_commands::api;
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use chrono::Utc;
use erp_domain::quantity::parse_qty;
use erp_repo::{
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use serde_json::json;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
use uuid::Uuid;

#[tokio::test]
async fn legacy_reservation_twenty_same_source_and_release_commit_one_log_pair() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, source) = fixture(&pool).await?;
    let f = Arc::new(f);
    sqlx::query("with qa_shop as (insert into shops(tenant_id,shop_id,platform,platform_shop_id,shop_name) values($1,gen_random_uuid(),'qa',gen_random_uuid()::text,'QA mapping shop') returning shop_id) insert into channel_sku_mappings(tenant_id,sku_id,status,shop_id,platform,platform_sku_id) values($1,$2,'ACTIVE',(select shop_id from qa_shop),'qa',$2::text)")
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(&pool)
        .await?;
    for action in ["reservations", "releases"] {
        let mut tasks = JoinSet::new();
        let barrier = Arc::new(Barrier::new(20));
        for _ in 0..20 {
            let f = f.clone();
            let barrier = barrier.clone();
            tasks.spawn(async move {
                barrier.wait().await;
                api(&f, action, body(&f, source), None, None).await
            });
        }
        while let Some(result) = tasks.join_next().await {
            let (status, result) = result??;
            assert_eq!(status, StatusCode::OK, "{result}");
        }
    }
    let now = saved(&independent, &f).await?;
    assert_eq!(now["stock"]["operations"].as_array().map(Vec::len), Some(2));
    assert_eq!(now["stock"]["audits"].as_array().map(Vec::len), Some(2));
    assert_eq!(now["stock"]["outbox"].as_array().map(Vec::len), Some(2));
    assert_eq!(now["stock"]["balances"][0]["reserved_micros"], 900_000);
    assert_eq!(now["reservations"][0]["status"], "RELEASED");
    Ok(())
}

#[tokio::test]
async fn legacy_reservation_twenty_sources_respect_exact_available_and_rollback_rejections()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, _) = fixture(&pool).await?;
    let f = Arc::new(f);
    let mut tasks = JoinSet::new();
    let barrier = Arc::new(Barrier::new(20));
    for _ in 0..20 {
        let f = f.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            api(&f, "reservations", body(&f, Uuid::new_v4()), None, None).await
        });
    }
    let mut success = 0;
    let mut rejected = 0;
    while let Some(result) = tasks.join_next().await {
        let (status, result) = result??;
        if status == StatusCode::OK {
            success += 1;
        } else {
            assert_eq!(status, StatusCode::CONFLICT, "{result}");
            assert_eq!(result["code"], "inventory_insufficient");
            rejected += 1;
        }
    }
    assert_eq!((success, rejected), (1, 19));
    let now = saved(&independent, &f).await?;
    assert_eq!(now["reservations"].as_array().map(Vec::len), Some(1));
    assert_eq!(now["stock"]["balances"][0]["reserved_micros"], 1_900_000);
    assert_eq!(now["stock"]["operations"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[tokio::test]
async fn legacy_reservation_current_revocation_blocks_replay_and_release() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, source) = fixture(&pool).await?;
    let (status, result) = api(&f, "reservations", body(&f, source), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    sqlx::query(
        "delete from user_warehouse_access where tenant_id=$1 and user_id=$2 and warehouse_id=$3",
    )
    .bind(f.tenant.0)
    .bind(f.actor)
    .bind(f.warehouse)
    .execute(&pool)
    .await?;
    let before = saved(&independent, &f).await?;
    for action in ["reservations", "releases"] {
        let (status, result) = api(&f, action, body(&f, source), None, None).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{result}");
    }
    assert_eq!(saved(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn legacy_reservation_invalid_quantity_identity_and_canonical_aliases() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, source) = fixture(&pool).await?;
    let before = saved(&independent, &f).await?;
    for (field, value) in [
        ("quantity", json!(0)),
        ("quantity", json!(-1)),
        ("quantity", json!(i64::MAX)),
        ("order_item_id", json!(Uuid::nil())),
        ("order_item_id", json!("bad-source")),
        ("order_id", json!(Uuid::nil())),
    ] {
        let mut request = body(&f, source);
        request[field] = value;
        let (status, result) = api(&f, "reservations", request, None, None).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}: {result}");
    }
    assert_eq!(saved(&independent, &f).await?, before);
    let mut request = body(&f, source);
    request["reference_id"] = json!(source.to_string().to_uppercase());
    let (status, result) = api(&f, "reservations", request, None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let before = saved(&independent, &f).await?;
    let mut wrong = body(&f, source);
    wrong["reference_id"] = json!(Uuid::new_v4());
    let (status, result) = api(&f, "releases", wrong, None, None).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
    assert_eq!(saved(&independent, &f).await?, before);
    Ok(())
}

async fn kit_log_failure(table: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (mut f, source) = fixture(&pool).await?;
    let first = f.sku;
    let second = Uuid::new_v4();
    let parent = Uuid::new_v4();
    for (sku, code, kind) in [
        (second, "QA-LEGACY-PART-B", "NORMAL"),
        (parent, "QA-LEGACY-KIT", "KIT"),
    ] {
        sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,sku_kind,product_id) values($1,$2,$3,$4,(select product_id from qa_parent))")
            .bind(f.tenant.0)
            .bind(sku)
            .bind(code)
            .bind(kind)
            .execute(&pool)
            .await?;
    }
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: second,
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
    for sku in [first, second] {
        sqlx::query(
            "insert into sku_boms(tenant_id,bundle_sku_id,component_sku_id,qty) values($1,$2,$3,1)",
        )
        .bind(f.tenant.0)
        .bind(parent)
        .bind(sku)
        .execute(&pool)
        .await?;
        sqlx::query(
            "with qa_shop as (insert into shops(tenant_id,shop_id,platform,platform_shop_id,shop_name) values($1,gen_random_uuid(),'qa',gen_random_uuid()::text,'QA mapping shop') returning shop_id) insert into channel_sku_mappings(tenant_id,sku_id,status,shop_id,platform,platform_sku_id) values($1,$2,'ACTIVE',(select shop_id from qa_shop),'qa',$2::text)",
        )
        .bind(f.tenant.0)
        .bind(sku)
        .execute(&pool)
        .await?;
    }
    f.sku = parent;
    eprintln!(
        "QA legacy reservation kit tenant={} parent={parent} first={first} second={second} source={source}",
        f.tenant
    );
    let schema = format!("qa_legacy_kit_{}", Uuid::new_v4().simple());
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
        "QA legacy reservation failure tenant={} action=reservations schema={schema} table={table}",
        f.tenant
    );
    let before = saved(&independent, &f).await?;
    let (status, result) = api(&f, "reservations", body(&f, source), None, Some(url)).await?;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{result}");
    assert!(
        result["message"]
            .as_str()
            .or_else(|| result["text"].as_str())
            .is_some_and(|v| v.contains("check option") && v.contains(table)),
        "{result}"
    );
    let after = saved(&independent, &f).await?;
    eprintln!(
        "QA legacy reservation rollback tenant={} action=reservations table={table} before={before} after={after}",
        f.tenant
    );
    assert_eq!(
        after, before,
        "both component sources, stock, logs and Outbox must roll back"
    );
    let (status, result) = api(&f, "reservations", body(&f, source), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_ne!(
        result["record"]["sku_id"],
        parent.to_string(),
        "legacy compatibility returns the last component balance"
    );
    let now = saved(&independent, &f).await?;
    assert_eq!(now["reservations"].as_array().map(Vec::len), Some(2));
    assert_eq!(now["stock"]["operations"].as_array().map(Vec::len), Some(1));
    assert_eq!(now["stock"]["audits"].as_array().map(Vec::len), Some(1));
    assert_eq!(now["stock"]["outbox"].as_array().map(Vec::len), Some(2));
    Ok(())
}

#[tokio::test]
async fn legacy_reservation_kit_operation_log_fault_rolls_back_both_components() -> Result<()> {
    kit_log_failure("operation_logs").await
}
#[tokio::test]
async fn legacy_reservation_kit_audit_log_fault_rolls_back_both_components() -> Result<()> {
    kit_log_failure("audit_logs").await
}

#[tokio::test]
async fn legacy_reservation_mixed_precise_movements_share_physical_lock_order() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, _) = fixture(&pool).await?;
    let f = Arc::new(f);
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for index in 0..20 {
        let f = f.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
        let (action,request)=if index<10 {("reservations",body(&f,Uuid::new_v4()))}else {
            ("inbounds",json!({"request_id":Uuid::new_v4(),"warehouse_id":f.warehouse,"sku_id":f.sku,"quantity":1}))
        };barrier.wait().await;let result=api(&f,action,request,None,None).await?;Ok::<_,anyhow::Error>((action,result))
    });
    }
    let mut inbound = 0;
    let mut reserved = 0;
    while let Some(result) = tasks.join_next().await {
        let (action, (status, result)) = result??;
        if action == "inbounds" {
            assert_eq!(status, StatusCode::OK, "{result}");
            inbound += 1;
        } else if status == StatusCode::OK {
            reserved += 1;
        } else {
            assert_eq!(status, StatusCode::CONFLICT, "{result}");
            assert_eq!(result["code"], "inventory_insufficient");
        }
    }
    assert_eq!(inbound, 10);
    assert!((1..=10).contains(&reserved));
    let now = saved(&independent, &f).await?;
    assert_eq!(now["stock"]["balances"][0]["on_hand_micros"], 12_900_000);
    assert_eq!(
        now["stock"]["balances"][0]["reserved_micros"],
        900_000 + reserved * 1_000_000
    );
    assert_eq!(
        now["reservations"].as_array().map(Vec::len),
        Some(reserved as usize)
    );
    eprintln!(
        "QA legacy reservation mixed tenant={} inbound={inbound} reserved={reserved}",
        f.tenant
    );
    Ok(())
}
