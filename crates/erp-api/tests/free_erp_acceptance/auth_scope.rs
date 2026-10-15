//! Authorization storage failures must stop a request before its handler runs.

use super::{Result, isolated_pool};
use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn authorization_scope_storage_failure_does_not_admit_the_handler() -> Result<()> {
    // Verify the configured QA identity without changing the shared database.
    let guard_pool = isolated_pool().await?;
    guard_pool.close().await;

    let tenant = Uuid::new_v4();
    let jwt = Arc::new(JwtService::new("synthetic-scope-failure-key".into()));
    let now = Utc::now().timestamp() as usize;
    let claims = Claims::bearer(
        Uuid::new_v4().to_string(),
        tenant.to_string(),
        vec!["manufacturing.write".into()],
        false,
        now,
        now + 300,
    );
    let token = jwt.encode(&claims)?;
    let db = Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?;
    // Close only this test's newly created application pool. Other fixtures and
    // the PostgreSQL server remain live. Scope lookup now returns PoolClosed.
    db.pool().close().await;
    let app = build_app(AppState::new(db, jwt));
    let path = format!("/v1/tenants/{tenant}/manufacturing/boms");

    for (credential, expected) in [
        (Some(token.as_str()), StatusCode::SERVICE_UNAVAILABLE),
        (Some("invalid-token"), StatusCode::UNAUTHORIZED),
        (None, StatusCode::UNAUTHORIZED),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri(&path)
            .header("content-type", "application/json");
        if let Some(credential) = credential {
            request = request.header("authorization", format!("Bearer {credential}"));
        }
        // Deliberately invalid BOM fields: 422 proves the handler's extractor
        // was admitted even though authorization storage was unavailable.
        let response = app.clone().oneshot(request.body(Body::from("{}"))?).await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        eprintln!(
            "QA scope failure status={status} body={}",
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(status, expected, "{}", String::from_utf8_lossy(&bytes));
        if expected == StatusCode::SERVICE_UNAVAILABLE {
            assert_eq!(&bytes[..], b"data scope check unavailable");
        }
    }
    Ok(())
}

#[tokio::test]
async fn authorization_scope_policy_failure_does_not_disable_scope_checks() -> Result<()> {
    let pool = isolated_pool().await?;
    let schema = format!("qa_scope_policy_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("create schema {schema}"))
        .execute(&pool)
        .await?;
    for table in [
        "user_shop_access",
        "user_warehouse_access",
        "user_merchant_access",
        "user_organization_access",
    ] {
        sqlx::query(&format!(
            "create view {schema}.{table} as select * from public.{table}"
        ))
        .execute(&pool)
        .await?;
    }
    // Scoped views leave the shared public schema untouched. DataScopesRepo's
    // four queries succeed; only the tenant policy columns are unavailable.
    sqlx::query(&format!(
        "create view {schema}.tenants as select tenant_id from public.tenants"
    ))
    .execute(&pool)
    .await?;
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let separator = if url.contains('?') { '&' } else { '?' };
    let url = format!("{url}{separator}options=-c%20search_path%3D{schema}%2Cpg_catalog");
    assert!(super::approved_database_url(&url)?);
    let db = Db::connect_lazy(&url)?;
    let actual_path: String = sqlx::query_scalar("show search_path")
        .fetch_one(db.pool())
        .await?;
    assert_eq!(actual_path, format!("{schema},pg_catalog"));
    let jwt = Arc::new(JwtService::new("synthetic-scope-policy-key".into()));
    let tenant = Uuid::new_v4();
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        Uuid::new_v4().to_string(),
        tenant.to_string(),
        vec!["manufacturing.write".into()],
        false,
        now,
        now + 300,
    ))?;
    let app = build_app(AppState::new(db, jwt));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/tenants/{tenant}/manufacturing/boms"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from("{}"))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    eprintln!(
        "QA scope policy schema={schema} status={status} body={}",
        String::from_utf8_lossy(&bytes)
    );
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(&bytes[..], b"tenant scope policy unavailable");
    Ok(())
}

#[tokio::test]
async fn authorization_scope_allows_only_the_granted_warehouse() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    let user = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let other_warehouse = Uuid::new_v4();
    let sku = Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-AUTH-SCOPE')")
        .bind(tenant)
        .execute(&pool)
        .await?;
    sqlx::query("insert into users(tenant_id,user_id,email,password_hash) values($1,$2,$3,'synthetic-not-for-password-login')")
        .bind(tenant).bind(user).bind(format!("{user}@scope.invalid")).execute(&pool).await?;
    for id in [warehouse, other_warehouse] {
        sqlx::query(
            "insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Scope QA',$3)",
        )
        .bind(tenant)
        .bind(id)
        .bind(id.to_string())
        .execute(&pool)
        .await?;
    }
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-SCOPE-SKU',(select product_id from qa_parent))")
        .bind(tenant)
        .bind(sku)
        .execute(&pool)
        .await?;
    // Healthy warehouse authorization needs an authoritative, activated stock
    // key. An unactivated legacy row has a separate reconciliation acceptance.
    erp_repo::inventory::InventoryService::new(&pool)
        .apply(erp_repo::inventory::InventoryDelta {
            tenant_id: erp_types::TenantId(tenant),
            warehouse_id: warehouse,
            sku_id: sku,
            delta_on_hand: 1,
            delta_reserved: 0,
            delta_in_transit: 0,
            delta_frozen: 0,
            reason_type: "QA_OPENING".into(),
            ref_type: Some("QA_AUTH_OPENING".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(tenant)
    .bind(user)
    .bind(warehouse)
    .execute(&pool)
    .await?;
    let jwt = Arc::new(JwtService::new("synthetic-granted-scope-key".into()));
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        user.to_string(),
        tenant.to_string(),
        vec!["inventory.read".into()],
        false,
        now,
        now + 300,
    ))?;
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        jwt,
    ));
    for (destination, selected, expected) in [
        (tenant, warehouse, StatusCode::OK),
        (tenant, other_warehouse, StatusCode::FORBIDDEN),
        (Uuid::new_v4(), warehouse, StatusCode::FORBIDDEN),
    ] {
        let response = app.clone().oneshot(
            Request::builder()
                .uri(format!("/v1/tenants/{destination}/inventory/atp?warehouse_id={selected}&sku_id={sku}&request_qty=1"))
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())?,
        ).await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        assert_eq!(status, expected, "{}", String::from_utf8_lossy(&bytes));
        if expected == StatusCode::OK {
            let response: Value = serde_json::from_slice(&bytes)?;
            assert_eq!(response["available"], 1);
            assert_eq!(response["warehouse_id"], warehouse.to_string());
        }
    }
    let effects: (i64, i64) = sqlx::query_as("select on_hand,(select count(*) from inventory_ledgers where tenant_id=$1) from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(tenant).bind(warehouse).bind(sku).fetch_one(&pool).await?;
    assert_eq!(effects, (1, 1));
    eprintln!("QA authorized warehouse tenant={tenant} user={user} warehouse={warehouse}");
    Ok(())
}
