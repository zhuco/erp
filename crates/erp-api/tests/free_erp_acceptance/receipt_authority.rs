//! Real account scope is checked after the actual receipt lock, before any effect.
use super::trade_facts::{Fixture, SourceFacts, fixture, seed};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::time::{Duration, Instant};
use uuid::Uuid;

async fn business(pool: &PgPool, f: &Fixture) -> Result<Value> {
    let mut result = serde_json::Map::new();
    for table in [
        "shared_receipt_quality",
        "shared_purchase_receipts",
        "shared_stock_ledgers",
        "partner_balances",
        "shared_supplier_returns",
        "shared_receipt_value_commands",
        "shared_receipt_cost_lots",
        "shared_receipt_cost_allocations",
        "command_receipts",
    ] {
        let value:Value=sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]') from {table} x where tenant_id=$1"))
            .bind(f.tenant.0).fetch_one(pool).await?;
        result.insert(table.into(), value);
    }
    let present: bool =
        sqlx::query_scalar("select to_regclass('public.shared_receipt_value_actors') is not null")
            .fetch_one(pool)
            .await?;
    if present {
        let actors:Value=sqlx::query_scalar("select coalesce(jsonb_agg(to_jsonb(x) order by request_id),'[]') from shared_receipt_value_actors x where tenant_id=$1")
            .bind(f.tenant.0).fetch_one(pool).await?;
        result.insert("shared_receipt_value_actors".into(), actors);
    }
    Ok(Value::Object(result))
}
async fn receipt(pool: &PgPool, f: &Fixture) -> Result<Uuid> {
    seed(
        pool,
        f,
        &SourceFacts {
            arrived: 2_000_000,
            released: 1_000_000,
            returned: 0,
            issued: 0,
            unit_cent: 1,
            release_costs: vec![1],
            return_cost: 0,
        },
    )
    .await
}
fn body(f: &Fixture, q: Uuid, request: &str, action: &str, warehouse: Uuid) -> Value {
    let mut value = json!({"request_id":request,"supplier_id":Uuid::new_v4(),"warehouse_id":warehouse,
        "quality_id":q,"items":[{"sku_id":f.sku,"qty":1,"price_cent":1}]});
    value[action] = json!(100_000);
    value
}
async fn three(pool: &PgPool, f: &Fixture, warehouse: Uuid) -> Result<Vec<(u16, bool)>> {
    let mut results = Vec::new();
    for action in ["release_qty", "return_qty", "issue_qty"] {
        let q = receipt(pool, f).await?;
        let before = business(pool, f).await?;
        let r = f
            .account
            .request(
                "POST",
                &format!("/v1/tenants/{}/purchase-orders", f.tenant),
                body(f, q, &format!("QA-scope-{action}"), action, warehouse),
            )
            .await?;
        let unchanged = business(pool, f).await? == before;
        eprintln!(
            "QA receipt_authority_action tenant={} case=immediate action={action} quality={q} status={} zero_effects={unchanged}",
            f.tenant,
            r.0.as_u16()
        );
        results.push((r.0.as_u16(), unchanged));
    }
    Ok(results)
}

#[tokio::test]
async fn receipt_authority_removed_warehouse_blocks_all_three_actions() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool, "authority-warehouse").await?;
    sqlx::query(
        "delete from user_warehouse_access where tenant_id=$1 and user_id=$2 and warehouse_id=$3",
    )
    .bind(f.tenant.0)
    .bind(f.account.actor)
    .bind(f.warehouse)
    .execute(&pool)
    .await?;
    let statuses = three(&pool, &f, f.warehouse).await?;
    eprintln!(
        "QA receipt_authority_result tenant={} case=warehouse statuses={statuses:?}",
        f.tenant
    );
    assert_eq!(statuses, vec![(404, true); 3]);
    Ok(())
}

#[tokio::test]
async fn receipt_authority_removed_current_role_blocks_old_token() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool, "authority-role").await?;
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in ('purchase_orders.write','PROCUREMENT.ORDER.WRITE')")
        .bind(f.tenant.0).execute(&pool).await?;
    let statuses = three(&pool, &f, f.warehouse).await?;
    eprintln!(
        "QA receipt_authority_result tenant={} case=role statuses={statuses:?}",
        f.tenant
    );
    assert_eq!(statuses, vec![(403, true); 3]);
    Ok(())
}

#[tokio::test]
async fn receipt_authority_requested_warehouse_must_match_original_source() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool, "authority-source-warehouse").await?;
    let other = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,code,name) values($1,$2,$3,'QA source mismatch')")
        .bind(f.tenant.0).bind(other).bind(other.to_string()).execute(&pool).await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.tenant.0)
    .bind(f.account.actor)
    .bind(other)
    .execute(&pool)
    .await?;
    let statuses = three(&pool, &f, other).await?;
    eprintln!(
        "QA receipt_authority_result tenant={} case=source-warehouse statuses={statuses:?} requested={other} actual={}",
        f.tenant, f.warehouse
    );
    assert_eq!(statuses, vec![(404, true); 3]);
    Ok(())
}

async fn waiting(pool: &PgPool, f: &Fixture, q: Uuid) -> Result<()> {
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        let live: bool = sqlx::query_scalar(
            "select exists(select 1 from pg_locks l join pg_stat_activity a on a.pid=l.pid
            where l.locktype='advisory' and not l.granted and l.objsubid=2
              and l.classid=hashtext($1)::oid and l.objid=hashtext($2)::oid
              and a.datname=current_database() and a.usename=current_user)",
        )
        .bind(f.tenant.0.to_string())
        .bind(format!("purchase-release:{q}"))
        .fetch_one(pool)
        .await?;
        if live {
            return Ok(());
        }
        if Instant::now() >= until {
            return Err(anyhow::anyhow!("no actual receipt writer observed waiting"));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
async fn receipt_authority_actual_waiting_writer_rechecks_all_revocations() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let mut observed = Vec::new();
    for kind in [
        "warehouse",
        "role",
        "session",
        "epoch",
        "disabled",
        "expired",
    ] {
        let f = fixture(&pool, &format!("authority-wait-{kind}")).await?;
        let q = receipt(&pool, &f).await?;
        let before = business(&independent, &f).await?;
        let mut gate = pool.begin().await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(f.tenant.0.to_string())
            .bind(format!("purchase-release:{q}"))
            .execute(&mut *gate)
            .await?;
        let client = f.account.clone();
        let path = format!("/v1/tenants/{}/purchase-orders", f.tenant);
        let value = body(&f, q, &format!("QA-wait-{kind}"), "return_qty", f.warehouse);
        let attempt = tokio::spawn(async move { client.request("POST", &path, value).await });
        let wait = waiting(&independent, &f, q).await;
        if let Err(error) = wait {
            gate.rollback().await?;
            let _ = attempt.await?;
            return Err(error);
        }
        let mutation=match kind {
            "warehouse"=>sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
                .bind(f.tenant.0).bind(f.account.actor).execute(&independent).await,
            "role"=>sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in ('purchase_orders.write','PROCUREMENT.ORDER.WRITE')")
                .bind(f.tenant.0).execute(&independent).await,
            "session"=>sqlx::query("update auth_sessions set status='REVOKED' where tenant_id=$1 and session_id=$2")
                .bind(f.tenant.0).bind(f.account.session).execute(&independent).await,
            "epoch"=>sqlx::query("update tenants set credentials_epoch=credentials_epoch+1 where tenant_id=$1")
                .bind(f.tenant.0).execute(&independent).await,
            "disabled"=>sqlx::query("update users set status='DISABLED' where tenant_id=$1 and user_id=$2")
                .bind(f.tenant.0).bind(f.account.actor).execute(&independent).await,
            _=>sqlx::query("update auth_sessions set expires_at=clock_timestamp()-interval '1 minute' where tenant_id=$1 and session_id=$2")
                .bind(f.tenant.0).bind(f.account.session).execute(&independent).await,
        };
        gate.rollback().await?;
        mutation?;
        let r = attempt.await??;
        let unchanged = business(&independent, &f).await? == before;
        let expected = match kind {
            "warehouse" => 404,
            "role" => 403,
            _ => 401,
        };
        eprintln!(
            "QA receipt_authority_result tenant={} case=wait-{kind} quality={q} writer_observed=true status={} expected={expected} zero_effects={unchanged}",
            f.tenant,
            r.0.as_u16()
        );
        observed.push((kind.to_string(), r.0.as_u16(), expected, unchanged));
    }
    assert!(
        observed
            .iter()
            .all(|(_, got, expected, unchanged)| got == expected && *unchanged),
        "{observed:?}"
    );
    Ok(())
}

#[tokio::test]
async fn receipt_authority_two_real_devices_keep_original_actor_after_revocation() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "authority-two-devices").await?;
    let other = f.account.device("WEB", "QA-receipt-device-B").await?;
    assert_ne!(other.session, f.account.session);
    let q = receipt(&pool, &f).await?;
    let path = format!("/v1/tenants/{}/purchase-orders", f.tenant);
    let value = body(&f, q, "QA-actor-original", "return_qty", f.warehouse);
    let first = f.account.request("POST", &path, value.clone()).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let before = business(&independent, &f).await?;
    sqlx::query("update auth_sessions set status='REVOKED' where tenant_id=$1 and session_id=$2")
        .bind(f.tenant.0)
        .bind(f.account.session)
        .execute(&pool)
        .await?;
    let replay = other.request("POST", &path, value.clone()).await?;
    assert_eq!(replay.0, StatusCode::OK, "{}", replay.1);
    assert_eq!(replay.1["replay"], true);
    assert_eq!(replay.1["amount"], first.1["amount"]);
    let original:(Uuid,Uuid,Uuid,i32)=sqlx::query_as("select user_id,session_id,warehouse_id,credential_epoch from shared_receipt_value_actors where tenant_id=$1 and request_id='QA-actor-original'")
        .bind(f.tenant.0).fetch_one(&independent).await?;
    assert_eq!(
        original,
        (f.account.actor, f.account.session, f.warehouse, 0)
    );
    let denied = f.account.request("POST", &path, value).await?;
    assert_eq!(denied.0, StatusCode::UNAUTHORIZED);
    assert_eq!(business(&independent, &f).await?, before);
    eprintln!(
        "QA receipt_authority_result tenant={} case=two-devices quality={q} session_a={} session_b={} replay=200 revoked=401 original_actor_preserved=true zero_replay_writes=true",
        f.tenant, f.account.session, other.session
    );
    Ok(())
}

#[tokio::test]
async fn receipt_authority_new_database_grant_works_with_old_permission_token() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool, "authority-new-grant").await?;
    sqlx::query(
        "delete from role_permissions where tenant_id=$1 and perm_code='purchase_orders.write'",
    )
    .bind(f.tenant.0)
    .execute(&pool)
    .await?;
    let old = f.account.device("WEB", "QA-before-purchase-grant").await?;
    let claims = old.jwt.decode(&old.token)?;
    assert!(
        !claims
            .perms
            .iter()
            .any(|p| p == "purchase_orders.write" || p == "PROCUREMENT.ORDER.WRITE")
    );
    sqlx::query("insert into role_permissions(tenant_id,role_id,perm_code) select tenant_id,role_id,'PROCUREMENT.ORDER.WRITE' from user_roles where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0).bind(f.account.actor).execute(&pool).await?;
    let q = receipt(&pool, &f).await?;
    let r = old
        .request(
            "POST",
            &format!("/v1/tenants/{}/purchase-orders", f.tenant),
            body(&f, q, "QA-current-new-grant", "return_qty", f.warehouse),
        )
        .await?;
    assert_eq!(r.0, StatusCode::OK, "{}", r.1);
    let recorded:String=sqlx::query_scalar("select permission_code from shared_receipt_value_actors where tenant_id=$1 and request_id='QA-current-new-grant'")
        .bind(f.tenant.0).fetch_one(&pool).await?;
    assert_eq!(recorded, "PROCUREMENT.ORDER.WRITE");
    eprintln!(
        "QA receipt_authority_result tenant={} case=new-grant quality={q} status=200 old_claim_lacks_grant=true stored_current_permission=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_authority_admin_claim_cannot_replace_sid_or_credential_epoch() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "authority-claims").await?;
    let q = receipt(&pool, &f).await?;
    let before = business(&independent, &f).await?;
    for kind in ["missing-sid", "wrong-epoch"] {
        let mut client = f.account.clone();
        let mut claims = client.jwt.decode(&client.token)?;
        claims.is_admin = true;
        if kind == "missing-sid" {
            claims.sid = None;
            claims.cep = None;
        } else {
            claims.cep = Some(1);
        }
        client.token = client.jwt.encode(&claims)?;
        let r = client
            .request(
                "POST",
                &format!("/v1/tenants/{}/purchase-orders", f.tenant),
                body(
                    &f,
                    q,
                    &format!("QA-admin-{kind}"),
                    "return_qty",
                    f.warehouse,
                ),
            )
            .await?;
        assert_eq!(r.0, StatusCode::UNAUTHORIZED, "{}", r.1);
    }
    assert_eq!(business(&independent, &f).await?, before);
    eprintln!(
        "QA receipt_authority_result tenant={} case=claims quality={q} missing_sid=401 wrong_epoch=401 admin_bypass=false zero_effects=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_authority_expiry_after_effect_sql_rolls_back_all_tables() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "authority-late-expiry").await?;
    let q = receipt(&pool, &f).await?;
    let name = format!("qa_receipt_expiry_{}", Uuid::new_v4().simple());
    let delay_key = format!("actor-expiry:{q}");
    sqlx::query(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then perform pg_advisory_xact_lock(hashtext(new.tenant_id::text),hashtext('{delay_key}')); perform pg_sleep(2); end if; return new; end $$",f.tenant))
        .execute(&pool).await?;
    sqlx::query(&format!("create trigger {name} before insert on shared_receipt_value_actors for each row execute function {name}()"))
        .execute(&pool).await?;
    let before = business(&independent, &f).await?;
    sqlx::query("update auth_sessions set expires_at=clock_timestamp()+interval '1.5 seconds' where tenant_id=$1 and session_id=$2")
        .bind(f.tenant.0).bind(f.account.session).execute(&pool).await?;
    let client = f.account.clone();
    let path = format!("/v1/tenants/{}/purchase-orders", f.tenant);
    let value = body(&f, q, "QA-commit-expired", "return_qty", f.warehouse);
    let attempt = tokio::spawn(async move { client.request("POST", &path, value).await });
    let until = Instant::now() + Duration::from_secs(15);
    let mut observed = false;
    while Instant::now() < until {
        observed=sqlx::query_scalar("select exists(select 1 from pg_locks l join pg_stat_activity a on a.pid=l.pid
          where l.locktype='advisory' and l.granted and l.objsubid=2 and l.classid=hashtext($1)::oid
            and l.objid=hashtext($2)::oid and a.datname=current_database() and a.usename=current_user)")
            .bind(f.tenant.0.to_string()).bind(&delay_key).fetch_one(&independent).await?;
        if observed || attempt.is_finished() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let result = attempt.await?;
    let drop_trigger = sqlx::query(&format!(
        "drop trigger {name} on shared_receipt_value_actors"
    ))
    .execute(&pool)
    .await;
    let drop_function = sqlx::query(&format!("drop function {name}()"))
        .execute(&pool)
        .await;
    drop_trigger?;
    drop_function?;
    assert!(
        observed,
        "expiry must be observed after the actual stock/AP/cost SQL, not at request entry"
    );
    let r = result?;
    assert_eq!(r.0, StatusCode::UNAUTHORIZED, "{}", r.1);
    assert_eq!(business(&independent, &f).await?, before);
    eprintln!(
        "QA receipt_authority_result tenant={} case=late-expiry quality={q} delayed_actor_insert=true status=401 zero_effects=true",
        f.tenant
    );
    Ok(())
}
