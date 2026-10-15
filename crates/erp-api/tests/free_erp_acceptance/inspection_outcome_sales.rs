//! Original sales inspection history survives a later real source revision.
use super::{
    Result,
    inspection_outcome_edges::{effect, owned_facts, scoped_http},
    inspection_outcomes::{frozen, record},
    isolated_pool,
    production_quality_sources::sales,
    production_receipt_release::http,
};
use axum::http::StatusCode;
use erp_repo::shared_contract::SharedContractRepo;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn inspection_outcome_sales_original_retry_and_history_survive_a_new_source_revision()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (r, receipt_freeze) = frozen(&pool, "outcome-sales-original-revision").await?;
    let sale = sales(&pool, &r).await?;
    let template: Uuid = sqlx::query_scalar(
        "select template_id from shared_inspection_freezes where tenant_id=$1 and freeze_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(receipt_freeze)
    .fetch_one(&pool)
    .await?;
    let (status,result)=http(&r.f,"quality/inspection-freezes",json!({
        "request_id":Uuid::new_v4().to_string(),"template_id":template,"source_kind":"SALES_ORDER",
        "source_id":sale,"item_code":"QA-P","batch_code":"QA-SALES-ORIGINAL"
    })).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let freeze = effect(&result)?;
    let request = Uuid::new_v4().to_string();
    let (status, original) = record(&r, freeze, &request, Some("1.5")).await?;
    assert_eq!(status, StatusCode::OK, "{original}");
    let item = effect(&original)?;
    let owner: Option<Uuid> = sqlx::query_scalar(
        "select work_order_id from mfg_quality_items where tenant_id=$1 and item_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(item)
    .fetch_one(&pool)
    .await?;
    assert_eq!(owner, None, "sales does not invent a work order");
    let revision_request = Uuid::new_v4().to_string();
    SharedContractRepo::new(&pool)
        .save_sales_revision(erp_repo::shared_contract::TradeSalesRevisionInput {
            tenant_id: r.f.tenant,
            request_id: &revision_request,
            payload_hash: &revision_request,
            so_id: Some(sale),
            customer_code: "QA-CUSTOMER",
            expected_version: Some(1),
            action: "REVISE",
            lines: &[erp_domain::trade::SalesLine {
                line_no: 1,
                item_code: "OTHER-PRODUCT".into(),
                qty_micros: 1_000_000,
                price_cent: 100,
                discount_cent: 0,
                tax_cent: 0,
                due_date: "2026-10-05".into(),
                address: "QA-GATE".into(),
                source: "RESALE".into(),
            }],
        })
        .await??;
    let before = owned_facts(&pool, &r).await?;
    let (status, replay) = record(&r, freeze, &request, Some("1.500000")).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replay"], true);
    assert_eq!(replay["reading"], original["reading"]);
    let (status, history) = scoped_http(
        &r,
        "GET",
        &format!("quality/inspection-freezes/{freeze}/readings?item_code=WIDTH"),
        json!({}),
        vec!["quality.read".into()],
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{history}");
    assert_eq!(history["history"][0]["reading"], original["reading"]);
    let (status,result)=http(&r.f,&format!("quality/inspection-freezes/{freeze}/readings/{item}/reviews"),
        json!({"request_id":Uuid::new_v4().to_string(),"decision":"REJECT","reason":"Product no longer belongs to the current source"})).await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
    assert_eq!(before, owned_facts(&pool, &r).await?);
    Ok(())
}
