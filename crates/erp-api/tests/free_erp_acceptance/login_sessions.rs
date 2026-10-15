//! Second-seat races use the shipped user update route and an independent count.
use super::fulfillment_sources::{allocated, fixture};
use super::{Result, isolated_pool};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::Value;
use sha2::Digest;
use std::sync::Arc;
use tokio::task::JoinSet;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn second_seat_http_race_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    eprintln!("second-seat tenant={tenant} actor={actor} session={session_id}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, credentials_epoch) values ($1, 'a03h-seat', 'FREE', 1, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, 'QA-synthetic-no-login', 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(format!("a03h-{actor}@example.test"))
    .execute(&pool)
    .await?;
    let role_id = Uuid::new_v4();
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a03h-admin')")
        .bind(role_id)
        .bind(tenant)
        .execute(&pool)
        .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(tenant)
        .bind(actor)
        .bind(role_id)
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'SYS.USER.WRITE')",
    )
    .bind(tenant)
    .bind(role_id)
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into auth_sessions(session_id, tenant_id, user_id, status, client_kind, expires_at, credentials_epoch) values ($1, $2, $3, 'ACTIVE', 'WEB', now() + interval '3600 seconds', 1)",
    )
    .bind(session_id)
    .bind(tenant)
    .bind(actor)
    .execute(&pool)
    .await?;
    let mut seconds = Vec::new();
    for index in 0..20 {
        let user_id = Uuid::new_v4();
        seconds.push(user_id);
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, 'QA-synthetic-no-login', 'DISABLED')",
        )
        .bind(user_id)
        .bind(tenant)
        .bind(format!("a03h-second-{index}@example.test"))
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("second-seat-http-test".into()));
    let now = chrono::Utc::now().timestamp() as usize;
    let mut claims = Claims::bearer(
        actor.to_string(),
        tenant.to_string(),
        vec!["users.write".into()],
        false,
        now,
        now + 3600,
    );
    claims.sid = Some(session_id.to_string());
    claims.cep = Some(1);
    let token = jwt.encode(&claims)?;
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let mut tasks = JoinSet::new();
    for user_id in seconds {
        let app = app.clone();
        let token = token.clone();
        tasks.spawn(async move {
            let response = app
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(format!("/v1/tenants/{tenant}/users/{user_id}"))
                        .header("authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"status":"ACTIVE"}"#))
                        .expect("request"),
                )
                .await
                .expect("response");
            let status = response.status();
            let bytes = response
                .into_body()
                .collect()
                .await
                .expect("body")
                .to_bytes();
            let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            (
                status,
                body["code"].as_str().unwrap_or("").to_string(),
                user_id,
            )
        });
    }
    let mut admitted = 0;
    let mut rejected = 0;
    while let Some(joined) = tasks.join_next().await {
        let (status, code, user_id) = joined?;
        eprintln!(
            "second-seat-result user={user_id} status={} code={code}",
            status.as_u16()
        );
        if status == StatusCode::OK {
            admitted += 1;
        } else if status == StatusCode::CONFLICT && code == "seat_limit" {
            rejected += 1;
        } else {
            anyhow::bail!("unexpected seat result {status} {code} {user_id}");
        }
    }
    assert_eq!(
        admitted, 0,
        "an occupied free seat cannot admit another login"
    );
    assert_eq!(rejected, 20);
    let active: i64 =
        sqlx::query_scalar("select count(*) from users where tenant_id = $1 and status = 'ACTIVE'")
            .bind(tenant)
            .fetch_one(&independent)
            .await?;
    assert_eq!(active, 1);
    let actor_status: String =
        sqlx::query_scalar("select status from users where tenant_id = $1 and user_id = $2")
            .bind(tenant)
            .bind(actor)
            .fetch_one(&independent)
            .await?;
    assert_eq!(actor_status, "ACTIVE");
    eprintln!("second-seat-ok tenant={tenant} active={active} rejected={rejected}");
    Ok(())
}

#[tokio::test]
async fn same_account_sessions_read_order_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let owner = fixture(&pool).await?;
    let tenant = owner.f.tenant.0;
    let actor = owner.f.actor;
    let order = owner.order;
    let shipment = allocated(&owner).await?;
    eprintln!("multi-session tenant={tenant} actor={actor} order={order} shipment={shipment}");
    sqlx::query(
        "insert into role_permissions(tenant_id, role_id, perm_code) select $1, role_id, 'OMS.SHIPPING.READ' from user_roles where tenant_id = $1 and user_id = $2",
    )
    .bind(tenant)
    .bind(actor)
    .execute(&pool)
    .await?;
    sqlx::query("delete from auth_sessions where tenant_id = $1 and user_id = $2")
        .bind(tenant)
        .bind(actor)
        .execute(&pool)
        .await?;
    let mut sessions = Vec::new();
    for kind in ["DESKTOP", "WEB", "WEB"] {
        let session_id = Uuid::new_v4();
        sqlx::query(
            "insert into auth_sessions(session_id, tenant_id, user_id, status, client_kind, expires_at, credentials_epoch) values ($1, $2, $3, 'ACTIVE', $4, now() + interval '3600 seconds', 1)",
        )
        .bind(session_id)
        .bind(tenant)
        .bind(actor)
        .bind(kind)
        .execute(&pool)
        .await?;
        sessions.push((session_id, kind));
    }
    let jwt = Arc::new(JwtService::new("multi-session-order-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt.clone(),
    ));
    let now = chrono::Utc::now().timestamp() as usize;
    for (session_id, kind) in &sessions {
        let mut claims = Claims::bearer(
            actor.to_string(),
            tenant.to_string(),
            vec!["orders.read".into(), "shipping.read".into()],
            false,
            now,
            now + 3600,
        );
        claims.sid = Some(session_id.to_string());
        claims.cep = Some(1);
        let token = jwt.encode(&claims)?;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/tenants/{tenant}/oms/shipments/{shipment}"))
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())?,
            )
            .await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        eprintln!(
            "multi-session-read session={session_id} kind={kind} status={}",
            status.as_u16()
        );
        assert_eq!(
            status,
            StatusCode::OK,
            "{body} {}",
            String::from_utf8_lossy(&bytes)
        );
        let seen = body["order_id"].as_str().unwrap_or("");
        assert_eq!(seen, order.to_string(), "{body}");
        assert_eq!(
            body["shipment_id"].as_str().unwrap_or(""),
            shipment.to_string(),
            "{body}"
        );
    }
    let active_users: i64 =
        sqlx::query_scalar("select count(*) from users where tenant_id = $1 and status = 'ACTIVE'")
            .bind(tenant)
            .fetch_one(&independent)
            .await?;
    let active_sessions: i64 = sqlx::query_scalar(
        "select count(*) from auth_sessions where tenant_id = $1 and user_id = $2 and status = 'ACTIVE'",
    )
    .bind(tenant)
    .bind(actor)
    .fetch_one(&independent)
    .await?;
    assert_eq!(active_users, 1);
    assert_eq!(active_sessions, 3);
    eprintln!(
        "multi-session-ok tenant={tenant} actor={actor} order={order} shipment={shipment} sessions={active_sessions}"
    );
    Ok(())
}

#[tokio::test]
async fn password_login_reuses_browser_tab_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    eprintln!("password-login tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, credentials_epoch) values ($1, 'a03j-login', 'FREE', 1, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=65536,t=3,p=4$e2sG+56d5ad9Zqhn3AyI0Q$+V/BdTqdWryDDwRw5r1Jz3knecMc7FwWB5FiPk5RDvw";
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(format!("a03j-{actor}@example.test"))
    .bind(hash)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("password-login-tab-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let email = format!("a03j-{actor}@example.test");
    let wrong = login_http(&app, tenant, &email, "wrong-a03j", "WEB", "tab-a03j").await?;
    assert_eq!(wrong.0, StatusCode::UNAUTHORIZED, "{}", wrong.1);
    let first =
        login_after_reported_backoff(&app, tenant, &email, "ErpQa!2026", "WEB", "tab-a03j").await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let session = Uuid::parse_str(first.1["session_id"].as_str().unwrap_or(""))?;
    let second = login_http(&app, tenant, &email, "ErpQa!2026", "WEB", "tab-a03j").await?;
    assert_eq!(second.0, StatusCode::OK, "{}", second.1);
    assert_eq!(
        second.1["session_id"], first.1["session_id"],
        "{}",
        second.1
    );
    let desktop = login_http(&app, tenant, &email, "ErpQa!2026", "DESKTOP", "desk-a03j").await?;
    assert_eq!(desktop.0, StatusCode::OK, "{}", desktop.1);
    let desktop_session = Uuid::parse_str(desktop.1["session_id"].as_str().unwrap_or(""))?;
    assert_ne!(desktop_session, session);
    let active: i64 = sqlx::query_scalar(
        "select count(*) from auth_sessions where tenant_id = $1 and user_id = $2 and status = 'ACTIVE'",
    )
    .bind(tenant)
    .bind(actor)
    .fetch_one(&independent)
    .await?;
    let users: i64 =
        sqlx::query_scalar("select count(*) from users where tenant_id = $1 and status = 'ACTIVE'")
            .bind(tenant)
            .fetch_one(&independent)
            .await?;
    assert_eq!(active, 2);
    assert_eq!(users, 1);
    eprintln!(
        "password-login-ok tenant={tenant} actor={actor} web={session} desktop={desktop_session} active={active}"
    );
    Ok(())
}

#[tokio::test]
async fn formal_session_renew_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("formal-renew tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03s-renew', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    let email = format!("a03s-{actor}@example.test");
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&email)
    .bind(hash)
    .execute(&pool)
    .await?;
    let role = Uuid::new_v4();
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a03s')")
        .bind(role)
        .bind(tenant)
        .execute(&pool)
        .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(tenant)
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'WMS.INVENTORY.WRITE')",
    )
    .bind(tenant)
    .bind(role)
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, 'a03s', $3)",
    )
    .bind(tenant)
    .bind(warehouse)
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id, sku_id, sku_code,product_id) values($1, $2, $3,(select product_id from qa_parent))")
        .bind(tenant)
        .bind(sku)
        .bind(format!("SKU-A03S-{sku}"))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id, user_id, warehouse_id) values ($1, $2, $3)",
    )
    .bind(tenant)
    .bind(actor)
    .bind(warehouse)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("formal-session-renew-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let login = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03s").await?;
    assert_eq!(login.0, StatusCode::OK, "{}", login.1);
    let session = login.1["session_id"].as_str().unwrap_or("").to_string();
    let old_token = login.1["access_token"].as_str().unwrap_or("").to_string();
    assert!(!session.is_empty() && !old_token.is_empty(), "{}", login.1);
    let before_expiry: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "select expires_at from auth_sessions where tenant_id = $1 and session_id = $2",
    )
    .bind(tenant)
    .bind(Uuid::parse_str(&session)?)
    .fetch_one(&independent)
    .await?;
    let request_id = format!("a03s-in-{session}");
    let inbound = export_http(
        &app,
        &old_token,
        "POST",
        &format!("/v1/tenants/{tenant}/inventory/inbounds"),
        Some(
            serde_json::json!({
                "warehouse_id": warehouse,
                "sku_id": sku,
                "request_id": request_id,
                "quantity": 1,
                "reason": "PURCHASE_RECEIPT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(inbound.0, StatusCode::OK, "{}", inbound.1);
    let ledgers_before: i64 = sqlx::query_scalar(
        "select count(*) from inventory_ledgers where tenant_id = $1 and ref_id = $2",
    )
    .bind(tenant)
    .bind(&request_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(ledgers_before, 1);
    sqlx::query(
        "update auth_sessions set expires_at = now() - interval '1 second' where tenant_id = $1 and session_id = $2",
    )
    .bind(tenant)
    .bind(Uuid::parse_str(&session)?)
    .execute(&pool)
    .await?;
    let expired = me_http(&app, &old_token).await?;
    assert_ne!(expired, StatusCode::OK, "expired token must not read /me");
    let renewed = export_http(
        &app,
        &old_token,
        "POST",
        "/auth/sessions/renew",
        Some(serde_json::json!({ "session_id": session }).to_string()),
    )
    .await?;
    assert_eq!(renewed.0, StatusCode::OK, "{}", renewed.1);
    let renewed_body: Value = serde_json::from_str(&renewed.1).unwrap_or(Value::Null);
    assert_eq!(renewed_body["session_id"].as_str().unwrap_or(""), session);
    let new_token = renewed_body["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(!new_token.is_empty(), "{}", renewed.1);
    let after_expiry: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "select expires_at from auth_sessions where tenant_id = $1 and session_id = $2",
    )
    .bind(tenant)
    .bind(Uuid::parse_str(&session)?)
    .fetch_one(&independent)
    .await?;
    assert!(
        after_expiry > before_expiry,
        "renewal must extend the same session"
    );
    let other = export_http(
        &app,
        &old_token,
        "POST",
        "/auth/sessions/renew",
        Some(serde_json::json!({ "session_id": Uuid::new_v4() }).to_string()),
    )
    .await?;
    assert_ne!(other.0, StatusCode::OK, "{}", other.1);
    let me = me_http(&app, &new_token).await?;
    assert_eq!(me, StatusCode::OK, "renewed token must read /me");
    let replay = export_http(
        &app,
        &new_token,
        "POST",
        &format!("/v1/tenants/{tenant}/inventory/inbounds"),
        Some(
            serde_json::json!({
                "warehouse_id": warehouse,
                "sku_id": sku,
                "request_id": request_id,
                "quantity": 1,
                "reason": "PURCHASE_RECEIPT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(replay.0, StatusCode::OK, "{}", replay.1);
    let replay_body: Value = serde_json::from_str(&replay.1).unwrap_or(Value::Null);
    assert_eq!(replay_body["replay"].as_bool().unwrap_or(false), true);
    let row: (i64, i64, String) = sqlx::query_as(
        "select
            (select count(*) from auth_sessions where tenant_id = $1 and user_id = $2),
            (select count(*) from users where tenant_id = $1 and status = 'ACTIVE'),
            (select status from auth_sessions where tenant_id = $1 and session_id = $3)",
    )
    .bind(tenant)
    .bind(actor)
    .bind(Uuid::parse_str(&session)?)
    .fetch_one(&independent)
    .await?;
    let ledgers: i64 = sqlx::query_scalar(
        "select count(*) from inventory_ledgers where tenant_id = $1 and ref_id = $2",
    )
    .bind(tenant)
    .bind(&request_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 1);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, "ACTIVE");
    assert_eq!(ledgers, 1);
    eprintln!(
        "formal-renew-ok tenant={tenant} actor={actor} session={session} sessions={} users={} status={} ledgers={ledgers} before={before_expiry} after={after_expiry}",
        row.0, row.1, row.2
    );
    Ok(())
}

#[tokio::test]
async fn outbox_cross_tenant_requeue_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    sqlx::query(
        "create table if not exists outbox_events (
            tenant_id uuid not null,
            event_month date not null,
            outbox_id uuid not null default gen_random_uuid(),
            event_type text not null,
            payload jsonb not null,
            status text not null default 'PENDING',
            attempts int not null default 0,
            next_retry_at timestamptz not null default now(),
            last_error text not null default '',
            created_at timestamptz not null default now(),
            updated_at timestamptz not null default now(),
            primary key (tenant_id, event_month, outbox_id)
        )",
    )
    .execute(&pool)
    .await?;
    for statement in [
        "alter table outbox_events add column if not exists attempts int not null default 0",
        "alter table outbox_events add column if not exists dedup_key text",
        "alter table outbox_events add column if not exists last_attempt_at timestamptz not null default now()",
        "alter table outbox_events add column if not exists published_at timestamptz",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let tenant = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let outbox_id = Uuid::new_v4();
    let marker = format!("MSG-A04L-{outbox_id}");
    eprintln!(
        "outbox-isolation tenant={tenant} foreign={foreign} actor={actor} outbox={outbox_id}"
    );
    for (id, name) in [(tenant, "a04l-owner"), (foreign, "a04l-foreign")] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(id)
        .bind(name)
        .execute(&pool)
        .await?;
    }
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    let owner_email = format!("a04l-{actor}@example.test");
    let foreign_email = format!("a04l-{foreign_actor}@example.test");
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE'), ($5, $6, $7, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&owner_email)
    .bind(hash)
    .bind(foreign_actor)
    .bind(foreign)
    .bind(&foreign_email)
    .execute(&pool)
    .await?;
    for (user, company) in [(actor, tenant), (foreign_actor, foreign)] {
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a04l')")
            .bind(role)
            .bind(company)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(company)
            .bind(user)
            .bind(role)
            .execute(&pool)
            .await?;
        for perm in ["SYS.DLQ.READ", "SYS.DLQ.REPLAY"] {
            sqlx::query(
                "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, $3)",
            )
            .bind(company)
            .bind(role)
            .bind(perm)
            .execute(&pool)
            .await?;
        }
    }
    sqlx::query(
        "insert into outbox_events(tenant_id, event_month, outbox_id, event_type, payload, status, last_error)
         values ($1, date '2026-10-01', $2, 'ORDER_SAVED', $3::jsonb, 'FAILED', 'owner failed once')",
    )
    .bind(tenant)
    .bind(outbox_id)
    .bind(serde_json::json!({"body": marker}).to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("outbox-isolation-http-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner = login_http(
        &app,
        tenant,
        &owner_email,
        "boundary-a03k",
        "WEB",
        "tab-a04l",
    )
    .await?;
    assert_eq!(owner.0, StatusCode::OK, "{}", owner.1);
    let owner_token = owner.1["access_token"].as_str().unwrap_or("").to_string();
    let other = login_http(
        &app,
        foreign,
        &foreign_email,
        "boundary-a03k",
        "WEB",
        "tab-a04l-foreign",
    )
    .await?;
    assert_eq!(other.0, StatusCode::OK, "{}", other.1);
    let foreign_token = other.1["access_token"].as_str().unwrap_or("").to_string();
    let listed = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{tenant}/outbox-events"),
        None,
    )
    .await?;
    assert_eq!(listed.0, StatusCode::OK, "{}", listed.1);
    assert!(listed.1.contains(&outbox_id.to_string()), "{}", listed.1);
    assert_eq!(listed.1.matches("ORDER_SAVED").count(), 1, "{}", listed.1);
    let stolen_list = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{tenant}/outbox-events"),
        None,
    )
    .await?;
    assert_ne!(stolen_list.0, StatusCode::OK, "{}", stolen_list.1);
    assert!(!stolen_list.1.contains(&marker), "{}", stolen_list.1);
    let stolen_requeue = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{tenant}/outbox-events/{outbox_id}/requeue"),
        Some("{}".to_string()),
    )
    .await?;
    assert_ne!(stolen_requeue.0, StatusCode::OK, "{}", stolen_requeue.1);
    assert!(!stolen_requeue.1.contains(&marker), "{}", stolen_requeue.1);
    let row: (String, String, i64) = sqlx::query_as(
        "select status, payload->>'body',
            (select count(*) from outbox_events where tenant_id = $2)
         from outbox_events where tenant_id = $1 and outbox_id = $3",
    )
    .bind(tenant)
    .bind(foreign)
    .bind(outbox_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "FAILED");
    assert_eq!(row.1, marker);
    assert_eq!(row.2, 0);
    eprintln!(
        "outbox-isolation-ok tenant={tenant} foreign={foreign} outbox={outbox_id} status={} body={} foreign_rows={}",
        row.0, row.1, row.2
    );
    Ok(())
}

#[tokio::test]
async fn warehouse_scope_hides_other_stock_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let allowed = Uuid::new_v4();
    let hidden = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("warehouse-scope tenant={tenant} actor={actor} allowed={allowed} hidden={hidden}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a04k-scope', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    let email = format!("a04k-{actor}@example.test");
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&email)
    .bind(hash)
    .execute(&pool)
    .await?;
    let role = Uuid::new_v4();
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a04k')")
        .bind(role)
        .bind(tenant)
        .execute(&pool)
        .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(tenant)
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await?;
    for perm in ["SYS.USER.WRITE", "SYS.USER.READ", "WMS.INVENTORY.READ"] {
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, $3)",
        )
        .bind(tenant)
        .bind(role)
        .bind(perm)
        .execute(&pool)
        .await?;
    }
    for (id, name) in [(allowed, "WH-A04K-ALLOW"), (hidden, "WH-A04K-HIDE")] {
        sqlx::query(
            "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
        )
        .bind(tenant)
        .bind(id)
        .bind(name)
        .bind(id.to_string())
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into user_warehouse_access(tenant_id, user_id, warehouse_id) values ($1, $2, $3)",
        )
        .bind(tenant)
        .bind(actor)
        .bind(id)
        .execute(&pool)
        .await?;
    }
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id, sku_id, sku_code,product_id) values($1, $2, $3,(select product_id from qa_parent))")
        .bind(tenant)
        .bind(sku)
        .bind(format!("SKU-A04K-{sku}"))
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into inventory_balances(tenant_id, warehouse_id, sku_id, on_hand, reserved, frozen, in_transit, on_hand_micros, reserved_micros, frozen_micros, in_transit_micros) values ($1, $2, $3, 3, 0, 0, 0, 3000000, 0, 0, 0), ($1, $4, $3, 9, 0, 0, 0, 9000000, 0, 0, 0)",
    )
    .bind(tenant)
    .bind(allowed)
    .bind(sku)
    .bind(hidden)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("warehouse-scope-http-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let login = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a04k").await?;
    assert_eq!(login.0, StatusCode::OK, "{}", login.1);
    let token = login.1["access_token"].as_str().unwrap_or("").to_string();
    let foreign = Uuid::new_v4();
    let stolen = export_http(
        &app,
        &token,
        "PUT",
        &format!("/v1/tenants/{tenant}/users/{actor}/data-scopes"),
        Some(
            serde_json::json!({
                "shop_ids": [],
                "warehouse_ids": [foreign],
                "merchant_ids": [],
                "organization_ids": []
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(stolen.0, StatusCode::OK, "{}", stolen.1);
    let narrowed = export_http(
        &app,
        &token,
        "PUT",
        &format!("/v1/tenants/{tenant}/users/{actor}/data-scopes"),
        Some(
            serde_json::json!({
                "shop_ids": [],
                "warehouse_ids": [allowed],
                "merchant_ids": [],
                "organization_ids": []
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(narrowed.0, StatusCode::OK, "{}", narrowed.1);
    let listed = export_http(
        &app,
        &token,
        "GET",
        &format!("/v1/tenants/{tenant}/inventory"),
        None,
    )
    .await?;
    assert_eq!(listed.0, StatusCode::OK, "{}", listed.1);
    assert!(listed.1.contains(&allowed.to_string()), "{}", listed.1);
    assert!(
        !listed.1.contains(&hidden.to_string()),
        "hidden warehouse leaked: {}",
        listed.1
    );
    assert!(!listed.1.contains("WH-A04K-HIDE"), "{}", listed.1);
    let direct = export_http(
        &app,
        &token,
        "GET",
        &format!("/v1/tenants/{tenant}/inventory?warehouse_id={hidden}"),
        None,
    )
    .await?;
    assert_ne!(direct.0, StatusCode::OK, "{}", direct.1);
    assert!(!direct.1.contains("WH-A04K-HIDE"), "{}", direct.1);
    let grants: i64 = sqlx::query_scalar(
        "select count(*) from user_warehouse_access where tenant_id = $1 and user_id = $2",
    )
    .bind(tenant)
    .bind(actor)
    .fetch_one(&independent)
    .await?;
    let kept: i64 = sqlx::query_scalar(
        "select count(*) from user_warehouse_access where tenant_id = $1 and user_id = $2 and warehouse_id = $3",
    )
    .bind(tenant)
    .bind(actor)
    .bind(allowed)
    .fetch_one(&independent)
    .await?;
    let balances: (i64, i64) = sqlx::query_as(
        "select
            (select on_hand_micros from inventory_balances where tenant_id = $1 and warehouse_id = $2),
            (select on_hand_micros from inventory_balances where tenant_id = $1 and warehouse_id = $3)",
    )
    .bind(tenant)
    .bind(allowed)
    .bind(hidden)
    .fetch_one(&independent)
    .await?;
    assert_eq!(grants, 1);
    assert_eq!(kept, 1);
    assert_eq!(balances, (3_000_000, 9_000_000));
    eprintln!(
        "warehouse-scope-ok tenant={tenant} actor={actor} allowed={allowed} hidden={hidden} grants={grants}"
    );
    Ok(())
}

#[tokio::test]
async fn formal_password_recover_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let email = format!("a03t-{actor}@example.test");
    eprintln!("formal-recover tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03t-recover', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&email)
    .bind(hash)
    .execute(&pool)
    .await?;
    let token = "recover-a03t-once";
    let digest = sha2::Sha256::digest(format!("{tenant}:{email}:{token}").as_bytes());
    let token_hash = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    sqlx::query(
        "insert into password_reset_tokens(tenant_id, user_id, token_hash, expires_at) values ($1, $2, $3, now() + interval '600 seconds')",
    )
    .bind(tenant)
    .bind(actor)
    .bind(&token_hash)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("formal-password-recover-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let old_login = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03t").await?;
    assert_eq!(old_login.0, StatusCode::OK, "{}", old_login.1);
    let old_token = old_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let session = old_login.1["session_id"].as_str().unwrap_or("").to_string();
    let before = me_http(&app, &old_token).await?;
    assert_eq!(before, StatusCode::OK);
    let recovered = export_http(
        &app,
        "",
        "POST",
        "/auth/password/recover",
        Some(
            serde_json::json!({
                "tenant_id": tenant,
                "email": email,
                "token": token,
                "new_password": "recovered-a03t"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(recovered.0, StatusCode::OK, "{}", recovered.1);
    let recovered_body: Value = serde_json::from_str(&recovered.1).unwrap_or(Value::Null);
    assert_eq!(recovered_body["status"].as_str().unwrap_or(""), "RECOVERED");
    assert!(
        !recovered.1.contains(token),
        "response must not echo the token"
    );
    assert!(
        !recovered.1.contains("recovered-a03t"),
        "response must not echo the password"
    );
    let reused = export_http(
        &app,
        "",
        "POST",
        "/auth/password/recover",
        Some(
            serde_json::json!({
                "tenant_id": tenant,
                "email": email,
                "token": token,
                "new_password": "recovered-again"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(reused.0, StatusCode::OK, "{}", reused.1);
    let stale = me_http(&app, &old_token).await?;
    assert_ne!(stale, StatusCode::OK, "old token must fail after recovery");
    let old_password =
        login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03t-old").await?;
    assert_ne!(old_password.0, StatusCode::OK, "{}", old_password.1);
    let new_login = login_after_reported_backoff(
        &app,
        tenant,
        &email,
        "recovered-a03t",
        "WEB",
        "tab-a03t-new",
    )
    .await?;
    assert_eq!(new_login.0, StatusCode::OK, "{}", new_login.1);
    let row: (i32, i64, Option<chrono::DateTime<chrono::Utc>>, String) = sqlx::query_as(
        "select t.credentials_epoch,
            (select count(*) from users where tenant_id = $1 and status = 'ACTIVE'),
            r.used_at,
            u.password_hash
         from tenants t
         join users u on u.tenant_id = t.tenant_id and u.user_id = $2
         join password_reset_tokens r on r.tenant_id = t.tenant_id and r.user_id = u.user_id",
    )
    .bind(tenant)
    .bind(actor)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 2);
    assert_eq!(row.1, 1);
    assert!(row.2.is_some());
    assert!(row.3.starts_with("$argon2id$"));
    assert!(!row.3.contains("recovered-a03t"));
    eprintln!(
        "formal-recover-ok tenant={tenant} actor={actor} session={session} epoch={} users={} used={}",
        row.0,
        row.1,
        row.2.unwrap_or_default()
    );
    Ok(())
}

#[tokio::test]
async fn session_limit_boundary_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    eprintln!("session-limit tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03k-limit', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    let email = format!("a03k-{actor}@example.test");
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&email)
    .bind(hash)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("session-limit-boundary-test".into()));
    let shared = sqlx::postgres::PgPoolOptions::new()
        .max_connections(24)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)
        .await?;
    let app = build_app(AppState::new(Db::from_pool(shared), jwt));
    let mut held = Vec::new();
    for index in 0..8 {
        let (status, body) = login_http(
            &app,
            tenant,
            &email,
            "boundary-a03k",
            "WEB",
            &format!("seed-a03k-{index}"),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        held.push(body["session_id"].as_str().unwrap_or("").to_string());
    }
    assert_eq!(held.len(), 8);
    let seed = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "seed-a03k-0").await?;
    assert_eq!(seed.0, StatusCode::OK, "{}", seed.1);
    assert_eq!(seed.1["session_id"], held[0]);
    let revoked_seed = revoke_http(
        &app,
        seed.1["access_token"].as_str().unwrap_or(""),
        &held[0],
    )
    .await?;
    assert_eq!(
        revoked_seed,
        StatusCode::OK,
        "seed session must be revoked before the seven-session race"
    );
    let mut opened = 0_i64;
    let mut refused = 0_i64;
    for round in 0..100 {
        let results = race_logins(&app, tenant, &email, 20, &format!("race7-{round}")).await?;
        let mut winner = None;
        for (status, body) in results {
            if status == StatusCode::OK {
                opened += 1;
                let session = body["session_id"].as_str().unwrap_or("").to_string();
                let token = body["access_token"].as_str().unwrap_or("").to_string();
                assert!(!session.is_empty() && !token.is_empty(), "{body}");
                winner = Some((session, token));
            } else if status == StatusCode::CONFLICT && body["code"] == "session_limit" {
                refused += 1;
            } else {
                anyhow::bail!("unexpected seven-session result {status} {body}");
            }
        }
        let (session, token) =
            winner.ok_or_else(|| anyhow::anyhow!("round {round} opened none"))?;
        let me = me_http(&app, &token).await?;
        assert_eq!(me, StatusCode::OK, "protected call for {session}");
        let revoked = revoke_http(&app, &token, &session).await?;
        assert_eq!(revoked, StatusCode::OK, "{session}");
    }
    assert_eq!(opened, 100);
    assert_eq!(refused, 1900);
    let at_seven: i64 = sqlx::query_scalar(
        "select count(*) from auth_sessions where tenant_id = $1 and user_id = $2 and status = 'ACTIVE' and expires_at > now()",
    )
    .bind(tenant)
    .bind(actor)
    .fetch_one(&independent)
    .await?;
    assert_eq!(at_seven, 7);
    let eighth = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "seed-a03k-8").await?;
    assert_eq!(eighth.0, StatusCode::OK, "{}", eighth.1);
    held.push(eighth.1["session_id"].as_str().unwrap_or("").to_string());
    let mut full_refused = 0_i64;
    for round in 0..100 {
        let (status, body) = login_http(
            &app,
            tenant,
            &email,
            "boundary-a03k",
            "WEB",
            &format!("full-a03k-{round}"),
        )
        .await?;
        if status == StatusCode::CONFLICT && body["code"] == "session_limit" {
            full_refused += 1;
        } else {
            anyhow::bail!("full boundary admitted {status} {body}");
        }
    }
    assert_eq!(full_refused, 100);
    let still_seven: i64 = sqlx::query_scalar(
        "select count(*) from auth_sessions where tenant_id = $1 and user_id = $2 and status = 'ACTIVE' and expires_at > now()",
    )
    .bind(tenant)
    .bind(actor)
    .fetch_one(&independent)
    .await?;
    assert_eq!(still_seven, 8);
    let keeper = &held[2];
    let victim = held[1].clone();
    let (status, body) = login_http(
        &app,
        tenant,
        &email,
        "boundary-a03k",
        "WEB",
        "revoke-reader",
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let seed = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "seed-a03k-1").await?;
    assert_eq!(seed.0, StatusCode::OK, "{}", seed.1);
    assert_eq!(seed.1["session_id"], victim);
    let revoked = revoke_http(&app, seed.1["access_token"].as_str().unwrap_or(""), &victim).await?;
    assert_eq!(revoked, StatusCode::OK);
    let mut after_revoke = 0_i64;
    for round in 0..100 {
        let results = race_logins(&app, tenant, &email, 20, &format!("revoke-{round}")).await?;
        let mut winner = None;
        for (status, body) in results {
            if status == StatusCode::OK {
                after_revoke += 1;
                winner = Some((
                    body["session_id"].as_str().unwrap_or("").to_string(),
                    body["access_token"].as_str().unwrap_or("").to_string(),
                ));
            } else if status == StatusCode::CONFLICT && body["code"] == "session_limit" {
            } else {
                anyhow::bail!("unexpected revoke-boundary result {status} {body}");
            }
        }
        let (session, token) =
            winner.ok_or_else(|| anyhow::anyhow!("revoke round {round} opened none"))?;
        assert_eq!(me_http(&app, &token).await?, StatusCode::OK);
        assert_eq!(revoke_http(&app, &token, &session).await?, StatusCode::OK);
    }
    assert_eq!(after_revoke, 100);
    let refill = login_http(
        &app,
        tenant,
        &email,
        "boundary-a03k",
        "WEB",
        "seed-before-expiry",
    )
    .await?;
    assert_eq!(refill.0, StatusCode::OK, "{}", refill.1);
    let before_expiry: i64 = sqlx::query_scalar(
        "select count(*) from auth_sessions where tenant_id=$1 and user_id=$2 and status='ACTIVE' and expires_at>now()",
    ).bind(tenant).bind(actor).fetch_one(&independent).await?;
    assert_eq!(
        before_expiry, 8,
        "expiry race starts full; one expiry frees exactly one slot"
    );
    sqlx::query(
        "update auth_sessions set expires_at = now() - interval '1 second' where tenant_id = $1 and session_id = $2::uuid",
    )
    .bind(tenant)
    .bind(keeper)
    .execute(&pool)
    .await?;
    let mut after_expiry = 0_i64;
    for round in 0..100 {
        let results = race_logins(&app, tenant, &email, 20, &format!("expire-{round}")).await?;
        let mut winner = None;
        for (status, body) in results {
            if status == StatusCode::OK {
                after_expiry += 1;
                winner = Some((
                    body["session_id"].as_str().unwrap_or("").to_string(),
                    body["access_token"].as_str().unwrap_or("").to_string(),
                ));
            } else if status == StatusCode::CONFLICT && body["code"] == "session_limit" {
            } else {
                anyhow::bail!("unexpected expiry-boundary result {status} {body}");
            }
        }
        let (session, token) =
            winner.ok_or_else(|| anyhow::anyhow!("expiry round {round} opened none"))?;
        assert_eq!(me_http(&app, &token).await?, StatusCode::OK);
        assert_eq!(revoke_http(&app, &token, &session).await?, StatusCode::OK);
    }
    assert_eq!(after_expiry, 100);
    let active: i64 = sqlx::query_scalar(
        "select count(*) from auth_sessions where tenant_id = $1 and user_id = $2 and status = 'ACTIVE' and expires_at > now()",
    )
    .bind(tenant)
    .bind(actor)
    .fetch_one(&independent)
    .await?;
    let users: i64 =
        sqlx::query_scalar("select count(*) from users where tenant_id = $1 and status = 'ACTIVE'")
            .bind(tenant)
            .fetch_one(&independent)
            .await?;
    assert_eq!(active, 7, "revoke and expiry each restore one live session");
    assert_eq!(users, 1);
    eprintln!(
        "session-limit-ok tenant={tenant} actor={actor} opened={opened} refused={refused} full={full_refused} revoke={after_revoke} expiry={after_expiry} active={active}"
    );
    Ok(())
}

#[tokio::test]
async fn cross_session_revoke_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    eprintln!("cross-revoke tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03l-revoke', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    let email = format!("a03l-{actor}@example.test");
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&email)
    .bind(hash)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("cross-session-revoke-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let web = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03l").await?;
    assert_eq!(web.0, StatusCode::OK, "{}", web.1);
    let desk = login_http(
        &app,
        tenant,
        &email,
        "boundary-a03k",
        "DESKTOP",
        "desk-a03l",
    )
    .await?;
    assert_eq!(desk.0, StatusCode::OK, "{}", desk.1);
    let web_session = web.1["session_id"].as_str().unwrap_or("").to_string();
    let web_token = web.1["access_token"].as_str().unwrap_or("").to_string();
    let desk_session = desk.1["session_id"].as_str().unwrap_or("").to_string();
    let desk_token = desk.1["access_token"].as_str().unwrap_or("").to_string();
    assert_ne!(web_session, desk_session);
    assert_eq!(me_http(&app, &web_token).await?, StatusCode::OK);
    assert_eq!(me_http(&app, &desk_token).await?, StatusCode::OK);
    let revoked = revoke_http(&app, &web_token, &desk_session).await?;
    assert_eq!(revoked, StatusCode::OK);
    let desk_after = me_http(&app, &desk_token).await?;
    assert_eq!(desk_after, StatusCode::UNAUTHORIZED);
    let web_after = me_http(&app, &web_token).await?;
    assert_eq!(web_after, StatusCode::OK);
    let reason: String = sqlx::query_scalar(
        "select status || ':' || coalesce(revoke_reason, '') from auth_sessions where tenant_id = $1 and session_id = $2::uuid",
    )
    .bind(tenant)
    .bind(&desk_session)
    .fetch_one(&independent)
    .await?;
    let web_status: String = sqlx::query_scalar(
        "select status from auth_sessions where tenant_id = $1 and session_id = $2::uuid",
    )
    .bind(tenant)
    .bind(&web_session)
    .fetch_one(&independent)
    .await?;
    assert_eq!(reason, "REVOKED:USER");
    assert_eq!(web_status, "ACTIVE");
    eprintln!(
        "cross-revoke-ok tenant={tenant} actor={actor} web={web_session} desktop={desk_session} desktop_status={reason} web_status={web_status}"
    );
    Ok(())
}

#[tokio::test]
async fn password_change_invalidates_other_session_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    eprintln!("password-invalidate tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03m-pass', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    let email = format!("a03m-{actor}@example.test");
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&email)
    .bind(hash)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("password-invalidate-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let web = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03m").await?;
    let desk = login_http(
        &app,
        tenant,
        &email,
        "boundary-a03k",
        "DESKTOP",
        "desk-a03m",
    )
    .await?;
    assert_eq!(web.0, StatusCode::OK, "{}", web.1);
    assert_eq!(desk.0, StatusCode::OK, "{}", desk.1);
    let web_token = web.1["access_token"].as_str().unwrap_or("").to_string();
    let desk_token = desk.1["access_token"].as_str().unwrap_or("").to_string();
    let web_session = web.1["session_id"].as_str().unwrap_or("").to_string();
    let desk_session = desk.1["session_id"].as_str().unwrap_or("").to_string();
    assert_eq!(me_http(&app, &desk_token).await?, StatusCode::OK);
    let changed = password_http(&app, &web_token, "boundary-a03k", "boundary-a03m-next").await?;
    assert_eq!(changed.0, StatusCode::OK, "{}", changed.1);
    let desk_after = me_http(&app, &desk_token).await?;
    assert_eq!(desk_after, StatusCode::UNAUTHORIZED);
    let web_after = me_http(&app, &web_token).await?;
    assert_eq!(web_after, StatusCode::UNAUTHORIZED);
    let epoch: i32 =
        sqlx::query_scalar("select credentials_epoch from tenants where tenant_id = $1")
            .bind(tenant)
            .fetch_one(&independent)
            .await?;
    let active: i64 = sqlx::query_scalar(
        "select count(*) from auth_sessions where tenant_id = $1 and status = 'ACTIVE'",
    )
    .bind(tenant)
    .fetch_one(&independent)
    .await?;
    let plain: i64 = sqlx::query_scalar(
        "select count(*) from users where tenant_id = $1 and position('boundary-a03m-next' in password_hash) > 0",
    )
    .bind(tenant)
    .fetch_one(&independent)
    .await?;
    assert_eq!(epoch, 2);
    assert_eq!(active, 2);
    assert_eq!(plain, 0);
    eprintln!(
        "password-invalidate-ok tenant={tenant} actor={actor} web={web_session} desktop={desk_session} epoch={epoch} active={active}"
    );
    Ok(())
}

#[tokio::test]
async fn disable_account_invalidates_other_session_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    eprintln!("disable-invalidate tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03n-disable', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    let email = format!("a03n-{actor}@example.test");
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&email)
    .bind(hash)
    .execute(&pool)
    .await?;
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a03n-admin')")
        .bind(Uuid::new_v4())
        .bind(tenant)
        .execute(&pool)
        .await?;
    let role: Uuid = sqlx::query_scalar("select role_id from roles where tenant_id = $1")
        .bind(tenant)
        .fetch_one(&pool)
        .await?;
    sqlx::query(
        "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'SYS.USER.WRITE')",
    )
    .bind(tenant)
    .bind(role)
    .execute(&pool)
    .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(tenant)
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await?;
    let jwt = Arc::new(JwtService::new("disable-invalidate-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let web = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03n").await?;
    let desk = login_http(
        &app,
        tenant,
        &email,
        "boundary-a03k",
        "DESKTOP",
        "desk-a03n",
    )
    .await?;
    assert_eq!(web.0, StatusCode::OK, "{}", web.1);
    assert_eq!(desk.0, StatusCode::OK, "{}", desk.1);
    let web_token = web.1["access_token"].as_str().unwrap_or("").to_string();
    let desk_token = desk.1["access_token"].as_str().unwrap_or("").to_string();
    let web_session = web.1["session_id"].as_str().unwrap_or("").to_string();
    let desk_session = desk.1["session_id"].as_str().unwrap_or("").to_string();
    assert_eq!(me_http(&app, &desk_token).await?, StatusCode::OK);
    let disabled = disable_http(&app, &web_token, tenant, actor).await?;
    assert_eq!(disabled.0, StatusCode::OK, "{}", disabled.1);
    assert_eq!(me_http(&app, &desk_token).await?, StatusCode::UNAUTHORIZED);
    assert_eq!(me_http(&app, &web_token).await?, StatusCode::UNAUTHORIZED);
    let status: String =
        sqlx::query_scalar("select status from users where tenant_id = $1 and user_id = $2")
            .bind(tenant)
            .bind(actor)
            .fetch_one(&independent)
            .await?;
    let epoch: i32 =
        sqlx::query_scalar("select credentials_epoch from tenants where tenant_id = $1")
            .bind(tenant)
            .fetch_one(&independent)
            .await?;
    assert_eq!(status, "DISABLED");
    assert_eq!(epoch, 2);
    eprintln!(
        "disable-invalidate-ok tenant={tenant} actor={actor} web={web_session} desktop={desk_session} status={status} epoch={epoch}"
    );
    Ok(())
}

#[tokio::test]
async fn expired_token_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    eprintln!("expired-token tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03o-expire', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    let email = format!("a03o-{actor}@example.test");
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(&email)
    .bind(hash)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("expired-token-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let web = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03o").await?;
    assert_eq!(web.0, StatusCode::OK, "{}", web.1);
    let session = web.1["session_id"].as_str().unwrap_or("").to_string();
    let token = web.1["access_token"].as_str().unwrap_or("").to_string();
    assert_eq!(me_http(&app, &token).await?, StatusCode::OK);
    sqlx::query(
        "update auth_sessions set expires_at = now() - interval '1 second' where tenant_id = $1 and session_id = $2::uuid",
    )
    .bind(tenant)
    .bind(&session)
    .execute(&pool)
    .await?;
    assert_eq!(me_http(&app, &token).await?, StatusCode::UNAUTHORIZED);
    let status: String = sqlx::query_scalar(
        "select status from auth_sessions where tenant_id = $1 and session_id = $2::uuid and expires_at <= now()",
    )
    .bind(tenant)
    .bind(&session)
    .fetch_one(&independent)
    .await?;
    assert_eq!(status, "ACTIVE");
    eprintln!("expired-token-ok tenant={tenant} actor={actor} session={session} status={status}");
    Ok(())
}

#[tokio::test]
async fn cross_tenant_session_does_not_read_foreign_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("cross-tenant tenant={owner} foreign={foreign} actor={owner_actor}");
    for (tenant, name) in [(owner, "a03p-owner"), (foreign, "a03p-foreign")] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
    }
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor) in [(owner, owner_actor), (foreign, foreign_actor)] {
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("a03p-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("cross-tenant-session-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a03p-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a03p",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    let token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let session = owner_login.1["session_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert_eq!(me_http(&app, &token).await?, StatusCode::OK);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/tenants/{foreign}/users/{foreign_actor}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "{}",
        String::from_utf8_lossy(&bytes)
    );
    let foreign_sessions: i64 =
        sqlx::query_scalar("select count(*) from auth_sessions where tenant_id = $1")
            .bind(foreign)
            .fetch_one(&independent)
            .await?;
    let owner_status: String = sqlx::query_scalar(
        "select status from auth_sessions where tenant_id = $1 and session_id = $2::uuid",
    )
    .bind(owner)
    .bind(&session)
    .fetch_one(&independent)
    .await?;
    assert_eq!(foreign_sessions, 0);
    assert_eq!(owner_status, "ACTIVE");
    eprintln!(
        "cross-tenant-ok tenant={owner} foreign={foreign} actor={owner_actor} session={session} foreign_sessions={foreign_sessions}"
    );
    Ok(())
}

#[tokio::test]
async fn two_clients_order_version_conflict_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let order = Uuid::new_v4();
    let shop = Uuid::new_v4();
    eprintln!("order-conflict tenant={tenant} actor={actor} order={order}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03q-order', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(format!("a03q-{actor}@example.test"))
    .bind(hash)
    .execute(&pool)
    .await?;
    let role = Uuid::new_v4();
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a03q-order')")
        .bind(role)
        .bind(tenant)
        .execute(&pool)
        .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(tenant)
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.write')",
    )
    .bind(tenant)
    .bind(role)
    .execute(&pool)
    .await?;
    sqlx::query("insert into shops(tenant_id, shop_id, platform, platform_shop_id, shop_name) values ($1, $2, 'TAOBAO', $3, 'a03q')")
        .bind(tenant)
        .bind(shop)
        .bind(shop.to_string())
        .execute(&pool)
        .await?;
    sqlx::query("insert into orders(tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status, order_created_at, order_status, fulfillment_status, exception_status, exception_flags, inventory_shortage, version) values ($1, '2026-10-01', $2, 'TAOBAO', $3, $4, 'PAID', now(), 'PENDING_ALLOCATE', 'NOT_ALLOCATED', 'NONE', '[]', false, 7)")
        .bind(tenant)
        .bind(order)
        .bind(shop)
        .bind(order.to_string())
        .execute(&pool)
        .await?;
    sqlx::query("insert into order_index(tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at) values ($1, $2, '2026-10-01', 'TAOBAO', $3, $4, now())")
        .bind(tenant)
        .bind(order)
        .bind(order.to_string())
        .bind(shop)
        .execute(&pool)
        .await?;
    let jwt = Arc::new(JwtService::new("order-version-conflict-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let email = format!("a03q-{actor}@example.test");
    let web = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03q").await?;
    let desk = login_http(
        &app,
        tenant,
        &email,
        "boundary-a03k",
        "DESKTOP",
        "desk-a03q",
    )
    .await?;
    assert_eq!(web.0, StatusCode::OK, "{}", web.1);
    assert_eq!(desk.0, StatusCode::OK, "{}", desk.1);
    let web_token = web.1["access_token"].as_str().unwrap_or("");
    let desk_token = desk.1["access_token"].as_str().unwrap_or("");
    let saved = order_edit(
        &app,
        web_token,
        tenant,
        order,
        7,
        "seller_remark",
        "web-edit",
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let blocked = order_edit(
        &app,
        desk_token,
        tenant,
        order,
        7,
        "buyer_message",
        "desk-edit",
    )
    .await?;
    assert_eq!(blocked.0, StatusCode::CONFLICT, "{}", blocked.1);
    let row: (i32, String, String) = sqlx::query_as(
        "select version, coalesce(seller_remark, ''), coalesce(buyer_message, '') from orders where tenant_id = $1 and order_id = $2",
    )
    .bind(tenant)
    .bind(order)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 8);
    assert_eq!(row.1, "web-edit");
    assert_eq!(row.2, "");
    eprintln!(
        "order-conflict-ok tenant={tenant} actor={actor} order={order} version={} seller={} buyer={}",
        row.0, row.1, row.2
    );
    Ok(())
}

#[tokio::test]
async fn two_clients_order_same_command_retry_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    sqlx::query(
        "create table if not exists idempotency_keys (
          tenant_id uuid not null,
          \"key\" text not null,
          route text not null,
          request_hash text not null,
          response_status int not null,
          response_body jsonb not null,
          created_at timestamptz not null default now(),
          unique (tenant_id, route, \"key\")
        )",
    )
    .execute(&pool)
    .await?;
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let order = Uuid::new_v4();
    let shop = Uuid::new_v4();
    eprintln!("order-retry tenant={tenant} actor={actor} order={order}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a03r-retry', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(format!("a03r-{actor}@example.test"))
    .bind(hash)
    .execute(&pool)
    .await?;
    let role = Uuid::new_v4();
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a03r-retry')")
        .bind(role)
        .bind(tenant)
        .execute(&pool)
        .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(tenant)
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.write')",
    )
    .bind(tenant)
    .bind(role)
    .execute(&pool)
    .await?;
    sqlx::query("insert into shops(tenant_id, shop_id, platform, platform_shop_id, shop_name) values ($1, $2, 'TAOBAO', $3, 'a03r')")
        .bind(tenant)
        .bind(shop)
        .bind(shop.to_string())
        .execute(&pool)
        .await?;
    sqlx::query("insert into orders(tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status, order_created_at, order_status, fulfillment_status, exception_status, exception_flags, inventory_shortage, version) values ($1, '2026-10-01', $2, 'TAOBAO', $3, $4, 'PAID', now(), 'PENDING_ALLOCATE', 'NOT_ALLOCATED', 'NONE', '[]', false, 7)")
        .bind(tenant)
        .bind(order)
        .bind(shop)
        .bind(order.to_string())
        .execute(&pool)
        .await?;
    sqlx::query("insert into order_index(tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at) values ($1, $2, '2026-10-01', 'TAOBAO', $3, $4, now())")
        .bind(tenant)
        .bind(order)
        .bind(order.to_string())
        .bind(shop)
        .execute(&pool)
        .await?;
    let jwt = Arc::new(JwtService::new("order-same-command-retry-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let email = format!("a03r-{actor}@example.test");
    let web = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a03r").await?;
    let desk = login_http(
        &app,
        tenant,
        &email,
        "boundary-a03k",
        "DESKTOP",
        "desk-a03r",
    )
    .await?;
    assert_eq!(web.0, StatusCode::OK, "{}", web.1);
    assert_eq!(desk.0, StatusCode::OK, "{}", desk.1);
    let web_token = web.1["access_token"].as_str().unwrap_or("");
    let desk_token = desk.1["access_token"].as_str().unwrap_or("");
    let command = "a03r-same-command";
    let first = order_edit_command(
        &app,
        web_token,
        tenant,
        order,
        7,
        "seller_remark",
        "same-edit",
        command,
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let first_version = first.1["order"]["version"].as_i64().unwrap_or(0);
    let replay = order_edit_command(
        &app,
        desk_token,
        tenant,
        order,
        7,
        "seller_remark",
        "same-edit",
        command,
    )
    .await?;
    assert_eq!(replay.0, StatusCode::OK, "{}", replay.1);
    assert_eq!(
        replay.1["order"]["version"].as_i64().unwrap_or(0),
        first_version
    );
    assert_eq!(
        replay.1["order"]["seller_remark"].as_str().unwrap_or(""),
        "same-edit"
    );
    let other = order_edit_command(
        &app,
        desk_token,
        tenant,
        order,
        7,
        "buyer_message",
        "other-edit",
        "a03r-other-command",
    )
    .await?;
    assert_eq!(other.0, StatusCode::CONFLICT, "{}", other.1);
    let row: (i32, String, String, i64) = sqlx::query_as(
        "select version, coalesce(seller_remark, ''), coalesce(buyer_message, ''),
            (select count(*) from idempotency_keys where tenant_id = $1 and \"key\" = $3)
         from orders where tenant_id = $1 and order_id = $2",
    )
    .bind(tenant)
    .bind(order)
    .bind(command)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 8);
    assert_eq!(row.1, "same-edit");
    assert_eq!(row.2, "");
    assert_eq!(row.3, 1);
    eprintln!(
        "order-retry-ok tenant={tenant} actor={actor} order={order} version={} seller={} buyer={} receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn export_job_cross_tenant_download_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    sqlx::query(
        "create table if not exists export_jobs (
          job_id uuid primary key,
          tenant_id uuid not null,
          job_type text not null,
          params_json jsonb not null default '{}'::jsonb,
          status text not null default 'PENDING',
          output_url text,
          progress int not null default 0,
          attempts int not null default 0,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )",
    )
    .execute(&pool)
    .await?;
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let job = Uuid::new_v4();
    let marker = format!("FILE-A04I-{job}");
    eprintln!("export-isolation tenant={owner} foreign={foreign} job={job}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a04i-owner"),
        (foreign, foreign_actor, "a04i-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'exports.read'), ($1, $2, 'exports.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into export_jobs(job_id, tenant_id, job_type, params_json, status, output_url, progress) values ($1, $2, 'ORDER_EXPORT', '{\"marker\":\"FILE-A04I\"}'::jsonb, 'DONE', $3, 100)",
    )
    .bind(job)
    .bind(owner)
    .bind(&marker)
    .execute(&pool)
    .await?;
    let export_dir = std::env::temp_dir().join(format!("erp-a04i-{owner}"));
    std::fs::create_dir_all(export_dir.join(owner.to_string()))?;
    std::fs::write(
        export_dir
            .join(owner.to_string())
            .join(format!("{job}.csv")),
        marker.as_bytes(),
    )?;
    let previous_export_dir = std::env::var("EXPORT_BASE_DIR").ok();
    // Safety: this process is the acceptance test process. The shipped download
    // path reads EXPORT_BASE_DIR once per request, and the test restores it.
    unsafe {
        std::env::set_var("EXPORT_BASE_DIR", &export_dir);
    }
    let jwt = Arc::new(JwtService::new("export-isolation-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a04i-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a04i",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a04i-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a04i",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"].as_str().unwrap_or("");
    let foreign_token = foreign_login.1["access_token"].as_str().unwrap_or("");
    let own = export_http(
        &app,
        owner_token,
        "GET",
        &format!("/exports/jobs/{job}"),
        None,
    )
    .await?;
    assert_eq!(own.0, StatusCode::OK, "{}", own.1);
    let own_body: Value = serde_json::from_str(&own.1).unwrap_or(Value::Null);
    assert_eq!(own_body["output_url"].as_str().unwrap_or(""), marker);
    let foreign_get = export_http(
        &app,
        foreign_token,
        "GET",
        &format!("/exports/jobs/{job}"),
        None,
    )
    .await?;
    assert_eq!(foreign_get.0, StatusCode::NOT_FOUND, "{}", foreign_get.1);
    assert!(!foreign_get.1.contains(&marker));
    let foreign_file = export_http(
        &app,
        foreign_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_ne!(foreign_file.0, StatusCode::OK);
    assert!(!foreign_file.1.contains(&marker));
    let own_file = export_http(
        &app,
        owner_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_eq!(own_file.0, StatusCode::OK, "{}", own_file.1);
    assert!(own_file.1.contains(&marker));
    if let Some(previous) = previous_export_dir {
        unsafe {
            std::env::set_var("EXPORT_BASE_DIR", previous);
        }
    } else {
        unsafe {
            std::env::remove_var("EXPORT_BASE_DIR");
        }
    }
    let resolved_export_dir = std::fs::canonicalize(&export_dir)?;
    let resolved_temp_dir = std::fs::canonicalize(std::env::temp_dir())?;
    assert!(resolved_export_dir.starts_with(&resolved_temp_dir));
    assert_eq!(resolved_export_dir.file_name(), export_dir.file_name());
    assert!(
        !std::fs::symlink_metadata(&export_dir)?
            .file_type()
            .is_symlink()
    );
    std::fs::remove_dir_all(&resolved_export_dir)?;
    let row: (String, String, i64) = sqlx::query_as(
        "select status, coalesce(output_url, ''),
            (select count(*) from export_jobs where tenant_id = $2 and job_id = $3)
         from export_jobs where tenant_id = $1 and job_id = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(job)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "DONE");
    assert_eq!(row.1, marker);
    assert_eq!(row.2, 0);
    eprintln!(
        "export-isolation-ok tenant={owner} foreign={foreign} actor={owner_actor} job={job} status={} foreign_jobs={}",
        row.0, row.2
    );
    Ok(())
}

#[tokio::test]
async fn lost_response_one_effect_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists edit_drafts (
          tenant_id uuid not null,
          draft_id uuid not null,
          user_id text not null,
          page_code text not null,
          object_id text not null,
          version_no bigint not null,
          body_text text not null,
          status text not null,
          service_address text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, draft_id),
          unique (tenant_id, user_id, page_code, object_id)
        )",
        "create table if not exists lost_response_lookups (
          tenant_id uuid not null,
          lookup_id uuid not null,
          request_id text not null,
          stored_status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, lookup_id),
          unique (tenant_id, request_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("lost-once tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u05h-owner"),
        (foreign, foreign_actor, "u05h-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("lost-once-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u05h-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u05h",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u05h-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u05h",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let draft = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/edit-drafts"),
        Some(
            serde_json::json!({
                "request_id": "u05h-command",
                "user_id": format!("user-{owner_actor}"),
                "page": "ORDERS",
                "object_id": "SO-U05H",
                "version": 2,
                "body": "qty=1",
                "service_address": "svc-u05h"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(draft.0, StatusCode::OK, "{}", draft.1);
    let draft_body: Value = serde_json::from_str(&draft.1).unwrap_or(Value::Null);
    let effect = draft_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!effect.is_empty(), "{}", draft.1);
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/lost-responses?command_request=u05h-command"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let looked = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/lost-responses"),
        Some(
            serde_json::json!({
                "request_id": "u05h-lookup",
                "command_request": "u05h-command"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(looked.0, StatusCode::OK, "{}", looked.1);
    let looked_body: Value = serde_json::from_str(&looked.1).unwrap_or(Value::Null);
    assert_eq!(looked_body["effect_id"].as_str().unwrap_or(""), "LOOKED_UP");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/lost-responses?command_request=u05h-command"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["status"].as_str().unwrap_or(""), "SUCCEEDED");
    assert_eq!(read_body["effect_id"].as_str().unwrap_or(""), effect);
    assert_eq!(read_body["effect_count"].as_i64().unwrap_or(0), 1);
    let foreign_lookup = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/lost-responses?command_request=u05h-command"),
        None,
    )
    .await?;
    assert_ne!(foreign_lookup.0, StatusCode::OK, "{}", foreign_lookup.1);
    assert!(!foreign_lookup.1.contains(&effect));
    let foreign_draft = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/edit-drafts?user_id=user-{owner_actor}&page=ORDERS&object_id=SO-U05H"),
        None,
    )
    .await?;
    assert_ne!(foreign_draft.0, StatusCode::OK, "{}", foreign_draft.1);
    assert!(!foreign_draft.1.contains("qty=1"));
    let owner_draft = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/edit-drafts?user_id=user-{owner_actor}&page=ORDERS&object_id=SO-U05H"),
        None,
    )
    .await?;
    assert_eq!(owner_draft.0, StatusCode::OK, "{}", owner_draft.1);
    let owner_draft_body: Value = serde_json::from_str(&owner_draft.1).unwrap_or(Value::Null);
    assert_eq!(owner_draft_body["body"].as_str().unwrap_or(""), "qty=1");
    assert_eq!(
        owner_draft_body["tenant_id"].as_str().unwrap_or(""),
        owner.to_string()
    );
    let row: (Option<String>, Option<i64>, Option<String>, i64) = sqlx::query_as(
        "select
            (select effect_id from command_receipts where tenant_id = $1 and request_id = 'u05h-command'),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'u05h-command'),
            (select body_text from edit_drafts where tenant_id = $1 and object_id = 'SO-U05H'),
            (select count(*)::bigint from edit_drafts where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some(effect.as_str()));
    assert_eq!(row.1, Some(1));
    assert_eq!(row.2.as_deref(), Some("qty=1"));
    assert_eq!(row.3, 0);
    eprintln!("lost-once-ok tenant={owner} foreign={foreign} actor={owner_actor} effect={effect}");
    Ok(())
}

#[tokio::test]
async fn paper_out_refuses_printed_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists device_print_jobs (
          tenant_id uuid not null,
          job_id uuid not null,
          device_id text not null,
          document_no text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, job_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let device = format!("printer-{owner}");
    let document = format!("SO-U07E-{owner}");
    eprintln!("paper-out tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u07e-owner"),
        (foreign, foreign_actor, "u07e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("paper-out-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u07e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u07e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u07e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u07e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/device-prints?device_id={device}&document_no={document}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let paper = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/device-prints"),
        Some(
            serde_json::json!({
                "request_id": "u07e-paper",
                "device_id": device,
                "document_no": document,
                "agent": true,
                "status": "PAPER_OUT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(paper.0, StatusCode::OK, "{}", paper.1);
    assert!(!paper.1.contains("PRINTED"));
    let offline = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/device-prints"),
        Some(
            serde_json::json!({
                "request_id": "u07e-offline",
                "device_id": device,
                "document_no": document,
                "agent": true,
                "status": "OFFLINE"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(offline.0, StatusCode::OK, "{}", offline.1);
    let queued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/device-prints"),
        Some(
            serde_json::json!({
                "request_id": "u07e-queued",
                "device_id": device,
                "document_no": document,
                "agent": true,
                "status": "QUEUED"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(queued.0, StatusCode::OK, "{}", queued.1);
    let queued_body: Value = serde_json::from_str(&queued.1).unwrap_or(Value::Null);
    assert_eq!(queued_body["effect_id"].as_str().unwrap_or(""), "QUEUED");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/device-prints?device_id={device}&document_no={document}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["status"].as_str().unwrap_or(""), "QUEUED");
    assert_eq!(read_body["printed"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/device-prints?device_id={device}&document_no={document}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&document));
    let row: (Option<String>, i64, i64, i64) = sqlx::query_as(
        "select
            (select status from device_print_jobs where tenant_id = $1 and document_no = $3),
            (select count(*)::bigint from device_print_jobs where tenant_id = $1 and status = 'PRINTED'),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id in ('u07e-paper','u07e-offline')),
            (select count(*)::bigint from device_print_jobs where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&document)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("QUEUED"));
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "paper-out-ok tenant={owner} foreign={foreign} actor={owner_actor} document={document}"
    );
    Ok(())
}

#[tokio::test]
async fn export_cell_parse_refuses_formula_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists file_cells (
          tenant_id uuid not null,
          cell_id uuid not null,
          file_no text not null,
          kind_code text not null,
          value_text text not null,
          encoding_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, cell_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let file_no = format!("FILE-U07F-{owner}");
    let safe_text = format!("sku-{owner}");
    let safe_qty = "1.5";
    eprintln!("export-cell tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u07f-owner"),
        (foreign, foreign_actor, "u07f-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("export-cell-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u07f-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u07f",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u07f-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u07f",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/file-cells?file_no={file_no}&kind=TEXT"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let formula = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/file-cells"),
        Some(
            serde_json::json!({
                "request_id": "u07f-formula",
                "file_no": file_no,
                "kind": "TEXT",
                "value": "=1+1"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(formula.0, StatusCode::OK, "{}", formula.1);
    assert!(!formula.1.contains("=1+1"));
    let plus = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/file-cells"),
        Some(
            serde_json::json!({
                "request_id": "u07f-plus",
                "file_no": file_no,
                "kind": "CODE",
                "value": "+cmd"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(plus.0, StatusCode::OK, "{}", plus.1);
    let at = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/file-cells"),
        Some(
            serde_json::json!({
                "request_id": "u07f-at",
                "file_no": file_no,
                "kind": "TEXT",
                "value": "@sum"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(at.0, StatusCode::OK, "{}", at.1);
    let text = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/file-cells"),
        Some(
            serde_json::json!({
                "request_id": "u07f-text",
                "file_no": file_no,
                "kind": "TEXT",
                "value": safe_text
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(text.0, StatusCode::OK, "{}", text.1);
    let text_body: Value = serde_json::from_str(&text.1).unwrap_or(Value::Null);
    assert_eq!(text_body["effect_id"].as_str().unwrap_or(""), "TEXT");
    let qty = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/file-cells"),
        Some(
            serde_json::json!({
                "request_id": "u07f-qty",
                "file_no": file_no,
                "kind": "QTY",
                "value": safe_qty
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(qty.0, StatusCode::OK, "{}", qty.1);
    let text_read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/file-cells?file_no={file_no}&kind=TEXT"),
        None,
    )
    .await?;
    assert_eq!(text_read.0, StatusCode::OK, "{}", text_read.1);
    let text_read_body: Value = serde_json::from_str(&text_read.1).unwrap_or(Value::Null);
    assert_eq!(
        text_read_body["parsed_value"].as_str().unwrap_or(""),
        safe_text
    );
    assert_eq!(text_read_body["encoding"].as_str().unwrap_or(""), "TEXT");
    assert_eq!(text_read_body["formula"].as_bool().unwrap_or(true), false);
    let qty_read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/file-cells?file_no={file_no}&kind=QTY"),
        None,
    )
    .await?;
    assert_eq!(qty_read.0, StatusCode::OK, "{}", qty_read.1);
    let qty_read_body: Value = serde_json::from_str(&qty_read.1).unwrap_or(Value::Null);
    assert_eq!(
        qty_read_body["parsed_value"].as_str().unwrap_or(""),
        safe_qty
    );
    assert_eq!(qty_read_body["encoding"].as_str().unwrap_or(""), "QTY");
    assert_eq!(qty_read_body["formula"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/file-cells?file_no={file_no}&kind=TEXT"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&safe_text));
    let row: (Option<String>, Option<String>, i64, i64, i64) = sqlx::query_as(
        "select
            (select value_text from file_cells where tenant_id = $1 and file_no = $3 and kind_code = 'TEXT'),
            (select value_text from file_cells where tenant_id = $1 and file_no = $3 and kind_code = 'QTY'),
            (select count(*)::bigint from file_cells where tenant_id = $1 and (value_text like '=%' or value_text like '+%' or value_text like '@%')),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id in ('u07f-formula','u07f-plus','u07f-at')),
            (select count(*)::bigint from file_cells where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&file_no)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some(safe_text.as_str()));
    assert_eq!(row.1.as_deref(), Some(safe_qty));
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 0);
    eprintln!("export-cell-ok tenant={owner} foreign={foreign} actor={owner_actor} file={file_no}");
    Ok(())
}

#[tokio::test]
async fn page_total_is_not_full_query_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists report_aggregates (
          tenant_id uuid not null,
          aggregate_id uuid not null,
          query_no text not null,
          scope_code text not null,
          dimension_code text not null,
          total_qty bigint not null,
          formula_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, aggregate_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let query_no = format!("Q-U08F-{owner}");
    eprintln!("page-total tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u08f-owner"),
        (foreign, foreign_actor, "u08f-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("page-total-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u08f-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u08f",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u08f-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u08f",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/report-aggregates?query_no={query_no}&dimension=warehouse"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let script = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/report-aggregates"),
        Some(
            serde_json::json!({
                "request_id": "u08f-script",
                "query_no": query_no,
                "scope": "FULL",
                "dimension": "warehouse",
                "total": 1000000,
                "formula": "script:qty"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(script.0, StatusCode::OK, "{}", script.1);
    let page = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/report-aggregates"),
        Some(
            serde_json::json!({
                "request_id": "u08f-page",
                "query_no": query_no,
                "scope": "PAGE",
                "dimension": "warehouse",
                "total": 100,
                "formula": "qty"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(page.0, StatusCode::OK, "{}", page.1);
    let page_body: Value = serde_json::from_str(&page.1).unwrap_or(Value::Null);
    assert_eq!(page_body["effect_id"].as_str().unwrap_or(""), "PAGE");
    let full = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/report-aggregates"),
        Some(
            serde_json::json!({
                "request_id": "u08f-full",
                "query_no": query_no,
                "scope": "FULL",
                "dimension": "warehouse",
                "total": 1000000,
                "formula": "qty"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(full.0, StatusCode::OK, "{}", full.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/report-aggregates?query_no={query_no}&dimension=warehouse"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["page_total"].as_i64().unwrap_or(0), 100);
    assert_eq!(read_body["full_total"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(read_body["loaded"].as_i64().unwrap_or(0), 1);
    assert_eq!(read_body["matched"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(
        read_body["page_stands_for_full"].as_bool().unwrap_or(true),
        false
    );
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/report-aggregates?query_no={query_no}&dimension=warehouse"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("1000000"));
    let row: (Option<i64>, Option<i64>, i64, i64) = sqlx::query_as(
        "select
            (select total_qty from report_aggregates where tenant_id = $1 and query_no = $3 and scope_code = 'PAGE'),
            (select total_qty from report_aggregates where tenant_id = $1 and query_no = $3 and scope_code = 'FULL'),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'u08f-script'),
            (select count(*)::bigint from report_aggregates where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&query_no)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, Some(100));
    assert_eq!(row.1, Some(1_000_000));
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "page-total-ok tenant={owner} foreign={foreign} actor={owner_actor} query={query_no}"
    );
    Ok(())
}

#[tokio::test]
async fn formula_fault_locates_field_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let query_no = format!("Q-U08G-{owner}");
    eprintln!("formula-fault tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u08g-owner"),
        (foreign, foreign_actor, "u08g-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("formula-fault-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u08g-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u08g",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u08g-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u08g",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/formula-faults?query_no={query_no}&field=amount"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let legal = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/formula-faults"),
        Some(
            serde_json::json!({
                "request_id": "u08g-legal",
                "query_no": query_no,
                "field": "amount",
                "formula": "qty"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(legal.0, StatusCode::OK, "{}", legal.1);
    let illegal = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/formula-faults"),
        Some(
            serde_json::json!({
                "request_id": "u08g-illegal",
                "query_no": query_no,
                "field": "amount",
                "formula": "script:qty"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(illegal.0, StatusCode::OK, "{}", illegal.1);
    let illegal_body: Value = serde_json::from_str(&illegal.1).unwrap_or(Value::Null);
    assert_eq!(illegal_body["effect_id"].as_str().unwrap_or(""), "amount");
    let cyclic = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/formula-faults"),
        Some(
            serde_json::json!({
                "request_id": "u08g-cyclic",
                "query_no": query_no,
                "field": "margin",
                "formula": "margin+qty"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(cyclic.0, StatusCode::OK, "{}", cyclic.1);
    let illegal_read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/formula-faults?query_no={query_no}&field=amount"),
        None,
    )
    .await?;
    assert_eq!(illegal_read.0, StatusCode::OK, "{}", illegal_read.1);
    let illegal_read_body: Value = serde_json::from_str(&illegal_read.1).unwrap_or(Value::Null);
    assert_eq!(illegal_read_body["field"].as_str().unwrap_or(""), "amount");
    assert_eq!(
        illegal_read_body["decision"].as_str().unwrap_or(""),
        "ILLEGAL"
    );
    assert_eq!(
        illegal_read_body["executed"].as_bool().unwrap_or(true),
        false
    );
    let cyclic_read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/formula-faults?query_no={query_no}&field=margin"),
        None,
    )
    .await?;
    assert_eq!(cyclic_read.0, StatusCode::OK, "{}", cyclic_read.1);
    let cyclic_read_body: Value = serde_json::from_str(&cyclic_read.1).unwrap_or(Value::Null);
    assert_eq!(cyclic_read_body["field"].as_str().unwrap_or(""), "margin");
    assert_eq!(
        cyclic_read_body["decision"].as_str().unwrap_or(""),
        "CYCLIC"
    );
    assert_eq!(
        cyclic_read_body["executed"].as_bool().unwrap_or(true),
        false
    );
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/formula-faults?query_no={query_no}&field=amount"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("script:qty"));
    let row: (Option<String>, Option<String>, i64, i64) = sqlx::query_as(
        "select
            (select decision from formula_faults where tenant_id = $1 and query_no = $3 and field_code = 'amount'),
            (select decision from formula_faults where tenant_id = $1 and query_no = $3 and field_code = 'margin'),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'u08g-legal'),
            (select count(*)::bigint from formula_faults where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&query_no)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("ILLEGAL"));
    assert_eq!(row.1.as_deref(), Some("CYCLIC"));
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "formula-fault-ok tenant={owner} foreign={foreign} actor={owner_actor} query={query_no}"
    );
    Ok(())
}

#[tokio::test]
async fn fold_keeps_total_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists report_tree_nodes (
          tenant_id uuid not null,
          node_id uuid not null,
          query_no text not null,
          level_no integer not null,
          node_code text not null,
          parent_code text not null,
          total_qty bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, node_id)
        )",
        "alter table report_tree_nodes add column if not exists total_qty bigint not null default 0",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let query_no = format!("Q-U08H-{owner}");
    let node = format!("GRP-{owner}");
    eprintln!("fold-total tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u08h-owner"),
        (foreign, foreign_actor, "u08h-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("fold-total-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u08h-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u08h",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u08h-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u08h",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/report-tree-nodes?query_no={query_no}&node={node}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let changed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/report-tree-nodes"),
        Some(
            serde_json::json!({
                "request_id": "u08h-changed",
                "query_no": query_no,
                "level": 2,
                "node": node,
                "parent": "ROOT",
                "total_before": 480,
                "total_after": 100
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(changed.0, StatusCode::OK, "{}", changed.1);
    let kept = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/report-tree-nodes"),
        Some(
            serde_json::json!({
                "request_id": "u08h-fold",
                "query_no": query_no,
                "level": 2,
                "node": node,
                "parent": "ROOT",
                "total_before": 480,
                "total_after": 480
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(kept.0, StatusCode::OK, "{}", kept.1);
    let kept_body: Value = serde_json::from_str(&kept.1).unwrap_or(Value::Null);
    assert_eq!(kept_body["effect_id"].as_str().unwrap_or(""), node);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/report-tree-nodes?query_no={query_no}&node={node}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["level"].as_i64().unwrap_or(0), 2);
    assert_eq!(read_body["node"].as_str().unwrap_or(""), node);
    assert_eq!(read_body["total"].as_i64().unwrap_or(0), 480);
    assert_eq!(
        read_body["folded_changed_total"].as_bool().unwrap_or(true),
        false
    );
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/report-tree-nodes?query_no={query_no}&node={node}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&node));
    let row: (Option<i64>, i64, i64) = sqlx::query_as(
        "select
            (select total_qty from report_tree_nodes where tenant_id = $1 and query_no = $3 and node_code = $4),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'u08h-changed'),
            (select count(*)::bigint from report_tree_nodes where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&query_no)
    .bind(&node)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, Some(480));
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 0);
    eprintln!("fold-total-ok tenant={owner} foreign={foreign} actor={owner_actor} node={node}");
    Ok(())
}

#[tokio::test]
async fn referenced_item_stays_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_items (
          tenant_id uuid not null,
          item_id uuid not null,
          item_code text not null,
          name text not null,
          spec text not null default '',
          category text not null default '',
          barcode text not null default '',
          buy boolean not null,
          make boolean not null,
          sell boolean not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, item_id),
          unique (tenant_id, item_code)
        )",
        "create table if not exists shared_item_refs (
          tenant_id uuid not null,
          item_id uuid not null,
          document_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, item_id, document_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item_code = format!("ITEM-D01E-{owner}");
    eprintln!("referenced-item tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d01e-owner"),
        (foreign, foreign_actor, "d01e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("referenced-item-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d01e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d01e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d01e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d01e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-items?item_code={item_code}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-items"),
        Some(
            serde_json::json!({
                "request_id": "d01e-item",
                "item_code": item_code,
                "name": "bolt",
                "spec": "M8",
                "category": "fastener",
                "barcode": format!("BC-{owner}"),
                "buy": true,
                "make": true,
                "sell": true
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let item_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!item_id.is_empty());
    let document = format!("SO-D01E-{owner}");
    let referenced = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-item-refs"),
        Some(
            serde_json::json!({
                "request_id": "d01e-ref",
                "item_id": item_id,
                "document_id": document
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(referenced.0, StatusCode::OK, "{}", referenced.1);
    let deleted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-item-retirements"),
        Some(
            serde_json::json!({
                "request_id": "d01e-delete",
                "item_id": item_id,
                "requested": "DELETE"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(deleted.0, StatusCode::OK, "{}", deleted.1);
    assert!(!deleted.1.contains("DELETED"));
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-items?item_code={item_code}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["item_id"].as_str().unwrap_or(""), item_id);
    assert_eq!(read_body["status"].as_str().unwrap_or(""), "ACTIVE");
    assert_eq!(read_body["buy"].as_bool().unwrap_or(false), true);
    assert_eq!(read_body["make"].as_bool().unwrap_or(false), true);
    assert_eq!(read_body["sell"].as_bool().unwrap_or(false), true);
    assert_eq!(read_body["refs"].as_i64().unwrap_or(0), 1);
    assert_eq!(read_body["hard_deleted"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-items?item_code={item_code}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&item_code));
    let row: (Option<String>, i64, i64, i64) = sqlx::query_as(
        "select
            (select status from shared_items where tenant_id = $1 and item_code = $3),
            (select count(*)::bigint from shared_item_refs where tenant_id = $1 and document_id = $4),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'd01e-delete'),
            (select count(*)::bigint from shared_items where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&item_code)
    .bind(&document)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("ACTIVE"));
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "referenced-item-ok tenant={owner} foreign={foreign} actor={owner_actor} item={item_id}"
    );
    Ok(())
}

#[tokio::test]
async fn unit_qty_read_back_keeps_scale_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists unit_qty_samples (
          tenant_id uuid not null,
          sample_id uuid not null,
          unit_code text not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, sample_id)
        )",
        "create table if not exists legacy_unit_reads (
          tenant_id uuid not null,
          read_id uuid not null,
          legacy_units bigint not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, read_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let unit = format!("KG-{owner}");
    eprintln!("unit-qty tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d02d-owner"),
        (foreign, foreign_actor, "d02d-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("unit-qty-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d02d-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d02d",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d02d-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d02d",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/unit-qtys?unit_code={unit}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let scale = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/unit-qtys"),
        Some(
            serde_json::json!({
                "request_id": "d02d-scale",
                "unit_code": unit,
                "qty_text": "0.1250001",
                "display_text": "0.1250001"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(scale.0, StatusCode::OK, "{}", scale.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/unit-qtys"),
        Some(
            serde_json::json!({
                "request_id": "d02d-qty",
                "unit_code": unit,
                "qty_text": "0.125",
                "display_text": "0.125"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    assert_eq!(saved_body["effect_id"].as_str().unwrap_or(""), "125000");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/unit-qtys?unit_code={unit}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["qty_micros"].as_i64().unwrap_or(0), 125_000);
    assert_eq!(read_body["qty_text"].as_str().unwrap_or(""), "0.125000");
    assert_eq!(
        read_body["collapsed_to_zero"].as_bool().unwrap_or(true),
        false
    );
    assert_ne!(read_body["qty_text"].as_str().unwrap_or(""), "0");
    let missing_legacy = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/legacy-unit-reads?legacy_units=1"),
        None,
    )
    .await?;
    assert_ne!(missing_legacy.0, StatusCode::OK, "{}", missing_legacy.1);
    let legacy = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/legacy-unit-reads"),
        Some(
            serde_json::json!({
                "request_id": "d02d-legacy",
                "legacy_units": 1,
                "display_text": "1"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(legacy.0, StatusCode::OK, "{}", legacy.1);
    let legacy_body: Value = serde_json::from_str(&legacy.1).unwrap_or(Value::Null);
    assert_eq!(legacy_body["effect_id"].as_str().unwrap_or(""), "1000000");
    let legacy_read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/legacy-unit-reads?legacy_units=1"),
        None,
    )
    .await?;
    assert_eq!(legacy_read.0, StatusCode::OK, "{}", legacy_read.1);
    let legacy_read_body: Value = serde_json::from_str(&legacy_read.1).unwrap_or(Value::Null);
    assert_eq!(
        legacy_read_body["qty_micros"].as_i64().unwrap_or(0),
        1_000_000
    );
    assert_eq!(
        legacy_read_body["qty_text"].as_str().unwrap_or(""),
        "1.000000"
    );
    assert_ne!(legacy_read_body["qty_text"].as_str().unwrap_or(""), "0.001");
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/unit-qtys?unit_code={unit}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("0.125000"));
    let row: (Option<i64>, Option<i64>, i64, i64) = sqlx::query_as(
        "select
            (select qty_micros from unit_qty_samples where tenant_id = $1 and unit_code = $3),
            (select qty_micros from legacy_unit_reads where tenant_id = $1 and legacy_units = 1),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'd02d-scale'),
            (select count(*)::bigint from unit_qty_samples where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&unit)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, Some(125_000));
    assert_eq!(row.1, Some(1_000_000));
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!("unit-qty-ok tenant={owner} foreign={foreign} actor={owner_actor} unit={unit}");
    Ok(())
}

#[tokio::test]
async fn zero_ratio_refuses_conversion_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists unit_conversions (
          tenant_id uuid not null,
          conversion_id uuid not null,
          base_unit text not null,
          aux_unit text not null,
          ratio_micros bigint not null,
          rounding_code text not null,
          aux_micros bigint not null,
          base_micros bigint not null,
          back_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, conversion_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let base = format!("G-{owner}");
    let aux = format!("KG-{owner}");
    eprintln!("zero-ratio tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d02e-owner"),
        (foreign, foreign_actor, "d02e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("zero-ratio-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d02e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d02e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d02e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d02e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/unit-conversions?base_unit={base}&aux_unit={aux}&rounding_code=HALF_UP"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let zero = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/unit-conversions"),
        Some(
            serde_json::json!({
                "request_id": "d02e-zero",
                "base_unit": base,
                "aux_unit": aux,
                "ratio_text": "0",
                "rounding_code": "HALF_UP",
                "aux_text": "0.125",
                "display_text": "0.125"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(zero.0, StatusCode::OK, "{}", zero.1);
    let overflow = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/unit-conversions"),
        Some(
            serde_json::json!({
                "request_id": "d02e-over",
                "base_unit": base,
                "aux_unit": aux,
                "ratio_text": "1000000000000",
                "rounding_code": "HALF_UP",
                "aux_text": "1000000000000",
                "display_text": "1000000000000"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(overflow.0, StatusCode::OK, "{}", overflow.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/unit-conversions"),
        Some(
            serde_json::json!({
                "request_id": "d02e-ok",
                "base_unit": base,
                "aux_unit": aux,
                "ratio_text": "1000",
                "rounding_code": "HALF_UP",
                "aux_text": "0.125",
                "display_text": "0.125"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/unit-conversions?base_unit={base}&aux_unit={aux}&rounding_code=HALF_UP"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(
        read_body["ratio_micros"].as_i64().unwrap_or(0),
        1_000_000_000
    );
    assert_eq!(read_body["aux_micros"].as_i64().unwrap_or(0), 125_000);
    assert!(read_body["base_micros"].as_i64().unwrap_or(0) > 0);
    assert_eq!(
        read_body["zero_ratio_stored"].as_bool().unwrap_or(true),
        false
    );
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/unit-conversions?base_unit={base}&aux_unit={aux}&rounding_code=HALF_UP"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    let row: (Option<i64>, i64, i64) = sqlx::query_as(
        "select
            (select ratio_micros from unit_conversions where tenant_id = $1 and base_unit = $3 and aux_unit = $4),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id in ('d02e-zero','d02e-over')),
            (select count(*)::bigint from unit_conversions where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&base)
    .bind(&aux)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, Some(1_000_000_000));
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 0);
    eprintln!("zero-ratio-ok tenant={owner} foreign={foreign} actor={owner_actor} base={base}");
    Ok(())
}

#[tokio::test]
async fn balance_equals_opening_plus_ledger_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_quality_balances (
          tenant_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          on_hand bigint not null,
          reserved_qty bigint not null,
          unavailable_qty bigint not null,
          available_qty bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, warehouse_id, location_id, sku_id, batch_code, quality_status)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let bad_sku = Uuid::new_v4();
    eprintln!(
        "balance-check tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} bad_sku={bad_sku}"
    );
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i01e-owner"),
        (foreign, foreign_actor, "i01e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i01e")
        .execute(&pool)
        .await?;
    for (item, code) in [(sku, "GOOD"), (bad_sku, "BAD")] {
        sqlx::query(
            "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
        )
        .bind(owner)
        .bind(item)
        .bind(product)
        .bind(format!("SKU-{code}-{item}"))
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i01e")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let batch = format!("B-{owner}");
    let bad_batch = format!("BAD-{owner}");
    let jwt = Arc::new(JwtService::new("balance-check-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i01e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i01e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i01e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i01e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/quality-rebuilds?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    for (request_id, item, item_batch, qty) in [
        ("opening:i01e-good", sku, &batch, 1_000_000_i64),
        ("i01e-move", sku, &batch, 250_000_i64),
        ("opening:i01e-bad", bad_sku, &bad_batch, 1_000_000_i64),
    ] {
        let posted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-ledgers"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": item,
                    "batch_code": item_batch,
                    "quality_status": "GOOD",
                    "bucket": "ON_HAND",
                    "delta_micros": qty
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    }
    let voided = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "void:i01e",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": bad_sku,
                "batch_code": bad_batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(voided.0, StatusCode::OK, "{}", voided.1);
    let matched = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/quality-rebuilds"),
        Some(
            serde_json::json!({
                "request_id": "i01e-match",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(matched.0, StatusCode::OK, "{}", matched.1);
    let mismatch = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/quality-rebuilds"),
        Some(
            serde_json::json!({
                "request_id": "i01e-mismatch",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": bad_sku,
                "batch_code": bad_batch,
                "quality_status": "GOOD"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(mismatch.0, StatusCode::CONFLICT, "{}", mismatch.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/quality-rebuilds?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["opening"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["ledger"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["stored"].as_i64().unwrap_or(0), 1_250_000);
    assert_eq!(
        body["opening"].as_i64().unwrap_or(0) + body["ledger"].as_i64().unwrap_or(0),
        body["stored"].as_i64().unwrap_or(0)
    );
    assert_eq!(body["status"].as_str().unwrap_or(""), "MATCHED");
    let bad_read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/quality-rebuilds?warehouse_id={warehouse}&sku_id={bad_sku}&batch_code={bad_batch}"),
        None,
    )
    .await?;
    assert_eq!(bad_read.0, StatusCode::OK, "{}", bad_read.1);
    let bad_body: Value = serde_json::from_str(&bad_read.1).unwrap_or(Value::Null);
    assert_eq!(bad_body["opening"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(bad_body["ledger"].as_i64().unwrap_or(-1), 0);
    assert_eq!(bad_body["stored"].as_i64().unwrap_or(0), 1_250_000);
    assert_ne!(
        bad_body["stored"].as_i64().unwrap_or(0),
        bad_body["opening"].as_i64().unwrap_or(0) + bad_body["ledger"].as_i64().unwrap_or(0)
    );
    assert_eq!(bad_body["status"].as_str().unwrap_or(""), "MISMATCH");
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/quality-rebuilds?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&sku.to_string()));
    let row: (String, i64, String, i64, i64, i64) = sqlx::query_as(
        "select
            (select status from shared_balance_checks where tenant_id = $1 and sku_id = $3),
            (select count(*) from shared_quality_balances where tenant_id = $1 and sku_id = $4),
            (select status from shared_balance_checks where tenant_id = $1 and sku_id = $4),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i01e-mismatch'),
            (select opening_qty + ledger_qty from shared_balance_checks where tenant_id = $1 and sku_id = $3),
            (select count(*) from shared_balance_checks where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(sku)
    .bind(bad_sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "MATCHED");
    assert_eq!(row.1, 0);
    assert_eq!(row.2, "MISMATCH");
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 1_250_000);
    assert_eq!(row.5, 0);
    eprintln!(
        "balance-check-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} bad_sku={bad_sku} matched={} bad_balances={} mismatch={} mismatch_receipts={} stored={} foreign_checks={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn sale_during_count_keeps_sheet_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_count_sheets (
          tenant_id uuid not null,
          count_id uuid not null,
          cutoff_at text not null,
          counted_qty bigint not null,
          adjustment_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, count_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("count-sale tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i03d-owner"),
        (foreign, foreign_actor, "i03d-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i03d")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i03d")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("count-sale-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i03d-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i03d",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i03d-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i03d",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/count-sheets?count_id={}", Uuid::nil()),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let stocked = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i03d-stock",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 10_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stocked.0, StatusCode::OK, "{}", stocked.1);
    // Snapshot after the actual stock commit, using the database clock rather
    // than a calendar literal that eventually falls before this fixture.
    let cutoff: String = sqlx::query_scalar("select clock_timestamp()::text")
        .fetch_one(&independent)
        .await?;
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/count-sheets"),
        Some(
            serde_json::json!({
                "request_id": "i03d-open",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "cutoff_at": cutoff
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let count_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!count_id.is_empty(), "{}", opened.1);
    let sold = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/count-sales"),
        Some(
            serde_json::json!({
                "request_id": "i03d-sale",
                "count_id": count_id,
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(sold.0, StatusCode::OK, "{}", sold.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/count-sheets?count_id={count_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["counted"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["sale_rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["sold"].as_i64().unwrap_or(0), 1_000_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/count-sheets?count_id={count_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&count_id));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select counted_qty from shared_count_sheets where tenant_id = $1 and count_id = $3),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and source_line = $4 and delta_micros < 0),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and bucket = 'ON_HAND'),
            (select count(*) from shared_count_sheets where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&count_id)?)
    .bind(&count_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 10_000_000);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 9_000_000);
    assert_eq!(row.3, 0);
    eprintln!(
        "count-sale-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} count={count_id} counted={} sale_rows={} on_hand={} foreign_sheets={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn repeat_receive_six_refused_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_transfer_lines (
          tenant_id uuid not null,
          line_id uuid not null,
          transfer_id uuid not null,
          kind text not null,
          delta_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, line_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let from_warehouse = Uuid::new_v4();
    let to_warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("repeat-receive tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i03c-owner"),
        (foreign, foreign_actor, "i03c-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i03c")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    for (warehouse, name) in [(from_warehouse, "i03c-from"), (to_warehouse, "i03c-to")] {
        sqlx::query(
            "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
        )
        .bind(owner)
        .bind(warehouse)
        .bind(name)
        .bind(warehouse.to_string())
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("repeat-receive-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i03c-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i03c",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i03c-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i03c",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/transfer-receives?transfer_id={}",
            Uuid::nil()
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let stocked = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i03c-stock",
                "warehouse_id": from_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 10_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stocked.0, StatusCode::OK, "{}", stocked.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/transfer-issues"),
        Some(
            serde_json::json!({
                "request_id": "i03c-issue",
                "from_warehouse_id": from_warehouse,
                "to_warehouse_id": to_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 10_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let issued_body: Value = serde_json::from_str(&issued.1).unwrap_or(Value::Null);
    let transfer_id = issued_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!transfer_id.is_empty(), "{}", issued.1);
    let first = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/transfer-receives"),
        Some(
            serde_json::json!({
                "request_id": "i03c-first",
                "transfer_id": transfer_id,
                "to_warehouse_id": to_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let second = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/transfer-receives"),
        Some(
            serde_json::json!({
                "request_id": "i03c-second",
                "transfer_id": transfer_id,
                "to_warehouse_id": to_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(second.0, StatusCode::CONFLICT, "{}", second.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/transfer-receives?transfer_id={transfer_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["issued"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["received"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["refused"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["in_transit"].as_i64().unwrap_or(0), 4_000_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/transfer-receives?transfer_id={transfer_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&transfer_id));
    let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(delta_micros) filter (where kind = 'RECEIVE'), 0)::bigint from shared_transfer_lines where tenant_id = $1 and transfer_id = $3),
            (select count(*) from shared_transfer_lines where tenant_id = $1 and transfer_id = $3 and kind = 'RECEIVE'),
            (select requested_micros from shared_transfer_refusals where tenant_id = $1 and transfer_id = $3),
            (select coalesce(sum(delta_micros) filter (where bucket = 'IN_TRANSIT'), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and source_line = $4),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i03c-second')",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&transfer_id)?)
    .bind(&transfer_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 6_000_000);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 6_000_000);
    assert_eq!(row.3, 4_000_000);
    assert_eq!(row.4, 0);
    eprintln!(
        "repeat-receive-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} transfer={transfer_id} received={} receive_rows={} refused={} in_transit={} second_receipts={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn loss_one_leaves_three_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_transfer_lines (
          tenant_id uuid not null,
          line_id uuid not null,
          transfer_id uuid not null,
          kind text not null,
          delta_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, line_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let from_warehouse = Uuid::new_v4();
    let to_warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("transfer-loss tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i03b-owner"),
        (foreign, foreign_actor, "i03b-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i03b")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    for (warehouse, name) in [(from_warehouse, "i03b-from"), (to_warehouse, "i03b-to")] {
        sqlx::query(
            "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
        )
        .bind(owner)
        .bind(warehouse)
        .bind(name)
        .bind(warehouse.to_string())
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("transfer-loss-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i03b-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i03b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i03b-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i03b",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/transfer-losses?transfer_id={}",
            Uuid::nil()
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let stocked = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i03b-stock",
                "warehouse_id": from_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 10_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stocked.0, StatusCode::OK, "{}", stocked.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/transfer-issues"),
        Some(
            serde_json::json!({
                "request_id": "i03b-issue",
                "from_warehouse_id": from_warehouse,
                "to_warehouse_id": to_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 10_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let issued_body: Value = serde_json::from_str(&issued.1).unwrap_or(Value::Null);
    let transfer_id = issued_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!transfer_id.is_empty(), "{}", issued.1);
    let partial = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/transfer-receives"),
        Some(
            serde_json::json!({
                "request_id": "i03b-partial",
                "transfer_id": transfer_id,
                "to_warehouse_id": to_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(partial.0, StatusCode::OK, "{}", partial.1);
    let lost = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/transfer-losses"),
        Some(
            serde_json::json!({
                "request_id": "i03b-loss",
                "transfer_id": transfer_id,
                "to_warehouse_id": to_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(lost.0, StatusCode::OK, "{}", lost.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/transfer-losses?transfer_id={transfer_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["issued"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["received"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["loss"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["in_transit"].as_i64().unwrap_or(0), 3_000_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/transfer-losses?transfer_id={transfer_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&transfer_id));
    let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(delta_micros) filter (where kind = 'ISSUE'), 0)::bigint from shared_transfer_lines where tenant_id = $1 and transfer_id = $3),
            (select coalesce(sum(delta_micros) filter (where kind = 'RECEIVE'), 0)::bigint from shared_transfer_lines where tenant_id = $1 and transfer_id = $3),
            (select coalesce(sum(delta_micros) filter (where kind = 'LOSS'), 0)::bigint from shared_transfer_lines where tenant_id = $1 and transfer_id = $3),
            (select coalesce(sum(delta_micros) filter (where bucket = 'IN_TRANSIT'), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and source_line = $4),
            (select count(*) from shared_transfer_lines where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&transfer_id)?)
    .bind(&transfer_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 10_000_000);
    assert_eq!(row.1, 6_000_000);
    assert_eq!(row.2, 1_000_000);
    assert_eq!(row.3, 3_000_000);
    assert_eq!(row.4, 0);
    eprintln!(
        "transfer-loss-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} transfer={transfer_id} issued={} received={} loss={} in_transit={} foreign_lines={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn issue_ten_receive_six_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_transfer_lines (
          tenant_id uuid not null,
          line_id uuid not null,
          transfer_id uuid not null,
          kind text not null,
          delta_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, line_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let from_warehouse = Uuid::new_v4();
    let to_warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("partial-transfer tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i03a-owner"),
        (foreign, foreign_actor, "i03a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i03a")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    for (warehouse, name) in [(from_warehouse, "i03a-from"), (to_warehouse, "i03a-to")] {
        sqlx::query(
            "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
        )
        .bind(owner)
        .bind(warehouse)
        .bind(name)
        .bind(warehouse.to_string())
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("partial-transfer-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i03a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i03a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i03a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i03a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/transfer-issues?transfer_id={}",
            Uuid::nil()
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let received_stock = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i03a-receive",
                "warehouse_id": from_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 10_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received_stock.0, StatusCode::OK, "{}", received_stock.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/transfer-issues"),
        Some(
            serde_json::json!({
                "request_id": "i03a-issue",
                "from_warehouse_id": from_warehouse,
                "to_warehouse_id": to_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 10_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let issued_body: Value = serde_json::from_str(&issued.1).unwrap_or(Value::Null);
    let transfer_id = issued_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!transfer_id.is_empty(), "{}", issued.1);
    let partial = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/transfer-receives"),
        Some(
            serde_json::json!({
                "request_id": "i03a-partial",
                "transfer_id": transfer_id,
                "to_warehouse_id": to_warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(partial.0, StatusCode::OK, "{}", partial.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/transfer-issues?transfer_id={transfer_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["issued"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["received"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["in_transit"].as_i64().unwrap_or(0), 4_000_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/transfer-issues?transfer_id={transfer_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&transfer_id));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(delta_micros) filter (where kind = 'ISSUE'), 0)::bigint from shared_transfer_lines where tenant_id = $1 and transfer_id = $3),
            (select coalesce(sum(delta_micros) filter (where kind = 'RECEIVE'), 0)::bigint from shared_transfer_lines where tenant_id = $1 and transfer_id = $3),
            (select coalesce(sum(delta_micros) filter (where bucket = 'IN_TRANSIT'), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and source_line = $4),
            (select count(*) from shared_transfer_lines where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&transfer_id)?)
    .bind(&transfer_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 10_000_000);
    assert_eq!(row.1, 6_000_000);
    assert_eq!(row.2, 4_000_000);
    assert_eq!(row.3, 0);
    eprintln!(
        "partial-transfer-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} transfer={transfer_id} issued={} received={} in_transit={} foreign_lines={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn repeat_scan_adds_no_qty_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_scans (
          tenant_id uuid not null,
          scan_id uuid not null,
          code text not null,
          kind text not null,
          status text not null,
          actor_user_id text not null,
          qty_micros bigint not null default 0,
          created_at timestamptz not null default now(),
          primary key (tenant_id, scan_id)
        )",
        "alter table shared_scans add column if not exists qty_micros bigint not null default 0",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("repeat-scan tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i06a-owner"),
        (foreign, foreign_actor, "i06a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("repeat-scan-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i06a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i06a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i06a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i06a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let code = format!("ITEM-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/scans?code={code}&kind=ITEM"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let first = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/scans"),
        Some(
            serde_json::json!({
                "request_id": "i06a-scan",
                "code": code,
                "kind": "ITEM",
                "actor_user_id": owner_actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let first_body: Value = serde_json::from_str(&first.1).unwrap_or(Value::Null);
    let scan_id = first_body["effect_id"].as_str().unwrap_or("").to_string();
    let repeat = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/scans"),
        Some(
            serde_json::json!({
                "request_id": "i06a-repeat",
                "code": code,
                "kind": "ITEM",
                "actor_user_id": owner_actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(repeat.0, StatusCode::OK, "{}", repeat.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/scans?code={code}&kind=ITEM"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["scan_id"].as_str().unwrap_or(""), scan_id);
    assert_eq!(body["qty_micros"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["verified_rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["document_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/scans?code={code}&kind=ITEM"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&scan_id));
    let row: (String, i64, i64, i64, i64) = sqlx::query_as(
        "select scan_id::text, qty_micros,
            (select count(*) from shared_scans where tenant_id = $1 and status = 'VERIFIED'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i06a-repeat'),
            (select count(*) from shared_scans where tenant_id = $2)
         from shared_scans where tenant_id = $1 and status = 'VERIFIED'",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, scan_id);
    assert_eq!(row.1, 1_000_000);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 0);
    eprintln!(
        "repeat-scan-ok tenant={owner} foreign={foreign} actor={owner_actor} code={code} scan={} qty={} rows={} repeat_receipts={} foreign_scans={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn wrong_batch_located_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_scans (
          tenant_id uuid not null,
          scan_id uuid not null,
          code text not null,
          kind text not null,
          status text not null,
          actor_user_id text not null,
          qty_micros bigint not null default 0,
          expected_batch text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, scan_id)
        )",
        "alter table shared_scans add column if not exists qty_micros bigint not null default 0",
        "alter table shared_scans add column if not exists expected_batch text not null default ''",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("wrong-batch tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i06b-owner"),
        (foreign, foreign_actor, "i06b-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("wrong-batch-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i06b-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i06b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i06b-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i06b",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let expected = format!("B-EXP-{owner}");
    let scanned = format!("B-BAD-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/scan-batch-faults?scanned_batch={scanned}&expected_batch={expected}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let matched = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/scan-batch-faults"),
        Some(
            serde_json::json!({
                "request_id": "i06b-match",
                "scanned_batch": expected,
                "expected_batch": expected
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(matched.0, StatusCode::OK, "{}", matched.1);
    let located = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/scan-batch-faults"),
        Some(
            serde_json::json!({
                "request_id": "i06b-locate",
                "scanned_batch": scanned,
                "expected_batch": expected
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(located.0, StatusCode::OK, "{}", located.1);
    let located_body: Value = serde_json::from_str(&located.1).unwrap_or(Value::Null);
    let scan_id = located_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!scan_id.is_empty(), "{}", located.1);
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/scan-batch-faults"),
        Some(
            serde_json::json!({
                "request_id": "i06b-again",
                "scanned_batch": scanned,
                "expected_batch": expected
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(again.0, StatusCode::OK, "{}", again.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/scan-batch-faults?scanned_batch={scanned}&expected_batch={expected}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["scan_id"].as_str().unwrap_or(""), scan_id);
    assert_eq!(body["scanned_batch"].as_str().unwrap_or(""), scanned);
    assert_eq!(body["expected_batch"].as_str().unwrap_or(""), expected);
    assert_eq!(body["status"].as_str().unwrap_or(""), "WRONG_BATCH");
    assert_eq!(body["document_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/scan-batch-faults?scanned_batch={scanned}&expected_batch={expected}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&scan_id));
    let row: (String, String, String, i64, i64, i64, i64) = sqlx::query_as(
        "select scan_id::text, code, expected_batch, qty_micros,
            (select count(*) from shared_scans where tenant_id = $1 and status = 'WRONG_BATCH'),
            (select count(*) from command_receipts where tenant_id = $1 and status = 'ACCEPTED' and effect_id like 'DOC-%'),
            (select count(*) from shared_scans where tenant_id = $2)
         from shared_scans where tenant_id = $1 and status = 'WRONG_BATCH'",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, scan_id);
    assert_eq!(row.1, scanned);
    assert_eq!(row.2, expected);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 1);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 0);
    eprintln!(
        "wrong-batch-ok tenant={owner} foreign={foreign} actor={owner_actor} scanned={} expected={} scan={} qty={} rows={} documents={} foreign_scans={}",
        row.1, row.2, row.0, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn offline_print_not_complete_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_print_events (
          tenant_id uuid not null,
          event_id uuid not null,
          job_ref text not null,
          state text not null,
          copies bigint not null,
          reason text not null default '',
          actor_kind text not null,
          actor_user_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, event_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("offline-print tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i06c-owner"),
        (foreign, foreign_actor, "i06c-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("offline-print-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i06c-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i06c",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i06c-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i06c",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let job = format!("JOB-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/offline-prints?job_ref={job}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/offline-prints"),
        Some(
            serde_json::json!({
                "request_id": "i06c-early",
                "job_ref": job,
                "actor_user_id": owner_actor,
                "complete": true
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(early.0, StatusCode::OK, "{}", early.1);
    let stored = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/offline-prints"),
        Some(
            serde_json::json!({
                "request_id": "i06c-offline",
                "job_ref": job,
                "actor_user_id": owner_actor,
                "complete": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stored.0, StatusCode::OK, "{}", stored.1);
    let stored_body: Value = serde_json::from_str(&stored.1).unwrap_or(Value::Null);
    let event_id = stored_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!event_id.is_empty(), "{}", stored.1);
    let complete = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/offline-prints"),
        Some(
            serde_json::json!({
                "request_id": "i06c-complete",
                "job_ref": job,
                "actor_user_id": owner_actor,
                "complete": true
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(complete.0, StatusCode::OK, "{}", complete.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/offline-prints?job_ref={job}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["event_id"].as_str().unwrap_or(""), event_id);
    assert_eq!(body["status"].as_str().unwrap_or(""), "FAILED");
    assert_eq!(body["failed_rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["confirmed_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/offline-prints?job_ref={job}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&event_id));
    let row: (String, String, String, i64, i64, i64, i64) = sqlx::query_as(
        "select event_id::text, state, reason,
            (select count(*) from shared_print_events where tenant_id = $1 and state = 'FAILED'),
            (select count(*) from shared_print_events where tenant_id = $1 and state = 'CONFIRMED'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i06c-complete'),
            (select count(*) from shared_print_events where tenant_id = $2)
         from shared_print_events where tenant_id = $1 and job_ref = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&job)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, event_id);
    assert_eq!(row.1, "FAILED");
    assert_eq!(row.2, "OFFLINE");
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 0);
    eprintln!(
        "offline-print-ok tenant={owner} foreign={foreign} actor={owner_actor} job={job} event={} state={} reason={} failed={} confirmed={} complete_receipts={} foreign_events={}",
        row.0, row.1, row.2, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn paper_code_rescan_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("paper-rescan tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i06d-owner"),
        (foreign, foreign_actor, "i06d-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("paper-rescan-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i06d-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i06d",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i06d-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i06d",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let paper = format!("PAPER-{owner}");
    let item = format!("ITEM-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/paper-rescans?paper_code={paper}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let unknown = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/paper-rescans"),
        Some(
            serde_json::json!({
                "request_id": "i06d-unknown",
                "paper_code": paper,
                "actor_user_id": owner_actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(unknown.0, StatusCode::OK, "{}", unknown.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/paper-codes"),
        Some(
            serde_json::json!({
                "request_id": "i06d-issue",
                "paper_code": paper,
                "item_code": item,
                "actor_user_id": owner_actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let recognized = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/paper-rescans"),
        Some(
            serde_json::json!({
                "request_id": "i06d-rescan",
                "paper_code": paper,
                "actor_user_id": owner_actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(recognized.0, StatusCode::OK, "{}", recognized.1);
    let recognized_body: Value = serde_json::from_str(&recognized.1).unwrap_or(Value::Null);
    let scan_id = recognized_body["effect_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(!scan_id.is_empty(), "{}", recognized.1);
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/paper-rescans"),
        Some(
            serde_json::json!({
                "request_id": "i06d-again",
                "paper_code": paper,
                "actor_user_id": owner_actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(again.0, StatusCode::OK, "{}", again.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/paper-rescans?paper_code={paper}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["scan_id"].as_str().unwrap_or(""), scan_id);
    assert_eq!(body["item_code"].as_str().unwrap_or(""), item);
    assert_eq!(body["status"].as_str().unwrap_or(""), "RECOGNIZED");
    assert_eq!(body["document_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/paper-rescans?paper_code={paper}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&scan_id));
    let row: (String, String, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select code_id::text, item_code, qty_micros,
            (select count(*) from shared_paper_codes where tenant_id = $1 and status = 'ISSUED'),
            (select count(*) from shared_paper_codes where tenant_id = $1 and status = 'RECOGNIZED'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id in ('i06d-unknown','i06d-again')),
            (select count(*) from shared_paper_codes where tenant_id = $2)
         from shared_paper_codes where tenant_id = $1 and status = 'RECOGNIZED'",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, scan_id);
    assert_eq!(row.1, item);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 1);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 0);
    eprintln!(
        "paper-rescan-ok tenant={owner} foreign={foreign} actor={owner_actor} paper={paper} item={} scan={} qty={} issued={} recognized={} refused_receipts={} foreign_codes={}",
        row.1, row.0, row.2, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn held_stock_not_available_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_inventory_alerts (
          tenant_id uuid not null,
          alert_id uuid not null,
          rule_code text not null,
          object_code text not null,
          cutoff_at text not null,
          reason text not null,
          observed bigint not null,
          threshold bigint not null,
          state text not null,
          notified boolean not null,
          purchase_id text not null default '',
          work_order_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, alert_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("held-available tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i07a-owner"),
        (foreign, foreign_actor, "i07a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let product = Uuid::new_v4();
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i07a")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i07a")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("held-available-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i07a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i07a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i07a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i07a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let object = format!("ITEM-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/available-alerts?sku_id={sku}&object_code={object}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    for (request_id, batch, quality, delta) in [
        ("i07a-good", "B-GOOD", "GOOD", 1_000_000_i64),
        ("i07a-hold", "B-HOLD", "HOLD", 250_000_i64),
        ("i07a-frozen", "B-FROZEN", "HOLD", 250_000_i64),
    ] {
        let posted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-ledgers"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch,
                    "quality_status": quality,
                    "bucket": "ON_HAND",
                    "delta_micros": delta
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    }
    let raised = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/available-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07a-alert",
                "sku_id": sku,
                "object_code": object,
                "cutoff_at": "2026-10-05T00:00:00Z",
                "threshold": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(raised.0, StatusCode::OK, "{}", raised.1);
    let raised_body: Value = serde_json::from_str(&raised.1).unwrap_or(Value::Null);
    let alert_id = raised_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!alert_id.is_empty(), "{}", raised.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/available-alerts?sku_id={sku}&object_code={object}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["alert_id"].as_str().unwrap_or(""), alert_id);
    assert_eq!(body["observed"].as_i64().unwrap_or(-1), 1_000_000);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 1_500_000);
    assert_eq!(body["unavailable"].as_i64().unwrap_or(0), 500_000);
    assert_eq!(body["stock_rows"].as_i64().unwrap_or(0), 3);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/available-alerts?sku_id={sku}&object_code={object}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&alert_id));
    let row: (String, i64, String, String, i64, i64, i64, i64) = sqlx::query_as(
        "select alert_id::text, observed, purchase_id, work_order_id,
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3 and bucket = 'ON_HAND'),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3 and bucket = 'ON_HAND' and quality_status = 'HOLD'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and sku_id = $3),
            (select count(*) from shared_inventory_alerts where tenant_id = $2)
         from shared_inventory_alerts where tenant_id = $1 and rule_code = 'LOW_STOCK'",
    )
    .bind(owner)
    .bind(foreign)
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, alert_id);
    assert_eq!(row.1, 1_000_000);
    assert!(row.2.is_empty());
    assert!(row.3.is_empty());
    assert_eq!(row.4, 1_500_000);
    assert_eq!(row.5, 500_000);
    assert_eq!(row.6, 3);
    assert_eq!(row.7, 0);
    eprintln!(
        "held-available-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} warehouse={warehouse} location={location} alert={} observed={} on_hand={} unavailable={} stock_rows={} foreign_alerts={}",
        row.0, row.1, row.4, row.5, row.6, row.7
    );
    Ok(())
}

#[tokio::test]
async fn alert_handle_keeps_facts_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_inventory_alerts (
          tenant_id uuid not null,
          alert_id uuid not null,
          rule_code text not null,
          object_code text not null,
          cutoff_at text not null,
          reason text not null,
          observed bigint not null,
          threshold bigint not null,
          state text not null,
          notified boolean not null,
          purchase_id text not null default '',
          work_order_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, alert_id)
        )",
        "create table if not exists shared_purchase_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let product = Uuid::new_v4();
    eprintln!("alert-handle tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i07b-owner"),
        (foreign, foreign_actor, "i07b-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i07b")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i07b")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("alert-handle-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i07b-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i07b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i07b-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i07b",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let object = format!("ITEM-{owner}");
    let posted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i07b-stock",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": "B-GOOD",
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    let raised = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/inventory-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07b-open",
                "rule_code": "LOW_STOCK",
                "object_code": object,
                "cutoff_at": "2026-10-05T00:00:00Z",
                "reason": "below safety",
                "observed": 250_000,
                "threshold": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(raised.0, StatusCode::OK, "{}", raised.1);
    let raised_body: Value = serde_json::from_str(&raised.1).unwrap_or(Value::Null);
    let alert_id = raised_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!alert_id.is_empty(), "{}", raised.1);
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/inventory-alert-handles?alert_id={alert_id}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let handled = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/inventory-alert-handles"),
        Some(
            serde_json::json!({
                "request_id": "i07b-handle",
                "alert_id": alert_id,
                "decision": "buyer reviewed"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(handled.0, StatusCode::OK, "{}", handled.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/inventory-alert-handles?alert_id={alert_id}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "HANDLED");
    assert_eq!(body["stock_rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["purchase_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["work_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/inventory-alert-handles?alert_id={alert_id}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&alert_id));
    let row: (String, String, String, i64, i64, i64, i64) = sqlx::query_as(
        "select state, purchase_id, work_order_id,
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and sku_id = $3),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3),
            (select count(*) from shared_purchase_lines where tenant_id = $1),
            (select count(*) from shared_inventory_alerts where tenant_id = $2)
         from shared_inventory_alerts where tenant_id = $1 and alert_id = $4",
    )
    .bind(owner)
    .bind(foreign)
    .bind(sku)
    .bind(Uuid::parse_str(&alert_id)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "HANDLED");
    assert!(row.1.is_empty());
    assert!(row.2.is_empty());
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 250_000);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 0);
    eprintln!(
        "alert-handle-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} alert={alert_id} state={} stock_rows={} on_hand={} purchase_rows={} foreign_alerts={}",
        row.0, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn purchase_remainder_open_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_purchase_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          po_id uuid not null,
          version_no integer not null,
          status text not null,
          supplier_code text not null,
          warehouse_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, po_id, version_no)
        )",
        "create table if not exists shared_purchase_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_purchase_receipts (
          tenant_id uuid not null,
          receipt_id uuid not null,
          po_id uuid not null,
          line_no integer not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, receipt_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    eprintln!("purchase-remainder tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t01a-owner"),
        (foreign, foreign_actor, "t01a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'purchase_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("purchase-remainder-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t01a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t01a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t01a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t01a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t01a-open",
                "supplier_code": "S-1",
                "warehouse_code": "WH-1",
                "action": "OPEN",
                "lines": [{
                    "line_no": 1,
                    "item_code": item,
                    "qty_micros": 10_000_000,
                    "price_cent": 2000,
                    "tax_cent": 260,
                    "due_date": "2026-10-20"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let revision_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let po_id: Uuid = sqlx::query_scalar(
        "select po_id from shared_purchase_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .fetch_one(&pool)
    .await?;
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/purchase-remainders?po_id={po_id}&line_no=1"),
        None,
    )
    .await?;
    assert_eq!(missing.0, StatusCode::OK, "{}", missing.1);
    let missing_body: Value = serde_json::from_str(&missing.1).unwrap_or(Value::Null);
    assert_eq!(missing_body["remaining"].as_i64().unwrap_or(0), 10_000_000);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(
            serde_json::json!({
                "request_id": "t01a-receive",
                "supplier_id": Uuid::new_v4(),
                "warehouse_id": Uuid::new_v4(),
                "po_id": po_id,
                "line_no": 1,
                "receive_qty": 6_000_000,
                "items": [{
                    "sku_id": Uuid::new_v4(),
                    "qty": 6,
                    "price_cent": 2000
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-closes"),
        Some(serde_json::json!({ "request_id": "t01a-early", "po_id": po_id }).to_string()),
    )
    .await?;
    assert_eq!(early.0, StatusCode::CONFLICT, "{}", early.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/purchase-remainders?po_id={po_id}&line_no=1"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "OPEN");
    assert_eq!(body["ordered"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["received"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["remaining"].as_i64().unwrap_or(0), 4_000_000);
    assert_eq!(body["closed_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/purchase-remainders?po_id={po_id}&line_no=1"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&po_id.to_string()));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select qty_micros from shared_purchase_lines where tenant_id = $1 and revision_id = $2 and line_no = 1),
            (select coalesce(sum(qty_micros), 0)::bigint from shared_purchase_receipts where tenant_id = $1 and po_id = $3),
            (select count(*) from shared_purchase_revisions where tenant_id = $1 and po_id = $3 and status = 'CLOSED'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't01a-early')",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .bind(po_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 10_000_000);
    assert_eq!(row.1, 6_000_000);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "purchase-remainder-ok tenant={owner} foreign={foreign} actor={owner_actor} po={po_id} revision={revision_id} ordered={} received={} closed={} early_receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn purchase_revision_floor_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_purchase_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          po_id uuid not null,
          version_no integer not null,
          status text not null,
          supplier_code text not null,
          warehouse_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, po_id, version_no)
        )",
        "create table if not exists shared_purchase_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_purchase_receipts (
          tenant_id uuid not null,
          receipt_id uuid not null,
          po_id uuid not null,
          line_no integer not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, receipt_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    eprintln!("purchase-floor tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t01b-owner"),
        (foreign, foreign_actor, "t01b-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'purchase_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("purchase-floor-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t01b-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t01b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t01b-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t01b",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t01b-open",
                "supplier_code": "S-1",
                "warehouse_code": "WH-1",
                "action": "OPEN",
                "lines": [{
                    "line_no": 1,
                    "item_code": item,
                    "qty_micros": 10_000_000,
                    "price_cent": 2000,
                    "tax_cent": 260,
                    "due_date": "2026-10-20"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let revision_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let po_id: Uuid = sqlx::query_scalar(
        "select po_id from shared_purchase_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .fetch_one(&pool)
    .await?;
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(
            serde_json::json!({
                "request_id": "t01b-receive",
                "supplier_id": Uuid::new_v4(),
                "warehouse_id": Uuid::new_v4(),
                "po_id": po_id,
                "line_no": 1,
                "receive_qty": 6_000_000,
                "items": [{
                    "sku_id": Uuid::new_v4(),
                    "qty": 6,
                    "price_cent": 2000
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let below = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t01b-below",
                "po_id": po_id,
                "supplier_code": "S-1",
                "warehouse_code": "WH-1",
                "action": "REVISE",
                "lines": [{
                    "line_no": 1,
                    "item_code": item,
                    "qty_micros": 5_000_000,
                    "price_cent": 2000,
                    "tax_cent": 260,
                    "due_date": "2026-10-20"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(below.0, StatusCode::CONFLICT, "{}", below.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/purchase-revision-floors?po_id={po_id}&line_no=1&attempted=5000000"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "HELD");
    assert_eq!(body["revision_id"].as_str().unwrap_or(""), revision_id);
    assert_eq!(body["version_no"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["ordered"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["received"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["attempted"].as_i64().unwrap_or(0), 5_000_000);
    assert_eq!(body["refused_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/purchase-revision-floors?po_id={po_id}&line_no=1&attempted=5000000"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&po_id.to_string()));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select qty_micros from shared_purchase_lines where tenant_id = $1 and revision_id = $2 and line_no = 1),
            (select coalesce(sum(qty_micros), 0)::bigint from shared_purchase_receipts where tenant_id = $1 and po_id = $3),
            (select count(*) from shared_purchase_revisions where tenant_id = $1 and po_id = $3 and version_no > 1),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't01b-below')",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .bind(po_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 10_000_000);
    assert_eq!(row.1, 6_000_000);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "purchase-floor-ok tenant={owner} foreign={foreign} actor={owner_actor} po={po_id} revision={revision_id} ordered={} received={} later_versions={} below_receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn purchase_revision_rollback_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_purchase_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          po_id uuid not null,
          version_no integer not null,
          status text not null,
          supplier_code text not null,
          warehouse_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, po_id, version_no)
        )",
        "create table if not exists shared_purchase_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    eprintln!("purchase-rollback tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t01c-owner"),
        (foreign, foreign_actor, "t01c-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'purchase_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("purchase-rollback-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t01c-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t01c",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t01c-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t01c",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t01c-open",
                "supplier_code": "S-1",
                "warehouse_code": "WH-1",
                "action": "OPEN",
                "lines": [{
                    "line_no": 1,
                    "item_code": item,
                    "qty_micros": 1_000_000,
                    "price_cent": 100,
                    "tax_cent": 13,
                    "due_date": "2026-10-20"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let revision_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let po_id: Uuid = sqlx::query_scalar(
        "select po_id from shared_purchase_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .fetch_one(&pool)
    .await?;
    let mut lines = Vec::with_capacity(100);
    for line_no in 1..=99 {
        lines.push(serde_json::json!({
            "line_no": line_no,
            "item_code": format!("ITEM-{owner}-{line_no}"),
            "qty_micros": 1_000_000,
            "price_cent": 100,
            "tax_cent": 13,
            "due_date": "2026-10-20"
        }));
    }
    lines.push(serde_json::json!({
        "line_no": 99,
        "item_code": format!("ITEM-{owner}-DUP"),
        "qty_micros": 1_000_000,
        "price_cent": 100,
        "tax_cent": 13,
        "due_date": "2026-10-20"
    }));
    assert_eq!(lines.len(), 100);
    let refused = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t01c-bad",
                "po_id": po_id,
                "supplier_code": "S-1",
                "warehouse_code": "WH-1",
                "action": "REVISE",
                "lines": lines
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(refused.0, StatusCode::CONFLICT, "{}", refused.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/purchase-revision-rollbacks?po_id={po_id}&refused_line_count=100"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "ROLLED_BACK");
    assert_eq!(body["revision_id"].as_str().unwrap_or(""), revision_id);
    assert_eq!(body["open_lines"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["later_versions"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["refused_line_count"].as_i64().unwrap_or(0), 100);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/purchase-revision-rollbacks?po_id={po_id}&refused_line_count=100"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&po_id.to_string()));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select count(*) from shared_purchase_lines where tenant_id = $1 and revision_id = $2),
            (select count(*) from shared_purchase_revisions where tenant_id = $1 and po_id = $3 and version_no > 1),
            (select count(*) from shared_purchase_lines where tenant_id = $1 and item_code like $4),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't01c-bad')",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .bind(po_id)
    .bind(format!("ITEM-{owner}-%"))
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 1);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "purchase-rollback-ok tenant={owner} foreign={foreign} actor={owner_actor} po={po_id} revision={revision_id} open_lines={} later_versions={} partial_lines={} bad_receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn purchase_arrival_split_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_purchase_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          po_id uuid not null,
          version_no integer not null,
          status text not null,
          supplier_code text not null,
          warehouse_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, po_id, version_no)
        )",
        "create table if not exists shared_purchase_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_purchase_receipts (
          tenant_id uuid not null,
          receipt_id uuid not null,
          po_id uuid not null,
          line_no integer not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, receipt_id)
        )",
        "create table if not exists shared_receipt_quality (
          tenant_id uuid not null,
          quality_id uuid not null,
          receipt_id uuid not null,
          po_id uuid not null,
          line_no integer not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          supplier_code text not null,
          batch_code text not null,
          arrived_micros bigint not null,
          released_micros bigint not null default 0,
          issued_micros bigint not null default 0,
          unit_cost_cent bigint not null,
          payable_cent bigint not null default 0,
          created_at timestamptz not null default now(),
          primary key (tenant_id, quality_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    eprintln!("purchase-split tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t02a-owner"),
        (foreign, foreign_actor, "t02a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'purchase_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("purchase-split-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t02a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t02a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t02a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t02a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t02a-open",
                "supplier_code": "S-1",
                "warehouse_code": "WH-1",
                "action": "OPEN",
                "lines": [{
                    "line_no": 1,
                    "item_code": item,
                    "qty_micros": 10_000_000,
                    "price_cent": 1500,
                    "tax_cent": 195,
                    "due_date": "2026-10-20"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let revision_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let po_id: Uuid = sqlx::query_scalar(
        "select po_id from shared_purchase_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .fetch_one(&pool)
    .await?;
    let arrive = |request_id: &'static str, qty: i64| {
        serde_json::json!({
            "request_id": request_id,
            "supplier_id": Uuid::new_v4(),
            "warehouse_id": warehouse,
            "po_id": po_id,
            "line_no": 1,
            "arrival_qty": qty,
            "location_id": location,
            "batch_code": format!("LOT-{owner}"),
            "unit_cost_cent": 1500,
            "items": [{ "sku_id": sku, "qty": 1, "price_cent": 1500 }]
        })
        .to_string()
    };
    let first = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(arrive("t02a-first", 6_000_000)),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let second = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(arrive("t02a-second", 4_000_000)),
    )
    .await?;
    assert_eq!(second.0, StatusCode::OK, "{}", second.1);
    let over = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(arrive("t02a-over", 1_000_000)),
    )
    .await?;
    assert_eq!(over.0, StatusCode::CONFLICT, "{}", over.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/purchase-arrival-splits?po_id={po_id}&line_no=1"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "RECEIVED");
    assert_eq!(body["ordered"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["first"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["second"].as_i64().unwrap_or(0), 4_000_000);
    assert_eq!(body["over_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/purchase-arrival-splits?po_id={po_id}&line_no=1"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&po_id.to_string()));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(qty_micros), 0)::bigint from shared_purchase_receipts where tenant_id = $1 and po_id = $2),
            (select count(*) from shared_purchase_receipts where tenant_id = $1 and po_id = $2),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3 and bucket = 'HELD'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't02a-over')",
    )
    .bind(owner)
    .bind(po_id)
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 10_000_000);
    assert_eq!(row.1, 2);
    assert_eq!(row.2, 10_000_000);
    assert_eq!(row.3, 0);
    eprintln!(
        "purchase-split-ok tenant={owner} foreign={foreign} actor={owner_actor} po={po_id} sku={sku} received={} receipts={} held={} over_receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn purchase_receipt_offset_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_purchase_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          po_id uuid not null,
          version_no integer not null,
          status text not null,
          supplier_code text not null,
          warehouse_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, po_id, version_no)
        )",
        "create table if not exists shared_purchase_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_purchase_receipts (
          tenant_id uuid not null,
          receipt_id uuid not null,
          po_id uuid not null,
          line_no integer not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, receipt_id)
        )",
        "create table if not exists shared_receipt_quality (
          tenant_id uuid not null,
          quality_id uuid not null,
          receipt_id uuid not null,
          po_id uuid not null,
          line_no integer not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          supplier_code text not null,
          batch_code text not null,
          arrived_micros bigint not null,
          released_micros bigint not null default 0,
          issued_micros bigint not null default 0,
          unit_cost_cent bigint not null,
          payable_cent bigint not null default 0,
          created_at timestamptz not null default now(),
          primary key (tenant_id, quality_id)
        )",
        "create table if not exists shared_supplier_returns (
          tenant_id uuid not null,
          return_id uuid not null,
          receipt_id uuid not null,
          qty_micros bigint not null,
          cost_cent bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, return_id)
        )",
        "create table if not exists partner_balances (
          tenant_id uuid not null,
          balance_id uuid not null,
          kind text not null,
          partner_code text not null,
          currency text not null,
          source_type text not null,
          source_id text not null,
          amount_cent bigint not null,
          settled_cent bigint not null default 0,
          due_date text,
          created_at timestamptz not null default now(),
          primary key (tenant_id, balance_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    eprintln!("purchase-offset tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t02b-owner"),
        (foreign, foreign_actor, "t02b-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'purchase_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Purchase offset QA',$3)")
        .bind(owner).bind(warehouse).bind(warehouse.to_string()).execute(&pool).await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(owner)
    .bind(owner_actor)
    .bind(warehouse)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("purchase-offset-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t02b-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t02b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t02b-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t02b",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t02b-open",
                "supplier_code": "S-1",
                "warehouse_code": "WH-1",
                "action": "OPEN",
                "lines": [{
                    "line_no": 1,
                    "item_code": item,
                    "qty_micros": 6_000_000,
                    "price_cent": 1500,
                    "tax_cent": 195,
                    "due_date": "2026-10-20"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let revision_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let po_id: Uuid = sqlx::query_scalar(
        "select po_id from shared_purchase_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .fetch_one(&pool)
    .await?;
    let arrived = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(
            serde_json::json!({
                "request_id": "t02b-arrive",
                "supplier_id": Uuid::new_v4(),
                "warehouse_id": warehouse,
                "po_id": po_id,
                "line_no": 1,
                "arrival_qty": 6_000_000,
                "location_id": location,
                "batch_code": format!("LOT-{owner}"),
                "unit_cost_cent": 1500,
                "items": [{ "sku_id": sku, "qty": 1, "price_cent": 1500 }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(arrived.0, StatusCode::OK, "{}", arrived.1);
    let arrived_body: Value = serde_json::from_str(&arrived.1).unwrap_or(Value::Null);
    let quality_id = arrived_body["quality_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let held = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(
            serde_json::json!({
                "request_id": "t02b-held",
                "supplier_id": Uuid::new_v4(),
                "warehouse_id": warehouse,
                "quality_id": quality_id,
                "issue_qty": 6_000_000,
                "items": [{ "sku_id": sku, "qty": 1, "price_cent": 1500 }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(held.0, StatusCode::CONFLICT, "{}", held.1);
    let released = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(
            serde_json::json!({
                "request_id": "t02b-release",
                "supplier_id": Uuid::new_v4(),
                "warehouse_id": warehouse,
                "quality_id": quality_id,
                "release_qty": 6_000_000,
                "items": [{ "sku_id": sku, "qty": 1, "price_cent": 1500 }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(released.0, StatusCode::OK, "{}", released.1);
    let returned = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(
            serde_json::json!({
                "request_id": "t02b-return",
                "supplier_id": Uuid::new_v4(),
                "warehouse_id": warehouse,
                "quality_id": quality_id,
                "return_qty": 2_000_000,
                "items": [{ "sku_id": sku, "qty": 1, "price_cent": 1500 }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(returned.0, StatusCode::OK, "{}", returned.1);
    let over = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/purchase-orders"),
        Some(
            serde_json::json!({
                "request_id": "t02b-over-return",
                "supplier_id": Uuid::new_v4(),
                "warehouse_id": warehouse,
                "quality_id": quality_id,
                "return_qty": 5_000_000,
                "items": [{ "sku_id": sku, "qty": 1, "price_cent": 1500 }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(over.0, StatusCode::CONFLICT, "{}", over.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/purchase-receipt-offsets?quality_id={quality_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "OFFSET");
    assert_eq!(body["arrived"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["released"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["returned_qty"].as_i64().unwrap_or(0), 2_000_000);
    assert_eq!(body["payable_cent"].as_i64().unwrap_or(0), 6_000);
    assert_eq!(body["returned_cost"].as_i64().unwrap_or(0), 3_000);
    assert_eq!(body["held_micros"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["on_hand_micros"].as_i64().unwrap_or(0), 4_000_000);
    assert_eq!(body["issue_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["over_return_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/purchase-receipt-offsets?quality_id={quality_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&quality_id));
    let row: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select payable_cent from shared_receipt_quality where tenant_id = $1 and quality_id = $2),
            (select coalesce(sum(amount_cent), 0)::bigint from partner_balances where tenant_id = $1 and source_id = $3),
            (select coalesce(sum(qty_micros), 0)::bigint from shared_supplier_returns where tenant_id = $1 and receipt_id = $3::uuid),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $4 and bucket = 'ON_HAND'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't02b-held'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't02b-over-return')",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&quality_id)?)
    .bind(
        sqlx::query_scalar::<_, Uuid>(
            "select receipt_id from shared_receipt_quality where tenant_id = $1 and quality_id = $2",
        )
        .bind(owner)
        .bind(Uuid::parse_str(&quality_id)?)
        .fetch_one(&pool)
        .await?
        .to_string(),
    )
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 6_000);
    assert_eq!(row.1, 6_000);
    assert_eq!(row.2, 2_000_000);
    assert_eq!(row.3, 4_000_000);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    eprintln!(
        "purchase-offset-ok tenant={owner} foreign={foreign} actor={owner_actor} quality={quality_id} sku={sku} payable={} ap={} returned_qty={} on_hand={} held_receipts={} over_receipts={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn manual_sale_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_sales_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          so_id uuid not null,
          version_no integer not null,
          status text not null,
          customer_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, so_id, version_no)
        )",
        "create table if not exists shared_sales_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          discount_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          address text not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_sales_demands (
          tenant_id uuid not null,
          demand_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, demand_id),
          unique (tenant_id, so_id, line_no)
        )",
        "create table if not exists shared_sales_boms (
          tenant_id uuid not null,
          bom_id uuid not null,
          so_id uuid not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, bom_id)
        )",
        "create table if not exists shared_trade_confirms (
          tenant_id uuid not null,
          confirm_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          decision text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, confirm_id)
        )",
        "create table if not exists shared_sales_deliveries (
          tenant_id uuid not null,
          delivery_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          order_version integer not null,
          qty_micros bigint not null,
          stage text not null,
          carrier_ref text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, delivery_id)
        )",
        "alter table shared_sales_deliveries add column if not exists item_code text not null default ''",
        "alter table shared_sales_deliveries add column if not exists unit_cost_cent bigint not null default 0",
        "create table if not exists platform_tokens (
          tenant_id uuid not null,
          shop_id uuid not null,
          access_token text,
          refresh_token text,
          expires_at timestamptz,
          scopes text,
          last_refresh_at timestamptz,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, shop_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    eprintln!("manual-sale tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t03a-owner"),
        (foreign, foreign_actor, "t03a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'sales_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("manual-sale-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t03a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t03a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t03a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t03a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t03a-open",
                "customer_code": "C-1",
                "action": "SUBMIT",
                "lines": [{
                    "line_no": 1,
                    "item_code": item,
                    "qty_micros": 10_000_000,
                    "price_cent": 3000,
                    "discount_cent": 100,
                    "tax_cent": 390,
                    "due_date": "2026-10-20",
                    "address": "Dock-1",
                    "source": "RESALE"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let revision_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let so_id: Uuid = sqlx::query_scalar(
        "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .fetch_one(&pool)
    .await?;
    let confirmed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-confirms"),
        Some(
            serde_json::json!({ "request_id": "t03a-confirm", "so_id": so_id, "line_no": 1 })
                .to_string(),
        ),
    )
    .await?;
    assert_eq!(confirmed.0, StatusCode::OK, "{}", confirmed.1);
    let fulfilled = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t03a-fulfill",
                "so_id": so_id,
                "customer_code": "C-1",
                "action": "FULFILL",
                "lines": [{
                    "line_no": 1,
                    "item_code": item,
                    "qty_micros": 10_000_000,
                    "price_cent": 3000,
                    "discount_cent": 100,
                    "tax_cent": 390,
                    "due_date": "2026-10-20",
                    "address": "Dock-1",
                    "source": "RESALE"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(fulfilled.0, StatusCode::OK, "{}", fulfilled.1);
    let delivered = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t03a-deliver",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(delivered.0, StatusCode::OK, "{}", delivered.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/manual-sales?so_id={so_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "MANUAL");
    assert_eq!(body["shop_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["platform_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["confirm_rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["demand_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["delivered"].as_i64().unwrap_or(0), 6_000_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/manual-sales?so_id={so_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&so_id.to_string()));
    let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select count(*) from shops where tenant_id = $1),
            (select count(*) from platform_tokens where tenant_id = $1),
            (select count(*) from shared_trade_confirms where tenant_id = $1 and so_id = $2),
            (select count(*) from shared_sales_demands where tenant_id = $1 and so_id = $2),
            (select coalesce(sum(qty_micros), 0)::bigint from shared_sales_deliveries where tenant_id = $1 and so_id = $2)",
    )
    .bind(owner)
    .bind(so_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 0);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 6_000_000);
    eprintln!(
        "manual-sale-ok tenant={owner} foreign={foreign} actor={owner_actor} so={so_id} shops={} tokens={} confirms={} demands={} delivered={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn sales_revision_floor_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_sales_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          so_id uuid not null,
          version_no integer not null,
          status text not null,
          customer_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, so_id, version_no)
        )",
        "create table if not exists shared_sales_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          discount_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          address text not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_sales_deliveries (
          tenant_id uuid not null,
          delivery_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          order_version integer not null,
          qty_micros bigint not null,
          stage text not null,
          carrier_ref text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, delivery_id)
        )",
        "alter table shared_sales_deliveries add column if not exists item_code text not null default ''",
        "alter table shared_sales_deliveries add column if not exists unit_cost_cent bigint not null default 0",
        "create table if not exists shared_sales_cancellations (
          tenant_id uuid not null,
          cancel_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, cancel_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    eprintln!("sales-floor tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t03b-owner"),
        (foreign, foreign_actor, "t03b-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'sales_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("sales-floor-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t03b-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t03b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t03b-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t03b",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let line = serde_json::json!({
        "line_no": 1,
        "item_code": item,
        "qty_micros": 10_000_000,
        "price_cent": 3000,
        "discount_cent": 100,
        "tax_cent": 390,
        "due_date": "2026-10-20",
        "address": "Dock-1",
        "source": "RESALE"
    });
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t03b-open",
                "customer_code": "C-1",
                "action": "SUBMIT",
                "lines": [line.clone()]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let submit_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let so_id: Uuid = sqlx::query_scalar(
        "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&submit_id)?)
    .fetch_one(&pool)
    .await?;
    let fulfilled = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t03b-fulfill",
                "so_id": so_id,
                "customer_code": "C-1",
                "expected_version": 1,
                "action": "FULFILL",
                "lines": [line.clone()]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(fulfilled.0, StatusCode::OK, "{}", fulfilled.1);
    let fulfilled_body: Value = serde_json::from_str(&fulfilled.1).unwrap_or(Value::Null);
    let revision_id = fulfilled_body["effect_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let delivered = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t03b-deliver",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(delivered.0, StatusCode::OK, "{}", delivered.1);
    let mut below = line.clone();
    below["qty_micros"] = serde_json::json!(5_000_000);
    let refused = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t03b-below",
                "so_id": so_id,
                "customer_code": "C-1",
                "expected_version": 2,
                "action": "REVISE",
                "lines": [below]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(refused.0, StatusCode::CONFLICT, "{}", refused.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/sales-revision-floors?so_id={so_id}&line_no=1&attempted=5000000"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "HELD");
    assert_eq!(body["revision_id"].as_str().unwrap_or(""), revision_id);
    assert_eq!(body["version_no"].as_i64().unwrap_or(0), 2);
    assert_eq!(body["ordered"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["delivered"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["attempted"].as_i64().unwrap_or(0), 5_000_000);
    assert_eq!(body["later_versions"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/sales-revision-floors?so_id={so_id}&line_no=1&attempted=5000000"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&so_id.to_string()));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select qty_micros from shared_sales_lines where tenant_id = $1 and revision_id = $2 and line_no = 1),
            (select coalesce(sum(qty_micros), 0)::bigint from shared_sales_deliveries where tenant_id = $1 and so_id = $3),
            (select count(*) from shared_sales_revisions where tenant_id = $1 and so_id = $3 and version_no > 2),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't03b-below')",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .bind(so_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 10_000_000);
    assert_eq!(row.1, 6_000_000);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "sales-floor-ok tenant={owner} foreign={foreign} actor={owner_actor} so={so_id} revision={revision_id} ordered={} delivered={} later_versions={} below_receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn sales_return_net_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_sales_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          so_id uuid not null,
          version_no integer not null,
          status text not null,
          customer_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, so_id, version_no)
        )",
        "create table if not exists shared_sales_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          discount_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          address text not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_sales_deliveries (
          tenant_id uuid not null,
          delivery_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          order_version integer not null,
          qty_micros bigint not null,
          stage text not null,
          carrier_ref text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, delivery_id)
        )",
        "alter table shared_sales_deliveries add column if not exists item_code text not null default ''",
        "alter table shared_sales_deliveries add column if not exists unit_cost_cent bigint not null default 0",
        "create table if not exists shared_sales_cancellations (
          tenant_id uuid not null,
          cancel_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, cancel_id)
        )",
        "create table if not exists shared_sales_returns (
          tenant_id uuid not null,
          return_id uuid not null,
          delivery_id uuid not null,
          good_micros bigint not null,
          rejected_micros bigint not null,
          cost_cent bigint not null,
          credit_cent bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, return_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists partner_balances (
          tenant_id uuid not null,
          balance_id uuid not null,
          kind text not null,
          partner_code text not null,
          currency text not null,
          source_type text not null,
          source_id text not null,
          amount_cent bigint not null,
          settled_cent bigint not null default 0,
          due_date text,
          created_at timestamptz not null default now(),
          primary key (tenant_id, balance_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    let batch = format!("B-{owner}");
    eprintln!("sales-return-net tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t05a-owner"),
        (foreign, foreign_actor, "t05a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'sales_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("t05a")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("t05a")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("sales-return-net-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t05a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t05a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t05a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t05a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let line = serde_json::json!({
        "line_no": 1,
        "item_code": item,
        "qty_micros": 6_000_000,
        "price_cent": 2500,
        "discount_cent": 0,
        "tax_cent": 325,
        "due_date": "2026-10-20",
        "address": "Dock-1",
        "source": "RESALE"
    });
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t05a-open",
                "customer_code": "C-1",
                "action": "SUBMIT",
                "lines": [line.clone()]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let submit_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let so_id: Uuid = sqlx::query_scalar(
        "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&submit_id)?)
    .fetch_one(&pool)
    .await?;
    let fulfilled = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t05a-fulfill",
                "so_id": so_id,
                "customer_code": "C-1",
                "expected_version": 1,
                "action": "FULFILL",
                "lines": [line]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(fulfilled.0, StatusCode::OK, "{}", fulfilled.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "t05a-receive",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let source = format!("{so_id}:1");
    let reserved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-reservations"),
        Some(
            serde_json::json!({
                "request_id": "t05a-reserve",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "source_line": source,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(reserved.0, StatusCode::OK, "{}", reserved.1);
    let planned = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t05a-plan",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(planned.0, StatusCode::OK, "{}", planned.1);
    let planned_body: Value = serde_json::from_str(&planned.1).unwrap_or(Value::Null);
    let delivery_id = planned_body["delivery_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    for action in ["RESERVE", "PICK", "REVIEW"] {
        let stepped = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
            Some(
                serde_json::json!({
                    "request_id": format!("t05a-step-{}", action.to_ascii_lowercase()),
                    "line_no": 1,
                    "expected_version": 2,
                    "qty_micros": 6_000_000,
                    "action": action,
                    "delivery_id": delivery_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(stepped.0, StatusCode::OK, "{}", stepped.1);
    }
    let returned = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/returns"),
        Some(
            serde_json::json!({
                "request_id": "t05a-return",
                "delivery_id": delivery_id,
                "good_micros": 2_000_000,
                "rejected_micros": 0,
                "unit_cost_cent": 2500,
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "customer_code": "C-1"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(returned.0, StatusCode::OK, "{}", returned.1);
    let over = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/returns"),
        Some(
            serde_json::json!({
                "request_id": "t05a-over",
                "delivery_id": delivery_id,
                "good_micros": 5_000_000,
                "rejected_micros": 0,
                "unit_cost_cent": 2500,
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "customer_code": "C-1"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(over.0, StatusCode::CONFLICT, "{}", over.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/sales-return-nets?delivery_id={delivery_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "NET");
    assert_eq!(body["delivered"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["returned"].as_i64().unwrap_or(0), 2_000_000);
    assert_eq!(body["net"].as_i64().unwrap_or(0), 4_000_000);
    assert_eq!(body["credit_cent"].as_i64().unwrap_or(0), 5_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/sales-return-nets?delivery_id={delivery_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&delivery_id));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(good_micros + rejected_micros), 0)::bigint from shared_sales_returns where tenant_id = $1 and delivery_id = $2),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and request_id = 't05a-return' and bucket = 'ON_HAND'),
            (select coalesce(sum(-amount_cent), 0)::bigint from partner_balances where tenant_id = $1 and source_type = 'SALES_RETURN' and source_id = $3),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't05a-over')",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&delivery_id)?)
    .bind(&delivery_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 2_000_000);
    assert_eq!(row.1, 2_000_000);
    assert_eq!(row.2, 5_000);
    assert_eq!(row.3, 0);
    eprintln!(
        "sales-return-net-ok tenant={owner} foreign={foreign} actor={owner_actor} so={so_id} delivery={delivery_id} returned={} stock={} credit={} over_receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn sales_delivery_repeat_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_sales_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          so_id uuid not null,
          version_no integer not null,
          status text not null,
          customer_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, so_id, version_no)
        )",
        "create table if not exists shared_sales_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          discount_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          address text not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_sales_deliveries (
          tenant_id uuid not null,
          delivery_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          order_version integer not null,
          qty_micros bigint not null,
          stage text not null,
          carrier_ref text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, delivery_id)
        )",
        "alter table shared_sales_deliveries add column if not exists item_code text not null default ''",
        "alter table shared_sales_deliveries add column if not exists unit_cost_cent bigint not null default 0",
        "create table if not exists shared_sales_cancellations (
          tenant_id uuid not null,
          cancel_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, cancel_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    let batch = format!("B-{owner}");
    eprintln!("sales-repeat tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t04b-owner"),
        (foreign, foreign_actor, "t04b-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'sales_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("t04b")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("t04b")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("sales-repeat-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t04b-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t04b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t04b-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t04b",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let line = serde_json::json!({
        "line_no": 1,
        "item_code": item,
        "qty_micros": 10_000_000,
        "price_cent": 2500,
        "discount_cent": 0,
        "tax_cent": 325,
        "due_date": "2026-10-20",
        "address": "Dock-1",
        "source": "RESALE"
    });
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t04b-open",
                "customer_code": "C-1",
                "action": "SUBMIT",
                "lines": [line.clone()]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let submit_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let so_id: Uuid = sqlx::query_scalar(
        "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&submit_id)?)
    .fetch_one(&pool)
    .await?;
    let fulfilled = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t04b-fulfill",
                "so_id": so_id,
                "customer_code": "C-1",
                "expected_version": 1,
                "action": "FULFILL",
                "lines": [line]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(fulfilled.0, StatusCode::OK, "{}", fulfilled.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "t04b-receive",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 10_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let source = format!("{so_id}:1");
    let reserved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-reservations"),
        Some(
            serde_json::json!({
                "request_id": "t04b-reserve",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "source_line": source,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(reserved.0, StatusCode::OK, "{}", reserved.1);
    let planned = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t04b-plan",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(planned.0, StatusCode::OK, "{}", planned.1);
    let planned_body: Value = serde_json::from_str(&planned.1).unwrap_or(Value::Null);
    let delivery_id = planned_body["delivery_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    for action in ["RESERVE", "PICK"] {
        let stepped = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
            Some(
                serde_json::json!({
                    "request_id": format!("t04b-step-{}", action.to_ascii_lowercase()),
                    "line_no": 1,
                    "expected_version": 2,
                    "qty_micros": 6_000_000,
                    "action": action,
                    "delivery_id": delivery_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(stepped.0, StatusCode::OK, "{}", stepped.1);
    }
    let reviewed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t04b-review",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 6_000_000,
                "action": "REVIEW",
                "delivery_id": delivery_id,
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(reviewed.0, StatusCode::OK, "{}", reviewed.1);
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t04b-again",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 6_000_000,
                "action": "REVIEW",
                "delivery_id": delivery_id,
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(again.0, StatusCode::CONFLICT, "{}", again.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/sales-delivery-repeats?delivery_id={delivery_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "REVIEWED");
    assert_eq!(body["qty"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), -6_000_000);
    assert_eq!(body["reserved"].as_i64().unwrap_or(0), -6_000_000);
    assert_eq!(body["deduction_rows"].as_i64().unwrap_or(0), 2);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/sales-delivery-repeats?delivery_id={delivery_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&delivery_id));
    let row: (String, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select stage from shared_sales_deliveries where tenant_id = $1 and delivery_id = $2),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and source_line = $3 and bucket = 'ON_HAND'),
            (select coalesce(sum(delta_micros) filter (where delta_micros < 0), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and source_line = $3 and bucket = 'RESERVED'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and source_line = $3 and delta_micros < 0),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't04b-again')",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&delivery_id)?)
    .bind(&source)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "REVIEWED");
    assert_eq!(row.1, -6_000_000);
    assert_eq!(row.2, -6_000_000);
    assert_eq!(row.3, 2);
    assert_eq!(row.4, 0);
    eprintln!(
        "sales-repeat-ok tenant={owner} foreign={foreign} actor={owner_actor} so={so_id} delivery={delivery_id} stage={} on_hand={} reserved={} deduction_rows={} again_receipts={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn sales_delivery_split_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_sales_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          so_id uuid not null,
          version_no integer not null,
          status text not null,
          customer_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, so_id, version_no)
        )",
        "create table if not exists shared_sales_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          discount_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          address text not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_sales_deliveries (
          tenant_id uuid not null,
          delivery_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          order_version integer not null,
          qty_micros bigint not null,
          stage text not null,
          carrier_ref text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, delivery_id)
        )",
        "alter table shared_sales_deliveries add column if not exists item_code text not null default ''",
        "alter table shared_sales_deliveries add column if not exists unit_cost_cent bigint not null default 0",
        "create table if not exists shared_sales_cancellations (
          tenant_id uuid not null,
          cancel_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, cancel_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    eprintln!("sales-split tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t04a-owner"),
        (foreign, foreign_actor, "t04a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'sales_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("sales-split-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("t04a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t04a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t04a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t04a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let line = serde_json::json!({
        "line_no": 1,
        "item_code": item,
        "qty_micros": 10_000_000,
        "price_cent": 3000,
        "discount_cent": 0,
        "tax_cent": 390,
        "due_date": "2026-10-20",
        "address": "Dock-1",
        "source": "RESALE"
    });
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t04a-open",
                "customer_code": "C-1",
                "action": "SUBMIT",
                "lines": [line.clone()]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let submit_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let so_id: Uuid = sqlx::query_scalar(
        "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&submit_id)?)
    .fetch_one(&pool)
    .await?;
    let fulfilled = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t04a-fulfill",
                "so_id": so_id,
                "customer_code": "C-1",
                "expected_version": 1,
                "action": "FULFILL",
                "lines": [line]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(fulfilled.0, StatusCode::OK, "{}", fulfilled.1);
    let first = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t04a-first",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 6_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let second = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t04a-second",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 4_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(second.0, StatusCode::OK, "{}", second.1);
    let over = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-orders/{so_id}/deliveries"),
        Some(
            serde_json::json!({
                "request_id": "t04a-over",
                "line_no": 1,
                "expected_version": 2,
                "qty_micros": 1
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(over.0, StatusCode::CONFLICT, "{}", over.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/sales-delivery-splits?so_id={so_id}&line_no=1"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "PLANNED");
    assert_eq!(body["ordered"].as_i64().unwrap_or(0), 10_000_000);
    assert_eq!(body["first"].as_i64().unwrap_or(0), 6_000_000);
    assert_eq!(body["second"].as_i64().unwrap_or(0), 4_000_000);
    assert_eq!(body["stock_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/sales-delivery-splits?so_id={so_id}&line_no=1"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&so_id.to_string()));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select qty_micros from shared_sales_deliveries where tenant_id = $1 and so_id = $2 and stage = 'PLANNED' order by created_at, delivery_id limit 1),
            (select qty_micros from shared_sales_deliveries where tenant_id = $1 and so_id = $2 and stage = 'PLANNED' order by created_at desc, delivery_id desc limit 1),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't04a-over'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and source_line = $3)",
    )
    .bind(owner)
    .bind(so_id)
    .bind(format!("{so_id}:1"))
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 6_000_000);
    assert_eq!(row.1, 4_000_000);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "sales-split-ok tenant={owner} foreign={foreign} actor={owner_actor} so={so_id} first={} second={} over_receipts={} stock_rows={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn sales_revision_conflict_keeps_draft_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_sales_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          so_id uuid not null,
          version_no integer not null,
          status text not null,
          customer_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, so_id, version_no)
        )",
        "create table if not exists shared_sales_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          discount_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          address text not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists edit_drafts (
          tenant_id uuid not null,
          draft_id uuid not null,
          user_id text not null,
          page_code text not null,
          object_id text not null,
          version_no bigint not null,
          body_text text not null,
          status text not null,
          service_address text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, draft_id),
          unique (tenant_id, user_id, page_code, object_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    let user = format!("user-{owner_actor}");
    eprintln!("sales-conflict tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "t03c-owner"),
        (foreign, foreign_actor, "t03c-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'sales_orders.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("sales-conflict-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let web_login = login_http(
        &app,
        owner,
        &format!("t03c-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-t03c-a",
    )
    .await?;
    let desktop_login = login_http(
        &app,
        owner,
        &format!("t03c-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "tab-t03c-b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("t03c-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-t03c",
    )
    .await?;
    assert_eq!(web_login.0, StatusCode::OK, "{}", web_login.1);
    assert_eq!(desktop_login.0, StatusCode::OK, "{}", desktop_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let web_token = web_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let desktop_token = desktop_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert_ne!(web_token, desktop_token);
    let line = serde_json::json!({
        "line_no": 1,
        "item_code": item,
        "qty_micros": 10_000_000,
        "price_cent": 3000,
        "discount_cent": 100,
        "tax_cent": 390,
        "due_date": "2026-10-20",
        "address": "Dock-1",
        "source": "RESALE"
    });
    let opened = export_http(
        &app,
        &web_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t03c-open",
                "customer_code": "C-1",
                "action": "SUBMIT",
                "lines": [line.clone()]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let submit_id = opened_body["effect_id"].as_str().unwrap_or("").to_string();
    let so_id: Uuid = sqlx::query_scalar(
        "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&submit_id)?)
    .fetch_one(&pool)
    .await?;
    let draft_body = format!("keep:{so_id}:v1");
    let held = export_http(
        &app,
        &desktop_token,
        "POST",
        &format!("/v1/tenants/{owner}/edit-drafts"),
        Some(
            serde_json::json!({
                "request_id": "t03c-draft",
                "user_id": user,
                "page": "SALES",
                "object_id": so_id.to_string(),
                "version": 1,
                "body": draft_body,
                "service_address": "desk-t03c"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(held.0, StatusCode::OK, "{}", held.1);
    let mut winner_line = line.clone();
    winner_line["qty_micros"] = serde_json::json!(9_000_000);
    let winner = export_http(
        &app,
        &web_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t03c-win",
                "so_id": so_id,
                "customer_code": "C-1",
                "expected_version": 1,
                "action": "REVISE",
                "lines": [winner_line]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(winner.0, StatusCode::OK, "{}", winner.1);
    let winner_body: Value = serde_json::from_str(&winner.1).unwrap_or(Value::Null);
    let winner_id = winner_body["effect_id"].as_str().unwrap_or("").to_string();
    let loser = export_http(
        &app,
        &desktop_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "t03c-lose",
                "so_id": so_id,
                "customer_code": "C-1",
                "expected_version": 1,
                "action": "REVISE",
                "lines": [line]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(loser.0, StatusCode::CONFLICT, "{}", loser.1);
    let read = export_http(
        &app,
        &desktop_token,
        "GET",
        &format!("/v1/tenants/{owner}/sales-revision-conflicts?so_id={so_id}&user_id={user}&expected_version=1"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "HELD");
    assert_eq!(body["revision_id"].as_str().unwrap_or(""), winner_id);
    assert_eq!(body["stored_version"].as_i64().unwrap_or(0), 2);
    assert_eq!(body["expected_version"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["later_versions"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["draft_body"].as_str().unwrap_or(""), draft_body);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/sales-revision-conflicts?so_id={so_id}&user_id={user}&expected_version=1"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&so_id.to_string()));
    let row: (i32, i64, String, i64, i64) = sqlx::query_as(
        "select
            (select version_no from shared_sales_revisions where tenant_id = $1 and so_id = $2 and revision_id = $3),
            (select count(*) from shared_sales_revisions where tenant_id = $1 and so_id = $2 and version_no > 2),
            (select body_text from edit_drafts where tenant_id = $1 and object_id = $4 and status = 'HELD'),
            (select version_no from edit_drafts where tenant_id = $1 and object_id = $4 and status = 'HELD'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 't03c-lose')",
    )
    .bind(owner)
    .bind(so_id)
    .bind(Uuid::parse_str(&winner_id)?)
    .bind(so_id.to_string())
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 2);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, draft_body);
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 0);
    eprintln!(
        "sales-conflict-ok tenant={owner} foreign={foreign} actor={owner_actor} so={so_id} revision={winner_id} stored={} later={} draft_version={} lose_receipts={}",
        row.0, row.1, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn short_suggestion_names_gap_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_batches (
          tenant_id uuid not null,
          batch_id uuid not null,
          item_code text not null,
          batch_code text not null,
          produced_on text not null,
          expires_on text not null,
          frozen boolean not null default false,
          serialized boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, batch_id),
          unique (tenant_id, item_code, batch_code)
        )",
        "alter table shared_batches add column if not exists quality_status text not null default 'GOOD'",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_allocation_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          request_ref text not null,
          rule_code text not null,
          batch_code text not null,
          location_code text not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          excluded_text text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
        "create table if not exists shared_allocation_issues (
          tenant_id uuid not null,
          issue_id uuid not null,
          suggestion_id uuid not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, issue_id)
        )",
        "create table if not exists shared_sales_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          so_id uuid not null,
          version_no integer not null,
          status text not null,
          customer_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, so_id, version_no)
        )",
        "create table if not exists shared_sales_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          discount_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          address text not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_sales_demands (
          tenant_id uuid not null,
          demand_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, demand_id),
          unique (tenant_id, so_id, line_no)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let short_sku = Uuid::new_v4();
    let covered_sku = Uuid::new_v4();
    let short_item = format!("SHORT-{owner}");
    let covered_item = format!("COVERED-{owner}");
    let short_batch = format!("B-SHORT-{owner}");
    let covered_batch = format!("B-COVERED-{owner}");
    let request_ref = format!("pick-short-{owner}");
    eprintln!(
        "short-suggestion tenant={owner} foreign={foreign} actor={owner_actor} sku={short_sku}"
    );
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i08f-owner"),
        (foreign, foreign_actor, "i08f-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    for (sku, name) in [(short_sku, "i08f-short"), (covered_sku, "i08f-covered")] {
        let product_id = if sku == short_sku {
            product
        } else {
            Uuid::new_v4()
        };
        sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
            .bind(owner)
            .bind(product_id)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
        )
        .bind(owner)
        .bind(sku)
        .bind(product_id)
        .bind(format!("SKU-{sku}"))
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i08f")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("short-suggestion-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i08f-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i08f",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i08f-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i08f",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    for (item, batch, request_id) in [
        (&short_item, &short_batch, "i08f-short-batch"),
        (&covered_item, &covered_batch, "i08f-covered-batch"),
    ] {
        let saved = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/shared-batches"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "item_code": item,
                    "batch_code": batch,
                    "produced_on": "2026-09-01",
                    "expires_on": "2027-09-01",
                    "serialized": false,
                    "quality_status": "GOOD"
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    }
    for (sku, batch, qty, request_id) in [
        (short_sku, &short_batch, 250_000, "i08f-short-stock"),
        (covered_sku, &covered_batch, 1_000_000, "i08f-covered-stock"),
    ] {
        let posted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-ledgers"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch,
                    "quality_status": "GOOD",
                    "bucket": "ON_HAND",
                    "delta_micros": qty
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    }
    for (item, qty, request_id) in [
        (&short_item, 1_000_000, "i08f-short-submit"),
        (&covered_item, 250_000, "i08f-covered-submit"),
    ] {
        let submitted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/sales-revisions"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "customer_code": "C-1",
                    "action": "SUBMIT",
                    "lines": [{
                        "line_no": 1,
                        "item_code": item,
                        "qty_micros": qty,
                        "price_cent": 100,
                        "discount_cent": 0,
                        "tax_cent": 0,
                        "due_date": "2026-10-20",
                        "address": "A-1",
                        "source": "MAKE"
                    }]
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(submitted.0, StatusCode::OK, "{}", submitted.1);
        let body: Value = serde_json::from_str(&submitted.1).unwrap_or(Value::Null);
        let revision_id = body["effect_id"].as_str().unwrap_or("").to_string();
        let so_id: Uuid = sqlx::query_scalar(
            "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
        )
        .bind(owner)
        .bind(Uuid::parse_str(&revision_id)?)
        .fetch_one(&pool)
        .await?;
        let fulfilled = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/sales-revisions"),
            Some(
                serde_json::json!({
                    "request_id": format!("{request_id}-fulfill"),
                    "so_id": so_id.to_string(),
                    "customer_code": "C-1",
                    "expected_version": 1,
                    "action": "FULFILL",
                    "lines": [{
                        "line_no": 1,
                        "item_code": item,
                        "qty_micros": qty,
                        "price_cent": 100,
                        "discount_cent": 0,
                        "tax_cent": 0,
                        "due_date": "2026-10-20",
                        "address": "A-1",
                        "source": "MAKE"
                    }]
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(fulfilled.0, StatusCode::OK, "{}", fulfilled.1);
    }
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/short-suggestions?item_code={short_item}&request_ref={request_ref}&sku_id={short_sku}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let covered = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/short-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08f-covered",
                "item_code": covered_item,
                "sku_id": covered_sku,
                "location_code": "A-1",
                "request_ref": format!("pick-covered-{owner}")
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(covered.0, StatusCode::CONFLICT, "{}", covered.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/short-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08f-suggest",
                "item_code": short_item,
                "sku_id": short_sku,
                "location_code": "A-1",
                "request_ref": request_ref
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let suggestion_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/short-suggestions?item_code={short_item}&request_ref={request_ref}&sku_id={short_sku}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "SHORT");
    assert_eq!(body["batch_code"].as_str().unwrap_or(""), short_batch);
    assert_eq!(body["chosen_qty"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["excluded"].as_str().unwrap_or(""), "short 750000");
    assert_eq!(body["issue_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/short-suggestions?item_code={short_item}&request_ref={request_ref}&sku_id={short_sku}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&suggestion_id));
    let row: (i64, i64, String, i64, i64) = sqlx::query_as(
        "select
            (select count(*) from shared_batches where tenant_id = $1 and item_code = $2),
            (select count(*) from shared_allocation_suggestions where tenant_id = $1 and request_ref = $3),
            (select excluded_text from shared_allocation_suggestions where tenant_id = $1 and request_ref = $3),
            (select count(*) from shared_allocation_issues where tenant_id = $1),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $4 and quality_status = 'GOOD')",
    )
    .bind(owner)
    .bind(&short_item)
    .bind(&request_ref)
    .bind(short_sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 1);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, "short 750000");
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 250_000);
    eprintln!(
        "short-suggestion-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={short_sku} item={short_item} suggestion={suggestion_id} batches={} suggestions={} excluded={} issues={} stock={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn frozen_batch_excluded_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_batches (
          tenant_id uuid not null,
          batch_id uuid not null,
          item_code text not null,
          batch_code text not null,
          produced_on text not null,
          expires_on text not null,
          frozen boolean not null default false,
          serialized boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, batch_id),
          unique (tenant_id, item_code, batch_code)
        )",
        "alter table shared_batches add column if not exists quality_status text not null default 'GOOD'",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_allocation_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          request_ref text not null,
          rule_code text not null,
          batch_code text not null,
          location_code text not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          excluded_text text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
        "create table if not exists shared_allocation_issues (
          tenant_id uuid not null,
          issue_id uuid not null,
          suggestion_id uuid not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, issue_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    let frozen = format!("B-FRZ-{owner}");
    let open = format!("B-OPEN-{owner}");
    let request_ref = format!("pick-frozen-{owner}");
    eprintln!("frozen-suggestion tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i08e-owner"),
        (foreign, foreign_actor, "i08e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i08e")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i08e")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("frozen-suggestion-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i08e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i08e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i08e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i08e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    for (batch, request_id) in [(&frozen, "i08e-frozen-batch"), (&open, "i08e-open-batch")] {
        let saved = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/shared-batches"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "item_code": item,
                    "batch_code": batch,
                    "produced_on": "2026-09-01",
                    "expires_on": "2027-09-01",
                    "serialized": false,
                    "quality_status": "GOOD"
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    }
    let freeze = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/batch-freezes"),
        Some(
            serde_json::json!({
                "request_id": "i08e-freeze",
                "item_code": item,
                "batch_code": frozen
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(freeze.0, StatusCode::OK, "{}", freeze.1);
    for (batch, request_id) in [(&frozen, "i08e-frozen-stock"), (&open, "i08e-open-stock")] {
        let posted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-ledgers"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch,
                    "quality_status": "GOOD",
                    "bucket": "ON_HAND",
                    "delta_micros": 250_000
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    }
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/frozen-suggestions?item_code={item}&request_ref={request_ref}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/frozen-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08e-early",
                "item_code": format!("MISSING-{owner}"),
                "sku_id": sku,
                "location_code": "A-1",
                "request_ref": format!("missing-{owner}")
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(early.0, StatusCode::CONFLICT, "{}", early.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/frozen-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08e-suggest",
                "item_code": item,
                "sku_id": sku,
                "location_code": "A-1",
                "request_ref": request_ref
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let suggestion_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/frozen-suggestions?item_code={item}&request_ref={request_ref}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "EXCLUDED");
    assert_eq!(body["chosen"].as_str().unwrap_or(""), open);
    assert_eq!(
        body["excluded"].as_str().unwrap_or(""),
        format!("{frozen} frozen")
    );
    assert_eq!(body["issue_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/frozen-suggestions?item_code={item}&request_ref={request_ref}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&suggestion_id));
    let row: (bool, i64, String, i64, i64) = sqlx::query_as(
        "select
            (select frozen from shared_batches where tenant_id = $1 and batch_code = $2),
            (select count(*) from shared_allocation_suggestions where tenant_id = $1 and batch_code = $2),
            (select batch_code from shared_allocation_suggestions where tenant_id = $1 and request_ref = $3),
            (select count(*) from shared_allocation_issues where tenant_id = $1),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $4 and quality_status = 'GOOD')",
    )
    .bind(owner)
    .bind(&frozen)
    .bind(&request_ref)
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert!(row.0);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, open);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 500_000);
    eprintln!(
        "frozen-suggestion-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} item={item} frozen={frozen} suggestion={suggestion_id} chosen={} issues={} stock={}",
        row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn expired_batch_excluded_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_batches (
          tenant_id uuid not null,
          batch_id uuid not null,
          item_code text not null,
          batch_code text not null,
          produced_on text not null,
          expires_on text not null,
          frozen boolean not null default false,
          serialized boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, batch_id),
          unique (tenant_id, item_code, batch_code)
        )",
        "alter table shared_batches add column if not exists quality_status text not null default 'GOOD'",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_allocation_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          request_ref text not null,
          rule_code text not null,
          batch_code text not null,
          location_code text not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          excluded_text text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
        "create table if not exists shared_allocation_issues (
          tenant_id uuid not null,
          issue_id uuid not null,
          suggestion_id uuid not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, issue_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    let expired = format!("B-EXP-{owner}");
    let first = format!("B-A-{owner}");
    let second = format!("B-B-{owner}");
    let request_ref = format!("pick-dated-{owner}");
    eprintln!("dated-suggestion tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i08d-owner"),
        (foreign, foreign_actor, "i08d-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i08d")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i08d")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("dated-suggestion-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i08d-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i08d",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i08d-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i08d",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    for (batch, expires, request_id) in [
        (&second, "2027-09-01", "i08d-second-batch"),
        (&expired, "2026-10-01", "i08d-expired-batch"),
        (&first, "2027-09-01", "i08d-first-batch"),
    ] {
        let saved = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/shared-batches"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "item_code": item,
                    "batch_code": batch,
                    "produced_on": "2026-09-01",
                    "expires_on": expires,
                    "serialized": false,
                    "quality_status": "GOOD"
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    }
    for (batch, request_id) in [
        (&expired, "i08d-expired-stock"),
        (&second, "i08d-second-stock"),
        (&first, "i08d-first-stock"),
    ] {
        let posted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-ledgers"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch,
                    "quality_status": "GOOD",
                    "bucket": "ON_HAND",
                    "delta_micros": 250_000
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    }
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/dated-suggestions?item_code={item}&request_ref={request_ref}&sku_id={sku}&cutoff=2026-10-05"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/dated-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08d-early",
                "item_code": format!("MISSING-{owner}"),
                "sku_id": sku,
                "location_code": "A-1",
                "request_ref": format!("missing-{owner}"),
                "cutoff": "2026-10-05"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(early.0, StatusCode::CONFLICT, "{}", early.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/dated-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08d-suggest",
                "item_code": item,
                "sku_id": sku,
                "location_code": "A-1",
                "request_ref": request_ref,
                "cutoff": "2026-10-05"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/dated-suggestions?item_code={item}&request_ref={request_ref}&sku_id={sku}&cutoff=2026-10-05"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "EXCLUDED");
    assert_eq!(
        body["excluded"].as_str().unwrap_or(""),
        format!("{expired} expired")
    );
    let chosen = body["chosen"].as_array().cloned().unwrap_or_default();
    assert_eq!(chosen.len(), 2);
    assert_eq!(chosen[0].as_str().unwrap_or(""), first);
    assert_eq!(chosen[1].as_str().unwrap_or(""), second);
    assert_eq!(body["issue_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/dated-suggestions?item_code={item}&request_ref={request_ref}&sku_id={sku}&cutoff=2026-10-05"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&expired));
    let row: (i64, String, String, i64, i64) = sqlx::query_as(
        "select
            (select count(*) from shared_allocation_suggestions where tenant_id = $1 and batch_code = $2),
            (select batch_code from shared_allocation_suggestions where tenant_id = $1 and request_ref = $3 order by created_at, batch_code limit 1),
            (select batch_code from shared_allocation_suggestions where tenant_id = $1 and request_ref = $3 order by created_at desc, batch_code desc limit 1),
            (select count(*) from shared_allocation_issues where tenant_id = $1),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $4 and quality_status = 'GOOD')",
    )
    .bind(owner)
    .bind(&expired)
    .bind(&request_ref)
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 0);
    assert_eq!(row.1, first);
    assert_eq!(row.2, second);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 750_000);
    eprintln!(
        "dated-suggestion-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} item={item} expired={expired} first={} second={} issues={} stock={}",
        row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn manual_choice_uses_stored_stock_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_allocation_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          request_ref text not null,
          rule_code text not null,
          batch_code text not null,
          location_code text not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          excluded_text text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
        "create table if not exists shared_allocation_issues (
          tenant_id uuid not null,
          issue_id uuid not null,
          suggestion_id uuid not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, issue_id)
        )",
        "create table if not exists shared_allocation_choices (
          tenant_id uuid not null,
          choice_id uuid not null,
          suggestion_id uuid not null,
          chosen_batch text not null,
          reason text not null,
          decision text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, choice_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let held_sku = Uuid::new_v4();
    let empty_sku = Uuid::new_v4();
    let legal_sku = Uuid::new_v4();
    let held_product = Uuid::new_v4();
    let empty_product = Uuid::new_v4();
    let legal_product = Uuid::new_v4();
    let suggested_batch = format!("B-SUG-{owner}");
    let held_batch = format!("B-HOLD-{owner}");
    let empty_batch = format!("B-EMPTY-{owner}");
    let legal_batch = format!("B-GOOD-{owner}");
    eprintln!("manual-choice tenant={owner} foreign={foreign} actor={owner_actor} sku={legal_sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i08c-owner"),
        (foreign, foreign_actor, "i08c-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    for (product, sku, name) in [
        (held_product, held_sku, "i08c-held"),
        (empty_product, empty_sku, "i08c-empty"),
        (legal_product, legal_sku, "i08c-legal"),
    ] {
        sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
            .bind(owner)
            .bind(product)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
        )
        .bind(owner)
        .bind(sku)
        .bind(product)
        .bind(format!("SKU-{sku}"))
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i08c")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("manual-choice-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i08c-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i08c",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i08c-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i08c",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    for (sku, batch, quality, request_id) in [
        (
            held_sku,
            &suggested_batch,
            "GOOD",
            "i08c-held-suggest-stock",
        ),
        (held_sku, &held_batch, "HOLD", "i08c-held-stock"),
        (
            empty_sku,
            &suggested_batch,
            "GOOD",
            "i08c-empty-suggest-stock",
        ),
        (
            legal_sku,
            &suggested_batch,
            "GOOD",
            "i08c-legal-suggest-stock",
        ),
        (legal_sku, &legal_batch, "GOOD", "i08c-legal-stock"),
    ] {
        let posted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-ledgers"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch,
                    "quality_status": quality,
                    "bucket": "ON_HAND",
                    "delta_micros": 250_000
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    }
    let mut suggestions = Vec::new();
    for (sku, request_id, request_ref) in [
        (held_sku, "i08c-held-save", format!("pick-held-{owner}")),
        (empty_sku, "i08c-empty-save", format!("pick-empty-{owner}")),
        (legal_sku, "i08c-legal-save", format!("pick-legal-{owner}")),
    ] {
        let saved = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-backed-suggestions"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "sku_id": sku,
                    "batch_code": suggested_batch,
                    "location_code": "A-1",
                    "request_ref": request_ref
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
        let body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
        suggestions.push(body["effect_id"].as_str().unwrap_or("").to_string());
    }
    let held_suggestion = &suggestions[0];
    let empty_suggestion = &suggestions[1];
    let legal_suggestion = &suggestions[2];
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-backed-choices?suggestion_id={legal_suggestion}&sku_id={legal_sku}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let held_choice = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-choices"),
        Some(
            serde_json::json!({
                "request_id": "i08c-held",
                "suggestion_id": held_suggestion,
                "sku_id": held_sku,
                "chosen_batch": held_batch,
                "reason": "buyer asked for the held batch"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(held_choice.0, StatusCode::CONFLICT, "{}", held_choice.1);
    let empty_choice = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-choices"),
        Some(
            serde_json::json!({
                "request_id": "i08c-empty",
                "suggestion_id": empty_suggestion,
                "sku_id": empty_sku,
                "chosen_batch": empty_batch,
                "reason": "buyer asked for an empty batch"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(empty_choice.0, StatusCode::CONFLICT, "{}", empty_choice.1);
    let chosen = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-choices"),
        Some(
            serde_json::json!({
                "request_id": "i08c-choose",
                "suggestion_id": legal_suggestion,
                "sku_id": legal_sku,
                "chosen_batch": legal_batch,
                "reason": "buyer asked for the covered batch"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(chosen.0, StatusCode::OK, "{}", chosen.1);
    let chosen_body: Value = serde_json::from_str(&chosen.1).unwrap_or(Value::Null);
    let choice_id = chosen_body["effect_id"].as_str().unwrap_or("").to_string();
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-backed-choices?suggestion_id={legal_suggestion}&sku_id={legal_sku}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "MANUAL");
    assert_eq!(
        body["suggested_batch"].as_str().unwrap_or(""),
        suggested_batch
    );
    assert_eq!(body["chosen_batch"].as_str().unwrap_or(""), legal_batch);
    assert_eq!(body["good"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["held"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["issue_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-backed-choices?suggestion_id={legal_suggestion}&sku_id={legal_sku}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&choice_id));
    let row: (i64, i64, i64, String, String, i64, i64) = sqlx::query_as(
        "select
            (select count(*) from shared_allocation_choices where tenant_id = $1 and suggestion_id = $2),
            (select count(*) from shared_allocation_choices where tenant_id = $1 and suggestion_id = $3),
            (select count(*) from shared_allocation_choices where tenant_id = $1 and suggestion_id = $4),
            (select batch_code from shared_allocation_suggestions where tenant_id = $1 and suggestion_id = $4),
            (select rule_code from shared_allocation_suggestions where tenant_id = $1 and suggestion_id = $4),
            (select count(*) from shared_allocation_issues where tenant_id = $1 and suggestion_id = $4),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $5 and batch_code = $6 and quality_status = 'GOOD')",
    )
    .bind(owner)
    .bind(Uuid::parse_str(held_suggestion)?)
    .bind(Uuid::parse_str(empty_suggestion)?)
    .bind(Uuid::parse_str(legal_suggestion)?)
    .bind(legal_sku)
    .bind(&legal_batch)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 0);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, suggested_batch);
    assert_eq!(row.4, "FEFO");
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 250_000);
    eprintln!(
        "manual-choice-ok tenant={owner} foreign={foreign} actor={owner_actor} legal_sku={legal_sku} held_suggestion={held_suggestion} empty_suggestion={empty_suggestion} legal_suggestion={legal_suggestion} choice={choice_id} held_choices={} empty_choices={} legal_choices={} suggested_batch={} rule={} issues={} good={}",
        row.0, row.1, row.2, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn taken_lot_conflicts_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_allocation_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          request_ref text not null,
          rule_code text not null,
          batch_code text not null,
          location_code text not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          excluded_text text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
        "create table if not exists shared_allocation_issues (
          tenant_id uuid not null,
          issue_id uuid not null,
          suggestion_id uuid not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, issue_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let taken = Uuid::new_v4();
    let kept = Uuid::new_v4();
    let taken_product = Uuid::new_v4();
    let kept_product = Uuid::new_v4();
    let taken_batch = format!("B-TAKEN-{owner}");
    let kept_batch = format!("B-KEPT-{owner}");
    eprintln!("taken-lot tenant={owner} foreign={foreign} actor={owner_actor} sku={taken}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i08b-owner"),
        (foreign, foreign_actor, "i08b-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    for (product, sku, name) in [
        (taken_product, taken, "i08b-taken"),
        (kept_product, kept, "i08b-kept"),
    ] {
        sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
            .bind(owner)
            .bind(product)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
        )
        .bind(owner)
        .bind(sku)
        .bind(product)
        .bind(format!("SKU-{sku}"))
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i08b")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("taken-lot-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i08b-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i08b",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i08b-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i08b",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    for (sku, batch, request_id) in [
        (taken, &taken_batch, "i08b-taken-stock"),
        (kept, &kept_batch, "i08b-kept-stock"),
    ] {
        let posted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-ledgers"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch,
                    "quality_status": "GOOD",
                    "bucket": "ON_HAND",
                    "delta_micros": 1_000_000
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    }
    let taken_saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08b-taken-save",
                "sku_id": taken,
                "batch_code": taken_batch,
                "location_code": "A-1",
                "request_ref": format!("pick-taken-{owner}")
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(taken_saved.0, StatusCode::OK, "{}", taken_saved.1);
    let taken_body: Value = serde_json::from_str(&taken_saved.1).unwrap_or(Value::Null);
    let taken_suggestion = taken_body["effect_id"].as_str().unwrap_or("").to_string();
    let kept_saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08b-kept-save",
                "sku_id": kept,
                "batch_code": kept_batch,
                "location_code": "A-1",
                "request_ref": format!("pick-kept-{owner}")
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(kept_saved.0, StatusCode::OK, "{}", kept_saved.1);
    let kept_body: Value = serde_json::from_str(&kept_saved.1).unwrap_or(Value::Null);
    let kept_suggestion = kept_body["effect_id"].as_str().unwrap_or("").to_string();
    let steal = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i08b-steal",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": taken,
                "batch_code": taken_batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": -750_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(steal.0, StatusCode::OK, "{}", steal.1);
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-backed-issues?suggestion_id={taken_suggestion}&sku_id={taken}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let conflict = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-issues"),
        Some(
            serde_json::json!({
                "request_id": "i08b-conflict",
                "suggestion_id": taken_suggestion,
                "sku_id": taken
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(conflict.0, StatusCode::OK, "{}", conflict.1);
    assert_eq!(conflict.0, StatusCode::CONFLICT, "{}", conflict.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-issues"),
        Some(
            serde_json::json!({
                "request_id": "i08b-issue",
                "suggestion_id": kept_suggestion,
                "sku_id": kept
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let issued_body: Value = serde_json::from_str(&issued.1).unwrap_or(Value::Null);
    let issue_id = issued_body["effect_id"].as_str().unwrap_or("").to_string();
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/stock-backed-issues?suggestion_id={kept_suggestion}&sku_id={kept}"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "ISSUED");
    assert_eq!(body["suggested"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["issued"].as_i64().unwrap_or(0), 1_000_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/stock-backed-issues?suggestion_id={kept_suggestion}&sku_id={kept}"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&issue_id));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $2 and quality_status = 'GOOD'),
            (select count(*) from shared_allocation_issues where tenant_id = $1 and suggestion_id = $3),
            (select count(*) from shared_allocation_issues where tenant_id = $1 and suggestion_id = $4),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i08b-conflict')",
    )
    .bind(owner)
    .bind(taken)
    .bind(Uuid::parse_str(&taken_suggestion)?)
    .bind(Uuid::parse_str(&kept_suggestion)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 250_000);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    eprintln!(
        "taken-lot-ok tenant={owner} foreign={foreign} actor={owner_actor} taken_sku={taken} kept_sku={kept} taken_suggestion={taken_suggestion} kept_suggestion={kept_suggestion} issue={issue_id} taken_on_hand={} taken_issues={} kept_issues={} conflict_receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn suggestion_save_keeps_stock_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_allocation_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          request_ref text not null,
          rule_code text not null,
          batch_code text not null,
          location_code text not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          excluded_text text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
        "create table if not exists shared_allocation_issues (
          tenant_id uuid not null,
          issue_id uuid not null,
          suggestion_id uuid not null,
          qty_micros bigint not null,
          stock_version bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, issue_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let product = Uuid::new_v4();
    let batch = format!("B-KEEP-{owner}");
    eprintln!("stock-suggestion tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i08a-owner"),
        (foreign, foreign_actor, "i08a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i08a")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i08a")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("stock-suggestion-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i08a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i08a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i08a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i08a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-backed-suggestions?suggestion_id={owner}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08a-early",
                "sku_id": sku,
                "batch_code": batch,
                "location_code": "A-1",
                "request_ref": format!("pick-empty-{owner}")
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(early.0, StatusCode::OK, "{}", early.1);
    let stock = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i08a-stock",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stock.0, StatusCode::OK, "{}", stock.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08a-save",
                "sku_id": sku,
                "batch_code": batch,
                "location_code": "A-1",
                "request_ref": format!("pick-{owner}")
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let suggestion_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!suggestion_id.is_empty());
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-backed-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i08a-again",
                "sku_id": sku,
                "batch_code": batch,
                "location_code": "A-1",
                "request_ref": format!("pick-{owner}")
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(again.0, StatusCode::OK, "{}", again.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-backed-suggestions?suggestion_id={suggestion_id}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "KEPT");
    assert_eq!(body["suggested"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["issue_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["batch_code"].as_str().unwrap_or(""), batch);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-backed-suggestions?suggestion_id={suggestion_id}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&suggestion_id));
    let row: (i64, i64, i64, i64, String) = sqlx::query_as(
        "select s.qty_micros,
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3 and batch_code = s.batch_code and quality_status = 'GOOD'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and sku_id = $3),
            (select count(*) from shared_allocation_issues where tenant_id = $1),
            s.excluded_text
         from shared_allocation_suggestions s
         where s.tenant_id = $1 and s.suggestion_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&suggestion_id)?)
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 250_000);
    assert_eq!(row.1, 250_000);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    assert!(row.4.contains("saved without deducting stock"));
    let refused: i64 = sqlx::query_scalar(
        "select count(*) from command_receipts where tenant_id = $1 and request_id in ('i08a-early', 'i08a-again')",
    )
    .bind(owner)
    .fetch_one(&independent)
    .await?;
    assert_eq!(refused, 0);
    eprintln!(
        "stock-suggestion-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} suggestion={suggestion_id} suggested={} on_hand={} stock_rows={} issues={} refused_receipts={refused}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn shortage_suggestion_not_production_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_sales_revisions (
          tenant_id uuid not null,
          revision_id uuid not null,
          so_id uuid not null,
          version_no integer not null,
          status text not null,
          customer_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id),
          unique (tenant_id, so_id, version_no)
        )",
        "create table if not exists shared_sales_lines (
          tenant_id uuid not null,
          revision_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          price_cent bigint not null,
          discount_cent bigint not null,
          tax_cent bigint not null,
          due_date text not null,
          address text not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, revision_id, line_no)
        )",
        "create table if not exists shared_sales_demands (
          tenant_id uuid not null,
          demand_id uuid not null,
          so_id uuid not null,
          line_no integer not null,
          item_code text not null,
          qty_micros bigint not null,
          source_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, demand_id),
          unique (tenant_id, so_id, line_no)
        )",
        "create table if not exists shared_inventory_alerts (
          tenant_id uuid not null,
          alert_id uuid not null,
          rule_code text not null,
          object_code text not null,
          cutoff_at text not null,
          reason text not null,
          observed bigint not null,
          threshold bigint not null,
          state text not null,
          notified boolean not null,
          purchase_id text not null default '',
          work_order_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, alert_id)
        )",
        "create table if not exists shared_purchase_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          item_code text not null,
          qty_micros bigint not null,
          paid boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let short_sku = Uuid::new_v4();
    let covered_sku = Uuid::new_v4();
    let short_product = Uuid::new_v4();
    let covered_product = Uuid::new_v4();
    let short_item = format!("MAKE-{owner}");
    let covered_item = format!("COVERED-{owner}");
    eprintln!(
        "shortage-suggestion tenant={owner} foreign={foreign} actor={owner_actor} sku={short_sku}"
    );
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i07f-owner"),
        (foreign, foreign_actor, "i07f-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(short_product)
        .bind("i07f-short")
        .execute(&pool)
        .await?;
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(covered_product)
        .bind("i07f-covered")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(short_sku)
    .bind(short_product)
    .bind(format!("SKU-{short_sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(covered_sku)
    .bind(covered_product)
    .bind(format!("SKU-{covered_sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i07f")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("shortage-suggestion-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i07f-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i07f",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i07f-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i07f",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/shortage-suggestions?item_code={short_item}&sku_id={short_sku}"
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let stock = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i07f-stock",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": short_sku,
                "batch_code": "B-SHORT",
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stock.0, StatusCode::OK, "{}", stock.1);
    let covered_stock = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i07f-covered-stock",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": covered_sku,
                "batch_code": "B-COVERED",
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(covered_stock.0, StatusCode::OK, "{}", covered_stock.1);
    let submit = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "i07f-submit",
                "customer_code": "CUST-i07f",
                "action": "SUBMIT",
                "lines": [{
                    "line_no": 1,
                    "item_code": short_item,
                    "qty_micros": 1_000_000,
                    "price_cent": 100,
                    "discount_cent": 0,
                    "tax_cent": 0,
                    "due_date": "2026-10-20",
                    "address": "dock",
                    "source": "MAKE"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(submit.0, StatusCode::OK, "{}", submit.1);
    let submit_body: Value = serde_json::from_str(&submit.1).unwrap_or(Value::Null);
    let revision_id = submit_body["effect_id"].as_str().unwrap_or("").to_string();
    let so_id: uuid::Uuid = sqlx::query_scalar(
        "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&revision_id)?)
    .fetch_one(&pool)
    .await?;
    let fulfill = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "i07f-fulfill",
                "so_id": so_id,
                "customer_code": "CUST-i07f",
                "expected_version": 1,
                "action": "FULFILL",
                "lines": [{
                    "line_no": 1,
                    "item_code": short_item,
                    "qty_micros": 1_000_000,
                    "price_cent": 100,
                    "discount_cent": 0,
                    "tax_cent": 0,
                    "due_date": "2026-10-20",
                    "address": "dock",
                    "source": "MAKE"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(fulfill.0, StatusCode::OK, "{}", fulfill.1);
    let covered_submit = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "i07f-covered-submit",
                "customer_code": "CUST-i07f",
                "action": "SUBMIT",
                "lines": [{
                    "line_no": 1,
                    "item_code": covered_item,
                    "qty_micros": 250_000,
                    "price_cent": 100,
                    "discount_cent": 0,
                    "tax_cent": 0,
                    "due_date": "2026-10-20",
                    "address": "dock",
                    "source": "MAKE"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(covered_submit.0, StatusCode::OK, "{}", covered_submit.1);
    let covered_body: Value = serde_json::from_str(&covered_submit.1).unwrap_or(Value::Null);
    let covered_revision = covered_body["effect_id"].as_str().unwrap_or("").to_string();
    let covered_so: uuid::Uuid = sqlx::query_scalar(
        "select so_id from shared_sales_revisions where tenant_id = $1 and revision_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&covered_revision)?)
    .fetch_one(&pool)
    .await?;
    let covered_fulfill = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/sales-revisions"),
        Some(
            serde_json::json!({
                "request_id": "i07f-covered-fulfill",
                "so_id": covered_so,
                "customer_code": "CUST-i07f",
                "expected_version": 1,
                "action": "FULFILL",
                "lines": [{
                    "line_no": 1,
                    "item_code": covered_item,
                    "qty_micros": 250_000,
                    "price_cent": 100,
                    "discount_cent": 0,
                    "tax_cent": 0,
                    "due_date": "2026-10-20",
                    "address": "dock",
                    "source": "MAKE"
                }]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(covered_fulfill.0, StatusCode::OK, "{}", covered_fulfill.1);
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shortage-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i07f-early",
                "item_code": covered_item,
                "sku_id": covered_sku
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(early.0, StatusCode::OK, "{}", early.1);
    let raised = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shortage-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i07f-suggest",
                "item_code": short_item,
                "sku_id": short_sku
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(raised.0, StatusCode::OK, "{}", raised.1);
    let raised_body: Value = serde_json::from_str(&raised.1).unwrap_or(Value::Null);
    let suggestion_id = raised_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!suggestion_id.is_empty());
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shortage-suggestions"),
        Some(
            serde_json::json!({
                "request_id": "i07f-again",
                "item_code": short_item,
                "sku_id": short_sku
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(again.0, StatusCode::OK, "{}", again.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/shortage-suggestions?item_code={short_item}&sku_id={short_sku}"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "SUGGEST_ONLY");
    assert_eq!(body["demand"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["shortage"].as_i64().unwrap_or(0), 750_000);
    assert_eq!(body["paid"].as_bool().unwrap_or(true), false);
    assert_eq!(body["work_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/shortage-suggestions?item_code={short_item}&sku_id={short_sku}"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&suggestion_id));
    let row: (i64, bool, String, i64, i64, i64) = sqlx::query_as(
        "select s.qty_micros, s.paid, a.state, a.threshold, a.observed,
            (select count(*) from command_receipts where tenant_id = $1 and request_id in ('i07f-early', 'i07f-again'))
         from shared_purchase_suggestions s
         join shared_inventory_alerts a on a.tenant_id = s.tenant_id and a.purchase_id = s.suggestion_id::text
         where s.tenant_id = $1 and s.suggestion_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&suggestion_id)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 750_000);
    assert!(!row.1);
    assert_eq!(row.2, "SUGGEST_ONLY");
    assert_eq!(row.3, 1_000_000);
    assert_eq!(row.4, 250_000);
    assert_eq!(row.5, 0);
    eprintln!(
        "shortage-suggestion-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={short_sku} suggestion={suggestion_id} qty={} paid={} state={} demand={} on_hand={} refused_receipts={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn aging_from_stored_receipt_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_inventory_alerts (
          tenant_id uuid not null,
          alert_id uuid not null,
          rule_code text not null,
          object_code text not null,
          cutoff_at text not null,
          reason text not null,
          observed bigint not null,
          threshold bigint not null,
          state text not null,
          notified boolean not null,
          purchase_id text not null default '',
          work_order_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, alert_id)
        )",
        "create table if not exists shared_purchase_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          item_code text not null,
          qty_micros bigint not null,
          paid boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let aged = Uuid::new_v4();
    let fresh = Uuid::new_v4();
    let aged_product = Uuid::new_v4();
    let fresh_product = Uuid::new_v4();
    eprintln!("aging-alert tenant={owner} foreign={foreign} actor={owner_actor} sku={aged}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i07e-owner"),
        (foreign, foreign_actor, "i07e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(aged_product)
        .bind("i07e-aged")
        .execute(&pool)
        .await?;
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(fresh_product)
        .bind("i07e-fresh")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(aged)
    .bind(aged_product)
    .bind(format!("SKU-{aged}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(fresh)
    .bind(fresh_product)
    .bind(format!("SKU-{fresh}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i07e")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("aging-alert-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i07e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i07e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i07e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i07e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/aging-alerts?alert_id={owner}&sku_id={aged}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let fresh_post = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i07e-fresh",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": fresh,
                "batch_code": "B-FRESH",
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(fresh_post.0, StatusCode::OK, "{}", fresh_post.1);
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/aging-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07e-early",
                "sku_id": fresh,
                "cutoff": "2020-01-01"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(early.0, StatusCode::OK, "{}", early.1);
    let aged_post = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i07e-aged",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": aged,
                "batch_code": "B-AGED",
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(aged_post.0, StatusCode::OK, "{}", aged_post.1);
    let stored_day: String = sqlx::query_scalar(
        "select to_char(min(created_at) at time zone 'UTC', 'YYYY-MM-DD') from shared_stock_ledgers where tenant_id = $1 and sku_id = $2 and delta_micros > 0",
    )
    .bind(owner)
    .bind(aged)
    .fetch_one(&pool)
    .await?;
    let raised = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/aging-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07e-open",
                "sku_id": aged,
                "cutoff": stored_day
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(raised.0, StatusCode::OK, "{}", raised.1);
    let raised_body: Value = serde_json::from_str(&raised.1).unwrap_or(Value::Null);
    let alert_id = raised_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!alert_id.is_empty());
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/aging-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07e-again",
                "sku_id": aged,
                "cutoff": stored_day
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(again.0, StatusCode::OK, "{}", again.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/aging-alerts?alert_id={alert_id}&sku_id={aged}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "OPEN");
    assert_eq!(body["inbound_on"].as_str().unwrap_or(""), stored_day);
    assert_eq!(body["cutoff"].as_str().unwrap_or(""), stored_day);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["purchase_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["work_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/aging-alerts?alert_id={alert_id}&sku_id={aged}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&alert_id));
    let row: (String, String, String, i64, i64, i64, i64) = sqlx::query_as(
        "select a.state, a.cutoff_at, a.reason, a.observed,
            (select count(*) from shared_inventory_alerts where tenant_id = $1 and rule_code = 'AGING'),
            (select count(*) from shared_purchase_suggestions where tenant_id = $1),
            (select count(*) from command_receipts where tenant_id = $1 and request_id in ('i07e-early', 'i07e-again'))
         from shared_inventory_alerts a
         where a.tenant_id = $1 and a.alert_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&alert_id)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "OPEN");
    assert_eq!(row.1, stored_day);
    assert!(row.2.contains(&stored_day));
    assert_eq!(row.3, 250_000);
    assert_eq!(row.4, 1);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 0);
    eprintln!(
        "aging-alert-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={aged} alert={alert_id} state={} cutoff={} observed={} alerts={} purchases={} refused_receipts={}",
        row.0, row.1, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn expiring_from_stored_date_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_batches (
          tenant_id uuid not null,
          batch_id uuid not null,
          item_code text not null,
          batch_code text not null,
          produced_on text not null,
          expires_on text not null,
          frozen boolean not null default false,
          serialized boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, batch_id),
          unique (tenant_id, item_code, batch_code)
        )",
        "alter table shared_batches add column if not exists quality_status text not null default 'GOOD'",
        "create table if not exists shared_inventory_alerts (
          tenant_id uuid not null,
          alert_id uuid not null,
          rule_code text not null,
          object_code text not null,
          cutoff_at text not null,
          reason text not null,
          observed bigint not null,
          threshold bigint not null,
          state text not null,
          notified boolean not null,
          purchase_id text not null default '',
          work_order_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, alert_id)
        )",
        "create table if not exists shared_purchase_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          item_code text not null,
          qty_micros bigint not null,
          paid boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let item = format!("ITEM-{owner}");
    let near = format!("B-NEAR-{owner}");
    let later = format!("B-LATER-{owner}");
    eprintln!("expiring-alert tenant={owner} foreign={foreign} actor={owner_actor} item={item}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i07d-owner"),
        (foreign, foreign_actor, "i07d-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("expiring-alert-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i07d-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i07d",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i07d-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i07d",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/expiring-alerts?alert_id={owner}&item_code={item}&batch_code={near}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let later_batch = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-batches"),
        Some(
            serde_json::json!({
                "request_id": "i07d-later-batch",
                "item_code": item,
                "batch_code": later,
                "produced_on": "2026-09-01",
                "expires_on": "2027-09-01",
                "serialized": false,
                "quality_status": "GOOD"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(later_batch.0, StatusCode::OK, "{}", later_batch.1);
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/expiring-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07d-early",
                "item_code": item,
                "batch_code": later,
                "cutoff": "2026-10-05"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(early.0, StatusCode::OK, "{}", early.1);
    let near_batch = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-batches"),
        Some(
            serde_json::json!({
                "request_id": "i07d-near-batch",
                "item_code": item,
                "batch_code": near,
                "produced_on": "2026-09-01",
                "expires_on": "2026-10-04",
                "serialized": false,
                "quality_status": "GOOD"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(near_batch.0, StatusCode::OK, "{}", near_batch.1);
    let raised = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/expiring-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07d-open",
                "item_code": item,
                "batch_code": near,
                "cutoff": "2026-10-05"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(raised.0, StatusCode::OK, "{}", raised.1);
    let raised_body: Value = serde_json::from_str(&raised.1).unwrap_or(Value::Null);
    let alert_id = raised_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!alert_id.is_empty());
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/expiring-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07d-again",
                "item_code": item,
                "batch_code": near,
                "cutoff": "2026-10-05"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(again.0, StatusCode::OK, "{}", again.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/expiring-alerts?alert_id={alert_id}&item_code={item}&batch_code={near}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "OPEN");
    assert_eq!(body["expires_on"].as_str().unwrap_or(""), "2026-10-04");
    assert_eq!(body["cutoff"].as_str().unwrap_or(""), "2026-10-05");
    assert_eq!(body["produced_on"].as_str().unwrap_or(""), "2026-09-01");
    assert_eq!(body["purchase_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["work_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/expiring-alerts?alert_id={alert_id}&item_code={item}&batch_code={near}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&alert_id));
    let row: (String, String, String, String, i64, i64, i64) = sqlx::query_as(
        "select a.state, a.cutoff_at, a.reason, b.expires_on,
            (select count(*) from shared_inventory_alerts where tenant_id = $1 and rule_code = 'EXPIRING'),
            (select count(*) from shared_purchase_suggestions where tenant_id = $1),
            (select count(*) from command_receipts where tenant_id = $1 and request_id in ('i07d-early', 'i07d-again'))
         from shared_inventory_alerts a
         join shared_batches b on b.tenant_id = a.tenant_id and b.item_code = a.object_code and b.batch_code = $3
         where a.tenant_id = $1 and a.alert_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&alert_id)?)
    .bind(&near)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "OPEN");
    assert_eq!(row.1, "2026-10-05");
    assert!(row.2.contains("2026-10-04"));
    assert_eq!(row.3, "2026-10-04");
    assert_eq!(row.4, 1);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 0);
    eprintln!(
        "expiring-alert-ok tenant={owner} foreign={foreign} actor={owner_actor} item={item} alert={alert_id} state={} cutoff={} expires={} alerts={} purchases={} refused_receipts={}",
        row.0, row.1, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn stock_increase_clears_alert_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists shared_inventory_alerts (
          tenant_id uuid not null,
          alert_id uuid not null,
          rule_code text not null,
          object_code text not null,
          cutoff_at text not null,
          reason text not null,
          observed bigint not null,
          threshold bigint not null,
          state text not null,
          notified boolean not null,
          purchase_id text not null default '',
          work_order_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, alert_id)
        )",
        "create table if not exists shared_purchase_suggestions (
          tenant_id uuid not null,
          suggestion_id uuid not null,
          item_code text not null,
          qty_micros bigint not null,
          paid boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, suggestion_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let product = Uuid::new_v4();
    eprintln!("alert-clear tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i07c-owner"),
        (foreign, foreign_actor, "i07c-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i07c")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i07c")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("alert-clear-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i07c-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i07c",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i07c-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i07c",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let object = format!("ITEM-{owner}");
    let first = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i07c-short",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": "B-GOOD",
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let raised = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/inventory-alerts"),
        Some(
            serde_json::json!({
                "request_id": "i07c-open",
                "rule_code": "LOW_STOCK",
                "object_code": object,
                "cutoff_at": "2026-10-05T00:00:00Z",
                "reason": "below safety",
                "observed": 250_000,
                "threshold": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(raised.0, StatusCode::OK, "{}", raised.1);
    let raised_body: Value = serde_json::from_str(&raised.1).unwrap_or(Value::Null);
    let alert_id = raised_body["effect_id"].as_str().unwrap_or("").to_string();
    let early = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/alert-clears"),
        Some(
            serde_json::json!({
                "request_id": "i07c-early",
                "alert_id": alert_id,
                "sku_id": sku
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(early.0, StatusCode::OK, "{}", early.1);
    let second = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i07c-fill",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": "B-FILL",
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 750_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(second.0, StatusCode::OK, "{}", second.1);
    let cleared = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/alert-clears"),
        Some(
            serde_json::json!({
                "request_id": "i07c-clear",
                "alert_id": alert_id,
                "sku_id": sku
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(cleared.0, StatusCode::OK, "{}", cleared.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/alert-clears?alert_id={alert_id}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["status"].as_str().unwrap_or(""), "CLEAR");
    assert_eq!(body["observed"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["purchase_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/alert-clears?alert_id={alert_id}&sku_id={sku}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&alert_id));
    let row: (String, i64, String, i64, i64, i64) = sqlx::query_as(
        "select state, observed, reason,
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3 and quality_status = 'GOOD'),
            (select count(*) from shared_purchase_suggestions where tenant_id = $1),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i07c-early')
         from shared_inventory_alerts where tenant_id = $1 and alert_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&alert_id)?)
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "CLEAR");
    assert_eq!(row.1, 1_000_000);
    assert!(row.2.contains("stock recovered"));
    assert_eq!(row.3, 1_000_000);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    eprintln!(
        "alert-clear-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} alert={alert_id} state={} observed={} on_hand={} purchases={} early_receipts={}",
        row.0, row.1, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn closed_period_adjustment_keeps_original_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_item_costs (
          tenant_id uuid not null,
          entry_id uuid not null,
          item_code text not null,
          kind text not null,
          qty_micros bigint not null,
          amount_cent bigint not null,
          unit_cent bigint not null,
          estimated boolean not null default false,
          algorithm text not null,
          source_ref text not null default '',
          period_code text not null,
          entry_sequence bigint generated always as identity,
          created_at timestamptz not null default now(),
          primary key (tenant_id, entry_id)
        )",
        "create table if not exists shared_cost_periods (
          tenant_id uuid not null,
          period_code text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, period_code)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("closed-adjust tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i05j-owner"),
        (foreign, foreign_actor, "i05j-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("closed-adjust-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i05j-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i05j",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i05j-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i05j",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let item = format!("ITEM-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-period-adjustments?item_code={item}&closed_period=2026-10&next_period=2026-11"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/cost-receipts"),
        Some(
            serde_json::json!({
                "request_id": "i05j-receive",
                "item_code": item,
                "qty_micros": 1_000_000,
                "amount_cent": 2000,
                "period_code": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let received_body: Value = serde_json::from_str(&received.1).unwrap_or(Value::Null);
    let original_id = received_body["effect_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let closed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/cost-period-closes"),
        Some(serde_json::json!({"request_id": "i05j-close", "period_code": "2026-10"}).to_string()),
    )
    .await?;
    assert_eq!(closed.0, StatusCode::OK, "{}", closed.1);
    let inside = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/cost-period-adjustments"),
        Some(
            serde_json::json!({
                "request_id": "i05j-inside",
                "item_code": item,
                "amount_cent": 300,
                "closed_period": "2026-10",
                "next_period": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(inside.0, StatusCode::OK, "{}", inside.1);
    let adjusted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/cost-period-adjustments"),
        Some(
            serde_json::json!({
                "request_id": "i05j-adjust",
                "item_code": item,
                "amount_cent": 300,
                "closed_period": "2026-10",
                "next_period": "2026-11"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(adjusted.0, StatusCode::OK, "{}", adjusted.1);
    let adjusted_body: Value = serde_json::from_str(&adjusted.1).unwrap_or(Value::Null);
    let adjustment_id = adjusted_body["effect_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(!adjustment_id.is_empty(), "{}", adjusted.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-period-adjustments?item_code={item}&closed_period=2026-10&next_period=2026-11"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["original_cent"].as_i64().unwrap_or(0), 2000);
    assert_eq!(body["adjustment_cent"].as_i64().unwrap_or(0), 300);
    assert_eq!(body["period_code"].as_str().unwrap_or(""), "2026-11");
    assert!(!body["original_rewritten"].as_bool().unwrap_or(true));
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-period-adjustments?item_code={item}&closed_period=2026-10&next_period=2026-11"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&item));
    let row: (i64, String, i64, String, i64, i64) = sqlx::query_as(
        "select
            (select amount_cent from shared_item_costs where tenant_id = $1 and kind = 'RECEIPT'),
            (select period_code from shared_item_costs where tenant_id = $1 and kind = 'RECEIPT'),
            (select amount_cent from shared_item_costs where tenant_id = $1 and kind = 'ADJUST'),
            (select period_code from shared_item_costs where tenant_id = $1 and kind = 'ADJUST'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i05j-inside'),
            (select count(*) from shared_item_costs where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 2000);
    assert_eq!(row.1, "2026-10");
    assert_eq!(row.2, 300);
    assert_eq!(row.3, "2026-11");
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    eprintln!(
        "closed-adjust-ok tenant={owner} foreign={foreign} actor={owner_actor} item={item} original={original_id} adjustment={adjustment_id} original_cent={} adjust_cent={} next={} inside_receipts={} foreign_rows={}",
        row.0, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn zero_unit_cost_legal_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_item_costs (
          tenant_id uuid not null,
          entry_id uuid not null,
          item_code text not null,
          kind text not null,
          qty_micros bigint not null,
          amount_cent bigint not null,
          unit_cent bigint not null,
          estimated boolean not null default false,
          algorithm text not null,
          source_ref text not null default '',
          period_code text not null,
          entry_sequence bigint generated always as identity,
          created_at timestamptz not null default now(),
          primary key (tenant_id, entry_id)
        )",
        "create table if not exists shared_cost_periods (
          tenant_id uuid not null,
          period_code text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, period_code)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("zero-unit tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i05i-owner"),
        (foreign, foreign_actor, "i05i-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("zero-unit-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i05i-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i05i",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i05i-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i05i",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let item = format!("ITEM-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-receipts?item_code={item}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/cost-receipts"),
        Some(
            serde_json::json!({
                "request_id": "i05i-receive",
                "item_code": item,
                "qty_micros": 1_000_000,
                "amount_cent": 0,
                "period_code": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let received_body: Value = serde_json::from_str(&received.1).unwrap_or(Value::Null);
    let entry_id = received_body["effect_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(!entry_id.is_empty(), "{}", received.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-receipts?item_code={item}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["qty_micros"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["value_cent"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["unit_cent"].as_i64().unwrap_or(-1), 0);
    assert!(!body["estimated"].as_bool().unwrap_or(true));
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-receipts?item_code={item}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&item));
    let row: (i64, i64, i64, bool, i64) = sqlx::query_as(
        "select qty_micros, amount_cent, unit_cent, estimated,
            (select count(*) from shared_item_costs where tenant_id = $2)
         from shared_item_costs where tenant_id = $1 and kind = 'RECEIPT'",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 1_000_000);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 0);
    assert!(!row.3);
    assert_eq!(row.4, 0);
    eprintln!(
        "zero-unit-ok tenant={owner} foreign={foreign} actor={owner_actor} item={item} entry={entry_id} qty={} value={} unit={} estimated={} foreign_rows={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn three_micro_return_exact_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_item_costs (
          tenant_id uuid not null,
          entry_id uuid not null,
          item_code text not null,
          kind text not null,
          qty_micros bigint not null,
          amount_cent bigint not null,
          unit_cent bigint not null,
          estimated boolean not null default false,
          algorithm text not null,
          source_ref text not null default '',
          period_code text not null,
          entry_sequence bigint generated always as identity,
          created_at timestamptz not null default now(),
          primary key (tenant_id, entry_id)
        )",
        "create table if not exists shared_cost_periods (
          tenant_id uuid not null,
          period_code text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, period_code)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("three-micro tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i05h-owner"),
        (foreign, foreign_actor, "i05h-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("three-micro-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i05h-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i05h",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i05h-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i05h",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let item = format!("ITEM-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-returns?item_code={item}&issue_request=i05h-issue"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/cost-receipts"),
        Some(
            serde_json::json!({
                "request_id": "i05h-receive",
                "item_code": item,
                "qty_micros": 3,
                "amount_cent": 2,
                "period_code": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/cost-issues"),
        Some(
            serde_json::json!({
                "request_id": "i05h-issue",
                "item_code": item,
                "qty_micros": 3,
                "period_code": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let issued_body: Value = serde_json::from_str(&issued.1).unwrap_or(Value::Null);
    let issue_id = issued_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!issue_id.is_empty(), "{}", issued.1);
    for index in 1..=3 {
        let returned = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/cost-returns"),
            Some(
                serde_json::json!({
                    "request_id": format!("i05h-return-{index}"),
                    "item_code": item,
                    "issue_request": "i05h-issue",
                    "qty_micros": 1,
                    "period_code": "2026-10"
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(returned.0, StatusCode::OK, "{}", returned.1);
    }
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-returns?item_code={item}&issue_request=i05h-issue"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["first_cent"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["second_cent"].as_i64().unwrap_or(0), 0);
    assert_eq!(body["third_cent"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["original_cent"].as_i64().unwrap_or(0), 2);
    assert_eq!(body["returned_cent"].as_i64().unwrap_or(0), 2);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/cost-returns?item_code={item}&issue_request=i05h-issue"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&item));
    let row: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select amount_cent from shared_item_costs where tenant_id = $1 and kind = 'ISSUE' and source_ref = 'i05h-issue'),
            (select amount_cent from shared_item_costs where tenant_id = $1 and kind = 'RETURN' and source_ref = 'i05h-issue' order by entry_sequence offset 0 limit 1),
            (select amount_cent from shared_item_costs where tenant_id = $1 and kind = 'RETURN' and source_ref = 'i05h-issue' order by entry_sequence offset 1 limit 1),
            (select amount_cent from shared_item_costs where tenant_id = $1 and kind = 'RETURN' and source_ref = 'i05h-issue' order by entry_sequence offset 2 limit 1),
            (select count(*) from shared_item_costs where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 2);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 0);
    eprintln!(
        "three-micro-ok tenant={owner} foreign={foreign} actor={owner_actor} item={item} issue={issue_id} original={} first={} second={} third={} foreign_rows={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn serial_twenty_way_issue_at_most_once_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_batches (
          tenant_id uuid not null,
          batch_id uuid not null,
          item_code text not null,
          batch_code text not null,
          produced_on text not null,
          expires_on text not null,
          frozen boolean not null default false,
          serialized boolean not null default false,
          quality_status text not null default 'GOOD',
          created_at timestamptz not null default now(),
          primary key (tenant_id, batch_id),
          unique (tenant_id, item_code, batch_code)
        )",
        "alter table shared_batches add column if not exists quality_status text not null default 'GOOD'",
        "create table if not exists shared_serials (
          tenant_id uuid not null,
          serial_id uuid not null,
          item_code text not null,
          batch_code text not null,
          serial_no text not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, serial_id),
          unique (tenant_id, item_code, serial_no)
        )",
        "alter table shared_serials add column if not exists status text not null default 'OPEN'",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("serial-race tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i04k-owner"),
        (foreign, foreign_actor, "i04k-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i04k")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i04k")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("serial-race-test".into()));
    let app = build_app(AppState::new(Db::from_pool(pool.clone()), jwt));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i04k-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i04k",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i04k-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i04k",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let item = format!("ITEM-{owner}");
    let batch = format!("B-SN-{owner}");
    let serial = format!("SN-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/serial-issues?item_code={item}&serial_no={serial}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-batches"),
        Some(
            serde_json::json!({
                "request_id": "i04k-save",
                "item_code": item,
                "batch_code": batch,
                "produced_on": "2026-09-01",
                "expires_on": "2027-09-01",
                "serialized": true,
                "quality_status": "GOOD"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let assigned = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-serials"),
        Some(
            serde_json::json!({
                "request_id": "i04k-assign",
                "item_code": item,
                "batch_code": batch,
                "serial_no": serial
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(assigned.0, StatusCode::OK, "{}", assigned.1);
    let assigned_body: Value = serde_json::from_str(&assigned.1).unwrap_or(Value::Null);
    let serial_id = assigned_body["effect_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(!serial_id.is_empty(), "{}", assigned.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i04k-receive",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let mut tasks = JoinSet::new();
    for index in 0..20 {
        let app = app.clone();
        let token = owner_token.clone();
        let item = item.clone();
        let batch = batch.clone();
        let serial = serial.clone();
        tasks.spawn(async move {
            let request_id = format!("i04k-issue-{index}");
            let (status, body) = export_http(
                &app,
                &token,
                "POST",
                &format!("/v1/tenants/{owner}/serial-issues"),
                Some(
                    serde_json::json!({
                        "request_id": request_id,
                        "warehouse_id": warehouse,
                        "location_id": location,
                        "sku_id": sku,
                        "item_code": item,
                        "batch_code": batch,
                        "serial_no": serial,
                        "as_of": "2026-10-05"
                    })
                    .to_string(),
                ),
            )
            .await
            .expect("serial issue response");
            (index, status, body)
        });
    }
    let mut admitted = 0;
    let mut rejected = 0;
    let mut ledger = String::new();
    while let Some(joined) = tasks.join_next().await {
        let (index, status, body) = joined?;
        eprintln!(
            "serial-race-result index={index} status={} body={body}",
            status.as_u16()
        );
        if status == StatusCode::OK {
            admitted += 1;
            let parsed: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            assert!(!parsed["replay"].as_bool().unwrap_or(true), "{body}");
            ledger = parsed["effect_id"].as_str().unwrap_or("").to_string();
        } else if status == StatusCode::CONFLICT {
            rejected += 1;
        } else {
            anyhow::bail!("unexpected serial issue {status} {body}");
        }
    }
    assert_eq!(admitted, 1, "one serial can leave stock only once");
    assert_eq!(rejected, 19);
    assert!(!ledger.is_empty());
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/serial-issues?item_code={item}&serial_no={serial}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["serial_id"].as_str().unwrap_or(""), serial_id);
    assert_eq!(body["successes"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["issue_rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["status"].as_str().unwrap_or(""), "ISSUED");
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/serial-issues?item_code={item}&serial_no={serial}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&serial_id));
    let row: (String, String, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select serial_id::text from shared_serials where tenant_id = $1 and serial_no = $3),
            (select status from shared_serials where tenant_id = $1 and serial_no = $3),
            (select count(*) from command_receipts where tenant_id = $1 and status = 'SUCCEEDED' and request_id like 'i04k-issue-%'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and source_line = $3 and delta_micros < 0),
            (select -delta_micros from shared_stock_ledgers where tenant_id = $1 and source_line = $3 and delta_micros < 0),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and batch_code = $4 and bucket = 'ON_HAND'),
            (select count(*) from shared_serials where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&serial)
    .bind(&batch)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, serial_id);
    assert_eq!(row.1, "ISSUED");
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 1_000_000);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 0);
    eprintln!(
        "serial-race-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} serial={} status={} successes={} rows={} issued={} on_hand={} foreign_serials={}",
        row.0, row.1, row.2, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn batch_quarter_issue_exact_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_batches (
          tenant_id uuid not null,
          batch_id uuid not null,
          item_code text not null,
          batch_code text not null,
          produced_on text not null,
          expires_on text not null,
          frozen boolean not null default false,
          serialized boolean not null default false,
          quality_status text not null default 'GOOD',
          created_at timestamptz not null default now(),
          primary key (tenant_id, batch_id),
          unique (tenant_id, item_code, batch_code)
        )",
        "alter table shared_batches add column if not exists quality_status text not null default 'GOOD'",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("batch-quarter tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i04j-owner"),
        (foreign, foreign_actor, "i04j-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i04j")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i04j")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("batch-quarter-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i04j-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i04j",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i04j-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i04j",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let item = format!("ITEM-{owner}");
    let batch = format!("B-Q-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/batch-quarter-issues?request_id=i04j-issue&batch_code={batch}"
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-batches"),
        Some(
            serde_json::json!({
                "request_id": "i04j-save",
                "item_code": item,
                "batch_code": batch,
                "produced_on": "2026-09-01",
                "expires_on": "2027-09-01",
                "serialized": false,
                "quality_status": "GOOD"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i04j-receive",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/batch-quarter-issues"),
        Some(
            serde_json::json!({
                "request_id": "i04j-issue",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "item_code": item,
                "batch_code": batch,
                "as_of": "2026-10-05"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let issued_body: Value = serde_json::from_str(&issued.1).unwrap_or(Value::Null);
    let ledger = issued_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!ledger.is_empty(), "{}", issued.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/batch-quarter-issues?request_id=i04j-issue&batch_code={batch}"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["ledger_id"].as_str().unwrap_or(""), ledger);
    assert_eq!(body["issued_micros"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["rows"].as_i64().unwrap_or(0), 1);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/batch-quarter-issues?request_id=i04j-issue&batch_code={batch}"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&ledger));
    let row: (String, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select ledger_id::text from shared_stock_ledgers where tenant_id = $1 and request_id = 'i04j-issue'),
            (select -delta_micros from shared_stock_ledgers where tenant_id = $1 and request_id = 'i04j-issue'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and request_id = 'i04j-issue'),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and batch_code = $3 and bucket = 'ON_HAND'),
            (select count(*) from shared_stock_ledgers where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&batch)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, ledger);
    assert_eq!(row.1, 250_000);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 750_000);
    assert_eq!(row.4, 0);
    eprintln!(
        "batch-quarter-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} batch={batch} ledger={} issued={} rows={} on_hand={} foreign_ledgers={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn expired_batch_refuses_ordinary_issue_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_batches (
          tenant_id uuid not null,
          batch_id uuid not null,
          item_code text not null,
          batch_code text not null,
          produced_on text not null,
          expires_on text not null,
          frozen boolean not null default false,
          serialized boolean not null default false,
          quality_status text not null default 'GOOD',
          created_at timestamptz not null default now(),
          primary key (tenant_id, batch_id),
          unique (tenant_id, item_code, batch_code)
        )",
        "alter table shared_batches add column if not exists quality_status text not null default 'GOOD'",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("expired-batch tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i04i-owner"),
        (foreign, foreign_actor, "i04i-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("expired-batch-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i04i-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i04i",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i04i-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i04i",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let item = format!("ITEM-{owner}");
    let expired = format!("B-EXP-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/batch-issues?item_code={item}&batch_code={expired}&as_of=2026-10-05"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-batches"),
        Some(
            serde_json::json!({
                "request_id": "i04i-save",
                "item_code": item,
                "batch_code": expired,
                "produced_on": "2026-09-01",
                "expires_on": "2026-10-01",
                "serialized": false,
                "quality_status": "GOOD"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/batch-issues"),
        Some(
            serde_json::json!({
                "request_id": "i04i-issue",
                "item_code": item,
                "batch_code": expired,
                "as_of": "2026-10-05",
                "qty_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::CONFLICT, "{}", issued.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/batch-issues?item_code={item}&batch_code={expired}&as_of=2026-10-05"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["reason"].as_str().unwrap_or(""), "EXPIRED");
    assert_eq!(body["quality_status"].as_str().unwrap_or(""), "GOOD");
    assert_eq!(body["expires_on"].as_str().unwrap_or(""), "2026-10-01");
    assert_eq!(body["frozen"].as_bool().unwrap_or(true), false);
    assert_eq!(body["receipts"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["stock_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/batch-issues?item_code={item}&batch_code={expired}&as_of=2026-10-05"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&expired));
    let row: (String, String, String, bool, i64, i64, i64) = sqlx::query_as(
        "select
            (select reason from shared_batch_issue_refusals where tenant_id = $1 and batch_code = $3),
            (select quality_status from shared_batches where tenant_id = $1 and batch_code = $3),
            (select expires_on from shared_batches where tenant_id = $1 and batch_code = $3),
            (select frozen from shared_batches where tenant_id = $1 and batch_code = $3),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i04i-issue'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and batch_code = $3),
            (select count(*) from shared_batches where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&expired)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "EXPIRED");
    assert_eq!(row.1, "GOOD");
    assert_eq!(row.2, "2026-10-01");
    assert_eq!(row.3, false);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    assert_eq!(row.6, 0);
    eprintln!(
        "expired-batch-ok tenant={owner} foreign={foreign} actor={owner_actor} batch={expired} reason={} quality={} expires={} frozen={} receipts={} stock={} foreign_batches={}",
        row.0, row.1, row.2, row.3, row.4, row.5, row.6
    );
    Ok(())
}

#[tokio::test]
async fn held_batch_refuses_ordinary_issue_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_batches (
          tenant_id uuid not null,
          batch_id uuid not null,
          item_code text not null,
          batch_code text not null,
          produced_on text not null,
          expires_on text not null,
          frozen boolean not null default false,
          serialized boolean not null default false,
          quality_status text not null default 'GOOD',
          created_at timestamptz not null default now(),
          primary key (tenant_id, batch_id),
          unique (tenant_id, item_code, batch_code)
        )",
        "alter table shared_batches add column if not exists quality_status text not null default 'GOOD'",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("held-batch tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i04h-owner"),
        (foreign, foreign_actor, "i04h-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("held-batch-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i04h-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i04h",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i04h-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i04h",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let item = format!("ITEM-{owner}");
    let held = format!("B-HOLD-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/batch-issues?item_code={item}&batch_code={held}&as_of=2026-10-05"
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-batches"),
        Some(
            serde_json::json!({
                "request_id": "i04h-save",
                "item_code": item,
                "batch_code": held,
                "produced_on": "2026-09-01",
                "expires_on": "2027-09-01",
                "serialized": false,
                "quality_status": "HOLD"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/batch-issues"),
        Some(
            serde_json::json!({
                "request_id": "i04h-issue",
                "item_code": item,
                "batch_code": held,
                "as_of": "2026-10-05",
                "qty_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::CONFLICT, "{}", issued.1);
    assert!(issued.1.contains("held"));
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/batch-issues?item_code={item}&batch_code={held}&as_of=2026-10-05"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["reason"].as_str().unwrap_or(""), "HELD");
    assert_eq!(body["quality_status"].as_str().unwrap_or(""), "HOLD");
    assert_eq!(body["frozen"].as_bool().unwrap_or(true), false);
    assert_eq!(body["receipts"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["stock_rows"].as_i64().unwrap_or(-1), 0);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/batch-issues?item_code={item}&batch_code={held}&as_of=2026-10-05"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&held));
    let row: (String, String, bool, i64, i64, i64) = sqlx::query_as(
        "select
            (select reason from shared_batch_issue_refusals where tenant_id = $1 and batch_code = $3),
            (select quality_status from shared_batches where tenant_id = $1 and batch_code = $3),
            (select frozen from shared_batches where tenant_id = $1 and batch_code = $3),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i04h-issue'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and batch_code = $3),
            (select count(*) from shared_batches where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&held)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "HELD");
    assert_eq!(row.1, "HOLD");
    assert_eq!(row.2, false);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    eprintln!(
        "held-batch-ok tenant={owner} foreign={foreign} actor={owner_actor} batch={held} reason={} quality={} frozen={} receipts={} stock={} foreign_batches={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn issue_retry_adds_zero_ledger_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("issue-retry tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i02i-owner"),
        (foreign, foreign_actor, "i02i-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i02i")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i02i")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("issue-retry-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i02i-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i02i",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i02i-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i02i",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let source = format!("SO-RETRY-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-issue-retries?request_id=i02i-issue&source_line={source}&first_ledger={sku}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i02i-receive",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let issue_body = serde_json::json!({
        "request_id": "i02i-issue",
        "warehouse_id": warehouse,
        "location_id": location,
        "sku_id": sku,
        "batch_code": batch,
        "quality_status": "GOOD",
        "source_line": source,
        "qty_micros": 250_000
    })
    .to_string();
    let first = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-issues"),
        Some(issue_body.clone()),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let first_body: Value = serde_json::from_str(&first.1).unwrap_or(Value::Null);
    assert_eq!(first_body["replay"].as_bool().unwrap_or(true), false);
    let first_ledger = first_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!first_ledger.is_empty(), "{}", first.1);
    let retry = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-issues"),
        Some(issue_body),
    )
    .await?;
    assert_eq!(retry.0, StatusCode::OK, "{}", retry.1);
    let retry_body: Value = serde_json::from_str(&retry.1).unwrap_or(Value::Null);
    assert_eq!(retry_body["replay"].as_bool().unwrap_or(false), true);
    assert_eq!(retry_body["effect_id"].as_str().unwrap_or(""), first_ledger);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-issue-retries?request_id=i02i-issue&source_line={source}&first_ledger={first_ledger}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["ledger_id"].as_str().unwrap_or(""), first_ledger);
    assert_eq!(body["ledger_rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["receipt_rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(body["issued_micros"].as_i64().unwrap_or(0), 250_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-issue-retries?request_id=i02i-issue&source_line={source}&first_ledger={first_ledger}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&first_ledger));
    let row: (String, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select ledger_id::text from shared_stock_ledgers where tenant_id = $1 and request_id = 'i02i-issue'),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and request_id = 'i02i-issue'),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'i02i-issue' and status = 'SUCCEEDED'),
            (select coalesce(sum(-delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and request_id = 'i02i-issue'),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and bucket = 'ON_HAND'),
            (select count(*) from shared_stock_ledgers where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, first_ledger);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 250_000);
    assert_eq!(row.4, 750_000);
    assert_eq!(row.5, 0);
    eprintln!(
        "issue-retry-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} ledger={} rows={} receipts={} issued={} on_hand={} foreign_ledgers={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn short_issue_conflict_rolls_stock_back_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("short-issue tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i02h-owner"),
        (foreign, foreign_actor, "i02h-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i02h")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i02h")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("short-issue-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i02h-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i02h",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i02h-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i02h",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let source = format!("SO-SHORT-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-issues?source_line={source}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i02h-receive",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let short = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-issues"),
        Some(
            serde_json::json!({
                "request_id": source,
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "source_line": source,
                "qty_micros": 400_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(short.0, StatusCode::CONFLICT, "{}", short.1);
    assert!(!short.1.contains("500"));
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-issues?source_line={source}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["source_status"].as_str().unwrap_or(""), "OPEN");
    assert_eq!(body["stock_rows"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["receipts"].as_i64().unwrap_or(-1), 0);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["requested"].as_i64().unwrap_or(0), 400_000);
    assert_eq!(body["available"].as_i64().unwrap_or(-1), 250_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-issues?source_line={source}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&source));
    let row: (String, i64, i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select status from shared_issue_sources where tenant_id = $1 and source_line = $3),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and source_line = $3),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = $3 and status = 'SUCCEEDED'),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and bucket = 'ON_HAND'),
            (select requested_micros from shared_issue_refusals where tenant_id = $1 and source_line = $3),
            (select count(*) from shared_issue_sources where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&source)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "OPEN");
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 250_000);
    assert_eq!(row.4, 400_000);
    assert_eq!(row.5, 0);
    eprintln!(
        "short-issue-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} status={} stock_rows={} receipts={} on_hand={} requested={} foreign_sources={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn release_keeps_other_source_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("other-source tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i02g-owner"),
        (foreign, foreign_actor, "i02g-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i02g")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i02g")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("other-source-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i02g-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i02g",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i02g-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i02g",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let source_x = format!("SO-X-{owner}");
    let source_y = format!("SO-Y-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-releases?source_x={source_x}&source_y={source_y}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i02g-receive",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    for (request_id, source, qty) in [
        ("i02g-x", &source_x, 250_000_i64),
        ("i02g-y", &source_y, 400_000_i64),
    ] {
        let reserved = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-reservations"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch,
                    "quality_status": "GOOD",
                    "source_line": source,
                    "qty_micros": qty
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(reserved.0, StatusCode::OK, "{}", reserved.1);
    }
    let released = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-releases"),
        Some(
            serde_json::json!({
                "request_id": "i02g-release",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "source_line": source_x,
                "qty_micros": 250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(released.0, StatusCode::OK, "{}", released.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-releases?source_x={source_x}&source_y={source_y}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["released_x"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["open_y"].as_i64().unwrap_or(0), 400_000);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 1_000_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-releases?source_x={source_x}&source_y={source_y}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&source_y));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and source_line = $3 and bucket = 'RESERVED'),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and source_line = $4 and bucket = 'RESERVED'),
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and bucket = 'ON_HAND'),
            (select count(*) from shared_stock_ledgers where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&source_x)
    .bind(&source_y)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 0);
    assert_eq!(row.1, 400_000);
    assert_eq!(row.2, 1_000_000);
    assert_eq!(row.3, 0);
    eprintln!(
        "other-source-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} x={} y={} on_hand={} foreign_rows={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn tenth_after_two_nines_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("tenth-remainder tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i01d-owner"),
        (foreign, foreign_actor, "i01d-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i01d")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i01d")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("tenth-remainder-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i01d-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i01d",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i01d-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i01d",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-tenths?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    for (request_id, delta) in [
        ("i01d-receive", 1_900_000_i64),
        ("i01d-first", -900_000),
        ("i01d-second", -900_000),
    ] {
        let posted = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/stock-ledgers"),
            Some(
                serde_json::json!({
                    "request_id": request_id,
                    "warehouse_id": warehouse,
                    "location_id": location,
                    "sku_id": sku,
                    "batch_code": batch,
                    "quality_status": "GOOD",
                    "bucket": "ON_HAND",
                    "delta_micros": delta
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    }
    let third = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i01d-third",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": -900_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(third.0, StatusCode::OK, "{}", third.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-tenths?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["received"].as_i64().unwrap_or(0), 1_900_000);
    assert_eq!(body["first"].as_i64().unwrap_or(0), 900_000);
    assert_eq!(body["second"].as_i64().unwrap_or(0), 900_000);
    assert_eq!(body["left"].as_i64().unwrap_or(0), 100_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-tenths?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&sku.to_string()));
    let row: (i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and sku_id = $3),
            (select count(*) from shared_stock_ledgers where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 100_000);
    assert_eq!(row.1, 3);
    assert_eq!(row.2, 0);
    eprintln!(
        "tenth-remainder-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} left={} rows={} foreign_rows={}",
        row.0, row.1, row.2
    );
    Ok(())
}

#[tokio::test]
async fn issue_leaves_sellable_remainder_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_stock_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          warehouse_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          batch_code text not null,
          quality_status text not null,
          bucket text not null,
          delta_micros bigint not null,
          source_line text not null default '',
          request_id text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let location = Uuid::new_v4();
    let product = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("issue-remainder tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "i01c-owner"),
        (foreign, foreign_actor, "i01c-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into products(tenant_id, product_id, name) values ($1, $2, $3)")
        .bind(owner)
        .bind(product)
        .bind("i01c")
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into skus(tenant_id, sku_id, product_id, sku_code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(sku)
    .bind(product)
    .bind(format!("SKU-{sku}"))
    .execute(&pool)
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, $3, $4)",
    )
    .bind(owner)
    .bind(warehouse)
    .bind("i01c")
    .bind(warehouse.to_string())
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("issue-remainder-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("i01c-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-i01c",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("i01c-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-i01c",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let batch = format!("B-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-ledgers?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let received = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i01c-receive",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": 1_000_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(received.0, StatusCode::OK, "{}", received.1);
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i01c-issue",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": -250_000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let over = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/stock-ledgers"),
        Some(
            serde_json::json!({
                "request_id": "i01c-over",
                "warehouse_id": warehouse,
                "location_id": location,
                "sku_id": sku,
                "batch_code": batch,
                "quality_status": "GOOD",
                "bucket": "ON_HAND",
                "delta_micros": -750_001
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(over.0, StatusCode::OK, "{}", over.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-ledgers?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["on_hand"].as_i64().unwrap_or(0), 1_000_000);
    assert_eq!(body["issued"].as_i64().unwrap_or(0), 250_000);
    assert_eq!(body["sellable"].as_i64().unwrap_or(0), 750_000);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/stock-ledgers?warehouse_id={warehouse}&sku_id={sku}&batch_code={batch}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&sku.to_string()));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3 and delta_micros > 0),
            (select coalesce(sum(-delta_micros), 0)::bigint from shared_stock_ledgers where tenant_id = $1 and sku_id = $3 and delta_micros < 0),
            (select count(*) from shared_stock_ledgers where tenant_id = $1 and sku_id = $3),
            (select count(*) from shared_stock_ledgers where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(sku)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 1_000_000);
    assert_eq!(row.1, 250_000);
    assert_eq!(row.2, 2);
    assert_eq!(row.3, 0);
    eprintln!(
        "issue-remainder-ok tenant={owner} foreign={foreign} actor={owner_actor} sku={sku} received={} issued={} rows={} foreign_rows={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn revoked_attachment_grant_refuses_download_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_attachments (
          tenant_id uuid not null,
          attachment_id uuid not null,
          object_id text not null,
          file_name text not null,
          version_no integer not null,
          byte_count bigint not null,
          digest text not null,
          storage_key text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, attachment_id)
        )",
        "create table if not exists shared_attachment_grants (
          tenant_id uuid not null,
          attachment_id uuid not null,
          actor_user_id text not null,
          revoked boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, attachment_id, actor_user_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("attachment-revoke tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d08f-owner"),
        (foreign, foreign_actor, "d08f-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("attachment-revoke-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d08f-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d08f",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d08f-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d08f",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let actor = format!("actor-{owner_actor}");
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-attachments"),
        Some(
            serde_json::json!({
                "request_id": "d08f-save",
                "object_id": format!("ITEM-{owner}"),
                "file_name": "drawing.pdf",
                "digest": format!("digest-{owner}"),
                "byte_count": 2048,
                "actor_user_id": actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let attachment_id = saved_body["effect_id"]
        .as_str()
        .unwrap_or("")
        .split(',')
        .next()
        .unwrap_or("")
        .to_string();
    assert!(!attachment_id.is_empty(), "{}", saved.1);
    let now_unix = 1_760_000_000_i64;
    let linked = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/attachment-downloads"),
        Some(
            serde_json::json!({
                "request_id": "d08f-link",
                "attachment_id": attachment_id,
                "actor_user_id": actor,
                "expires_at": now_unix + 3600,
                "now_unix": now_unix
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(linked.0, StatusCode::OK, "{}", linked.1);
    let before = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/attachment-downloads?attachment_id={attachment_id}&actor_user_id={actor}&now_unix={now_unix}"),
        None,
    )
    .await?;
    assert_eq!(before.0, StatusCode::OK, "{}", before.1);
    let before_body: Value = serde_json::from_str(&before.1).unwrap_or(Value::Null);
    let key = before_body["storage_key"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(key.starts_with(&format!("{owner}/")), "{}", before.1);
    let revoked = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/attachment-revokes"),
        Some(
            serde_json::json!({
                "request_id": "d08f-revoke",
                "attachment_id": attachment_id,
                "actor_user_id": actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(revoked.0, StatusCode::OK, "{}", revoked.1);
    let revoked_body: Value = serde_json::from_str(&revoked.1).unwrap_or(Value::Null);
    assert_eq!(revoked_body["effect_id"].as_str().unwrap_or(""), "REVOKED");
    let after = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/attachment-downloads?attachment_id={attachment_id}&actor_user_id={actor}&now_unix={now_unix}"),
        None,
    )
    .await?;
    assert_ne!(after.0, StatusCode::OK, "{}", after.1);
    assert!(!after.1.contains(&key));
    let foreign_revoke = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{owner}/attachment-revokes"),
        Some(
            serde_json::json!({
                "request_id": "d08f-foreign",
                "attachment_id": attachment_id,
                "actor_user_id": actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(foreign_revoke.0, StatusCode::OK, "{}", foreign_revoke.1);
    let row: (bool, i64, String, i64) = sqlx::query_as(
        "select
            (select revoked from shared_attachment_grants where tenant_id = $1 and attachment_id = $3 and actor_user_id = $4),
            (select count(*) from shared_attachments where tenant_id = $1 and attachment_id = $3),
            (select storage_key from shared_attachments where tenant_id = $1 and attachment_id = $3),
            (select count(*) from shared_attachments where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&attachment_id)?)
    .bind(&actor)
    .fetch_one(&independent)
    .await?;
    assert!(row.0);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, key);
    assert_eq!(row.3, 0);
    eprintln!(
        "attachment-revoke-ok tenant={owner} foreign={foreign} actor={owner_actor} attachment={attachment_id} revoked={} files={} foreign_files={}",
        row.0, row.1, row.3
    );
    Ok(())
}

#[tokio::test]
async fn attachment_download_refuses_foreign_expired_and_revoked_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_attachments (
          tenant_id uuid not null,
          attachment_id uuid not null,
          object_id text not null,
          file_name text not null,
          version_no integer not null,
          byte_count bigint not null,
          digest text not null,
          storage_key text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, attachment_id)
        )",
        "create table if not exists shared_attachment_grants (
          tenant_id uuid not null,
          attachment_id uuid not null,
          actor_user_id text not null,
          revoked boolean not null default false,
          created_at timestamptz not null default now(),
          primary key (tenant_id, attachment_id, actor_user_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("attachment-download tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d08e-owner"),
        (foreign, foreign_actor, "d08e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("attachment-download-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d08e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d08e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d08e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d08e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let actor = format!("actor-{owner_actor}");
    let other = format!("other-{owner_actor}");
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-attachments"),
        Some(
            serde_json::json!({
                "request_id": "d08e-save",
                "object_id": format!("ITEM-{owner}"),
                "file_name": "drawing.pdf",
                "digest": format!("digest-{owner}"),
                "byte_count": 1024,
                "actor_user_id": actor
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let effect = saved_body["effect_id"].as_str().unwrap_or("");
    let attachment_id = effect.split(',').next().unwrap_or("").to_string();
    assert!(!attachment_id.is_empty(), "{}", saved.1);
    let now_unix = 1_760_000_000_i64;
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/attachment-downloads?attachment_id={attachment_id}&actor_user_id={actor}&now_unix={now_unix}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let unauthorized = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/attachment-downloads"),
        Some(
            serde_json::json!({
                "request_id": "d08e-other",
                "attachment_id": attachment_id,
                "actor_user_id": other,
                "expires_at": now_unix + 60,
                "now_unix": now_unix
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(unauthorized.0, StatusCode::OK, "{}", unauthorized.1);
    let expired = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/attachment-downloads"),
        Some(
            serde_json::json!({
                "request_id": "d08e-expired",
                "attachment_id": attachment_id,
                "actor_user_id": actor,
                "expires_at": now_unix,
                "now_unix": now_unix
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(expired.0, StatusCode::OK, "{}", expired.1);
    let linked = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/attachment-downloads"),
        Some(
            serde_json::json!({
                "request_id": "d08e-link",
                "attachment_id": attachment_id,
                "actor_user_id": actor,
                "expires_at": now_unix + 3600,
                "now_unix": now_unix
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(linked.0, StatusCode::OK, "{}", linked.1);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/attachment-downloads?attachment_id={attachment_id}&actor_user_id={actor}&now_unix={now_unix}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&attachment_id));
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/attachment-downloads?attachment_id={attachment_id}&actor_user_id={actor}&now_unix={now_unix}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    let key = body["storage_key"].as_str().unwrap_or("").to_string();
    assert!(key.starts_with(&format!("{owner}/")), "{}", read.1);
    assert_eq!(body["revoked"].as_bool().unwrap_or(true), false);
    assert_eq!(body["expires_at"].as_i64().unwrap_or(0), now_unix + 3600);
    let later = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/attachment-downloads?attachment_id={attachment_id}&actor_user_id={actor}&now_unix={}", now_unix + 3600),
        None,
    )
    .await?;
    assert_ne!(later.0, StatusCode::OK, "{}", later.1);
    assert!(!later.1.contains(&key));
    let row: (String, i64, bool, i64, i64) = sqlx::query_as(
        "select
            (select storage_key from shared_attachments where tenant_id = $1 and attachment_id = $3),
            (select expires_at from attachment_download_links where tenant_id = $1 and attachment_id = $3 and actor_user_id = $4),
            (select revoked from shared_attachment_grants where tenant_id = $1 and attachment_id = $3 and actor_user_id = $4),
            (select count(*) from attachment_download_links where tenant_id = $1 and actor_user_id = $5),
            (select count(*) from shared_attachments where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&attachment_id)?)
    .bind(&actor)
    .bind(&other)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, key);
    assert_eq!(row.1, now_unix + 3600);
    assert!(!row.2);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 0);
    eprintln!(
        "attachment-download-ok tenant={owner} foreign={foreign} actor={owner_actor} attachment={attachment_id} expires={} revoked={} other_links={} foreign_files={}",
        row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn unexplained_opening_gap_refuses_enable_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists opening_documents (
          tenant_id uuid not null,
          opening_id uuid not null,
          cutoff_date text not null,
          status text not null,
          version_no integer not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, opening_id)
        )",
        "create table if not exists opening_qty_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          opening_id uuid not null,
          delta_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists opening_cost_ledgers (
          tenant_id uuid not null,
          ledger_id uuid not null,
          opening_id uuid not null,
          amount_cent bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, ledger_id)
        )",
        "create table if not exists partner_balances (
          tenant_id uuid not null,
          balance_id uuid not null,
          kind text not null,
          partner_code text not null,
          currency text not null,
          source_type text not null,
          source_id text not null,
          amount_cent bigint not null,
          settled_cent bigint not null default 0,
          due_date text,
          created_at timestamptz not null default now(),
          primary key (tenant_id, balance_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("opening-gap tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d07e-owner"),
        (foreign, foreign_actor, "d07e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("opening-gap-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d07e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d07e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d07e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d07e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let cutoff = format!("2026-10-05-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/opening-gaps?cutoff_date={cutoff}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let bare = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/opening-confirms"),
        Some(
            serde_json::json!({
                "request_id": "d07e-bare",
                "cutoff_date": cutoff,
                "qty_micros": 1250000,
                "cost_cent": 800,
                "partner_cent": 300
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(bare.0, StatusCode::OK, "{}", bare.1);
    let blank = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/opening-gaps"),
        Some(
            serde_json::json!({
                "request_id": "d07e-blank",
                "cutoff_date": cutoff,
                "qty_gap": 250000,
                "cost_gap": 80,
                "explanation": " "
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(blank.0, StatusCode::OK, "{}", blank.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/opening-gaps"),
        Some(
            serde_json::json!({
                "request_id": "d07e-gap",
                "cutoff_date": cutoff,
                "qty_gap": 250000,
                "cost_gap": 80,
                "explanation": "count sheet short one carton"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/opening-gaps?cutoff_date={cutoff}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(body["qty_gap"].as_i64().unwrap_or(0), 250000);
    assert_eq!(body["cost_gap"].as_i64().unwrap_or(0), 80);
    assert_eq!(
        body["explanation"].as_str().unwrap_or(""),
        "count sheet short one carton"
    );
    assert_eq!(body["enabled"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/opening-gaps?cutoff_date={cutoff}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("count sheet short one carton"));
    let row: (i64, i64, String, i64, i64) = sqlx::query_as(
        "select
            (select qty_gap from opening_gap_notes where tenant_id = $1 and cutoff_date = $3),
            (select cost_gap from opening_gap_notes where tenant_id = $1 and cutoff_date = $3),
            (select explanation from opening_gap_notes where tenant_id = $1 and cutoff_date = $3),
            (select count(*) from opening_documents where tenant_id = $1) + (select count(*) from opening_qty_ledgers where tenant_id = $1) + (select count(*) from opening_cost_ledgers where tenant_id = $1) + (select count(*) from partner_balances where tenant_id = $1 and source_type = 'OPENING'),
            (select count(*) from opening_gap_notes where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&cutoff)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 250000);
    assert_eq!(row.1, 80);
    assert_eq!(row.2, "count sheet short one carton");
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 0);
    eprintln!(
        "opening-gap-ok tenant={owner} foreign={foreign} actor={owner_actor} qty={} cost={} enabled_rows={} foreign_notes={}",
        row.0, row.1, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn import_error_lines_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_items (
          tenant_id uuid not null,
          item_id uuid not null,
          item_code text not null,
          name text not null,
          spec text not null default '',
          category text not null default '',
          barcode text not null default '',
          buy boolean not null,
          make boolean not null,
          sell boolean not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, item_id),
          unique (tenant_id, item_code)
        )",
        "create table if not exists shared_import_jobs (
          tenant_id uuid not null,
          job_id uuid not null,
          file_digest text not null,
          column_map text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, job_id)
        )",
        "create table if not exists shared_import_rows (
          tenant_id uuid not null,
          job_id uuid not null,
          line_no integer not null,
          item_code text not null,
          name text not null,
          unit_code text not null,
          status text not null,
          error_text text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, job_id, line_no)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("import-errors tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d06a-owner"),
        (foreign, foreign_actor, "d06a-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("import-errors-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d06a-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d06a",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d06a-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d06a",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let code = format!("SKU-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/import-prechecks?job_id={owner}&line_no=2"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let posted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/import-prechecks"),
        Some(
            serde_json::json!({
                "request_id": "d06a-precheck",
                "file_digest": format!("digest-{owner}"),
                "column_map": "code,name,unit",
                "rows": [
                    {"line_no": 1, "item_code": code, "name": "legal", "unit_code": "PCS"},
                    {"line_no": 2, "item_code": code, "name": "again", "unit_code": "PCS"},
                    {"line_no": 3, "item_code": format!("BLANK-{owner}"), "name": "blank", "unit_code": ""},
                    {"line_no": 4, "item_code": format!("SCALE-{owner}"), "name": "0.1250001", "unit_code": "PCS"}
                ]
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    let posted_body: Value = serde_json::from_str(&posted.1).unwrap_or(Value::Null);
    let job_id = posted_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!job_id.is_empty(), "{}", posted.1);
    for (line_no, expected) in [
        (2, "duplicate code in file"),
        (3, "unit is required"),
        (4, "quantity scale exceeded"),
    ] {
        let read = export_http(
            &app,
            &owner_token,
            "GET",
            &format!("/v1/tenants/{owner}/import-prechecks?job_id={job_id}&line_no={line_no}"),
            None,
        )
        .await?;
        assert_eq!(read.0, StatusCode::OK, "{}", read.1);
        let body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
        assert_eq!(body["line_no"].as_i64().unwrap_or(0), line_no);
        assert_eq!(body["status"].as_str().unwrap_or(""), "ERROR");
        assert_eq!(body["error_text"].as_str().unwrap_or(""), expected);
        assert_eq!(body["items_written"].as_i64().unwrap_or(-1), 0);
    }
    let ready = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/import-prechecks?job_id={job_id}&line_no=1"),
        None,
    )
    .await?;
    assert_ne!(ready.0, StatusCode::OK, "{}", ready.1);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/import-prechecks?job_id={job_id}&line_no=2"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&code));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select count(*) from shared_import_rows where tenant_id = $1 and job_id = $3 and status = 'ERROR' and line_no = 2 and error_text = 'duplicate code in file'),
            (select count(*) from shared_import_rows where tenant_id = $1 and job_id = $3 and status = 'ERROR' and line_no = 3 and error_text = 'unit is required'),
            (select count(*) from shared_import_rows where tenant_id = $1 and job_id = $3 and status = 'ERROR' and line_no = 4 and error_text = 'quantity scale exceeded'),
            (select count(*) from shared_items where tenant_id = $1) + (select count(*) from shared_items where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&job_id)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 1);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    eprintln!(
        "import-errors-ok tenant={owner} foreign={foreign} actor={owner_actor} job={job_id} duplicate={} blank={} scale={} items={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn twenty_employee_files_add_no_seats_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists member_seats (
          tenant_id uuid not null,
          user_id text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, user_id)
        )",
        "create table if not exists shared_employees (
          tenant_id uuid not null,
          employee_id uuid not null,
          employee_code text not null,
          user_id text not null,
          department text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, employee_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("employee-seats tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d05g-owner"),
        (foreign, foreign_actor, "d05g-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("employee-seats-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d05g-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d05g",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d05g-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d05g",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-employees"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    for index in 1..=20 {
        let saved = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/shared-employees"),
            Some(
                serde_json::json!({
                    "request_id": format!("d05g-{index}"),
                    "employee_code": format!("E-{owner}-{index}"),
                    "user_id": format!("user-{owner}-{index}"),
                    "department": "assembly"
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    }
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-employees"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["files"].as_i64().unwrap_or(0), 20);
    assert_eq!(read_body["seats"].as_i64().unwrap_or(-1), 0);
    assert_eq!(read_body["seats_opened"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-employees"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select
            (select count(*) from shared_employees where tenant_id = $1),
            (select count(*) from member_seats where tenant_id = $1),
            (select count(*) from shared_employees where tenant_id = $2),
            (select count(*) from member_seats where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 20);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "employee-seats-ok tenant={owner} foreign={foreign} actor={owner_actor} files={} seats={} foreign_files={} foreign_seats={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn rate_change_keeps_frozen_order_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_employees (
          tenant_id uuid not null,
          employee_id uuid not null,
          employee_code text not null,
          user_id text not null,
          department text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, employee_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("labor-rate tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d05f-owner"),
        (foreign, foreign_actor, "d05f-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("labor-rate-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d05f-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d05f",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d05f-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d05f",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-employees"),
        Some(
            serde_json::json!({
                "request_id": "d05f-employee",
                "employee_code": format!("E-{owner}"),
                "user_id": format!("user-{owner}"),
                "department": "assembly"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let employee_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!employee_id.is_empty(), "{}", saved.1);
    let order_no = format!("WO-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/labor-rates?employee_id={employee_id}&order_no={order_no}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let frozen = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/labor-rates"),
        Some(
            serde_json::json!({
                "request_id": "d05f-frozen",
                "employee_id": employee_id,
                "order_no": order_no,
                "rate_cent": 200,
                "frozen": true
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(frozen.0, StatusCode::OK, "{}", frozen.1);
    let frozen_body: Value = serde_json::from_str(&frozen.1).unwrap_or(Value::Null);
    assert_eq!(frozen_body["effect_id"].as_str().unwrap_or(""), "FROZEN");
    let changed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/labor-rates"),
        Some(
            serde_json::json!({
                "request_id": "d05f-next",
                "employee_id": employee_id,
                "order_no": order_no,
                "rate_cent": 250,
                "frozen": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(changed.0, StatusCode::OK, "{}", changed.1);
    let changed_body: Value = serde_json::from_str(&changed.1).unwrap_or(Value::Null);
    assert_eq!(changed_body["effect_id"].as_str().unwrap_or(""), "CHANGED");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/labor-rates?employee_id={employee_id}&order_no={order_no}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["frozen_rate_cent"].as_i64().unwrap_or(0), 200);
    assert_eq!(read_body["next_rate_cent"].as_i64().unwrap_or(0), 250);
    assert_eq!(
        read_body["frozen_rewritten"].as_bool().unwrap_or(true),
        false
    );
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/labor-rates?employee_id={employee_id}&order_no={order_no}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("200"));
    let row: (i64, i64, i64, i64) = sqlx::query_as(
        "select o.rate_cent, v.rate_cent,
            (select count(*) from labor_rate_orders where tenant_id = $1 and frozen = true and rate_cent = 200),
            (select count(*) from labor_rate_orders where tenant_id = $2)
         from labor_rate_orders o
         join labor_rate_versions v on v.tenant_id = o.tenant_id and v.employee_id = o.employee_id and v.order_no = o.order_no
         where o.tenant_id = $1 and o.employee_id = $3 and o.order_no = $4 and o.frozen = true",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&employee_id).unwrap())
    .bind(&order_no)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 200);
    assert_eq!(row.1, 250);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    eprintln!(
        "labor-rate-ok tenant={owner} foreign={foreign} actor={owner_actor} employee={employee_id} order={order_no} frozen={} next={} frozen_rows={} foreign_orders={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

#[tokio::test]
async fn employee_leave_keeps_labor_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists member_seats (
          tenant_id uuid not null,
          user_id text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, user_id)
        )",
        "create table if not exists shared_employees (
          tenant_id uuid not null,
          employee_id uuid not null,
          employee_code text not null,
          user_id text not null,
          department text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, employee_id)
        )",
        "create table if not exists shared_labor_reports (
          tenant_id uuid not null,
          employee_id uuid not null,
          report_id text not null,
          reporter_user_id text not null,
          certifier_user_id text not null,
          minutes bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, employee_id, report_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("employee-leave tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d05e-owner"),
        (foreign, foreign_actor, "d05e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("employee-leave-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d05e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d05e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d05e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d05e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let code = format!("E-{owner}");
    let login_user = format!("user-{owner}");
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-employees"),
        Some(
            serde_json::json!({
                "request_id": "d05e-employee",
                "employee_code": code,
                "user_id": login_user,
                "department": "assembly"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let employee_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!employee_id.is_empty(), "{}", saved.1);
    assert_ne!(employee_id, login_user);
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/labor-reports?employee_id={employee_id}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let reported = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/labor-reports"),
        Some(
            serde_json::json!({
                "request_id": "d05e-labor",
                "employee_id": employee_id,
                "reporter_user_id": format!("reporter-{owner}"),
                "certifier_user_id": format!("certifier-{owner}"),
                "minutes": 30
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(reported.0, StatusCode::OK, "{}", reported.1);
    let left = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/employee-leaves"),
        Some(
            serde_json::json!({
                "request_id": "d05e-leave",
                "employee_id": employee_id
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(left.0, StatusCode::OK, "{}", left.1);
    let left_body: Value = serde_json::from_str(&left.1).unwrap_or(Value::Null);
    assert_eq!(left_body["effect_id"].as_str().unwrap_or(""), "LEFT");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/labor-reports?employee_id={employee_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["status"].as_str().unwrap_or(""), "LEFT");
    assert_eq!(read_body["seats"].as_i64().unwrap_or(-1), 0);
    assert_eq!(read_body["reports"].as_i64().unwrap_or(0), 1);
    assert_eq!(read_body["seat_opened"].as_bool().unwrap_or(true), false);
    assert_eq!(
        read_body["reporter_user_id"].as_str().unwrap_or(""),
        format!("reporter-{owner}")
    );
    assert_ne!(
        read_body["reporter_user_id"].as_str().unwrap_or(""),
        read_body["certifier_user_id"].as_str().unwrap_or("")
    );
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/labor-reports?employee_id={employee_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&code));
    let row: (String, String, i64, i64, i64, i64) = sqlx::query_as(
        "select e.status, r.reporter_user_id,
            (select count(*) from member_seats where tenant_id = $1),
            (select count(*) from shared_labor_reports where tenant_id = $1 and employee_id = $2),
            (select count(*) from shared_employees where tenant_id = $3),
            (select count(*) from member_seats where tenant_id = $3)
         from shared_employees e
         join shared_labor_reports r on r.tenant_id = e.tenant_id and r.employee_id = e.employee_id
         where e.tenant_id = $1 and e.employee_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(&employee_id).unwrap())
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "LEFT");
    assert_eq!(row.1, format!("reporter-{owner}"));
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    eprintln!(
        "employee-leave-ok tenant={owner} foreign={foreign} actor={owner_actor} employee={employee_id} status={} reporter={} seats={} reports={} foreign_employees={} foreign_seats={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn duplicate_location_code_refused_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_locations (
          tenant_id uuid not null,
          location_id uuid not null,
          parent_id uuid,
          code text not null,
          purpose text not null,
          is_default boolean not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, location_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("dup-location tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d04g-owner"),
        (foreign, foreign_actor, "d04g-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("dup-location-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d04g-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d04g",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d04g-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d04g",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let code = format!("WH-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-locations?code={code}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-locations"),
        Some(
            serde_json::json!({
                "request_id": "d04g-one",
                "code": code,
                "purpose": "FINISHED",
                "is_default": true
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let location_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!location_id.is_empty(), "{}", saved.1);
    let duplicate = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-locations"),
        Some(
            serde_json::json!({
                "request_id": "d04g-two",
                "code": code,
                "purpose": "RAW",
                "is_default": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(duplicate.0, StatusCode::OK, "{}", duplicate.1);
    let foreign_post = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-locations"),
        Some(
            serde_json::json!({
                "request_id": "d04g-foreign",
                "code": code,
                "purpose": "HOLD",
                "is_default": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(foreign_post.0, StatusCode::OK, "{}", foreign_post.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-locations?code={code}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["location_id"].as_str().unwrap_or(""), location_id);
    assert_eq!(read_body["code"].as_str().unwrap_or(""), code);
    assert_eq!(read_body["status"].as_str().unwrap_or(""), "ACTIVE");
    assert_eq!(read_body["rows"].as_i64().unwrap_or(0), 1);
    assert_eq!(read_body["duplicated"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/shared-locations?code={code}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&location_id));
    let row: (Uuid, String, String, i64, i64, i64) = sqlx::query_as(
        "select location_id, code, purpose,
            (select count(*) from shared_locations where tenant_id = $1 and code = $2),
            (select count(*) from command_receipts where tenant_id = $1 and request_id in ('d04g-two', 'd04g-foreign')),
            (select count(*) from shared_locations where tenant_id = $3 and code = $2)
         from shared_locations where tenant_id = $1 and code = $2",
    )
    .bind(owner)
    .bind(&code)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, Uuid::parse_str(&location_id).unwrap());
    assert_eq!(row.1, code);
    assert_eq!(row.2, "FINISHED");
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    eprintln!(
        "dup-location-ok tenant={owner} foreign={foreign} actor={owner_actor} location={} code={} purpose={} rows={} refused_receipts={} foreign_same_code={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn rename_keeps_stock_identity_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_locations (
          tenant_id uuid not null,
          location_id uuid not null,
          parent_id uuid,
          code text not null,
          purpose text not null,
          is_default boolean not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, location_id)
        )",
        "create table if not exists shared_location_stock (
          tenant_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          on_hand bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, location_id, sku_id)
        )",
        "create table if not exists shared_location_owners (
          tenant_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          owner_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, location_id, sku_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("location-rename tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d04f-owner"),
        (foreign, foreign_actor, "d04f-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("location-rename-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d04f-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d04f",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d04f-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d04f",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let old_code = format!("OLD-{owner}");
    let new_code = format!("NEW-{owner}");
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-locations"),
        Some(
            serde_json::json!({
                "request_id": "d04f-bin",
                "code": old_code,
                "purpose": "FINISHED",
                "is_default": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let location_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!location_id.is_empty(), "{}", saved.1);
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/location-retirements?location_id={location_id}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let placed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/location-stock"),
        Some(
            serde_json::json!({
                "request_id": "d04f-stock",
                "location_id": location_id,
                "sku_id": sku,
                "on_hand": 1250000,
                "owner_code": "OWN"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(placed.0, StatusCode::OK, "{}", placed.1);
    let renamed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/location-renames"),
        Some(
            serde_json::json!({
                "request_id": "d04f-rename",
                "location_id": location_id,
                "code": new_code
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(renamed.0, StatusCode::OK, "{}", renamed.1);
    let renamed_body: Value = serde_json::from_str(&renamed.1).unwrap_or(Value::Null);
    assert_eq!(
        renamed_body["effect_id"].as_str().unwrap_or(""),
        location_id
    );
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/location-retirements?location_id={location_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["location_id"].as_str().unwrap_or(""), location_id);
    assert_eq!(read_body["code"].as_str().unwrap_or(""), new_code);
    assert_ne!(read_body["code"].as_str().unwrap_or(""), old_code);
    assert_eq!(read_body["on_hand"].as_i64().unwrap_or(0), 1_250_000);
    assert_eq!(read_body["owner_code"].as_str().unwrap_or(""), "OWN");
    assert_eq!(read_body["deleted"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/location-retirements?location_id={location_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&new_code));
    let row: (String, i64, String, i64, i64, i64) = sqlx::query_as(
        "select l.code, s.on_hand, o.owner_code,
            (select count(*) from shared_locations where tenant_id = $1),
            (select count(*) from shared_location_stock where tenant_id = $1 and location_id = $4),
            (select count(*) from shared_locations where tenant_id = $2)
         from shared_locations l
         join shared_location_stock s on s.tenant_id = l.tenant_id and s.location_id = l.location_id
         join shared_location_owners o on o.tenant_id = s.tenant_id and o.location_id = s.location_id and o.sku_id = s.sku_id
         where l.tenant_id = $1 and l.location_id = $4 and l.code = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&new_code)
    .bind(Uuid::parse_str(&location_id).unwrap())
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, new_code);
    assert_eq!(row.1, 1_250_000);
    assert_eq!(row.2, "OWN");
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 1);
    assert_eq!(row.5, 0);
    eprintln!(
        "location-rename-ok tenant={owner} foreign={foreign} actor={owner_actor} location={location_id} code={} on_hand={} owner_code={} locations={} stock_rows={} foreign_locations={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn stocked_location_refuses_delete_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_locations (
          tenant_id uuid not null,
          location_id uuid not null,
          parent_id uuid,
          code text not null,
          purpose text not null,
          is_default boolean not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, location_id)
        )",
        "create table if not exists shared_location_refs (
          tenant_id uuid not null,
          location_id uuid not null,
          document_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, location_id, document_id)
        )",
        "create table if not exists shared_location_stock (
          tenant_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          on_hand bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, location_id, sku_id)
        )",
        "create table if not exists shared_location_owners (
          tenant_id uuid not null,
          location_id uuid not null,
          sku_id uuid not null,
          owner_code text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, location_id, sku_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("stocked-location tenant={owner} foreign={foreign} actor={owner_actor} sku={sku}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d04e-owner"),
        (foreign, foreign_actor, "d04e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("stocked-location-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d04e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d04e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d04e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d04e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let code = format!("BIN-{owner}");
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-locations"),
        Some(
            serde_json::json!({
                "request_id": "d04e-bin",
                "code": code,
                "purpose": "FINISHED",
                "is_default": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let location_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!location_id.is_empty(), "{}", saved.1);
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/location-retirements?location_id={location_id}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let placed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/location-stock"),
        Some(
            serde_json::json!({
                "request_id": "d04e-stock",
                "location_id": location_id,
                "sku_id": sku,
                "on_hand": 1250000,
                "owner_code": "OWN"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(placed.0, StatusCode::OK, "{}", placed.1);
    let deleted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/location-retirements"),
        Some(
            serde_json::json!({
                "request_id": "d04e-delete",
                "location_id": location_id,
                "requested": "DELETE"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(deleted.0, StatusCode::OK, "{}", deleted.1);
    assert!(!deleted.1.contains("DELETED"));
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/location-retirements?location_id={location_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["location_id"].as_str().unwrap_or(""), location_id);
    assert_eq!(read_body["code"].as_str().unwrap_or(""), code);
    assert_eq!(read_body["status"].as_str().unwrap_or(""), "ACTIVE");
    assert_eq!(read_body["on_hand"].as_i64().unwrap_or(0), 1_250_000);
    assert_eq!(read_body["owner_code"].as_str().unwrap_or(""), "OWN");
    assert_eq!(read_body["deleted"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/location-retirements?location_id={location_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&code));
    let row: (String, String, i64, String, i64, i64) = sqlx::query_as(
        "select l.code, l.status, s.on_hand, o.owner_code,
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'd04e-delete'),
            (select count(*) from shared_locations where tenant_id = $2 and code = $3)
         from shared_locations l
         join shared_location_stock s on s.tenant_id = l.tenant_id and s.location_id = l.location_id
         join shared_location_owners o on o.tenant_id = s.tenant_id and o.location_id = s.location_id and o.sku_id = s.sku_id
         where l.tenant_id = $1 and l.location_id = $4",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&code)
    .bind(Uuid::parse_str(&location_id).unwrap())
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, code);
    assert_eq!(row.1, "ACTIVE");
    assert_eq!(row.2, 1_250_000);
    assert_eq!(row.3, "OWN");
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    eprintln!(
        "stocked-location-ok tenant={owner} foreign={foreign} actor={owner_actor} location={location_id} code={} status={} on_hand={} owner_code={} delete_receipts={} foreign_locations={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn refused_qty_keeps_draft_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists unit_conversions (
          tenant_id uuid not null,
          conversion_id uuid not null,
          base_unit text not null,
          aux_unit text not null,
          ratio_micros bigint not null,
          rounding_code text not null,
          aux_micros bigint not null,
          base_micros bigint not null,
          back_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, conversion_id)
        )",
        "create table if not exists unit_qty_samples (
          tenant_id uuid not null,
          sample_id uuid not null,
          unit_code text not null,
          qty_micros bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, sample_id)
        )",
        "create table if not exists edit_drafts (
          tenant_id uuid not null,
          draft_id uuid not null,
          user_id text not null,
          page_code text not null,
          object_id text not null,
          version_no bigint not null,
          body_text text not null,
          status text not null,
          service_address text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, draft_id),
          unique (tenant_id, user_id, page_code, object_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("qty-draft tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d02f-owner"),
        (foreign, foreign_actor, "d02f-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("qty-draft-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d02f-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d02f",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d02f-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d02f",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let user = format!("user-{owner_actor}");
    let object_id = format!("CONV-{owner}");
    let refused = "0";
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/edit-drafts?user_id={user}&page=UNITS&object_id={object_id}&refused_text={refused}"
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let zero = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/unit-conversions"),
        Some(
            serde_json::json!({
                "request_id": "d02f-zero",
                "base_unit": format!("G-{owner}"),
                "aux_unit": format!("KG-{owner}"),
                "ratio_text": refused,
                "rounding_code": "HALF_UP",
                "aux_text": "0.125",
                "display_text": ""
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(zero.0, StatusCode::OK, "{}", zero.1);
    let over = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/unit-qtys"),
        Some(
            serde_json::json!({
                "request_id": "d02f-scale",
                "unit_code": format!("KG-{owner}"),
                "qty_text": "0.1250001",
                "display_text": ""
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(over.0, StatusCode::OK, "{}", over.1);
    let held = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/edit-drafts"),
        Some(
            serde_json::json!({
                "request_id": "d02f-draft",
                "user_id": user,
                "page": "UNITS",
                "object_id": object_id,
                "version": 1,
                "body": refused,
                "service_address": "svc-d02f"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(held.0, StatusCode::OK, "{}", held.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/edit-drafts?user_id={user}&page=UNITS&object_id={object_id}&refused_text={refused}"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["body"].as_str().unwrap_or(""), refused);
    assert_eq!(read_body["conversions"].as_i64().unwrap_or(-1), 0);
    assert_eq!(read_body["samples"].as_i64().unwrap_or(-1), 0);
    assert_eq!(read_body["written"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/edit-drafts?user_id={user}&page=UNITS&object_id={object_id}&refused_text={refused}"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(refused) || foreign_read.1.contains("tenant mismatch"));
    let row: (String, String, i64, i64, i64, i64) = sqlx::query_as(
        "select body_text, status,
            (select count(*) from unit_conversions where tenant_id = $1),
            (select count(*) from unit_qty_samples where tenant_id = $1),
            (select count(*) from command_receipts where tenant_id = $1 and request_id in ('d02f-zero', 'd02f-scale')),
            (select count(*) from edit_drafts where tenant_id = $2)
         from edit_drafts where tenant_id = $1 and object_id = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&object_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, refused);
    assert_eq!(row.1, "HELD");
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, 0);
    eprintln!(
        "qty-draft-ok tenant={owner} foreign={foreign} actor={owner_actor} body={} status={} conversions={} samples={} refused_receipts={} foreign_drafts={}",
        row.0, row.1, row.2, row.3, row.4, row.5
    );
    Ok(())
}

#[tokio::test]
async fn disable_keeps_receivable_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists shared_partners (
          tenant_id uuid not null,
          partner_id uuid not null,
          name text not null,
          customer boolean not null,
          supplier boolean not null,
          customer_no text not null default '',
          supplier_no text not null default '',
          term_days integer not null,
          tax_no text not null default '',
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, partner_id)
        )",
        "create table if not exists shared_partner_role_balances (
          tenant_id uuid not null,
          balance_id uuid not null,
          partner_id uuid not null,
          customer_no text not null,
          supplier_no text not null,
          receivable_cent bigint not null,
          payable_cent bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, balance_id)
        )",
        "create table if not exists shared_partner_merges (
          tenant_id uuid not null,
          source_id uuid not null,
          target_id uuid not null,
          moved_refs bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, source_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("partner-disable tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "d03e-owner"),
        (foreign, foreign_actor, "d03e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("partner-disable-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("d03e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-d03e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("d03e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-d03e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/shared-partners"),
        Some(
            serde_json::json!({
                "request_id": "d03e-partner",
                "name": "north",
                "customer": true,
                "supplier": true,
                "customer_no": format!("C-{owner}"),
                "supplier_no": format!("S-{owner}"),
                "term_days": 30,
                "tax_no": "TAX-D03E"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let partner_id = saved_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!partner_id.is_empty());
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/partner-disables?partner_id={partner_id}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let balance = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/partner-role-balances"),
        Some(
            serde_json::json!({
                "request_id": "d03e-balance",
                "partner_id": partner_id,
                "receivable_cent": 800,
                "payable_cent": 300
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(balance.0, StatusCode::OK, "{}", balance.1);
    let deleted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/partner-disables"),
        Some(
            serde_json::json!({
                "request_id": "d03e-delete",
                "partner_id": partner_id,
                "requested": "DELETE"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(deleted.0, StatusCode::OK, "{}", deleted.1);
    let disabled = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/partner-disables"),
        Some(
            serde_json::json!({
                "request_id": "d03e-disable",
                "partner_id": partner_id,
                "requested": "DISABLE"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(disabled.0, StatusCode::OK, "{}", disabled.1);
    let disabled_body: Value = serde_json::from_str(&disabled.1).unwrap_or(Value::Null);
    assert_eq!(
        disabled_body["effect_id"].as_str().unwrap_or(""),
        "DISABLED"
    );
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/partner-disables?partner_id={partner_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["status"].as_str().unwrap_or(""), "DISABLED");
    assert_eq!(read_body["receivable_cent"].as_i64().unwrap_or(0), 800);
    assert_eq!(read_body["payable_cent"].as_i64().unwrap_or(0), 300);
    assert_eq!(
        read_body["balance_removed"].as_bool().unwrap_or(true),
        false
    );
    let foreign_merge = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{owner}/partner-merges"),
        Some(
            serde_json::json!({
                "request_id": "d03e-foreign-merge",
                "source_id": partner_id,
                "target_id": partner_id
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(foreign_merge.0, StatusCode::OK, "{}", foreign_merge.1);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/partner-disables?partner_id={partner_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("800"));
    let row: (Option<String>, Option<i64>, i64, i64) = sqlx::query_as(
        "select
            (select status from shared_partners where tenant_id = $1 and partner_id = $3),
            (select receivable_cent from shared_partner_role_balances where tenant_id = $1 and partner_id = $3),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'd03e-delete'),
            (select count(*)::bigint from shared_partners where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&partner_id)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("DISABLED"));
    assert_eq!(row.1, Some(800));
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "partner-disable-ok tenant={owner} foreign={foreign} actor={owner_actor} partner={partner_id}"
    );
    Ok(())
}

#[tokio::test]
async fn theme_switch_keeps_draft_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists page_instances (
          tenant_id uuid not null,
          page_row_id uuid not null,
          user_id text not null,
          page_code text not null,
          instance_code text not null,
          service_code text not null,
          theme_code text not null,
          left_hidden boolean not null,
          right_hidden boolean not null,
          subtable_hidden boolean not null,
          query_text text not null,
          draft_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, page_row_id),
          unique (tenant_id, user_id, page_code, instance_code)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let draft = format!("qty=1-{owner}");
    eprintln!("theme-switch tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u06e-owner"),
        (foreign, foreign_actor, "u06e-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("theme-switch-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u06e-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u06e",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u06e-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u06e",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/page-instances"),
        Some(
            serde_json::json!({
                "request_id": "u06e-open",
                "page": "ORDERS",
                "instance": "tab-u06e",
                "service": "sales.orders",
                "theme": "DARK",
                "left_hidden": false,
                "right_hidden": false,
                "subtable_hidden": false,
                "query": "SO-U06E",
                "draft": draft
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let unknown = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/page-themes"),
        Some(
            serde_json::json!({
                "request_id": "u06e-bad",
                "page": "ORDERS",
                "instance": "tab-u06e",
                "theme": "BLUE"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(unknown.0, StatusCode::OK, "{}", unknown.1);
    let switched = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/page-themes"),
        Some(
            serde_json::json!({
                "request_id": "u06e-theme",
                "page": "ORDERS",
                "instance": "tab-u06e",
                "theme": "LIGHT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(switched.0, StatusCode::OK, "{}", switched.1);
    let switched_body: Value = serde_json::from_str(&switched.1).unwrap_or(Value::Null);
    assert_eq!(switched_body["effect_id"].as_str().unwrap_or(""), "LIGHT");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/page-instances?page=ORDERS&instance=tab-u06e"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["theme"].as_str().unwrap_or(""), "LIGHT");
    assert_eq!(read_body["draft"].as_str().unwrap_or(""), draft);
    assert_eq!(read_body["query"].as_str().unwrap_or(""), "SO-U06E");
    let foreign_switch = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{owner}/page-themes"),
        Some(
            serde_json::json!({
                "request_id": "u06e-foreign",
                "page": "ORDERS",
                "instance": "tab-u06e",
                "theme": "DARK"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(foreign_switch.0, StatusCode::OK, "{}", foreign_switch.1);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/page-instances?page=ORDERS&instance=tab-u06e"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&draft));
    let row: (Option<String>, Option<String>, i64, i64) = sqlx::query_as(
        "select
            (select theme_code from page_instances where tenant_id = $1 and instance_code = 'tab-u06e'),
            (select draft_text from page_instances where tenant_id = $1 and instance_code = 'tab-u06e'),
            (select count(*)::bigint from page_instances where tenant_id = $1),
            (select count(*)::bigint from page_instances where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("LIGHT"));
    assert_eq!(row.1.as_deref(), Some(draft.as_str()));
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    eprintln!("theme-switch-ok tenant={owner} foreign={foreign} actor={owner_actor} draft={draft}");
    Ok(())
}

#[tokio::test]
async fn zoom_read_keeps_row_and_draft_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists zoom_states (
          tenant_id uuid not null,
          zoom_id uuid not null,
          page_code text not null,
          percent integer not null,
          row_id text not null,
          draft_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, zoom_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let row_id = format!("row-u06d-{owner}");
    let draft = format!("qty=1-{owner}");
    eprintln!("zoom-read tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u06d-owner"),
        (foreign, foreign_actor, "u06d-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("zoom-read-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u06d-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u06d",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u06d-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u06d",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/zoom-states?page=ORDERS&percent=150"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let changed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/zoom-states"),
        Some(
            serde_json::json!({
                "request_id": "u06d-changed",
                "page": "ORDERS",
                "percent": 150,
                "before_row": row_id,
                "after_row": format!("{row_id}-other"),
                "draft_before": draft,
                "draft_after": draft
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(changed.0, StatusCode::OK, "{}", changed.1);
    let stored = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/zoom-states"),
        Some(
            serde_json::json!({
                "request_id": "u06d-zoom",
                "page": "ORDERS",
                "percent": 150,
                "before_row": row_id,
                "after_row": row_id,
                "draft_before": draft,
                "draft_after": draft
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stored.0, StatusCode::OK, "{}", stored.1);
    let stored_body: Value = serde_json::from_str(&stored.1).unwrap_or(Value::Null);
    assert_eq!(stored_body["effect_id"].as_str().unwrap_or(""), "150");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/zoom-states?page=ORDERS&percent=150"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["percent"].as_i64().unwrap_or(0), 150);
    assert_eq!(read_body["row_id"].as_str().unwrap_or(""), row_id);
    assert_eq!(read_body["draft"].as_str().unwrap_or(""), draft);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/zoom-states?page=ORDERS&percent=150"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&row_id));
    assert!(!foreign_read.1.contains(&draft));
    let row: (Option<String>, Option<String>, i64, i64) = sqlx::query_as(
        "select
            (select row_id from zoom_states where tenant_id = $1 and page_code = 'ORDERS' and percent = 150),
            (select draft_text from zoom_states where tenant_id = $1 and page_code = 'ORDERS' and percent = 150),
            (select count(*)::bigint from zoom_states where tenant_id = $1),
            (select count(*)::bigint from zoom_states where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some(row_id.as_str()));
    assert_eq!(row.1.as_deref(), Some(draft.as_str()));
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    eprintln!("zoom-read-ok tenant={owner} foreign={foreign} actor={owner_actor} row={row_id}");
    Ok(())
}

#[tokio::test]
async fn window_close_relogin_one_effect_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists command_objects (
          tenant_id uuid not null,
          request_id text not null,
          object_type text not null,
          object_id text not null,
          version_no bigint not null,
          line_no integer not null,
          line_status text not null,
          recoverable boolean not null,
          error_text text not null,
          accepted_at timestamptz not null default now(),
          primary key (tenant_id, request_id, line_no)
        )",
        "create table if not exists edit_drafts (
          tenant_id uuid not null,
          draft_id uuid not null,
          user_id text not null,
          page_code text not null,
          object_id text not null,
          version_no bigint not null,
          body_text text not null,
          status text not null,
          service_address text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, draft_id),
          unique (tenant_id, user_id, page_code, object_id)
        )",
        "create table if not exists draft_recoveries (
          tenant_id uuid not null,
          recovery_id uuid not null,
          draft_id uuid not null,
          decision text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, recovery_id)
        )",
        "create table if not exists lost_response_lookups (
          tenant_id uuid not null,
          lookup_id uuid not null,
          request_id text not null,
          stored_status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, lookup_id),
          unique (tenant_id, request_id)
        )",
        "alter table edit_drafts add column if not exists service_address text not null default ''",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let object_id = format!("SO-U05I-{owner}");
    let service = format!("https://svc.u05i.invalid/{owner}");
    let other_service = format!("https://other.u05i.invalid/{owner}");
    eprintln!("relogin-once tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u05i-owner"),
        (foreign, foreign_actor, "u05i-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read'), ($1, $2, 'MFG.PRODUCTION.WRITE')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("relogin-once-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u05i-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u05i",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u05i-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u05i",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/commands"),
        Some(
            serde_json::json!({
                "request_id": "u05i-command",
                "object_type": "SALE",
                "object_id": object_id,
                "version_no": 2
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    assert_eq!(
        opened_body["status"].as_str().unwrap_or(""),
        "ACCEPTED",
        "{}",
        opened.1
    );
    assert_eq!(
        opened_body["effect_id"].as_str().unwrap_or(""),
        object_id,
        "{}",
        opened.1
    );
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/commands/u05i-command/window-closes?close_request=u05i-close"
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let closed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/commands/u05i-command/window-closes"),
        Some(
            serde_json::json!({
                "request_id": "u05i-close",
                "client_unknown": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(closed.0, StatusCode::OK, "{}", closed.1);
    let closed_body: Value = serde_json::from_str(&closed.1).unwrap_or(Value::Null);
    assert_eq!(
        closed_body["decision"].as_str().unwrap_or(""),
        "KEEP",
        "{}",
        closed.1
    );
    assert_eq!(
        closed_body["effect_id"].as_str().unwrap_or(""),
        object_id,
        "{}",
        closed.1
    );
    assert_eq!(closed_body["executed"].as_bool().unwrap_or(true), false);
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/commands/u05i-command/window-closes"),
        Some(
            serde_json::json!({
                "request_id": "u05i-close",
                "client_unknown": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(again.0, StatusCode::OK, "{}", again.1);
    let again_body: Value = serde_json::from_str(&again.1).unwrap_or(Value::Null);
    assert_eq!(again_body["replay"].as_bool().unwrap_or(false), true);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/commands/u05i-command/window-closes?close_request=u05i-close"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["decision"].as_str().unwrap_or(""), "KEEP");
    assert_eq!(read_body["effect_id"].as_str().unwrap_or(""), object_id);
    assert_eq!(read_body["effect_count"].as_i64().unwrap_or(0), 1);
    assert_eq!(read_body["executed"].as_bool().unwrap_or(true), false);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/commands/u05i-command/window-closes?close_request=u05i-close"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&object_id));
    let draft = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/edit-drafts"),
        Some(
            serde_json::json!({
                "request_id": "u05i-draft",
                "user_id": format!("user-{owner_actor}"),
                "page": "ORDERS",
                "object_id": object_id,
                "version": 2,
                "body": "qty=1",
                "service_address": service
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(draft.0, StatusCode::OK, "{}", draft.1);
    let draft_body: Value = serde_json::from_str(&draft.1).unwrap_or(Value::Null);
    let draft_id = draft_body["effect_id"].as_str().unwrap_or("").to_string();
    assert!(!draft_id.is_empty(), "{}", draft.1);
    let wrong_service = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/edit-drafts/restore"),
        Some(
            serde_json::json!({
                "request_id": "u05i-other-service",
                "draft_id": draft_id,
                "current_version": 2,
                "service_address": other_service
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(wrong_service.0, StatusCode::OK, "{}", wrong_service.1);
    let foreign_restore = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{owner}/edit-drafts/restore"),
        Some(
            serde_json::json!({
                "request_id": "u05i-foreign-restore",
                "draft_id": draft_id,
                "current_version": 2,
                "service_address": service
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(foreign_restore.0, StatusCode::OK, "{}", foreign_restore.1);
    assert!(!foreign_restore.1.contains("qty=1"));
    let restored = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/edit-drafts/restore"),
        Some(
            serde_json::json!({
                "request_id": "u05i-restore",
                "draft_id": draft_id,
                "current_version": 2,
                "service_address": service
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(restored.0, StatusCode::OK, "{}", restored.1);
    let restored_body: Value = serde_json::from_str(&restored.1).unwrap_or(Value::Null);
    assert_eq!(
        restored_body["effect_id"].as_str().unwrap_or(""),
        "RECHECKED"
    );
    let row: (Option<String>, Option<i64>, Option<i64>, Option<String>, Option<i64>, i64) = sqlx::query_as(
        "select
            (select object_id from command_objects where tenant_id = $1 and request_id = 'u05i-command'),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'u05i-command' and status = 'ACCEPTED'),
            (select count(*)::bigint from command_receipts where tenant_id = $1 and request_id = 'u05i-close'),
            (select service_address from edit_drafts where tenant_id = $1 and draft_id = $3),
            (select count(*)::bigint from draft_recoveries where tenant_id = $1 and draft_id = $3),
            (select count(*)::bigint from edit_drafts where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&draft_id)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some(object_id.as_str()));
    assert_eq!(row.1, Some(1));
    assert_eq!(row.2, Some(1));
    assert_eq!(row.3.as_deref(), Some(service.as_str()));
    assert_eq!(row.4, Some(1));
    assert_eq!(row.5, 0);
    eprintln!(
        "relogin-once-ok tenant={owner} foreign={foreign} actor={owner_actor} effect={object_id} draft={draft_id}"
    );
    Ok(())
}

#[tokio::test]
async fn conflict_keeps_unsaved_input_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists versioned_documents (
          tenant_id uuid not null,
          document_id uuid not null,
          version_no bigint not null,
          body_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, document_id)
        )",
        "create table if not exists version_conflicts (
          tenant_id uuid not null,
          conflict_id uuid not null,
          document_id uuid not null,
          actor_user_id text not null,
          expected_version bigint not null,
          stored_version bigint not null,
          kept_input text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, conflict_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("conflict-keep tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u04j-owner"),
        (foreign, foreign_actor, "u04j-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'MFG.PRODUCTION.WRITE')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("conflict-keep-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u04j-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u04j",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u04j-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u04j",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/documents"),
        Some(
            serde_json::json!({
                "request_id": "u04j-open",
                "body_text": "saved-body"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let document_id = opened_body["document_id"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(!document_id.is_empty(), "{}", opened.1);
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/documents/{document_id}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/documents/{document_id}"),
        Some(
            serde_json::json!({
                "request_id": "u04j-save",
                "expected_version": 1,
                "body_text": "saved-v2"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    assert_eq!(saved_body["conflict"].as_bool().unwrap_or(true), false);
    assert_eq!(saved_body["version_no"].as_i64().unwrap_or(0), 2);
    let conflicted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/documents/{document_id}"),
        Some(
            serde_json::json!({
                "request_id": "u04j-stale",
                "expected_version": 1,
                "body_text": "unsaved-input"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(conflicted.0, StatusCode::OK, "{}", conflicted.1);
    let conflict_body: Value = serde_json::from_str(&conflicted.1).unwrap_or(Value::Null);
    assert_eq!(conflict_body["conflict"].as_bool().unwrap_or(false), true);
    assert_eq!(conflict_body["version_no"].as_i64().unwrap_or(0), 2);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/documents/{document_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["stored_body"].as_str().unwrap_or(""), "saved-v2");
    assert_eq!(
        read_body["kept_input"].as_str().unwrap_or(""),
        "unsaved-input"
    );
    assert_eq!(read_body["version_no"].as_i64().unwrap_or(0), 2);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/documents/{document_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("unsaved-input"));
    let row: (Option<String>, Option<i64>, Option<String>, i64) = sqlx::query_as(
        "select
            (select body_text from versioned_documents where tenant_id = $1 and document_id = $3),
            (select version_no from versioned_documents where tenant_id = $1 and document_id = $3),
            (select kept_input from version_conflicts where tenant_id = $1 and document_id = $3 and actor_user_id = $4),
            (select count(*)::bigint from version_conflicts where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&document_id)?)
    .bind(owner_actor.to_string())
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("saved-v2"));
    assert_eq!(row.1, Some(2));
    assert_eq!(row.2.as_deref(), Some("unsaved-input"));
    assert_eq!(row.3, 0);
    eprintln!(
        "conflict-keep-ok tenant={owner} foreign={foreign} actor={owner_actor} document={document_id}"
    );
    Ok(())
}

#[tokio::test]
async fn object_batch_locates_saved_and_conflict_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists object_batch_results (
          tenant_id uuid not null,
          object_id text not null,
          version_no integer not null,
          decision text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, object_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("object-batch tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u04i-owner"),
        (foreign, foreign_actor, "u04i-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("object-batch-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u04i-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u04i",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u04i-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u04i",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let saved_id = format!("OBJ-SAVED-{owner}");
    let conflict_id = format!("OBJ-CONFLICT-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/object-batch-results?saved_id={saved_id}&conflict_id={conflict_id}"
        ),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let posted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/object-batch-results"),
        Some(
            serde_json::json!({
                "request_id": "u04i-pair",
                "saved_id": saved_id,
                "saved_version": 2,
                "conflict_id": conflict_id,
                "conflict_version": 4
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(posted.0, StatusCode::OK, "{}", posted.1);
    let posted_body: Value = serde_json::from_str(&posted.1).unwrap_or(Value::Null);
    assert_eq!(
        posted_body["effect_id"].as_str().unwrap_or(""),
        format!("{saved_id}:SAVED/{conflict_id}:CONFLICT")
    );
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/object-batch-results?saved_id={saved_id}&conflict_id={conflict_id}"
        ),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["saved"].as_str().unwrap_or(""), "SAVED");
    assert_eq!(read_body["saved_version"].as_i64().unwrap_or(0), 2);
    assert_eq!(read_body["conflict"].as_str().unwrap_or(""), "CONFLICT");
    assert_eq!(read_body["conflict_version"].as_i64().unwrap_or(0), 4);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!(
            "/v1/tenants/{owner}/object-batch-results?saved_id={saved_id}&conflict_id={conflict_id}"
        ),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains(&saved_id));
    assert!(!foreign_read.1.contains("CONFLICT"));
    let row: (
        Option<String>,
        Option<i32>,
        Option<String>,
        Option<i32>,
        i64,
    ) = sqlx::query_as(
        "select
            (select decision from object_batch_results where tenant_id = $1 and object_id = $3),
            (select version_no from object_batch_results where tenant_id = $1 and object_id = $3),
            (select decision from object_batch_results where tenant_id = $1 and object_id = $4),
            (select version_no from object_batch_results where tenant_id = $1 and object_id = $4),
            (select count(*)::bigint from object_batch_results where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&saved_id)
    .bind(&conflict_id)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("SAVED"));
    assert_eq!(row.1, Some(2));
    assert_eq!(row.2.as_deref(), Some("CONFLICT"));
    assert_eq!(row.3, Some(4));
    assert_eq!(row.4, 0);
    eprintln!(
        "object-batch-ok tenant={owner} foreign={foreign} actor={owner_actor} saved={saved_id} conflict={conflict_id}"
    );
    Ok(())
}

#[tokio::test]
async fn invalid_last_line_refuses_document_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists cell_edits (
          tenant_id uuid not null,
          edit_id uuid not null,
          document_no text not null,
          row_id text not null,
          column_id text not null,
          unit_code text not null,
          qty_micros bigint not null,
          display_text text not null,
          confirmed boolean not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, edit_id)
        )",
        "create table if not exists document_save_results (
          tenant_id uuid not null,
          document_no text not null,
          line_count bigint not null,
          decision text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, document_no)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("document-refuse tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u04h-owner"),
        (foreign, foreign_actor, "u04h-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("document-refuse-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u04h-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u04h",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u04h-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u04h",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let document_no = format!("SO-U04H-{owner}");
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/document-saves?document_no={document_no}"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let refused = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/document-saves"),
        Some(
            serde_json::json!({
                "request_id": "u04h-refuse",
                "document_no": document_no,
                "line_count": 100,
                "valid_count": 99
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(refused.0, StatusCode::OK, "{}", refused.1);
    assert!(refused.1.contains("row-100"), "{}", refused.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/document-saves?document_no={document_no}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["decision"].as_str().unwrap_or(""), "REFUSED");
    assert_eq!(read_body["saved_lines"].as_i64().unwrap_or(-1), 0);
    assert_eq!(read_body["invalid_row"].as_str().unwrap_or(""), "row-100");
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/document-saves?document_no={document_no}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("row-100"));
    let row: (Option<String>, Option<i64>, Option<String>, i64, i64) = sqlx::query_as(
        "select
            (select decision from document_save_results where tenant_id = $1 and document_no = $3),
            (select line_count from document_save_results where tenant_id = $1 and document_no = $3),
            (select effect_id from command_receipts where tenant_id = $1 and request_id = $4 and status = 'FAILED'),
            (select count(*)::bigint from cell_edits where tenant_id = $1 and document_no = $3),
            (select count(*)::bigint from document_save_results where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(&document_no)
    .bind(format!("refused:{document_no}"))
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("REFUSED"));
    assert_eq!(row.1, Some(0));
    assert_eq!(row.2.as_deref(), Some("row-100"));
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 0);
    eprintln!(
        "document-refuse-ok tenant={owner} foreign={foreign} actor={owner_actor} document={document_no} row={}",
        row.2.as_deref().unwrap_or("")
    );
    Ok(())
}

#[tokio::test]
async fn inverted_amount_keeps_input_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists search_conditions (
          tenant_id uuid not null,
          condition_id uuid not null,
          user_id text not null,
          page_code text not null,
          kind_code text not null,
          join_code text not null,
          field_name text not null,
          low_text text not null,
          high_text text not null,
          decision text not null,
          generation_no bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, condition_id)
        )",
        "create table if not exists search_resets (
          tenant_id uuid not null,
          reset_id uuid not null,
          page_code text not null,
          decision text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, reset_id)
        )",
        "create table if not exists query_generations (
          tenant_id uuid not null,
          query_id uuid not null,
          generation bigint not null,
          condition_digest text not null,
          cancelled boolean not null default false,
          applied boolean not null default false,
          result_text text not null default '',
          created_at timestamptz not null default now(),
          primary key (tenant_id, query_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("inverted-amount tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u03h-owner"),
        (foreign, foreign_actor, "u03h-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("inverted-amount-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u03h-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u03h",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u03h-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u03h",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/search-conditions?page=ORDERS"),
        None,
    )
    .await?;
    assert_ne!(missing.0, StatusCode::OK, "{}", missing.1);
    let kept = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/search-conditions"),
        Some(
            serde_json::json!({
                "request_id": "u03h-kept",
                "user_id": format!("user-{owner_actor}"),
                "page": "ORDERS",
                "kind": "AMOUNT",
                "join": "AND",
                "field": "amount",
                "low": "500",
                "high": "100",
                "generation": 1
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(kept.0, StatusCode::OK, "{}", kept.1);
    let kept_body: Value = serde_json::from_str(&kept.1).unwrap_or(Value::Null);
    assert_eq!(kept_body["effect_id"].as_str().unwrap_or(""), "KEPT");
    let active = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/search-conditions"),
        Some(
            serde_json::json!({
                "request_id": "u03h-query",
                "user_id": format!("user-{owner_actor}"),
                "page": "ORDERS",
                "kind": "AMOUNT",
                "join": "AND",
                "field": "amount",
                "low": "10",
                "high": "20",
                "generation": 2
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(active.0, StatusCode::OK, "{}", active.1);
    let opened = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/query-generations"),
        Some(
            serde_json::json!({
                "request_id": "u03h-open-old",
                "scope": "ORDERS",
                "generation": 1,
                "digest": "amount-500-100"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let opened_body: Value = serde_json::from_str(&opened.1).unwrap_or(Value::Null);
    let old_query = opened_body["query_id"].as_str().unwrap_or("").to_string();
    assert!(!old_query.is_empty(), "{}", opened.1);
    let newer = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/query-generations"),
        Some(
            serde_json::json!({
                "request_id": "u03h-open-new",
                "scope": "ORDERS",
                "generation": 2,
                "digest": "amount-10-20"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(newer.0, StatusCode::OK, "{}", newer.1);
    let newer_body: Value = serde_json::from_str(&newer.1).unwrap_or(Value::Null);
    let new_query = newer_body["query_id"].as_str().unwrap_or("").to_string();
    assert!(!new_query.is_empty(), "{}", newer.1);
    let current = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/query-generations/complete"),
        Some(
            serde_json::json!({
                "request_id": "u03h-current",
                "query_id": new_query,
                "result": "current-10-20"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(current.0, StatusCode::OK, "{}", current.1);
    let current_body: Value = serde_json::from_str(&current.1).unwrap_or(Value::Null);
    assert_eq!(current_body["effect_id"].as_str().unwrap_or(""), "applied");
    let late = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/query-generations/complete"),
        Some(
            serde_json::json!({
                "request_id": "u03h-late",
                "query_id": old_query,
                "result": "late-500-100"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(late.0, StatusCode::OK, "{}", late.1);
    let late_body: Value = serde_json::from_str(&late.1).unwrap_or(Value::Null);
    assert_eq!(late_body["effect_id"].as_str().unwrap_or(""), "stale");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/search-conditions?page=ORDERS"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["decision"].as_str().unwrap_or(""), "KEPT");
    assert_eq!(read_body["low"].as_str().unwrap_or(""), "500");
    assert_eq!(read_body["high"].as_str().unwrap_or(""), "100");
    assert_eq!(
        read_body["late_result"].as_str().unwrap_or(""),
        "late-500-100"
    );
    assert_eq!(
        read_body["current_result"].as_str().unwrap_or(""),
        "current-10-20"
    );
    let reset = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/search-resets"),
        Some(
            serde_json::json!({
                "request_id": "u03h-reset",
                "page": "ORDERS",
                "previous": "500-100"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(reset.0, StatusCode::OK, "{}", reset.1);
    let reset_body: Value = serde_json::from_str(&reset.1).unwrap_or(Value::Null);
    assert_eq!(reset_body["effect_id"].as_str().unwrap_or(""), "CLEARED");
    let after = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/search-conditions?page=ORDERS"),
        None,
    )
    .await?;
    assert_ne!(after.0, StatusCode::OK, "{}", after.1);
    assert!(!after.1.contains("late-500-100"));
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/search-conditions?page=ORDERS"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("500"));
    assert!(!foreign_read.1.contains("late-500-100"));
    let row: (Option<String>, Option<String>, Option<bool>, Option<String>, Option<String>, i64) = sqlx::query_as(
        "select
            (select decision from search_conditions where tenant_id = $1 and field_name = 'amount' and generation_no = 0),
            (select low_text || '-' || high_text from search_conditions where tenant_id = $1 and field_name = 'amount' and generation_no = 0),
            (select applied from query_generations where tenant_id = $1 and query_id = $3),
            (select result_text from query_generations where tenant_id = $1 and query_id = $3),
            (select result_text from query_generations where tenant_id = $1 and query_id = $4),
            (select count(*)::bigint from search_conditions where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&old_query)?)
    .bind(Uuid::parse_str(&new_query)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0.as_deref(), Some("CLEARED"));
    assert_eq!(row.1.as_deref(), Some("-"));
    assert_eq!(row.2, Some(false));
    assert_eq!(row.3.as_deref(), Some("late-500-100"));
    assert_eq!(row.4.as_deref(), Some("current-10-20"));
    assert_eq!(row.5, 0);
    eprintln!(
        "inverted-amount-ok tenant={owner} foreign={foreign} actor={owner_actor} late={} current={}",
        row.3.as_deref().unwrap_or(""),
        row.4.as_deref().unwrap_or("")
    );
    Ok(())
}

#[tokio::test]
async fn grid_page_after_500_keeps_cross_page_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists grid_queries (
          tenant_id uuid not null,
          query_id uuid not null,
          page_code text not null,
          filter_text text not null,
          sort_key text not null,
          tie_break text not null,
          page_offset bigint not null,
          page_limit integer not null,
          loaded_count bigint not null,
          matched_count bigint not null,
          count_kind text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, query_id)
        )",
        "create table if not exists grid_selections (
          tenant_id uuid not null,
          selection_id uuid not null,
          query_id uuid not null,
          selection_kind text not null,
          row_id text not null,
          snapshot_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, selection_id)
        )",
        "create table if not exists grid_batch_gates (
          tenant_id uuid not null,
          gate_id uuid not null,
          current_query text not null,
          requested_query text not null,
          decision text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, gate_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("grid-page tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u02h-owner"),
        (foreign, foreign_actor, "u02h-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("grid-page-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u02h-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u02h",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u02h-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u02h",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let saved = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/grid-queries"),
        Some(
            serde_json::json!({
                "request_id": "u02h-page",
                "page": "ORDERS",
                "filter": "status=open",
                "sort_key": "date",
                "row_id": "row-501",
                "offset": 500,
                "limit": 500,
                "loaded": 1,
                "matched": 1000,
                "exact": true
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let saved_body: Value = serde_json::from_str(&saved.1).unwrap_or(Value::Null);
    let query_id = saved_body["query_id"].as_str().unwrap_or("").to_string();
    assert!(!query_id.is_empty(), "{}", saved.1);
    let selected = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/grid-selections"),
        Some(
            serde_json::json!({
                "request_id": "u02h-cross",
                "query_id": query_id,
                "kind": "CROSS_PAGE",
                "row_id": "row-501",
                "snapshot": "",
                "excluded": 0,
                "loaded": 1,
                "matched": 1000
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(selected.0, StatusCode::OK, "{}", selected.1);
    let stale = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/grid-batch-gates"),
        Some(
            serde_json::json!({
                "request_id": "u02h-stale",
                "current": "status=open",
                "requested": "status=closed"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stale.0, StatusCode::OK, "{}", stale.1);
    let stale_body: Value = serde_json::from_str(&stale.1).unwrap_or(Value::Null);
    assert_eq!(stale_body["effect_id"].as_str().unwrap_or(""), "BLOCKED");
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/grid-queries?query_id={query_id}"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["offset"].as_i64().unwrap_or(0), 500);
    assert_eq!(read_body["loaded"].as_i64().unwrap_or(0), 1);
    assert_eq!(read_body["matched"].as_i64().unwrap_or(0), 1000);
    let rows = read_body["cross_page_rows"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].as_str().unwrap_or(""), "row-501");
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/grid-queries?query_id={query_id}"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("row-501"));
    let row: (i64, i64, String, String, i64) = sqlx::query_as(
        "select
            (select page_offset from grid_queries where tenant_id = $1 and query_id = $3),
            (select matched_count from grid_queries where tenant_id = $1 and query_id = $3),
            (select row_id from grid_selections where tenant_id = $1 and query_id = $3 and selection_kind = 'CROSS_PAGE'),
            (select decision from grid_batch_gates where tenant_id = $1 and current_query = 'status=open' and requested_query = 'status=closed'),
            (select count(*)::bigint from grid_queries where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(&query_id)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 500);
    assert_eq!(row.1, 1000);
    assert_eq!(row.2, "row-501");
    assert_eq!(row.3, "BLOCKED");
    assert_eq!(row.4, 0);
    eprintln!(
        "grid-page-ok tenant={owner} foreign={foreign} actor={owner_actor} query={query_id} row={}",
        row.2
    );
    Ok(())
}

#[tokio::test]
async fn page_instance_isolation_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists page_instances (
          tenant_id uuid not null,
          page_row_id uuid not null,
          user_id text not null,
          page_code text not null,
          instance_code text not null,
          service_code text not null,
          theme_code text not null,
          left_hidden boolean not null,
          right_hidden boolean not null,
          subtable_hidden boolean not null,
          query_text text not null,
          draft_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, page_row_id),
          unique (tenant_id, user_id, page_code, instance_code)
        )",
        "create table if not exists page_close_tasks (
          tenant_id uuid not null,
          task_id uuid not null,
          page_code text not null,
          instance_code text not null,
          task_kind text not null,
          decision text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, task_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("page-instance tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "u01g-owner"),
        (foreign, foreign_actor, "u01g-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'orders.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("page-instance-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("u01g-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-u01g",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("u01g-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-u01g",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let first = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/page-instances"),
        Some(
            serde_json::json!({
                "request_id": "u01g-a",
                "page": "TRADE",
                "instance": "tab-a",
                "service": "sales.orders",
                "theme": "DARK",
                "left_hidden": false,
                "right_hidden": false,
                "subtable_hidden": false,
                "query": "SO-A",
                "draft": "draft-a"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let second = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/page-instances"),
        Some(
            serde_json::json!({
                "request_id": "u01g-b",
                "page": "TRADE",
                "instance": "tab-b",
                "service": "sales.orders",
                "theme": "LIGHT",
                "left_hidden": false,
                "right_hidden": false,
                "subtable_hidden": false,
                "query": "SO-B",
                "draft": "draft-b"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(second.0, StatusCode::OK, "{}", second.1);
    let hide = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/page-panels/hides"),
        Some(
            serde_json::json!({
                "request_id": "u01g-hide",
                "page": "TRADE",
                "instance": "tab-a",
                "panel": "LEFT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(hide.0, StatusCode::OK, "{}", hide.1);
    let read_a = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/page-instances?page=TRADE&instance=tab-a"),
        None,
    )
    .await?;
    assert_eq!(read_a.0, StatusCode::OK, "{}", read_a.1);
    let body_a: Value = serde_json::from_str(&read_a.1).unwrap_or(Value::Null);
    assert_eq!(body_a["query"].as_str().unwrap_or(""), "SO-A");
    assert_eq!(body_a["draft"].as_str().unwrap_or(""), "draft-a");
    assert_eq!(body_a["left_hidden"].as_bool().unwrap_or(false), true);
    assert_eq!(body_a["theme"].as_str().unwrap_or(""), "DARK");
    let read_b = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/page-instances?page=TRADE&instance=tab-b"),
        None,
    )
    .await?;
    assert_eq!(read_b.0, StatusCode::OK, "{}", read_b.1);
    let body_b: Value = serde_json::from_str(&read_b.1).unwrap_or(Value::Null);
    assert_eq!(body_b["query"].as_str().unwrap_or(""), "SO-B");
    assert_eq!(body_b["draft"].as_str().unwrap_or(""), "draft-b");
    assert_eq!(body_b["left_hidden"].as_bool().unwrap_or(true), false);
    let close = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/page-closes"),
        Some(
            serde_json::json!({
                "request_id": "u01g-close",
                "page": "TRADE",
                "instance": "tab-a",
                "task_kind": "WRITE",
                "dirty": true,
                "accepted_write": true
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(close.0, StatusCode::OK, "{}", close.1);
    let close_body: Value = serde_json::from_str(&close.1).unwrap_or(Value::Null);
    assert_eq!(close_body["effect_id"].as_str().unwrap_or(""), "WRITE_KEPT");
    let after = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/page-instances?page=TRADE&instance=tab-a"),
        None,
    )
    .await?;
    assert_eq!(after.0, StatusCode::OK, "{}", after.1);
    let after_body: Value = serde_json::from_str(&after.1).unwrap_or(Value::Null);
    assert_eq!(after_body["draft"].as_str().unwrap_or(""), "draft-a");
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/page-instances?page=TRADE&instance=tab-a"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("draft-a"));
    let row: (String, String, bool, String, i64) = sqlx::query_as(
        "select
            (select query_text from page_instances where tenant_id = $1 and instance_code = 'tab-a'),
            (select query_text from page_instances where tenant_id = $1 and instance_code = 'tab-b'),
            (select left_hidden from page_instances where tenant_id = $1 and instance_code = 'tab-a'),
            (select decision from page_close_tasks where tenant_id = $1 and instance_code = 'tab-a' and task_kind = 'WRITE'),
            (select count(*)::bigint from page_instances where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "SO-A");
    assert_eq!(row.1, "SO-B");
    assert!(row.2);
    assert_eq!(row.3, "WRITE_KEPT");
    assert_eq!(row.4, 0);
    eprintln!(
        "page-instance-ok tenant={owner} foreign={foreign} actor={owner_actor} query={} decision={}",
        row.0, row.3
    );
    Ok(())
}

#[tokio::test]
async fn heavy_flood_keeps_other_order_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists work_admissions (
          tenant_id uuid not null,
          admission_id uuid not null,
          kind text not null,
          status text not null,
          retry_after_seconds bigint not null,
          command_id text not null,
          created_at timestamptz not null default now(),
          closed_at timestamptz,
          primary key (tenant_id, admission_id)
        )",
        "alter table work_admissions add column if not exists closed_at timestamptz",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    sqlx::query(
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
    )
    .execute(&pool)
    .await?;
    let owner = Uuid::new_v4();
    let other = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let other_actor = Uuid::new_v4();
    let queue_pool = super::qa_database::isolated_admission_pool(&pool).await?;
    let independent = queue_pool.clone();
    eprintln!("heavy-flood tenant={owner} other={other}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a10n-owner"),
        (other, other_actor, "a10n-other"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'MFG.PRODUCTION.WRITE')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("heavy-flood-test".into()));
    let app = build_app(AppState::new(
        Db::from_pool(queue_pool),
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a10n-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a10n",
    )
    .await?;
    let other_login = login_http(
        &app,
        other,
        &format!("a10n-other-{other_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a10n",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(other_login.0, StatusCode::OK, "{}", other_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let other_token = other_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    for (request, command) in [("a10n-h1", "cmd-a10n-h1"), ("a10n-h2", "cmd-a10n-h2")] {
        let queued = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/work/admissions"),
            Some(
                serde_json::json!({
                    "request_id": request,
                    "kind": "HEAVY",
                    "command_id": command,
                    "retry_after_seconds": 30
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(queued.0, StatusCode::OK, "{}", queued.1);
        let body: Value = serde_json::from_str(&queued.1).unwrap_or(Value::Null);
        assert_eq!(body["status"].as_str().unwrap_or(""), "QUEUED");
    }
    let third = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/work/admissions"),
        Some(
            serde_json::json!({
                "request_id": "a10n-h3",
                "kind": "HEAVY",
                "command_id": "cmd-a10n-h3",
                "retry_after_seconds": 30
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(third.0, StatusCode::TOO_MANY_REQUESTS, "{}", third.1);
    let third_body: Value = serde_json::from_str(&third.1).unwrap_or(Value::Null);
    assert_eq!(third_body["status"].as_str().unwrap_or(""), "429");
    assert_eq!(
        third_body["command_id"].as_str().unwrap_or(""),
        "cmd-a10n-h3"
    );
    let other_heavy = export_http(
        &app,
        &other_token,
        "POST",
        &format!("/v1/tenants/{other}/work/admissions"),
        Some(
            serde_json::json!({
                "request_id": "a10n-other-heavy",
                "kind": "HEAVY",
                "command_id": "cmd-a10n-other-heavy",
                "retry_after_seconds": 45
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(
        other_heavy.0,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        other_heavy.1
    );
    let other_heavy_body: Value = serde_json::from_str(&other_heavy.1).unwrap_or(Value::Null);
    assert_eq!(other_heavy_body["status"].as_str().unwrap_or(""), "429");
    assert_eq!(
        other_heavy_body["command_id"].as_str().unwrap_or(""),
        "cmd-a10n-other-heavy"
    );
    let other_order = export_http(
        &app,
        &other_token,
        "POST",
        &format!("/v1/tenants/{other}/work/admissions"),
        Some(
            serde_json::json!({
                "request_id": "a10n-other-order",
                "kind": "ORDER",
                "command_id": "cmd-a10n-other-order",
                "retry_after_seconds": 45
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(other_order.0, StatusCode::OK, "{}", other_order.1);
    let other_order_body: Value = serde_json::from_str(&other_order.1).unwrap_or(Value::Null);
    assert_eq!(
        other_order_body["status"].as_str().unwrap_or(""),
        "ACCEPTED"
    );
    let foreign_on_owner = export_http(
        &app,
        &other_token,
        "POST",
        &format!("/v1/tenants/{owner}/work/admissions"),
        Some(
            serde_json::json!({
                "request_id": "a10n-cross",
                "kind": "ORDER",
                "command_id": "cmd-a10n-cross",
                "retry_after_seconds": 45
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(foreign_on_owner.0, StatusCode::OK, "{}", foreign_on_owner.1);
    assert_ne!(
        foreign_on_owner.0,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        foreign_on_owner.1
    );
    let row: (i64, i64, String, i64) = sqlx::query_as(
        "select
            (select count(*)::bigint from work_admissions where tenant_id = $1 and kind = 'HEAVY' and status = 'QUEUED' and closed_at is null),
            (select count(*)::bigint from work_admissions where tenant_id = $1 and command_id = 'cmd-a10n-h3' and status = '429'),
            (select status from work_admissions where tenant_id = $2 and command_id = 'cmd-a10n-other-order'),
            (select count(*)::bigint from work_admissions where tenant_id = $2 and kind = 'HEAVY' and status = 'QUEUED' and closed_at is null)",
    )
    .bind(owner)
    .bind(other)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 2);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, "ACCEPTED");
    assert_eq!(row.3, 0);
    eprintln!(
        "heavy-flood-ok tenant={owner} other={other} actor={owner_actor} queued={} other_order={}",
        row.0, row.2
    );
    Ok(())
}

#[tokio::test]
async fn usage_notice_read_does_not_charge_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists usage_quotas (
          tenant_id uuid not null,
          quota_id uuid not null,
          resource_kind text not null,
          quota_qty bigint not null,
          task_cap integer not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, quota_id),
          unique (tenant_id, resource_kind)
        )",
        "create table if not exists usage_meters (
          tenant_id uuid not null,
          meter_id uuid not null,
          task_id text not null,
          resource_kind text not null,
          quantity bigint not null,
          window_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, meter_id),
          unique (tenant_id, task_id, resource_kind, window_text)
        )",
        "create table if not exists usage_notices (
          tenant_id uuid not null,
          notice_id uuid not null,
          resource_kind text not null,
          decision text not null,
          expand_option text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, notice_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("usage-notice-read tenant={owner} foreign={foreign}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a10m-owner"),
        (foreign, foreign_actor, "a10m-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'exports.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("usage-notice-read-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a10m-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a10m",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a10m-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a10m",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let missing = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-notices?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_eq!(missing.0, StatusCode::NOT_FOUND, "{}", missing.1);
    let quota = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-quotas"),
        Some(
            serde_json::json!({
                "request_id": "a10m-quota",
                "kind": "EXPORT",
                "quota": 10,
                "task_cap": 2,
                "feature_code": "PAID-SEAT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(quota.0, StatusCode::OK, "{}", quota.1);
    let meter = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-meters"),
        Some(
            serde_json::json!({
                "request_id": "a10m-meter",
                "task": "task-a10m-once",
                "kind": "EXPORT",
                "quantity": 6,
                "window": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(meter.0, StatusCode::OK, "{}", meter.1);
    let inside = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-notices"),
        Some(
            serde_json::json!({
                "request_id": "a10m-inside",
                "kind": "EXPORT",
                "window": "2026-10",
                "adding": 1,
                "offered": 20
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(inside.0, StatusCode::OK, "{}", inside.1);
    let notice = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-notices"),
        Some(
            serde_json::json!({
                "request_id": "a10m-notice",
                "kind": "EXPORT",
                "window": "2026-10",
                "adding": 9,
                "offered": 20
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(notice.0, StatusCode::OK, "{}", notice.1);
    let notice_body: Value = serde_json::from_str(&notice.1).unwrap_or(Value::Null);
    assert_eq!(
        notice_body["effect_id"].as_str().unwrap_or(""),
        "OVER_LIMIT"
    );
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-notices?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["decision"].as_str().unwrap_or(""), "OVER_LIMIT");
    assert_eq!(read_body["expand"].as_str().unwrap_or(""), "EXPAND");
    let meters = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-meters?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_eq!(meters.0, StatusCode::OK, "{}", meters.1);
    let meters_body: Value = serde_json::from_str(&meters.1).unwrap_or(Value::Null);
    assert_eq!(meters_body["used"].as_i64().unwrap_or(0), 6);
    assert_eq!(meters_body["tasks"].as_i64().unwrap_or(0), 1);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-notices?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("OVER_LIMIT"));
    let row: (i64, String, String, i64, i64) = sqlx::query_as(
        "select
            (select count(*)::bigint from usage_notices where tenant_id = $1),
            (select decision from usage_notices where tenant_id = $1 and resource_kind = 'EXPORT'),
            (select expand_option from usage_notices where tenant_id = $1 and resource_kind = 'EXPORT'),
            (select coalesce(sum(quantity), 0)::bigint from usage_meters where tenant_id = $1),
            (select count(*)::bigint from usage_notices where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 1);
    assert_eq!(row.1, "OVER_LIMIT");
    assert_eq!(row.2, "EXPAND");
    assert_eq!(row.3, 6);
    assert_eq!(row.4, 0);
    eprintln!(
        "usage-notice-read-ok tenant={owner} foreign={foreign} actor={owner_actor} used={} decision={}",
        row.3, row.1
    );
    Ok(())
}

#[tokio::test]
async fn usage_replay_keeps_one_charge_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists usage_quotas (
          tenant_id uuid not null,
          quota_id uuid not null,
          resource_kind text not null,
          quota_qty bigint not null,
          task_cap integer not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, quota_id),
          unique (tenant_id, resource_kind)
        )",
        "create table if not exists usage_meters (
          tenant_id uuid not null,
          meter_id uuid not null,
          task_id text not null,
          resource_kind text not null,
          quantity bigint not null,
          window_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, meter_id),
          unique (tenant_id, task_id, resource_kind, window_text)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("usage-replay tenant={owner} foreign={foreign}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a10l-owner"),
        (foreign, foreign_actor, "a10l-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'exports.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("usage-replay-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a10l-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a10l",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a10l-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a10l",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let quota = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-quotas"),
        Some(
            serde_json::json!({
                "request_id": "a10l-quota",
                "kind": "EXPORT",
                "quota": 100,
                "task_cap": 2,
                "feature_code": "PAID-SEAT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(quota.0, StatusCode::OK, "{}", quota.1);
    let first = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-meters"),
        Some(
            serde_json::json!({
                "request_id": "a10l-charge",
                "task": "task-a10l-once",
                "kind": "EXPORT",
                "quantity": 5,
                "window": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let first_body: Value = serde_json::from_str(&first.1).unwrap_or(Value::Null);
    assert_eq!(first_body["replay"].as_bool().unwrap_or(true), false);
    let same = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-meters"),
        Some(
            serde_json::json!({
                "request_id": "a10l-charge",
                "task": "task-a10l-once",
                "kind": "EXPORT",
                "quantity": 5,
                "window": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(same.0, StatusCode::OK, "{}", same.1);
    let same_body: Value = serde_json::from_str(&same.1).unwrap_or(Value::Null);
    assert_eq!(same_body["replay"].as_bool().unwrap_or(false), true);
    let again = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-meters"),
        Some(
            serde_json::json!({
                "request_id": "a10l-again",
                "task": "task-a10l-once",
                "kind": "EXPORT",
                "quantity": 9,
                "window": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(again.0, StatusCode::OK, "{}", again.1);
    let again_body: Value = serde_json::from_str(&again.1).unwrap_or(Value::Null);
    assert_eq!(again_body["replay"].as_bool().unwrap_or(false), true);
    assert_eq!(
        again_body["effect_id"].as_str().unwrap_or(""),
        "KEPT_CHARGE:5"
    );
    let foreign_charge = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-meters"),
        Some(
            serde_json::json!({
                "request_id": "a10l-foreign",
                "task": "task-a10l-once",
                "kind": "EXPORT",
                "quantity": 5,
                "window": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(foreign_charge.0, StatusCode::OK, "{}", foreign_charge.1);
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-meters?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["quota"].as_i64().unwrap_or(0), 100);
    assert_eq!(read_body["used"].as_i64().unwrap_or(0), 5);
    assert_eq!(read_body["tasks"].as_i64().unwrap_or(0), 1);
    let row: (i64, i64, String, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(quantity), 0)::bigint from usage_meters where tenant_id = $1 and resource_kind = 'EXPORT'),
            (select count(*)::bigint from usage_meters where tenant_id = $1 and task_id = 'task-a10l-once'),
            (select effect_id from command_receipts where tenant_id = $1 and request_id = 'a10l-again'),
            (select count(*)::bigint from usage_meters where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 5);
    assert_eq!(row.1, 1);
    assert_eq!(row.2, "KEPT_CHARGE:5");
    assert_eq!(row.3, 0);
    eprintln!(
        "usage-replay-ok tenant={owner} foreign={foreign} actor={owner_actor} used={} effect={}",
        row.0, row.2
    );
    Ok(())
}

#[tokio::test]
async fn usage_task_cap_keeps_admitted_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists usage_quotas (
          tenant_id uuid not null,
          quota_id uuid not null,
          resource_kind text not null,
          quota_qty bigint not null,
          task_cap integer not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, quota_id),
          unique (tenant_id, resource_kind)
        )",
        "create table if not exists usage_meters (
          tenant_id uuid not null,
          meter_id uuid not null,
          task_id text not null,
          resource_kind text not null,
          quantity bigint not null,
          window_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, meter_id),
          unique (tenant_id, task_id, resource_kind, window_text)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("usage-cap tenant={owner} foreign={foreign}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a10k-owner"),
        (foreign, foreign_actor, "a10k-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'exports.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("usage-cap-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a10k-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a10k",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a10k-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a10k",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let quota = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-quotas"),
        Some(
            serde_json::json!({
                "request_id": "a10k-quota",
                "kind": "EXPORT",
                "quota": 100,
                "task_cap": 2,
                "feature_code": "PAID-SEAT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(quota.0, StatusCode::OK, "{}", quota.1);
    for (request, task, quantity) in [
        ("a10k-one", "task-a10k-one", 3),
        ("a10k-two", "task-a10k-two", 4),
    ] {
        let saved = export_http(
            &app,
            &owner_token,
            "POST",
            &format!("/v1/tenants/{owner}/usage-meters"),
            Some(
                serde_json::json!({
                    "request_id": request,
                    "task": task,
                    "kind": "EXPORT",
                    "quantity": quantity,
                    "window": "2026-10"
                })
                .to_string(),
            ),
        )
        .await?;
        assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    }
    let third = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-meters"),
        Some(
            serde_json::json!({
                "request_id": "a10k-third",
                "task": "task-a10k-third",
                "kind": "EXPORT",
                "quantity": 1,
                "window": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(third.0, StatusCode::OK, "{}", third.1);
    assert!(!third.1.contains("task-a10k-third"));
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-meters?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["quota"].as_i64().unwrap_or(0), 100);
    assert_eq!(read_body["used"].as_i64().unwrap_or(0), 7);
    assert_eq!(read_body["tasks"].as_i64().unwrap_or(0), 2);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-meters?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    let row: (i64, i64, String, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(quantity), 0)::bigint from usage_meters where tenant_id = $1 and resource_kind = 'EXPORT'),
            (select count(*)::bigint from usage_meters where tenant_id = $1 and task_id = 'task-a10k-third'),
            (select status from command_receipts where tenant_id = $1 and request_id = 'a10k-third'),
            (select count(*)::bigint from usage_meters where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 7);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, "FAILED");
    assert_eq!(row.3, 0);
    eprintln!(
        "usage-cap-ok tenant={owner} foreign={foreign} actor={owner_actor} used={} receipt={}",
        row.0, row.2
    );
    Ok(())
}

#[tokio::test]
async fn usage_over_limit_keeps_admitted_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists usage_quotas (
          tenant_id uuid not null,
          quota_id uuid not null,
          resource_kind text not null,
          quota_qty bigint not null,
          task_cap integer not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, quota_id),
          unique (tenant_id, resource_kind)
        )",
        "create table if not exists usage_meters (
          tenant_id uuid not null,
          meter_id uuid not null,
          task_id text not null,
          resource_kind text not null,
          quantity bigint not null,
          window_text text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, meter_id),
          unique (tenant_id, task_id, resource_kind, window_text)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    eprintln!("usage-keep tenant={owner} foreign={foreign}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a10j-owner"),
        (foreign, foreign_actor, "a10j-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'exports.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("usage-keep-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a10j-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a10j",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a10j-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a10j",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let quota = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-quotas"),
        Some(
            serde_json::json!({
                "request_id": "a10j-quota",
                "kind": "EXPORT",
                "quota": 10,
                "task_cap": 2,
                "feature_code": "PAID-SEAT"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(quota.0, StatusCode::OK, "{}", quota.1);
    let admitted = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-meters"),
        Some(
            serde_json::json!({
                "request_id": "a10j-admit",
                "task": "task-a10j-admit",
                "kind": "EXPORT",
                "quantity": 6,
                "window": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(admitted.0, StatusCode::OK, "{}", admitted.1);
    let over = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/usage-meters"),
        Some(
            serde_json::json!({
                "request_id": "a10j-over",
                "task": "task-a10j-over",
                "kind": "EXPORT",
                "quantity": 5,
                "window": "2026-10"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(over.0, StatusCode::OK, "{}", over.1);
    assert!(!over.1.contains("task-a10j-over"));
    let read = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-meters?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_eq!(read.0, StatusCode::OK, "{}", read.1);
    let read_body: Value = serde_json::from_str(&read.1).unwrap_or(Value::Null);
    assert_eq!(read_body["quota"].as_i64().unwrap_or(0), 10);
    assert_eq!(read_body["used"].as_i64().unwrap_or(0), 6);
    assert_eq!(read_body["tasks"].as_i64().unwrap_or(0), 1);
    let foreign_read = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/usage-meters?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_ne!(foreign_read.0, StatusCode::OK, "{}", foreign_read.1);
    assert!(!foreign_read.1.contains("task-a10j-admit"));
    let foreign_own = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/v1/tenants/{foreign}/usage-meters?kind=EXPORT&window=2026-10"),
        None,
    )
    .await?;
    assert_eq!(foreign_own.0, StatusCode::OK, "{}", foreign_own.1);
    let foreign_body: Value = serde_json::from_str(&foreign_own.1).unwrap_or(Value::Null);
    assert_eq!(foreign_body["used"].as_i64().unwrap_or(-1), 0);
    assert_eq!(foreign_body["tasks"].as_i64().unwrap_or(-1), 0);
    let row: (i64, i64, String, i64) = sqlx::query_as(
        "select
            (select coalesce(sum(quantity), 0)::bigint from usage_meters where tenant_id = $1 and resource_kind = 'EXPORT'),
            (select count(*)::bigint from usage_meters where tenant_id = $1 and task_id = 'task-a10j-over'),
            (select status from command_receipts where tenant_id = $1 and request_id = 'a10j-over'),
            (select count(*)::bigint from usage_meters where tenant_id = $2)",
    )
    .bind(owner)
    .bind(foreign)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 6);
    assert_eq!(row.1, 0);
    assert_eq!(row.2, "FAILED");
    assert_eq!(row.3, 0);
    eprintln!(
        "usage-keep-ok tenant={owner} foreign={foreign} actor={owner_actor} used={} receipt={}",
        row.0, row.2
    );
    Ok(())
}

#[tokio::test]
async fn unfinished_exit_hides_taken_marker_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists export_jobs (
          job_id uuid primary key,
          tenant_id uuid not null,
          job_type text not null,
          params_json jsonb not null default '{}'::jsonb,
          status text not null default 'PENDING',
          output_url text,
          progress int not null default 0,
          attempts int not null default 0,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )",
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists exit_requests (
          tenant_id uuid not null,
          request_row_id uuid not null,
          applicant_id text not null,
          confirmation_text text not null,
          cutoff_text text not null,
          file_range text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_row_id)
        )",
        "create table if not exists exit_manifests (
          tenant_id uuid not null,
          manifest_id uuid not null,
          item_kind text not null,
          item_no text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, manifest_id)
        )",
        "alter table exit_manifests add column if not exists line_count bigint",
        "alter table exit_manifests add column if not exists digest text",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let job = Uuid::new_v4();
    let marker = format!("TAKEN-A09M-{job}");
    eprintln!("unfinished-exit tenant={owner} foreign={foreign} job={job}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a09m-owner"),
        (foreign, foreign_actor, "a09m-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'exports.read'), ($1, $2, 'exports.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into export_jobs(job_id, tenant_id, job_type, params_json, status, output_url, progress) values ($1, $2, 'ORDER_EXPORT', '{}'::jsonb, 'PENDING', $3, 10)",
    )
    .bind(job)
    .bind(owner)
    .bind(&marker)
    .execute(&pool)
    .await?;
    let export_dir = std::env::temp_dir().join(format!("erp-a09m-{owner}"));
    std::fs::create_dir_all(export_dir.join(owner.to_string()))?;
    std::fs::write(
        export_dir
            .join(owner.to_string())
            .join(format!("{job}.csv")),
        format!("{marker}\n"),
    )?;
    let previous_export_dir = std::env::var("EXPORT_BASE_DIR").ok();
    // Safety: this process is the acceptance test process. The shipped download
    // path reads EXPORT_BASE_DIR once per request, and the test restores it.
    unsafe {
        std::env::set_var("EXPORT_BASE_DIR", &export_dir);
    }
    let jwt = Arc::new(JwtService::new("unfinished-exit-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a09m-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a09m",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a09m-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a09m",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let pending = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}"),
        None,
    )
    .await?;
    assert_eq!(pending.0, StatusCode::OK, "{}", pending.1);
    let pending_body: Value = serde_json::from_str(&pending.1).unwrap_or(Value::Null);
    assert_eq!(pending_body["status"].as_str().unwrap_or(""), "PENDING");
    assert!(pending_body["output_url"].is_null());
    assert!(!pending.1.contains(&marker));
    let pending_file = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_ne!(pending_file.0, StatusCode::OK, "{}", pending_file.1);
    assert!(!pending_file.1.contains(&marker));
    let open = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/exit-requests"),
        Some(
            serde_json::json!({
                "request_id": "a09m-open",
                "applicant": "user-a09m",
                "confirmation": "confirm exit",
                "cutoff": "2026-10-04T00:00:00+08:00",
                "file_range": "MANIFEST",
                "wipe": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(open.0, StatusCode::OK, "{}", open.1);
    let listed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/exit-manifests"),
        Some(
            serde_json::json!({
                "request_id": "a09m-list",
                "item_kind": "EXPORT_FILE",
                "item_no": job.to_string(),
                "line_count": 1,
                "digest": format!("{:x}", sha2::Sha256::digest(format!("{marker}\n").as_bytes()))
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(listed.0, StatusCode::OK, "{}", listed.1);
    sqlx::query("update export_jobs set status = 'DONE', progress = 100 where tenant_id = $1 and job_id = $2")
        .bind(owner)
        .bind(job)
        .execute(&pool)
        .await?;
    let undelivered = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}"),
        None,
    )
    .await?;
    assert_eq!(undelivered.0, StatusCode::OK, "{}", undelivered.1);
    let undelivered_body: Value = serde_json::from_str(&undelivered.1).unwrap_or(Value::Null);
    assert_eq!(undelivered_body["status"].as_str().unwrap_or(""), "DONE");
    assert!(undelivered_body["output_url"].is_null());
    assert!(!undelivered.1.contains(&marker));
    let undelivered_file = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_eq!(
        undelivered_file.0,
        StatusCode::CONFLICT,
        "{}",
        undelivered_file.1
    );
    assert!(!undelivered_file.1.contains(&marker));
    let deliver = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/exit-requests/deliver"),
        Some(serde_json::json!({"request_id": "a09m-deliver", "wipe": false}).to_string()),
    )
    .await?;
    assert_eq!(deliver.0, StatusCode::OK, "{}", deliver.1);
    let delivered = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}"),
        None,
    )
    .await?;
    assert_eq!(delivered.0, StatusCode::OK, "{}", delivered.1);
    let delivered_body: Value = serde_json::from_str(&delivered.1).unwrap_or(Value::Null);
    assert_eq!(delivered_body["output_url"].as_str().unwrap_or(""), marker);
    let foreign_get = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/exports/jobs/{job}"),
        None,
    )
    .await?;
    assert_ne!(foreign_get.0, StatusCode::OK, "{}", foreign_get.1);
    assert!(!foreign_get.1.contains(&marker));
    if let Some(previous) = previous_export_dir {
        unsafe {
            std::env::set_var("EXPORT_BASE_DIR", previous);
        }
    } else {
        unsafe {
            std::env::remove_var("EXPORT_BASE_DIR");
        }
    }
    let _ = std::fs::remove_dir_all(&export_dir);
    let row: (String, String, String, i64) = sqlx::query_as(
        "select j.status, coalesce(j.output_url, ''), r.status,
            (select count(*) from exit_requests where tenant_id = $2)
         from export_jobs j
         join exit_requests r on r.tenant_id = j.tenant_id
         where j.tenant_id = $1 and j.job_id = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(job)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "DONE");
    assert_eq!(row.1, marker);
    assert_eq!(row.2, "DELIVERED");
    assert_eq!(row.3, 0);
    eprintln!(
        "unfinished-exit-ok tenant={owner} foreign={foreign} actor={owner_actor} job={job} status={}",
        row.2
    );
    Ok(())
}

#[tokio::test]
async fn exit_download_link_expiry_revoke_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists download_links (
          tenant_id uuid not null,
          link_id uuid not null,
          actor_user_id text not null,
          object_id text not null,
          expires_at timestamptz not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, link_id)
        )",
        "alter table download_links add column if not exists status text not null default 'ACTIVE'",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let object_id = format!("export-a09l-{owner}");
    eprintln!("exit-link tenant={owner} foreign={foreign} object={object_id}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a09l-owner"),
        (foreign, foreign_actor, "a09l-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'exports.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("exit-link-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a09l-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a09l",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a09l-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a09l",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let issued = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/download-links"),
        Some(serde_json::json!({"request_id": "a09l-issue", "object_id": object_id}).to_string()),
    )
    .await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let issued_body: Value = serde_json::from_str(&issued.1).unwrap_or(Value::Null);
    let link_id = issued_body["link_id"].as_str().unwrap_or("");
    assert!(!link_id.is_empty());
    assert!(!issued.1.contains("SO-A09L"));
    let allowed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/download-links/{link_id}/authorizations"),
        Some(serde_json::json!({"request_id": "a09l-allow", "object_id": object_id}).to_string()),
    )
    .await?;
    assert_eq!(allowed.0, StatusCode::OK, "{}", allowed.1);
    sqlx::query(
        "update download_links set expires_at = now() - interval '1 second' where tenant_id = $1 and link_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(link_id)?)
    .execute(&pool)
    .await?;
    let expired = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/download-links/{link_id}/authorizations"),
        Some(serde_json::json!({"request_id": "a09l-expired", "object_id": object_id}).to_string()),
    )
    .await?;
    assert_ne!(expired.0, StatusCode::OK, "{}", expired.1);
    assert!(!expired.1.contains(&object_id));
    sqlx::query(
        "update download_links set expires_at = now() + interval '15 minutes' where tenant_id = $1 and link_id = $2",
    )
    .bind(owner)
    .bind(Uuid::parse_str(link_id)?)
    .execute(&pool)
    .await?;
    let restored = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/download-links/{link_id}/authorizations"),
        Some(
            serde_json::json!({"request_id": "a09l-restored", "object_id": object_id}).to_string(),
        ),
    )
    .await?;
    assert_eq!(restored.0, StatusCode::OK, "{}", restored.1);
    let foreign_auth = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{owner}/download-links/{link_id}/revocations"),
        Some(serde_json::json!({"request_id": "a09l-foreign-revoke"}).to_string()),
    )
    .await?;
    assert_ne!(foreign_auth.0, StatusCode::OK, "{}", foreign_auth.1);
    let revoked = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/download-links/{link_id}/revocations"),
        Some(serde_json::json!({"request_id": "a09l-revoke"}).to_string()),
    )
    .await?;
    assert_eq!(revoked.0, StatusCode::OK, "{}", revoked.1);
    let after = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/download-links/{link_id}/authorizations"),
        Some(serde_json::json!({"request_id": "a09l-after", "object_id": object_id}).to_string()),
    )
    .await?;
    assert_ne!(after.0, StatusCode::OK, "{}", after.1);
    assert!(!after.1.contains(&object_id));
    let foreign_use = export_http(
        &app,
        &foreign_token,
        "POST",
        &format!("/v1/tenants/{foreign}/download-links/{link_id}/authorizations"),
        Some(serde_json::json!({"request_id": "a09l-foreign", "object_id": object_id}).to_string()),
    )
    .await?;
    assert_ne!(foreign_use.0, StatusCode::OK, "{}", foreign_use.1);
    let row: (String, i64, i64, i64) = sqlx::query_as(
        "select status,
            (select count(*) from download_links where tenant_id = $1),
            (select count(*) from download_links where tenant_id = $2),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'a09l-after')
         from download_links where tenant_id = $1 and link_id = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(Uuid::parse_str(link_id)?)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "REVOKED");
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 0);
    assert_eq!(row.3, 0);
    eprintln!(
        "exit-link-ok tenant={owner} foreign={foreign} actor={owner_actor} link={link_id} status={}",
        row.0
    );
    Ok(())
}

#[tokio::test]
async fn exit_file_download_matches_manifest_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists export_jobs (
          job_id uuid primary key,
          tenant_id uuid not null,
          job_type text not null,
          params_json jsonb not null default '{}'::jsonb,
          status text not null default 'PENDING',
          output_url text,
          progress int not null default 0,
          attempts int not null default 0,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )",
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists exit_requests (
          tenant_id uuid not null,
          request_row_id uuid not null,
          applicant_id text not null,
          confirmation_text text not null,
          cutoff_text text not null,
          file_range text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_row_id)
        )",
        "create table if not exists exit_manifests (
          tenant_id uuid not null,
          manifest_id uuid not null,
          item_kind text not null,
          item_no text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, manifest_id)
        )",
        "alter table exit_manifests add column if not exists line_count bigint",
        "alter table exit_manifests add column if not exists digest text",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let job = Uuid::new_v4();
    let body = format!("order,qty\nSO-A09K,1\n");
    let lines = body.bytes().filter(|byte| *byte == b'\n').count() as i64;
    let digest = format!("{:x}", sha2::Sha256::digest(body.as_bytes()));
    eprintln!("exit-file tenant={owner} foreign={foreign} job={job} lines={lines}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a09k-owner"),
        (foreign, foreign_actor, "a09k-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'exports.read'), ($1, $2, 'exports.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into export_jobs(job_id, tenant_id, job_type, params_json, status, output_url, progress) values ($1, $2, 'ORDER_EXPORT', '{}'::jsonb, 'DONE', $3, 100)",
    )
    .bind(job)
    .bind(owner)
    .bind(format!("FILE-A09K-{job}"))
    .execute(&pool)
    .await?;
    let export_dir = std::env::temp_dir().join(format!("erp-a09k-{owner}"));
    std::fs::create_dir_all(export_dir.join(owner.to_string()))?;
    std::fs::write(
        export_dir
            .join(owner.to_string())
            .join(format!("{job}.csv")),
        body.as_bytes(),
    )?;
    let previous_export_dir = std::env::var("EXPORT_BASE_DIR").ok();
    // Safety: this process is the acceptance test process. The shipped download
    // path reads EXPORT_BASE_DIR once per request, and the test restores it.
    unsafe {
        std::env::set_var("EXPORT_BASE_DIR", &export_dir);
    }
    let jwt = Arc::new(JwtService::new("exit-file-match-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a09k-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a09k",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a09k-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a09k",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let foreign_token = foreign_login.1["access_token"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let open = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/exit-requests"),
        Some(
            serde_json::json!({
                "request_id": "a09k-open",
                "applicant": "user-a09k",
                "confirmation": "confirm exit",
                "cutoff": "2026-10-04T00:00:00+08:00",
                "file_range": "MANIFEST",
                "wipe": false
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(open.0, StatusCode::OK, "{}", open.1);
    let early = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_eq!(early.0, StatusCode::OK, "{}", early.1);
    assert!(early.1.contains("SO-A09K"));
    let listed = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/exit-manifests"),
        Some(
            serde_json::json!({
                "request_id": "a09k-list",
                "item_kind": "EXPORT_FILE",
                "item_no": job.to_string(),
                "line_count": lines,
                "digest": digest
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(listed.0, StatusCode::OK, "{}", listed.1);
    let undelivered = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_eq!(undelivered.0, StatusCode::CONFLICT, "{}", undelivered.1);
    assert!(!undelivered.1.contains("SO-A09K"));
    let deliver = export_http(
        &app,
        &owner_token,
        "POST",
        &format!("/v1/tenants/{owner}/exit-requests/deliver"),
        Some(serde_json::json!({"request_id": "a09k-deliver", "wipe": false}).to_string()),
    )
    .await?;
    assert_eq!(deliver.0, StatusCode::OK, "{}", deliver.1);
    std::fs::write(
        export_dir
            .join(owner.to_string())
            .join(format!("{job}.csv")),
        "order,qty\nSTOLEN,9\n",
    )?;
    let mismatch = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_eq!(mismatch.0, StatusCode::CONFLICT, "{}", mismatch.1);
    assert!(!mismatch.1.contains("STOLEN"));
    assert!(!mismatch.1.contains("SO-A09K"));
    std::fs::write(
        export_dir
            .join(owner.to_string())
            .join(format!("{job}.csv")),
        body.as_bytes(),
    )?;
    let matched = export_http(
        &app,
        &owner_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_eq!(matched.0, StatusCode::OK, "{}", matched.1);
    assert!(matched.1.contains("SO-A09K"));
    let foreign_file = export_http(
        &app,
        &foreign_token,
        "GET",
        &format!("/exports/jobs/{job}/file"),
        None,
    )
    .await?;
    assert_ne!(foreign_file.0, StatusCode::OK);
    assert!(!foreign_file.1.contains("SO-A09K"));
    if let Some(previous) = previous_export_dir {
        unsafe {
            std::env::set_var("EXPORT_BASE_DIR", previous);
        }
    } else {
        unsafe {
            std::env::remove_var("EXPORT_BASE_DIR");
        }
    }
    let _ = std::fs::remove_dir_all(&export_dir);
    let row: (String, i64, String, i64, i64, String) = sqlx::query_as(
        "select r.status, m.line_count, m.digest,
            (select count(*) from exit_manifests where tenant_id = $1 and item_kind = 'EXPORT_FILE'),
            (select count(*) from exit_manifests where tenant_id = $2),
            j.status
         from exit_requests r
         join exit_manifests m on m.tenant_id = r.tenant_id and m.item_no = $3
         join export_jobs j on j.tenant_id = r.tenant_id and j.job_id = $3::uuid
         where r.tenant_id = $1",
    )
    .bind(owner)
    .bind(foreign)
    .bind(job.to_string())
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "DELIVERED");
    assert_eq!(row.1, lines);
    assert_eq!(row.2, digest);
    assert_eq!(row.3, 1);
    assert_eq!(row.4, 0);
    assert_eq!(row.5, "DONE");
    eprintln!(
        "exit-file-ok tenant={owner} foreign={foreign} actor={owner_actor} job={job} lines={} status={}",
        row.1, row.0
    );
    Ok(())
}

#[tokio::test]
async fn sku_import_failure_cross_tenant_download_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    sqlx::query(
        "create table if not exists export_jobs (
          job_id uuid primary key,
          tenant_id uuid not null,
          job_type text not null,
          params_json jsonb not null default '{}'::jsonb,
          status text not null default 'PENDING',
          output_url text,
          progress int not null default 0,
          attempts int not null default 0,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )",
    )
    .execute(&pool)
    .await?;
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let job = Uuid::new_v4();
    let marker = format!("FAIL-A04M-{job}");
    eprintln!("import-failure-isolation tenant={owner} foreign={foreign} job={job}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a04m-owner"),
        (foreign, foreign_actor, "a04m-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'skus.write'), ($1, $2, 'skus.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into export_jobs(job_id, tenant_id, job_type, params_json, status, progress)
         values ($1, $2, 'SKU_IMPORT', jsonb_build_object('failed_rows', 1, 'file_name', 'a04m.csv', 'marker', $3), 'FAILED', 100)",
    )
    .bind(job)
    .bind(owner)
    .bind(&marker)
    .execute(&pool)
    .await?;
    let failure_dir = std::env::temp_dir().join(format!("erp-a04m-{owner}"));
    std::fs::create_dir_all(failure_dir.join(owner.to_string()))?;
    std::fs::create_dir_all(failure_dir.join(foreign.to_string()))?;
    let failure_body = format!("1,SKU-A04M,name,{marker}\n");
    std::fs::write(
        failure_dir
            .join(owner.to_string())
            .join(format!("{job}.csv")),
        failure_body.as_bytes(),
    )?;
    std::fs::write(
        failure_dir
            .join(foreign.to_string())
            .join(format!("{job}.csv")),
        format!("1,SKU-STOLEN,name,STOLEN-{job}\n").as_bytes(),
    )?;
    let previous_failure_dir = std::env::var("SKU_IMPORT_FAILURE_BASE_DIR").ok();
    // Safety: this process is the acceptance test process. The shipped download
    // reads SKU_IMPORT_FAILURE_BASE_DIR once per request, and the test restores it.
    unsafe {
        std::env::set_var("SKU_IMPORT_FAILURE_BASE_DIR", &failure_dir);
    }
    let jwt = Arc::new(JwtService::new("import-failure-isolation-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a04m-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a04m",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a04m-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a04m",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"].as_str().unwrap_or("");
    let foreign_token = foreign_login.1["access_token"].as_str().unwrap_or("");
    let own_path = format!("/v1/tenants/{owner}/skus/import/jobs/{job}/failures/file");
    let foreign_path = format!("/v1/tenants/{foreign}/skus/import/jobs/{job}/failures/file");
    let owner_path_as_foreign = format!("/v1/tenants/{owner}/skus/import/jobs/{job}/failures/file");
    let own = export_http(&app, owner_token, "GET", &own_path, None).await?;
    assert_eq!(own.0, StatusCode::OK, "{}", own.1);
    assert!(own.1.contains(&marker));
    let crossed = export_http(&app, foreign_token, "GET", &owner_path_as_foreign, None).await?;
    assert_ne!(crossed.0, StatusCode::OK, "{}", crossed.1);
    assert!(!crossed.1.contains(&marker));
    let stolen = export_http(&app, foreign_token, "GET", &foreign_path, None).await?;
    assert_ne!(stolen.0, StatusCode::OK, "{}", stolen.1);
    assert!(!stolen.1.contains(&marker));
    assert!(!stolen.1.contains("STOLEN"));
    if let Some(previous) = previous_failure_dir {
        unsafe {
            std::env::set_var("SKU_IMPORT_FAILURE_BASE_DIR", previous);
        }
    } else {
        unsafe {
            std::env::remove_var("SKU_IMPORT_FAILURE_BASE_DIR");
        }
    }
    let _ = std::fs::remove_dir_all(&failure_dir);
    let row: (String, String, i64) = sqlx::query_as(
        "select status, coalesce(params_json->>'marker', ''),
            (select count(*) from export_jobs where tenant_id = $2 and job_id = $3)
         from export_jobs where tenant_id = $1 and job_id = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(job)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "FAILED");
    assert_eq!(row.1, marker);
    assert_eq!(row.2, 0);
    eprintln!(
        "import-failure-isolation-ok tenant={owner} foreign={foreign} actor={owner_actor} job={job} status={} foreign_jobs={}",
        row.0, row.2
    );
    Ok(())
}

#[tokio::test]
async fn after_sales_import_cross_tenant_read_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    sqlx::query(
        "create table if not exists after_sales_import_jobs (
          job_id uuid primary key,
          tenant_id uuid not null,
          file_name text,
          file_path text not null,
          file_type text not null default 'CSV',
          status text not null default 'PENDING',
          progress int not null default 0,
          total_rows int not null default 0,
          success_rows int not null default 0,
          failed_rows int not null default 0,
          attempts int not null default 0,
          last_error text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )",
    )
    .execute(&pool)
    .await?;
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let job = Uuid::new_v4();
    let marker = format!("AS-A04N-{job}");
    let stored_path = format!("imports/after_sales/{owner}/{marker}.csv");
    eprintln!("after-sales-import-isolation tenant={owner} foreign={foreign} job={job}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a04n-owner"),
        (foreign, foreign_actor, "a04n-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'aftersales.read'), ($1, $2, 'aftersales.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    sqlx::query(
        "insert into after_sales_import_jobs(job_id, tenant_id, file_name, file_path, file_type, status, progress)
         values ($1, $2, $3, $4, 'CSV', 'PENDING', 0)",
    )
    .bind(job)
    .bind(owner)
    .bind(&marker)
    .bind(&stored_path)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("after-sales-import-isolation-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a04n-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a04n",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a04n-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a04n",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"].as_str().unwrap_or("");
    let foreign_token = foreign_login.1["access_token"].as_str().unwrap_or("");
    let own = export_http(
        &app,
        owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/after-sales/import-jobs/{job}"),
        None,
    )
    .await?;
    assert_eq!(own.0, StatusCode::OK, "{}", own.1);
    assert!(own.1.contains(&marker));
    assert!(
        !own.1.contains(&stored_path),
        "owner response leaked file_path: {}",
        own.1
    );
    let crossed = export_http(
        &app,
        foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/after-sales/import-jobs/{job}"),
        None,
    )
    .await?;
    assert_ne!(crossed.0, StatusCode::OK, "{}", crossed.1);
    assert!(!crossed.1.contains(&marker));
    assert!(!crossed.1.contains(&stored_path));
    let foreign_same_id = export_http(
        &app,
        foreign_token,
        "GET",
        &format!("/v1/tenants/{foreign}/after-sales/import-jobs/{job}"),
        None,
    )
    .await?;
    assert_ne!(foreign_same_id.0, StatusCode::OK, "{}", foreign_same_id.1);
    assert!(!foreign_same_id.1.contains(&marker));
    assert!(!foreign_same_id.1.contains(&stored_path));
    let foreign_list = export_http(
        &app,
        foreign_token,
        "GET",
        &format!("/v1/tenants/{foreign}/after-sales/import-jobs"),
        None,
    )
    .await?;
    assert_eq!(foreign_list.0, StatusCode::OK, "{}", foreign_list.1);
    assert!(!foreign_list.1.contains(&marker));
    assert!(!foreign_list.1.contains(&stored_path));
    let row: (String, String, i64) = sqlx::query_as(
        "select status, file_path,
            (select count(*) from after_sales_import_jobs where tenant_id = $2)
         from after_sales_import_jobs where tenant_id = $1 and job_id = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(job)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "PENDING");
    assert_eq!(row.1, stored_path);
    assert_eq!(row.2, 0);
    eprintln!(
        "after-sales-import-isolation-ok tenant={owner} foreign={foreign} actor={owner_actor} job={job} status={} foreign_jobs={}",
        row.0, row.2
    );
    Ok(())
}

#[tokio::test]
async fn demo_document_skips_real_ledger_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists demo_scopes (
          tenant_id uuid not null,
          scope_id uuid not null,
          scope_code text not null,
          status text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, scope_id)
        )",
        "create table if not exists scoped_documents (
          tenant_id uuid not null,
          document_id uuid not null,
          scope_code text not null,
          document_no text not null,
          amount_cent bigint not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, document_id)
        )",
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let sku = Uuid::new_v4();
    eprintln!("demo-ledger tenant={tenant} actor={actor} warehouse={warehouse} sku={sku}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a06l-demo', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(format!("a06l-{actor}@example.test"))
    .bind(hash)
    .execute(&pool)
    .await?;
    let role = Uuid::new_v4();
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a06l')")
        .bind(role)
        .bind(tenant)
        .execute(&pool)
        .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(tenant)
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await?;
    sqlx::query("insert into warehouses(tenant_id, warehouse_id, name, code) values ($1, $2, 'WH-A06L', $3)")
        .bind(tenant)
        .bind(warehouse)
        .bind(format!("WH-{warehouse}"))
        .execute(&pool)
        .await?;
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id, sku_id, sku_code,product_id) values($1, $2, $3,(select product_id from qa_parent))")
        .bind(tenant)
        .bind(sku)
        .bind(format!("SKU-{sku}"))
        .execute(&pool)
        .await?;
    let jwt = Arc::new(JwtService::new("demo-ledger-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let login = login_http(
        &app,
        tenant,
        &format!("a06l-{actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a06l",
    )
    .await?;
    assert_eq!(login.0, StatusCode::OK, "{}", login.1);
    let token = login.1["access_token"].as_str().unwrap_or("");
    let opened = export_http(
        &app,
        token,
        "POST",
        &format!("/v1/tenants/{tenant}/demo-scopes"),
        Some(r#"{"request_id":"a06l-open"}"#.to_string()),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let demo = export_http(
        &app,
        token,
        "POST",
        &format!("/v1/tenants/{tenant}/scoped-documents"),
        Some(
            serde_json::json!({
                "request_id": "a06l-demo",
                "scope": "DEMO",
                "document_no": "DEMO-SO-A06L",
                "amount_cent": 1300
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(demo.0, StatusCode::OK, "{}", demo.1);
    let demo_body: Value = serde_json::from_str(&demo.1).unwrap_or(Value::Null);
    assert_eq!(demo_body["replay"].as_bool().unwrap_or(true), false);
    let real = export_http(
        &app,
        token,
        "POST",
        &format!("/v1/tenants/{tenant}/scoped-documents"),
        Some(
            serde_json::json!({
                "request_id": "a06l-real",
                "scope": "REAL",
                "document_no": "SO-A06L",
                "amount_cent": 2500,
                "warehouse_id": warehouse,
                "sku_id": sku
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(real.0, StatusCode::OK, "{}", real.1);
    let left = export_http(
        &app,
        token,
        "POST",
        &format!("/v1/tenants/{tenant}/demo-scopes/leave"),
        Some(r#"{"request_id":"a06l-leave"}"#.to_string()),
    )
    .await?;
    assert_eq!(left.0, StatusCode::OK, "{}", left.1);
    let row: (String, i64, i64, i64, i64) = sqlx::query_as(
        "select status,
            (select count(*) from scoped_documents where tenant_id = $1 and scope_code = 'DEMO'),
            (select count(*) from scoped_documents where tenant_id = $1 and scope_code = 'REAL'),
            (select count(*) from inventory_ledgers where tenant_id = $1 and ref_id = 'DEMO-SO-A06L'),
            (select count(*) from inventory_ledgers where tenant_id = $1 and ref_id = 'SO-A06L' and reason_type = 'SCOPED_DOCUMENT')
         from demo_scopes where tenant_id = $1 and scope_code = 'DEMO'",
    )
    .bind(tenant)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "LEFT");
    assert_eq!(row.1, 1);
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 0);
    assert_eq!(row.4, 1);
    eprintln!(
        "demo-ledger-ok tenant={tenant} actor={actor} demo_status={} demo_docs={} real_docs={} demo_ledgers={} real_ledgers={}",
        row.0, row.1, row.2, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn formal_login_failure_backoff_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    for statement in [
        "create table if not exists login_attempts (
          tenant_id uuid not null,
          attempt_id uuid not null,
          account_id text not null,
          source_text text not null,
          device_id text not null,
          failed_count integer not null,
          backoff_seconds bigint not null,
          secret_ref text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, attempt_id)
        )",
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let tenant = Uuid::new_v4();
    let other = Uuid::new_v4();
    let actor = Uuid::new_v4();
    let other_actor = Uuid::new_v4();
    eprintln!("login-backoff tenant={tenant} other={other} actor={actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (id, name, user) in [
        (tenant, "a07l-owner", actor),
        (other, "a07l-other", other_actor),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(id)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(user)
        .bind(id)
        .bind(format!("{name}-{user}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
    }
    let jwt = Arc::new(JwtService::new("login-backoff-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let email = format!("a07l-owner-{actor}@example.test");
    let mut waits = Vec::new();
    for expected in [1_i64, 2, 4] {
        let failed = login_http(&app, tenant, &email, "wrong-password", "WEB", "tab-a07l").await?;
        assert_eq!(failed.0, StatusCode::UNAUTHORIZED, "{}", failed.1);
        let body = failed.1.to_string();
        assert!(!body.contains("wrong-password"));
        assert!(!body.contains("boundary-a03k"));
        assert!(!body.contains("login-ref"));
        assert_eq!(
            failed.1["retry_after_seconds"].as_i64().unwrap_or(0),
            expected
        );
        waits.push(expected);
        let blocked = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a07l").await?;
        assert_eq!(blocked.0, StatusCode::UNAUTHORIZED, "{}", blocked.1);
        assert_eq!(blocked.1["code"].as_str().unwrap_or(""), "login_backoff");
        assert!(blocked.1["retry_after_seconds"].as_i64().unwrap_or(0) >= 1);
        assert!(!blocked.1.to_string().contains("boundary-a03k"));
        tokio::time::sleep(std::time::Duration::from_millis(
            (expected as u64) * 1000 + 50,
        ))
        .await;
    }
    assert_eq!(waits, vec![1, 2, 4]);
    let other_email = format!("a07l-other-{other_actor}@example.test");
    let other_login = login_http(
        &app,
        other,
        &other_email,
        "boundary-a03k",
        "WEB",
        "tab-a07l-other",
    )
    .await?;
    assert_eq!(other_login.0, StatusCode::OK, "{}", other_login.1);
    let row: (i32, i64, String, i64, i64) = sqlx::query_as(
        "select failed_count, backoff_seconds, secret_ref,
            (select count(*) from login_attempts where tenant_id = $1),
            (select count(*) from login_attempts where tenant_id = $2)
         from login_attempts
         where tenant_id = $1 and account_id = $3 and failed_count = 3",
    )
    .bind(tenant)
    .bind(other)
    .bind(&email)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 3);
    assert_eq!(row.1, 4);
    assert!(row.2.starts_with("login-ref:"));
    assert!(!row.2.contains("wrong-password"));
    assert_eq!(row.3, 3);
    assert_eq!(row.4, 0);
    eprintln!(
        "login-backoff-ok tenant={tenant} other={other} actor={actor} failed_count={} wait={} attempts={} other_attempts={}",
        row.0, row.1, row.3, row.4
    );
    Ok(())
}

#[tokio::test]
async fn stats_health_hides_endpoint_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let (database, port): (String, i32) =
        sqlx::query_as("select current_database(), inet_server_port()")
            .fetch_one(&pool)
            .await?;
    assert!(matches!(database.as_str(), "erp_shared_contract_test" | "erp_free_ui_acceptance"));
    assert_eq!(port, 15438);
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let marker = format!("http://clickhouse.a08j.invalid/{owner}");
    let issuer = format!("http://sso.a08j.invalid/{owner}");
    eprintln!("stats-health tenant={owner} foreign={foreign} actor={owner_actor}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a08j-owner"),
        (foreign, foreign_actor, "a08j-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'stats.read')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
    }
    let previous_clickhouse = std::env::var("ERP_CLICKHOUSE_URL").ok();
    let previous_issuer = std::env::var("ERP_KEYCLOAK_ISSUER").ok();
    // Safety: this process is the acceptance test process. The health handler
    // reads these names once per request, and the test restores them.
    unsafe {
        std::env::set_var("ERP_CLICKHOUSE_URL", &marker);
        std::env::set_var("ERP_KEYCLOAK_ISSUER", &issuer);
    }
    let jwt = Arc::new(JwtService::new("stats-health-isolation-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a08j-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a08j",
    )
    .await?;
    let foreign_login = login_http(
        &app,
        foreign,
        &format!("a08j-foreign-{foreign_actor}@example.test"),
        "boundary-a03k",
        "DESKTOP",
        "desk-a08j",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    assert_eq!(foreign_login.0, StatusCode::OK, "{}", foreign_login.1);
    let owner_token = owner_login.1["access_token"].as_str().unwrap_or("");
    let foreign_token = foreign_login.1["access_token"].as_str().unwrap_or("");
    let own = export_http(
        &app,
        owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/stats/health"),
        None,
    )
    .await?;
    assert_eq!(own.0, StatusCode::OK, "{}", own.1);
    assert!(own.1.contains("CLICKHOUSE") || own.1.contains("POSTGRES"));
    assert!(!own.1.contains(&marker), "{}", own.1);
    assert!(!own.1.contains(&issuer), "{}", own.1);
    assert!(!own.1.contains("clickhouse_endpoint"));
    assert!(!own.1.contains("sso_issuer"));
    let crossed = export_http(
        &app,
        foreign_token,
        "GET",
        &format!("/v1/tenants/{owner}/stats/health"),
        None,
    )
    .await?;
    assert_ne!(crossed.0, StatusCode::OK, "{}", crossed.1);
    assert!(!crossed.1.contains(&marker));
    assert!(!crossed.1.contains(&issuer));
    let probe = export_http(&app, "", "GET", "/healthz", None).await?;
    assert_eq!(probe.0, StatusCode::OK, "{}", probe.1);
    assert!(!probe.1.contains(&marker));
    assert!(!probe.1.contains(&issuer));
    assert!(!probe.1.contains(owner.to_string().as_str()));
    if let Some(previous) = previous_clickhouse {
        unsafe {
            std::env::set_var("ERP_CLICKHOUSE_URL", previous);
        }
    } else {
        unsafe {
            std::env::remove_var("ERP_CLICKHOUSE_URL");
        }
    }
    if let Some(previous) = previous_issuer {
        unsafe {
            std::env::set_var("ERP_KEYCLOAK_ISSUER", previous);
        }
    } else {
        unsafe {
            std::env::remove_var("ERP_KEYCLOAK_ISSUER");
        }
    }
    eprintln!(
        "stats-health-ok tenant={owner} foreign={foreign} actor={owner_actor} probe={}",
        probe.1
    );
    Ok(())
}

#[tokio::test]
async fn print_output_cross_tenant_complete_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let owner_actor = Uuid::new_v4();
    let foreign_actor = Uuid::new_v4();
    let job = Uuid::new_v4();
    let template = Uuid::new_v4();
    let shop = Uuid::new_v4();
    let marker = format!("PRINT-A04J-{job}");
    eprintln!("print-isolation tenant={owner} foreign={foreign} job={job}");
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    for (tenant, actor, name) in [
        (owner, owner_actor, "a04j-owner"),
        (foreign, foreign_actor, "a04j-foreign"),
    ] {
        sqlx::query(
            "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, $2, 'FREE', 1, 8, 1)",
        )
        .bind(tenant)
        .bind(name)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
        )
        .bind(actor)
        .bind(tenant)
        .bind(format!("{name}-{actor}@example.test"))
        .bind(hash)
        .execute(&pool)
        .await?;
        let role = Uuid::new_v4();
        sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, $3)")
            .bind(role)
            .bind(tenant)
            .bind(name)
            .execute(&pool)
            .await?;
        sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
            .bind(tenant)
            .bind(actor)
            .bind(role)
            .execute(&pool)
            .await?;
        sqlx::query(
            "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'print_jobs.read'), ($1, $2, 'print_jobs.write')",
        )
        .bind(tenant)
        .bind(role)
        .execute(&pool)
        .await?;
        sqlx::query(
            "insert into print_devices(tenant_id, name, fingerprint, device_fingerprint, status, device_token) values ($1, $2, $3, $3, 'ACTIVE', $4)",
        )
        .bind(tenant)
        .bind(name)
        .bind(format!("fp-{name}-{tenant}"))
        .bind(format!("token-{name}-{tenant}"))
        .execute(&pool)
        .await?;
    }
    sqlx::query("insert into shops(tenant_id,shop_id,platform,platform_shop_id) values($1,$2,'qa',$3)")
        .bind(owner).bind(shop).bind(shop.to_string()).execute(&pool).await?;
    sqlx::query("insert into print_templates(tenant_id,template_id,shop_id,platform,template_type) values($1,$2,$3,'qa','WAYBILL')")
        .bind(owner).bind(template).bind(shop).execute(&pool).await?;
    sqlx::query(
        "insert into print_jobs(tenant_id, job_id, type, ref_type, ref_id, status, device_fingerprint, output_content, requested_by_user_id,template_id) values ($1, $2, 'WAYBILL', 'ORDER', $2, 'DISPATCHED', $3, $4, $5,$6)",
    )
    .bind(owner)
    .bind(job)
    .bind(format!("fp-a04j-owner-{owner}"))
    .bind(&marker)
    .bind(owner_actor)
    .bind(template)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("print-isolation-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let owner_login = login_http(
        &app,
        owner,
        &format!("a04j-owner-{owner_actor}@example.test"),
        "boundary-a03k",
        "WEB",
        "tab-a04j",
    )
    .await?;
    assert_eq!(owner_login.0, StatusCode::OK, "{}", owner_login.1);
    let owner_token = owner_login.1["access_token"].as_str().unwrap_or("");
    let own = export_http(
        &app,
        owner_token,
        "GET",
        &format!("/v1/tenants/{owner}/print-jobs/{job}/output"),
        None,
    )
    .await?;
    assert_eq!(own.0, StatusCode::CONFLICT, "{}", own.1);
    assert!(!own.1.contains(&marker));
    let stolen = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/print-agent/jobs/{job}/complete"))
                .header("x-device-token", format!("token-a04j-foreign-{foreign}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "status": "DONE",
                        "output_content": "STOLEN-A04J"
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let stolen_status = stolen.status();
    let stolen_body =
        String::from_utf8_lossy(&stolen.into_body().collect().await?.to_bytes()).into_owned();
    assert_ne!(stolen_status, StatusCode::OK, "{stolen_body}");
    assert!(!stolen_body.contains(&marker));
    assert!(!stolen_body.contains("STOLEN-A04J"));
    let row: (String, String, i64) = sqlx::query_as(
        "select status, coalesce(output_content, ''),
            (select count(*) from print_jobs where tenant_id = $2 and job_id = $3)
         from print_jobs where tenant_id = $1 and job_id = $3",
    )
    .bind(owner)
    .bind(foreign)
    .bind(job)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "DISPATCHED");
    assert_eq!(row.1, marker);
    assert_eq!(row.2, 0);
    eprintln!(
        "print-isolation-ok tenant={owner} foreign={foreign} actor={owner_actor} job={job} status={} foreign_jobs={}",
        row.0, row.2
    );
    Ok(())
}

#[tokio::test]
async fn company_setting_keeps_work_order_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for statement in [
        "create table if not exists settings (
          setting_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          scope_type text not null,
          scope_id uuid,
          key text not null,
          value_json jsonb not null default '{}'::jsonb,
          version int not null default 1,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )",
        "create unique index if not exists idx_settings_scope_key on settings(tenant_id, scope_type, scope_id, key)",
        "create table if not exists audit_logs (
          audit_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          user_id uuid,
          action text not null,
          entity_type text not null,
          entity_id text,
          before_json jsonb,
          after_json jsonb,
          ip text,
          ua text,
          created_at timestamptz not null default now()
        )",
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
        "create table if not exists mfg_work_orders (
          tenant_id uuid not null,
          work_order_id uuid not null default gen_random_uuid(),
          work_order_no text not null,
          product_code text not null,
          bom_id uuid,
          status text not null,
          order_qty_micros bigint not null,
          issued_micros bigint not null default 0,
          consumed_micros bigint not null default 0,
          created_at timestamptz not null default now(),
          primary key (tenant_id, work_order_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let tenant = Uuid::new_v4();
    let actor = Uuid::new_v4();
    eprintln!("setting-order tenant={tenant} actor={actor}");
    sqlx::query(
        "insert into tenants(tenant_id, name, plan_code, login_seat_limit, max_active_sessions, credentials_epoch) values ($1, 'a05h-setting', 'FREE', 1, 8, 1)",
    )
    .bind(tenant)
    .execute(&pool)
    .await?;
    let hash = "$argon2id$v=19$m=8,t=1,p=1$bm90YXNhbHRub3Rhc2FsdA$vOIz62RLeK2ElUlm3o2w824tghv0T/PsnFq6jG8BFCA";
    sqlx::query(
        "insert into users(user_id, tenant_id, email, password_hash, status) values ($1, $2, $3, $4, 'ACTIVE')",
    )
    .bind(actor)
    .bind(tenant)
    .bind(format!("a05h-{actor}@example.test"))
    .bind(hash)
    .execute(&pool)
    .await?;
    let role = Uuid::new_v4();
    sqlx::query("insert into roles(role_id, tenant_id, name) values ($1, $2, 'a05h')")
        .bind(role)
        .bind(tenant)
        .execute(&pool)
        .await?;
    sqlx::query("insert into user_roles(tenant_id, user_id, role_id) values ($1, $2, $3)")
        .bind(tenant)
        .bind(actor)
        .bind(role)
        .execute(&pool)
        .await?;
    sqlx::query(
        "insert into role_permissions(tenant_id, role_id, perm_code) values ($1, $2, 'SYS.SETTINGS.WRITE'), ($1, $2, 'SYS.SETTINGS.READ'), ($1, $2, 'MFG.PRODUCTION.WRITE')",
    )
    .bind(tenant)
    .bind(role)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("company-setting-work-order-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let email = format!("a05h-{actor}@example.test");
    let web = login_http(&app, tenant, &email, "boundary-a03k", "WEB", "tab-a05h").await?;
    let desk = login_http(
        &app,
        tenant,
        &email,
        "boundary-a03k",
        "DESKTOP",
        "desk-a05h",
    )
    .await?;
    assert_eq!(web.0, StatusCode::OK, "{}", web.1);
    assert_eq!(desk.0, StatusCode::OK, "{}", desk.1);
    let web_token = web.1["access_token"].as_str().unwrap_or("").to_string();
    let desk_token = desk.1["access_token"].as_str().unwrap_or("").to_string();
    let opened = export_http(
        &app,
        &web_token,
        "POST",
        &format!("/v1/tenants/{tenant}/manufacturing/work-orders"),
        Some(
            serde_json::json!({
                "request_id": "a05h-order",
                "work_order_no": "WO-A05H",
                "product_code": "P-A05H",
                "order_qty": "1"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(opened.0, StatusCode::OK, "{}", opened.1);
    let first = export_http(
        &app,
        &web_token,
        "PUT",
        &format!("/v1/tenants/{tenant}/settings"),
        Some(
            serde_json::json!({
                "scope_type": "TENANT",
                "scope_id": tenant,
                "key": "company.mode",
                "value": {"make_enabled": true, "currency": "CNY"}
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let first_body: Value = serde_json::from_str(&first.1).unwrap_or(Value::Null);
    assert_eq!(first_body["version"].as_i64().unwrap_or(0), 1);
    let second = export_http(
        &app,
        &desk_token,
        "PUT",
        &format!("/v1/tenants/{tenant}/settings"),
        Some(
            serde_json::json!({
                "scope_type": "TENANT",
                "scope_id": tenant,
                "key": "company.mode",
                "value": {"make_enabled": false, "currency": "USD"},
                "expected_version": 1
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(second.0, StatusCode::OK, "{}", second.1);
    let stale = export_http(
        &app,
        &web_token,
        "PUT",
        &format!("/v1/tenants/{tenant}/settings"),
        Some(
            serde_json::json!({
                "scope_type": "TENANT",
                "scope_id": tenant,
                "key": "company.mode",
                "value": {"make_enabled": true, "currency": "EUR"},
                "expected_version": 1
            })
            .to_string(),
        ),
    )
    .await?;
    assert_eq!(stale.0, StatusCode::CONFLICT, "{}", stale.1);
    let listed = export_http(
        &app,
        &desk_token,
        "GET",
        &format!("/v1/tenants/{tenant}/manufacturing/work-orders?work_order_id=00000000-0000-0000-0000-000000000000"),
        None,
    )
    .await?;
    assert_ne!(listed.0, StatusCode::NOT_FOUND, "{}", listed.1);
    let row: (i32, String, String, i64, i64) = sqlx::query_as(
        "select s.version, s.value_json->>'currency', w.status, w.order_qty_micros,
            (select count(*) from mfg_work_orders where tenant_id = $1)
         from settings s
         join mfg_work_orders w on w.tenant_id = s.tenant_id
         where s.tenant_id = $1 and s.key = 'company.mode' and w.work_order_no = 'WO-A05H'",
    )
    .bind(tenant)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, 2);
    assert_eq!(row.1, "USD");
    assert_eq!(row.2, "PLANNED");
    assert_eq!(row.3, 1_000_000);
    assert_eq!(row.4, 1);
    eprintln!(
        "setting-order-ok tenant={tenant} actor={actor} version={} currency={} order_status={} orders={}",
        row.0, row.1, row.2, row.4
    );
    Ok(())
}

#[tokio::test]
async fn onboarding_same_command_retry_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for statement in [
        "create table if not exists onboarding_jobs (
          tenant_id uuid not null,
          job_id uuid not null,
          identity_text text not null,
          verification_token text not null,
          company_name text not null,
          mode_code text not null,
          timezone_name text not null,
          currency text not null,
          status text not null,
          expires_at timestamptz not null,
          opened_at timestamptz not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, job_id),
          unique (tenant_id, verification_token)
        )",
        "create table if not exists command_receipts (
          tenant_id uuid not null,
          request_id text not null,
          payload_hash text not null,
          status text not null,
          effect_id text not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, request_id)
        )",
    ] {
        sqlx::query(statement).execute(&pool).await?;
    }
    let tenant = Uuid::new_v4();
    eprintln!("onboarding-retry tenant={tenant}");
    sqlx::query("insert into tenants(tenant_id, name) values ($1, 'a01k-space')")
        .bind(tenant)
        .execute(&pool)
        .await?;
    let jwt = Arc::new(JwtService::new("onboarding-same-command-test".into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    let body = serde_json::json!({
        "tenant_id": tenant,
        "request_id": "a01k-open",
        "identity": "user-a01k",
        "verification": "verify-a01k",
        "company_name": "Mill A01K",
        "mode": "TRADE",
        "timezone": "Asia/Shanghai",
        "currency": "CNY",
        "expires_at": "2026-10-05T00:00:00Z",
        "opened_at": "2026-10-04T00:00:00Z"
    })
    .to_string();
    let first = export_http(&app, "", "POST", "/v1/onboarding/open", Some(body.clone())).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let first_body: Value = serde_json::from_str(&first.1).unwrap_or(Value::Null);
    assert_eq!(first_body["replay"].as_bool().unwrap_or(true), false);
    let effect = first_body["effect_id"].as_str().unwrap_or("").to_string();
    let replay = export_http(&app, "", "POST", "/v1/onboarding/open", Some(body)).await?;
    assert_eq!(replay.0, StatusCode::OK, "{}", replay.1);
    let replay_body: Value = serde_json::from_str(&replay.1).unwrap_or(Value::Null);
    assert_eq!(replay_body["replay"].as_bool().unwrap_or(false), true);
    assert_eq!(replay_body["effect_id"].as_str().unwrap_or(""), effect);
    let other = export_http(
        &app,
        "",
        "POST",
        "/v1/onboarding/open",
        Some(
            serde_json::json!({
                "tenant_id": tenant,
                "request_id": "a01k-other",
                "identity": "user-a01k",
                "verification": "verify-a01k",
                "company_name": "Mill A01K Other",
                "mode": "TRADE",
                "timezone": "Asia/Shanghai",
                "currency": "CNY",
                "expires_at": "2026-10-05T00:00:00Z",
                "opened_at": "2026-10-04T00:00:00Z"
            })
            .to_string(),
        ),
    )
    .await?;
    assert_ne!(other.0, StatusCode::OK, "{}", other.1);
    let progress = export_http(
        &app,
        "",
        "GET",
        &format!("/v1/onboarding/progress?tenant_id={tenant}&request_id=a01k-open"),
        None,
    )
    .await?;
    assert_eq!(progress.0, StatusCode::OK, "{}", progress.1);
    let progress_body: Value = serde_json::from_str(&progress.1).unwrap_or(Value::Null);
    assert_eq!(progress_body["status"].as_str().unwrap_or(""), "OPEN");
    let row: (String, String, i64, i64) = sqlx::query_as(
        "select status, company_name,
            (select count(*) from onboarding_jobs where tenant_id = $1),
            (select count(*) from command_receipts where tenant_id = $1 and request_id = 'a01k-open')
         from onboarding_jobs where tenant_id = $1 and verification_token = 'verify-a01k'",
    )
    .bind(tenant)
    .fetch_one(&independent)
    .await?;
    assert_eq!(row.0, "OPEN");
    assert_eq!(row.1, "Mill A01K");
    assert_eq!(row.2, 1);
    assert_eq!(row.3, 1);
    eprintln!(
        "onboarding-retry-ok tenant={tenant} status={} company={} jobs={} receipts={}",
        row.0, row.1, row.2, row.3
    );
    Ok(())
}

async fn export_http(
    app: &axum::Router,
    token: &str,
    method: &str,
    path: &str,
    body: Option<String>,
) -> Result<(StatusCode, String)> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if !token.is_empty() {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.unwrap_or_default()))?)
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok((status, String::from_utf8_lossy(&bytes).into_owned()))
}

async fn order_edit(
    app: &axum::Router,
    token: &str,
    tenant: Uuid,
    order: Uuid,
    version: i32,
    field: &str,
    value: &str,
) -> Result<(StatusCode, Value)> {
    order_edit_command(app, token, tenant, order, version, field, value, "").await
}

async fn order_edit_command(
    app: &axum::Router,
    token: &str,
    tenant: Uuid,
    order: Uuid,
    version: i32,
    field: &str,
    value: &str,
    command: &str,
) -> Result<(StatusCode, Value)> {
    let mut body = serde_json::json!({"expected_version": version});
    body[field] = Value::String(value.to_string());
    let mut request = Request::builder()
        .method("PUT")
        .uri(format!("/v1/tenants/{tenant}/orders/{order}"))
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json");
    if !command.is_empty() {
        request = request.header("Idempotency-Key", command);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::from(body.to_string()))?)
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let parsed = serde_json::from_slice(&bytes)
        .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into()));
    Ok((status, parsed))
}

async fn disable_http(
    app: &axum::Router,
    token: &str,
    tenant: Uuid,
    user: Uuid,
) -> Result<(StatusCode, Value)> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/v1/tenants/{tenant}/users/{user}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"status":"DISABLED"}"#.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into()));
    Ok((status, body))
}

async fn password_http(
    app: &axum::Router,
    token: &str,
    current: &str,
    next: &str,
) -> Result<(StatusCode, Value)> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/me/password")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "current_password": current,
                        "new_password": next
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into()));
    Ok((status, body))
}

async fn race_logins(
    app: &axum::Router,
    tenant: Uuid,
    email: &str,
    count: usize,
    prefix: &str,
) -> Result<Vec<(StatusCode, Value)>> {
    let mut tasks = JoinSet::new();
    for index in 0..count {
        let app = app.clone();
        let email = email.to_string();
        let label = format!("{prefix}-{index}");
        tasks.spawn(async move {
            login_http(&app, tenant, &email, "boundary-a03k", "WEB", &label).await
        });
    }
    let mut results = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        results.push(joined??);
    }
    Ok(results)
}

async fn me_http(app: &axum::Router, token: &str) -> Result<StatusCode> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/me")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())?,
        )
        .await?;
    Ok(response.status())
}

async fn revoke_http(app: &axum::Router, token: &str, session: &str) -> Result<StatusCode> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/me/sessions/{session}"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())?,
        )
        .await?;
    Ok(response.status())
}

async fn login_after_reported_backoff(
    app: &axum::Router,
    tenant: Uuid,
    email: &str,
    password: &str,
    kind: &str,
    label: &str,
) -> Result<(StatusCode, Value)> {
    let first = login_http(app, tenant, email, password, kind, label).await?;
    if first.1["code"] != "login_backoff" {
        return Ok(first);
    }
    assert_eq!(
        first.0,
        StatusCode::UNAUTHORIZED,
        "a reported authentication backoff must remain a denial"
    );
    let wait = first.1["retry_after_seconds"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("backoff must report a positive retry delay"))?;
    assert!(
        (1..=5).contains(&wait),
        "fresh single-error fixture must have a bounded reported delay"
    );
    tokio::time::sleep(std::time::Duration::from_secs(wait) + std::time::Duration::from_millis(20))
        .await;
    eprintln!("QA login_backoff_wait tenant={tenant} seconds={wait} respected=true");
    login_http(app, tenant, email, password, kind, label).await
}

async fn login_http(
    app: &axum::Router,
    tenant: Uuid,
    email: &str,
    password: &str,
    kind: &str,
    label: &str,
) -> Result<(StatusCode, Value)> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "tenant_id": tenant,
                        "email": email,
                        "password": password,
                        "client_kind": kind,
                        "client_label": label
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or(Value::String(String::from_utf8_lossy(&bytes).into()));
    Ok((status, body))
}
