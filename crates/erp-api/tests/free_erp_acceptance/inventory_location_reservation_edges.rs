//! Real source keys, original results, contention, permissions and late rollback.
use super::inventory_location_commands::{Fixture, api, api_headers, facts, movement};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
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
    let (status, result) = api(
        &f,
        "inbounds",
        movement(&f, &Uuid::new_v4().to_string(), 5),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    Ok(f)
}
fn command(f: &Fixture, id: &str, source: &str, quantity: i64) -> Value {
    json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"request_id":id,"quantity":quantity,
        "reference_id":source,"location_id":f.location,"batch_code":"QA-BATCH","quality_status":"GOOD"})
}
async fn saved(pool: &PgPool, f: &Fixture) -> Result<Value> {
    Ok(
        json!({"stock":facts(pool,f).await?,"sources":sqlx::query_scalar::<_,Value>("select jsonb_build_object(
        'reservations',coalesce((select jsonb_agg(to_jsonb(r) order by source_id,reservation_id) from inventory_reservations r where tenant_id=$1),'[]'::jsonb),
        'events',coalesce((select jsonb_agg(to_jsonb(e) order by request_id) from inventory_location_reservation_events e where tenant_id=$1),'[]'::jsonb))")
        .bind(f.tenant.0).fetch_one(pool).await?}),
    )
}
async fn ok(f: &Fixture, action: &str, body: Value) -> Result<Value> {
    let (status, result) = api(f, action, body, None, None).await?;
    assert_eq!(status, StatusCode::OK, "{action}: {result}");
    Ok(result)
}

#[tokio::test]
async fn inventory_location_reservation_partial_release_retains_originals_and_other_source()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let reserve_id = Uuid::new_v4().to_string();
    let original = command(&f, &reserve_id, "SOURCE-X", 2);
    let reserve = ok(&f, "reservations", original.clone()).await?;
    ok(
        &f,
        "reservations",
        command(&f, &Uuid::new_v4().to_string(), "SOURCE-Y", 1),
    )
    .await?;
    let release_id = Uuid::new_v4().to_string();
    let first = command(&f, &release_id, "SOURCE-X", 1);
    let release = ok(&f, "releases", first.clone()).await?;
    assert_eq!(release["reservation_record"]["remaining_quantity"], "1");
    assert_eq!(release["record"]["quantities"]["reserved"], "2");
    ok(
        &f,
        "releases",
        command(&f, &Uuid::new_v4().to_string(), "SOURCE-X", 1),
    )
    .await?;
    for (action, body, expected) in [
        ("reservations".to_owned(), original, reserve),
        ("releases".to_owned(), first, release.clone()),
        (format!("commands/{release_id}"), Value::Null, release),
    ] {
        let replay = ok(&f, &action, body).await?;
        let mut expected = expected;
        expected["replay"] = json!(true);
        assert_eq!(
            replay, expected,
            "immutable original after later source release"
        );
    }
    let now = saved(&independent, &f).await?;
    assert_eq!(now["stock"]["balances"][0]["reserved_micros"], 1_000_000);
    let sources = now["sources"]["reservations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("sources"))?;
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0]["source_id"], "SOURCE-X");
    assert_eq!(sources[0]["qty_micros"], 0);
    assert_eq!(sources[0]["original_qty_micros"], 2_000_000);
    assert_eq!(sources[0]["released_micros"], 2_000_000);
    assert_eq!(sources[1]["source_id"], "SOURCE-Y");
    assert_eq!(sources[1]["qty_micros"], 1_000_000);
    Ok(())
}

#[tokio::test]
async fn inventory_location_reservation_identity_source_key_and_validation_fences() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let other_sku = Uuid::new_v4();
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-SOURCE-OTHER-SKU',(select product_id from qa_parent))")
        .bind(f.tenant.0)
        .bind(other_sku)
        .execute(&pool)
        .await?;
    let id = Uuid::new_v4().to_string();
    let original = command(&f, &id, "SOURCE-X", 2);
    ok(&f, "reservations", original.clone()).await?;
    let before = saved(&independent, &f).await?;
    for (field, value) in [
        ("quantity", json!(1)),
        ("reference_id", json!("SOURCE-Y")),
        ("location_id", json!(Uuid::new_v4())),
        ("batch_code", json!("other")),
        ("sku_id", json!(Uuid::new_v4())),
        ("order_id", json!(Uuid::new_v4())),
    ] {
        let mut body = original.clone();
        body[field] = value;
        let (status, result) = api(&f, "reservations", body, None, None).await?;
        assert_eq!(status, StatusCode::CONFLICT, "{field}: {result}");
        assert_eq!(result["code"], "inventory_command_identity_conflict");
    }
    let (status, result) = api(&f, "releases", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    let mut body = original.clone();
    body["request_id"] = json!(Uuid::new_v4());
    let (status, result) = api(&f, "reservations", body.clone(), None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    assert_eq!(result["code"], "inventory_reservation_source_conflict");
    for (field, value) in [
        ("sku_id", json!(other_sku)),
        ("location_id", json!(f.location)),
        ("batch_code", json!("other")),
        ("reference_id", json!("MISSING")),
        ("order_id", json!(Uuid::new_v4())),
    ] {
        let mut changed = body.clone();
        changed[field] = value;
        if field == "location_id" {
            changed["batch_code"] = json!("other-location-key");
        }
        let (status, result) = api(&f, "releases", changed, None, None).await?;
        assert_eq!(status, StatusCode::CONFLICT, "{field}: {result}");
    }
    for (field, value, expected) in [
        ("quality_status", json!("HOLD"), StatusCode::CONFLICT),
        ("quality_status", json!("BAD"), StatusCode::BAD_REQUEST),
        ("batch_code", json!(""), StatusCode::BAD_REQUEST),
        ("quantity", json!(0), StatusCode::BAD_REQUEST),
        ("quantity", json!(i64::MAX), StatusCode::BAD_REQUEST),
        (
            "order_item_id",
            json!("other-source"),
            StatusCode::BAD_REQUEST,
        ),
        (
            "ignored_field",
            json!(true),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let mut changed = body.clone();
        changed[field] = value;
        let (status, result) = api(&f, "reservations", changed, None, None).await?;
        assert_eq!(status, expected, "{field}: {result}");
    }
    let (status, result) = api_headers(
        &f,
        "releases",
        body.clone(),
        None,
        None,
        vec!["different".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    let (status, result) = api_headers(
        &f,
        "releases",
        body,
        None,
        None,
        vec!["a".into(), "b".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
    assert_eq!(saved(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_location_reservation_twenty_same_identity_header_and_body() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(fixture(&pool).await?);
    let id = Uuid::new_v4().to_string();
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for index in 0..20 {
        let f = f.clone();
        let id = id.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            let mut body = command(&f, &id, "SOURCE-X", 2);
            let headers = if index < 10 {
                body.as_object_mut()
                    .ok_or_else(|| anyhow::anyhow!("body"))?
                    .remove("request_id");
                vec![id]
            } else {
                Vec::new()
            };
            barrier.wait().await;
            api_headers(&f, "reservations", body, None, None, headers).await
        });
    }
    let mut first = 0;
    let mut replay = 0;
    while let Some(result) = tasks.join_next().await {
        let (status, body) = result??;
        assert_eq!(status, StatusCode::OK, "{body}");
        if body["replay"] == false {
            first += 1;
        } else {
            replay += 1;
        }
    }
    assert_eq!((first, replay), (1, 19));
    let now = saved(&independent, &f).await?;
    assert_eq!(
        now["sources"]["reservations"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(now["sources"]["events"].as_array().map(Vec::len), Some(1));
    assert_eq!(now["stock"]["balances"][0]["reserved_micros"], 2_000_000);
    Ok(())
}

#[tokio::test]
async fn inventory_location_reservation_twenty_sources_and_releases_never_overallocate()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(fixture(&pool).await?);
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for index in 0..20 {
        let f = f.clone();
        let barrier = barrier.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            api(
                &f,
                "reservations",
                command(
                    &f,
                    &Uuid::new_v4().to_string(),
                    &format!("SOURCE-{index:02}"),
                    1,
                ),
                None,
                None,
            )
            .await
        });
    }
    let mut success = 0;
    let mut rejected = 0;
    let mut source = String::new();
    while let Some(result) = tasks.join_next().await {
        let (status, body) = result??;
        if status == StatusCode::OK {
            success += 1;
            source = body["source_line"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("source"))?
                .to_owned();
        } else {
            assert_eq!(status, StatusCode::CONFLICT, "{body}");
            assert_eq!(body["code"], "inventory_location_insufficient_available");
            rejected += 1;
        }
    }
    assert_eq!((success, rejected), (5, 15));
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let f = f.clone();
        let barrier = barrier.clone();
        let source = source.clone();
        tasks.spawn(async move {
            barrier.wait().await;
            api(
                &f,
                "releases",
                command(&f, &Uuid::new_v4().to_string(), &source, 1),
                None,
                None,
            )
            .await
        });
    }
    success = 0;
    rejected = 0;
    while let Some(result) = tasks.join_next().await {
        let (status, body) = result??;
        if status == StatusCode::OK {
            success += 1;
        } else {
            assert_eq!(status, StatusCode::CONFLICT, "{body}");
            assert_eq!(body["code"], "inventory_reservation_insufficient_remaining");
            rejected += 1;
        }
    }
    assert_eq!((success, rejected), (1, 19));
    let now = saved(&independent, &f).await?;
    assert_eq!(now["stock"]["balances"][0]["reserved_micros"], 4_000_000);
    assert_eq!(now["sources"]["events"].as_array().map(Vec::len), Some(6));
    Ok(())
}

#[tokio::test]
async fn inventory_location_reservation_log_fault_rolls_back_source_projection_and_outbox()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for table in ["operation_logs", "audit_logs"] {
        let f = fixture(&pool).await?;
        sqlx::query(
            "with qa_shop as (insert into shops(tenant_id,shop_id,platform,platform_shop_id,shop_name) values($1,gen_random_uuid(),'qa',gen_random_uuid()::text,'QA mapping shop') returning shop_id) insert into channel_sku_mappings(tenant_id,sku_id,status,shop_id,platform,platform_sku_id) values($1,$2,'ACTIVE',(select shop_id from qa_shop),'qa',$2::text)",
        )
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(&pool)
        .await?;
        let schema = format!("qa_location_reservation_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("create schema {schema}"))
            .execute(&pool)
            .await?;
        sqlx::query(&format!("create view {schema}.{table} as select * from public.{table} where false with local check option"))
            .execute(&pool).await?;
        let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
        let separator = if url.contains('?') { '&' } else { '?' };
        let url =
            format!("{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog");
        assert!(super::approved_database_url(&url)?);
        eprintln!(
            "QA location reservation failure tenant={} schema={schema} table={table}",
            f.tenant
        );
        for (action, quantity) in [("reservations", 2), ("releases", 1)] {
            let original = command(&f, &Uuid::new_v4().to_string(), "SOURCE-X", quantity);
            let before = saved(&independent, &f).await?;
            let (status, result) =
                api(&f, action, original.clone(), None, Some(url.clone())).await?;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{result}");
            let message = result["message"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("fault message"))?;
            assert!(
                message.contains("check option") && message.contains(table),
                "{message}"
            );
            assert_eq!(
                saved(&independent, &f).await?,
                before,
                "late {action} fault must roll everything back"
            );
            ok(&f, action, original).await?;
        }
        let now = saved(&independent, &f).await?;
        assert_eq!(now["stock"]["outbox"].as_array().map(Vec::len), Some(2));
        assert_eq!(now["sources"]["events"].as_array().map(Vec::len), Some(2));
        assert_eq!(now["stock"]["balances"][0]["reserved_micros"], 1_000_000);
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_reservation_current_access_actor_and_disabled_location() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let original = command(&f, &id, "SOURCE-X", 2);
    ok(&f, "reservations", original.clone()).await?;
    let actor = super::account_login::paid_peer(
        &pool,
        f.tenant,
        f.warehouse,
        super::account_login::STOCK_PERMISSIONS,
    )
    .await?;
    let (status, result) = api(&f, "reservations", original.clone(), Some(&actor), None).await?;
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
    sqlx::query(
        "update warehouse_locations set is_disabled=true where tenant_id=$1 and location_id=$2",
    )
    .bind(f.tenant.0)
    .bind(f.location)
    .execute(&pool)
    .await?;
    let before = saved(&independent, &f).await?;
    let (status, result) = api(
        &f,
        "reservations",
        command(&f, &Uuid::new_v4().to_string(), "SOURCE-Y", 1),
        None,
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
    assert_eq!(saved(&independent, &f).await?, before);
    ok(&f, "reservations", original.clone()).await?;
    ok(
        &f,
        "releases",
        command(&f, &Uuid::new_v4().to_string(), "SOURCE-X", 1),
    )
    .await?;
    sqlx::query(
        "delete from user_warehouse_access where tenant_id=$1 and user_id=$2 and warehouse_id=$3",
    )
    .bind(f.tenant.0)
    .bind(f.actor)
    .bind(f.warehouse)
    .execute(&pool)
    .await?;
    let before = saved(&independent, &f).await?;
    for (action, body) in [
        ("reservations".to_owned(), original),
        (format!("commands/{id}"), Value::Null),
        (
            "releases".to_owned(),
            command(&f, &Uuid::new_v4().to_string(), "SOURCE-X", 1),
        ),
    ] {
        let (status, result) = api(&f, &action, body, None, None).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{result}");
    }
    assert_eq!(saved(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_location_reservation_source_and_events_cannot_be_rewritten() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let result = ok(
        &f,
        "reservations",
        command(&f, &Uuid::new_v4().to_string(), "SOURCE-X", 2),
    )
    .await?;
    let reservation = Uuid::parse_str(
        result["reservation_record"]["reservation_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("reservation"))?,
    )?;
    let before = saved(&independent, &f).await?;
    for statement in [
        "update inventory_reservations set batch_code='changed' where tenant_id=$1 and reservation_id=$2",
        "update inventory_reservations set original_qty_micros=3000000 where tenant_id=$1 and reservation_id=$2",
        "update inventory_reservations set qty_micros=1000000,qty=1,released_micros=1000000 where tenant_id=$1 and reservation_id=$2",
        "delete from inventory_reservations where tenant_id=$1 and reservation_id=$2",
        "update inventory_location_reservation_events set remaining_micros=0 where tenant_id=$1 and reservation_id=$2",
        "delete from inventory_location_reservation_events where tenant_id=$1 and reservation_id=$2",
    ] {
        let error = sqlx::query(statement)
            .bind(f.tenant.0)
            .bind(reservation)
            .execute(&pool)
            .await
            .expect_err("source facts require canonical immutable events");
        eprintln!(
            "QA location reservation guard tenant={} statement={statement} error={error}",
            f.tenant
        );
        assert_eq!(saved(&independent, &f).await?, before);
    }
    Ok(())
}
