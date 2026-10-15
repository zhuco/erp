//! Ordinary accounts authenticate through the shipped password login handler.
//! The fixed credential and signing key belong only to fresh, isolated QA users.
use super::{Result, isolated_pool};
use anyhow::{Context, anyhow};
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, JwtService, build_app};
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
use tokio::sync::OnceCell;
use tower::ServiceExt;
use uuid::Uuid;

const PASSWORD: &str = "ERP-owned-QA-real-password-login-2026!";
const CHANGED_PASSWORD: &str = "ERP-owned-QA-real-password-after-change-2026!";
const SIGNING_KEY: &str = "isolated-erp-qa-server-signing-key-real-login";
pub(super) const STOCK_PERMISSIONS: &[&str] = &[
    "WMS.INVENTORY.READ",
    "WMS.INVENTORY.WRITE",
    "MFG.PRODUCTION.WRITE",
    "manufacturing.read",
    "quality.read",
    "quality.write",
];

#[derive(Clone)]
pub(super) struct AccountLogin {
    pub(super) tenant: TenantId,
    pub(super) actor: Uuid,
    pub(super) session: Uuid,
    pub(super) jwt: Arc<JwtService>,
    pub(super) token: String,
    email: String,
}

async fn password_hash() -> Result<String> {
    static HASH: OnceCell<String> = OnceCell::const_new();
    HASH.get_or_try_init(|| async {
        tokio::task::spawn_blocking(|| {
            let salt = SaltString::generate(&mut rand::thread_rng());
            Argon2::default()
                .hash_password(PASSWORD.as_bytes(), &salt)
                .map(|value| value.to_string())
                .map_err(|error| anyhow!("QA password hash failed: {error}"))
        })
        .await?
    })
    .await
    .cloned()
}

pub(super) async fn create_user(
    pool: &PgPool,
    tenant: TenantId,
    actor: Uuid,
    email: String,
    permissions: &[&str],
) -> Result<AccountLogin> {
    let mut tx = pool.begin().await?;
    sqlx::query("insert into users(tenant_id,user_id,email,password_hash,status) values($1,$2,$3,$4,'ACTIVE')")
        .bind(tenant.0).bind(actor).bind(&email).bind(password_hash().await?)
        .execute(&mut *tx).await?;
    let role = Uuid::new_v4();
    sqlx::query("insert into roles(tenant_id,role_id,name) values($1,$2,$3)")
        .bind(tenant.0)
        .bind(role)
        .bind(format!("QA-real-login-{role}"))
        .execute(&mut *tx)
        .await?;
    sqlx::query("insert into user_roles(tenant_id,user_id,role_id) values($1,$2,$3)")
        .bind(tenant.0)
        .bind(actor)
        .bind(role)
        .execute(&mut *tx)
        .await?;
    for permission in permissions {
        sqlx::query("insert into role_permissions(tenant_id,role_id,perm_code) values($1,$2,$3)")
            .bind(tenant.0)
            .bind(role)
            .bind(permission)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    login(
        tenant,
        actor,
        email,
        PASSWORD,
        "DESKTOP",
        "QA-owned-device-A",
    )
    .await
}

/// Cross-principal tests use paid QA tenants so they do not bypass the free seat rule.
pub(super) async fn paid_peer(
    pool: &PgPool,
    tenant: TenantId,
    warehouse: Uuid,
    permissions: &[&str],
) -> Result<AccountLogin> {
    let mut tx = pool.begin().await?;
    let name: String = sqlx::query_scalar("select name from tenants where tenant_id=$1 for update")
        .bind(tenant.0)
        .fetch_one(&mut *tx)
        .await?;
    assert!(
        matches!(
            name.as_str(),
            "QA-INVENTORY-COMMANDS"
                | "QA-INVENTORY-LOCATION-COMMANDS"
                | "QA-INVENTORY-LOCATION-OUTBOUND"
                | "QA-INVENTORY-LOCATION-RESERVATIONS"
        ),
        "paid peer setup is restricted to this harness's fresh inventory tenants"
    );
    let occupied: i64 =
        sqlx::query_scalar("select count(*) from users where tenant_id=$1 and status='ACTIVE'")
            .bind(tenant.0)
            .fetch_one(&mut *tx)
            .await?;
    let seats = i32::try_from(occupied.checked_add(1).context("QA seat count overflow")?)?;
    sqlx::query("update tenants set plan_code='PAID',login_seat_limit=$2 where tenant_id=$1")
        .bind(tenant.0)
        .bind(seats)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let actor = Uuid::new_v4();
    let account = create_user(
        pool,
        tenant,
        actor,
        format!("{actor}@paid-peer.qa.invalid"),
        permissions,
    )
    .await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(tenant.0)
    .bind(actor)
    .bind(warehouse)
    .execute(pool)
    .await?;
    eprintln!(
        "QA real_login_paid_peer tenant={tenant} actor={actor} warehouse={warehouse} seats={seats}"
    );
    Ok(account)
}

async fn login(
    tenant: TenantId,
    actor: Uuid,
    email: String,
    password: &str,
    kind: &str,
    label: &str,
) -> Result<AccountLogin> {
    let jwt = Arc::new(JwtService::new(SIGNING_KEY.into()));
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        Arc::clone(&jwt),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"tenant_id":tenant,"email":email,"password":password,
            "client_kind":kind,"client_label":label})
                    .to_string(),
                ))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let value: Value = serde_json::from_slice(&bytes).context("real login must return JSON")?;
    if status != StatusCode::OK {
        return Err(anyhow!(
            "real password login rejected: {status}, code={}",
            value["code"]
        ));
    }
    let token = value["access_token"]
        .as_str()
        .context("login token missing")?
        .to_owned();
    let session = Uuid::parse_str(
        value["session_id"]
            .as_str()
            .context("login session missing")?,
    )?;
    let claims = jwt.decode(&token)?;
    assert_eq!(claims.sub, actor.to_string());
    assert_eq!(claims.tenant_id, tenant.to_string());
    assert_eq!(claims.sid.as_deref(), Some(session.to_string().as_str()));
    assert!(claims.cep.is_some());
    assert!(
        !claims.is_admin,
        "ordinary business login must not become platform admin"
    );
    eprintln!(
        "QA real_login_password tenant={tenant} actor={actor} kind={kind} label={label} session={session} epoch={:?}",
        claims.cep
    );
    Ok(AccountLogin {
        tenant,
        actor,
        session,
        jwt,
        token,
        email,
    })
}

impl AccountLogin {
    /// Only fresh QA credentials; callers must never print this request body.
    pub(super) fn password_login_payload(&self, kind: &str, label: &str) -> Value {
        json!({"tenant_id":self.tenant,"email":self.email,"password":PASSWORD,
               "client_kind":kind,"client_label":label})
    }

    pub(super) fn app(&self, url: &str) -> Result<Router> {
        Ok(build_app(AppState::new(
            Db::connect_lazy(url)?,
            Arc::clone(&self.jwt),
        )))
    }

    pub(super) async fn device(&self, kind: &str, label: &str) -> Result<Self> {
        login(
            self.tenant,
            self.actor,
            self.email.clone(),
            PASSWORD,
            kind,
            label,
        )
        .await
    }

    pub(super) async fn request(
        &self,
        method: &str,
        path: &str,
        body: Value,
    ) -> Result<(StatusCode, Value)> {
        let response = self
            .app(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("authorization", format!("Bearer {}", self.token))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))?,
            )
            .await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        let value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
        eprintln!(
            "QA real_login_http tenant={} actor={} session={} method={method} path={path} status={} response={value}",
            self.tenant,
            self.actor,
            self.session,
            status.as_u16()
        );
        Ok((status, value))
    }
}

struct StockAccount {
    account: AccountLogin,
    warehouse: Uuid,
    sku: Uuid,
}

async fn stock_account(pool: &PgPool, case: &str) -> Result<StockAccount> {
    let tenant = TenantId(Uuid::new_v4());
    let actor = Uuid::new_v4();
    let warehouse = Uuid::new_v4();
    let sku = Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-ERP-REAL-LOGIN')")
        .bind(tenant.0)
        .execute(pool)
        .await?;
    let account = create_user(
        pool,
        tenant,
        actor,
        format!("{actor}@real-login.qa.invalid"),
        STOCK_PERMISSIONS,
    )
    .await?;
    sqlx::query(
        "insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Real login QA',$3)",
    )
    .bind(tenant.0)
    .bind(warehouse)
    .bind(warehouse.to_string())
    .execute(pool)
    .await?;
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-REAL-LOGIN',(select product_id from qa_parent))")
        .bind(tenant.0)
        .bind(sku)
        .execute(pool)
        .await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(tenant.0)
    .bind(actor)
    .bind(warehouse)
    .execute(pool)
    .await?;
    InventoryService::new(pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: tenant,
                warehouse_id: warehouse,
                sku_id: sku,
            },
            on_hand: parse_qty("4.9")?,
            reserved: parse_qty("0")?,
            frozen: parse_qty("0")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_REAL_LOGIN_OPENING".into(),
            ref_type: Some("QA_REAL_LOGIN_OPENING".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    eprintln!(
        "QA real_login_fixture tenant={tenant} actor={actor} warehouse={warehouse} sku={sku} case={case} session={}",
        account.session
    );
    Ok(StockAccount {
        account,
        warehouse,
        sku,
    })
}

fn movement(f: &StockAccount, request: &str, quantity: i64) -> Value {
    json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"request_id":request,"quantity":quantity,
           "reason":"QA_REAL_LOGIN","reference_id":"QA-REAL-LOGIN-REF"})
}

/// Explicitly selected immutable evidence exercises a real pre-repair v1 commit.
/// The normal cases remain self-contained when this historical QA gate is absent.
async fn preserved_v1_original(pool: &PgPool) -> Result<()> {
    let Some(path) = std::env::var_os("ERP_REAL_LOGIN_LEGACY_FIXTURE") else {
        return Ok(());
    };
    let original: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let field = |name: &str| {
        original[name]
            .as_str()
            .with_context(|| format!("legacy QA {name} missing"))
    };
    let tenant = TenantId(Uuid::parse_str(field("tenant_id")?)?);
    let actor = Uuid::parse_str(field("actor_user_id")?)?;
    let warehouse = Uuid::parse_str(field("warehouse_id")?)?;
    let sku = Uuid::parse_str(field("sku_id")?)?;
    let request = field("request_id")?;
    let name: String = sqlx::query_scalar("select name from tenants where tenant_id=$1")
        .bind(tenant.0)
        .fetch_one(pool)
        .await?;
    assert_eq!(name, "QA-ERP-REAL-LOGIN");
    let before: (String, Value) = sqlx::query_as("select payload_hash,original_result from command_receipts where tenant_id=$1 and request_id=$2 and actor_user_id=$3")
        .bind(tenant.0).bind(request).bind(actor).fetch_one(pool).await?;
    assert_eq!(before.0, field("payload_hash")?);
    assert_eq!(before.1, original["original_result"]);
    assert_eq!(before.1["version"], 1);
    let account = login(
        tenant,
        actor,
        field("email")?.to_owned(),
        PASSWORD,
        "WEB",
        "QA-restored-v1",
    )
    .await?;
    assert_ne!(before.1["input"]["session_id"], account.session.to_string());
    let root = format!("/v1/tenants/{tenant}/inventory");
    let (status, read) = account
        .request("GET", &format!("{root}/commands/{request}"), Value::Null)
        .await?;
    assert_eq!(status, StatusCode::OK, "{read}");
    let (status, replay) = account.request("POST", &format!("{root}/inbounds"), json!({
        "request_id":request,"warehouse_id":warehouse,"sku_id":sku,
        "quantity":before.1["input"]["quantity_delta"],
        "reason":before.1["input"]["reason"],"reference_id":before.1["input"]["reference_id"]
    })).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["effect_id"], before.1["ledger_id"]);
    assert_eq!(replay["payload_hash"], before.0);
    assert_eq!(replay["record"], read["record"]);
    assert_eq!(replay["record"]["quantities"]["on_hand"], "5.9");
    assert_eq!(replay["replay"], true);
    let after: (String, Value) = sqlx::query_as("select payload_hash,original_result from command_receipts where tenant_id=$1 and request_id=$2")
        .bind(tenant.0).bind(request).fetch_one(pool).await?;
    assert_eq!(after, before, "do not upgrade or rewrite the old receipt");
    let (hand, count, effects): (i64, i64, i64) = sqlx::query_as(
        "select b.on_hand_micros,
        (select count(*) from command_receipts where tenant_id=$1),
        (select count(*) from inventory_ledgers where tenant_id=$1 and ref_id=$4)
        from inventory_balances b where tenant_id=$1 and warehouse_id=$2 and sku_id=$3",
    )
    .bind(tenant.0)
    .bind(warehouse)
    .bind(sku)
    .bind(request)
    .fetch_one(pool)
    .await?;
    assert_eq!((hand, count, effects), (6_900_000, 2, 1));
    eprintln!(
        "QA real_login_legacy_original tenant={tenant} request_id={request} new_session={} original_version=1 original_hash={} hand={hand} receipts={count} effects={effects}",
        account.session, before.0
    );
    Ok(())
}

#[tokio::test]
async fn real_login_two_devices_replay_original_inventory_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    preserved_v1_original(&independent).await?;
    let f = stock_account(&pool, "cross-device-original").await?;
    let b = f.account.device("WEB", "QA-owned-device-B").await?;
    assert_ne!(f.account.session, b.session);
    let original = Uuid::new_v4().to_string();
    let path = format!("/v1/tenants/{}/inventory/inbounds", f.account.tenant);
    let (status, first) = f
        .account
        .request("POST", &path, movement(&f, &original, 1))
        .await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (status, later) = b
        .request("POST", &path, movement(&f, &Uuid::new_v4().to_string(), 1))
        .await?;
    assert_eq!(status, StatusCode::OK, "{later}");
    let (status, retry) = b.request("POST", &path, movement(&f, &original, 1)).await?;
    let (hand, receipts, original_effects, original_session): (i64,i64,i64,String) = sqlx::query_as(
        "select b.on_hand_micros,(select count(*) from command_receipts where tenant_id=$1),
        (select count(*) from inventory_ledgers where tenant_id=$1 and ref_id=$4),
        (select original_result->'input'->>'session_id' from command_receipts where tenant_id=$1 and request_id=$4)
        from inventory_balances b where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(f.account.tenant.0).bind(f.warehouse).bind(f.sku).bind(&original).fetch_one(&independent).await?;
    eprintln!(
        "QA real_login_cross_device tenant={} request_id={original} session_a={} session_b={} hand={hand} receipts={receipts} original_effects={original_effects} original_session={original_session} retry_status={}",
        f.account.tenant,
        f.account.session,
        b.session,
        status.as_u16()
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "same user, same business request must replay across sessions: {retry}"
    );
    assert_eq!(retry["effect_id"], first["effect_id"]);
    assert_eq!(retry["record"], first["record"]);
    assert_eq!(retry["replay"], true);
    assert_ne!(retry["record"], later["record"]);
    assert_eq!((hand, receipts, original_effects), (6_900_000, 2, 1));
    assert_eq!(original_session, f.account.session.to_string());
    Ok(())
}

#[tokio::test]
async fn real_login_named_device_reuses_session_at_ceiling() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = stock_account(&pool, "same-device-ceiling").await?;
    for index in 0..7 {
        f.account
            .device("WEB", &format!("QA-extra-device-{index}"))
            .await?;
    }
    let reused = f.account.device("DESKTOP", "QA-owned-device-A").await?;
    assert_eq!(reused.session, f.account.session);
    let active: i64 = sqlx::query_scalar("select count(*) from auth_sessions where tenant_id=$1 and user_id=$2 and status='ACTIVE' and expires_at>now()")
        .bind(f.account.tenant.0).bind(f.account.actor).fetch_one(&independent).await?;
    assert_eq!(active, 8);
    eprintln!(
        "QA real_login_session_ceiling tenant={} session={} active={active}",
        f.account.tenant, reused.session
    );
    Ok(())
}

#[tokio::test]
async fn real_login_after_password_change_uses_current_epoch_session() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = stock_account(&pool, "password-new-session").await?;
    let (status, value) = f
        .account
        .request(
            "PUT",
            "/me/password",
            json!({"current_password":PASSWORD,"new_password":CHANGED_PASSWORD}),
        )
        .await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    let fresh = login(
        f.account.tenant,
        f.account.actor,
        f.account.email.clone(),
        CHANGED_PASSWORD,
        "DESKTOP",
        "QA-owned-device-A",
    )
    .await?;
    let path = format!("/v1/tenants/{}/inventory/inbounds", f.account.tenant);
    let request = Uuid::new_v4().to_string();
    let (status, value) = fresh
        .request("POST", &path, movement(&f, &request, 1))
        .await?;
    let (current_epoch, session_epoch, hand): (i32,i32,i64) = sqlx::query_as(
        "select t.credentials_epoch,s.credentials_epoch,b.on_hand_micros from tenants t
        join auth_sessions s on s.tenant_id=t.tenant_id and s.session_id=$2
        join inventory_balances b on b.tenant_id=t.tenant_id and b.warehouse_id=$3 and b.sku_id=$4 where t.tenant_id=$1")
        .bind(f.account.tenant.0).bind(fresh.session).bind(f.warehouse).bind(f.sku).fetch_one(&independent).await?;
    eprintln!(
        "QA real_login_password_epoch tenant={} old_session={} new_session={} tenant_epoch={current_epoch} session_epoch={session_epoch} hand={hand} status={}",
        f.account.tenant,
        f.account.session,
        fresh.session,
        status.as_u16()
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "new valid password must issue a usable session: {value}"
    );
    assert_ne!(fresh.session, f.account.session);
    assert_eq!((current_epoch, session_epoch, hand), (1, 1, 5_900_000));
    Ok(())
}
