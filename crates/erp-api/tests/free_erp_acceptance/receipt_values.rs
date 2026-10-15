//! Real receipt return writes conserve original posted cents and command intent.
use super::trade_facts::{Fixture, SourceFacts, fixture, seed};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn post_return(
    f: &Fixture,
    quality: Uuid,
    request: &str,
    qty: i64,
) -> Result<(StatusCode, Value)> {
    post_action(f, quality, request, "return_qty", qty).await
}

pub(super) async fn post_action(
    f: &Fixture,
    quality: Uuid,
    request: &str,
    field: &str,
    qty: i64,
) -> Result<(StatusCode, Value)> {
    let mut body = json!({"request_id":request,"supplier_id":Uuid::new_v4(),"warehouse_id":f.warehouse,
        "quality_id":quality,"items":[{"sku_id":f.sku,"qty":1,"price_cent":1}]});
    body[field] = json!(qty);
    f.account
        .request(
            "POST",
            &format!("/v1/tenants/{}/purchase-orders", f.tenant),
            body,
        )
        .await
}

pub(super) async fn facts(pool: &PgPool, f: &Fixture, quality: Uuid) -> Result<Value> {
    Ok(sqlx::query_scalar("select jsonb_build_object(
        'quality',(select to_jsonb(q) from shared_receipt_quality q where q.tenant_id=$1 and q.quality_id=$2),
        'returns',(select coalesce(jsonb_agg(to_jsonb(x) order by return_id),'[]') from shared_supplier_returns x where x.tenant_id=$1),
        'postings',(select coalesce(jsonb_agg(to_jsonb(x) order by balance_id),'[]') from partner_balances x where x.tenant_id=$1),
        'stock',(select coalesce(jsonb_agg(to_jsonb(x) order by ledger_id),'[]') from shared_stock_ledgers x where x.tenant_id=$1),
        'commands',(select coalesce(jsonb_agg(to_jsonb(x) order by request_id),'[]') from command_receipts x where x.tenant_id=$1),
        'values',(select coalesce(jsonb_agg(to_jsonb(x) order by request_id),'[]') from shared_receipt_value_commands x where x.tenant_id=$1),
        'lots',(select coalesce(jsonb_agg(to_jsonb(x) order by lot_id),'[]') from shared_receipt_cost_lots x where x.tenant_id=$1),
        'allocations',(select coalesce(jsonb_agg(to_jsonb(x) order by request_id,lot_id),'[]') from shared_receipt_cost_allocations x where x.tenant_id=$1))")
        .bind(f.tenant.0).bind(quality).fetch_one(pool).await?)
}

#[tokio::test]
async fn receipt_values_two_half_returns_restore_one_original_cent() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "return-half-cent").await?;
    let quality = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 1_000_000,
            released: 1_000_000,
            returned: 0,
            issued: 0,
            unit_cent: 1,
            release_costs: vec![1],
            return_cost: 0,
        },
    )
    .await?;
    let a = post_return(&f, quality, "QA-half-return-A", 500_000).await?;
    let b = post_return(&f, quality, "QA-half-return-B", 500_000).await?;
    assert_eq!(a.0, StatusCode::OK, "{}", a.1);
    assert_eq!(b.0, StatusCode::OK, "{}", b.1);
    let row:(i64,i64,i64,i64,i64)=sqlx::query_as("select q.payable_cent,
        (select sum(cost_cent)::bigint from shared_supplier_returns where tenant_id=$1 and receipt_id=q.receipt_id),
        (select sum(qty_micros)::bigint from shared_supplier_returns where tenant_id=$1 and receipt_id=q.receipt_id),
        (select sum(amount_cent)::bigint from partner_balances where tenant_id=$1 and source_id=q.receipt_id::text),
        (select sum(delta_micros)::bigint from shared_stock_ledgers where tenant_id=$1 and source_line=q.quality_id::text and bucket='ON_HAND')
        from shared_receipt_quality q where q.tenant_id=$1 and q.quality_id=$2")
        .bind(f.tenant.0).bind(quality).fetch_one(&independent).await?;
    eprintln!(
        "QA receipt_values_result tenant={} case=two-half quality={quality} amount_a={} amount_b={} payable={} returned_cost={} returned_qty={} ap={} stock={}",
        f.tenant, a.1["amount"], b.1["amount"], row.0, row.1, row.2, row.3, row.4
    );
    assert_eq!(
        row,
        (0, 1, 1_000_000, 0, 0),
        "two valid half returns cannot restore more than their actual one-cent release value"
    );
    assert_eq!(
        (a.1["amount"].clone(), b.1["amount"].clone()),
        (json!(1), json!(0))
    );
    Ok(())
}

#[tokio::test]
async fn receipt_values_same_command_changed_quantity_is_conflict() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "return-semantic-conflict").await?;
    let quality = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 1_000_000,
            released: 1_000_000,
            returned: 0,
            issued: 0,
            unit_cent: 1,
            release_costs: vec![1],
            return_cost: 0,
        },
    )
    .await?;
    let first = post_return(&f, quality, "QA-return-original", 250_000).await?;
    assert_eq!(first.0, StatusCode::OK, "{}", first.1);
    let before = facts(&independent, &f, quality).await?;
    let retry = post_return(&f, quality, "QA-return-original", 500_000).await?;
    let unchanged = facts(&independent, &f, quality).await? == before;
    eprintln!(
        "QA receipt_values_result tenant={} case=changed-qty quality={quality} first={} retry={} unchanged={unchanged}",
        f.tenant,
        first.0.as_u16(),
        retry.0.as_u16()
    );
    assert_eq!(
        retry.0,
        StatusCode::CONFLICT,
        "same identity with new quantity must not replay old success: {}",
        retry.1
    );
    assert!(unchanged);
    Ok(())
}
