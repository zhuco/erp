//! Explicit source adoption and complete authorized paging use real HTTP/DB facts.
use super::{
    Result,
    inventory_commands::Fixture,
    isolated_pool,
    material_documents::{context, read, save},
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

pub(super) async fn get(f: &Fixture, path: &str) -> Result<(StatusCode, Value)> {
    let database = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let response = build_app(AppState::new(
        Db::connect_lazy(&database)?,
        f.login.jwt.clone(),
    ))
    .oneshot(
        Request::builder()
            .method("GET")
            .uri(format!("/v1/tenants/{}/manufacturing/{path}", f.tenant))
            .header("authorization", format!("Bearer {}", f.login.token))
            .body(Body::empty())?,
    )
    .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA material_source_get tenant={} path={path} status={} response={body}",
        f.tenant,
        status.as_u16()
    );
    Ok((status, body))
}
fn input(f: &Fixture, doc: Uuid, request: &str) -> Value {
    json!({"request_id":request,"document_id":doc,"expected_version":0,"lines":[{"line_id":Uuid::new_v4(),"kind":"ISSUE",
        "warehouse_id":f.warehouse,"sku_id":f.sku,"unit_code":"kg","qty":"0.25"}]})
}
async fn start(pool: &PgPool) -> Result<(Fixture, Uuid, Uuid, Value)> {
    let (f, order) = context(pool).await?;
    let doc = Uuid::new_v4();
    let body = input(&f, doc, "original-save");
    let (status, result) = save(&f, order, body.clone()).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    eprintln!(
        "QA material_source tenant={} order={order} document={doc}",
        f.tenant
    );
    Ok((f, order, doc, body))
}
async fn changed(pool: &PgPool, f: &Fixture, order: Uuid, qty: i64) -> Result<()> {
    sqlx::query(
        "update mfg_work_orders set order_qty_micros=$3 where tenant_id=$1 and work_order_id=$2",
    )
    .bind(f.tenant.0)
    .bind(order)
    .bind(qty)
    .execute(pool)
    .await?;
    Ok(())
}
async fn review(f: &Fixture, order: Uuid, doc: Uuid) -> Result<Value> {
    let (status, value) = get(
        f,
        &format!("work-orders/{order}/material-documents/{doc}/source-review"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    Ok(value)
}
async fn adopt(
    f: &Fixture,
    order: Uuid,
    doc: Uuid,
    request: &str,
    version: i32,
    token: &str,
) -> Result<(StatusCode, Value)> {
    http(
        f,
        &format!("work-orders/{order}/material-documents/{doc}/source-adoptions"),
        json!({"request_id":request,"expected_version":version,"review_token":token}),
    )
    .await
}

#[tokio::test]
async fn material_source_explicit_adoption_preserves_original_and_later_edit_replay() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order, doc, mut body) = start(&pool).await?;
    changed(&pool, &f, order, 20000000).await?;
    let old = review(&f, order, doc).await?;
    assert_eq!(old["changed"], true);
    changed(&pool, &f, order, 30000000).await?;
    let (status, value) = adopt(
        &f,
        order,
        doc,
        "stale-review",
        1,
        old["review_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing token"))?,
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT, "{value}");
    assert_eq!(value["code"], "MATERIAL_SOURCE_CHANGED");
    let latest = review(&f, order, doc).await?;
    let token = latest["review_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing token"))?;
    let (status, adopted) = adopt(&f, order, doc, "adopt-source", 1, token).await?;
    assert_eq!(status, StatusCode::OK, "{adopted}");
    assert_eq!(adopted["snapshot"]["version_no"], 2);
    assert_eq!(
        adopted["snapshot"]["source"]["order_qty_micros"],
        "30000000"
    );
    assert_eq!(
        adopted["snapshot"]["source_adoption"]["previous_source"]["order_qty_micros"],
        "10000000"
    );
    assert_eq!(
        read(&f, order, doc, Some("original-save")).await?.1["snapshot"]["source"]["order_qty_micros"],
        "10000000"
    );
    assert_eq!(
        read(&f, order, doc, Some("adopt-source")).await?.1["snapshot"],
        adopted["snapshot"]
    );
    body["request_id"] = json!("later-edit");
    body["expected_version"] = json!(2);
    body["lines"][0]["qty"] = json!("0.5");
    assert_eq!(save(&f, order, body).await?.0, StatusCode::OK);
    let (status, replay) = adopt(&f, order, doc, "adopt-source", 1, token).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replay"], true);
    assert_eq!(replay["snapshot"], adopted["snapshot"]);
    assert_eq!(
        read(&f, order, doc, None).await?.1["snapshot"]["version_no"],
        3
    );
    Ok(())
}

#[tokio::test]
async fn material_source_pages_filter_complete_authorized_scope_before_pagination() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order, _, _) = start(&pool).await?;
    let other = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Source scope other',$3)")
        .bind(f.tenant.0).bind(other).bind(other.to_string()).execute(&pool).await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.tenant.0)
    .bind(f.actor)
    .bind(other)
    .execute(&pool)
    .await?;
    for index in 1..8 {
        let doc = Uuid::new_v4();
        let mut body = input(&f, doc, &format!("page-{index}"));
        if index >= 6 {
            body["lines"][0]["warehouse_id"] = json!(other);
        }
        assert_eq!(save(&f, order, body).await?.0, StatusCode::OK);
    }
    sqlx::query(
        "delete from user_warehouse_access where tenant_id=$1 and user_id=$2 and warehouse_id=$3",
    )
    .bind(f.tenant.0)
    .bind(f.actor)
    .bind(other)
    .execute(&pool)
    .await?;
    let (_, first) = get(
        &f,
        &format!("work-orders/{order}/material-documents?offset=0&limit=3&status=DRAFT"),
    )
    .await?;
    let (_, second) = get(
        &f,
        &format!("work-orders/{order}/material-documents?offset=3&limit=3&status=DRAFT"),
    )
    .await?;
    assert_eq!(first["total"], 6);
    assert_eq!(second["total"], 6);
    let ids = first["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("missing page"))?
        .iter()
        .chain(
            second["data"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("missing page"))?,
        )
        .map(|r| r["document_id"].as_str().unwrap_or_default())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), 6);
    assert_eq!(
        get(
            &f,
            &format!("work-orders/{order}/material-documents?offset=6&limit=3")
        )
        .await?
        .1["data"],
        json!([])
    );
    assert_eq!(
        get(
            &f,
            &format!("work-orders/{order}/material-documents?offset=0&limit=3&status=POSTED")
        )
        .await?
        .1["total"],
        0
    );
    assert_eq!(
        get(
            &f,
            &format!("work-orders/{order}/material-documents?offset=0&limit=2001")
        )
        .await?
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    Ok(())
}

#[tokio::test]
async fn material_source_adoption_audit_fault_rolls_back_new_version_and_original_receipt()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order, doc, _) = start(&pool).await?;
    changed(&pool, &f, order, 20000000).await?;
    let reviewed = review(&f, order, doc).await?;
    let token = reviewed["review_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing token"))?;
    let name = format!("qa_material_adopt_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("create function {name}() returns trigger language plpgsql as $$ begin if NEW.tenant_id='{}'::uuid and NEW.action='manufacturing.material.source.adopt' then raise exception 'owned source adoption fault'; end if; return NEW; end $$",f.tenant.0)).execute(&pool).await?;
    sqlx::query(&format!(
        "create trigger {name} before insert on audit_logs for each row execute function {name}()"
    ))
    .execute(&pool)
    .await?;
    let outcome = adopt(&f, order, doc, "fault-adopt", 1, token).await;
    sqlx::query(&format!("drop trigger {name} on audit_logs"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop function {name}()"))
        .execute(&pool)
        .await?;
    assert_eq!(outcome?.0, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        read(&f, order, doc, None).await?.1["snapshot"]["version_no"],
        1
    );
    let receipt: i64 = sqlx::query_scalar(
        "select count(*) from command_receipts where tenant_id=$1 and request_id='fault-adopt'",
    )
    .bind(f.tenant.0)
    .fetch_one(&pool)
    .await?;
    assert_eq!(receipt, 0);
    assert_eq!(
        adopt(&f, order, doc, "fault-adopt", 1, token).await?.0,
        StatusCode::OK
    );
    Ok(())
}

#[tokio::test]
async fn material_source_token_is_bound_to_document_version_and_current_warehouse_grants()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order, doc, _) = start(&pool).await?;
    changed(&pool, &f, order, 20000000).await?;
    let reviewed = review(&f, order, doc).await?;
    let token = reviewed["review_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing token"))?;
    let other = Uuid::new_v4();
    assert_eq!(
        save(&f, order, input(&f, other, "other-doc")).await?.0,
        StatusCode::OK
    );
    assert_eq!(
        adopt(&f, order, other, "wrong-doc-token", 1, token)
            .await?
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        adopt(&f, order, doc, "wrong-version", 2, token).await?.0,
        StatusCode::CONFLICT
    );
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.actor)
        .execute(&pool)
        .await?;
    assert_eq!(
        get(
            &f,
            &format!("work-orders/{order}/material-documents/{doc}/source-review")
        )
        .await?
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        adopt(&f, order, doc, "revoked", 1, token).await?.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(
            &f,
            &format!("work-orders/{order}/material-documents?offset=0&limit=3")
        )
        .await?
        .1["total"],
        0
    );
    Ok(())
}
