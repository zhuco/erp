//! Fractional quantities, cumulative original-cost returns and real concurrency.

use super::{Result, SharedContractRepo, SharedWrite, TenantId, isolated_pool};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

async fn fixture(pool: &PgPool) -> Result<TenantId> {
    let tenant = TenantId(Uuid::new_v4());
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-COST-RETURN')")
        .bind(Uuid::from(tenant))
        .execute(pool)
        .await?;
    eprintln!("QA item cost tenant={}", Uuid::from(tenant));
    Ok(tenant)
}

fn stored<T>(value: Result<T, erp_domain::receipt::ReceiptError>) -> Result<T> {
    value.map_err(|error| anyhow::anyhow!("cost receipt rejected: {error:?}"))
}

async fn return_totals(reader: &mut PgConnection, tenant: TenantId) -> Result<(i64, i64, i64)> {
    Ok(sqlx::query_as("select coalesce(sum(qty_micros),0)::bigint,coalesce(sum(amount_cent),0)::bigint,count(*) from shared_item_costs where tenant_id=$1 and kind='RETURN'")
        .bind(Uuid::from(tenant)).fetch_one(reader).await?)
}

#[tokio::test]
async fn cost_full_issue_preserves_two_cent_value_and_zero_unit_rebuild() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let repo = SharedContractRepo::new(&pool);
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "seed",
            payload_hash: "seed",
            item_code: "QA-SUBCENT",
            qty_micros: 3_000_000,
            amount_cent: 2,
            period_code: "2026-10",
        })
        .await?,
    )?;
    stored(
        repo.rebuild_item_cost(tenant, "rebuild", "rebuild", "QA-SUBCENT")
            .await?,
    )?;
    let issued = stored(
        repo.post_cost_issue(erp_repo::shared_contract::CostIssueInput {
            tenant_id: tenant,
            request_id: "issue",
            payload_hash: "issue",
            item_code: "QA-SUBCENT",
            qty_micros: 3_000_000,
            period_code: "2026-10",
        })
        .await?,
    )?;
    assert_eq!(issued.1, 2, "the final issue owns the full remaining value");
    let (quantity, value, _, estimated) = repo.read_item_cost(tenant, "QA-SUBCENT").await?;
    assert_eq!((quantity, value), (0, 0));
    assert!(estimated, "an empty position has no unit quotation");
    let amount: i64 = sqlx::query_scalar(
        "select amount_cent from shared_item_costs where tenant_id=$1 and kind='ISSUE'",
    )
    .bind(Uuid::from(tenant))
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(amount, 2);
    Ok(())
}

#[tokio::test]
async fn cost_partial_returns_allocate_cumulative_cent_tail() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let repo = SharedContractRepo::new(&pool);
    // Three millionths cost two cents; this also catches whole-unit conversion.
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "seed",
            payload_hash: "seed",
            item_code: "QA-FRACTION",
            qty_micros: 3,
            amount_cent: 2,
            period_code: "2026-10",
        })
        .await?,
    )?;
    assert_eq!(
        stored(
            repo.post_cost_issue(erp_repo::shared_contract::CostIssueInput {
                tenant_id: tenant,
                request_id: "issue",
                payload_hash: "issue",
                item_code: "QA-FRACTION",
                qty_micros: 3,
                period_code: "2026-10"
            })
            .await?
        )?
        .1,
        2
    );
    for (index, expected) in [1, 0, 1].into_iter().enumerate() {
        let request = format!("return-{index}");
        let (effect, amount, estimated) = stored(
            repo.post_cost_return(erp_repo::shared_contract::CostReturnCommand {
                tenant_id: tenant,
                request_id: &request,
                payload_hash: &request,
                item_code: "QA-FRACTION",
                issue_request: "issue",
                return_qty: 1,
                period_code: "2026-10",
            })
            .await?,
        )?;
        assert!(matches!(effect, SharedWrite::Stored(_)));
        assert_eq!(amount, expected);
        assert!(!estimated);
        let replay = stored(
            repo.post_cost_return(erp_repo::shared_contract::CostReturnCommand {
                tenant_id: tenant,
                request_id: &request,
                payload_hash: &request,
                item_code: "QA-FRACTION",
                issue_request: "issue",
                return_qty: 1,
                period_code: "2026-10",
            })
            .await?,
        )?;
        assert!(matches!(replay.0, SharedWrite::Replay(_)));
        assert_eq!(replay.1, expected);
    }
    assert_eq!(return_totals(&mut reader, tenant).await?, (3, 2, 3));
    assert!(
        repo.post_cost_return(erp_repo::shared_contract::CostReturnCommand {
            tenant_id: tenant,
            request_id: "over",
            payload_hash: "over",
            item_code: "QA-FRACTION",
            issue_request: "issue",
            return_qty: 1,
            period_code: "2026-10"
        })
        .await
        .is_err()
    );
    let missing: i64 = sqlx::query_scalar(
        "select count(*) from command_receipts where tenant_id=$1 and request_id='over'",
    )
    .bind(Uuid::from(tenant))
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(missing, 0);
    Ok(())
}

#[tokio::test]
async fn cost_twenty_original_returns_never_exceed_source_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let repo = SharedContractRepo::new(&pool);
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "seed",
            payload_hash: "seed",
            item_code: "QA-CONCURRENT",
            qty_micros: 500_000,
            amount_cent: 600,
            period_code: "2026-10",
        })
        .await?,
    )?;
    stored(
        repo.post_cost_issue(erp_repo::shared_contract::CostIssueInput {
            tenant_id: tenant,
            request_id: "issue",
            payload_hash: "issue",
            item_code: "QA-CONCURRENT",
            qty_micros: 500_000,
            period_code: "2026-10",
        })
        .await?,
    )?;
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..20 {
        let pool = pool.clone();
        tasks.spawn(async move {
            let request = format!("return-{index}");
            SharedContractRepo::new(&pool)
                .post_cost_return(erp_repo::shared_contract::CostReturnCommand {
                    tenant_id: tenant,
                    request_id: &request,
                    payload_hash: &request,
                    item_code: "QA-CONCURRENT",
                    issue_request: "issue",
                    return_qty: 400_000,
                    period_code: "2026-10",
                })
                .await
        });
    }
    let mut accepted = 0;
    let mut rejected = 0;
    while let Some(result) = tasks.join_next().await {
        match result? {
            Ok(Ok((SharedWrite::Stored(_), 480, false))) => accepted += 1,
            Err(error) if error.to_string().contains("return exceeds") => rejected += 1,
            other => anyhow::bail!("unexpected return result: {other:?}"),
        }
    }
    assert_eq!((accepted, rejected), (1, 19));
    assert_eq!(return_totals(&mut reader, tenant).await?, (400_000, 480, 1));
    Ok(())
}

#[tokio::test]
async fn cost_twenty_same_commands_return_one_original_result() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let mut blocker = pool.begin().await?;
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext('item-cost:QA-REPLAY'))")
        .bind(Uuid::from(tenant).to_string())
        .execute(&mut *blocker)
        .await?;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let pool = pool.clone();
        tasks.spawn(async move {
            SharedContractRepo::new(&pool)
                .post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
                    tenant_id: tenant,
                    request_id: "same",
                    payload_hash: "same",
                    item_code: "QA-REPLAY",
                    qty_micros: 1_000_000,
                    amount_cent: 1200,
                    period_code: "2026-10",
                })
                .await
        });
    }
    // Hold the item fence until at least two commands have reached a lock.
    // Before the repair both have read a missing receipt; after it the second
    // waits at the command fence. This avoids relying on scheduler timing.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut waiting = 0_i64;
    while waiting < 2 && tokio::time::Instant::now() < deadline {
        waiting = sqlx::query_scalar("select count(*) from pg_locks where locktype='advisory' and not granted and classid::bigint=(hashtext($1)::bigint & 4294967295)")
            .bind(Uuid::from(tenant).to_string()).fetch_one(&mut *reader).await?;
        if waiting < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    blocker.commit().await?;
    assert_eq!(
        waiting, 2,
        "two independent commands really reached the contention point"
    );
    let mut effects = std::collections::BTreeSet::new();
    let mut writes = 0;
    while let Some(result) = tasks.join_next().await {
        let (effect, amount, estimated) = stored(result??)?;
        assert_eq!(amount, 1200);
        assert!(!estimated);
        match effect {
            SharedWrite::Stored(receipt) => {
                writes += 1;
                effects.insert(receipt.effect_id);
            }
            SharedWrite::Replay(receipt) => {
                effects.insert(receipt.effect_id);
            }
        }
    }
    assert_eq!((writes, effects.len()), (1, 1));
    let rows: i64 = sqlx::query_scalar("select count(*) from shared_item_costs where tenant_id=$1")
        .bind(Uuid::from(tenant))
        .fetch_one(&mut *reader)
        .await?;
    assert_eq!(rows, 1);
    Ok(())
}

#[tokio::test]
async fn cost_same_identity_different_business_payload_is_refused() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let repo = SharedContractRepo::new(&pool);
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "same",
            payload_hash: "opaque-caller-hash",
            item_code: "QA-PAYLOAD",
            qty_micros: 1_000_000,
            amount_cent: 100,
            period_code: "2026-10",
        })
        .await?,
    )?;
    for (item, quantity, amount, period) in [
        ("OTHER", 1_000_000, 100, "2026-10"),
        ("QA-PAYLOAD", 2_000_000, 100, "2026-10"),
        ("QA-PAYLOAD", 1_000_000, 101, "2026-10"),
        ("QA-PAYLOAD", 1_000_000, 100, "2026-11"),
    ] {
        assert!(
            repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
                tenant_id: tenant,
                request_id: "same",
                payload_hash: "opaque-caller-hash",
                item_code: item,
                qty_micros: quantity,
                amount_cent: amount,
                period_code: period
            })
            .await
            .is_err()
        );
    }
    assert_eq!(
        repo.read_item_cost(tenant, "QA-PAYLOAD").await?,
        (1_000_000, 100, 100, false)
    );
    Ok(())
}

#[tokio::test]
async fn cost_estimated_quantity_overflow_has_no_append_or_receipt() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let repo = SharedContractRepo::new(&pool);
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "max",
            payload_hash: "max",
            item_code: "QA-MAX",
            qty_micros: i64::MAX,
            amount_cent: 0,
            period_code: "2026-10",
        })
        .await?,
    )?;
    assert!(
        repo.post_estimated_cost(tenant, "overflow", "overflow", "QA-MAX", 1, "2026-10")
            .await
            .is_err()
    );
    let (entries,receipts):(i64,i64)=sqlx::query_as("select (select count(*) from shared_item_costs where tenant_id=$1),(select count(*) from command_receipts where tenant_id=$1)")
        .bind(Uuid::from(tenant)).fetch_one(&mut *reader).await?;
    assert_eq!((entries, receipts), (1, 1));
    assert_eq!(
        repo.read_item_cost(tenant, "QA-MAX").await?,
        (i64::MAX, 0, 0, false)
    );
    stored(
        repo.rebuild_item_cost(tenant, "rebuild", "rebuild", "QA-MAX")
            .await?,
    )?;
    Ok(())
}

#[tokio::test]
async fn cost_unknown_receipt_stays_estimated_until_its_stock_is_consumed() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let repo = SharedContractRepo::new(&pool);
    stored(
        repo.post_estimated_cost(
            tenant,
            "unknown",
            "unknown",
            "QA-UNKNOWN",
            500_000,
            "2026-10",
        )
        .await?,
    )?;
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "known",
            payload_hash: "known",
            item_code: "QA-UNKNOWN",
            qty_micros: 500_000,
            amount_cent: 600,
            period_code: "2026-10",
        })
        .await?,
    )?;
    assert_eq!(
        repo.read_item_cost(tenant, "QA-UNKNOWN").await?,
        (1_000_000, 600, 600, true)
    );
    let issue = stored(
        repo.post_cost_issue(erp_repo::shared_contract::CostIssueInput {
            tenant_id: tenant,
            request_id: "issue",
            payload_hash: "issue",
            item_code: "QA-UNKNOWN",
            qty_micros: 1_000_000,
            period_code: "2026-10",
        })
        .await?,
    )?;
    assert_eq!(
        (issue.1, issue.2),
        (600, true),
        "known value is carried without claiming complete valuation"
    );
    let returned = stored(
        repo.post_cost_return(erp_repo::shared_contract::CostReturnCommand {
            tenant_id: tenant,
            request_id: "return",
            payload_hash: "return",
            item_code: "QA-UNKNOWN",
            issue_request: "issue",
            return_qty: 1_000_000,
            period_code: "2026-10",
        })
        .await?,
    )?;
    assert_eq!((returned.1, returned.2), (600, true));
    assert!(repo.read_item_cost(tenant, "QA-UNKNOWN").await?.3);
    stored(
        repo.post_cost_issue(erp_repo::shared_contract::CostIssueInput {
            tenant_id: tenant,
            request_id: "issue-again",
            payload_hash: "issue-again",
            item_code: "QA-UNKNOWN",
            qty_micros: 1_000_000,
            period_code: "2026-10",
        })
        .await?,
    )?;
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "fresh",
            payload_hash: "fresh",
            item_code: "QA-UNKNOWN",
            qty_micros: 1_000_000,
            amount_cent: 1200,
            period_code: "2026-10",
        })
        .await?,
    )?;
    assert_eq!(
        repo.read_item_cost(tenant, "QA-UNKNOWN").await?,
        (1_000_000, 1200, 1200, false)
    );
    Ok(())
}

async fn await_lock_waiter(reader: &mut PgConnection, tenant: TenantId, key: &str) -> Result<()> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        let waiting:i64=sqlx::query_scalar("select count(*) from pg_locks where locktype='advisory' and not granted and classid::bigint=(hashtext($1)::bigint & 4294967295) and objid::bigint=(hashtext($2)::bigint & 4294967295)")
            .bind(Uuid::from(tenant).to_string()).bind(key).fetch_one(&mut *reader).await?;
        if waiting == 1 {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    anyhow::bail!("expected real PostgreSQL advisory lock waiter was not observed")
}

#[tokio::test]
async fn cost_period_close_waits_for_active_posting_fence() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let repo = SharedContractRepo::new(&pool);
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "seed",
            payload_hash: "seed",
            item_code: "QA-CLOSE",
            qty_micros: 1_000_000,
            amount_cent: 100,
            period_code: "2026-10",
        })
        .await?,
    )?;
    let mut blocker = pool.begin().await?;
    sqlx::query(
        "select pg_advisory_xact_lock_shared(hashtext($1),hashtext('cost-period:2026-10'))",
    )
    .bind(Uuid::from(tenant).to_string())
    .execute(&mut *blocker)
    .await?;
    let closing_pool = pool.clone();
    let closing = tokio::spawn(async move {
        SharedContractRepo::new(&closing_pool)
            .close_item_cost_period(tenant, "close", "close", "2026-10")
            .await
    });
    await_lock_waiter(&mut reader, tenant, "cost-period:2026-10").await?;
    blocker.commit().await?;
    stored(closing.await??)?;
    assert!(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "late",
            payload_hash: "late",
            item_code: "QA-CLOSE",
            qty_micros: 1_000_000,
            amount_cent: 100,
            period_code: "2026-10"
        })
        .await
        .is_err()
    );
    let replay = stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "seed",
            payload_hash: "seed",
            item_code: "QA-CLOSE",
            qty_micros: 1_000_000,
            amount_cent: 100,
            period_code: "2026-10",
        })
        .await?,
    )?;
    assert!(matches!(replay.0, SharedWrite::Replay(_)));
    let count: i64 =
        sqlx::query_scalar("select count(*) from shared_item_costs where tenant_id=$1")
            .bind(Uuid::from(tenant))
            .fetch_one(&mut *reader)
            .await?;
    assert_eq!(count, 1);
    Ok(())
}

#[tokio::test]
async fn cost_history_orders_effects_after_fences_not_transaction_start() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let mut blocker = pool.begin().await?;
    sqlx::query(
        "select pg_advisory_xact_lock(hashtext($1),hashtext('cost-command:delayed-issue'))",
    )
    .bind(Uuid::from(tenant).to_string())
    .execute(&mut *blocker)
    .await?;
    let delayed_pool = pool.clone();
    let delayed = tokio::spawn(async move {
        SharedContractRepo::new(&delayed_pool)
            .post_cost_issue(erp_repo::shared_contract::CostIssueInput {
                tenant_id: tenant,
                request_id: "delayed-issue",
                payload_hash: "delayed-issue",
                item_code: "QA-ORDER",
                qty_micros: 1_000_000,
                period_code: "2026-10",
            })
            .await
    });
    await_lock_waiter(&mut reader, tenant, "cost-command:delayed-issue").await?;
    let repo = SharedContractRepo::new(&pool);
    stored(
        repo.post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: tenant,
            request_id: "seed",
            payload_hash: "seed",
            item_code: "QA-ORDER",
            qty_micros: 1_000_000,
            amount_cent: 100,
            period_code: "2026-10",
        })
        .await?,
    )?;
    blocker.commit().await?;
    assert_eq!(stored(delayed.await??)?.1, 100);
    let (older_clock,newer_sequence):(bool,bool)=sqlx::query_as("select i.created_at<r.created_at,i.entry_sequence>r.entry_sequence from shared_item_costs i join shared_item_costs r on r.tenant_id=i.tenant_id and r.kind='RECEIPT' where i.tenant_id=$1 and i.kind='ISSUE'")
        .bind(Uuid::from(tenant)).fetch_one(&mut *reader).await?;
    assert!(
        older_clock && newer_sequence,
        "transaction-start timestamps would invert these real effects"
    );
    assert_eq!(
        repo.read_item_cost(tenant, "QA-ORDER").await?,
        (0, 0, 0, true)
    );
    Ok(())
}
