use anyhow::{Context, Result};
use erp_domain::manufacturing::BomLine;
use erp_repo::shared_contract::{SharedContractRepo, SharedWrite};
use erp_types::TenantId;

use super::{isolated_pool, qty};

#[tokio::test]
async fn manufacturing_bom_shared_subtree_is_stored_once_and_replays() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = TenantId(uuid::Uuid::new_v4());
    sqlx::query("insert into tenants(tenant_id, name) values ($1,'QA-BOM-graph')")
        .bind(uuid::Uuid::from(tenant))
        .execute(&pool)
        .await?;
    let mut lines = Vec::new();
    for (parent, component, per, supply, line_no) in [
        ("P", "A", "2", "MAKE", 10),
        ("P", "B", "3", "MAKE", 20),
        ("A", "S", "5", "MAKE", 30),
        ("B", "S", "7", "MAKE", 40),
        ("S", "R", "0.125", "BUY", 50),
    ] {
        lines.push(BomLine {
            parent: parent.into(),
            component: component.into(),
            qty_per: qty(per),
            unit_code: "KG".into(),
            scrap_ppm: 0,
            supply: supply.into(),
            line_no,
        });
    }
    let repo = SharedContractRepo::new(&pool);
    let (_, bom_id) = repo
        .save_bom_structure(tenant, "graph-save", "graph-save", "P", &lines)
        .await?
        .map_err(|error| anyhow::anyhow!("save rejected: {error:?}"))?;
    let (_, explode_id, count) = repo
        .explode_bom_structure(tenant, "graph-expand", "graph-expand", bom_id, 1_000_000)
        .await?
        .map_err(|error| anyhow::anyhow!("expand rejected: {error:?}"))?;
    assert_eq!(count, 5);
    let (raw_rows, raw_qty, depth): (i64, i64, i32) = sqlx::query_as(
        "select count(*)::bigint, sum(qty_micros)::bigint, max(depth) from shared_mfg_bom_explodes where tenant_id=$1 and explode_id=$2 and component_code='R'",
    ).bind(uuid::Uuid::from(tenant)).bind(explode_id).fetch_one(&pool).await?;
    assert_eq!((raw_rows, raw_qty, depth), (1, 3_875_000, 3));
    let (write, replay_id, replay_count) = repo
        .explode_bom_structure(tenant, "graph-expand", "graph-expand", bom_id, 1_000_000)
        .await?
        .map_err(|error| anyhow::anyhow!("replay rejected: {error:?}"))?;
    assert!(matches!(write, SharedWrite::Replay(_)));
    assert_eq!((replay_id, replay_count), (explode_id, 5));
    let (rows, receipts): (i64, i64) = sqlx::query_as(
        "select (select count(*) from shared_mfg_bom_explodes where tenant_id=$1), (select count(*) from command_receipts where tenant_id=$1)",
    ).bind(uuid::Uuid::from(tenant)).fetch_one(&pool).await?;
    assert_eq!((rows, receipts), (5, 2));
    assert!(
        repo.explode_bom_structure(
            TenantId(uuid::Uuid::new_v4()),
            "graph-foreign",
            "graph-foreign",
            bom_id,
            1_000_000
        )
        .await
        .is_err()
    );
    let source_qty: i64 = sqlx::query_scalar(
        "select qty_per_micros from shared_mfg_bom_lines where tenant_id=$1 and bom_id=$2 and line_no=50",
    ).bind(uuid::Uuid::from(tenant)).bind(bom_id).fetch_optional(&pool).await?
        .context("source BOM line remains")?;
    assert_eq!(source_qty, 125_000);
    println!(
        "BOM_GRAPH_READ_BACK tenant={tenant:?} bom={bom_id} explode={explode_id} raw={raw_qty} rows={rows} receipts={receipts}"
    );
    Ok(())
}
