//! Actual-source remainder, historical replay and concurrent procurement writes.
use super::receipt_values::{facts, post_action, post_return};
use super::trade_facts::{Fixture, SourceFacts, fixture, seed};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

async fn source(
    pool: &PgPool,
    f: &Fixture,
    arrived: i64,
    released: i64,
    issued: i64,
    cost: i64,
) -> Result<Uuid> {
    seed(
        pool,
        f,
        &SourceFacts {
            arrived,
            released,
            issued,
            returned: 0,
            unit_cent: cost,
            release_costs: if released > 0 { vec![cost] } else { vec![] },
            return_cost: 0,
        },
    )
    .await
}
async fn read(f: &Fixture, q: Uuid) -> Result<Value> {
    let r = f
        .account
        .request(
            "GET",
            &format!(
                "/v1/tenants/{}/purchase-receipt-offsets?quality_id={q}",
                f.tenant
            ),
            Value::Null,
        )
        .await?;
    assert_eq!(r.0, StatusCode::OK, "{}", r.1);
    Ok(r.1)
}
fn ok(r: &(StatusCode, Value)) {
    assert_eq!(r.0, StatusCode::OK, "{}", r.1);
}

#[tokio::test]
async fn receipt_value_edges_later_release_keeps_prior_zero_tail() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "value-later-release").await?;
    let q = source(&pool, &f, 2_000_000, 1_000_000, 0, 1).await?;
    let a = post_return(&f, q, "QA-tail-return-A", 500_000).await?;
    ok(&a);
    assert_eq!(a.1["amount"], 1);
    let r = post_action(&f, q, "QA-tail-release-B", "release_qty", 1_000_000).await?;
    ok(&r);
    let before = facts(&independent, &f, q).await?;
    let replay = post_return(&f, q, "QA-tail-return-A", 500_000).await?;
    ok(&replay);
    assert_eq!(replay.1["amount"], 1);
    assert_eq!(replay.1["replay"], true);
    assert_eq!(facts(&independent, &f, q).await?, before);
    let b = post_return(&f, q, "QA-tail-return-B", 1_500_000).await?;
    ok(&b);
    assert_eq!(b.1["amount"], 1);
    let actual = read(&f, q).await?;
    assert_eq!(actual["payable_cent"], 0);
    assert_eq!(actual["returned_cost"], 2);
    assert_eq!(actual["on_hand_micros"], 0);
    eprintln!(
        "QA receipt_values_result tenant={} case=later-release quality={q} returned=2 payable=0 stock=0 replay_unchanged=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_value_edges_two_actual_half_release_costs_are_preserved() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool, "value-half-releases").await?;
    let q = source(&pool, &f, 1_000_000, 0, 0, 1).await?;
    for (request, field) in [
        ("QA-half-release-A", "release_qty"),
        ("QA-half-release-B", "release_qty"),
        ("QA-half-lot-return-A", "return_qty"),
        ("QA-half-lot-return-B", "return_qty"),
    ] {
        let r = post_action(&f, q, request, field, 500_000).await?;
        ok(&r);
        if field == "return_qty" {
            assert_eq!(r.1["amount"], 1);
        }
    }
    let actual = read(&f, q).await?;
    assert_eq!(actual["released_cost_cent"], 2);
    assert_eq!(actual["returned_cost"], 2);
    assert_eq!(actual["payable_cent"], 0);
    assert_eq!(actual["on_hand_micros"], 0);
    eprintln!(
        "QA receipt_values_result tenant={} case=half-releases quality={q} gross=2 returned=2 payable=0 stock=0",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_value_edges_issue_and_return_share_original_one_cent() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "value-issue-return").await?;
    let q = source(&pool, &f, 1_000_000, 1_000_000, 0, 1).await?;
    let issued = post_action(&f, q, "QA-half-issue", "issue_qty", 500_000).await?;
    ok(&issued);
    assert_eq!(issued.1["left"], 500_000);
    let returned = post_return(&f, q, "QA-half-after-issue", 500_000).await?;
    ok(&returned);
    assert_eq!(returned.1["amount"], 0);
    let issue_cost:i64=sqlx::query_scalar("select amount_cent from shared_receipt_value_commands where tenant_id=$1 and request_id='QA-half-issue'")
        .bind(f.tenant.0).fetch_one(&independent).await?;
    assert_eq!(issue_cost, 1);
    let actual = read(&f, q).await?;
    assert_eq!(actual["issued_qty"], 500_000);
    assert_eq!(actual["returned_cost"], 0);
    assert_eq!(actual["payable_cent"], 1);
    assert_eq!(actual["on_hand_micros"], 0);
    let before = facts(&independent, &f, q).await?;
    let again = post_action(&f, q, "QA-half-issue", "issue_qty", 500_000).await?;
    ok(&again);
    assert_eq!(again.1["left"], 500_000);
    assert_eq!(again.1["replay"], true);
    let changed = post_action(&f, q, "QA-half-issue", "return_qty", 500_000).await?;
    assert_eq!(changed.0, StatusCode::CONFLICT);
    assert_eq!(facts(&independent, &f, q).await?, before);
    eprintln!(
        "QA receipt_values_result tenant={} case=issue-return quality={q} issue_cost=1 returned=0 payable=1 stock=0 replay_unchanged=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_value_edges_known_zero_and_missing_history_are_distinct() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "value-zero-history").await?;
    let q = source(&pool, &f, 1_000_000, 1_000_000, 0, 0).await?;
    for request in ["QA-zero-return-A", "QA-zero-return-B"] {
        let r = post_return(&f, q, request, 500_000).await?;
        ok(&r);
        assert_eq!(r.1["amount"], 0);
    }
    let actual = read(&f, q).await?;
    assert_eq!(actual["payable_cent"], 0);
    let unknown = source(&pool, &f, 1_000_000, 1_000_000, 250_000, 1).await?;
    let before = facts(&independent, &f, unknown).await?;
    let r = post_return(&f, unknown, "QA-unknown-issue-return", 500_000).await?;
    assert_eq!(r.0, StatusCode::CONFLICT);
    assert_eq!(facts(&independent, &f, unknown).await?, before);
    eprintln!(
        "QA receipt_values_result tenant={} case=zero-history quality={q} returned=0 unknown_quality={unknown} unknown_rejected=409 zero_writes=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_value_edges_twenty_competing_returns_cannot_overconsume() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "value-twenty-race").await?;
    let q = source(&pool, &f, 1_000_000, 1_000_000, 0, 1).await?;
    let requests: Vec<_> = (0..20).map(|n| format!("QA-race-half-{n}")).collect();
    let results =
        futures::future::join_all(requests.iter().map(|r| post_return(&f, q, r, 500_000))).await;
    let mut accepted = 0;
    let mut rejected = 0;
    let mut cents = 0;
    for r in results {
        let r = r?;
        match r.0 {
            StatusCode::OK => {
                accepted += 1;
                cents += r.1["amount"]
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("missing original amount"))?
            }
            StatusCode::CONFLICT => rejected += 1,
            _ => return Err(anyhow::anyhow!("unexpected {}: {}", r.0, r.1)),
        }
    }
    assert_eq!((accepted, rejected, cents), (2, 18, 1));
    let count: i64 =
        sqlx::query_scalar("select count(*) from shared_receipt_value_commands where tenant_id=$1")
            .bind(f.tenant.0)
            .fetch_one(&independent)
            .await?;
    assert_eq!(count, 2);
    let actual = read(&f, q).await?;
    assert_eq!(actual["payable_cent"], 0);
    assert_eq!(actual["on_hand_micros"], 0);
    eprintln!(
        "QA receipt_values_result tenant={} case=twenty-race quality={q} accepted=2 rejected=18 cents=1 commands=2",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_value_edges_legacy_id_needs_original_source_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "value-legacy-replay").await?;
    let q = seed(
        &pool,
        &f,
        &SourceFacts {
            arrived: 1_000_000,
            released: 1_000_000,
            returned: 500_000,
            issued: 0,
            unit_cent: 1,
            release_costs: vec![1],
            return_cost: 1,
        },
    )
    .await?;
    sqlx::query("update shared_stock_ledgers set request_id='QA-legacy-half-return' where tenant_id=$1 and source_line=$2 and bucket='ON_HAND' and delta_micros=-500000")
        .bind(f.tenant.0).bind(q.to_string()).execute(&pool).await?;
    sqlx::query("insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id) values($1,'QA-legacy-half-return','QA-legacy-half-return','SUCCEEDED','1')")
        .bind(f.tenant.0).execute(&pool).await?;
    let before = facts(&independent, &f, q).await?;
    let replay = post_return(&f, q, "QA-legacy-half-return", 500_000).await?;
    ok(&replay);
    assert_eq!(replay.1["amount"], 1);
    assert_eq!(replay.1["replay"], true);
    let changed = post_return(&f, q, "QA-legacy-half-return", 250_000).await?;
    assert_eq!(changed.0, StatusCode::CONFLICT);
    assert_eq!(facts(&independent, &f, q).await?, before);
    let final_return = post_return(&f, q, "QA-legacy-tail-return", 500_000).await?;
    ok(&final_return);
    assert_eq!(final_return.1["amount"], 0);
    let actual = read(&f, q).await?;
    assert_eq!(actual["payable_cent"], 0);
    eprintln!(
        "QA receipt_values_result tenant={} case=legacy-replay quality={q} replay=200 changed=409 original_amount=1 final_amount=0 zero_replay_writes=true",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_value_edges_failure_after_source_writes_rolls_back_every_fact() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "value-rollback").await?;
    let q = source(&pool, &f, 1_000_000, 1_000_000, 0, 1).await?;
    let name = format!("qa_receipt_value_{}", Uuid::new_v4().simple());
    let create = format!(
        "create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then raise exception 'owned QA receipt command storage fault'; end if; return new; end $$",
        f.tenant
    );
    sqlx::query(&create).execute(&pool).await?;
    sqlx::query(&format!("create trigger {name} before insert on shared_receipt_value_commands for each row execute function {name}()")).execute(&pool).await?;
    let before = facts(&independent, &f, q).await?;
    let result = post_return(&f, q, "QA-storage-return", 500_000).await;
    let remove_trigger = sqlx::query(&format!(
        "drop trigger {name} on shared_receipt_value_commands"
    ))
    .execute(&pool)
    .await;
    let remove_function = sqlx::query(&format!("drop function {name}()"))
        .execute(&pool)
        .await;
    remove_trigger?;
    remove_function?;
    let r = result?;
    assert_eq!(r.0, StatusCode::INTERNAL_SERVER_ERROR, "{}", r.1);
    assert_eq!(facts(&independent, &f, q).await?, before);
    let retry = post_return(&f, q, "QA-storage-return", 500_000).await?;
    ok(&retry);
    assert_eq!(retry.1["amount"], 1);
    assert_eq!(retry.1["replay"], false);
    eprintln!(
        "QA receipt_values_result tenant={} case=rollback quality={q} rejected=500 zero_failed_writes=true retry=200 amount=1",
        f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn receipt_value_edges_source_action_and_racing_identity_stay_bound() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = fixture(&pool, "value-source-identity").await?;
    let q = source(&pool, &f, 1_000_000, 1_000_000, 0, 1).await?;
    let other = source(&pool, &f, 1_000_000, 1_000_000, 0, 1).await?;
    let first = post_return(&f, q, "QA-same-return", 500_000).await?;
    ok(&first);
    let before = facts(&independent, &f, q).await?;
    let changed = post_return(&f, other, "QA-same-return", 500_000).await?;
    assert_eq!(changed.0, StatusCode::CONFLICT);
    let mixed=f.account.request("POST",&format!("/v1/tenants/{}/purchase-orders",f.tenant),
        json!({"request_id":"QA-mixed-actions","supplier_id":Uuid::new_v4(),"warehouse_id":f.warehouse,
            "quality_id":q,"release_qty":500_000,"return_qty":500_000,"items":[{"sku_id":f.sku,"qty":1,"price_cent":1}]})).await?;
    assert_eq!(mixed.0, StatusCode::BAD_REQUEST);
    assert_eq!(facts(&independent, &f, q).await?, before);
    let requests: Vec<_> = (0..20).map(|_| "QA-same-racing-return").collect();
    let results =
        futures::future::join_all(requests.iter().map(|r| post_return(&f, q, r, 500_000))).await;
    let mut stored = 0;
    let mut replayed = 0;
    for r in results {
        let r = r?;
        ok(&r);
        if r.1["replay"] == true {
            replayed += 1
        } else {
            stored += 1
        }
    }
    assert_eq!((stored, replayed), (1, 19));
    let actual = read(&f, q).await?;
    assert_eq!(actual["returned_cost"], 1);
    assert_eq!(actual["payable_cent"], 0);
    eprintln!(
        "QA receipt_values_result tenant={} case=source-identity quality={q} other={other} changed=409 mixed=400 stored=1 replayed=19",
        f.tenant
    );
    Ok(())
}
