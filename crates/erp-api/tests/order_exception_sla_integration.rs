use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Datelike, Duration, Utc};
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

mod support;

fn build_state(db: Db) -> (Router, JwtService) {
    std::env::set_var("IDEMPOTENCY_DISABLED", "true");
    let jwt = JwtService::new("test-secret".to_string());
    let state = AppState::new(db, Arc::new(jwt.clone()));
    let app = build_app(state);
    (app, jwt)
}

async fn connect_test_db(case_name: &str) -> Option<Db> {
    let database_url = support::test_database_url();
    match Db::connect(&database_url).await {
        Ok(db) => Some(db),
        Err(err) => {
            eprintln!("skipping {case_name}: {err}");
            None
        }
    }
}

fn build_token(jwt: &JwtService, tenant_id: &str, perms: Vec<&str>) -> String {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: uuid::Uuid::new_v4().to_string(),
        tenant_id: tenant_id.to_string(),
        perms: perms.into_iter().map(|value| value.to_string()).collect(),
        is_admin: false,
        iat: now,
        exp: now + 3600,
    };
    jwt.encode(&claims).expect("token should encode")
}

async fn setup_schema(pool: &PgPool) -> Result<()> {
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
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, platform, platform_shop_id)
        )
        "#,
        r#"
        create table if not exists order_index (
          tenant_id uuid not null,
          order_id uuid not null,
          order_month date not null,
          platform text not null,
          platform_order_id text not null,
          shop_id uuid not null,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists orders (
          tenant_id uuid not null,
          order_month date not null,
          order_id uuid not null,
          platform text not null,
          shop_id uuid not null,
          platform_order_id text not null,
          status text not null,
          oms_status text not null default 'REVIEW_PENDING',
          order_status text not null default 'PENDING_REVIEW',
          fulfillment_status text not null default 'NOT_SUBMITTED',
          exception_status text not null default 'MANUAL_HOLD',
          review_status text not null default 'MANUAL_REVIEW',
          exception_flags jsonb not null default '[]'::jsonb,
          logistics_company text,
          tracking_no text,
          ship_warehouse_id uuid,
          receiver_name text,
          receiver_address text,
          exception_reason text,
          total_amount_cent bigint not null default 0,
          inventory_shortage boolean not null default false,
          version int not null default 0,
          order_created_at timestamptz not null,
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_month, order_id)
        )
        "#,
        r#"
        create table if not exists audit_logs (
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
        )
        "#,
        r#"
        create table if not exists order_exception_tickets (
          ticket_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          reason_code text not null,
          status text not null default 'OPEN',
          handler_id uuid,
          handled_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, order_id, reason_code)
        )
        "#,
        r#"
        create table if not exists order_reviews (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          status text not null default 'PENDING',
          reviewed_by uuid,
          reviewed_at timestamptz,
          notes text not null default '',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists order_status_history (
          history_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          order_id uuid not null,
          from_status text,
          to_status text not null,
          action_code text,
          operator_id uuid,
          reason text,
          payload jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists order_action_logs (
          log_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          order_id uuid not null,
          action_code text not null,
          idempotency_key text not null,
          status text,
          operator_id uuid,
          request_json jsonb,
          response_json jsonb,
          error_message text,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists operation_logs (
          log_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          actor_type text not null,
          actor_user_id uuid,
          action text not null,
          ref_type text,
          ref_id text,
          metadata jsonb,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists settings (
          setting_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          scope_type text not null,
          scope_id uuid,
          key text not null,
          value_json jsonb not null default '{}'::jsonb,
          version int not null default 1,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create unique index if not exists idx_test_settings_unique
          on settings(
            tenant_id,
            scope_type,
            coalesce(scope_id, '00000000-0000-0000-0000-000000000000'::uuid),
            key
          )
        "#,
        r#"
        create table if not exists order_settlement_records (
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          status text not null default 'OPEN',
          locked_at timestamptz,
          export_job_id text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, order_id)
        )
        "#,
        r#"
        create table if not exists order_reconciliation_batches (
          tenant_id uuid not null references tenants(tenant_id),
          batch_id text not null,
          platform text not null,
          shop_id uuid not null,
          period_start date not null,
          period_end date not null,
          status text not null default 'PENDING',
          note text,
          settled_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, batch_id)
        )
        "#,
    ];

    for statement in statements {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}

#[tokio::test]
async fn oms_exception_pool_routes_validate_path_before_db_access() {
    let database_url = support::test_database_url();
    let db = Db::connect_lazy(&database_url).expect("lazy db should be constructible");
    let (app, jwt) = build_state(db);
    let token = build_token(
        &jwt,
        &uuid::Uuid::new_v4().to_string(),
        vec!["orders.read", "orders.write"],
    );

    let cases = [
        (
            "GET",
            "/v1/tenants/not-a-uuid/oms/orders/exceptions",
            StatusCode::BAD_REQUEST,
        ),
        (
            "GET",
            "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid",
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/claim",
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/assign",
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/escalate",
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/resolve",
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/hold",
            StatusCode::BAD_REQUEST,
        ),
        (
            "POST",
            "/v1/tenants/not-a-uuid/oms/orders/exceptions/not-a-uuid/reject",
            StatusCode::BAD_REQUEST,
        ),
    ];

    for (method, uri, expected_status) in cases {
        let request = Request::builder()
            .uri(uri)
            .method(method)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            expected_status,
            "route {method} {uri} should exist and validate path params before DB access",
        );
    }
}

#[tokio::test]
async fn oms_exception_pool_exposes_sla_and_action_routes() -> Result<()> {
    if let Err(error) = oms_exception_pool_exposes_sla_and_action_routes_inner().await {
        let message = error.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping oms_exception_pool_exposes_sla_and_action_routes: {message}");
            return Ok(());
        }
        return Err(error);
    }
    Ok(())
}

#[tokio::test]
async fn oms_settlement_and_reconciliation_workbench_routes() -> Result<()> {
    if let Err(error) = oms_settlement_and_reconciliation_workbench_routes_inner().await {
        let message = error.to_string();
        if message.contains("pool timed out") || message.contains("DATABASE_URL") {
            eprintln!("skipping oms_settlement_and_reconciliation_workbench_routes: {message}");
            return Ok(());
        }
        return Err(error);
    }
    Ok(())
}

async fn oms_settlement_and_reconciliation_workbench_routes_inner() -> Result<()> {
    let Some(db) = connect_test_db("oms_settlement_and_reconciliation_workbench_routes").await
    else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let order_id_a = uuid::Uuid::new_v4();
    let order_id_b = uuid::Uuid::new_v4();
    let reviewer_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let created_from = now - Duration::minutes(1);
    let created_to = now + Duration::minutes(1);
    let period_start = created_from.date_naive();
    let period_end = created_to.date_naive();
    let order_month = now.date_naive().with_day(1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("oms-settlement-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name)
        values ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-SETTLEMENT-1")
    .bind("结算店铺")
    .execute(db.pool())
    .await?;

    for (
        order_id,
        platform_order_id,
        receiver_name,
        amount_cent,
        exception_flags,
        exception_reason,
    ) in [
        (
            order_id_a,
            "PLAT-SETTLE-1",
            "张三",
            10000_i64,
            "[]",
            Option::<&str>::None,
        ),
        (
            order_id_b,
            "PLAT-SETTLE-2",
            "李四",
            8000_i64,
            r#"["MAPPING_MISSING"]"#,
            Some("货品映射缺失"),
        ),
    ] {
        sqlx::query(
            r#"
            insert into order_index (
                tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
            ) values ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .bind(order_month)
        .bind("taobao")
        .bind(platform_order_id)
        .bind(shop_id)
        .bind(now)
        .execute(db.pool())
        .await?;

        sqlx::query(
            r#"
            insert into orders (
                tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
                oms_status, order_status, fulfillment_status, exception_status, review_status,
                exception_flags, receiver_name, receiver_address, exception_reason, total_amount_cent,
                inventory_shortage, version, order_created_at, updated_at
            ) values (
                $1, $2, $3, $4, $5, $6, 'PENDING_SHIPMENT',
                'REVIEW_PENDING', 'PENDING_REVIEW', 'NOT_SUBMITTED', 'MANUAL_HOLD', 'MANUAL_REVIEW',
                $7::jsonb, $8, '测试地址', $9, $10,
                false, 0, $11, $11
            )
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .bind(order_month)
        .bind("taobao")
        .bind(shop_id)
        .bind(platform_order_id)
        .bind(exception_flags)
        .bind(receiver_name)
        .bind(exception_reason)
        .bind(amount_cent)
        .bind(now)
        .execute(db.pool())
        .await?;

        sqlx::query(
            r#"
            insert into order_reviews (
                tenant_id, order_id, status, notes, created_at, updated_at
            ) values ($1, $2, 'PENDING', '', $3, $3)
            on conflict (tenant_id, order_id) do nothing
            "#,
        )
        .bind(tenant_id)
        .bind(order_id)
        .bind(now)
        .execute(db.pool())
        .await?;
    }

    sqlx::query(
        r#"
        insert into order_exception_tickets (
            tenant_id, order_id, reason_code, status, created_at, updated_at
        ) values ($1, $2, 'MAPPING_MISSING', 'OPEN', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id_b)
    .bind(now - Duration::minutes(25))
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state(db.clone());
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["orders.read", "orders.write", "exports.write"],
    );

    let default_rule_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/settlements/rules"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let default_rule_response = app.clone().oneshot(default_rule_request).await.unwrap();
    assert_eq!(default_rule_response.status(), StatusCode::OK);

    let update_rule_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/settlements/rules"
        ))
        .method("PUT")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "stack_mode": "STACKED",
                "member_discount_rate": 3,
                "promo_discount_rate": 5,
                "max_discount_rate": 12
            })
            .to_string(),
        ))
        .unwrap();
    let update_rule_response = app.clone().oneshot(update_rule_request).await.unwrap();
    assert_eq!(update_rule_response.status(), StatusCode::OK);

    let settlement_list_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/settlements?created_from={}&created_to={}",
            created_from.to_rfc3339(),
            created_to.to_rfc3339()
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let settlement_list_response = app.clone().oneshot(settlement_list_request).await.unwrap();
    assert_eq!(settlement_list_response.status(), StatusCode::OK);
    let settlement_list_body = settlement_list_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let settlement_list_json: serde_json::Value = serde_json::from_slice(&settlement_list_body)?;
    assert_eq!(
        settlement_list_json
            .get("items")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );
    assert_eq!(
        settlement_list_json
            .get("summary")
            .and_then(|value| value.get("receivable_amount_cent"))
            .and_then(|value| value.as_i64()),
        Some(16560)
    );

    let lock_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/settlements/{order_id_a}/lock"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "operator_id": reviewer_id,
                "operator_name": "结算员甲"
            })
            .to_string(),
        ))
        .unwrap();
    let lock_response = app.clone().oneshot(lock_request).await.unwrap();
    assert_eq!(lock_response.status(), StatusCode::OK);

    let export_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/settlements/{order_id_a}/export"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "operator_id": reviewer_id,
                "operator_name": "结算员甲",
                "export_job_id": "job-settlement-001"
            })
            .to_string(),
        ))
        .unwrap();
    let export_response = app.clone().oneshot(export_request).await.unwrap();
    assert_eq!(export_response.status(), StatusCode::OK);
    let export_body = export_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let export_json: serde_json::Value = serde_json::from_slice(&export_body)?;
    assert_eq!(
        export_json
            .get("settlement_status")
            .and_then(|value| value.as_str()),
        Some("EXPORTED")
    );
    assert_eq!(
        export_json
            .get("export_job_id")
            .and_then(|value| value.as_str()),
        Some("job-settlement-001")
    );

    let reconcile_list_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/reconciliations?created_from={}&created_to={}",
            created_from.to_rfc3339(),
            created_to.to_rfc3339()
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let reconcile_list_response = app.clone().oneshot(reconcile_list_request).await.unwrap();
    assert_eq!(reconcile_list_response.status(), StatusCode::OK);
    let reconcile_list_body = reconcile_list_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let reconcile_list_json: serde_json::Value = serde_json::from_slice(&reconcile_list_body)?;
    let batches = reconcile_list_json
        .get("items")
        .and_then(|value| value.as_array())
        .expect("reconciliation items should exist");
    assert_eq!(batches.len(), 1);
    let batch_id = batches[0]
        .get("batch_id")
        .and_then(|value| value.as_str())
        .expect("batch_id should exist")
        .to_string();
    assert_eq!(
        batches[0].get("status").and_then(|value| value.as_str()),
        Some("PENDING")
    );
    assert_eq!(
        batches[0]
            .get("period_start")
            .and_then(|value| value.as_str()),
        Some(period_start.to_string().as_str())
    );
    assert_eq!(
        batches[0]
            .get("period_end")
            .and_then(|value| value.as_str()),
        Some(period_end.to_string().as_str())
    );

    let reconcile_detail_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/reconciliations/{batch_id}"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let reconcile_detail_response = app.clone().oneshot(reconcile_detail_request).await.unwrap();
    assert_eq!(reconcile_detail_response.status(), StatusCode::OK);
    let reconcile_detail_body = reconcile_detail_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let reconcile_detail_json: serde_json::Value = serde_json::from_slice(&reconcile_detail_body)?;
    assert_eq!(
        reconcile_detail_json
            .get("orders")
            .and_then(|value| value.as_array())
            .map(|items| items.len()),
        Some(2)
    );

    let settle_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/reconciliations/{batch_id}/settle"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "operator_id": reviewer_id,
                "operator_name": "对账员甲",
                "note": "差异已复核"
            })
            .to_string(),
        ))
        .unwrap();
    let settle_response = app.oneshot(settle_request).await.unwrap();
    assert_eq!(settle_response.status(), StatusCode::OK);
    let settle_body = settle_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let settle_json: serde_json::Value = serde_json::from_slice(&settle_body)?;
    assert_eq!(
        settle_json.get("status").and_then(|value| value.as_str()),
        Some("DONE")
    );

    Ok(())
}

async fn oms_exception_pool_exposes_sla_and_action_routes_inner() -> Result<()> {
    let Some(db) = connect_test_db("oms_exception_pool_exposes_sla_and_action_routes").await else {
        return Ok(());
    };
    setup_schema(db.pool()).await?;

    let tenant_id = uuid::Uuid::new_v4();
    let shop_id = uuid::Uuid::new_v4();
    let order_id = uuid::Uuid::new_v4();
    let reviewer_id = uuid::Uuid::new_v4();
    let now = Utc::now();
    let overdue_created_at = now - Duration::minutes(45);
    let order_month = overdue_created_at.date_naive().with_day(1).unwrap();

    sqlx::query(
        r#"
        insert into tenants (tenant_id, name, status)
        values ($1, $2, 'ACTIVE')
        "#,
    )
    .bind(tenant_id)
    .bind("oms-exception-tenant")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into shops (shop_id, tenant_id, platform, platform_shop_id, shop_name)
        values ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(shop_id)
    .bind(tenant_id)
    .bind("taobao")
    .bind("SHOP-OMS-1")
    .bind("异常店铺")
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_index (
            tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
        ) values ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind("taobao")
    .bind("PLAT-OMS-EX-1")
    .bind(shop_id)
    .bind(overdue_created_at)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into orders (
            tenant_id, order_month, order_id, platform, shop_id, platform_order_id, status,
            oms_status, order_status, fulfillment_status, exception_status, review_status,
            exception_flags, receiver_name, receiver_address, exception_reason, total_amount_cent,
            inventory_shortage, version, order_created_at, updated_at
        ) values (
            $1, $2, $3, $4, $5, $6, 'PENDING_SHIPMENT',
            'REVIEW_PENDING', 'PENDING_REVIEW', 'NOT_SUBMITTED', 'MANUAL_HOLD', 'MANUAL_REVIEW',
            '["MAPPING_MISSING"]'::jsonb, '张三', '测试地址', '货品映射缺失', 12800,
            false, 0, $7, $8
        )
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(order_month)
    .bind("taobao")
    .bind(shop_id)
    .bind("PLAT-OMS-EX-1")
    .bind(overdue_created_at)
    .bind(overdue_created_at)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_exception_tickets (
            tenant_id, order_id, reason_code, status, created_at, updated_at
        ) values ($1, $2, 'MAPPING_MISSING', 'OPEN', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(overdue_created_at)
    .execute(db.pool())
    .await?;

    sqlx::query(
        r#"
        insert into order_reviews (
            tenant_id, order_id, status, notes, created_at, updated_at
        ) values ($1, $2, 'PENDING', '', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(order_id)
    .bind(overdue_created_at)
    .execute(db.pool())
    .await?;

    let (app, jwt) = build_state(db.clone());
    let token = build_token(
        &jwt,
        &tenant_id.to_string(),
        vec!["orders.read", "orders.write"],
    );

    let list_request = Request::builder()
        .uri(format!("/v1/tenants/{tenant_id}/oms/orders/exceptions"))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let list_response = app.clone().oneshot(list_request).await.unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = list_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let list_json: serde_json::Value = serde_json::from_slice(&list_body)?;
    let items = list_json
        .get("items")
        .and_then(|value| value.as_array())
        .expect("items should exist");
    assert_eq!(items.len(), 1);
    let item = &items[0];
    assert_eq!(
        item.get("order_id").and_then(|value| value.as_str()),
        Some(order_id.to_string().as_str())
    );
    assert_eq!(
        item.get("exception_status")
            .and_then(|value| value.as_str()),
        Some("OPEN")
    );
    assert_eq!(
        item.get("audit_status").and_then(|value| value.as_str()),
        Some("REVIEW_REQUIRED")
    );
    assert_eq!(
        item.get("sla_status").and_then(|value| value.as_str()),
        Some("OVERDUE")
    );
    assert!(
        item.get("sla_remaining_minutes")
            .and_then(|value| value.as_i64())
            .unwrap_or_default()
            < 0
    );

    let claim_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/exceptions/{order_id}/claim"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "operator_id": reviewer_id,
                "operator_name": "处理人A"
            })
            .to_string(),
        ))
        .unwrap();
    let claim_response = app.clone().oneshot(claim_request).await.unwrap();
    assert_eq!(claim_response.status(), StatusCode::OK);
    let claim_body = claim_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let claim_json: serde_json::Value = serde_json::from_slice(&claim_body)?;
    assert_eq!(
        claim_json
            .get("exception_status")
            .and_then(|value| value.as_str()),
        Some("PROCESSING")
    );
    assert_eq!(
        claim_json
            .get("assignee_id")
            .and_then(|value| value.as_str()),
        Some(reviewer_id.to_string().as_str())
    );

    let escalate_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/exceptions/{order_id}/escalate"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "operator_id": reviewer_id,
                "operator_name": "处理人A",
                "reason": "超过首响时限"
            })
            .to_string(),
        ))
        .unwrap();
    let escalate_response = app.clone().oneshot(escalate_request).await.unwrap();
    assert_eq!(escalate_response.status(), StatusCode::OK);
    let escalate_body = escalate_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let escalate_json: serde_json::Value = serde_json::from_slice(&escalate_body)?;
    assert_eq!(
        escalate_json
            .get("exception_status")
            .and_then(|value| value.as_str()),
        Some("ESCALATED")
    );

    let resolve_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/exceptions/{order_id}/resolve"
        ))
        .method("POST")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "operator_id": reviewer_id,
                "operator_name": "处理人A",
                "reason": "已补全映射并复核通过"
            })
            .to_string(),
        ))
        .unwrap();
    let resolve_response = app.clone().oneshot(resolve_request).await.unwrap();
    assert_eq!(resolve_response.status(), StatusCode::OK);
    let resolve_body = resolve_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let resolve_json: serde_json::Value = serde_json::from_slice(&resolve_body)?;
    assert_eq!(
        resolve_json
            .get("exception_status")
            .and_then(|value| value.as_str()),
        Some("RESOLVED")
    );
    assert_eq!(
        resolve_json
            .get("blocking_reason")
            .and_then(|value| value.as_str()),
        Some("已补全映射并复核通过")
    );

    let detail_request = Request::builder()
        .uri(format!(
            "/v1/tenants/{tenant_id}/oms/orders/exceptions/{order_id}"
        ))
        .method("GET")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let detail_response = app.oneshot(detail_request).await.unwrap();
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_body = detail_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let detail_json: serde_json::Value = serde_json::from_slice(&detail_body)?;
    let action_logs = detail_json
        .get("action_logs")
        .and_then(|value| value.as_array())
        .expect("action_logs should exist");
    assert!(action_logs.len() >= 3);
    assert_eq!(
        detail_json
            .get("exception_status")
            .and_then(|value| value.as_str()),
        Some("RESOLVED")
    );
    assert_eq!(
        detail_json
            .get("sla_status")
            .and_then(|value| value.as_str()),
        Some("NORMAL")
    );

    Ok(())
}
