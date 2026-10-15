//! Actual legacy movement routes must commit stock and both logs atomically.
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
    inventory::InventoryService,
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
    sku: Uuid,
}

async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let f = Fixture {
        tenant: TenantId(Uuid::new_v4()),
        actor: Uuid::new_v4(),
        warehouse: Uuid::new_v4(),
        sku: Uuid::new_v4(),
    };
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-INVENTORY-MUTATION-ATOMICITY')")
        .bind(Uuid::from(f.tenant))
        .execute(pool)
        .await?;
    sqlx::query("insert into users(tenant_id,user_id,email,password_hash) values($1,$2,$3,'QA-synthetic-no-login')")
        .bind(Uuid::from(f.tenant)).bind(f.actor).bind(format!("{}@mutation.qa.invalid",f.actor)).execute(pool).await?;
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Atomic mutation QA',$3)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.warehouse.to_string()).execute(pool).await?;
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-MUTATION',(select product_id from qa_parent))")
        .bind(Uuid::from(f.tenant))
        .bind(f.sku)
        .execute(pool)
        .await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.actor)
    .bind(f.warehouse)
    .execute(pool)
    .await?;
    InventoryService::new(pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
            },
            on_hand: parse_qty("2.9")?,
            reserved: parse_qty("0.9")?,
            frozen: parse_qty("0.9")?,
            in_transit: parse_qty("2.3")?,
            reason_type: "QA_MUTATION_OPENING".into(),
            ref_type: Some("QA_MUTATION_OPENING".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    eprintln!(
        "QA mutation fixture tenant={} actor={} warehouse={} sku={}",
        f.tenant, f.actor, f.warehouse, f.sku
    );
    Ok(f)
}

async fn facts(pool: &PgPool, f: &Fixture) -> Result<Value> {
    let text: String = sqlx::query_scalar("select jsonb_build_object('balances',coalesce((select jsonb_agg(to_jsonb(b) order by warehouse_id,sku_id) from public.inventory_balances b where tenant_id=$1),'[]'::jsonb),'ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from public.inventory_ledgers l where tenant_id=$1),'[]'::jsonb),'outbox',coalesce((select jsonb_agg(to_jsonb(o) order by outbox_id) from public.outbox_events o where tenant_id=$1),'[]'::jsonb),'operations',coalesce((select jsonb_agg(to_jsonb(o) order by log_id) from public.operation_logs o where tenant_id=$1),'[]'::jsonb),'audits',coalesce((select jsonb_agg(to_jsonb(a) order by audit_id) from public.audit_logs a where tenant_id=$1),'[]'::jsonb),'activations',coalesce((select jsonb_agg(to_jsonb(a) order by warehouse_id,sku_id) from public.inventory_quantity_activations a where tenant_id=$1),'[]'::jsonb))::text")
        .bind(Uuid::from(f.tenant)).fetch_one(pool).await?;
    Ok(serde_json::from_str(&text)?)
}

async fn request(
    f: &Fixture,
    action: &str,
    failure_table: Option<&str>,
) -> Result<(StatusCode, Value)> {
    let guard = isolated_pool().await?;
    let mut url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    if let Some(table) = failure_table {
        assert!(matches!(table, "operation_logs" | "audit_logs"));
        let schema = format!("qa_inventory_mutation_{}", Uuid::new_v4().simple());
        sqlx::query(&format!("create schema {schema}"))
            .execute(&guard)
            .await?;
        // This owned updatable view rejects inserts only on this request's
        // search_path. No trigger, public table or other tenant is altered.
        sqlx::query(&format!("create view {schema}.{table} as select * from public.{table} where false with local check option"))
            .execute(&guard).await?;
        let separator = if url.contains('?') { '&' } else { '?' };
        url = format!("{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog");
        assert!(super::approved_database_url(&url)?);
        eprintln!(
            "QA mutation failure scope tenant={} schema={schema} table={table}",
            f.tenant
        );
    }
    let db = Db::connect_lazy(&url)?;
    let jwt = Arc::new(JwtService::new("synthetic-inventory-mutation-key".into()));
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        f.actor.to_string(),
        f.tenant.to_string(),
        vec!["inventory.write".into()],
        false,
        now,
        now + 300,
    ))?;
    let mut body = json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"reason":"QA_MUTATION","reference_id":"QA-MUTATION-REF"});
    body[if action == "adjustments" {
        "quantity_delta"
    } else {
        "quantity"
    }] = json!(1);
    let path = format!("/v1/tenants/{}/inventory/{action}", f.tenant);
    let response = build_app(AppState::new(db, jwt))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&path)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA mutation HTTP tenant={} path={path} failure={} status={status} body={body}",
        f.tenant,
        failure_table.unwrap_or("none")
    );
    Ok((status, body))
}

async fn refuses_partial_effect(table: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for action in ["adjustments", "inbounds", "outbounds"] {
        let f = fixture(&pool).await?;
        let before = facts(&independent, &f).await?;
        let (status, body) = request(&f, action, Some(table)).await?;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
        assert!(
            body["text"]
                .as_str()
                .is_some_and(|text| text.contains("check option")),
            "must reach the owned log failure, got {body}"
        );
        let after = facts(&independent, &f).await?;
        eprintln!(
            "QA mutation refusal tenant={} action={action} table={table} before={before} after={after}",
            f.tenant
        );
        assert_eq!(
            before, after,
            "a log failure must not commit stock, ledger, activation, outbox or either log"
        );
    }
    Ok(())
}

#[tokio::test]
async fn inventory_mutation_failure_operation_has_no_partial_effect() -> Result<()> {
    refuses_partial_effect("operation_logs").await
}

#[tokio::test]
async fn inventory_mutation_failure_audit_has_no_partial_effect() -> Result<()> {
    refuses_partial_effect("audit_logs").await
}

#[tokio::test]
async fn inventory_mutation_success_commits_stock_and_both_logs() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for (action, hand, available, reason, operation) in [
        (
            "adjustments",
            "3.9",
            "2.1",
            "STOCK_COUNT_ADJUST",
            "INVENTORY_ADJUSTED",
        ),
        (
            "inbounds",
            "3.9",
            "2.1",
            "PURCHASE_RECEIPT",
            "INVENTORY_INBOUND",
        ),
        (
            "outbounds",
            "1.9",
            "0.1",
            "TRANSFER_OUT",
            "INVENTORY_OUTBOUND",
        ),
    ] {
        let f = fixture(&pool).await?;
        let (status, body) = request(&f, action, None).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["record"]["quantity_state"], "precise");
        assert_eq!(body["record"]["quantities"]["on_hand"], hand);
        assert_eq!(body["record"]["quantities"]["available"], available);
        let after = facts(&independent, &f).await?;
        assert_eq!(after["ledgers"].as_array().map(Vec::len), Some(2));
        assert_eq!(after["operations"].as_array().map(Vec::len), Some(1));
        assert_eq!(after["audits"].as_array().map(Vec::len), Some(1));
        assert_eq!(after["operations"][0]["action"], operation);
        assert_eq!(after["audits"][0]["action"], format!("INVENTORY.{reason}"));
        let quantities = &body["record"]["quantities"];
        assert_eq!(
            &after["operations"][0]["metadata"]["quantities"],
            quantities
        );
        assert_eq!(&after["audits"][0]["after_json"]["quantities"], quantities);
        assert_eq!(after["audits"][0]["user_id"], f.actor.to_string());
        eprintln!(
            "QA mutation committed tenant={} action={action} facts={after}",
            f.tenant
        );
    }
    Ok(())
}
