//! All supplied sales revision lines survive the real HTTP and database boundary.
use super::{Result, isolated_pool, trade_facts};
use axum::http::StatusCode;
use serde_json::{Value, json};
use uuid::Uuid;

fn body(request: &str) -> Value {
    json!({"request_id":request,"sales_order_no":"QA-MULTI-LINE","source_type":"MANUAL","revision_action":"SUBMIT","customer_code":"QA-CUSTOMER","due_date":"2026-10-20","address":"QA-DOCK","items":[
        {"line_no":3,"sku_code":"QA-LINE-A","qty":"1.25","price_cent":100,"tax_cent":5},
        {"line_no":9,"sku_code":"QA-LINE-B","qty":"2.000001","price_cent":200,"tax_cent":7}
    ]})
}

#[tokio::test]
async fn trade_lines_multiline_decimal_and_changed_retry() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let f = trade_facts::fixture(&pool, "sales-multiline-decimal").await?;
    let path = format!("/v1/tenants/{}/sales-orders", f.tenant);
    let first = f
        .account
        .request("POST", &path, body("QA-lines-original"))
        .await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let rows:Vec<(i32,i64,i64,i64)>=sqlx::query_as("select line_no,qty_micros,price_cent,tax_cent from shared_sales_lines where tenant_id=$1 order by line_no").bind(f.tenant.0).fetch_all(&read).await?;
    assert_eq!(rows, vec![(3, 1_250_000, 100, 5), (9, 2_000_001, 200, 7)]);
    let same = f
        .account
        .request("POST", &path, body("QA-lines-original"))
        .await?;
    assert_eq!(same.0, StatusCode::OK);
    assert_eq!(same.1["revision_id"], first.1["revision_id"]);
    assert_eq!(same.1["replay"], true);
    let mut changed = body("QA-lines-original");
    changed["items"][1]["price_cent"] = json!(201);
    let conflict = f.account.request("POST", &path, changed).await?;
    assert_eq!(conflict.0, StatusCode::CONFLICT, "{}", conflict.1);
    let rows_after:Vec<(i32,i64,i64,i64)>=sqlx::query_as("select line_no,qty_micros,price_cent,tax_cent from shared_sales_lines where tenant_id=$1 order by line_no").bind(f.tenant.0).fetch_all(&read).await?;
    assert_eq!(rows_after, rows);
    Ok(())
}

#[tokio::test]
async fn trade_lines_last_invalid_line_and_duplicate_identity_are_atomic() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = trade_facts::fixture(&pool, "sales-multiline-invalid").await?;
    let path = format!("/v1/tenants/{}/sales-orders", f.tenant);
    let mut invalid = body("QA-lines-invalid");
    invalid["items"][1]["qty"] = json!("0.0000001");
    assert_eq!(
        f.account.request("POST", &path, invalid).await?.0,
        StatusCode::BAD_REQUEST
    );
    let mut duplicate = body("QA-lines-duplicate");
    duplicate["items"][1]["line_no"] = json!(3);
    assert_eq!(
        f.account.request("POST", &path, duplicate).await?.0,
        StatusCode::BAD_REQUEST
    );
    let mut tax = body("QA-lines-header-tax");
    tax["tax_cent"] = json!(10);
    assert_eq!(
        f.account.request("POST", &path, tax).await?.0,
        StatusCode::BAD_REQUEST
    );
    let counts:(i64,i64,i64)=sqlx::query_as("select (select count(*) from shared_sales_revisions where tenant_id=$1),(select count(*) from shared_sales_lines where tenant_id=$1),(select count(*) from command_receipts where tenant_id=$1)").bind(f.tenant.0).fetch_one(&pool).await?;
    assert_eq!(counts, (0, 0, 0));
    Ok(())
}

#[tokio::test]
async fn trade_lines_same_original_command_concurrent_creates_one_revision() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = trade_facts::fixture(&pool, "sales-multiline-race").await?;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..10 {
        let account = f.account.clone();
        let path = format!("/v1/tenants/{}/sales-orders", f.tenant);
        tasks.spawn(async move { account.request("POST", &path, body("QA-lines-race")).await });
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
    let counts:(i64,i64,i64)=sqlx::query_as("select (select count(*) from shared_sales_revisions where tenant_id=$1),(select count(*) from shared_sales_lines where tenant_id=$1),(select count(*) from command_receipts where tenant_id=$1)").bind(f.tenant.0).fetch_one(&pool).await?;
    assert_eq!(counts, (1, 2, 1));
    Ok(())
}

#[tokio::test]
async fn trade_lines_legacy_numeric_revision_units_are_not_rescaled() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = trade_facts::fixture(&pool, "sales-multiline-legacy").await?;
    let mut request = body("QA-lines-legacy");
    request["items"][0]["qty"] = json!(1_250_000);
    let result = f
        .account
        .request(
            "POST",
            &format!("/v1/tenants/{}/sales-orders", f.tenant),
            request,
        )
        .await?;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    let quantities: Vec<i64> = sqlx::query_scalar(
        "select qty_micros from shared_sales_lines where tenant_id=$1 order by line_no",
    )
    .bind(f.tenant.0)
    .fetch_all(&pool)
    .await?;
    assert_eq!(quantities, vec![1_250_000, 2_000_001]);
    let id: Uuid =
        sqlx::query_scalar("select revision_id from shared_sales_revisions where tenant_id=$1")
            .bind(f.tenant.0)
            .fetch_one(&pool)
            .await?;
    assert_eq!(result.1["revision_id"], id.to_string());
    Ok(())
}
