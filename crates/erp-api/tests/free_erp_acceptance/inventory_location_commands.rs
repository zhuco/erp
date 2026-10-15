//! Original structured inbound must bind its payload and canonical stock effect.
use super::{
    Result,
    account_login::{AccountLogin, STOCK_PERMISSIONS, create_user, paid_peer},
    isolated_pool,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_types::TenantId;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
use tower::ServiceExt;
use uuid::Uuid;

pub(super) struct Fixture {
    pub(super) tenant: TenantId,
    pub(super) actor: Uuid,
    pub(super) warehouse: Uuid,
    pub(super) sku: Uuid,
    pub(super) location: Uuid,
    pub(super) login: AccountLogin,
}

pub(super) async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let tenant = TenantId(Uuid::new_v4());
    let actor = Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-INVENTORY-LOCATION-COMMANDS')")
        .bind(tenant.0)
        .execute(pool)
        .await?;
    let login = create_user(
        pool,
        tenant,
        actor,
        format!("{actor}@location-command.qa.invalid"),
        STOCK_PERMISSIONS,
    )
    .await?;
    let f = Fixture {
        tenant,
        actor,
        warehouse: Uuid::new_v4(),
        sku: Uuid::new_v4(),
        location: Uuid::new_v4(),
        login,
    };
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Location command QA',$3)")
        .bind(f.tenant.0).bind(f.warehouse).bind(f.warehouse.to_string()).execute(pool).await?;
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-LOCATION-COMMAND',(select product_id from qa_parent))")
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(pool)
        .await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.tenant.0)
    .bind(f.actor)
    .bind(f.warehouse)
    .execute(pool)
    .await?;
    sqlx::query("insert into warehouse_locations(tenant_id,warehouse_id,location_id,code) values($1,$2,$3,'QA-LOC')")
        .bind(f.tenant.0).bind(f.warehouse).bind(f.location).execute(pool).await?;
    eprintln!(
        "QA location command fixture tenant={} actor={} warehouse={} sku={} location={}",
        f.tenant, f.actor, f.warehouse, f.sku, f.location
    );
    Ok(f)
}

pub(super) fn movement(f: &Fixture, id: &str, quantity: i64) -> Value {
    json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"request_id":id,"quantity":quantity,
        "reason":"QA_LOCATION","reference_id":"QA-LOCATION-REF","location_id":f.location,
        "batch_code":"QA-BATCH","quality_status":"GOOD"})
}

pub(super) async fn api(
    f: &Fixture,
    action: &str,
    body: Value,
    actor: Option<&AccountLogin>,
    url: Option<String>,
) -> Result<(StatusCode, Value)> {
    api_headers(f, action, body, actor, url, Vec::new()).await
}

pub(super) async fn api_headers(
    f: &Fixture,
    action: &str,
    body: Value,
    actor: Option<&AccountLogin>,
    url: Option<String>,
    headers: Vec<String>,
) -> Result<(StatusCode, Value)> {
    let caller = actor.unwrap_or(&f.login);
    let path = format!("/v1/tenants/{}/inventory/{action}", f.tenant);
    let url = url.unwrap_or(std::env::var("ERP_GRID_TEST_DATABASE_URL")?);
    let mut request = Request::builder()
        .method(if action.starts_with("commands/") {
            "GET"
        } else {
            "POST"
        })
        .uri(&path)
        .header("authorization", format!("Bearer {}", caller.token))
        .header("content-type", "application/json");
    for header in &headers {
        request = request.header("Idempotency-Key", header);
    }
    let response = caller
        .app(&url)?
        .oneshot(request.body(Body::from(body.to_string()))?)
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let response: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA location command HTTP tenant={} actor={} action={action} request={body} headers={headers:?} status={status} body={response}",
        f.tenant, caller.actor
    );
    Ok((status, response))
}

pub(super) async fn facts(pool: &PgPool, f: &Fixture) -> Result<Value> {
    Ok(sqlx::query_scalar("select jsonb_build_object(
        'balances',coalesce((select jsonb_agg(to_jsonb(b) order by warehouse_id,sku_id) from inventory_balances b where tenant_id=$1),'[]'::jsonb),
        'ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from inventory_ledgers l where tenant_id=$1),'[]'::jsonb),
        'shared_ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from shared_stock_ledgers l where tenant_id=$1),'[]'::jsonb),
        'receipts',coalesce((select jsonb_agg(to_jsonb(c) order by request_id) from command_receipts c where tenant_id=$1),'[]'::jsonb),
        'operations',coalesce((select jsonb_agg(to_jsonb(o) order by log_id) from operation_logs o where tenant_id=$1),'[]'::jsonb),
        'audits',coalesce((select jsonb_agg(to_jsonb(a) order by audit_id) from audit_logs a where tenant_id=$1),'[]'::jsonb),
        'outbox',coalesce((select jsonb_agg(to_jsonb(o) order by outbox_id) from outbox_events o where tenant_id=$1),'[]'::jsonb),
        'activations',coalesce((select jsonb_agg(to_jsonb(a) order by warehouse_id,sku_id) from inventory_quantity_activations a where tenant_id=$1),'[]'::jsonb))")
        .bind(f.tenant.0).fetch_one(pool).await?)
}

#[tokio::test]
async fn inventory_location_command_changed_payload_is_not_replay() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let (status, body) = api(&f, "inbounds", movement(&f, &id, 1), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let original = facts(&independent, &f).await?;
    let (status, body) = api(&f, "inbounds", movement(&f, &id, 2), None, None).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "changed original quantity cannot replay: {body}"
    );
    assert_eq!(facts(&independent, &f).await?, original);
    Ok(())
}

#[tokio::test]
async fn inventory_location_command_posts_primary_stock_and_required_logs() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let (status, body) = api(&f, "inbounds", movement(&f, &id, 1), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let saved = facts(&independent, &f).await?;
    assert_eq!(
        saved["balances"].as_array().map(Vec::len),
        Some(1),
        "structured stock must update the primary balance"
    );
    assert_eq!(saved["balances"][0]["on_hand_micros"], 1_000_000);
    assert_eq!(saved["ledgers"].as_array().map(Vec::len), Some(1));
    assert_eq!(saved["shared_ledgers"].as_array().map(Vec::len), Some(1));
    assert_eq!(saved["operations"].as_array().map(Vec::len), Some(1));
    assert_eq!(saved["audits"].as_array().map(Vec::len), Some(1));
    assert_eq!(saved["receipts"][0]["actor_user_id"], f.actor.to_string());
    Ok(())
}

#[tokio::test]
async fn inventory_location_command_binds_all_dimensions_actor_and_route() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let original = movement(&f, &id, 1);
    let (status, body) = api(&f, "inbounds", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let second = Uuid::new_v4();
    sqlx::query(
        "insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Second QA',$3)",
    )
    .bind(f.tenant.0)
    .bind(second)
    .bind(second.to_string())
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.tenant.0)
    .bind(f.actor)
    .bind(second)
    .execute(&pool)
    .await?;
    let actor = paid_peer(&pool, f.tenant, f.warehouse, STOCK_PERMISSIONS).await?;
    let before = facts(&independent, &f).await?;
    for (field, value) in [
        ("quantity", json!(2)),
        ("batch_code", json!("changed")),
        ("quality_status", json!("HOLD")),
        ("location_id", json!(Uuid::new_v4())),
        ("warehouse_id", json!(second)),
        ("sku_id", json!(Uuid::new_v4())),
        ("reason", json!("changed")),
        ("reference_id", json!("changed")),
    ] {
        let mut changed = original.clone();
        changed[field] = value;
        let (status, body) = api(&f, "inbounds", changed, None, None).await?;
        assert_eq!(status, StatusCode::CONFLICT, "{field}: {body}");
        assert_eq!(body["code"], "inventory_command_identity_conflict");
    }
    let (status, body) = api(&f, "inbounds", original.clone(), Some(&actor), None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (status, body) = api(
        &f,
        &format!("commands/{id}"),
        Value::Null,
        Some(&actor),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let mut basic = original;
    for field in ["location_id", "batch_code", "quality_status"] {
        basic
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("fixture object"))?
            .remove(field);
    }
    let (status, body) = api(&f, "inbounds", basic, None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_location_command_replays_fractional_original_after_later_stock() -> Result<()> {
    use erp_domain::quantity::parse_qty;
    use erp_repo::{
        inventory::InventoryService,
        inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
    };
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
            },
            on_hand: parse_qty("4.9")?,
            reserved: parse_qty("0.9")?,
            frozen: parse_qty("0.9")?,
            in_transit: parse_qty("2.3")?,
            reason_type: "QA_LOCATION_OPENING".into(),
            ref_type: Some("QA_LOCATION_OPENING".into()),
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;
    let id = Uuid::new_v4().to_string();
    let original = movement(&f, &id, 1);
    let (status, first) = api(&f, "inbounds", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["record"]["quantities"]["on_hand"], "5.9");
    assert_eq!(first["location_record"]["on_hand"], "1");
    let later_id = Uuid::new_v4().to_string();
    let (status, later) = api(&f, "inbounds", movement(&f, &later_id, 2), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{later}");
    assert_eq!(later["record"]["quantities"]["on_hand"], "7.9");
    assert_eq!(later["location_record"]["on_hand"], "3");
    let before = facts(&independent, &f).await?;
    for (action, body) in [
        ("inbounds".to_owned(), original),
        (format!("commands/{id}"), Value::Null),
    ] {
        let (status, replay) = api(&f, &action, body, None, None).await?;
        assert_eq!(status, StatusCode::OK, "{replay}");
        let mut expected = first.clone();
        expected["replay"] = json!(true);
        assert_eq!(
            replay, expected,
            "both original snapshots must survive later stock"
        );
    }
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_location_command_non_good_stock_is_frozen_and_unissuable() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for quality in ["GOOD", "PENDING", "FAILED", "HOLD"] {
        let f = fixture(&pool).await?;
        let id = Uuid::new_v4().to_string();
        let mut original = movement(&f, &id, 2);
        original["quality_status"] = json!(quality);
        let (status, body) = api(&f, "inbounds", original, None, None).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        let before = facts(&independent, &f).await?;
        assert_eq!(before["balances"][0]["on_hand_micros"], 2_000_000);
        assert_eq!(
            before["balances"][0]["frozen_micros"],
            if quality == "GOOD" { 0 } else { 2_000_000 }
        );
        if quality != "GOOD" {
            let outbound = json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"quantity":1,"request_id":Uuid::new_v4()});
            let (status, body) = api(&f, "outbounds", outbound, None, None).await?;
            assert_eq!(status, StatusCode::CONFLICT, "{body}");
            assert_eq!(facts(&independent, &f).await?, before);
        }
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_command_twenty_mixed_identities_have_one_effect() -> Result<()> {
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
            api_headers(&f, "inbounds", body, None, None, headers).await
        });
    }
    let mut first = 0;
    let mut original = None;
    while let Some(result) = tasks.join_next().await {
        let (status, body) = result??;
        assert_eq!(status, StatusCode::OK, "{body}");
        if body["replay"] == false {
            first += 1;
        }
        let mut saved = body;
        saved["replay"] = json!(true);
        if let Some(original) = &original {
            assert_eq!(&saved, original);
        } else {
            original = Some(saved);
        }
    }
    assert_eq!(first, 1);
    let saved = facts(&independent, &f).await?;
    assert_eq!(saved["balances"][0]["on_hand_micros"], 1_000_000);
    for field in [
        "ledgers",
        "shared_ledgers",
        "receipts",
        "operations",
        "audits",
    ] {
        assert_eq!(saved[field].as_array().map(Vec::len), Some(1), "{field}");
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_command_log_failure_rolls_back_every_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for quality in ["GOOD", "PENDING", "HOLD"] {
        for table in ["operation_logs", "audit_logs"] {
            let f = fixture(&pool).await?;
            // Synthetic mapping exercises the actual Outbox producer only; no external platform is called.
            sqlx::query(
                "with qa_shop as (insert into shops(tenant_id,shop_id,platform,platform_shop_id,shop_name) values($1,gen_random_uuid(),'qa',gen_random_uuid()::text,'QA mapping shop') returning shop_id) insert into channel_sku_mappings(tenant_id,sku_id,status,shop_id,platform,platform_sku_id) values($1,$2,'ACTIVE',(select shop_id from qa_shop),'qa',$2::text)",
            )
            .bind(f.tenant.0)
            .bind(f.sku)
            .execute(&pool)
            .await?;
            let id = Uuid::new_v4().to_string();
            let mut original = movement(&f, &id, 1);
            original["quality_status"] = json!(quality);
            let schema = format!("qa_location_command_{}", Uuid::new_v4().simple());
            sqlx::query(&format!("create schema {schema}"))
                .execute(&pool)
                .await?;
            sqlx::query(&format!("create view {schema}.{table} as select * from public.{table} where false with local check option"))
                .execute(&pool).await?;
            let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
            let separator = if url.contains('?') { '&' } else { '?' };
            let url = format!(
                "{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog"
            );
            assert!(super::approved_database_url(&url)?);
            eprintln!(
                "QA location command failure tenant={} schema={schema} table={table} quality={quality}",
                f.tenant
            );
            let before = facts(&independent, &f).await?;
            let (status, body) = api(&f, "inbounds", original.clone(), None, Some(url)).await?;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
            let message = body["message"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing database failure message"))?;
            assert!(
                message.contains("check option") && message.contains(table),
                "expected actual {table} view failure, got {message}"
            );
            assert_eq!(
                facts(&independent, &f).await?,
                before,
                "claim, activation, stock, projection, Outbox and logs must roll back"
            );
            let (status, body) = api(&f, "inbounds", original, None, None).await?;
            assert_eq!(
                status,
                StatusCode::OK,
                "same original ID after proved rollback: {body}"
            );
            let saved = facts(&independent, &f).await?;
            for field in [
                "ledgers",
                "shared_ledgers",
                "receipts",
                "operations",
                "audits",
            ] {
                assert_eq!(saved[field].as_array().map(Vec::len), Some(1), "{field}");
            }
            assert_eq!(
                saved["outbox"].as_array().map(Vec::len),
                Some(if quality == "GOOD" { 1 } else { 0 })
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_command_scope_validation_and_revocation() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let original = movement(&f, &id, 1);
    let before = facts(&independent, &f).await?;
    for (field, value) in [
        ("location_id", json!(Uuid::new_v4())),
        ("batch_code", json!("")),
        ("batch_code", json!("x".repeat(129))),
        ("batch_code", json!("a\nb")),
        ("quality_status", json!("UNKNOWN")),
        ("quantity", json!(i64::MAX)),
    ] {
        let mut body = original.clone();
        body[field] = value;
        let (status, body) = api(&f, "inbounds", body, None, None).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{field}: {body}");
        assert_eq!(facts(&independent, &f).await?, before);
    }
    sqlx::query(
        "update warehouse_locations set is_disabled=true where tenant_id=$1 and location_id=$2",
    )
    .bind(f.tenant.0)
    .bind(f.location)
    .execute(&pool)
    .await?;
    let (status, body) = api(&f, "inbounds", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(facts(&independent, &f).await?, before);
    sqlx::query(
        "update warehouse_locations set is_disabled=false where tenant_id=$1 and location_id=$2",
    )
    .bind(f.tenant.0)
    .bind(f.location)
    .execute(&pool)
    .await?;
    let (status, body) = api(&f, "inbounds", original.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.actor)
        .execute(&pool)
        .await?;
    let before = facts(&independent, &f).await?;
    for (action, body) in [
        ("inbounds".to_owned(), original),
        (format!("commands/{id}"), Value::Null),
    ] {
        let (status, body) = api(&f, &action, body, None, None).await?;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(facts(&independent, &f).await?, before);
    }
    Ok(())
}

#[tokio::test]
async fn inventory_location_command_linked_projection_is_immutable() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let (status, body) = api(&f, "inbounds", movement(&f, &id, 1), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = facts(&independent, &f).await?;
    for sql in [
        "update shared_stock_ledgers set delta_micros=delta_micros+1 where tenant_id=$1",
        "delete from shared_stock_ledgers where tenant_id=$1",
    ] {
        let error = sqlx::query(sql)
            .bind(f.tenant.0)
            .execute(&pool)
            .await
            .expect_err("linked source cannot change");
        assert!(
            error
                .to_string()
                .contains("linked shared inventory projection is immutable"),
            "{error}"
        );
        assert_eq!(facts(&independent, &f).await?, before);
    }
    Ok(())
}
