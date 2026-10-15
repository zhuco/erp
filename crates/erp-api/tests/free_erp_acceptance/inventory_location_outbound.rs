//! Real structured outbound must consume only this location's free GOOD stock.
use super::inventory_location_commands::{Fixture, api, api_headers, facts, movement};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use chrono::Utc;
use erp_domain::quantity::ScaledQty;
use erp_repo::{
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
use uuid::Uuid;

async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let f = super::inventory_location_commands::fixture(pool).await?;
    sqlx::query("update tenants set name='QA-INVENTORY-LOCATION-OUTBOUND' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    eprintln!(
        "QA location outbound fixture tenant={} actor={} warehouse={} sku={} location={}",
        f.tenant, f.actor, f.warehouse, f.sku, f.location
    );
    Ok(f)
}

async fn inbound(f: &Fixture, quantity: i64) -> Result<Value> {
    let (status, result) = api(
        f,
        "inbounds",
        movement(f, &Uuid::new_v4().to_string(), quantity),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "seed actual inbound: {result}");
    Ok(result)
}

async fn another_location(pool: &PgPool, f: &mut Fixture) -> Result<Uuid> {
    let previous = f.location;
    f.location = Uuid::new_v4();
    sqlx::query("insert into warehouse_locations(tenant_id,warehouse_id,location_id,code) values($1,$2,$3,$4)")
        .bind(f.tenant.0).bind(f.warehouse).bind(f.location).bind(f.location.to_string())
        .execute(pool).await?;
    Ok(previous)
}

/// A synthetic opening/reservation is a real, linked precision effect, not a
/// fabricated business command or an independently overwritten balance.
async fn seed_location_delta(pool: &PgPool, f: &Fixture, bucket: &str, micros: i64) -> Result<()> {
    let source = Uuid::new_v4();
    let amount = ScaledQty::from_micros(micros);
    let mut tx = pool.begin().await?;
    let delta = InventoryPreciseDelta {
        key: InventoryQuantityKey {
            tenant_id: f.tenant,
            warehouse_id: f.warehouse,
            sku_id: f.sku,
        },
        on_hand: if bucket == "ON_HAND" {
            amount
        } else {
            ScaledQty::ZERO
        },
        reserved: if bucket == "RESERVED" {
            amount
        } else {
            ScaledQty::ZERO
        },
        frozen: if bucket == "FROZEN" {
            amount
        } else {
            ScaledQty::ZERO
        },
        in_transit: ScaledQty::ZERO,
        reason_type: "STOCK_COUNT_ADJUST".into(),
        ref_type: Some("shared_stock_ledger".into()),
        ref_id: Some(source.to_string()),
        occurred_at: Utc::now(),
    };
    InventoryService::new(pool)
        .apply_precise_with_tx(&mut tx, delta)
        .await?;
    sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,'QA-BATCH','GOOD',$6,$7,'QA-SEED','QA-SEED')")
        .bind(f.tenant.0).bind(source).bind(f.warehouse).bind(f.location).bind(f.sku)
        .bind(bucket).bind(micros).execute(&mut *tx).await?;
    tx.commit().await?;
    eprintln!(
        "QA location outbound linked seed tenant={} ledger_id={source} location={} bucket={bucket} micros={micros}",
        f.tenant, f.location
    );
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_commits_both_stock_effects_and_original_result() -> Result<()>
{
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    inbound(&f, 3).await?;
    let id = Uuid::new_v4().to_string();
    let original = movement(&f, &id, 1);
    let (status, result) = api(&f, "outbounds", original.clone(), None, None).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "structured issue must complete: {result}"
    );
    assert_eq!(result["record"]["quantities"]["on_hand"], "2");
    assert_eq!(result["location_record"]["on_hand"], "2");
    assert_eq!(result["qty_micros"], -1_000_000);
    let saved = facts(&independent, &f).await?;
    assert_eq!(saved["balances"][0]["on_hand_micros"], 2_000_000);
    for field in [
        "ledgers",
        "shared_ledgers",
        "receipts",
        "operations",
        "audits",
    ] {
        assert_eq!(saved[field].as_array().map(Vec::len), Some(2), "{field}");
    }
    for (action, body) in [
        ("outbounds".to_owned(), original),
        (format!("commands/{id}"), Value::Null),
    ] {
        let (status, replay) = api(&f, &action, body, None, None).await?;
        assert_eq!(status, StatusCode::OK, "{replay}");
        let mut expected = result.clone();
        expected["replay"] = json!(true);
        assert_eq!(replay, expected);
    }
    assert_eq!(facts(&independent, &f).await?, saved);
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_does_not_consume_another_location() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let mut f = fixture(&pool).await?;
    inbound(&f, 1).await?;
    let original_location = f.location;
    f.location = Uuid::new_v4();
    sqlx::query("insert into warehouse_locations(tenant_id,warehouse_id,location_id,code) values($1,$2,$3,'QA-OTHER')")
        .bind(f.tenant.0).bind(f.warehouse).bind(f.location).execute(&pool).await?;
    inbound(&f, 5).await?;
    f.location = original_location;
    let (status, result) = api(
        &f,
        "outbounds",
        movement(&f, &Uuid::new_v4().to_string(), 1),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["record"]["quantities"]["on_hand"], "5");
    assert_eq!(result["location_record"]["on_hand"], "0");
    let before = facts(&independent, &f).await?;
    let (status, result) = api(
        &f,
        "outbounds",
        movement(&f, &Uuid::new_v4().to_string(), 1),
        None,
        None,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "empty source location: {result}"
    );
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_full_payload_and_actor_are_bound() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    inbound(&f, 5).await?;
    let id = Uuid::new_v4().to_string();
    let original = movement(&f, &id, 1);
    let (status, result) = api(&f, "outbounds", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let before = facts(&independent, &f).await?;
    for (field, value) in [
        ("quantity", json!(2)),
        ("sku_id", json!(Uuid::new_v4())),
        ("location_id", json!(Uuid::new_v4())),
        ("batch_code", json!("ANOTHER")),
        ("quality_status", json!("HOLD")),
        ("reason", json!("ANOTHER")),
        ("reference_id", json!("ANOTHER")),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        let (status, result) = api(&f, "outbounds", changed, None, None).await?;
        assert_eq!(status, StatusCode::CONFLICT, "{field}: {result}");
        assert_eq!(facts(&independent, &f).await?, before);
    }
    let actor = super::account_login::paid_peer(
        &pool,
        f.tenant,
        f.warehouse,
        super::account_login::STOCK_PERMISSIONS,
    )
    .await?;
    let (status, result) = api(&f, "outbounds", original.clone(), Some(&actor), None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    let (status, result) = api(
        &f,
        &format!("commands/{id}"),
        Value::Null,
        Some(&actor),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND, "{result}");
    let (status, result) = api(&f, "inbounds", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    let basic = json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"quantity":1,"request_id":id,
        "reason":"QA_LOCATION","reference_id":"QA-LOCATION-REF"});
    let (status, result) = api(&f, "outbounds", basic, None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_original_fractional_results_survive_later_stock() -> Result<()>
{
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    inbound(&f, 4).await?;
    seed_location_delta(&pool, &f, "ON_HAND", 900_000).await?;
    let id = Uuid::new_v4().to_string();
    let original = movement(&f, &id, 1);
    let (status, first) = api(&f, "outbounds", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["record"]["quantities"]["on_hand"], "3.9");
    assert_eq!(first["location_record"]["on_hand"], "3.9");
    inbound(&f, 2).await?;
    let current = facts(&independent, &f).await?;
    assert_eq!(current["balances"][0]["on_hand_micros"], 5_900_000);
    for (action, body) in [
        ("outbounds".to_owned(), original),
        (format!("commands/{id}"), Value::Null),
    ] {
        let (status, result) = api(&f, &action, body, None, None).await?;
        assert_eq!(status, StatusCode::OK, "{result}");
        let mut expected = first.clone();
        expected["replay"] = json!(true);
        assert_eq!(result, expected);
    }
    assert_eq!(facts(&independent, &f).await?, current);
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_subtracts_location_reserved_and_frozen() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let mut f = fixture(&pool).await?;
    inbound(&f, 1).await?;
    seed_location_delta(&pool, &f, "RESERVED", 300_000).await?;
    seed_location_delta(&pool, &f, "FROZEN", 200_000).await?;
    let local = another_location(&pool, &mut f).await?;
    inbound(&f, 5).await?;
    f.location = local;
    let before = facts(&independent, &f).await?;
    assert_eq!(before["balances"][0]["on_hand_micros"], 6_000_000);
    let (status, result) = api(
        &f,
        "outbounds",
        movement(&f, &Uuid::new_v4().to_string(), 1),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    assert_eq!(result["code"], "inventory_location_insufficient_available");
    assert_eq!(result["available_micros"], "500000");
    assert_eq!(result["requested_micros"], "1000000");
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_non_good_cannot_use_good_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    inbound(&f, 3).await?;
    for quality in ["PENDING", "FAILED", "HOLD"] {
        let mut incoming = movement(&f, &Uuid::new_v4().to_string(), 2);
        incoming["quality_status"] = json!(quality);
        let (status, result) = api(&f, "inbounds", incoming, None, None).await?;
        assert_eq!(status, StatusCode::OK, "{result}");
        let before = facts(&independent, &f).await?;
        let mut outgoing = movement(&f, &Uuid::new_v4().to_string(), 1);
        outgoing["quality_status"] = json!(quality);
        let (status, result) = api(&f, "outbounds", outgoing, None, None).await?;
        assert_eq!(status, StatusCode::CONFLICT, "{quality}: {result}");
        assert_eq!(result["code"], "inventory_location_quality_not_issuable");
        assert_eq!(facts(&independent, &f).await?, before);
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_twenty_same_id_have_one_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(fixture(&pool).await?);
    inbound(&f, 3).await?;
    let id = Uuid::new_v4().to_string();
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for index in 0..20 {
        let f = f.clone();
        let id = id.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            let mut body = movement(&f, &id, 1);
            let headers = if index % 2 == 0 {
                Vec::new()
            } else {
                body.as_object_mut()
                    .ok_or_else(|| anyhow::anyhow!("fixture object"))?
                    .remove("request_id");
                vec![id]
            };
            barrier.wait().await;
            api_headers(&f, "outbounds", body, None, None, headers).await
        });
    }
    let mut first_count = 0;
    let mut original = None;
    while let Some(result) = tasks.join_next().await {
        let (status, mut result) = result??;
        assert_eq!(status, StatusCode::OK, "{result}");
        first_count += usize::from(result["replay"] == false);
        result["replay"] = json!(true);
        if let Some(original) = &original {
            assert_eq!(&result, original);
        } else {
            original = Some(result);
        }
    }
    assert_eq!(first_count, 1);
    let saved = facts(&independent, &f).await?;
    assert_eq!(saved["balances"][0]["on_hand_micros"], 2_000_000);
    for field in [
        "ledgers",
        "shared_ledgers",
        "receipts",
        "operations",
        "audits",
    ] {
        assert_eq!(saved[field].as_array().map(Vec::len), Some(2), "{field}");
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_twenty_distinct_ids_do_not_borrow_other_location() -> Result<()>
{
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let mut f = fixture(&pool).await?;
    inbound(&f, 3).await?;
    let local = another_location(&pool, &mut f).await?;
    inbound(&f, 100).await?;
    f.location = local;
    let f = Arc::new(f);
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let f = f.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            let body = movement(&f, &Uuid::new_v4().to_string(), 1);
            barrier.wait().await;
            api(&f, "outbounds", body, None, None).await
        });
    }
    let mut accepted = 0;
    let mut rejected = 0;
    while let Some(result) = tasks.join_next().await {
        let (status, result) = result??;
        if status == StatusCode::OK {
            accepted += 1;
        } else {
            assert_eq!(status, StatusCode::CONFLICT, "{result}");
            assert_eq!(result["code"], "inventory_location_insufficient_available");
            rejected += 1;
        }
    }
    assert_eq!((accepted, rejected), (3, 17));
    let saved = facts(&independent, &f).await?;
    assert_eq!(saved["balances"][0]["on_hand_micros"], 100_000_000);
    let location_hand: i64 = sqlx::query_scalar("select coalesce(sum(delta_micros),0)::bigint from shared_stock_ledgers where tenant_id=$1 and location_id=$2 and bucket='ON_HAND'")
        .bind(f.tenant.0).bind(f.location).fetch_one(&independent).await?;
    assert_eq!(location_hand, 0);
    for field in [
        "ledgers",
        "shared_ledgers",
        "receipts",
        "operations",
        "audits",
    ] {
        assert_eq!(saved[field].as_array().map(Vec::len), Some(5), "{field}");
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_actual_log_fault_rolls_back_every_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for table in ["operation_logs", "audit_logs"] {
        let f = fixture(&pool).await?;
        inbound(&f, 3).await?;
        sqlx::query(
            "with qa_shop as (insert into shops(tenant_id,shop_id,platform,platform_shop_id,shop_name) values($1,gen_random_uuid(),'qa',gen_random_uuid()::text,'QA mapping shop') returning shop_id) insert into channel_sku_mappings(tenant_id,sku_id,status,shop_id,platform,platform_sku_id) values($1,$2,'ACTIVE',(select shop_id from qa_shop),'qa',$2::text)",
        )
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(&pool)
        .await?;
        let schema = format!("qa_location_outbound_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("create schema {schema}"))
            .execute(&pool)
            .await?;
        sqlx::query(&format!("create view {schema}.{table} as select * from public.{table} where false with local check option"))
            .execute(&pool).await?;
        let original = movement(&f, &Uuid::new_v4().to_string(), 1);
        let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
        let separator = if url.contains('?') { '&' } else { '?' };
        let url =
            format!("{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog");
        assert!(super::approved_database_url(&url)?);
        eprintln!(
            "QA location outbound failure tenant={} schema={schema} table={table}",
            f.tenant
        );
        let before = facts(&independent, &f).await?;
        let (status, result) = api(&f, "outbounds", original.clone(), None, Some(url)).await?;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{result}");
        let message = result["message"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing log failure"))?;
        assert!(
            message.contains("check option") && message.contains(table),
            "{message}"
        );
        assert_eq!(facts(&independent, &f).await?, before);
        let (status, result) = api(&f, "outbounds", original, None, None).await?;
        assert_eq!(
            status,
            StatusCode::OK,
            "original ID after proved rollback: {result}"
        );
        let saved = facts(&independent, &f).await?;
        assert_eq!(saved["outbox"].as_array().map(Vec::len), Some(1));
        assert_eq!(saved["outbox"][0]["payload"]["available_micros"], "2000000");
        assert_eq!(saved["balances"][0]["on_hand_micros"], 2_000_000);
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_identity_validation_and_current_scope() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    inbound(&f, 3).await?;
    let id = Uuid::new_v4().to_string();
    let original = movement(&f, &id, 1);
    let before = facts(&independent, &f).await?;
    for field in ["request_id", "location_id", "batch_code", "quality_status"] {
        let mut body = original.clone();
        body.as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("fixture object"))?
            .remove(field);
        let (status, result) = api(&f, "outbounds", body, None, None).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "missing {field}: {result}");
        assert_eq!(facts(&independent, &f).await?, before);
    }
    for headers in [
        vec![id.clone(), id.clone()],
        vec![Uuid::new_v4().to_string()],
    ] {
        let (status, result) = api_headers(
            &f,
            "outbounds",
            original.clone(),
            None,
            None,
            headers.clone(),
        )
        .await?;
        assert_eq!(
            status,
            if headers.len() == 2 {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::CONFLICT
            },
            "{result}"
        );
        assert_eq!(facts(&independent, &f).await?, before);
    }
    let (status, result) = api(&f, "outbounds", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    sqlx::query(
        "update warehouse_locations set is_disabled=true where tenant_id=$1 and location_id=$2",
    )
    .bind(f.tenant.0)
    .bind(f.location)
    .execute(&pool)
    .await?;
    let before = facts(&independent, &f).await?;
    let (status, result) = api(
        &f,
        "outbounds",
        movement(&f, &Uuid::new_v4().to_string(), 1),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
    let (status, result) = api(&f, "outbounds", original.clone(), None, None).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "already committed result after disabling: {result}"
    );
    assert_eq!(facts(&independent, &f).await?, before);
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.actor)
        .execute(&pool)
        .await?;
    for (action, body) in [
        ("outbounds".to_owned(), original),
        (format!("commands/{id}"), Value::Null),
    ] {
        let (status, result) = api(&f, &action, body, None, None).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{result}");
    }
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_location_outbound_unproven_history_is_not_sold_or_activated() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,'QA-BATCH','GOOD','ON_HAND',5000000,'QA-UNLINKED','QA-UNLINKED')")
        .bind(f.tenant.0).bind(Uuid::new_v4()).bind(f.warehouse).bind(f.location).bind(f.sku)
        .execute(&pool).await?;
    let before = facts(&independent, &f).await?;
    let (status, result) = api(
        &f,
        "outbounds",
        movement(&f, &Uuid::new_v4().to_string(), 1),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    assert_eq!(result["code"], "inventory_reconciliation_required");
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}
