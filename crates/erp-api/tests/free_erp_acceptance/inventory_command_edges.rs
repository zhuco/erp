//! Real HTTP, shared durable receipts and independently queried original effects.
use super::{
    Result,
    account_login::{AccountLogin, STOCK_PERMISSIONS, paid_peer},
    inventory_commands::{Fixture, fixture},
    isolated_pool,
};
use axum::{
    body::Body,
    http::{HeaderValue, Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
use tower::ServiceExt;
use uuid::Uuid;

struct Options<'a> {
    caller: Option<&'a AccountLogin>,
    headers: Vec<HeaderValue>,
    database_url: Option<String>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            caller: None,
            headers: Vec::new(),
            database_url: None,
        }
    }
}

fn movement(f: &Fixture, action: &str, id: &str, quantity: i64) -> Value {
    let mut body = json!({"warehouse_id":f.warehouse,"sku_id":f.sku,"request_id":id,
        "reason":"QA_COMMAND","reference_id":"QA-COMMAND-REF"});
    body[if action == "adjustments" {
        "quantity_delta"
    } else {
        "quantity"
    }] = json!(quantity);
    body
}

async fn api(
    f: &Fixture,
    action: &str,
    body: Value,
    options: Options<'_>,
) -> Result<(StatusCode, Value)> {
    let caller = options.caller.unwrap_or(&f.login);
    let path = format!("/v1/tenants/{}/inventory/{action}", f.tenant);
    let mut request = Request::builder()
        .method(if action.starts_with("commands/") {
            "GET"
        } else {
            "POST"
        })
        .uri(&path)
        .header("authorization", format!("Bearer {}", caller.token))
        .header("content-type", "application/json");
    for header in options.headers {
        request = request.header("Idempotency-Key", header);
    }
    let url = options
        .database_url
        .unwrap_or(std::env::var("ERP_GRID_TEST_DATABASE_URL")?);
    let response = caller
        .app(&url)?
        .oneshot(request.body(Body::from(body.to_string()))?)
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let response: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA inventory command edge HTTP tenant={} action={action} request={} status={status} body={response}",
        f.tenant, body
    );
    Ok((status, response))
}

async fn facts(pool: &PgPool, f: &Fixture) -> Result<Value> {
    Ok(sqlx::query_scalar(
        "select jsonb_build_object(
         'balances',coalesce((select jsonb_agg(to_jsonb(b) order by warehouse_id,sku_id) from inventory_balances b where tenant_id=$1),'[]'::jsonb),
         'ledgers',coalesce((select jsonb_agg(to_jsonb(l) order by ledger_id) from inventory_ledgers l where tenant_id=$1),'[]'::jsonb),
         'receipts',coalesce((select jsonb_agg(to_jsonb(c) order by request_id) from command_receipts c where tenant_id=$1),'[]'::jsonb),
         'operations',coalesce((select jsonb_agg(to_jsonb(o) order by log_id) from operation_logs o where tenant_id=$1),'[]'::jsonb),
         'audits',coalesce((select jsonb_agg(to_jsonb(a) order by audit_id) from audit_logs a where tenant_id=$1),'[]'::jsonb),
         'outbox',coalesce((select jsonb_agg(to_jsonb(o) order by outbox_id) from outbox_events o where tenant_id=$1),'[]'::jsonb),
         'activations',coalesce((select jsonb_agg(to_jsonb(a) order by warehouse_id,sku_id) from inventory_quantity_activations a where tenant_id=$1),'[]'::jsonb),
         'source_maps',coalesce((select jsonb_agg(to_jsonb(m) order by to_jsonb(m)::text) from inventory_quantity_source_maps m where tenant_id=$1),'[]'::jsonb))",
    ).bind(f.tenant.0).fetch_one(pool).await?)
}

async fn other_actor(pool: &PgPool, f: &Fixture) -> Result<AccountLogin> {
    paid_peer(pool, f.tenant, f.warehouse, STOCK_PERMISSIONS).await
}

#[tokio::test]
async fn inventory_command_v2_original_session_is_immutable() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let b = f.login.device("WEB", "QA-provenance-device-B").await?;
    let request = Uuid::new_v4().to_string();
    let original = movement(&f, "inbounds", &request, 1);
    let (status, first) = api(&f, "inbounds", original.clone(), Options::default()).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    let version: i32 = sqlx::query_scalar("select (original_result->>'version')::int from command_receipts where tenant_id=$1 and request_id=$2")
        .bind(f.tenant.0).bind(&request).fetch_one(&independent).await?;
    assert_eq!(version, 2);
    let before = facts(&independent, &f).await?;
    let refused = sqlx::query("update command_receipts set original_result=jsonb_set(original_result,'{input,session_id}',to_jsonb($3::text)) where tenant_id=$1 and request_id=$2")
        .bind(f.tenant.0).bind(&request).bind(b.session.to_string()).execute(&pool).await;
    let error = match refused {
        Ok(_) => anyhow::bail!("committed original session must be immutable"),
        Err(error) => error,
    };
    assert!(
        error.as_database_error().is_some_and(|value| value
            .message()
            .contains("committed inventory movement receipt is immutable")),
        "actual original receipt guard, not an unrelated SQL failure: {error}"
    );
    assert_eq!(facts(&independent, &f).await?, before);
    for (action, body) in [
        (format!("commands/{request}"), Value::Null),
        ("inbounds".into(), original),
    ] {
        let (status, body) = api(
            &f,
            &action,
            body,
            Options {
                caller: Some(&b),
                ..Options::default()
            },
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["effect_id"], first["effect_id"]);
        assert_eq!(body["record"], first["record"]);
        assert_eq!(body["replay"], true);
    }
    let (receipt_session, operation_session, audit_session): (String,String,String) = sqlx::query_as("select
        c.original_result->'input'->>'session_id',o.metadata->>'submitter_session_id',a.after_json->>'submitter_session_id'
        from command_receipts c join operation_logs o on o.tenant_id=c.tenant_id and o.log_id=(c.original_result->>'operation_log_id')::uuid
        join audit_logs a on a.tenant_id=c.tenant_id and a.audit_id=(c.original_result->>'audit_id')::uuid
        where c.tenant_id=$1 and c.request_id=$2")
        .bind(f.tenant.0).bind(&request).fetch_one(&independent).await?;
    assert_eq!(
        (receipt_session, operation_session, audit_session),
        (
            f.login.session.to_string(),
            f.login.session.to_string(),
            f.login.session.to_string()
        )
    );
    assert_eq!(facts(&independent, &f).await?, before);
    eprintln!(
        "QA inventory_command_v2_provenance tenant={} request_id={request} original_session={} attempted_session={} db_guard=true unchanged=true",
        f.tenant, f.login.session, b.session
    );
    Ok(())
}

#[tokio::test]
async fn inventory_command_every_kind_replays_one_original_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for action in ["inbounds", "outbounds", "adjustments"] {
        let f = fixture(&pool).await?;
        let id = Uuid::new_v4().to_string();
        let body = movement(&f, action, &id, 1);
        let (status, first) = api(&f, action, body.clone(), Options::default()).await?;
        assert_eq!(status, StatusCode::OK, "{first}");
        let before = facts(&independent, &f).await?;
        let (status, retry) = api(&f, action, body, Options::default()).await?;
        assert_eq!(status, StatusCode::OK, "{retry}");
        let (status, read) = api(
            &f,
            &format!("commands/{id}"),
            Value::Null,
            Options::default(),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{read}");
        for response in [&retry, &read] {
            assert_eq!(response["replay"], true);
            for field in [
                "request_id",
                "effect_id",
                "payload_hash",
                "operation_log_id",
                "audit_id",
                "record",
            ] {
                assert_eq!(first[field], response[field], "{action} {field}");
            }
        }
        assert_eq!(facts(&independent, &f).await?, before);
        assert_eq!(before["receipts"].as_array().map(Vec::len), Some(1));
        assert_eq!(before["ledgers"].as_array().map(Vec::len), Some(2));
        assert_eq!(before["operations"].as_array().map(Vec::len), Some(1));
        assert_eq!(before["audits"].as_array().map(Vec::len), Some(1));
    }
    Ok(())
}

#[tokio::test]
async fn inventory_command_full_payload_actor_and_kind_cannot_change() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for action in ["inbounds", "outbounds", "adjustments"] {
        let f = fixture(&pool).await?;
        let actor = other_actor(&pool, &f).await?;
        let second_warehouse = Uuid::new_v4();
        sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Second command QA',$3)")
            .bind(f.tenant.0).bind(second_warehouse).bind(second_warehouse.to_string()).execute(&pool).await?;
        sqlx::query(
            "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
        )
        .bind(f.tenant.0)
        .bind(f.actor)
        .bind(second_warehouse)
        .execute(&pool)
        .await?;
        let id = Uuid::new_v4().to_string();
        let original = movement(&f, action, &id, 1);
        let (status, body) = api(&f, action, original.clone(), Options::default()).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        let before = facts(&independent, &f).await?;
        for (field, value) in [
            (
                if action == "adjustments" {
                    "quantity_delta"
                } else {
                    "quantity"
                },
                json!(2),
            ),
            ("reason", json!("changed")),
            ("reference_id", json!("changed")),
            ("sku_id", json!(Uuid::new_v4())),
            ("warehouse_id", json!(second_warehouse)),
        ] {
            let mut changed = original.clone();
            changed[field] = value;
            let (status, body) = api(&f, action, changed, Options::default()).await?;
            assert_eq!(status, StatusCode::CONFLICT, "{field}: {body}");
            assert_eq!(body["code"], "inventory_command_identity_conflict");
        }
        let changed_action = if action == "inbounds" {
            "outbounds"
        } else {
            "inbounds"
        };
        let (status, body) = api(
            &f,
            changed_action,
            movement(&f, changed_action, &id, 1),
            Options::default(),
        )
        .await?;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        let (status, body) = api(
            &f,
            action,
            original,
            Options {
                caller: Some(&actor),
                ..Options::default()
            },
        )
        .await?;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(facts(&independent, &f).await?, before);
    }
    Ok(())
}

#[tokio::test]
async fn inventory_command_twenty_concurrent_retries_post_once_per_kind() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for action in ["inbounds", "outbounds", "adjustments"] {
        let f = Arc::new(fixture(&pool).await?);
        let id = Uuid::new_v4().to_string();
        let body = movement(&f, action, &id, 1);
        let barrier = Arc::new(Barrier::new(20));
        let mut tasks = JoinSet::new();
        for index in 0..20 {
            let f = Arc::clone(&f);
            let mut body = body.clone();
            let headers = if index % 2 == 0 {
                body.as_object_mut()
                    .ok_or_else(|| anyhow::anyhow!("object"))?
                    .remove("request_id");
                vec![HeaderValue::from_str(&id)?]
            } else {
                Vec::new()
            };
            let barrier = Arc::clone(&barrier);
            tasks.spawn(async move {
                barrier.wait().await;
                api(
                    &f,
                    action,
                    body,
                    Options {
                        headers,
                        ..Options::default()
                    },
                )
                .await
            });
        }
        let mut responses = Vec::new();
        while let Some(result) = tasks.join_next().await {
            responses.push(result??);
        }
        assert_eq!(responses.len(), 20);
        let first = &responses[0].1;
        let mut originals = 0;
        for (status, body) in &responses {
            assert_eq!(*status, StatusCode::OK, "{body}");
            if body["replay"] == false {
                originals += 1;
            }
            for field in [
                "record",
                "effect_id",
                "payload_hash",
                "operation_log_id",
                "audit_id",
            ] {
                assert_eq!(body[field], first[field]);
            }
        }
        assert_eq!(originals, 1);
        let facts = facts(&independent, &f).await?;
        assert_eq!(
            facts["balances"][0]["on_hand_micros"],
            if action == "outbounds" {
                3_900_000
            } else {
                5_900_000
            }
        );
        for field in ["receipts", "operations", "audits"] {
            assert_eq!(facts[field].as_array().map(Vec::len), Some(1));
        }
        assert_eq!(facts["ledgers"].as_array().map(Vec::len), Some(2));
        eprintln!(
            "QA inventory command concurrent tenant={} action={action} responses=20 original_effects=1",
            f.tenant
        );
    }
    // Different original commands contend for real availability, not a shared ID.
    let f = Arc::new(fixture(&pool).await?);
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let f = Arc::clone(&f);
        let barrier = Arc::clone(&barrier);
        tasks.spawn(async move {
            let body = movement(&f, "outbounds", &Uuid::new_v4().to_string(), 1);
            barrier.wait().await;
            api(&f, "outbounds", body, Options::default()).await
        });
    }
    let mut successes = 0;
    let mut conflicts = 0;
    while let Some(result) = tasks.join_next().await {
        let (status, body) = result??;
        match status {
            StatusCode::OK => {
                successes += 1;
                assert_eq!(body["replay"], false);
            }
            StatusCode::CONFLICT => {
                conflicts += 1;
                assert_eq!(body["code"], "inventory_insufficient");
            }
            _ => anyhow::bail!("unexpected contention status {status}: {body}"),
        }
    }
    assert_eq!((successes, conflicts), (3, 17));
    let facts = facts(&independent, &f).await?;
    assert_eq!(facts["balances"][0]["on_hand_micros"], 1_900_000);
    for field in ["receipts", "operations", "audits"] {
        assert_eq!(facts[field].as_array().map(Vec::len), Some(3));
    }
    assert_eq!(facts["ledgers"].as_array().map(Vec::len), Some(4));
    eprintln!(
        "QA inventory command distinct contention tenant={} requests=20 succeeded=3 conflicts=17",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inventory_command_invalid_header_or_body_never_falls_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for action in ["inbounds", "outbounds", "adjustments"] {
        let f = fixture(&pool).await?;
        let before = facts(&independent, &f).await?;
        for id in [
            String::new(),
            " ".into(),
            "bad id".into(),
            "a".repeat(129),
            "bad\nkey".into(),
        ] {
            let (status, body) =
                api(&f, action, movement(&f, action, &id, 1), Options::default()).await?;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        }
        let mut no_id = movement(&f, action, "unused", 1);
        no_id
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("object"))?
            .remove("request_id");
        for headers in [
            vec![HeaderValue::from_static(" ")],
            vec![HeaderValue::from_bytes(b"\xff")?],
            vec![
                HeaderValue::from_static("one"),
                HeaderValue::from_static("two"),
            ],
        ] {
            let (status, body) = api(
                &f,
                action,
                no_id.clone(),
                Options {
                    headers,
                    ..Options::default()
                },
            )
            .await?;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        }
        let (status, body) = api(
            &f,
            action,
            movement(&f, action, "body", 1),
            Options {
                headers: vec![HeaderValue::from_static("header")],
                ..Options::default()
            },
        )
        .await?;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(facts(&independent, &f).await?, before);
        let id = Uuid::new_v4().to_string();
        let (status, first) = api(
            &f,
            action,
            no_id.clone(),
            Options {
                headers: vec![HeaderValue::from_str(&id)?],
                ..Options::default()
            },
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{first}");
        let (status, retry) = api(
            &f,
            action,
            movement(&f, action, &id, 1),
            Options {
                headers: vec![HeaderValue::from_str(&id)?],
                ..Options::default()
            },
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{retry}");
        assert_eq!(retry["replay"], true);
        assert_eq!(retry["record"], first["record"]);
    }
    Ok(())
}

#[tokio::test]
async fn inventory_command_discarded_response_has_durable_original_readback() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let (status, _discarded) = api(
        &f,
        "inbounds",
        movement(&f, "inbounds", &id, 1),
        Options::default(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, later) = api(
        &f,
        "inbounds",
        movement(&f, "inbounds", &Uuid::new_v4().to_string(), 1),
        Options::default(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{later}");
    let before = facts(&independent, &f).await?;
    let (status, read) = api(
        &f,
        &format!("commands/{id}"),
        Value::Null,
        Options::default(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(read["record"]["quantities"]["on_hand"], "5.9");
    assert_eq!(later["record"]["quantities"]["on_hand"], "6.9");
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_command_readback_preserves_current_authorization() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let foreign = fixture(&pool).await?;
    let actor = other_actor(&pool, &f).await?;
    let write_only = paid_peer(&pool, f.tenant, f.warehouse, &["WMS.INVENTORY.WRITE"]).await?;
    let id = Uuid::new_v4().to_string();
    let original = movement(&f, "outbounds", &id, 1);
    let (status, body) = api(&f, "outbounds", original.clone(), Options::default()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = facts(&independent, &f).await?;
    for (options, expected) in [
        (
            Options {
                caller: Some(&actor),
                ..Options::default()
            },
            StatusCode::NOT_FOUND,
        ),
        (
            Options {
                caller: Some(&write_only),
                ..Options::default()
            },
            StatusCode::FORBIDDEN,
        ),
        (
            Options {
                caller: Some(&foreign.login),
                ..Options::default()
            },
            StatusCode::FORBIDDEN,
        ),
    ] {
        let (status, body) = api(&f, &format!("commands/{id}"), Value::Null, options).await?;
        assert_eq!(status, expected, "{body}");
    }
    let (status, body) = api(
        &f,
        &format!("commands/{}", Uuid::new_v4()),
        Value::Null,
        Options::default(),
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.actor)
        .execute(&pool)
        .await?;
    let (status, body) = api(
        &f,
        &format!("commands/{id}"),
        Value::Null,
        Options::default(),
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let (status, body) = api(&f, "outbounds", original, Options::default()).await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(facts(&independent, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_command_unsupported_dimensions_and_invalid_quantities_leave_no_effect()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for action in ["inbounds", "outbounds", "adjustments"] {
        let f = fixture(&pool).await?;
        let before = facts(&independent, &f).await?;
        let id = Uuid::new_v4().to_string();
        let mut body = movement(&f, action, &id, 1);
        body["batch_id"] = json!(Uuid::new_v4());
        let (status, error) = api(&f, action, body, Options::default()).await?;
        assert_eq!(status, StatusCode::NOT_FOUND, "{error}");
        assert_eq!(error["code"], "inventory_batch_not_found");
        for quantity in [0, i64::MAX] {
            let (status, error) = api(
                &f,
                action,
                movement(&f, action, &id, quantity),
                Options::default(),
            )
            .await?;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        }
        let mut body = movement(&f, action, &id, 1);
        body["unsupported_dimension"] = json!("present");
        let (status, error) = api(&f, action, body, Options::default()).await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
        assert_eq!(facts(&independent, &f).await?, before);
    }
    for action in ["inbounds", "outbounds"] {
        let f = fixture(&pool).await?;
        let before = facts(&independent, &f).await?;
        let mut body = movement(&f, action, "unused", 1);
        body.as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("object"))?
            .remove("request_id");
        body["location_id"] = json!(Uuid::new_v4());
        let (status, error) = api(&f, action, body, Options::default()).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
        assert_eq!(facts(&independent, &f).await?, before);
    }
    Ok(())
}

#[tokio::test]
async fn inventory_command_log_failures_roll_back_identity_and_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for action in ["inbounds", "outbounds", "adjustments"] {
        for table in ["operation_logs", "audit_logs"] {
            let f = fixture(&pool).await?;
            let before = facts(&independent, &f).await?;
            let schema = format!("qa_inventory_command_{}", Uuid::new_v4().simple());
            sqlx::query(&format!("create schema {schema}"))
                .execute(&pool)
                .await?;
            sqlx::query(&format!("create view {schema}.{table} as select * from public.{table} where false with local check option"))
                .execute(&pool).await?;
            let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
            let separator = if url.contains('?') { '&' } else { '?' };
            let url = format!(
                "{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog"
            );
            let id = Uuid::new_v4().to_string();
            let body = movement(&f, action, &id, 1);
            let (status, error) = api(
                &f,
                action,
                body.clone(),
                Options {
                    database_url: Some(url),
                    ..Options::default()
                },
            )
            .await?;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{error}");
            assert!(
                error.to_string().to_lowercase().contains("check option"),
                "{error}"
            );
            assert_eq!(facts(&independent, &f).await?, before);
            eprintln!(
                "QA inventory command rollback tenant={} action={action} table={table} request_id={id} schema={schema}",
                f.tenant
            );
            // The original ID is retained after a proved rollback; this is not
            // blanket permission to resend an unknown commit with another ID.
            let (status, success) = api(&f, action, body, Options::default()).await?;
            assert_eq!(status, StatusCode::OK, "{success}");
            assert_eq!(success["replay"], false);
        }
    }
    Ok(())
}

#[tokio::test]
async fn inventory_command_committed_snapshot_rejects_update_and_delete() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4().to_string();
    let (status, body) = api(
        &f,
        "inbounds",
        movement(&f, "inbounds", &id, 1),
        Options::default(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = facts(&independent, &f).await?;
    for sql in [
        "update command_receipts set payload_hash='changed' where tenant_id=$1 and request_id=$2",
        "delete from command_receipts where tenant_id=$1 and request_id=$2",
    ] {
        let error = sqlx::query(sql)
            .bind(f.tenant.0)
            .bind(&id)
            .execute(&pool)
            .await
            .expect_err("committed original result must be immutable");
        assert!(
            error.to_string().contains("receipt is immutable"),
            "{error}"
        );
    }
    assert_eq!(facts(&independent, &f).await?, before);
    let (status, read) = api(
        &f,
        &format!("commands/{id}"),
        Value::Null,
        Options::default(),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(read["record"], body["record"]);
    Ok(())
}

#[tokio::test]
async fn inventory_command_incomplete_corrupt_and_legacy_receipts_do_not_reexecute() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool).await?;
    for case in ["incomplete", "corrupt", "legacy"] {
        let id = format!("{case}-{}", Uuid::new_v4());
        if case == "legacy" {
            sqlx::query("insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id) values($1,$2,'unknown-legacy','SUCCEEDED','unknown-legacy')")
                .bind(f.tenant.0).bind(&id).execute(&pool).await?;
        } else {
            sqlx::query("insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id,movement_kind,actor_user_id,warehouse_id,sku_id,original_result)
                values($1,$2,'invalid-fixture',$3,'unknown','INBOUND',$4,$5,$6,$7)")
                .bind(f.tenant.0).bind(&id).bind(if case=="corrupt" {"SUCCEEDED"} else {"RUNNING"})
                .bind(f.actor).bind(f.warehouse).bind(f.sku)
                .bind(if case=="corrupt" {Some(json!({"version":1}))} else {None})
                .execute(&pool).await?;
        }
        let before = facts(&independent, &f).await?;
        let (status, body) = api(
            &f,
            "inbounds",
            movement(&f, "inbounds", &id, 1),
            Options::default(),
        )
        .await?;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        let (status, body) = api(
            &f,
            &format!("commands/{id}"),
            Value::Null,
            Options::default(),
        )
        .await?;
        assert_eq!(
            status,
            if case == "legacy" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::CONFLICT
            },
            "{body}"
        );
        if case != "legacy" {
            assert_eq!(body["code"], "inventory_command_reconciliation_required");
        }
        assert_eq!(facts(&independent, &f).await?, before);
    }
    Ok(())
}
