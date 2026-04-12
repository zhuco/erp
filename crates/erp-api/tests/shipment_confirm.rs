use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{build_app, AppState, Claims, JwtService};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn setup_schema(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let statements = [
        "create extension if not exists pgcrypto",
        r#"
        create table if not exists tenants (
          tenant_id uuid primary key,
          name text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shops (
          shop_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          platform text not null,
          platform_shop_id text not null,
          shop_name text not null default '',
          status text not null default 'ACTIVE',
          credentials jsonb not null default '{}'::jsonb,
          auth_meta_json jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipments (
          shipment_id uuid primary key,
          tenant_id uuid not null references tenants(tenant_id),
          shipment_type text not null default 'PLATFORM',
          shop_id uuid not null references shops(shop_id),
          order_id uuid not null,
          platform text not null,
          platform_order_id text not null,
          warehouse_id uuid,
          shipment_no text,
          carrier_code text,
          tracking_no text,
          status text not null default 'SUCCESS',
          ship_status text not null default 'NOT_SHIPPED',
          requested_at timestamptz,
          shipped_at timestamptz,
          weight_gram int,
          sender_address_id uuid,
          last_error text not null default '',
          last_push_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists shipment_packages (
          package_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          shipment_id uuid not null references shipments(shipment_id),
          package_no text not null,
          carrier_code text,
          tracking_no text,
          weight_gram int,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists outbox_events (
          tenant_id uuid not null references tenants(tenant_id),
          event_month date not null,
          outbox_id uuid not null default gen_random_uuid(),
          event_type text not null,
          payload jsonb not null,
          trace_id text,
          dedup_key text,
          status text not null default 'PENDING',
          attempts int not null default 0,
          next_retry_at timestamptz not null default now(),
          last_error text not null default '',
          last_attempt_at timestamptz not null default now(),
          published_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, event_month, outbox_id)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}

fn build_token(jwt: &JwtService, tenant_id: &str) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: vec!["shipping.write".to_string()],
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

#[tokio::test]
async fn confirm_shipment_enqueues_outbox() -> anyhow::Result<()> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let shipment_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'tenant')")
        .bind(tenant_id)
        .execute(db.pool())
        .await?;
    sqlx::query(
        "insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name) values ($1,$2,'taobao','SHOP-1','shop')",
    )
    .bind(shop_id)
    .bind(tenant_id)
    .execute(db.pool())
    .await?;
    sqlx::query(
        r#"
        insert into shipments (
            shipment_id, tenant_id, shipment_type, shop_id, order_id, platform,
            platform_order_id, carrier_code, tracking_no, status, ship_status
        ) values ($1,$2,'PLATFORM',$3,$4,'taobao','TB-ORDER-1','SF','TRACK-1','SUCCESS','NOT_SHIPPED')
        "#,
    )
    .bind(shipment_id)
    .bind(tenant_id)
    .bind(shop_id)
    .bind(order_id)
    .execute(db.pool())
    .await?;

    let jwt = JwtService::new("test-secret".to_string());
    let token = build_token(&jwt, &tenant_id.to_string());
    let state = AppState::new(db.clone(), std::sync::Arc::new(jwt));
    let app = build_app(state);

    let request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/shipments/{shipment_id}/confirm"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&serde_json::json!({}))?))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("queued"));

    let event_type: String =
        sqlx::query_scalar("select event_type from outbox_events order by created_at desc limit 1")
            .fetch_one(db.pool())
            .await?;
    assert_eq!(event_type, "SHIPMENT_CONFIRMED");

    let ship_status: String = sqlx::query_scalar(
        "select ship_status from shipments where tenant_id = $1 and shipment_id = $2",
    )
    .bind(tenant_id)
    .bind(shipment_id)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(ship_status, "SHIPPING");

    Ok(())
}
