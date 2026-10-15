use super::{Result, isolated_pool, trade_facts};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

async fn setup(pool: &PgPool, case: &str) -> Result<trade_facts::Fixture> {
    let f = trade_facts::fixture(pool, case).await?;
    sqlx::query("insert into shared_partners(tenant_id,partner_id,name,customer,supplier,customer_no,term_days,status) values($1,$2,'QA trade document customer',true,false,'QA-DOC-CUSTOMER',30,'ACTIVE')").bind(f.tenant.0).bind(Uuid::new_v4()).execute(pool).await?;
    super::masters::sku(pool, f.tenant.0, Uuid::new_v4(), "QA-DOC-B").await?;
    Ok(f)
}
fn body(f: &trade_facts::Fixture, id: &str) -> Value {
    json!({"request_id":id,"document_no":"QA-DOC-001","customer_code":"QA-DOC-CUSTOMER","currency":"CNY","action":"SUBMIT","items":[
        {"line_no":3,"sku_code":"QA-TRADE-FACT","qty":"1.25","price_cent":100,"discount_cent":0,"tax_cent":5,"due_date":"2026-10-20","address":"QA-DOCK","source_kind":"RESALE","warehouse_id":f.warehouse},
        {"line_no":9,"sku_code":"QA-DOC-B","qty":"2.000001","price_cent":200,"discount_cent":0,"tax_cent":7,"due_date":"2026-10-21","address":"QA-DOCK-B","source_kind":"RESALE","warehouse_id":f.warehouse}
    ]})
}
async fn save(f: &trade_facts::Fixture, body: Value) -> Result<(StatusCode, Value)> {
    f.account
        .request(
            "POST",
            &format!("/v1/tenants/{}/trade-sales-orders", f.tenant),
            body,
        )
        .await
}
async fn get(f: &trade_facts::Fixture, order: &str, query: &str) -> Result<(StatusCode, Value)> {
    f.account
        .request(
            "GET",
            &format!("/v1/tenants/{}/trade-sales-orders/{order}{query}", f.tenant),
            json!({}),
        )
        .await
}

#[tokio::test]
async fn trade_documents_same_saved_fact_list_detail_original_and_revision() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-same-fact").await?;
    let first = save(&f, body(&f, "QA-doc-first")).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    assert_eq!(first.1["document"]["header"]["total_cent"], 537);
    assert_eq!(first.1["document"]["items"][0]["qty_micros"], "1250000");
    let order = first.1["document"]["header"]["so_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing real order"))?;
    let detail = get(&f, order, "").await?;
    assert_eq!(detail.0, StatusCode::OK);
    assert_eq!(detail.1, first.1["document"]);
    let list = f
        .account
        .request(
            "GET",
            &format!("/v1/tenants/{}/trade-sales-orders?limit=100", f.tenant),
            json!({}),
        )
        .await?;
    assert_eq!(list.0, StatusCode::OK, "{}", list.1);
    let mut list_header = detail.1["header"].clone();
    list_header
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("missing header object"))?
        .remove("authority");
    assert_eq!(list.1["items"][0], list_header);
    let mut revision = body(&f, "QA-doc-revise");
    revision["so_id"] = json!(order);
    revision["action"] = json!("REVISE");
    revision["expected_version"] = json!(1);
    revision["items"][0]["price_cent"] = json!(101);
    let second = save(&f, revision.clone()).await?;
    assert_eq!(second.0, StatusCode::OK, "{}", second.1);
    assert_eq!(second.1["document"]["header"]["version_no"], 2);
    assert_eq!(second.1["document"]["header"]["total_cent"], 538);
    assert_eq!(get(&f, order, "").await?.1, second.1["document"]);
    assert_eq!(
        get(&f, order, "?request_id=QA-doc-first").await?.1,
        first.1["document"]
    );
    let original = f
        .account
        .request(
            "GET",
            &format!(
                "/v1/tenants/{}/trade-sales-order-commands/QA-doc-first",
                f.tenant
            ),
            json!({}),
        )
        .await?;
    assert_eq!(original.0, StatusCode::OK, "{}", original.1);
    assert_eq!(original.1, first.1["document"]);
    let replay = save(&f, body(&f, "QA-doc-first")).await?;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1["document"], first.1["document"]);
    revision["request_id"] = json!("QA-doc-stale");
    assert_eq!(save(&f, revision).await?.0, StatusCode::CONFLICT);
    Ok(())
}

#[tokio::test]
async fn trade_documents_current_warehouse_role_and_session_apply_to_cache_and_reads() -> Result<()>
{
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-current-authority").await?;
    let first = save(&f, body(&f, "QA-doc-authority")).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let order = first.1["document"]["header"]["so_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing order"))?;
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.account.actor)
        .execute(&pool)
        .await?;
    assert_eq!(get(&f, order, "").await?.0, StatusCode::NOT_FOUND);
    assert_eq!(
        save(&f, body(&f, "QA-doc-authority")).await?.0,
        StatusCode::NOT_FOUND
    );
    let list = f
        .account
        .request(
            "GET",
            &format!("/v1/tenants/{}/trade-sales-orders", f.tenant),
            json!({}),
        )
        .await?;
    assert_eq!(list.1["items"], json!([]));
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(f.tenant.0)
    .bind(f.account.actor)
    .bind(f.warehouse)
    .execute(&pool)
    .await?;
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code='orders.write'")
        .bind(f.tenant.0)
        .execute(&pool)
        .await?;
    assert_eq!(
        save(&f, body(&f, "QA-doc-authority")).await?.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(get(&f, order, "").await?.0, StatusCode::OK);
    sqlx::query("update auth_sessions set status='REVOKED' where tenant_id=$1 and session_id=$2")
        .bind(f.tenant.0)
        .bind(f.account.session)
        .execute(&pool)
        .await?;
    assert_eq!(get(&f, order, "").await?.0, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn trade_documents_invalid_last_sku_rolls_back_all_document_facts() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-atomic-last-line").await?;
    let mut request = body(&f, "QA-doc-bad-sku");
    request["items"][1]["sku_code"] = json!("QA-MISSING");
    assert_eq!(save(&f, request).await?.0, StatusCode::CONFLICT);
    let counts:(i64,i64,i64,i64)=sqlx::query_as("select (select count(*) from shared_sales_revisions where tenant_id=$1),(select count(*) from trade_sales_documents where tenant_id=$1),(select count(*) from trade_sales_document_revisions where tenant_id=$1),(select count(*) from command_receipts where tenant_id=$1)").bind(f.tenant.0).fetch_one(&pool).await?;
    assert_eq!(counts, (0, 0, 0, 0));
    let valid = save(&f, body(&f, "QA-doc-bad-sku")).await?;
    assert_eq!(valid.0, StatusCode::OK, "{}", valid.1);
    Ok(())
}

#[tokio::test]
async fn trade_documents_scope_and_legacy_bypass_are_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-bypass-owner").await?;
    let first = save(&f, body(&f, "QA-doc-bypass")).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let order = first.1["document"]["header"]["so_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing order"))?;
    let other = setup(&pool, "document-bypass-other").await?;
    assert_eq!(get(&other, order, "").await?.0, StatusCode::NOT_FOUND);
    let legacy=f.account.request("POST",&format!("/v1/tenants/{}/sales-orders",f.tenant),json!({"request_id":"QA-doc-legacy-bypass","so_id":order,"sales_order_no":"QA-DOC-001","source_type":"MANUAL","revision_action":"REVISE","expected_version":1,"customer_code":"QA-DOC-CUSTOMER","due_date":"2026-10-20","address":"QA-DOCK","items":[{"sku_code":"QA-TRADE-FACT","qty":1250000,"price_cent":100}]})).await?;
    assert_eq!(legacy.0, StatusCode::CONFLICT);
    assert_eq!(get(&f, order, "").await?.1, first.1["document"]);
    Ok(())
}

#[tokio::test]
async fn trade_documents_original_command_race_has_one_header_and_log_pair() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-command-race").await?;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..10 {
        let account = f.account.clone();
        let path = format!("/v1/tenants/{}/trade-sales-orders", f.tenant);
        let request = body(&f, "QA-doc-race");
        tasks.spawn(async move { account.request("POST", &path, request).await });
    }
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result??);
    }
    assert_eq!(results.len(), 10);
    for result in &results {
        assert_eq!(result.0, StatusCode::OK, "{}", result.1);
        assert_eq!(result.1["revision_id"], results[0].1["revision_id"]);
    }
    let counts:(i64,i64,i64,i64)=sqlx::query_as("select (select count(*) from trade_sales_document_revisions where tenant_id=$1),(select count(*) from operation_logs where tenant_id=$1 and action='TRADE_SALES_SAVE'),(select count(*) from audit_logs where tenant_id=$1 and action='trade.sales.save'),(select count(*) from outbox_events where tenant_id=$1 and event_type='trade.sales.saved')").bind(f.tenant.0).fetch_one(&pool).await?;
    assert_eq!(counts, (1, 1, 1, 1));
    Ok(())
}

#[tokio::test]
async fn trade_documents_frozen_facts_and_original_command_reject_database_rewrites() -> Result<()>
{
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-immutable-source").await?;
    let saved = save(&f, body(&f, "QA-doc-immutable")).await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let revision = Uuid::parse_str(
        saved.1["revision_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing revision"))?,
    )?;
    let order = saved.1["document"]["header"]["so_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing order"))?;
    assert!(
        sqlx::query(
            "update shared_sales_lines set qty_micros=2 where tenant_id=$1 and revision_id=$2"
        )
        .bind(f.tenant.0)
        .bind(revision)
        .execute(&pool)
        .await
        .is_err()
    );
    assert!(sqlx::query("insert into shared_sales_lines(tenant_id,revision_id,line_no,item_code,qty_micros,price_cent,discount_cent,tax_cent,due_date,address,source_kind) values($1,$2,99,'QA-TRADE-FACT',1000000,100,0,0,'2026-10-20','QA-DOCK','RESALE')").bind(f.tenant.0).bind(revision).execute(&pool).await.is_err());
    assert!(sqlx::query("update command_receipts set effect_id='changed' where tenant_id=$1 and request_id='QA-doc-immutable'").bind(f.tenant.0).execute(&pool).await.is_err());
    assert!(sqlx::query("insert into shared_sales_revisions(tenant_id,revision_id,so_id,version_no,status,customer_code) values($1,$2,$3,2,'SUBMITTED','QA-DOC-CUSTOMER')").bind(f.tenant.0).bind(Uuid::new_v4()).bind(Uuid::parse_str(order)?).execute(&pool).await.is_err());
    assert_eq!(get(&f, order, "").await?.1, saved.1["document"]);
    Ok(())
}

#[tokio::test]
async fn trade_documents_revision_cannot_remove_a_line_with_delivery_history() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-revision-used-line").await?;
    let saved = save(&f, body(&f, "QA-doc-used-line")).await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    let order = saved.1["document"]["header"]["so_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing order"))?;
    // A self-owned historical planning fact exercises omission; this is not shipment acceptance.
    sqlx::query("insert into shared_sales_deliveries(tenant_id,delivery_id,so_id,line_no,order_version,qty_micros,stage) values($1,$2,$3,9,1,1,'PLANNED')").bind(f.tenant.0).bind(Uuid::new_v4()).bind(Uuid::parse_str(order)?).execute(&pool).await?;
    let mut revision = body(&f, "QA-doc-remove-used");
    revision["so_id"] = json!(order);
    revision["expected_version"] = json!(1);
    revision["action"] = json!("REVISE");
    revision["items"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("missing items"))?
        .pop();
    assert_eq!(save(&f, revision).await?.0, StatusCode::CONFLICT);
    assert_eq!(get(&f, order, "").await?.1, saved.1["document"]);
    Ok(())
}

#[tokio::test]
async fn trade_documents_rejection_envelope_binds_fields_and_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-rejection-envelope").await?;
    let mut request = body(&f, "QA-doc-rejection");
    request["so_id"] = json!(Uuid::new_v4());
    request["expected_version"] = Value::Null;
    request["customer_code"] = json!("QA-MISSING-CUSTOMER");
    let result = f
        .account
        .request(
            "POST",
            &format!(
                "/v1/tenants/{}/trade-sales-orders?rejection_envelope=true",
                f.tenant
            ),
            request.clone(),
        )
        .await?;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    assert_eq!(result.1["code"], "trade_document_rejected_before_commit");
    assert_eq!(result.1["committed"], false);
    assert_eq!(result.1["fields"], request);
    let count: i64 =
        sqlx::query_scalar("select count(*) from shared_sales_revisions where tenant_id=$1")
            .bind(f.tenant.0)
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 0);
    let receipts: i64 =
        sqlx::query_scalar("select count(*) from command_receipts where tenant_id=$1")
            .bind(f.tenant.0)
            .fetch_one(&pool)
            .await?;
    assert_eq!(receipts, 0);
    request["customer_code"] = json!("QA-DOC-CUSTOMER");
    request["request_id"] = json!("QA-doc-corrected");
    let saved = save(&f, request).await?;
    assert_eq!(saved.0, StatusCode::OK, "{}", saved.1);
    Ok(())
}

#[tokio::test]
async fn trade_documents_authorization_denial_is_not_a_rejection_envelope() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = setup(&pool, "document-rejection-authority").await?;
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code='orders.write'")
        .bind(f.tenant.0)
        .execute(&pool)
        .await?;
    let result = f
        .account
        .request(
            "POST",
            &format!(
                "/v1/tenants/{}/trade-sales-orders?rejection_envelope=true",
                f.tenant
            ),
            body(&f, "QA-doc-denied-envelope"),
        )
        .await?;
    assert_eq!(result.0, StatusCode::FORBIDDEN, "{}", result.1);
    assert!(result.1["code"].is_null());
    let count: i64 = sqlx::query_scalar("select count(*) from command_receipts where tenant_id=$1")
        .bind(f.tenant.0)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    Ok(())
}
