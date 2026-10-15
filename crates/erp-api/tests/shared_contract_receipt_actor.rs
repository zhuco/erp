//! Direct repository contracts still use a real opened session and explicit stored grants.
use argon2::password_hash::{PasswordHash, SaltString};
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use erp_repo::access_plan::AccessPlanRepo;
use erp_repo::shared_contract::ReceiptValueActor;
use erp_types::{TenantId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn actor(
    pool: &PgPool,
    tenant: TenantId,
    warehouse: Uuid,
) -> anyhow::Result<ReceiptValueActor> {
    let identity: (String, String, String, i32) = sqlx::query_as(
        "select current_database(),current_user,host(inet_server_addr()),inet_server_port()",
    )
    .fetch_one(pool)
    .await?;
    if identity
        != (
            "erp_shared_contract_test".into(),
            "erp_grid_test".into(),
            "127.0.0.1".into(),
            15438,
        )
    {
        anyhow::bail!("protected receipt direct-contract database required");
    }
    let existing: i64 =
        sqlx::query_scalar("select count(*) from users where tenant_id=$1 and status='ACTIVE'")
            .bind(tenant.0)
            .fetch_one(pool)
            .await?;
    if existing != 0 {
        anyhow::bail!("direct receipt fixture requires its own unused account tenant");
    }
    let user = Uuid::new_v4();
    let role = Uuid::new_v4();
    let password = "owned-receipt-contract-password";
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("owned password hash failed: {e}"))?
        .to_string();
    let mut tx = pool.begin().await?;
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-RECEIPT-DIRECT') on conflict(tenant_id) do nothing")
        .bind(tenant.0).execute(&mut *tx).await?;
    sqlx::query("insert into warehouses(warehouse_id,tenant_id,code,name) values($1,$2,$3,'QA receipt direct') on conflict(warehouse_id) do nothing")
        .bind(warehouse).bind(tenant.0).bind(warehouse.to_string()).execute(&mut *tx).await?;
    sqlx::query("insert into users(user_id,tenant_id,email,password_hash,status) values($1,$2,$3,$4,'ACTIVE')")
        .bind(user).bind(tenant.0).bind(format!("{user}@receipt-direct.qa.invalid")).bind(&hash).execute(&mut *tx).await?;
    sqlx::query("insert into roles(role_id,tenant_id,name) values($1,$2,'QA receipt direct')")
        .bind(role)
        .bind(tenant.0)
        .execute(&mut *tx)
        .await?;
    sqlx::query("insert into user_roles(tenant_id,user_id,role_id) values($1,$2,$3)")
        .bind(tenant.0)
        .bind(user)
        .bind(role)
        .execute(&mut *tx)
        .await?;
    sqlx::query("insert into role_permissions(tenant_id,role_id,perm_code) values($1,$2,'PROCUREMENT.ORDER.WRITE')")
        .bind(tenant.0).bind(role).execute(&mut *tx).await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(tenant.0)
    .bind(user)
    .bind(warehouse)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let parsed = PasswordHash::new(&hash).map_err(|e| anyhow::anyhow!("owned PHC invalid: {e}"))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|e| anyhow::anyhow!("owned genuine password verification failed: {e}"))?;
    let session = AccessPlanRepo::new(pool)
        .open_session(
            tenant,
            UserId(user),
            &hash,
            "DESKTOP",
            "QA receipt direct",
            chrono::Utc::now() + chrono::Duration::hours(1),
        )
        .await?
        .map_err(|e| anyhow::anyhow!("owned genuine session not admitted: {e}"))?;
    eprintln!(
        "QA receipt_direct_actor tenant={tenant} actor={user} session={} warehouse={warehouse}",
        session.session_id
    );
    Ok(ReceiptValueActor {
        authentication_tenant: tenant,
        user_id: UserId(user),
        session_id: session.session_id,
        credential_epoch: session.credentials_epoch,
        warehouse_id: warehouse,
    })
}

pub(super) async fn http_token(
    app: &axum::Router,
    jwt: &erp_api::JwtService,
    actor: ReceiptValueActor,
) -> anyhow::Result<String> {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let response = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    serde_json::json!({
                      "tenant_id":actor.authentication_tenant,
                      "email":format!("{}@receipt-direct.qa.invalid",actor.user_id.0),
                      "password":"owned-receipt-contract-password",
                      "client_kind":"WEB","client_label":"QA-receipt-contract-web"
                    })
                    .to_string(),
                ))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    if status != axum::http::StatusCode::OK {
        anyhow::bail!("owned real login failed: {status}, code={}", value["code"]);
    }
    let token = value["access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("owned login token missing"))?
        .to_string();
    let session = value["session_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("owned login session missing"))?;
    let claims = jwt.decode(&token)?;
    if claims.sub != actor.user_id.to_string()
        || claims.tenant_id != actor.authentication_tenant.to_string()
        || claims.sid.as_deref() != Some(session)
        || claims.cep != Some(actor.credential_epoch)
        || claims.is_admin
    {
        anyhow::bail!("owned login returned a different actor or unqualified session");
    }
    eprintln!(
        "QA receipt_contract_password tenant={} actor={} session={session} epoch={} warehouse={}",
        actor.authentication_tenant, actor.user_id, actor.credential_epoch, actor.warehouse_id
    );
    Ok(token)
}
