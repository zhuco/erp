//! Original IDs must protect every movement and retain the original result.
use super::{
    Result,
    account_login::{AccountLogin, STOCK_PERMISSIONS, create_user},
    isolated_pool,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_domain::quantity::parse_qty;
use erp_repo::{
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use erp_types::TenantId;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub(super) struct Fixture {
    pub(super) tenant: TenantId,
    pub(super) actor: Uuid,
    pub(super) warehouse: Uuid,
    pub(super) sku: Uuid,
    pub(super) login: AccountLogin,
}

pub(super) async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let tenant = TenantId(Uuid::new_v4());
    let actor = Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-INVENTORY-COMMANDS')")
        .bind(tenant.0)
        .execute(pool)
        .await?;
    let login = create_user(
        pool,
        tenant,
        actor,
        format!("{actor}@command.qa.invalid"),
        STOCK_PERMISSIONS,
    )
    .await?;
    let f = Fixture {
        tenant,
        actor,
        warehouse: Uuid::new_v4(),
        sku: Uuid::new_v4(),
        login,
    };
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Inventory command QA',$3)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.warehouse.to_string()).execute(pool).await?;
    super::masters::sku(pool, f.tenant.0, f.sku, "QA-COMMAND").await?;
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
            on_hand: parse_qty("4.9")?,
            reserved: parse_qty("0.9")?,
            frozen: parse_qty("0.9")?,
            in_transit: parse_qty("2.3")?,
            reason_type: "QA_COMMAND_OPENING".into(),
            ref_type: Some("QA_COMMAND_OPENING".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    eprintln!(
        "QA inventory command fixture tenant={} actor={} warehouse={} sku={}",
        f.tenant, f.actor, f.warehouse, f.sku
    );
    Ok(f)
}

async fn post(
    f: &Fixture,
    action: &str,
    request_id: &str,
    quantity: i64,
) -> Result<(StatusCode, Value)> {
    let mut body = json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"request_id":request_id,
        "reason":"QA_COMMAND","reference_id":"QA-COMMAND-REF"});
    body[if action == "adjustments" {
        "quantity_delta"
    } else {
        "quantity"
    }] = json!(quantity);
    let path = format!("/v1/tenants/{}/inventory/{action}", f.tenant);
    let response = f
        .login
        .app(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&path)
                .header("authorization", format!("Bearer {}", f.login.token))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA inventory command HTTP tenant={} path={path} request_id={request_id} quantity={quantity} status={status} body={body}",
        f.tenant
    );
    Ok((status, body))
}

#[tokio::test]
async fn inventory_command_lost_response_reads_back_original_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let (status, first) = post(&f, "inbounds", &id, 1).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let path = format!("/v1/tenants/{}/inventory/commands/{id}", f.tenant);
    let response = f
        .login
        .app(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&path)
                .header("authorization", format!("Bearer {}", f.login.token))
                .body(Body::empty())?,
        )
        .await?;
    let lookup_status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let lookup: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(lookup_status, StatusCode::OK, "{lookup}");
    assert_eq!(lookup["request_id"], id);
    assert_eq!(lookup["effect_id"], first["effect_id"]);
    assert_eq!(lookup["replay"], true);
    let ledgers: i64 = sqlx::query_scalar(
        "select count(*) from inventory_ledgers where tenant_id = $1 and reason_type <> 'QA_COMMAND_OPENING'",
    )
    .bind(Uuid::from(f.tenant))
    .fetch_one(&independent)
    .await?;
    assert_eq!(ledgers, 1);
    eprintln!(
        "lost-response-ok tenant={} actor={} request={id} effect={} ledgers={ledgers}",
        f.tenant, f.actor, lookup["effect_id"]
    );
    Ok(())
}

#[tokio::test]
async fn inventory_command_outbound_original_identity_has_one_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let (first_status, first) = post(&f, "outbounds", &id, 1).await?;
    let (retry_status, retry) = post(&f, "outbounds", &id, 1).await?;
    assert_eq!(first_status, StatusCode::OK, "{first}");
    assert_eq!(retry_status, StatusCode::OK, "{retry}");
    let (hand,ledgers):(i64,i64)=sqlx::query_as("select on_hand_micros,(select count(*) from inventory_ledgers where tenant_id=$1) from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(f.sku).fetch_one(&independent).await?;
    eprintln!(
        "QA inventory command outbound readback tenant={} request_id={id} hand={hand} ledgers={ledgers}",
        f.tenant
    );
    assert_eq!(
        hand, 3_900_000,
        "same original ID must not deduct stock twice"
    );
    assert_eq!(ledgers, 2, "one opening and one posted movement");
    assert_eq!(first["record"], retry["record"]);
    assert_eq!(retry["replay"], true);
    Ok(())
}

#[tokio::test]
async fn inventory_command_inbound_replay_returns_original_balance_snapshot() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let original = Uuid::new_v4().to_string();
    let (status, first) = post(&f, "inbounds", &original, 1).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (status, later) = post(&f, "inbounds", &Uuid::new_v4().to_string(), 1).await?;
    assert_eq!(status, StatusCode::OK, "{later}");
    let (status, retry) = post(&f, "inbounds", &original, 1).await?;
    assert_eq!(status, StatusCode::OK, "{retry}");
    assert_eq!(first["record"]["quantities"]["on_hand"], "5.9");
    assert_eq!(later["record"]["quantities"]["on_hand"], "6.9");
    assert_eq!(
        retry["record"], first["record"],
        "later balance must not replace the original action result"
    );
    assert_eq!(retry["replay"], true);
    Ok(())
}
