//! Real OMS effects, mandatory records and current authority must share a transaction.
use super::fulfillment_sources::{Fulfillment, allocated, facts, fixture, request};
use super::{Result, isolated_pool};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn allocation_initial_and_replay_timestamps_are_the_persisted_row() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let suffix = format!("orders/{}/allocate", f.order);
    let first = request(&f, suffix.clone(), json!({})).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let shipment = &first.1["shipments"][0];
    let shipment_id = Uuid::parse_str(shipment["shipment_id"].as_str().unwrap_or(""))?;
    let stored: (chrono::DateTime<Utc>, chrono::DateTime<Utc>) = sqlx::query_as(
        "select created_at,updated_at from shipments where tenant_id=$1 and shipment_id=$2",
    )
    .bind(f.f.tenant.0)
    .bind(shipment_id)
    .fetch_one(&independent)
    .await?;
    let created: chrono::DateTime<Utc> = serde_json::from_value(shipment["created_at"].clone())?;
    let updated: chrono::DateTime<Utc> = serde_json::from_value(shipment["updated_at"].clone())?;
    assert_eq!(
        created, stored.0,
        "initial created_at must be database fact"
    );
    assert_eq!(
        updated, stored.1,
        "initial updated_at must be database fact"
    );
    let replay = request(&f, suffix, json!({})).await?;
    assert_eq!(replay.0, StatusCode::OK, "{}", replay.1);
    assert_eq!(replay.1["shipments"], first.1["shipments"]);
    let count: i64 =
        sqlx::query_scalar("select count(*) from shipments where tenant_id=$1 and order_id=$2")
            .bind(f.f.tenant.0)
            .bind(f.order)
            .fetch_one(&independent)
            .await?;
    assert_eq!(count, 1);
    eprintln!(
        "QA allocation_time_fact tenant={} shipment={shipment_id} created={created} updated={updated}",
        f.f.tenant
    );
    Ok(())
}

async fn wait_for_guard(pool: &PgPool, tenant: Uuid, key: String) -> Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar("select exists(select 1 from pg_locks where locktype='advisory' and not granted
                and classid::bigint=(hashtext($1)::bigint & 4294967295) and objid::bigint=(hashtext($2)::bigint & 4294967295) and objsubid=2)")
                .bind(tenant.to_string()).bind(&key).fetch_one(pool).await?;
            if waiting { return Ok::<(), anyhow::Error>(()); }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await??;
    Ok(())
}

#[tokio::test]
async fn fulfillment_atomic_rechecks_warehouse_after_order_lock_wait() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(fixture(&pool).await?);
    let shipment = allocated(&f).await?;
    let key = format!("fulfillment-order:{}", f.order);
    let mut blocker = pool.begin().await?;
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
        .bind(f.f.tenant.to_string())
        .bind(&key)
        .execute(&mut *blocker)
        .await?;
    let owned = Arc::clone(&f);
    let task = tokio::spawn(async move {
        request(&owned, format!("shipments/{shipment}/ship"), json!({})).await
    });
    let observed = wait_for_guard(&independent, f.f.tenant.0, key).await;
    let revoke = if observed.is_ok() {
        restrict_to_other_warehouse(&pool, &f).await
    } else {
        Ok(())
    };
    let before = complete_facts(&independent, &f).await?;
    blocker.rollback().await?;
    let (status, body) = task.await??;
    observed?;
    revoke?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "scope changed after middleware but before order admission: {body}"
    );
    assert_eq!(complete_facts(&independent, &f).await?, before);
    eprintln!(
        "QA fulfillment authority_race tenant={} case=revoke_before_admission status={} unchanged=true",
        f.f.tenant,
        status.as_u16()
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_atomic_scope_replacement_waits_for_authorized_commit() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = Arc::new(fixture(&pool).await?);
    let shipment = allocated(&f).await?;
    let other = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'QA serialized warehouse',$3)")
        .bind(f.f.tenant.0).bind(other).bind(other.to_string()).execute(&pool).await?;
    let key = format!("order-reservation-root:{}", f.root);
    let mut blocker = pool.begin().await?;
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
        .bind(f.f.tenant.to_string())
        .bind(&key)
        .execute(&mut *blocker)
        .await?;
    let owned = Arc::clone(&f);
    let shipping = tokio::spawn(async move {
        request(&owned, format!("shipments/{shipment}/ship"), json!({})).await
    });
    let observed = wait_for_guard(&independent, f.f.tenant.0, key).await;
    if let Err(error) = observed {
        blocker.rollback().await?;
        shipping.await??;
        return Err(error);
    }
    let writer_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)
        .await?;
    let writer_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&writer_pool)
        .await?;
    let tenant = f.f.tenant;
    let actor = erp_types::UserId(f.f.actor);
    let writer = tokio::spawn(async move {
        erp_repo::data_scopes::DataScopesRepo::new(&writer_pool)
            .replace_user_warehouses(tenant, actor, &[other])
            .await
    });
    let writer_wait = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let blocked: bool = sqlx::query_scalar("select exists(select 1 from pg_stat_activity where pid=$1
                and wait_event_type='Lock' and query like '%select user_id from users%' and cardinality(pg_blocking_pids(pid))>0)")
                .bind(writer_pid).fetch_one(&independent).await?;
            if blocked { return Ok::<(), anyhow::Error>(()); }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await;
    blocker.rollback().await?;
    let (status, body) = shipping.await??;
    writer.await??;
    writer_wait??;
    assert_eq!(
        status,
        StatusCode::OK,
        "admitted transaction commits before scope replacement: {body}"
    );
    let before = complete_facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "replay after scope replacement: {body}"
    );
    assert_eq!(complete_facts(&independent, &f).await?, before);
    eprintln!(
        "QA fulfillment authority_race tenant={} case=scope_writer_waits writer_pid={writer_pid} shipping=200 replay=403 unchanged=true",
        f.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_atomic_logs_bind_authenticated_actor_and_actual_event() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = allocated(&f).await?;
    let (status, body) = api(&f, &format!("shipments/{shipment}/ship"), None, true).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let snapshot = complete_facts(&independent, &f).await?;
    let operations = snapshot["operation_logs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("operation facts"))?;
    let logs: Vec<_> = operations
        .iter()
        .filter(|row| row["action"] == "OMS.SHIPMENT_SHIPPED")
        .collect();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["actor_type"], "USER");
    assert_eq!(logs[0]["actor_user_id"], f.f.actor.to_string());
    assert_eq!(logs[0]["metadata"]["request_id"], "QA-OMS-ATOMIC-REQUEST");
    assert_eq!(logs[0]["metadata"]["trace_id"], "QA-OMS-ATOMIC-TRACE");
    let audits = snapshot["audit_logs"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("audit facts"))?;
    let logs: Vec<_> = audits
        .iter()
        .filter(|row| row["action"] == "OMS.SHIPMENT.SHIPPED")
        .collect();
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0]["user_id"], f.f.actor.to_string());
    let events = snapshot["outbox_events"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("event facts"))?;
    let events: Vec<_> = events
        .iter()
        .filter(|row| row["event_type"] == "SHIPMENT_SHIPPED")
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["payload"]["shipment_id"], shipment.to_string());
    assert_eq!(
        events[0]["payload"]["warehouse_id"],
        f.f.warehouse.to_string()
    );
    assert_eq!(events[0]["payload"]["tracking_no"], body["tracking_no"]);
    assert_eq!(events[0]["payload"]["shipped_at"], body["shipped_at"]);
    let (status, body) = api(&f, &format!("shipments/{shipment}/ship"), None, true).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(complete_facts(&independent, &f).await?, snapshot);
    Ok(())
}

#[tokio::test]
async fn fulfillment_atomic_time_event_and_first_response_cite_stored_timestamp() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = allocated(&f).await?;
    let (status, body) = request(
        &f,
        format!("shipments/{shipment}/ship"),
        json!({
            "shipped_at":"2026-10-03T15:00:00.123456789Z", "tracking_no":"QA-OMS-STORED-TIME",
        }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let stored: (chrono::DateTime<Utc>, chrono::DateTime<Utc>) = sqlx::query_as(
        "select shipped_at,updated_at from shipments where tenant_id=$1 and shipment_id=$2",
    )
    .bind(f.f.tenant.0)
    .bind(shipment)
    .fetch_one(&independent)
    .await?;
    let result_time: chrono::DateTime<Utc> = serde_json::from_value(body["shipped_at"].clone())?;
    let updated: chrono::DateTime<Utc> = serde_json::from_value(body["updated_at"].clone())?;
    let event: Value = sqlx::query_scalar(
        "select payload from outbox_events where tenant_id=$1 and event_type='SHIPMENT_SHIPPED'",
    )
    .bind(f.f.tenant.0)
    .fetch_one(&independent)
    .await?;
    let event_time: chrono::DateTime<Utc> = serde_json::from_value(event["shipped_at"].clone())?;
    eprintln!(
        "QA fulfillment persisted_time tenant={} stored={} response={} event={} updated={} response_updated={}",
        f.f.tenant, stored.0, result_time, event_time, stored.1, updated
    );
    assert_eq!(
        event_time, stored.0,
        "outbound event must cite the database fact at its stored precision"
    );
    assert_eq!(
        result_time, stored.0,
        "first response must cite the same stored fact as replay"
    );
    assert_eq!(
        updated, stored.1,
        "response update time is the persisted update, not the shipment time"
    );
    let before = complete_facts(&independent, &f).await?;
    let (status, replay) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["shipped_at"], body["shipped_at"]);
    assert_eq!(replay["updated_at"], body["updated_at"]);
    assert_eq!(complete_facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn fulfillment_atomic_time_zero_affected_status_transition_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = allocated(&f).await?;
    sqlx::query("update shipments set status_version=100 where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0)
        .bind(shipment)
        .execute(&pool)
        .await?;
    let before = complete_facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    eprintln!(
        "QA fulfillment guarded_transition tenant={} status={} unchanged={}",
        f.f.tenant,
        status.as_u16(),
        complete_facts(&independent, &f).await? == before
    );
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "shipment UPDATE rejected by its version guard: {body}"
    );
    assert_eq!(
        complete_facts(&independent, &f).await?,
        before,
        "zero affected header update cannot consume source, post COGS or emit success"
    );
    Ok(())
}

async fn complete_facts(pool: &PgPool, f: &Fulfillment) -> Result<Value> {
    let mut result = facts(pool, f).await?;
    for table in ["orders", "order_items", "order_index", "waybills"] {
        let rows: Value = sqlx::query_scalar(&format!(
            "select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from {table} x where tenant_id=$1"
        )).bind(f.f.tenant.0).fetch_one(pool).await?;
        result[table] = rows;
    }
    Ok(result)
}

async fn api(
    f: &Fulfillment,
    suffix: &str,
    url: Option<String>,
    inventory_permission: bool,
) -> Result<(StatusCode, Value)> {
    let jwt = Arc::new(JwtService::new("synthetic-fulfillment-atomic-key".into()));
    let now = Utc::now().timestamp() as usize;
    let mut permissions = vec![
        "orders.read".into(),
        "orders.write".into(),
        "shipping.write".into(),
    ];
    if inventory_permission {
        permissions.push("inventory.write".into());
    }
    let mut claims = Claims::bearer(
        f.f.actor.to_string(),
        f.f.tenant.to_string(),
        permissions,
        false,
        now,
        now + 300,
    );
    claims.sid = Some(f.session_id.to_string());
    claims.cep = Some(1);
    let token = jwt.encode(&claims)?;
    let url = url.unwrap_or(std::env::var("ERP_GRID_TEST_DATABASE_URL")?);
    assert!(super::approved_database_url(&url)?);
    let response = build_app(AppState::new(Db::connect_lazy(&url)?, jwt))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/tenants/{}/oms/{suffix}", f.f.tenant))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("x-request-id", "QA-OMS-ATOMIC-REQUEST")
                .header("x-trace-id", "QA-OMS-ATOMIC-TRACE")
                .body(Body::from(
                    json!({"tracking_no":"QA-OMS-ATOMIC-TRACKING"}).to_string(),
                ))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body: Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes)));
    eprintln!(
        "QA fulfillment_http tenant={} suffix={suffix} status={} body={body}",
        f.f.tenant,
        status.as_u16()
    );
    Ok((status, body))
}

async fn restrict_to_other_warehouse(pool: &PgPool, f: &Fulfillment) -> Result<()> {
    let other = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'QA other authorized warehouse',$3)")
        .bind(f.f.tenant.0).bind(other).bind(other.to_string()).execute(pool).await?;
    erp_repo::data_scopes::DataScopesRepo::new(pool)
        .replace_user_warehouses(f.f.tenant, erp_types::UserId(f.f.actor), &[other])
        .await?;
    Ok(())
}

async fn authority_failure(action: &str, disabled: bool, inventory_permission: bool) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = if action == "allocate" {
        None
    } else {
        Some(allocated(&f).await?)
    };
    if disabled {
        sqlx::query("update users set status='DISABLED' where tenant_id=$1 and user_id=$2")
            .bind(f.f.tenant.0)
            .bind(f.f.actor)
            .execute(&pool)
            .await?;
    } else if inventory_permission {
        restrict_to_other_warehouse(&pool, &f).await?;
    }
    let suffix = match shipment {
        Some(id) if action == "ship" => format!("shipments/{id}/ship"),
        _ => format!("orders/{}/{action}", f.order),
    };
    let before = complete_facts(&independent, &f).await?;
    let (status, body) = api(&f, &suffix, None, inventory_permission).await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "current authority denied: {body}"
    );
    assert_eq!(
        complete_facts(&independent, &f).await?,
        before,
        "unauthorized OMS must have no business, log or event effect"
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_atomic_allocation_filters_current_warehouse_authority() -> Result<()> {
    authority_failure("allocate", false, true).await
}
#[tokio::test]
async fn fulfillment_atomic_shipping_rejects_revoked_warehouse() -> Result<()> {
    authority_failure("ship", false, true).await
}
#[tokio::test]
async fn fulfillment_atomic_release_rejects_revoked_warehouse() -> Result<()> {
    authority_failure("release", false, true).await
}
#[tokio::test]
async fn fulfillment_atomic_shipping_rejects_disabled_actor() -> Result<()> {
    authority_failure("ship", true, true).await
}
#[tokio::test]
async fn fulfillment_atomic_shipping_requires_inventory_write() -> Result<()> {
    authority_failure("ship", false, false).await
}

async fn mandatory_failure(action: &str, table: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let shipment = if action == "allocate" {
        None
    } else {
        Some(allocated(&f).await?)
    };
    let schema = format!("qa_oms_atomic_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("create schema {schema}"))
        .execute(&pool)
        .await?;
    let predicate = if table == "outbox_events" {
        "event_type <> 'SHIPMENT_SHIPPED'"
    } else {
        "false"
    };
    sqlx::query(&format!("create view {schema}.{table} as select * from public.{table} where {predicate} with local check option"))
        .execute(&pool).await?;
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let separator = if url.contains('?') { '&' } else { '?' };
    let url = format!("{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog");
    let suffix = match shipment {
        Some(id) if action == "ship" => format!("shipments/{id}/ship"),
        _ => format!("orders/{}/{action}", f.order),
    };
    let before = complete_facts(&independent, &f).await?;
    let (status, body) = api(&f, &suffix, Some(url), true).await?;
    eprintln!(
        "QA fulfillment mandatory_failure tenant={} action={action} table={table} schema={schema} unchanged={}",
        f.f.tenant,
        complete_facts(&independent, &f).await? == before
    );
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "mandatory {table} unavailable: {body}"
    );
    assert!(
        body.to_string().contains("check option") && body.to_string().contains(table),
        "actual injected fault: {body}"
    );
    assert_eq!(
        complete_facts(&independent, &f).await?,
        before,
        "mandatory failure rolls back the complete OMS transaction"
    );
    let (status, body) = api(&f, &suffix, None, true).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "successful retry after proved rollback: {body}"
    );
    let original = complete_facts(&independent, &f).await?;
    let (status, body) = api(&f, &suffix, None, true).await?;
    assert_eq!(status, StatusCode::OK, "same effect retry: {body}");
    assert_eq!(
        complete_facts(&independent, &f).await?,
        original,
        "replay adds no business/log/event effect"
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_atomic_allocate_operation_failure() -> Result<()> {
    mandatory_failure("allocate", "operation_logs").await
}
#[tokio::test]
async fn fulfillment_atomic_allocate_audit_failure() -> Result<()> {
    mandatory_failure("allocate", "audit_logs").await
}
#[tokio::test]
async fn fulfillment_atomic_release_operation_failure() -> Result<()> {
    mandatory_failure("release", "operation_logs").await
}
#[tokio::test]
async fn fulfillment_atomic_release_audit_failure() -> Result<()> {
    mandatory_failure("release", "audit_logs").await
}
#[tokio::test]
async fn fulfillment_atomic_ship_operation_failure() -> Result<()> {
    mandatory_failure("ship", "operation_logs").await
}
#[tokio::test]
async fn fulfillment_atomic_ship_audit_failure() -> Result<()> {
    mandatory_failure("ship", "audit_logs").await
}
#[tokio::test]
async fn fulfillment_atomic_ship_outbox_failure() -> Result<()> {
    mandatory_failure("ship", "outbox_events").await
}

#[tokio::test]
async fn fulfillment_atomic_replays_add_no_duplicate_records() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    allocated(&f).await?;
    let before = complete_facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("orders/{}/allocate", f.order), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        complete_facts(&independent, &f).await?,
        before,
        "allocation retry must not add created-shipment or audit records"
    );
    let (status, body) = request(&f, format!("orders/{}/release", f.order), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = complete_facts(&independent, &f).await?;
    let (status, body) = request(&f, format!("orders/{}/release", f.order), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        complete_facts(&independent, &f).await?,
        before,
        "terminal release retry must not add records"
    );
    Ok(())
}
