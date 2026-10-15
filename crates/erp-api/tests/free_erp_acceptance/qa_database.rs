//! Explicit local QA admission. Complete-schema tests also bind the installed SQL hashes.
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, postgres::PgConnectOptions};
use std::{collections::BTreeMap, path::Path, str::FromStr};
use tokio::sync::OnceCell;

const COMPLETE: &str = "erp_free_ui_acceptance";
const LEGACY: &str = "erp_shared_contract_test";

/// Global capacity tests own a real queue table, not another fixture's live rows.
pub(super) async fn isolated_admission_pool(pool: &PgPool) -> Result<PgPool> {
    verify(pool).await?;
    let schema = format!("qa_admission_{}", uuid::Uuid::new_v4().simple());
    sqlx::query(&format!("create schema {schema}"))
        .execute(pool)
        .await?;
    sqlx::query(&format!(
        "create table {schema}.work_admissions (like public.work_admissions including all)"
    ))
    .execute(pool)
    .await?;
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    if !approved_url(&url)? {
        bail!("isolated queue requires the same fixed QA database");
    }
    let separator = if url.contains('?') { '&' } else { '?' };
    let url = format!("{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog");
    let scoped = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await?;
    eprintln!("QA isolated_admission_schema={schema}");
    Ok(scoped)
}

pub(super) fn approved_url(url: &str) -> Result<bool> {
    let options = PgConnectOptions::from_str(url).context("invalid QA database URL")?;
    Ok(options.get_host() == "127.0.0.1"
        && options.get_port() == 15438
        && options.get_username() == "erp_grid_test"
        && matches!(options.get_database(), Some(COMPLETE | LEGACY)))
}

pub(super) async fn verify(pool: &PgPool) -> Result<()> {
    let (database, role, port, address): (String, String, i32, String) = sqlx::query_as(
        "select current_database(),current_user,inet_server_port(),host(inet_server_addr())",
    )
    .fetch_one(pool)
    .await?;
    if !matches!(database.as_str(), COMPLETE | LEGACY)
        || role != "erp_grid_test"
        || port != 15438
        || address != "127.0.0.1"
    {
        bail!("refusing a database outside the protected QA identity");
    }
    if database == COMPLETE {
        let marker: Option<String> = sqlx::query_scalar(
            "select shobj_description(oid,'pg_database') from pg_database where datname=current_database()",
        )
        .fetch_one(pool)
        .await?;
        if marker.as_deref() != Some("ERP owned free UI QA baseline 2026-10-08; no production") {
            bail!("complete QA database ownership marker mismatch");
        }
        static VALIDATED: OnceCell<()> = OnceCell::const_new();
        VALIDATED
            .get_or_try_init(|| async {
                let installed: Vec<(String, String)> = sqlx::query_as(
                    "select filename,sha256 from public.qa_acceptance_migration_progress order by filename",
                )
                .fetch_all(pool)
                .await?;
                let mut expected = BTreeMap::new();
                for entry in std::fs::read_dir(
                    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations"),
                )? {
                    let file = entry?.path();
                    if file.is_file() && file.extension().is_some_and(|value| value == "sql") {
                        let name = file
                            .file_name()
                            .and_then(|name| name.to_str())
                            .context("invalid migration filename")?
                            .to_owned();
                        expected.insert(name, hex::encode(Sha256::digest(std::fs::read(file)?)));
                    }
                }
                if expected.is_empty() || installed.into_iter().collect::<BTreeMap<_, _>>() != expected {
                    bail!("complete QA installed SQL hashes differ from current migration files");
                }
                Ok::<(), anyhow::Error>(())
            })
            .await?;
    }
    Ok(())
}

#[test]
fn admission_rejects_other_names_hosts_ports_and_roles() -> Result<()> {
    for database in [LEGACY, COMPLETE] {
        assert!(approved_url(&format!(
            "postgresql://erp_grid_test@127.0.0.1:15438/{database}"
        ))?);
    }
    for url in [
        "postgresql://erp_grid_test@127.0.0.1:5432/erp_free_ui_acceptance",
        "postgresql://erp_grid_test@localhost:15438/erp_free_ui_acceptance",
        "postgresql://erp_grid_test@127.0.0.1:15438/erp_grid_acceptance",
        "postgresql://erp_grid_test@127.0.0.1:15438/erp",
        "postgresql://another_role@127.0.0.1:15438/erp_free_ui_acceptance",
    ] {
        assert!(!approved_url(url)?);
    }
    Ok(())
}
