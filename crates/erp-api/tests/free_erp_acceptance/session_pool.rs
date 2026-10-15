//! Real password login must use one pooled connection and leave no session lock.
use super::account_login::{AccountLogin, STOCK_PERMISSIONS, create_user};
use super::{Result, approved_database_url, isolated_pool};
use anyhow::{Context, anyhow};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{AppState, build_app};
use erp_repo::{
    access_plan::AccessPlanRepo,
    db::Db,
    rbac::{RbacRepo, UserUpdate},
};
use erp_types::{TenantId, UserId};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    account: AccountLogin,
    pool: PgPool,
    app: Router,
    tag: String,
}

async fn fixture(independent: &PgPool, case: &str, limit: u32) -> Result<Fixture> {
    let tenant = TenantId(Uuid::new_v4());
    let actor = Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-SESSION-POOL')")
        .bind(tenant.0)
        .execute(independent)
        .await?;
    let account = create_user(
        independent,
        tenant,
        actor,
        format!("{actor}@session-pool.qa.invalid"),
        STOCK_PERMISSIONS,
    )
    .await?;
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    if !approved_database_url(&url)? {
        return Err(anyhow!("protected session-pool QA database required"));
    }
    let tag = format!("qa_login_pool_{tenant}");
    let options = PgConnectOptions::from_str(&url)?.application_name(&tag);
    let pool = PgPoolOptions::new()
        .max_connections(limit)
        .acquire_timeout(Duration::from_millis(500))
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("set statement_timeout='3s'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await?;
    let app = build_app(AppState::new(
        Db::from_pool(pool.clone()),
        Arc::clone(&account.jwt),
    ));
    eprintln!(
        "QA session_pool_fixture tenant={tenant} actor={actor} initial_session={} case={case} max_connections={limit} tag={tag}",
        account.session
    );
    Ok(Fixture {
        account,
        pool,
        app,
        tag,
    })
}

async fn login(app: Router, body: Value) -> Result<(StatusCode, Value)> {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()));
    Ok((status, value))
}

async fn sessions(independent: &PgPool, f: &Fixture) -> Result<(i64, i64)> {
    Ok(sqlx::query_as("select count(*),count(*) filter(where s.status='ACTIVE' and s.expires_at>clock_timestamp() and s.credentials_epoch=t.credentials_epoch and u.status='ACTIVE') from auth_sessions s join tenants t on t.tenant_id=s.tenant_id join users u on u.tenant_id=s.tenant_id and u.user_id=s.user_id where s.tenant_id=$1 and s.user_id=$2")
        .bind(f.account.tenant.0).bind(f.account.actor).fetch_one(independent).await?)
}

async fn verify_token(independent: &PgPool, f: &Fixture, body: &Value) -> Result<()> {
    let claims = f.account.jwt.decode(
        body["access_token"]
            .as_str()
            .context("server token missing")?,
    )?;
    assert_eq!(claims.sub, f.account.actor.to_string());
    assert_eq!(claims.tenant_id, f.account.tenant.to_string());
    assert_eq!(claims.cep, Some(0));
    assert!(!claims.is_admin);
    let sid = Uuid::parse_str(claims.sid.as_deref().context("server session missing")?)?;
    assert_eq!(body["session_id"], sid.to_string());
    let current: bool = sqlx::query_scalar("select exists(select 1 from auth_sessions where tenant_id=$1 and user_id=$2 and session_id=$3 and status='ACTIVE' and credentials_epoch=0 and expires_at>clock_timestamp())")
        .bind(f.account.tenant.0).bind(f.account.actor).bind(sid).fetch_one(independent).await?;
    assert!(
        current,
        "server-issued identity must bind the independently stored active session"
    );
    Ok(())
}

async fn advisory_locks(independent: &PgPool, f: &Fixture) -> Result<i64> {
    Ok(sqlx::query_scalar("select count(*) from pg_locks l join pg_stat_activity a on a.pid=l.pid where a.application_name=$1 and l.locktype='advisory' and l.granted")
        .bind(&f.tag).fetch_one(independent).await?)
}

#[tokio::test]
async fn session_pool_one_connection_real_password_login() -> Result<()> {
    let independent = isolated_pool().await?;
    let f = fixture(&independent, "one-connection", 1).await?;
    let (status, body) = login(
        f.app.clone(),
        f.account.password_login_payload("WEB", "QA-one-connection"),
    )
    .await?;
    if status == StatusCode::OK {
        verify_token(&independent, &f, &body).await?;
    }
    let (total, active) = sessions(&independent, &f).await?;
    let held_locks = advisory_locks(&independent, &f).await?;
    f.pool.close().await;
    let pool_timeout = body
        .as_str()
        .is_some_and(|text| text.contains("pool timed out"));
    eprintln!(
        "QA session_pool_result tenant={} case=one-connection statuses=[{}] total={total} active={active} held_locks={held_locks} pool_timeout={pool_timeout}",
        f.account.tenant,
        status.as_u16()
    );
    assert_eq!(
        status,
        StatusCode::OK,
        "one connection must suffice for a real password login"
    );
    assert_eq!((total, active), (2, 2));
    assert_eq!(held_locks, 0);
    Ok(())
}

#[tokio::test]
async fn session_pool_two_connections_concurrent_real_password_login() -> Result<()> {
    let independent = isolated_pool().await?;
    let f = fixture(&independent, "two-concurrent", 2).await?;
    let (a, b) = tokio::join!(
        login(
            f.app.clone(),
            f.account.password_login_payload("WEB", "QA-two-A")
        ),
        login(
            f.app.clone(),
            f.account.password_login_payload("WEB", "QA-two-B")
        )
    );
    let (a, b) = (a?, b?);
    for result in [&a, &b] {
        if result.0 == StatusCode::OK {
            verify_token(&independent, &f, &result.1).await?;
        }
    }
    let (total, active) = sessions(&independent, &f).await?;
    let held_locks = advisory_locks(&independent, &f).await?;
    f.pool.close().await;
    let pool_timeouts = [a.1.as_str(), b.1.as_str()]
        .map(|text| text.is_some_and(|value| value.contains("pool timed out")));
    eprintln!(
        "QA session_pool_result tenant={} case=two-concurrent statuses=[{},{}] total={total} active={active} held_locks={held_locks} pool_timeouts={pool_timeouts:?}",
        f.account.tenant,
        a.0.as_u16(),
        b.0.as_u16()
    );
    assert_eq!(
        (a.0, b.0),
        (StatusCode::OK, StatusCode::OK),
        "pool capacity must not be consumed by lock-holder/waiter guard connections"
    );
    assert_eq!((total, active), (3, 3));
    assert_eq!(held_locks, 0);
    Ok(())
}

#[tokio::test]
async fn session_pool_cancelled_login_has_no_session_advisory_lock() -> Result<()> {
    let independent = isolated_pool().await?;
    let f = fixture(&independent, "cancelled", 2).await?;
    let mut blocker = independent.begin().await?;
    let blocker_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("select tenant_id from tenants where tenant_id=$1 for update")
        .bind(f.account.tenant.0)
        .execute(&mut *blocker)
        .await?;
    let task = tokio::spawn(login(
        f.app.clone(),
        f.account.password_login_payload("WEB", "QA-cancelled"),
    ));
    let mut waiting = false;
    for _ in 0..100 {
        waiting = sqlx::query_scalar("select exists(select 1 from pg_stat_activity where application_name=$1 and $2=any(pg_blocking_pids(pid)))")
            .bind(&f.tag).bind(blocker_pid).fetch_one(&independent).await?;
        if waiting {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    if !waiting {
        task.abort();
        let _ = task.await;
        blocker.rollback().await?;
        f.pool.close().await;
        return Err(anyhow!("actual owned tenant-lock wait was not observed"));
    }
    task.abort();
    let cancelled = task.await.err().is_some_and(|error| error.is_cancelled());
    blocker.rollback().await?;
    for _ in 0..100 {
        if f.pool.num_idle() == f.pool.size() as usize {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let idle = f.pool.num_idle();
    let size = f.pool.size();
    let held_locks: i64 = sqlx::query_scalar("select count(*) from pg_locks l join pg_stat_activity a on a.pid=l.pid where a.application_name=$1 and l.locktype='advisory' and l.granted")
        .bind(&f.tag).fetch_one(&independent).await?;
    let (total, active) = sessions(&independent, &f).await?;
    f.pool.close().await;
    eprintln!(
        "QA session_pool_result tenant={} case=cancelled statuses=[] total={total} active={active} held_locks={held_locks} actual_wait={waiting} cancelled={cancelled} idle={idle} size={size}",
        f.account.tenant
    );
    assert!(cancelled);
    assert_eq!(
        idle, size as usize,
        "all request-owned connections return after rollback"
    );
    assert_eq!(
        held_locks, 0,
        "cancellation must not leave a session-level advisory lock on an idle pooled connection"
    );
    assert_eq!(
        (total, active),
        (1, 1),
        "the cancelled login must not create a session"
    );
    Ok(())
}

async fn lifecycle_change_while_login_waits(disable: bool) -> Result<()> {
    let independent = isolated_pool().await?;
    let case = if disable {
        "disabled-during-login"
    } else {
        "password-changed-during-login"
    };
    let f = fixture(&independent, case, 2).await?;
    let password_hash = if disable {
        None
    } else {
        Some(
            tokio::task::spawn_blocking(|| {
                use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
                let salt = SaltString::generate(&mut rand::thread_rng());
                Argon2::default()
                    .hash_password(b"ERP-owned-QA-password-changed-while-login-waits!", &salt)
                    .map(|hash| hash.to_string())
                    .map_err(|error| anyhow!("owned QA hash failed: {error}"))
            })
            .await??,
        )
    };
    let before_sessions: Value = sqlx::query_scalar(
        "select jsonb_agg(to_jsonb(s) order by session_id) from auth_sessions s where tenant_id=$1",
    )
    .bind(f.account.tenant.0)
    .fetch_one(&independent)
    .await?;
    let mut changer = independent.begin().await?;
    let changer_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *changer)
        .await?;
    let updated = RbacRepo::new(&independent)
        .update_user_in_tx(
            &mut changer,
            f.account.tenant,
            UserId(f.account.actor),
            UserUpdate {
                email: None,
                password_hash,
                status: if disable {
                    Some("DISABLED".into())
                } else {
                    None
                },
            },
        )
        .await?;
    assert!(updated.is_some());
    let epoch = AccessPlanRepo::new(&independent)
        .bump_credentials_epoch_in_tx(&mut changer, f.account.tenant)
        .await?;
    assert_eq!(epoch, 1);
    // The login lookup sees the committed old credential, then genuinely waits
    // on the same account/tenant transaction used by the shipped mutators.
    let task = tokio::spawn(login(
        f.app.clone(),
        f.account.password_login_payload("WEB", "QA-lifecycle-wait"),
    ));
    let mut waiting = false;
    for _ in 0..150 {
        waiting = sqlx::query_scalar("select exists(select 1 from pg_stat_activity where application_name=$1 and $2=any(pg_blocking_pids(pid)))")
            .bind(&f.tag).bind(changer_pid).fetch_one(&independent).await?;
        if waiting {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    if !waiting {
        task.abort();
        let _ = task.await;
        changer.rollback().await?;
        f.pool.close().await;
        return Err(anyhow!(
            "actual login/credential mutation lock wait not observed"
        ));
    }
    changer.commit().await?;
    let (status, body) = task.await??;
    let minted_current_epoch = if status == StatusCode::OK {
        let claims = f.account.jwt.decode(
            body["access_token"]
                .as_str()
                .context("server token missing")?,
        )?;
        assert_eq!(claims.sub, f.account.actor.to_string());
        assert_eq!(claims.tenant_id, f.account.tenant.to_string());
        assert!(!claims.is_admin);
        claims.cep == Some(1)
    } else {
        false
    };
    let (total, active) = sessions(&independent, &f).await?;
    let after_sessions: Value = sqlx::query_scalar(
        "select jsonb_agg(to_jsonb(s) order by session_id) from auth_sessions s where tenant_id=$1",
    )
    .bind(f.account.tenant.0)
    .fetch_one(&independent)
    .await?;
    let sessions_unchanged = before_sessions == after_sessions;
    let held_locks = advisory_locks(&independent, &f).await?;
    f.pool.close().await;
    eprintln!(
        "QA session_pool_result tenant={} case={case} statuses=[{}] total={total} active={active} held_locks={held_locks} actual_wait={waiting} current_epoch={epoch} minted_current_epoch={minted_current_epoch} sessions_unchanged={sessions_unchanged}",
        f.account.tenant,
        status.as_u16()
    );
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "credentials committed before session admission must not issue a current-generation token from a stale password/status lookup"
    );
    assert!(!minted_current_epoch);
    assert_eq!((total, active), (1, 0));
    assert!(
        sessions_unchanged,
        "denied login cannot reap or reopen any stored session"
    );
    Ok(())
}

#[tokio::test]
async fn session_pool_changed_password_does_not_issue_current_epoch_from_old_lookup() -> Result<()>
{
    lifecycle_change_while_login_waits(false).await
}

#[tokio::test]
async fn session_pool_disabled_user_does_not_issue_session_from_old_lookup() -> Result<()> {
    lifecycle_change_while_login_waits(true).await
}
