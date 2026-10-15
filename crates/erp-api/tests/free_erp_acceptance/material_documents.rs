//! Real drafts preserve stock, versions and immutable original results.
use super::{
    Result,
    inventory_commands::{Fixture, fixture},
    isolated_pool,
    production_receipt_release::http,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{AppState, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub(super) async fn context(pool: &PgPool) -> Result<(Fixture, Uuid)> {
    let f = fixture(pool).await?;
    let order = Uuid::new_v4();
    sqlx::query("update tenants set name='QA-MATERIAL-DOCUMENT-DRAFT' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-MATERIAL-DRAFT','QA-P','RELEASED',10000000)")
        .bind(f.tenant.0).bind(order).execute(pool).await?;
    eprintln!("QA material_document tenant={} order={order}", f.tenant);
    Ok((f, order))
}
fn body(f: &Fixture, document: Uuid, request: &str, version: i32) -> Value {
    json!({"request_id":request,"document_id":document,"expected_version":version,"lines":[
        {"line_id":Uuid::new_v4(),"kind":"ISSUE","warehouse_id":f.warehouse,"sku_id":f.sku,"unit_code":"kg","qty":"0.000001"},
        {"line_id":Uuid::new_v4(),"kind":"SUPPLEMENT","warehouse_id":f.warehouse,"sku_id":f.sku,"unit_code":"kg","qty":"0.25","reason":"actual supplementary draft"}
    ]})
}
pub(super) async fn save(f: &Fixture, order: Uuid, input: Value) -> Result<(StatusCode, Value)> {
    http(f, &format!("work-orders/{order}/material-documents"), input).await
}
pub(super) async fn read(
    f: &Fixture,
    order: Uuid,
    document: Uuid,
    request: Option<&str>,
) -> Result<(StatusCode, Value)> {
    let query = request
        .map(|id| format!("?request_id={id}"))
        .unwrap_or_default();
    let database = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let result=build_app(AppState::new(Db::connect_lazy(&database)?,f.login.jwt.clone())).oneshot(Request::builder().method("GET")
        .uri(format!("/v1/tenants/{}/manufacturing/work-orders/{order}/material-documents/{document}{query}",f.tenant))
        .header("authorization",format!("Bearer {}",f.login.token)).body(Body::empty())?).await?;
    let status = result.status();
    let bytes = result.into_body().collect().await?.to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    Ok((status, value))
}
async fn physical(pool: &PgPool, f: &Fixture) -> Result<Value> {
    let mut result = serde_json::Map::new();
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "mfg_material_moves",
        "shared_item_costs",
        "mfg_material_issue_values",
        "mfg_material_value_allocations",
    ] {
        let value:Value=sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(t) order by to_jsonb(t)::text),'[]'::jsonb) from {table} t where tenant_id=$1"))
            .bind(f.tenant.0).fetch_one(pool).await?;
        result.insert(table.into(), value);
    }
    Ok(Value::Object(result))
}

#[tokio::test]
async fn material_document_versions_original_replay_and_current_read_preserve_stock() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order) = context(&pool).await?;
    let before = physical(&pool, &f).await?;
    let doc = Uuid::new_v4();
    let mut first = body(&f, doc, "first", 0);
    let original_line = first["lines"][0].clone();
    for _ in 2..100 {
        let mut line = original_line.clone();
        line["line_id"] = json!(Uuid::new_v4());
        first["lines"]
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("missing draft lines"))?
            .push(line);
    }

    let (status, original) = save(&f, order, first.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{original}");
    assert_eq!(original["snapshot"]["status"], "DRAFT");
    assert_eq!(
        original["snapshot"]["lines"].as_array().map(Vec::len),
        Some(100)
    );
    assert_eq!(original["snapshot"]["posted"], false);
    assert_eq!(original["snapshot"]["lines"][0]["qty_micros"], "1");
    first["request_id"] = json!("second");
    first["expected_version"] = json!(1);
    first["lines"][0]["qty"] = json!("0.5");
    let (status, second) = save(&f, order, first.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{second}");
    assert_eq!(second["snapshot"]["version_no"], 2);
    assert_eq!(
        second["snapshot"]["lines"][0]["line_id"],
        original["snapshot"]["lines"][0]["line_id"]
    );
    first["request_id"] = json!("first");
    first["expected_version"] = json!(0);
    first["lines"][0]["qty"] = json!("0.000001");
    let (status, replay) = save(&f, order, first).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["snapshot"], original["snapshot"]);
    assert_eq!(replay["replay"], true);
    assert_eq!(
        read(&f, order, doc, Some("first")).await?.1["snapshot"],
        original["snapshot"]
    );
    assert_eq!(
        read(&f, order, doc, None).await?.1["snapshot"],
        second["snapshot"]
    );
    assert_eq!(
        read(&f, Uuid::new_v4(), doc, None).await?.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(physical(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn material_document_same_version_race_has_one_revision() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = context(&pool).await?;
    let doc = Uuid::new_v4();
    let input = body(&f, doc, "first", 0);
    assert_eq!(save(&f, order, input.clone()).await?.0, StatusCode::OK);
    let before = physical(&pool, &f).await?;
    let mut a = input.clone();
    a["request_id"] = json!("writer-a");
    a["expected_version"] = json!(1);
    a["lines"][0]["qty"] = json!("1");
    let mut b = input;
    b["request_id"] = json!("writer-b");
    b["expected_version"] = json!(1);
    b["lines"][0]["qty"] = json!("2");
    let (a, b) = tokio::join!(save(&f, order, a), save(&f, order, b));
    let mut statuses = [a?.0.as_u16(), b?.0.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    let revisions:i64=sqlx::query_scalar("select count(*) from mfg_material_document_revisions where tenant_id=$1 and document_id=$2").bind(f.tenant.0).bind(doc).fetch_one(&pool).await?;
    assert_eq!(revisions, 2);
    assert_eq!(physical(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn material_document_twenty_same_commands_have_one_original_save() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = context(&pool).await?;
    let doc = Uuid::new_v4();
    let input = body(&f, doc, "same", 0);
    let before = physical(&pool, &f).await?;
    let mut jobs = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let f = Fixture {
            tenant: f.tenant,
            actor: f.actor,
            warehouse: f.warehouse,
            sku: f.sku,
            login: f.login.clone(),
        };
        let input = input.clone();
        jobs.spawn(async move { save(&f, order, input).await });
    }
    let mut original = None;
    let mut writes = 0;
    while let Some(result) = jobs.join_next().await {
        let (status, value) = result??;
        assert_eq!(status, StatusCode::OK, "{value}");
        writes += usize::from(value["replay"] == false);
        if let Some(original) = &original {
            assert_eq!(&value["snapshot"], original);
        } else {
            original = Some(value["snapshot"].clone());
        }
    }
    assert_eq!(writes, 1);
    let rows: i64 = sqlx::query_scalar("select count(*) from command_receipts where tenant_id=$1")
        .bind(f.tenant.0)
        .fetch_one(&pool)
        .await?;
    assert_eq!(rows, 1);
    assert_eq!(physical(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn material_document_invalid_lines_changed_source_and_revoked_grants_preserve_draft()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = context(&pool).await?;
    let doc = Uuid::new_v4();
    let input = body(&f, doc, "first", 0);
    assert_eq!(save(&f, order, input.clone()).await?.0, StatusCode::OK);
    for case in [
        "wrong-sku",
        "duplicate-line",
        "too-many",
        "unknown-posted",
        "bad-source",
    ] {
        let mut next = input.clone();
        next["request_id"] = json!(case);
        next["expected_version"] = json!(1);
        match case {
            "wrong-sku" => next["lines"][0]["sku_id"] = json!(Uuid::new_v4()),
            "duplicate-line" => next["lines"][1]["line_id"] = next["lines"][0]["line_id"].clone(),
            "too-many" => {
                next["lines"] = json!(
                    (0..101)
                        .map(|_| next["lines"][0].clone())
                        .collect::<Vec<_>>()
                )
            }
            "unknown-posted" => next["posted"] = json!(true),
            _ => {
                next["lines"][0]["kind"] = json!("RETURN");
                next["lines"][0]["original_issue_id"] = json!(Uuid::new_v4());
            }
        }
        assert!(!save(&f, order, next).await?.0.is_success());
    }
    sqlx::query("update mfg_work_orders set order_qty_micros=20000000 where tenant_id=$1 and work_order_id=$2").bind(f.tenant.0).bind(order).execute(&pool).await?;
    let mut next = input.clone();
    next["request_id"] = json!("changed-source");
    next["expected_version"] = json!(1);
    let (status, value) = save(&f, order, next).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(value["code"], "MATERIAL_SOURCE_CHANGED");
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.actor)
        .execute(&pool)
        .await?;
    assert_eq!(read(&f, order, doc, None).await?.0, StatusCode::NOT_FOUND);
    assert_eq!(save(&f, order, input).await?.0, StatusCode::NOT_FOUND);
    let rows: i64 = sqlx::query_scalar(
        "select count(*) from mfg_material_document_revisions where tenant_id=$1",
    )
    .bind(f.tenant.0)
    .fetch_one(&pool)
    .await?;
    assert_eq!(rows, 1);
    Ok(())
}

#[tokio::test]
async fn material_document_audit_failure_rolls_back_header_lines_revision_and_receipt() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order) = context(&pool).await?;
    let doc = Uuid::new_v4();
    let input = body(&f, doc, "audit-failure", 0);
    let suffix = Uuid::new_v4().simple().to_string();
    let function = format!("qa_material_audit_{suffix}");
    let trigger = format!("qa_material_audit_{suffix}");
    sqlx::query(&format!("create function {function}() returns trigger language plpgsql as $$ begin if NEW.tenant_id='{}'::uuid and NEW.action='manufacturing.material.document.save' then raise exception 'owned material audit fault'; end if; return NEW; end $$",f.tenant.0)).execute(&pool).await?;
    sqlx::query(&format!("create trigger {trigger} before insert on audit_logs for each row execute function {function}()" )).execute(&pool).await?;
    let protected_tables = [
        "mfg_material_documents",
        "mfg_material_document_lines",
        "mfg_material_document_revisions",
        "command_receipts",
        "operation_logs",
        "audit_logs",
    ];
    let mut before_counts = Vec::new();
    for table in protected_tables {
        let count: i64 =
            sqlx::query_scalar(&format!("select count(*) from {table} where tenant_id=$1"))
                .bind(f.tenant.0)
                .fetch_one(&pool)
                .await?;
        before_counts.push(count);
    }
    let outcome = save(&f, order, input.clone()).await;
    sqlx::query(&format!("drop trigger {trigger} on audit_logs"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop function {function}()"))
        .execute(&pool)
        .await?;
    assert_eq!(outcome?.0, StatusCode::INTERNAL_SERVER_ERROR);
    for (table, before) in protected_tables.into_iter().zip(before_counts) {
        let count: i64 =
            sqlx::query_scalar(&format!("select count(*) from {table} where tenant_id=$1"))
                .bind(f.tenant.0)
                .fetch_one(&pool)
                .await?;
        assert_eq!(count, before, "rolled back {table}");
    }
    assert_eq!(save(&f, order, input).await?.0, StatusCode::OK);
    Ok(())
}

#[tokio::test]
async fn material_document_incomplete_original_receipt_is_not_a_successful_replay() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = context(&pool).await?;
    let doc = Uuid::new_v4();
    let input = body(&f, doc, "original", 0);
    assert_eq!(save(&f, order, input.clone()).await?.0, StatusCode::OK);
    sqlx::query(
        "update command_receipts set status='RUNNING' where tenant_id=$1 and request_id='original'",
    )
    .bind(f.tenant.0)
    .execute(&pool)
    .await?;
    let (status, value) = save(&f, order, input).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(value["code"], "MATERIAL_DOCUMENT_RECONCILIATION_REQUIRED");
    assert_eq!(
        read(&f, order, doc, Some("original")).await?.0,
        StatusCode::CONFLICT
    );
    let rows: i64 = sqlx::query_scalar(
        "select count(*) from mfg_material_document_revisions where tenant_id=$1",
    )
    .bind(f.tenant.0)
    .fetch_one(&pool)
    .await?;
    assert_eq!(rows, 1);
    Ok(())
}
