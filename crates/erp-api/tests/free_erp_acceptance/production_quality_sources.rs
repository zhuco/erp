//! Quality HTTP freezes must cite real production sources and remain authoritative at release.
use super::{
    Result, isolated_pool,
    production_receipt_release::{Receipt, facts, http, rejected, release, setup},
};
use axum::http::StatusCode;
use erp_repo::shared_contract::SharedContractRepo;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

async fn template(r: &Receipt) -> Result<Uuid> {
    let (status, body) = http(&r.f, "quality/inspection-templates", json!({
        "request_id":Uuid::new_v4().to_string(),"template_code":"QA-SOURCE-STANDARD","kind":"FINAL",
        "lines":[{"item_code":"WIDTH","required":true,"min_qty":"1","max_qty":"2","sample_count":1}]
    })).await?;
    assert_eq!(status, StatusCode::OK, "actual quality template: {body}");
    Ok(Uuid::parse_str(
        body["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing template id"))?,
    )?)
}

async fn freeze(
    r: &Receipt,
    template: Uuid,
    kind: &str,
    source: Uuid,
    item: &str,
) -> Result<(StatusCode, Value)> {
    http(
        &r.f,
        "quality/inspection-freezes",
        json!({
            "request_id":Uuid::new_v4().to_string(),"template_id":template,"source_kind":kind,
            "source_id":source,"item_code":item,"batch_code":"QA-PRODUCTION"
        }),
    )
    .await
}

async fn invalid_freeze(
    pool: &PgPool,
    case: &str,
    kind: &str,
    missing: bool,
    item: &str,
) -> Result<()> {
    let r = setup(pool, case).await?;
    let template = template(&r).await?;
    let before = facts(pool, &r).await?;
    let source = if missing {
        Uuid::new_v4()
    } else {
        r.work_order
    };
    let (status, body) = freeze(&r, template, kind, source, item).await?;
    let after = facts(pool, &r).await?;
    eprintln!(
        "QA production_quality_source_rejected tenant={} case={case} expected=422 actual={} unchanged={}",
        r.f.tenant,
        status.as_u16(),
        before == after
    );
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{case}: {body}");
    assert_eq!(before, after, "{case}: no freeze or command can commit");
    Ok(())
}

#[tokio::test]
async fn production_quality_source_unknown_kind_has_no_effect() -> Result<()> {
    invalid_freeze(
        &isolated_pool().await?,
        "source-unknown-kind",
        "UNSUPPORTED_SOURCE",
        false,
        "QA-P",
    )
    .await
}

#[tokio::test]
async fn production_quality_source_wrong_product_has_no_effect() -> Result<()> {
    invalid_freeze(
        &isolated_pool().await?,
        "source-wrong-product",
        "WORK_ORDER",
        false,
        "OTHER-PRODUCT",
    )
    .await
}

#[tokio::test]
async fn production_quality_source_missing_receipt_has_no_effect() -> Result<()> {
    invalid_freeze(
        &isolated_pool().await?,
        "source-missing-receipt",
        "PRODUCTION_RECEIPT",
        true,
        "QA-P",
    )
    .await
}

#[tokio::test]
async fn production_quality_source_receipt_required_reading_blocks_release() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-receipt-required").await?;
    let template = template(&r).await?;
    let (status, body) = freeze(&r, template, "RECEIPT", r.receipt, "QA-P").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_quality_source_case_normalization_does_not_skip_reading() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-case-normalization").await?;
    let template = template(&r).await?;
    let (status, body) = freeze(&r, template, "work_order", r.work_order, "QA-P").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await?;
    let stored: String = sqlx::query_scalar(
        "select source_kind from shared_inspection_freezes where tenant_id=$1 and freeze_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(Uuid::parse_str(
        body["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("freeze id"))?,
    )?)
    .fetch_one(&pool)
    .await?;
    assert_eq!(stored, "WORK_ORDER");
    Ok(())
}

#[tokio::test]
async fn production_quality_source_receipt_reading_is_in_original_release_proof() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-receipt-passed").await?;
    let template = template(&r).await?;
    let (status, body) = freeze(&r, template, "PRODUCTION_RECEIPT", r.receipt, "QA-P").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let freeze_id = body["effect_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("freeze id"))?;
    let (status, body) = http(
        &r.f,
        &format!("quality/inspection-freezes/{freeze_id}/readings"),
        json!({
            "request_id":Uuid::new_v4().to_string(),"item_code":"WIDTH","reading_qty":"1.5",
            "inspector":"QA-INSPECTOR","reviewer":"QA-REVIEWER"
        }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let command = Uuid::new_v4().to_string();
    let (status, body) = release(&r, &command, r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let snapshot: Value = sqlx::query_scalar(
        "select snapshot from mfg_receipt_release_commands where tenant_id=$1 and request_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(&command)
    .fetch_one(&pool)
    .await?;
    eprintln!(
        "QA production_quality_source_proof tenant={} request_id={command} required={} frozen={} readings={}",
        r.f.tenant,
        snapshot["quality"]["required_item_count"],
        snapshot["quality"]["frozen"].as_array().map_or(0, Vec::len),
        snapshot["quality"]["readings"]
            .as_array()
            .map_or(0, Vec::len)
    );
    assert_eq!(
        snapshot["quality"]["required_item_count"], 1,
        "receipt requirement cannot be declared absent"
    );
    assert_eq!(
        snapshot["quality"]["frozen"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        snapshot["quality"]["readings"][0]["reading_micros"],
        1_500_000
    );
    Ok(())
}

#[tokio::test]
async fn production_quality_source_empty_legacy_freeze_blocks_release() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-empty-legacy-freeze").await?;
    let template = template(&r).await?;
    // An old incomplete freeze is its own new QA row; do not delete actual frozen lines.
    sqlx::query("insert into shared_inspection_freezes(tenant_id,freeze_id,template_id,version_no,source_kind,source_id,item_code,batch_code) values($1,$2,$3,1,'WORK_ORDER',$4,'QA-P','QA-PRODUCTION')")
        .bind(r.f.tenant.0).bind(Uuid::new_v4()).bind(template).bind(r.work_order).execute(&pool).await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

async fn immutable(pool: &PgPool, header: bool) -> Result<()> {
    let case = if header {
        "source-immutable-header"
    } else {
        "source-immutable-line"
    };
    let r = setup(pool, case).await?;
    let template = template(&r).await?;
    let (status, body) = freeze(&r, template, "WORK_ORDER", r.work_order, "QA-P").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let freeze_id = Uuid::parse_str(
        body["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("freeze id"))?,
    )?;
    let mut tx = pool.begin().await?;
    let sql = if header {
        "update shared_inspection_freezes set item_code='OTHER-PRODUCT' where tenant_id=$1 and freeze_id=$2"
    } else {
        "update shared_inspection_freeze_lines set max_micros=9000000 where tenant_id=$1 and freeze_id=$2"
    };
    let outcome = sqlx::query(sql)
        .bind(r.f.tenant.0)
        .bind(freeze_id)
        .execute(&mut *tx)
        .await;
    let rejected = outcome.is_err();
    tx.rollback().await?;
    eprintln!(
        "QA production_quality_source_immutable tenant={} header={header} actual_rejected={rejected}",
        r.f.tenant
    );
    assert!(
        rejected,
        "frozen requirement provenance must not be rewritten"
    );
    Ok(())
}

#[tokio::test]
async fn production_quality_source_new_frozen_header_is_immutable() -> Result<()> {
    immutable(&isolated_pool().await?, true).await
}

#[tokio::test]
async fn production_quality_source_new_frozen_line_is_immutable() -> Result<()> {
    immutable(&isolated_pool().await?, false).await
}

async fn reading(r: &Receipt, freeze: &Value) -> Result<()> {
    let freeze_id = freeze["effect_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("freeze identity missing"))?;
    let (status, body) = http(
        &r.f,
        &format!("quality/inspection-freezes/{freeze_id}/readings"),
        json!({
            "request_id":Uuid::new_v4().to_string(),"item_code":"WIDTH","reading_qty":"1.5",
            "inspector":"QA-INSPECTOR","reviewer":"QA-REVIEWER"
        }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "actual reading: {body}");
    Ok(())
}

async fn review(r: &Receipt, request: &str) -> Result<(StatusCode, Value)> {
    http(
        &r.f,
        &format!("quality/receipts/{}/releases", r.receipt),
        json!({"request_id":request}),
    )
    .await
}

async fn foreign_source(receipt: bool) -> Result<()> {
    let pool = isolated_pool().await?;
    let case = if receipt {
        "source-foreign-receipt"
    } else {
        "source-foreign-order"
    };
    let r = setup(&pool, case).await?;
    let other = setup(&pool, &format!("{case}-owned-other")).await?;
    let template = template(&r).await?;
    let before = (facts(&pool, &r).await?, facts(&pool, &other).await?);
    let source = if receipt {
        other.receipt
    } else {
        other.work_order
    };
    let kind = if receipt {
        "PRODUCTION_RECEIPT"
    } else {
        "WORK_ORDER"
    };
    let (status, body) = freeze(&r, template, kind, source, "QA-P").await?;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "foreign source: {body}"
    );
    assert_eq!(
        before,
        (facts(&pool, &r).await?, facts(&pool, &other).await?)
    );
    eprintln!(
        "QA production_quality_source_foreign tenant={} other={} kind={kind} unchanged=true",
        r.f.tenant, other.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn production_quality_source_foreign_work_order_is_rejected() -> Result<()> {
    foreign_source(false).await
}

#[tokio::test]
async fn production_quality_source_foreign_receipt_is_rejected() -> Result<()> {
    foreign_source(true).await
}

#[tokio::test]
async fn production_quality_source_receipt_batch_is_owned() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-wrong-batch").await?;
    let template = template(&r).await?;
    let before = facts(&pool, &r).await?;
    let (status, body) = http(
        &r.f,
        "quality/inspection-freezes",
        json!({"request_id":Uuid::new_v4().to_string(),
        "template_id":template,"source_kind":"PRODUCTION_RECEIPT","source_id":r.receipt,
        "item_code":"QA-P","batch_code":"OTHER-BATCH"}),
    )
    .await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(before, facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn production_quality_source_nil_identity_is_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-nil-identity").await?;
    let template = template(&r).await?;
    let before = facts(&pool, &r).await?;
    let (status, body) = freeze(&r, template, "WORK_ORDER", Uuid::nil(), "QA-P").await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(before, facts(&pool, &r).await?);
    Ok(())
}

pub(super) async fn sales(pool: &PgPool, r: &Receipt) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let request = Uuid::new_v4().to_string();
    SharedContractRepo::new(pool)
        .save_sales_revision(erp_repo::shared_contract::TradeSalesRevisionInput {
            tenant_id: r.f.tenant,
            request_id: &request,
            payload_hash: &request,
            so_id: Some(id),
            customer_code: "QA-CUSTOMER",
            expected_version: None,
            action: "SUBMIT",
            lines: &[erp_domain::trade::SalesLine {
                line_no: 1,
                item_code: "QA-P".into(),
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
    Ok(id)
}

#[tokio::test]
async fn production_quality_source_actual_sales_revision_and_item_are_supported() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-sales-passed").await?;
    let so = sales(&pool, &r).await?;
    let template = template(&r).await?;
    let (status, body) = freeze(&r, template, "sales_order", so, "QA-P").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    reading(&r, &body).await?;
    let stored:Value=sqlx::query_scalar("select to_jsonb(b) from shared_inspection_source_bindings b where tenant_id=$1 and source_id=$2")
        .bind(r.f.tenant.0).bind(so).fetch_one(&pool).await?;
    assert_eq!(stored["source_kind"], "SALES_ORDER");
    assert_eq!(stored["source_snapshot"]["facts"]["version_no"], 1);
    assert!(stored["work_order_id"].is_null());
    Ok(())
}

#[tokio::test]
async fn production_quality_source_sales_item_outside_revision_is_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-sales-wrong-item").await?;
    let so = sales(&pool, &r).await?;
    let template = template(&r).await?;
    let before = facts(&pool, &r).await?;
    let (status, body) = freeze(&r, template, "SALES_ORDER", so, "OTHER-PRODUCT").await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(before, facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn production_quality_source_foreign_sales_revision_is_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-sales-foreign").await?;
    let other = setup(&pool, "source-sales-foreign-owned-other").await?;
    let so = sales(&pool, &other).await?;
    let template = template(&r).await?;
    let before = (facts(&pool, &r).await?, facts(&pool, &other).await?);
    let (status, body) = freeze(&r, template, "SALES_ORDER", so, "QA-P").await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(
        before,
        (facts(&pool, &r).await?, facts(&pool, &other).await?)
    );
    Ok(())
}

async fn template_budget(count: usize) -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, &format!("source-template-{count}")).await?;
    let lines: Vec<Value> = (0..count)
        .map(|n| {
            json!({"item_code":format!("OPTIONAL-{n:04}"),"required":false,
        "min_qty":"1","max_qty":"2","sample_count":1})
        })
        .collect();
    let (status, body) = http(
        &r.f,
        "quality/inspection-templates",
        json!({"request_id":Uuid::new_v4().to_string(),
        "template_code":"QA-SOURCE-BUDGET","kind":"FINAL","lines":lines}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let template = Uuid::parse_str(
        body["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("template missing"))?,
    )?;
    let before = facts(&pool, &r).await?;
    let (status, body) = freeze(&r, template, "WORK_ORDER", r.work_order, "QA-P").await?;
    if count > 1000 {
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(before, facts(&pool, &r).await?);
    } else {
        assert_eq!(status, StatusCode::OK, "{body}");
        let copied: i32 = sqlx::query_scalar(
            "select line_count from shared_inspection_source_bindings where tenant_id=$1",
        )
        .bind(r.f.tenant.0)
        .fetch_one(&pool)
        .await?;
        assert_eq!(copied, count as i32);
        let request = Uuid::new_v4().to_string();
        let (status, body) = release(&r, &request, r.work_order).await?;
        assert_eq!(status, StatusCode::OK, "{body}");
        let stored:i32=sqlx::query_scalar("select jsonb_array_length(snapshot#>'{quality,frozen}') from mfg_receipt_release_commands where tenant_id=$1 and request_id=$2")
            .bind(r.f.tenant.0).bind(request).fetch_one(&pool).await?;
        assert_eq!(
            stored, count as i32,
            "all frozen items remain in the original proof"
        );
    }
    eprintln!(
        "QA production_quality_source_budget tenant={} count={count} status={}",
        r.f.tenant,
        status.as_u16()
    );
    Ok(())
}

#[tokio::test]
async fn production_quality_source_one_thousand_items_are_completely_copied() -> Result<()> {
    template_budget(1000).await
}

#[tokio::test]
async fn production_quality_source_one_thousand_and_one_items_have_no_partial_freeze() -> Result<()>
{
    template_budget(1001).await
}

#[tokio::test]
async fn production_quality_source_review_blocks_unread_receipt_standard() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-review-unread").await?;
    let template = template(&r).await?;
    let (status, body) = freeze(&r, template, "PRODUCTION_RECEIPT", r.receipt, "QA-P").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = facts(&pool, &r).await?;
    let (status, body) = review(&r, &Uuid::new_v4().to_string()).await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(before, facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn production_quality_source_review_stores_proof_keeps_hold_and_replays() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-review-passed").await?;
    let template = template(&r).await?;
    let (status, body) = freeze(&r, template, "PRODUCTION_RECEIPT", r.receipt, "QA-P").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    reading(&r, &body).await?;
    let before = facts(&pool, &r).await?;
    let request = Uuid::new_v4().to_string();
    let (status, body) = review(&r, &request).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let after = facts(&pool, &r).await?;
    for (table, rows) in before
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("complete facts missing"))?
    {
        if ![
            "mfg_quality_releases",
            "mfg_quality_release_proofs",
            "command_receipts",
        ]
        .contains(&table.as_str())
        {
            assert_eq!(&after[table], rows, "quality review must not move {table}");
        }
    }
    assert_eq!(
        after["mfg_quality_release_proofs"].as_array().map(Vec::len),
        Some(1)
    );
    let proof = &after["mfg_quality_release_proofs"][0]["snapshot"];
    assert_eq!(proof["quality"]["required_item_count"], 1);
    assert_eq!(proof["inspector"], r.f.actor.to_string());
    assert_eq!(after["mfg_receipts"][0]["status"], "HOLD");
    let (status, replay) = review(&r, &request).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replay"], true);
    assert_eq!(after, facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn production_quality_source_review_does_not_use_other_batch_requirements() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-review-other-batch").await?;
    let request = Uuid::new_v4().to_string();
    SharedContractRepo::new(&pool)
        .save_open_required_item(
            r.f.tenant,
            &request,
            &request,
            r.work_order,
            "QA-OTHER-BATCH",
            "UNREAD-OTHER",
        )
        .await??;
    let (status, body) = review(&r, &Uuid::new_v4().to_string()).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let count:i64=sqlx::query_scalar("select (snapshot#>>'{quality,required_item_count}')::bigint from mfg_quality_release_proofs where tenant_id=$1")
        .bind(r.f.tenant.0).fetch_one(&pool).await?;
    assert_eq!(
        count, 0,
        "other batch requirements are not this receipt's requirements"
    );
    Ok(())
}

async fn legacy(kind: &str, item: &str, case: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, case).await?;
    let template = template(&r).await?;
    let id = Uuid::new_v4();
    let source = if kind == "receipt" {
        r.receipt
    } else {
        r.work_order
    };
    let mut tx = pool.begin().await?;
    sqlx::query("insert into shared_inspection_freezes(tenant_id,freeze_id,template_id,version_no,source_kind,source_id,item_code,batch_code) values($1,$2,$3,1,$4,$5,$6,'QA-PRODUCTION')")
        .bind(r.f.tenant.0).bind(id).bind(template).bind(kind).bind(source).bind(item).execute(&mut *tx).await?;
    sqlx::query("insert into shared_inspection_freeze_lines(tenant_id,freeze_id,item_code,required,min_micros,max_micros,sample_count) select tenant_id,$2,item_code,required,min_micros,max_micros,sample_count from shared_inspection_template_lines where tenant_id=$1 and template_id=$3")
        .bind(r.f.tenant.0).bind(id).bind(template).execute(&mut *tx).await?;
    tx.commit().await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_quality_source_legacy_receipt_alias_does_not_bypass_required_items()
-> Result<()> {
    legacy("receipt", "QA-P", "source-legacy-receipt-alias").await
}

#[tokio::test]
async fn production_quality_source_legacy_wrong_product_requires_reconciliation() -> Result<()> {
    legacy("WORK_ORDER", "WRONG-PRODUCT", "source-legacy-wrong-product").await
}

#[tokio::test]
async fn production_quality_source_bound_reading_command_and_copy_cannot_be_changed() -> Result<()>
{
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-immutable-complete").await?;
    let template = template(&r).await?;
    let (status, body) = freeze(&r, template, "WORK_ORDER", r.work_order, "QA-P").await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    reading(&r, &body).await?;
    let freeze = Uuid::parse_str(
        body["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("freeze id"))?,
    )?;
    let before = facts(&pool, &r).await?;
    let mut rejected_count = 0;
    for sql in [
        "update shared_inspection_source_bindings set source_snapshot='{}' where tenant_id=$1 and freeze_id=$2",
        "delete from shared_inspection_source_bindings where tenant_id=$1 and freeze_id=$2",
        "update mfg_quality_items set passed=false where tenant_id=$1 and freeze_id=$2",
        "update command_receipts set effect_id='forged' where tenant_id=$1 and request_id in(select request_id from shared_inspection_source_bindings where tenant_id=$1 and freeze_id=$2)",
        "delete from shared_inspection_freeze_lines where tenant_id=$1 and freeze_id=$2",
        "insert into shared_inspection_freeze_lines(tenant_id,freeze_id,item_code,required,min_micros,max_micros,sample_count) values($1,$2,'EXTRA',false,0,1,1)",
    ] {
        let mut tx = pool.begin().await?;
        if sqlx::query(sql)
            .bind(r.f.tenant.0)
            .bind(freeze)
            .execute(&mut *tx)
            .await
            .is_err()
        {
            rejected_count += 1;
        }
        tx.rollback().await?;
    }
    assert_eq!(rejected_count, 6);
    assert_eq!(before, facts(&pool, &r).await?);
    eprintln!(
        "QA production_quality_source_immutable_all tenant={} actual_rejected={rejected_count}",
        r.f.tenant
    );
    Ok(())
}

async fn insert_fault(table: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, &format!("source-fault-{table}")).await?;
    let template = template(&r).await?;
    let name = format!("qa_inspection_{}", Uuid::new_v4().simple());
    let sequence = format!("{name}_hit");
    let trigger = format!("{name}_trigger");
    sqlx::query(&format!("create sequence {sequence}"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then perform nextval('{sequence}'); raise exception 'QA_INSPECTION_REQUIRED_INSERT_FAILURE'; end if; return new; end $$",r.f.tenant)).execute(&pool).await?;
    sqlx::query(&format!(
        "create trigger {trigger} before insert on {table} for each row execute function {name}()"
    ))
    .execute(&pool)
    .await?;
    let request = Uuid::new_v4().to_string();
    let input = json!({"request_id":request,"template_id":template,"source_kind":"WORK_ORDER",
        "source_id":r.work_order,"item_code":"QA-P","batch_code":"QA-PRODUCTION"});
    let before = facts(&pool, &r).await?;
    let outcome = if table == "mfg_quality_release_proofs" {
        review(&r, &request).await
    } else {
        http(&r.f, "quality/inspection-freezes", input.clone()).await
    };
    let marker: bool = sqlx::query_scalar(&format!("select is_called from {sequence}"))
        .fetch_one(&pool)
        .await?;
    sqlx::query(&format!("drop trigger {trigger} on {table}"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop function {name}()"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("drop sequence {sequence}"))
        .execute(&pool)
        .await?;
    let (status, body) = outcome?;
    assert!(
        marker,
        "the actual required INSERT must reach this test's trigger"
    );
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert_eq!(before, facts(&pool, &r).await?);
    eprintln!(
        "QA production_quality_source_fault tenant={} table={table} request_id={request} actual_marker={marker} unchanged=true",
        r.f.tenant
    );
    let call = || async {
        if table == "mfg_quality_release_proofs" {
            review(&r, &request).await
        } else {
            http(&r.f, "quality/inspection-freezes", input.clone()).await
        }
    };
    let (status, body) = call().await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = facts(&pool, &r).await?;
    let (status, replay) = call().await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replay"], true);
    assert_eq!(before, facts(&pool, &r).await?);
    Ok(())
}

#[tokio::test]
async fn production_quality_source_binding_insert_failure_rolls_back_all_and_original_retry_succeeds()
-> Result<()> {
    insert_fault("shared_inspection_source_bindings").await
}

#[tokio::test]
async fn production_quality_source_review_proof_insert_failure_is_atomic() -> Result<()> {
    insert_fault("mfg_quality_release_proofs").await
}

async fn source_wait(sales_source: bool) -> Result<()> {
    let pool = isolated_pool().await?;
    let case = if sales_source {
        "source-sales-lock"
    } else {
        "source-work-order-lock"
    };
    let r = Arc::new(setup(&pool, case).await?);
    let template = template(&r).await?;
    let source = if sales_source {
        sales(&pool, &r).await?
    } else {
        r.work_order
    };
    let mut gate = pool.begin().await?;
    let blocker: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await?;
    if sales_source {
        sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
            .bind(r.f.tenant.0.to_string())
            .bind(format!("sales-revision:{source}"))
            .execute(&mut *gate)
            .await?;
    } else {
        sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
            .bind(r.f.tenant.0).bind(source).execute(&mut *gate).await?;
    }
    let task_r = Arc::clone(&r);
    let kind = if sales_source {
        "SALES_ORDER"
    } else {
        "WORK_ORDER"
    };
    let task = tokio::spawn(async move { freeze(&task_r, template, kind, source, "QA-P").await });
    let waiting = super::production_receipt_release::blocker_count(&pool, &r, blocker, 1).await;
    gate.commit().await?;
    let (status, body) = task.await??;
    let waiting = waiting?;
    assert_eq!(
        waiting, 1,
        "the freeze must actually wait on its owned source"
    );
    assert_eq!(status, StatusCode::OK, "{body}");
    eprintln!(
        "QA production_quality_source_wait tenant={} kind={kind} actual_waiting={waiting}",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn production_quality_source_freeze_uses_work_order_owner_lock() -> Result<()> {
    source_wait(false).await
}

#[tokio::test]
async fn production_quality_source_freeze_uses_sales_revision_owner_lock() -> Result<()> {
    source_wait(true).await
}
