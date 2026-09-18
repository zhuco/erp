use axum::{
    body::{self, Body},
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use serde_json::json;
use tower::ServiceExt;

mod support;

fn build_state() -> (axum::Router, JwtService) {
    std::env::set_var("IDEMPOTENCY_DISABLED", "true");
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    build_state_with_db(db)
}

fn build_state_with_db(db: Db) -> (axum::Router, JwtService) {
    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db, std::sync::Arc::new(jwt.clone()));
    let app = build_app(state);
    (app, jwt)
}

fn build_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|p| p.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

fn tenant_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn build_admin_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|p| p.to_string()).collect(),
        is_admin: true,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

async fn setup_admin_data_scope_schema(pool: &sqlx::PgPool) {
    let statements = [
        r#"
        create table if not exists user_shop_access (
          tenant_id uuid not null,
          user_id uuid not null,
          shop_id uuid not null,
          primary key (tenant_id, user_id, shop_id)
        )
        "#,
        r#"
        create table if not exists user_warehouse_access (
          tenant_id uuid not null,
          user_id uuid not null,
          warehouse_id uuid not null,
          primary key (tenant_id, user_id, warehouse_id)
        )
        "#,
        r#"
        create table if not exists user_merchant_access (
          tenant_id uuid not null,
          user_id uuid not null,
          merchant_id uuid not null,
          primary key (tenant_id, user_id, merchant_id)
        )
        "#,
        r#"
        create table if not exists user_organization_access (
          tenant_id uuid not null,
          user_id uuid not null,
          organization_id uuid not null,
          primary key (tenant_id, user_id, organization_id)
        )
        "#,
    ];

    for statement in statements {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("admin data scope schema should be created");
    }
}

#[tokio::test]
async fn tenant_mismatch_orders_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();
    let order_id = uuid::Uuid::new_v4();

    let token = build_token(&jwt, &tenant_b, vec!["orders.read", "orders.read_raw"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/orders/{order_id}/raw"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_mismatch_inventory_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["inventory.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/inventory"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_mismatch_shipping_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["orders.write", "shipping.write"]);
    let body = serde_json::json!({
        "order_id": uuid::Uuid::new_v4(),
        "warehouse_id": uuid::Uuid::new_v4(),
        "carrier_code": "ZTO",
        "tracking_no": "TRACK-403"
    });
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/shipments"))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("Idempotency-Key", "tenant-mismatch-ship")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_mismatch_printing_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["print_jobs.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/print-jobs"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_mismatch_aftersales_forbidden() {
    let (app, jwt) = build_state();
    let tenant_a = tenant_id();
    let tenant_b = tenant_id();

    let token = build_token(&jwt, &tenant_b, vec!["aftersales.read"]);
    let request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_a}/after-sale-cases"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_data_scope_multi_entity_extended() {
    let database_url = support::test_database_url();
    let db = match Db::connect(&database_url).await {
        Ok(db) => db,
        Err(err) => {
            let message = err.to_string();
            if message.contains("pool timed out") || message.contains("DATABASE_URL") {
                eprintln!("skipping admin_data_scope_multi_entity_extended: {message}");
                return;
            }
            panic!("test db should connect for admin data scope: {err}");
        }
    };
    setup_admin_data_scope_schema(db.pool()).await;

    let (app, jwt) = build_state_with_db(db);
    let tenant = tenant_id();
    let user_id = uuid::Uuid::new_v4();
    let merchant_id = uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
    let org_id = uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();

    let token = build_admin_token(&jwt, &tenant, vec!["users.write"]);
    let payload = json!({
        "shop_ids": [],
        "warehouse_ids": [],
        "merchant_ids": [merchant_id.to_string()],
        "organization_ids": [org_id.to_string()]
    });

    let request = Request::builder()
        .uri(format!("/admin/users/{}/data-scopes", user_id))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let data_scope: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(data_scope["merchant_ids"], json!([merchant_id.to_string()]));
    assert_eq!(data_scope["organization_ids"], json!([org_id.to_string()]));
}
