//! Owned precise batches, original commands and real per-batch ledger provenance.
use super::{
    Result,
    inventory_commands::{Fixture, fixture},
    isolated_pool,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
use tower::ServiceExt;
use uuid::Uuid;

struct Batches {
    f: Fixture,
    a: Uuid,
    b: Uuid,
}

async fn setup(pool: &PgPool, case: &str) -> Result<Batches> {
    let f = fixture(pool).await?;
    sqlx::query("update tenants set name='QA-ERP-BATCH-WRITES' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    sqlx::query("update skus set batch_enabled=true where tenant_id=$1 and sku_id=$2")
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(pool)
        .await?;
    let a = metadata(pool, &f, "A", f.sku).await?;
    let b = metadata(pool, &f, "B", f.sku).await?;
    for (id, hand) in [(a, 1_900_000_i64), (b, 3_000_000_i64)] {
        sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros) values($1,$2,$3,$4,$5,0)")
            .bind(f.tenant.0).bind(f.warehouse).bind(f.sku).bind(id).bind(hand)
            .execute(pool).await?;
    }
    eprintln!(
        "QA batch_write_fixture tenant={} case={case} actor={} warehouse={} sku={} a={a} b={b}",
        f.tenant, f.actor, f.warehouse, f.sku
    );
    Ok(Batches { f, a, b })
}

async fn metadata(pool: &PgPool, f: &Fixture, name: &str, sku: Uuid) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("insert into sku_batches(tenant_id,sku_id,batch_id,batch_no,expire_at) values($1,$2,$3,$4,$5)")
        .bind(f.tenant.0).bind(sku).bind(id).bind(name)
        .bind(Utc::now() + Duration::days(7)).execute(pool).await?;
    Ok(id)
}

fn movement(f: &Fixture, action: &str, id: Option<&str>, batch: Uuid, quantity: i64) -> Value {
    let mut value = json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"batch_id":batch,
        "reason":"QA_BATCH_WRITE","reference_id":"QA-BATCH-REF"});
    if let Some(id) = id {
        value["request_id"] = json!(id);
    }
    value[if action == "adjustments" {
        "quantity_delta"
    } else {
        "quantity"
    }] = json!(quantity);
    value
}

async fn http(f: &Fixture, path: &str, method: &str, body: Value) -> Result<(StatusCode, Value)> {
    let database = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let joiner = if database.contains('?') { '&' } else { '?' };
    let database = format!("{database}{joiner}application_name=qa_batch_{}", f.tenant);
    let response = f
        .login
        .app(&database)?
        .oneshot(
            Request::builder()
                .method(method)
                .uri(format!("/v1/tenants/{}/{path}", f.tenant))
                .header("authorization", format!("Bearer {}", f.login.token))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let response = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA batch_write_http tenant={} path={path} method={method} input={body} status={} response={response}",
        f.tenant,
        status.as_u16()
    );
    Ok((status, response))
}

async fn facts(pool: &PgPool, f: &Fixture) -> Result<Value> {
    let mut result = serde_json::Map::new();
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "inventory_batch_balances",
        "sku_batches",
        "inventory_reservations",
        "inventory_quantity_activations",
        "inventory_quantity_source_maps",
        "command_receipts",
        "operation_logs",
        "audit_logs",
        "outbox_events",
        "mfg_material_moves",
        "mfg_qty_ledgers",
        "mfg_work_orders",
        "inventory_batch_movements",
    ] {
        let exists: bool = sqlx::query_scalar("select to_regclass($1) is not null")
            .bind(format!("public.{table}"))
            .fetch_one(pool)
            .await?;
        let value = if exists {
            sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from public.{table} x where tenant_id=$1"))
                .bind(f.tenant.0).fetch_one(pool).await?
        } else {
            Value::Null
        };
        result.insert(table.into(), value);
    }
    Ok(Value::Object(result))
}

async fn quantity(pool: &PgPool, f: &Fixture, batch: Uuid) -> Result<Option<i64>> {
    Ok(sqlx::query_scalar("select on_hand_micros from inventory_batch_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 and batch_id=$4")
        .bind(f.tenant.0).bind(f.warehouse).bind(f.sku).bind(batch).fetch_one(pool).await?)
}

async fn provenance(pool: &PgPool, f: &Fixture, expected: i64) -> Result<()> {
    let exists: bool =
        sqlx::query_scalar("select to_regclass('public.inventory_batch_movements') is not null")
            .fetch_one(pool)
            .await?;
    assert!(
        exists,
        "real per-batch movement table must exist; absence cannot count as no effects"
    );
    let (count,broken):(i64,i64) = sqlx::query_as("select count(*),count(*) filter(where l.ledger_id is null or l.tenant_id<>b.tenant_id or l.warehouse_id<>b.warehouse_id or l.sku_id<>b.sku_id or b.after_on_hand_micros::numeric-b.before_on_hand_micros<>b.delta_on_hand_micros) from inventory_batch_movements b left join inventory_ledgers l on l.ledger_id=b.inventory_ledger_id where b.tenant_id=$1")
        .bind(f.tenant.0).fetch_one(pool).await?;
    eprintln!(
        "QA batch_write_provenance tenant={} effects={count} broken={broken}",
        f.tenant
    );
    assert_eq!(count, expected);
    assert_eq!(
        broken, 0,
        "every child references actual owned SKU ledger and exact quantity difference"
    );
    Ok(())
}

async fn legacy_success(action: &str, quantity_delta: i64, expected: i64) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, &format!("legacy_{action}")).await?;
    let (status, body) = http(
        &f.f,
        &format!("inventory/{action}"),
        "POST",
        movement(&f.f, action, None, f.a, quantity_delta),
    )
    .await?;
    let exact = quantity(&independent, &f.f, f.a).await?;
    eprintln!(
        "QA batch_write_quantity tenant={} batch={} exact={exact:?}",
        f.f.tenant, f.a
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(exact, Some(expected));
    assert_eq!(quantity(&independent, &f.f, f.b).await?, Some(3_000_000));
    provenance(&independent, &f.f, 1).await
}

#[tokio::test]
async fn batch_write_legacy_inbound_preserves_fractional_balance() -> Result<()> {
    legacy_success("inbounds", 1, 2_900_000).await
}

#[tokio::test]
async fn batch_write_legacy_outbound_preserves_fractional_balance() -> Result<()> {
    legacy_success("outbounds", 1, 900_000).await
}

#[tokio::test]
async fn batch_write_legacy_adjustment_preserves_fractional_balance() -> Result<()> {
    legacy_success("adjustments", -1, 900_000).await
}

async fn rejection(
    pool: &PgPool,
    f: &Batches,
    action: &str,
    body: Value,
    expected: StatusCode,
) -> Result<()> {
    let before = facts(pool, &f.f).await?;
    let (status, response) = http(&f.f, action, "POST", body).await?;
    let after = facts(pool, &f.f).await?;
    eprintln!(
        "QA batch_write_rejection tenant={} status={} unchanged={}",
        f.f.tenant,
        status.as_u16(),
        before == after
    );
    assert_eq!(status, expected, "{response}");
    assert_eq!(
        before, after,
        "no partial physical/source/log/receipt effect on rejection"
    );
    Ok(())
}

#[tokio::test]
async fn batch_write_foreign_enterprise_batch_is_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "foreign_batch").await?;
    let foreign = setup(&pool, "foreign_owner").await?;
    let before_foreign = facts(&independent, &foreign.f).await?;
    let result = rejection(
        &independent,
        &f,
        "inventory/inbounds",
        movement(&f.f, "inbounds", None, foreign.a, 1),
        StatusCode::NOT_FOUND,
    )
    .await;
    assert_eq!(facts(&independent, &foreign.f).await?, before_foreign);
    result
}

#[tokio::test]
async fn batch_write_wrong_sku_batch_is_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "wrong_sku").await?;
    let sku = Uuid::new_v4();
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-OTHER-BATCH-SKU',(select product_id from qa_parent))")
        .bind(f.f.tenant.0)
        .bind(sku)
        .execute(&pool)
        .await?;
    let wrong = metadata(&pool, &f.f, "OTHER", sku).await?;
    rejection(
        &independent,
        &f,
        "inventory/inbounds",
        movement(&f.f, "inbounds", None, wrong, 1),
        StatusCode::NOT_FOUND,
    )
    .await
}

#[tokio::test]
async fn batch_write_unknown_batch_precision_is_not_guessed() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "unknown_precision").await?;
    // Replace only this fixture's row with a genuinely unactivated old integer.
    sqlx::query("delete from inventory_batch_balances where tenant_id=$1 and batch_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.a)
        .execute(&pool)
        .await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand,reserved) values($1,$2,$3,$4,1,0)")
        .bind(f.f.tenant.0).bind(f.f.warehouse).bind(f.f.sku).bind(f.a).execute(&pool).await?;
    rejection(
        &independent,
        &f,
        "inventory/inbounds",
        movement(&f.f, "inbounds", None, f.a, 1),
        StatusCode::CONFLICT,
    )
    .await
}

#[tokio::test]
async fn batch_write_outbound_respects_batch_reserved_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "batch_reserved").await?;
    sqlx::query("update inventory_batch_balances set reserved_micros=900000 where tenant_id=$1 and batch_id=$2")
        .bind(f.f.tenant.0).bind(f.a).execute(&pool).await?;
    rejection(
        &independent,
        &f,
        "inventory/outbounds",
        movement(&f.f, "outbounds", None, f.a, 2),
        StatusCode::CONFLICT,
    )
    .await
}

#[tokio::test]
async fn batch_write_expired_batch_cannot_leave_ordinary_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "expired_outbound").await?;
    sqlx::query("update sku_batches set expire_at=now()-interval '1 day' where tenant_id=$1 and batch_id=$2")
        .bind(f.f.tenant.0).bind(f.a).execute(&pool).await?;
    rejection(
        &independent,
        &f,
        "inventory/outbounds",
        movement(&f.f, "outbounds", None, f.a, 1),
        StatusCode::CONFLICT,
    )
    .await
}

#[tokio::test]
async fn batch_write_new_empty_batch_uses_explicit_exact_zero() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "new_empty_batch").await?;
    let batch = metadata(&pool, &f.f, "NEW", f.f.sku).await?;
    let (status, body) = http(
        &f.f,
        "inventory/inbounds",
        "POST",
        movement(&f.f, "inbounds", None, batch, 1),
    )
    .await?;
    let exact = quantity(&independent, &f.f, batch).await?;
    eprintln!(
        "QA batch_write_new_quantity tenant={} batch={batch} exact={exact:?}",
        f.f.tenant
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(exact, Some(1_000_000));
    provenance(&independent, &f.f, 1).await
}

#[tokio::test]
async fn batch_write_original_uuid_command_has_immutable_result() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "original_result").await?;
    let id = Uuid::new_v4().to_string();
    let input = movement(&f.f, "inbounds", Some(&id), f.a, 1);
    let (status, first) = http(&f.f, "inventory/inbounds", "POST", input.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let original: Value = sqlx::query_scalar(
        "select original_result from command_receipts where tenant_id=$1 and request_id=$2",
    )
    .bind(f.f.tenant.0)
    .bind(&id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(original["input"]["batch_id"], json!(f.a));
    assert_eq!(original["batch_effects"].as_array().map(Vec::len), Some(1));
    sqlx::query("update sku_batches set batch_no='A-RENAMED',updated_at=now() where tenant_id=$1 and batch_id=$2")
        .bind(f.f.tenant.0).bind(f.a).execute(&pool).await?;
    let later_id = Uuid::new_v4().to_string();
    let (status, later) = http(
        &f.f,
        "inventory/inbounds",
        "POST",
        movement(&f.f, "inbounds", Some(&later_id), f.a, 1),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{later}");
    let before = facts(&independent, &f.f).await?;
    let (status, retry) = http(&f.f, "inventory/inbounds", "POST", input.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{retry}");
    assert_eq!(first["record"], retry["record"]);
    let (status, lookup) =
        http(&f.f, &format!("inventory/commands/{id}"), "GET", json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{lookup}");
    assert_eq!(lookup["record"], first["record"]);
    assert_eq!(lookup["batch_effects"], original["batch_effects"]);
    let mut different = input;
    different["batch_id"] = json!(f.b);
    let (status, error) = http(&f.f, "inventory/inbounds", "POST", different).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(facts(&independent, &f.f).await?, before);
    provenance(&independent, &f.f, 2).await
}

#[tokio::test]
async fn batch_write_twenty_original_submissions_have_one_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(setup(&pool, "concurrent_original").await?);
    let id = Uuid::new_v4().to_string();
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let f = Arc::clone(&f);
        let id = id.clone();
        let barrier = Arc::clone(&barrier);
        tasks.spawn(async move {
            barrier.wait().await;
            http(
                &f.f,
                "inventory/inbounds",
                "POST",
                movement(&f.f, "inbounds", Some(&id), f.a, 1),
            )
            .await
        });
    }
    let mut responses = Vec::new();
    while let Some(result) = tasks.join_next().await {
        responses.push(result??);
    }
    eprintln!(
        "QA batch_write_concurrency tenant={} results={} statuses={:?}",
        f.f.tenant,
        responses.len(),
        responses
            .iter()
            .map(|(s, _)| s.as_u16())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        responses.len(),
        20,
        "collect all actual outcomes before assertions"
    );
    for (status, body) in responses {
        assert_eq!(status, StatusCode::OK, "{body}");
    }
    assert_eq!(quantity(&independent, &f.f, f.a).await?, Some(2_900_000));
    provenance(&independent, &f.f, 1).await
}

async fn work_order(pool: &PgPool, f: &Fixture) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-BATCH-WO','QA-COMMAND','RELEASED',1000000)")
        .bind(f.tenant.0).bind(id).execute(pool).await?;
    Ok(id)
}

fn material(f: &Batches, kind: &str, qty: &str, original: Option<Uuid>) -> Value {
    json!({"request_id":Uuid::new_v4(),"kind":kind,"qty":qty,"reason":"QA_BATCH_MATERIAL",
        "warehouse_id":f.f.warehouse,"sku_id":f.f.sku,"batch_id":f.a,"original_issue_id":original})
}

#[tokio::test]
async fn batch_write_manufacturing_does_not_activate_unknown_history() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "material_unknown").await?;
    let order = work_order(&pool, &f.f).await?;
    sqlx::query("delete from inventory_batch_balances where tenant_id=$1 and batch_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.a)
        .execute(&pool)
        .await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand,reserved) values($1,$2,$3,$4,1,0)")
        .bind(f.f.tenant.0).bind(f.f.warehouse).bind(f.f.sku).bind(f.a).execute(&pool).await?;
    rejection(
        &independent,
        &f,
        &format!("manufacturing/work-orders/{order}/materials"),
        material(&f, "ISSUE", "0.25", None),
        StatusCode::CONFLICT,
    )
    .await
}

#[tokio::test]
async fn batch_write_manufacturing_return_and_consume_preserve_original_batch() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "material_return_consume").await?;
    let order = work_order(&pool, &f.f).await?;
    let path = format!("manufacturing/work-orders/{order}/materials");
    let (status, body) = http(&f.f, &path, "POST", material(&f, "ISSUE", "0.25", None)).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let original: Uuid = sqlx::query_scalar("select move_id from mfg_material_moves where tenant_id=$1 and work_order_id=$2 and kind='ISSUE'")
        .bind(f.f.tenant.0).bind(order).fetch_one(&independent).await?;
    let (status, body) = http(
        &f.f,
        &path,
        "POST",
        material(&f, "RETURN", "0.125", Some(original)),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before_consume = quantity(&independent, &f.f, f.a).await?;
    let (status, body) = http(
        &f.f,
        &path,
        "POST",
        material(&f, "CONSUME", "0.0625", Some(original)),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(before_consume, Some(1_775_000));
    assert_eq!(
        quantity(&independent, &f.f, f.a).await?,
        before_consume,
        "consumption uses issued custody rather than a second warehouse debit"
    );
    provenance(&independent, &f.f, 2).await
}

#[tokio::test]
async fn batch_write_original_outbound_and_adjustment_keep_fractional_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for (action, delta) in [("outbounds", 1), ("adjustments", -1)] {
        let f = setup(&pool, &format!("original_{action}")).await?;
        let id = Uuid::new_v4().to_string();
        let input = movement(&f.f, action, Some(&id), f.a, delta);
        let (status, first) =
            http(&f.f, &format!("inventory/{action}"), "POST", input.clone()).await?;
        assert_eq!(status, StatusCode::OK, "{first}");
        assert_eq!(quantity(&independent, &f.f, f.a).await?, Some(900_000));
        let before = facts(&independent, &f.f).await?;
        let (status, replay) = http(&f.f, &format!("inventory/{action}"), "POST", input).await?;
        assert_eq!(status, StatusCode::OK, "{replay}");
        assert_eq!(replay["batch_effects"], first["batch_effects"]);
        assert_eq!(facts(&independent, &f.f).await?, before);
        provenance(&independent, &f.f, 1).await?;
    }
    Ok(())
}

#[tokio::test]
async fn batch_write_nil_and_unmapped_dimensions_have_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "bad_dimension").await?;
    for id in [None, Some(Uuid::new_v4().to_string())] {
        rejection(
            &independent,
            &f,
            "inventory/inbounds",
            movement(&f.f, "inbounds", id.as_deref(), Uuid::nil(), 1),
            StatusCode::BAD_REQUEST,
        )
        .await?;
    }
    let mut mixed = movement(&f.f, "inbounds", Some(&Uuid::new_v4().to_string()), f.a, 1);
    mixed["location_id"] = json!(Uuid::new_v4());
    mixed["batch_code"] = json!("A");
    mixed["quality_status"] = json!("GOOD");
    rejection(
        &independent,
        &f,
        "inventory/inbounds",
        mixed.clone(),
        StatusCode::BAD_REQUEST,
    )
    .await?;
    mixed
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("object required"))?
        .remove("batch_id");
    rejection(
        &independent,
        &f,
        "inventory/inbounds",
        mixed,
        StatusCode::BAD_REQUEST,
    )
    .await
}

#[tokio::test]
async fn batch_write_inconsistent_total_is_rejected_without_repair_guess() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "inconsistent_totals").await?;
    sqlx::query("update inventory_batch_balances set on_hand_micros=1800000 where tenant_id=$1 and batch_id=$2")
        .bind(f.f.tenant.0).bind(f.a).execute(&pool).await?;
    rejection(
        &independent,
        &f,
        "inventory/inbounds",
        movement(&f.f, "inbounds", Some(&Uuid::new_v4().to_string()), f.a, 1),
        StatusCode::CONFLICT,
    )
    .await
}

#[tokio::test]
async fn batch_write_complete_1000_batch_scope_and_1001_rejection() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "capacity_limit").await?;
    let inserted = sqlx::query("with batches as (insert into sku_batches(tenant_id,sku_id,batch_no)
        select $1,$2,'QA-LIMIT-'||n::text from generate_series(1,998) n returning batch_id)
        insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros)
        select $1,$3,$2,batch_id,0,0 from batches")
        .bind(f.f.tenant.0).bind(f.f.sku).bind(f.f.warehouse).execute(&pool).await?;
    assert_eq!(inserted.rows_affected(), 998);
    let (status, body) = http(
        &f.f,
        "inventory/inbounds",
        "POST",
        movement(&f.f, "inbounds", Some(&Uuid::new_v4().to_string()), f.a, 1),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let extra = metadata(&pool, &f.f, "QA-LIMIT-1001", f.f.sku).await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros) values($1,$2,$3,$4,0,0)")
        .bind(f.f.tenant.0).bind(f.f.warehouse).bind(f.f.sku).bind(extra).execute(&pool).await?;
    rejection(
        &independent,
        &f,
        "inventory/inbounds",
        movement(&f.f, "inbounds", Some(&Uuid::new_v4().to_string()), f.a, 1),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await?;
    provenance(&independent, &f.f, 1).await
}

#[tokio::test]
async fn batch_write_fefo_children_bind_one_actual_sku_ledger() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "fefo_split").await?;
    sqlx::query("update inventory_batch_balances set on_hand_micros=case when batch_id=$2 then 600000 else 4300000 end where tenant_id=$1")
        .bind(f.f.tenant.0).bind(f.a).execute(&pool).await?;
    sqlx::query("update sku_batches set expire_at=now()+interval '1 day' where tenant_id=$1 and batch_id=$2")
        .bind(f.f.tenant.0).bind(f.a).execute(&pool).await?;
    let mut input = movement(&f.f, "outbounds", Some(&Uuid::new_v4().to_string()), f.a, 1);
    input
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("object required"))?
        .remove("batch_id");
    let (status, body) = http(&f.f, "inventory/outbounds", "POST", input).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["batch_effects"].as_array().map(Vec::len), Some(2));
    assert_eq!(quantity(&independent, &f.f, f.a).await?, Some(0));
    assert_eq!(quantity(&independent, &f.f, f.b).await?, Some(3_900_000));
    let parents: i64 = sqlx::query_scalar("select count(distinct inventory_ledger_id) from inventory_batch_movements where tenant_id=$1")
        .bind(f.f.tenant.0).fetch_one(&independent).await?;
    assert_eq!(parents, 1);
    provenance(&independent, &f.f, 2).await
}

#[tokio::test]
async fn batch_write_large_exact_effect_uses_decimal_strings() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "exact_large").await?;
    let id = Uuid::new_v4().to_string();
    let input = movement(&f.f, "inbounds", Some(&id), f.a, 9_007_199_255);
    let (status, first) = http(&f.f, "inventory/inbounds", "POST", input.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(
        first["batch_effects"][0]["delta_on_hand_micros"],
        json!("9007199255000000")
    );
    assert_eq!(
        first["batch_effects"][0]["after_on_hand_micros"],
        json!("9007199256900000")
    );
    assert_eq!(
        quantity(&independent, &f.f, f.a).await?,
        Some(9_007_199_256_900_000)
    );
    let before = facts(&independent, &f.f).await?;
    let (status, replay) = http(&f.f, "inventory/inbounds", "POST", input).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(first["batch_effects"], replay["batch_effects"]);
    assert_eq!(facts(&independent, &f.f).await?, before);
    provenance(&independent, &f.f, 1).await
}

#[tokio::test]
async fn batch_write_original_child_and_parent_source_are_immutable() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "immutable_source").await?;
    let (status, body) = http(
        &f.f,
        "inventory/inbounds",
        "POST",
        movement(&f.f, "inbounds", Some(&Uuid::new_v4().to_string()), f.a, 1),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = facts(&independent, &f.f).await?;
    let child: Uuid = sqlx::query_scalar(
        "select batch_movement_id from inventory_batch_movements where tenant_id=$1",
    )
    .bind(f.f.tenant.0)
    .fetch_one(&independent)
    .await?;
    let ledger: Uuid = sqlx::query_scalar(
        "select inventory_ledger_id from inventory_batch_movements where tenant_id=$1",
    )
    .bind(f.f.tenant.0)
    .fetch_one(&independent)
    .await?;
    for statement in [
        "update inventory_batch_movements set ref_id='FORGED' where batch_movement_id=$1",
        "delete from inventory_batch_movements where batch_movement_id=$1",
    ] {
        let result = sqlx::query(statement).bind(child).execute(&pool).await;
        assert!(result.is_err(), "original child mutation must fail");
    }
    assert!(
        sqlx::query("update inventory_ledgers set ref_id='FORGED' where ledger_id=$1")
            .bind(ledger)
            .execute(&pool)
            .await
            .is_err()
    );
    assert_eq!(facts(&independent, &f.f).await?, before);
    eprintln!(
        "QA batch_write_immutable tenant={} child={child} parent={ledger} mutations_rejected=3 unchanged=true",
        f.f.tenant
    );
    Ok(())
}

async fn mandatory_failure(table: &str, case: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, case).await?;
    let id = Uuid::new_v4().to_string();
    let input = movement(&f.f, "inbounds", Some(&id), f.a, 1);
    let before = facts(&independent, &f.f).await?;
    // Controlled fixture IDs form a unique scoped QA trigger, never a shared name.
    let name = format!("qa_batch_{}", f.f.tenant.0.simple());
    let function = format!(
        "create function {name}() returns trigger language plpgsql as $$ begin
        if new.tenant_id='{}'::uuid then raise exception 'QA_BATCH_REQUIRED_{table}'; end if;
        return new; end $$",
        f.f.tenant
    );
    sqlx::query(&function).execute(&pool).await?;
    let installed = sqlx::query(&format!(
        "create trigger {name} before insert on {table} for each row execute function {name}()"
    ))
    .execute(&pool)
    .await;
    if let Err(error) = installed {
        sqlx::query(&format!("drop function {name}()"))
            .execute(&pool)
            .await?;
        return Err(error.into());
    }
    let attempted = http(&f.f, "inventory/inbounds", "POST", input.clone()).await;
    sqlx::query(&format!("drop trigger {name} on {table}"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop function {name}()"))
        .execute(&pool)
        .await?;
    let (status, error) = attempted?;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{error}");
    assert!(
        error
            .to_string()
            .contains(&format!("QA_BATCH_REQUIRED_{table}")),
        "actual INSERT fault must be reached: {error}"
    );
    assert_eq!(facts(&independent, &f.f).await?, before);
    eprintln!(
        "QA batch_write_fault tenant={} table={table} actual_marker=true unchanged=true",
        f.f.tenant
    );
    let (status, retry) = http(&f.f, "inventory/inbounds", "POST", input.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{retry}");
    let (status, replay) = http(&f.f, "inventory/inbounds", "POST", input).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(retry["batch_effects"], replay["batch_effects"]);
    provenance(&independent, &f.f, 1).await
}

#[tokio::test]
async fn batch_write_required_child_failure_rolls_back_original_command() -> Result<()> {
    mandatory_failure("inventory_batch_movements", "child_failure").await
}

#[tokio::test]
async fn batch_write_required_audit_failure_rolls_back_child_and_command() -> Result<()> {
    mandatory_failure("audit_logs", "audit_failure").await
}

#[tokio::test]
async fn batch_write_expiry_is_read_after_actual_metadata_lock_wait() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(setup(&pool, "metadata_lock").await?);
    let mut before = facts(&independent, &f.f).await?;
    let mut metadata_tx = pool.begin().await?;
    sqlx::query("update sku_batches set expire_at=clock_timestamp()-interval '1 day' where tenant_id=$1 and batch_id=$2")
        .bind(f.f.tenant.0).bind(f.a).execute(&mut *metadata_tx).await?;
    let writer = Arc::clone(&f);
    let task = tokio::spawn(async move {
        http(
            &writer.f,
            "inventory/outbounds",
            "POST",
            movement(
                &writer.f,
                "outbounds",
                Some(&Uuid::new_v4().to_string()),
                writer.a,
                1,
            ),
        )
        .await
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut actual_wait = false;
    while std::time::Instant::now() < deadline {
        actual_wait = sqlx::query_scalar("select exists(select 1 from pg_stat_activity
            where datname=current_database() and usename=current_user and application_name=$1
             and wait_event_type='Lock' and position('select s.batch_id,s.expire_at,to_jsonb(s)' in query)>0)")
            .bind(format!("qa_batch_{}", f.f.tenant)).fetch_one(&independent).await?;
        if actual_wait || task.is_finished() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    metadata_tx.commit().await?;
    let (status, body) = task.await??;
    assert!(
        actual_wait,
        "actual owned HTTP metadata query must be observed waiting for the row lock"
    );
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let after = facts(&independent, &f.f).await?;
    // The deliberate metadata update is the only committed change.
    before["sku_batches"] = after["sku_batches"].clone();
    assert_eq!(before, after);
    eprintln!(
        "QA batch_write_metadata_lock tenant={} actual_wait=true status=409 physical_unchanged=true",
        f.f.tenant
    );
    Ok(())
}
