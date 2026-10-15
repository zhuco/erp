//! Ordinary password accounts submit arrivals against real purchase revisions.
use super::trade_facts::{Fixture, fixture};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

async fn revision(f: &Fixture, po: Uuid, action: &str, price: i64) -> Result<()> {
    let (status, body) = f.account.request("POST", &format!("/v1/tenants/{}/purchase-orders", f.tenant),
        json!({"request_id":format!("QA-arrival-{action}-{price}"), "supplier_id":Uuid::new_v4(),
            // The current shared revision branch consumes millionths in items[].qty.
            "warehouse_id":f.warehouse,"items":[{"sku_id":f.sku,"qty":20_000_000,"price_cent":price}],
            "revision_action":action,"po_id":po,"partner_code":"QA-ARRIVAL-SUPPLIER","due_date":"2030-01-01"})).await?;
    assert_eq!(status, StatusCode::OK, "purchase revision {action}: {body}");
    Ok(())
}

async fn arrival(
    f: &Fixture,
    po: Uuid,
    request: &str,
    qty: i64,
    price: Option<i64>,
) -> Result<(StatusCode, Value)> {
    let mut body = json!({"request_id":request,"supplier_id":Uuid::new_v4(),"warehouse_id":f.warehouse,
        "items":[{"sku_id":f.sku,"qty":1,"price_cent":200}],"po_id":po,"line_no":1,
        "location_id":f.location,"batch_code":"QA-ARRIVAL-BATCH","arrival_qty":qty});
    if let Some(price) = price {
        body["unit_cost_cent"] = json!(price);
    }
    f.account
        .request(
            "POST",
            &format!("/v1/tenants/{}/purchase-orders", f.tenant),
            body,
        )
        .await
}

async fn facts(pool: &PgPool, f: &Fixture) -> Result<Value> {
    Ok(sqlx::query_scalar("select jsonb_build_object(
        'quality',(select coalesce(jsonb_agg(to_jsonb(x) order by quality_id),'[]') from shared_receipt_quality x where tenant_id=$1),
        'receipts',(select coalesce(jsonb_agg(to_jsonb(x) order by receipt_id),'[]') from shared_purchase_receipts x where tenant_id=$1),
        'stock',(select coalesce(jsonb_agg(to_jsonb(x) order by ledger_id),'[]') from shared_stock_ledgers x where tenant_id=$1),
        'commands',(select coalesce(jsonb_agg(to_jsonb(x) order by request_id),'[]') from command_receipts x where tenant_id=$1))")
        .bind(f.tenant.0).fetch_one(pool).await?)
}

async fn observe(pool: &PgPool, f: &Fixture, case: &str, status: StatusCode) -> Result<Value> {
    let actual = facts(pool, f).await?;
    let rows = actual["quality"].as_array().expect("quality rows");
    eprintln!(
        "QA purchase_arrival tenant={} case={case} status={} qualities={} quantity={} unit_prices={}",
        f.tenant,
        status.as_u16(),
        rows.len(),
        rows.iter()
            .map(|r| r["arrived_micros"].as_i64().expect("stored qty"))
            .sum::<i64>(),
        json!(
            rows.iter()
                .map(|r| r["unit_cost_cent"].clone())
                .collect::<Vec<_>>()
        )
    );
    Ok(actual)
}

async fn reject_price(case: &str, price: Option<i64>, expected: StatusCode) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, case).await?;
    let po = Uuid::new_v4();
    revision(&f, po, "OPEN", 200).await?;
    let before = facts(&independent, &f).await?;
    let result = arrival(&f, po, "QA-arrival-refused", 1_000_000, price).await?;
    let after = observe(&independent, &f, case, result.0).await?;
    assert_eq!(result.0, expected, "{}", result.1);
    assert_eq!(
        after, before,
        "refused price must not receive stock or store an arrival command"
    );
    Ok(())
}

#[tokio::test]
async fn purchase_arrival_missing_price_is_not_a_zero_cost_declaration() -> Result<()> {
    reject_price(
        "arrival-missing-price",
        None,
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await
}

#[tokio::test]
async fn purchase_arrival_zero_cannot_replace_a_nonzero_source_price() -> Result<()> {
    reject_price("arrival-forged-zero", Some(0), StatusCode::CONFLICT).await
}

#[tokio::test]
async fn purchase_arrival_nonzero_price_must_match_its_source() -> Result<()> {
    reject_price("arrival-forged-price", Some(999), StatusCode::CONFLICT).await
}

#[tokio::test]
async fn purchase_arrival_explicit_zero_with_a_zero_price_source_is_valid() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "arrival-known-zero").await?;
    let po = Uuid::new_v4();
    revision(&f, po, "OPEN", 0).await?;
    let result = arrival(&f, po, "QA-arrival-zero", 1_000_000, Some(0)).await?;
    let after = observe(&independent, &f, "arrival-known-zero", result.0).await?;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    assert_eq!(after["quality"][0]["unit_cost_cent"], 0);
    assert_eq!(after["quality"][0]["arrived_micros"], 1_000_000);
    assert_eq!(after["quality"][0]["released_micros"], 0);
    Ok(())
}

#[tokio::test]
async fn purchase_arrival_latest_closed_revision_cannot_fall_back_to_an_open_one() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "arrival-closed-source").await?;
    let po = Uuid::new_v4();
    revision(&f, po, "OPEN", 200).await?;
    revision(&f, po, "CLOSE", 200).await?;
    let before = facts(&independent, &f).await?;
    let result = arrival(&f, po, "QA-arrival-closed", 1_000_000, Some(200)).await?;
    let after = observe(&independent, &f, "arrival-closed-source", result.0).await?;
    assert_eq!(result.0, StatusCode::CONFLICT, "{}", result.1);
    assert_eq!(after, before);
    Ok(())
}

#[tokio::test]
async fn purchase_arrival_same_command_cannot_change_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "arrival-changed-command").await?;
    let po = Uuid::new_v4();
    revision(&f, po, "OPEN", 200).await?;
    let first = arrival(&f, po, "QA-arrival-same", 1_000_000, Some(200)).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let before = facts(&independent, &f).await?;
    let result = arrival(&f, po, "QA-arrival-same", 2_000_000, Some(200)).await?;
    let after = observe(&independent, &f, "arrival-changed-command", result.0).await?;
    assert_eq!(result.0, StatusCode::CONFLICT, "{}", result.1);
    assert_eq!(after, before);
    Ok(())
}

#[tokio::test]
async fn purchase_arrival_replay_after_source_closure_keeps_original_result() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "arrival-original-replay").await?;
    let po = Uuid::new_v4();
    revision(&f, po, "OPEN", 200).await?;
    let first = arrival(&f, po, "QA-arrival-replay", 1_000_000, Some(200)).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    revision(&f, po, "CLOSE", 200).await?;
    let before = facts(&independent, &f).await?;
    let result = arrival(&f, po, "QA-arrival-replay", 1_000_000, Some(200)).await?;
    let after = observe(&independent, &f, "arrival-original-replay", result.0).await?;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    assert_eq!(result.1["quality_id"], first.1["quality_id"]);
    assert_eq!(result.1["replay"], true);
    assert_eq!(after, before);
    Ok(())
}

async fn legacy_arrival(pool: &PgPool, f: &Fixture, po: Uuid, request: &str) -> Result<Uuid> {
    let receipt = Uuid::new_v4();
    let quality = Uuid::new_v4();
    let mut tx = pool.begin().await?;
    let supplier: String = sqlx::query_scalar("select supplier_code from shared_purchase_revisions where tenant_id=$1 and po_id=$2 order by version_no desc limit 1")
        .bind(f.tenant.0).bind(po).fetch_one(&mut *tx).await?;
    sqlx::query("insert into shared_purchase_receipts(tenant_id,receipt_id,po_id,line_no,qty_micros) values($1,$2,$3,1,1000000)")
        .bind(f.tenant.0).bind(receipt).bind(po).execute(&mut *tx).await?;
    sqlx::query("insert into shared_receipt_quality(tenant_id,quality_id,receipt_id,po_id,line_no,warehouse_id,location_id,sku_id,supplier_code,batch_code,arrived_micros,unit_cost_cent) values($1,$2,$3,$4,1,$5,$6,$7,$8,'QA-ARRIVAL-BATCH',1000000,200)")
        .bind(f.tenant.0).bind(quality).bind(receipt).bind(po).bind(f.warehouse).bind(f.location).bind(f.sku).bind(supplier).execute(&mut *tx).await?;
    sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,'QA-ARRIVAL-BATCH','HELD','HELD',1000000,$6,$7)")
        .bind(f.tenant.0).bind(Uuid::new_v4()).bind(f.warehouse).bind(f.location).bind(f.sku).bind(quality.to_string()).bind(request).execute(&mut *tx).await?;
    sqlx::query("insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id) values($1,$2,'legacy-arrival-v1','SUCCEEDED',$3)")
        .bind(f.tenant.0).bind(request).bind(quality.to_string()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(quality)
}

#[tokio::test]
async fn purchase_arrival_legacy_hash_and_result_are_preserved_after_closure() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "arrival-legacy-replay").await?;
    let po = Uuid::new_v4();
    revision(&f, po, "OPEN", 200).await?;
    let original = legacy_arrival(&pool, &f, po, "QA-legacy-arrival").await?;
    revision(&f, po, "CLOSE", 200).await?;
    let before = facts(&independent, &f).await?;
    let result = arrival(&f, po, "QA-legacy-arrival", 1_000_000, Some(200)).await?;
    let after = observe(&independent, &f, "arrival-legacy-replay", result.0).await?;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    assert_eq!(result.1["quality_id"], original.to_string());
    assert_eq!(result.1["replay"], true);
    assert_eq!(
        after, before,
        "old receipt/hash/source must not be rewritten into v2"
    );
    Ok(())
}

#[tokio::test]
async fn purchase_arrival_legacy_weak_hash_does_not_allow_a_changed_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "arrival-legacy-mismatch").await?;
    let po = Uuid::new_v4();
    revision(&f, po, "OPEN", 200).await?;
    legacy_arrival(&pool, &f, po, "QA-legacy-arrival").await?;
    let before = facts(&independent, &f).await?;
    let result = arrival(&f, po, "QA-legacy-arrival", 2_000_000, Some(200)).await?;
    let after = observe(&independent, &f, "arrival-legacy-mismatch", result.0).await?;
    assert_eq!(result.0, StatusCode::CONFLICT, "{}", result.1);
    assert_eq!(after, before);
    Ok(())
}

#[tokio::test]
async fn purchase_arrival_missing_original_effect_cannot_replay_a_v2_receipt() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "arrival-missing-original-proof").await?;
    let po = Uuid::new_v4();
    revision(&f, po, "OPEN", 200).await?;
    let seed = arrival(&f, po, "QA-valid-arrival", 1_000_000, Some(200)).await?;
    assert_eq!(seed.0, StatusCode::OK, "{}", seed.1);
    // A same-intent hash alone is not proof that this other receipt has a real effect.
    // Only this newly owned fixture is extended; original rows remain immutable.
    let digest: String = sqlx::query_scalar("select payload_hash from command_receipts where tenant_id=$1 and request_id='QA-valid-arrival'")
        .bind(f.tenant.0).fetch_one(&pool).await?;
    let missing_quality = Uuid::new_v4();
    sqlx::query("insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id) values($1,'QA-unproven-arrival',$2,'SUCCEEDED',$3)")
        .bind(f.tenant.0).bind(digest).bind(missing_quality.to_string()).execute(&pool).await?;
    let before = facts(&independent, &f).await?;
    let result = arrival(&f, po, "QA-unproven-arrival", 1_000_000, Some(200)).await?;
    let after = observe(&independent, &f, "arrival-missing-original-proof", result.0).await?;
    assert_eq!(result.0, StatusCode::CONFLICT, "{}", result.1);
    assert!(
        result.1.to_string().contains("reconciliation"),
        "missing original source is not a successful arrival"
    );
    assert_eq!(
        after, before,
        "unknown original outcome is preserved without new effect or fabricated proof"
    );
    Ok(())
}
