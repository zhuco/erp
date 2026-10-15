//! Explicit Release benchmark of protected QA snapshot loading, graph expansion,
//! and committed result storage. It does not measure HTTP, UI, or public hosting.

use std::{collections::BTreeSet, time::Instant};

use anyhow::{Context, Result};
use erp_domain::{manufacturing::BomLine, quantity::ScaledQty};
use erp_repo::shared_contract::{SharedContractRepo, SharedWrite};
use erp_types::TenantId;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgPoolOptions};

async fn protected_pool() -> Result<PgPool> {
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")
        .context("QA database not configured: performance NOT_RUN")?;
    let options: sqlx::postgres::PgConnectOptions = url
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid QA database configuration"))?;
    if options.get_host() != "127.0.0.1"
        || options.get_port() != 15438
        || options.get_database() != Some("erp_shared_contract_test")
        || options.get_username() != "erp_grid_test"
    {
        anyhow::bail!("refusing an unapproved QA database identity");
    }
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    let identity: (String, i32) = sqlx::query_as("select current_database(), inet_server_port()")
        .fetch_one(&pool)
        .await?;
    if identity != ("erp_shared_contract_test".into(), 15438) {
        anyhow::bail!("QA database identity does not match");
    }
    Ok(pool)
}

fn fixture() -> Result<Vec<BomLine>> {
    const WIDTH: usize = 1_108;
    let mut rows = Vec::with_capacity(50_000);
    let mut edge = |parent: String, component: String, supply: &str| -> Result<()> {
        rows.push(BomLine {
            parent,
            component,
            qty_per: ScaledQty::from_micros(1_000_000),
            unit_code: "KG".into(),
            scrap_ppm: 0,
            supply: supply.into(),
            line_no: i32::try_from(rows.len() + 1)?,
        });
        Ok(())
    };
    for column in 0..WIDTH {
        edge("ROOT".into(), format!("L0_{column:04}"), "MAKE")?;
    }
    for (level, fanout) in [6, 6, 6, 6, 5, 5, 5, 5].into_iter().enumerate() {
        for column in 0..WIDTH {
            for step in 0..fanout {
                let child = (column + step) % WIDTH;
                let supply = if level == 7 && child != 0 {
                    "BUY"
                } else {
                    "MAKE"
                };
                edge(
                    format!("L{level}_{column:04}"),
                    format!("L{}_{child:04}", level + 1),
                    supply,
                )?;
            }
        }
    }
    for column in 0..113 {
        edge(
            "ROOT".into(),
            format!("L8_{column:04}"),
            if column == 0 { "MAKE" } else { "BUY" },
        )?;
    }
    for index in 0..23 {
        let parent = if index == 0 {
            "L8_0000".into()
        } else {
            format!("C{:02}", index - 1)
        };
        edge(
            parent,
            format!("C{index:02}"),
            if index == 22 { "BUY" } else { "MAKE" },
        )?;
    }
    for index in 0..4 {
        edge("ROOT".into(), format!("EXTRA{index}"), "BUY")?;
    }
    assert_eq!(rows.len(), 50_000);
    let nodes: BTreeSet<&str> = rows
        .iter()
        .flat_map(|line| [line.parent.as_str(), line.component.as_str()])
        .collect();
    assert_eq!(nodes.len(), 10_000);
    Ok(rows)
}

#[tokio::test]
#[ignore = "explicit protected QA Release performance run"]
async fn bom_snapshot_expand_and_commit_10000_items_50000_edges_32_levels() -> Result<()> {
    if cfg!(debug_assertions) {
        anyhow::bail!("performance requires the Release profile");
    }
    println!("BOM_PERFORMANCE_PID={}", std::process::id());
    let pool = protected_pool().await?;
    let tenant = TenantId(uuid::Uuid::new_v4());
    sqlx::query("insert into tenants(tenant_id,name) values ($1,'QA-BOM-performance')")
        .bind(uuid::Uuid::from(tenant))
        .execute(&pool)
        .await?;
    let rows = fixture()?;
    let mut digest = Sha256::new();
    for line in &rows {
        digest.update(
            format!(
                "{}\0{}\0{}\0{}\0{}\0{}\0{}\n",
                line.line_no,
                line.parent,
                line.component,
                line.qty_per.as_micros(),
                line.unit_code,
                line.scrap_ppm,
                line.supply
            )
            .as_bytes(),
        );
    }
    let input_sha256 = format!("{:x}", digest.finalize());
    let repo = SharedContractRepo::new(&pool);
    let (_, bom_id) = repo
        .save_bom_structure(tenant, "performance-source", &input_sha256, "ROOT", &rows)
        .await?
        .map_err(|error| anyhow::anyhow!("source rejected: {error:?}"))?;
    drop(rows);
    // Uniform layers contribute WIDTH times each layer's total. The direct
    // root edges add 117 units; the 23-edge tail carries 810001 units per edge.
    let expected_total = (1_108_i64
        * (1 + 6 + 36 + 216 + 1_296 + 6_480 + 32_400 + 162_000 + 810_000)
        + 117
        + 23 * 810_001)
        * 1_000_000;
    let mut samples_ms = Vec::new();
    let mut first_measured = None;
    for run in 0..32 {
        let request = format!("performance-expand-{run:02}");
        let start = Instant::now();
        let (_, explode_id, count) = repo
            .explode_bom_structure(tenant, &request, &request, bom_id, 1_000_000)
            .await?
            .map_err(|error| anyhow::anyhow!("expand rejected: {error:?}"))?;
        let elapsed = start.elapsed().as_secs_f64() * 1_000.0;
        let read_back: (i64, i64, i32) = sqlx::query_as(
            "select count(*)::bigint, sum(qty_micros)::bigint, max(depth) from shared_mfg_bom_explodes where tenant_id=$1 and explode_id=$2",
        ).bind(uuid::Uuid::from(tenant)).bind(explode_id).fetch_one(&pool).await?;
        assert_eq!((count, read_back), (50_000, (50_000, expected_total, 32)));
        if run >= 2 {
            if first_measured.is_none() {
                first_measured = Some((request, explode_id));
            }
            samples_ms.push(elapsed);
        }
        println!("BOM_PERFORMANCE_SAMPLE run={run} elapsed_ms={elapsed:.3} verified_rows={count}");
    }
    let (request, original) = first_measured.context("first measured run")?;
    let (write, replay, count) = repo
        .explode_bom_structure(tenant, &request, &request, bom_id, 1_000_000)
        .await?
        .map_err(|error| anyhow::anyhow!("replay rejected: {error:?}"))?;
    assert!(matches!(write, SharedWrite::Replay(_)));
    assert_eq!((replay, count), (original, 50_000));
    let total_rows: i64 = sqlx::query_scalar(
        "select count(*)::bigint from shared_mfg_bom_explodes where tenant_id=$1",
    )
    .bind(uuid::Uuid::from(tenant))
    .fetch_one(&pool)
    .await?;
    assert_eq!(total_rows, 32 * 50_000);
    let mut sorted = samples_ms.clone();
    sorted.sort_by(f64::total_cmp);
    let p95_ms = sorted[28];
    let max_ms = sorted[29];
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .context("workspace root")?;
    let output = workspace.join("qa/performance/free-erp-bom.json");
    let report = serde_json::json!({
        "metric": "ERP-P10", "status": "PARTIAL", "profile": "release",
        "scope": "protected QA stored snapshot loading, graph expansion and committed result storage",
        "items": 10000, "edges": 50000, "depth": 32, "warmup_runs": 2,
        "measured_runs": 30, "samples_ms": samples_ms, "p95_ms": p95_ms, "max_ms": max_ms,
        "timing_gate_ms": 5000, "timing_pass": max_ms <= 5000.0,
        "memory_status": "NOT_RUN", "http_status": "NOT_RUN", "ui_status": "NOT_RUN",
        "other_host_workload": "UNCONTROLLED", "input_sha256": input_sha256,
        "tenant_id": uuid::Uuid::from(tenant), "bom_id": bom_id, "pid": std::process::id(),
        "expected_sum_qty_micros": expected_total, "verified_rows": total_rows,
        "binary": std::env::current_exe()?.to_string_lossy(), "os": std::env::consts::OS,
        "completed_at": chrono::Utc::now().to_rfc3339()
    });
    std::fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "BOM_PERFORMANCE_REPORT={} p95_ms={p95_ms:.3} max_ms={max_ms:.3}",
        output.display()
    );
    assert!(
        max_ms <= 5000.0,
        "BOM complete-run timing gate failed; report retained"
    );
    Ok(())
}
