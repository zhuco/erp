//! Real scoped exact batch selection and stored material batch effects.
use super::{
    Result,
    inventory_commands::Fixture,
    isolated_pool,
    material_confirmation::{draft, line, owned, setup},
    material_documents::read,
    production_receipt_release::http,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{AppState, build_app};
use erp_domain::quantity::parse_qty;
use erp_repo::{
    db::Db,
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn batch(
    pool: &PgPool,
    f: &Fixture,
    name: &str,
    qty: Option<i64>,
    days: i64,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("insert into sku_batches(tenant_id,sku_id,batch_id,batch_no,expire_at) values($1,$2,$3,$4,$5)")
        .bind(f.tenant.0).bind(f.sku).bind(id).bind(name).bind(chrono::Utc::now()+chrono::Duration::days(days)).execute(pool).await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand,reserved,on_hand_micros,reserved_micros) values($1,$2,$3,$4,$5,0,$6,0)")
        .bind(f.tenant.0).bind(f.warehouse).bind(f.sku).bind(id).bind(qty.unwrap_or(7)/1000000).bind(qty).execute(pool).await?;
    Ok(id)
}
async fn start(pool: &PgPool) -> Result<(Fixture, Uuid, Uuid, Uuid)> {
    let (f, order) = setup(pool).await?;
    sqlx::query("update tenants set name='QA-MATERIAL-BATCH-OPTIONS' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    sqlx::query("update skus set batch_enabled=true where tenant_id=$1 and sku_id=$2")
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(pool)
        .await?;
    let a = batch(pool, &f, "RAW_%_A", Some(400000), 2).await?;
    let b = batch(pool, &f, "RAW_%_B", Some(600000), 1).await?;
    eprintln!(
        "QA material_batch_options tenant={} order={order} warehouse={} sku={} a={a} b={b}",
        f.tenant, f.warehouse, f.sku
    );
    Ok((f, order, a, b))
}
async fn get(
    f: &Fixture,
    order: Uuid,
    warehouse: Uuid,
    sku: Uuid,
    tail: &str,
) -> Result<(StatusCode, Value)> {
    let path = format!(
        "/v1/tenants/{}/manufacturing/work-orders/{order}/material-batch-options?warehouse_id={warehouse}&sku_id={sku}&{tail}",
        f.tenant
    );
    let response = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        f.login.jwt.clone(),
    ))
    .oneshot(
        Request::builder()
            .method("GET")
            .uri(&path)
            .header("authorization", format!("Bearer {}", f.login.token))
            .body(Body::empty())?,
    )
    .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA material_batch_get tenant={} path={path} status={} response={value}",
        f.tenant,
        status.as_u16()
    );
    Ok((status, value))
}
async fn all(pool: &PgPool, f: &Fixture) -> Result<Value> {
    let mut value = owned(pool, f).await?;
    for name in [
        "sku_batches",
        "inventory_batch_balances",
        "inventory_batch_movements",
    ] {
        value[name]=sqlx::query_scalar::<_,Value>(&format!("select coalesce(jsonb_agg(to_jsonb(t) order by to_jsonb(t)::text),'[]'::jsonb) from {name} t where tenant_id=$1")).bind(f.tenant.0).fetch_one(pool).await?;
    }
    Ok(value)
}
#[tokio::test]
async fn material_batch_options_complete_scope_literal_search_fefo_and_exact_available()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order, a, b) = start(&pool).await?;
    let expired = batch(&pool, &f, "EXPIRED", Some(0), -1).await?;
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
            },
            on_hand: parse_qty("0")?,
            reserved: parse_qty("0.1")?,
            frozen: parse_qty("0.1")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_BATCH_QUOTE_HOLDS".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: chrono::Utc::now(),
        })
        .await?;
    sqlx::query("update inventory_batch_balances set reserved_micros=100000 where tenant_id=$1 and batch_id=$2").bind(f.tenant.0).bind(b).execute(&pool).await?;
    let before = all(&pool, &f).await?;
    let (status, page) = get(
        &f,
        order,
        f.warehouse,
        f.sku,
        "query=RAW_%25_&offset=0&limit=1",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(page["total"], 2);
    assert_eq!(page["data"][0]["batch_id"], json!(b));
    assert_eq!(page["data"][0]["available_micros"], "500000");
    assert_eq!(page["warehouse_available_micros"], "800000");
    let (_, page) = get(
        &f,
        order,
        f.warehouse,
        f.sku,
        "query=RAW_%25_&offset=1&limit=1",
    )
    .await?;
    assert_eq!(page["total"], 2);
    assert_eq!(page["data"][0]["batch_id"], json!(a));
    assert_eq!(page["data"][0]["state"], "READY");
    let (_, page) = get(&f, order, f.warehouse, f.sku, "offset=0&limit=3").await?;
    assert_eq!(page["data"][0]["batch_id"], json!(expired));
    assert_eq!(page["data"][0]["state"], "EXPIRED");
    assert_eq!(all(&pool, &f).await?, before);
    Ok(())
}
#[tokio::test]
async fn material_batch_options_unknown_or_mismatched_quantities_never_guess_a_choice() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order, a, _) = start(&pool).await?;
    let legacy = Uuid::new_v4();
    sqlx::query("insert into sku_batches(tenant_id,sku_id,batch_id,batch_no) values($1,$2,$3,'OLD-UNKNOWN')").bind(f.tenant.0).bind(f.sku).bind(legacy).execute(&pool).await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand,reserved,on_hand_micros,reserved_micros) values($1,$2,$3,$4,7,0,null,null)").bind(f.tenant.0).bind(f.warehouse).bind(f.sku).bind(legacy).execute(&pool).await?;
    let before = all(&pool, &f).await?;
    let (status, page) = get(
        &f,
        order,
        f.warehouse,
        f.sku,
        "query=RAW_%25_B&offset=0&limit=1",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["needs_reconciliation"], true);
    assert_eq!(page["data"][0]["state"], "RECONCILIATION_REQUIRED");
    assert_eq!(page["data"][0]["available_micros"], Value::Null);
    assert_eq!(all(&pool, &f).await?, before);
    sqlx::query("update inventory_batch_balances set on_hand_micros=200000 where tenant_id=$1 and batch_id=$2").bind(f.tenant.0).bind(a).execute(&pool).await?;
    let (_, page) = get(&f, order, f.warehouse, f.sku, "offset=0&limit=2").await?;
    assert_eq!(page["needs_reconciliation"], true);
    Ok(())
}
#[tokio::test]
async fn material_batch_options_rechecks_current_warehouse_role_and_session() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order, _, _) = start(&pool).await?;
    assert_eq!(
        get(&f, Uuid::new_v4(), f.warehouse, f.sku, "offset=0&limit=1")
            .await?
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&f, order, f.warehouse, Uuid::new_v4(), "offset=0&limit=1")
            .await?
            .0,
        StatusCode::NOT_FOUND
    );
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.actor)
        .execute(&pool)
        .await?;
    assert_eq!(
        get(&f, order, f.warehouse, f.sku, "offset=0&limit=1")
            .await?
            .0,
        StatusCode::NOT_FOUND
    );
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.tenant.0)
    .bind(f.actor)
    .bind(f.warehouse)
    .execute(&pool)
    .await?;
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in('manufacturing.read','MFG.PRODUCTION.READ')").bind(f.tenant.0).execute(&pool).await?;
    assert_eq!(
        get(&f, order, f.warehouse, f.sku, "offset=0&limit=1")
            .await?
            .0,
        StatusCode::FORBIDDEN
    );
    sqlx::query("update auth_sessions set status='REVOKED' where tenant_id=$1 and session_id=$2")
        .bind(f.tenant.0)
        .bind(f.login.session)
        .execute(&pool)
        .await?;
    assert_eq!(
        get(&f, order, f.warehouse, f.sku, "offset=0&limit=1")
            .await?
            .0,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}
#[tokio::test]
async fn material_batch_options_full_1001_budget_rejects_even_a_one_row_filtered_page() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order, _, _) = start(&pool).await?;
    for n in 0..999 {
        batch(&pool, &f, &format!("OTHER-{n}"), Some(0), 5).await?;
    }
    let before = all(&pool, &f).await?;
    let (status, value) = get(
        &f,
        order,
        f.warehouse,
        f.sku,
        "query=RAW_%25_A&offset=0&limit=1",
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(value["code"], "inventory_batch_limit");
    assert_eq!(all(&pool, &f).await?, before);
    Ok(())
}
#[tokio::test]
async fn material_batch_options_real_issue_consume_return_preserve_selected_uuid_stock_and_cost()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order, a, _) = start(&pool).await?;
    let mut input = line(&f, "ISSUE", "0.25", None);
    input["batch_id"] = json!(a);
    let doc = draft(&f, order, vec![input]).await?;
    let path = format!("work-orders/{order}/material-documents/{doc}/confirm");
    let (status, value) = http(
        &f,
        &path,
        json!({"request_id":"batch-issue","expected_version":1}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    let issue: Uuid = serde_json::from_value(value["snapshot"]["effects"][0]["move_id"].clone())?;
    assert_eq!(
        value["snapshot"]["effects"][0]["value"]["amount_cent"],
        "300"
    );
    for (kind, request) in [("CONSUME", "batch-consume"), ("RETURN", "batch-return")] {
        let mut input = line(&f, kind, "0.125", Some(issue));
        input["batch_id"] = json!(a);
        let d = draft(&f, order, vec![input]).await?;
        let (status, value) = http(
            &f,
            &format!("work-orders/{order}/material-documents/{d}/confirm"),
            json!({"request_id":request,"expected_version":1}),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{value}");
        assert_eq!(value["snapshot"]["effects"][0]["batch_id"], json!(a));
        assert_eq!(
            value["snapshot"]["effects"][0]["value"]["amount_cent"],
            "150"
        );
    }
    let (_, page) = get(
        &f,
        order,
        f.warehouse,
        f.sku,
        "query=RAW_%25_A&offset=0&limit=1",
    )
    .await?;
    assert_eq!(page["data"][0]["on_hand_micros"], "275000");
    assert_eq!(page["warehouse_available_micros"], "875000");
    assert_eq!(
        read(&f, order, doc, Some("batch-issue")).await?.1["snapshot"],
        value["snapshot"]
    );
    Ok(())
}
