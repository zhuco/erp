//! Actual registered HTTP/Worker inventory-export acceptance on protected QA.
use super::{Result, isolated_pool};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_domain::quantity::parse_qty;
use erp_repo::db::Db;
use erp_repo::{
    export_jobs::{ExportJob, ExportJobsRepo},
    inventory::InventoryService,
    inventory_export_cursor::InventoryExportCursor,
    inventory_export_jobs::parse_spec,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use erp_types::TenantId;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    tenant: TenantId,
    actor: Uuid,
    other: Uuid,
    role: Uuid,
    warehouse: Uuid,
    denied: Uuid,
    sku: Uuid,
}

async fn fixture(pool: &PgPool) -> Result<Fixture> {
    let _ = base_path()?;
    let f = Fixture {
        tenant: TenantId(Uuid::new_v4()),
        actor: Uuid::new_v4(),
        other: Uuid::new_v4(),
        role: Uuid::new_v4(),
        warehouse: Uuid::new_v4(),
        denied: Uuid::new_v4(),
        sku: Uuid::new_v4(),
    };
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-INVENTORY-EXPORT')")
        .bind(Uuid::from(f.tenant))
        .execute(pool)
        .await?;
    for actor in [f.actor, f.other] {
        sqlx::query("insert into users(tenant_id,user_id,email,password_hash) values($1,$2,$3,'QA-synthetic-no-login')")
            .bind(Uuid::from(f.tenant)).bind(actor).bind(format!("{actor}@qa.invalid")).execute(pool).await?;
    }
    sqlx::query("insert into roles(tenant_id,role_id,name) values($1,$2,'Inventory exporter')")
        .bind(Uuid::from(f.tenant))
        .bind(f.role)
        .execute(pool)
        .await?;
    for perm in ["WMS.INVENTORY.READ", "OPS.EXPORT.READ", "OPS.EXPORT.WRITE"] {
        sqlx::query("insert into role_permissions(tenant_id,role_id,perm_code) values($1,$2,$3)")
            .bind(Uuid::from(f.tenant))
            .bind(f.role)
            .bind(perm)
            .execute(pool)
            .await?;
    }
    for actor in [f.actor, f.other] {
        sqlx::query("insert into user_roles(tenant_id,user_id,role_id) values($1,$2,$3)")
            .bind(Uuid::from(f.tenant))
            .bind(actor)
            .bind(f.role)
            .execute(pool)
            .await?;
    }
    for warehouse in [f.warehouse, f.denied] {
        sqlx::query(
            "insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Export QA',$3)",
        )
        .bind(Uuid::from(f.tenant))
        .bind(warehouse)
        .bind(warehouse.to_string())
        .execute(pool)
        .await?;
    }
    for actor in [f.actor, f.other] {
        sqlx::query(
            "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
        )
        .bind(Uuid::from(f.tenant))
        .bind(actor)
        .bind(f.warehouse)
        .execute(pool)
        .await?;
    }
    let product = Uuid::new_v4();
    sqlx::query("insert into products(tenant_id,product_id,name) values($1,$2,'Inventory export')")
        .bind(Uuid::from(f.tenant))
        .bind(product)
        .execute(pool)
        .await?;
    sqlx::query("insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'=1+1',$3)")
        .bind(Uuid::from(f.tenant))
        .bind(f.sku)
        .bind(product)
        .execute(pool)
        .await?;
    for warehouse in [f.warehouse, f.denied] {
        opening(pool, &f, warehouse, f.sku).await?;
    }
    eprintln!(
        "QA inventory export fixture tenant={} actor={} other={} warehouse={} denied={} sku={}",
        f.tenant, f.actor, f.other, f.warehouse, f.denied, f.sku
    );
    Ok(f)
}

async fn opening(pool: &PgPool, f: &Fixture, warehouse: Uuid, sku: Uuid) -> Result<()> {
    InventoryService::new(pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: warehouse,
                sku_id: sku,
            },
            on_hand: parse_qty("1.9")?,
            reserved: parse_qty("0.9")?,
            frozen: parse_qty("0.9")?,
            in_transit: parse_qty("2.3")?,
            reason_type: "QA_EXPORT_OPENING".into(),
            ref_type: Some("QA_EXPORT_OPENING".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    Ok(())
}

async fn response(
    f: &Fixture,
    actor: Uuid,
    path: &str,
    body: Option<Value>,
) -> Result<axum::response::Response> {
    let jwt = JwtService::new("synthetic-inventory-export-key".into());
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        actor.to_string(),
        f.tenant.to_string(),
        vec![
            "inventory.read".into(),
            "exports.read".into(),
            "exports.write".into(),
        ],
        false,
        now,
        now + 300,
    ))?;
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        Arc::new(jwt),
    ));
    Ok(app
        .oneshot(
            Request::builder()
                .uri(path)
                .method(if body.is_some() { "POST" } else { "GET" })
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))?,
        )
        .await?)
}

async fn http(
    f: &Fixture,
    actor: Uuid,
    path: &str,
    body: Option<Value>,
) -> Result<(StatusCode, Value)> {
    let response = response(f, actor, path, body).await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA inventory export HTTP tenant={} actor={actor} path={path} status={status} body={value}",
        f.tenant
    );
    Ok((status, value))
}

async fn admit(pool: &PgPool, f: &Fixture) -> Result<ExportJob> {
    let id = Uuid::new_v4();
    let (status, body) = http(
        f,
        f.actor,
        "/exports/jobs",
        Some(json!({"type":"INVENTORY_EXPORT","params":{"request_id":id}})),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["job_id"], id.to_string());
    Ok(ExportJobsRepo::new(pool)
        .get_job(f.tenant, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?)
}

async fn claim(pool: &PgPool, f: &Fixture, id: Uuid) -> Result<ExportJob> {
    // Simulate only this fixture's scheduler claim; never run the global queue.
    assert_eq!(sqlx::query("update export_jobs set status='RUNNING',attempts=attempts+1,progress=0 where tenant_id=$1 and job_id=$2 and status in ('PENDING','FAILED')")
        .bind(Uuid::from(f.tenant)).bind(id).execute(pool).await?.rows_affected(),1);
    Ok(ExportJobsRepo::new(pool)
        .get_job(f.tenant, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?)
}

fn base_path() -> Result<PathBuf> {
    let base = PathBuf::from(std::env::var("EXPORT_BASE_DIR").unwrap_or_else(|_| "exports".into()));
    let base = if base.is_absolute() {
        base
    } else {
        std::env::current_dir()?.join(base)
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| anyhow::anyhow!("workspace missing"))?
        .to_path_buf();
    anyhow::ensure!(
        base.starts_with(root),
        "QA export file destination outside workspace"
    );
    Ok(base)
}

fn path(job: &ExportJob) -> Result<PathBuf> {
    Ok(base_path()?
        .join(job.tenant_id.to_string())
        .join(format!("{}.csv", job.job_id)))
}

async fn run(pool: &PgPool, f: &Fixture, job: &ExportJob) -> Result<ExportJob> {
    let _ = path(job)?;
    let claimed = claim(pool, f, job.job_id).await?;
    erp_worker::jobs::inventory_export::execute_inventory_job(pool, &claimed, 1).await?;
    let done = ExportJobsRepo::new(pool)
        .get_job(f.tenant, job.job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?;
    assert_eq!(done.status, "DONE");
    eprintln!(
        "QA inventory export artifact job={} file={} params={}",
        done.job_id,
        path(&done)?.display(),
        done.params_json
    );
    Ok(done)
}

async fn source_facts(pool: &PgPool, f: &Fixture) -> Result<String> {
    Ok(sqlx::query_scalar("select jsonb_build_object('balances',(select jsonb_agg(to_jsonb(b) order by warehouse_id,sku_id) from inventory_balances b where tenant_id=$1),'ledgers',(select jsonb_agg(to_jsonb(l) order by ledger_id) from inventory_ledgers l where tenant_id=$1),'outbox',(select jsonb_agg(to_jsonb(o) order by outbox_id) from outbox_events o where tenant_id=$1))::text")
        .bind(Uuid::from(f.tenant)).fetch_one(pool).await?)
}

#[tokio::test]
async fn inventory_export_exact_csv_and_authorized_download() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let before = source_facts(&pool, &f).await?;
    let job = admit(&pool, &f).await?;
    let done = run(&pool, &f, &job).await?;
    let receipt = parse_spec(&done)?
        .artifact
        .ok_or_else(|| anyhow::anyhow!("missing receipt"))?;
    assert_eq!(receipt.rows, 1);
    let response = response(
        &f,
        f.actor,
        &format!("/exports/jobs/{}/file", job.job_id),
        None,
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["x-export-schema"], "2");
    assert_eq!(response.headers()["x-export-sha256"], receipt.sha256);
    let bytes = response.into_body().collect().await?.to_bytes();
    assert_eq!(hex::encode(Sha256::digest(&bytes)), receipt.sha256);
    assert_eq!(bytes.len() as u64, receipt.bytes);
    let mut csv = csv::Reader::from_reader(bytes.as_ref());
    assert_eq!(
        csv.headers()?.iter().collect::<Vec<_>>(),
        [
            "sku_id",
            "sku_code",
            "warehouse_id",
            "on_hand",
            "reserved",
            "available",
            "frozen",
            "in_transit",
            "quantity_scale",
            "snapshot_at"
        ]
    );
    let rows = csv.records().collect::<std::result::Result<Vec<_>, _>>()?;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(&row[1], "'=1+1");
    assert_eq!(&row[2], f.warehouse.to_string());
    for (column, expected) in [
        (3, "1.9"),
        (4, "0.9"),
        (5, "0.1"),
        (6, "0.9"),
        (7, "2.3"),
        (8, "6"),
    ] {
        assert_eq!(&row[column], expected);
    }
    assert_eq!(source_facts(&pool, &f).await?, before);
    for endpoint in [
        format!("/exports/jobs/{}", job.job_id),
        format!("/exports/jobs/{}/file", job.job_id),
    ] {
        assert_eq!(
            http(&f, f.other, &endpoint, None).await?.0,
            StatusCode::FORBIDDEN
        );
    }
    let (_, other) = http(&f, f.other, "/exports/jobs?type=INVENTORY_EXPORT", None).await?;
    assert_eq!(other["total"], 0);
    let (_, own) = http(&f, f.actor, "/exports/jobs?type=INVENTORY_EXPORT", None).await?;
    assert_eq!(own["total"], 1);
    Ok(())
}

#[tokio::test]
async fn inventory_export_request_identity_is_atomic_under_concurrent_retries() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4();
    let input = json!({"type":"INVENTORY_EXPORT","params":{"request_id":id}});
    let results = futures::future::join_all(
        (0..12).map(|_| http(&f, f.actor, "/exports/jobs", Some(input.clone()))),
    )
    .await;
    for result in results {
        let (status, body) = result?;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["job_id"], id.to_string());
    }
    let counts:(i64,i64)=sqlx::query_as("select (select count(*) from export_jobs where tenant_id=$1),(select count(*) from audit_logs where tenant_id=$1 and action='EXPORT.JOB.CREATE')")
        .bind(Uuid::from(f.tenant)).fetch_one(&pool).await?;
    assert_eq!(counts, (1, 1));
    assert_eq!(
        http(&f, f.other, "/exports/jobs", Some(input.clone()))
            .await?
            .0,
        StatusCode::CONFLICT
    );
    let mut changed = input;
    changed["params"]["query"] = json!({"available_min":"0.2"});
    assert_eq!(
        http(&f, f.actor, "/exports/jobs", Some(changed)).await?.0,
        StatusCode::CONFLICT
    );
    Ok(())
}

#[tokio::test]
async fn inventory_export_rejects_outside_scope_and_forged_internal_fields() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    for (params, expected) in [
        (
            json!({"request_id":Uuid::new_v4(),"query":{"warehouse_id":f.denied}}),
            StatusCode::FORBIDDEN,
        ),
        (
            json!({"request_id":Uuid::new_v4(),"actor_user_id":f.other}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"request_id":Uuid::new_v4(),"warehouse_ids":[f.denied]}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"request_id":Uuid::new_v4(),"query":{"available_min":"0.0000001"}}),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({"request_id":Uuid::new_v4(),"query":{"available_min":"2","available_max":"1"}}),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(
            http(
                &f,
                f.actor,
                "/exports/jobs",
                Some(json!({"type":"INVENTORY_EXPORT","params":params}))
            )
            .await?
            .0,
            expected
        );
    }
    let jobs: i64 = sqlx::query_scalar("select count(*) from export_jobs where tenant_id=$1")
        .bind(Uuid::from(f.tenant))
        .fetch_one(&pool)
        .await?;
    assert_eq!(jobs, 0);
    Ok(())
}

#[tokio::test]
async fn inventory_export_revoked_role_blocks_worker_and_download() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let done = run(&pool, &f, &admit(&pool, &f).await?).await?;
    let pending = admit(&pool, &f).await?;
    sqlx::query("delete from role_permissions where tenant_id=$1 and role_id=$2 and perm_code='WMS.INVENTORY.READ'")
        .bind(Uuid::from(f.tenant)).bind(f.role).execute(&pool).await?;
    let claimed = claim(&pool, &f, pending.job_id).await?;
    assert!(
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1)
            .await
            .is_err()
    );
    assert!(!path(&pending)?.exists());
    assert_eq!(
        http(
            &f,
            f.actor,
            &format!("/exports/jobs/{}/file", done.job_id),
            None
        )
        .await?
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, list) = http(&f, f.actor, "/exports/jobs?type=INVENTORY_EXPORT", None).await?;
    assert_eq!(list["total"], 0);
    assert_eq!(
        http(
            &f,
            f.actor,
            "/exports/jobs",
            Some(json!({"type":"INVENTORY_EXPORT","params":{"request_id":Uuid::new_v4()}}))
        )
        .await?
        .0,
        StatusCode::FORBIDDEN
    );
    Ok(())
}

#[tokio::test]
async fn inventory_export_all_grants_removed_never_expands_a_held_restricted_scope() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let done = run(&pool, &f, &admit(&pool, &f).await?).await?;
    let pending = admit(&pool, &f).await?;
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(Uuid::from(f.tenant))
        .bind(f.actor)
        .execute(&pool)
        .await?;
    let claimed = claim(&pool, &f, pending.job_id).await?;
    assert!(
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1)
            .await
            .is_err()
    );
    assert!(!path(&pending)?.exists());
    assert_eq!(
        http(
            &f,
            f.actor,
            &format!("/exports/jobs/{}/file", done.job_id),
            None
        )
        .await?
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http(&f, f.actor, "/exports/jobs?type=INVENTORY_EXPORT", None)
            .await?
            .1["total"],
        0
    );
    Ok(())
}

#[tokio::test]
async fn inventory_export_cursor_holds_exact_count_and_data_during_concurrent_writes() -> Result<()>
{
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.actor)
    .bind(f.denied)
    .execute(&pool)
    .await?;
    let job = admit(&pool, &f).await?;
    let spec = parse_spec(&job)?;
    let mut cursor = InventoryExportCursor::open(&pool, f.tenant, &spec).await?;
    assert_eq!(cursor.total(), 2);
    let first = cursor.fetch_next(1).await?;
    assert_eq!(first[0].quantities()?.on_hand, "1.9");
    for warehouse in [f.warehouse, f.denied] {
        InventoryService::new(&pool)
            .apply_precise(InventoryPreciseDelta {
                key: InventoryQuantityKey {
                    tenant_id: f.tenant,
                    warehouse_id: warehouse,
                    sku_id: f.sku,
                },
                on_hand: parse_qty("0.2")?,
                reserved: parse_qty("0")?,
                frozen: parse_qty("0")?,
                in_transit: parse_qty("0")?,
                reason_type: "QA_EXPORT_LATE".into(),
                ref_type: None,
                ref_id: Some(Uuid::new_v4().to_string()),
                occurred_at: Utc::now(),
            })
            .await?;
    }
    let rest = cursor.fetch_next(1000).await?;
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].quantities()?.on_hand, "1.9");
    assert!(cursor.fetch_next(1).await?.is_empty());
    cursor.finish().await?;
    let mut after = InventoryExportCursor::open(&pool, f.tenant, &spec).await?;
    for row in after.fetch_next(1000).await? {
        assert_eq!(row.quantities()?.on_hand, "2.1");
    }
    after.finish().await?;
    Ok(())
}

#[tokio::test]
async fn inventory_export_published_receipt_recovers_without_changing_snapshot() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let done = run(&pool, &f, &admit(&pool, &f).await?).await?;
    let bytes = tokio::fs::read(path(&done)?).await?;
    let params = done.params_json.clone();
    sqlx::query(
        "update export_jobs set status='FAILED',output_url=null where tenant_id=$1 and job_id=$2",
    )
    .bind(Uuid::from(f.tenant))
    .bind(done.job_id)
    .execute(&pool)
    .await?;
    let retry = run(&pool, &f, &done).await?;
    assert_eq!(retry.params_json, params);
    assert_eq!(tokio::fs::read(path(&retry)?).await?, bytes);
    assert_eq!(retry.attempts, 2);
    Ok(())
}

#[tokio::test]
async fn inventory_export_corrupt_same_length_file_never_completes_download_or_retry() -> Result<()>
{
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let done = run(&pool, &f, &admit(&pool, &f).await?).await?;
    let file = path(&done)?;
    let mut bytes = tokio::fs::read(&file).await?;
    bytes[0] = b'X';
    tokio::fs::write(&file, &bytes).await?;
    let response = response(
        &f,
        f.actor,
        &format!("/exports/jobs/{}/file", done.job_id),
        None,
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.into_body().collect().await.is_err(),
        "HTTP headers do not mean a verified complete file"
    );
    sqlx::query("update export_jobs set status='FAILED' where tenant_id=$1 and job_id=$2")
        .bind(Uuid::from(f.tenant))
        .bind(done.job_id)
        .execute(&pool)
        .await?;
    let claimed = claim(&pool, &f, done.job_id).await?;
    assert!(
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1)
            .await
            .is_err()
    );
    let failed = ExportJobsRepo::new(&pool)
        .get_job(f.tenant, done.job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?;
    assert_eq!(failed.status, "FAILED");
    assert!(failed.last_error.contains("hash mismatch"));
    assert_eq!(tokio::fs::read(file).await?, bytes);
    Ok(())
}

#[tokio::test]
async fn inventory_export_stale_attempt_cannot_finish_or_fail_a_new_attempt() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let job = admit(&pool, &f).await?;
    let stale = claim(&pool, &f, job.job_id).await?;
    sqlx::query("update export_jobs set attempts=attempts+1 where tenant_id=$1 and job_id=$2")
        .bind(Uuid::from(f.tenant))
        .bind(job.job_id)
        .execute(&pool)
        .await?;
    assert!(
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &stale, 1)
            .await
            .is_err()
    );
    let current = ExportJobsRepo::new(&pool)
        .get_job(f.tenant, job.job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?;
    assert_eq!(current.status, "RUNNING");
    assert_eq!(current.attempts, 2);
    assert!(!path(&current)?.exists());
    erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &current, 1).await?;
    Ok(())
}

#[tokio::test]
async fn inventory_export_unknown_stock_blocks_before_quantity_filter_and_file_creation()
-> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let unknown = Uuid::new_v4();
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-UNKNOWN-EXPORT',(select product_id from qa_parent))")
        .bind(Uuid::from(f.tenant))
        .bind(unknown)
        .execute(&pool)
        .await?;
    sqlx::query("insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand,reserved) values($1,$2,$3,1,0)")
        .bind(Uuid::from(f.tenant)).bind(f.warehouse).bind(unknown).execute(&pool).await?;
    let before = source_facts(&pool, &f).await?;
    let id = Uuid::new_v4();
    let (status,body)=http(&f,f.actor,"/exports/jobs",Some(json!({"type":"INVENTORY_EXPORT","params":{"request_id":id,"query":{"available_min":"999"}}}))).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let claimed = claim(&pool, &f, id).await?;
    assert!(
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1)
            .await
            .is_err()
    );
    assert!(!path(&claimed)?.exists());
    let failed = ExportJobsRepo::new(&pool)
        .get_job(f.tenant, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?;
    assert_eq!(failed.status, "FAILED");
    assert!(failed.last_error.contains("reconciliation"));
    assert_eq!(source_facts(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_export_new_grant_never_expands_previously_accepted_membership() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let job = admit(&pool, &f).await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(Uuid::from(f.tenant))
    .bind(f.actor)
    .bind(f.denied)
    .execute(&pool)
    .await?;
    let done = run(&pool, &f, &job).await?;
    let spec = parse_spec(&done)?;
    assert_eq!(spec.warehouse_ids, [f.warehouse]);
    assert_eq!(
        spec.artifact
            .ok_or_else(|| anyhow::anyhow!("receipt missing"))?
            .rows,
        1
    );
    Ok(())
}

#[tokio::test]
async fn inventory_export_oversized_code_fails_without_publishing_or_mutating_stock() -> Result<()>
{
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let job = admit(&pool, &f).await?;
    sqlx::query("update skus set sku_code=$3 where tenant_id=$1 and sku_id=$2")
        .bind(Uuid::from(f.tenant))
        .bind(f.sku)
        .bind("X".repeat(1025))
        .execute(&pool)
        .await?;
    let before = source_facts(&pool, &f).await?;
    let claimed = claim(&pool, &f, job.job_id).await?;
    assert!(
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1)
            .await
            .is_err()
    );
    assert!(!path(&job)?.exists());
    assert_eq!(source_facts(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn inventory_export_two_executors_publish_only_one_complete_artifact() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let job = admit(&pool, &f).await?;
    let claimed = claim(&pool, &f, job.job_id).await?;
    let _ = path(&job)?;
    let (a, b) = tokio::join!(
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1),
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1)
    );
    assert!(
        a.is_ok() ^ b.is_ok(),
        "exactly one executor must publish: {a:?}, {b:?}"
    );
    let done = ExportJobsRepo::new(&pool)
        .get_job(f.tenant, job.job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?;
    assert_eq!(done.status, "DONE");
    let receipt = parse_spec(&done)?
        .artifact
        .ok_or_else(|| anyhow::anyhow!("receipt missing"))?;
    assert_eq!(
        hex::encode(Sha256::digest(tokio::fs::read(path(&done)?).await?)),
        receipt.sha256
    );
    Ok(())
}

#[tokio::test]
async fn inventory_export_invalid_worker_page_size_records_a_recoverable_failure() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let job = admit(&pool, &f).await?;
    let claimed = claim(&pool, &f, job.job_id).await?;
    assert!(
        erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1001)
            .await
            .is_err()
    );
    let failed = ExportJobsRepo::new(&pool)
        .get_job(f.tenant, job.job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?;
    assert_eq!(failed.status, "FAILED");
    assert!(failed.last_error.contains("page size"));
    assert!(!path(&failed)?.exists());
    run(&pool, &f, &failed).await?;
    Ok(())
}

#[tokio::test]
async fn inventory_export_legacy_ownerless_job_is_hidden_and_never_downloaded() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4();
    sqlx::query("insert into export_jobs(tenant_id,job_id,job_type,status) values($1,$2,'INVENTORY_EXPORT','DONE')")
        .bind(Uuid::from(f.tenant)).bind(id).execute(&pool).await?;
    for endpoint in [
        format!("/exports/jobs/{id}"),
        format!("/exports/jobs/{id}/file"),
    ] {
        assert_eq!(
            http(&f, f.actor, &endpoint, None).await?.0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        http(&f, f.actor, "/exports/jobs?type=INVENTORY_EXPORT", None)
            .await?
            .1["total"],
        0
    );
    Ok(())
}

#[tokio::test]
async fn inventory_export_empty_exact_range_produces_only_header_and_valid_receipt() -> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let id = Uuid::new_v4();
    let (status,body)=http(&f,f.actor,"/exports/jobs",Some(json!({"type":"INVENTORY_EXPORT","params":{"request_id":id,"query":{"available_min":"0.100001"}}}))).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let job = ExportJobsRepo::new(&pool)
        .get_job(f.tenant, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?;
    let done = run(&pool, &f, &job).await?;
    assert_eq!(
        parse_spec(&done)?
            .artifact
            .ok_or_else(|| anyhow::anyhow!("receipt missing"))?
            .rows,
        0
    );
    let bytes = tokio::fs::read(path(&done)?).await?;
    let mut csv = csv::Reader::from_reader(bytes.as_slice());
    assert_eq!(csv.records().count(), 0);
    Ok(())
}

#[tokio::test]
async fn inventory_export_multiple_fetch_pages_and_download_chunks_keep_all_exact_rows()
-> Result<()> {
    let pool = isolated_pool().await?;
    let f = fixture(&pool).await?;
    for index in 0..1001 {
        let sku = Uuid::new_v4();
        sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,$3,(select product_id from qa_parent))")
            .bind(Uuid::from(f.tenant))
            .bind(sku)
            .bind(format!("QA-EXPORT-LARGE-{index:04}"))
            .execute(&pool)
            .await?;
        opening(&pool, &f, f.warehouse, sku).await?;
    }
    let before = source_facts(&pool, &f).await?;
    let job = admit(&pool, &f).await?;
    let claimed = claim(&pool, &f, job.job_id).await?;
    erp_worker::jobs::inventory_export::execute_inventory_job(&pool, &claimed, 1000).await?;
    let done = ExportJobsRepo::new(&pool)
        .get_job(f.tenant, job.job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("job missing"))?;
    let receipt = parse_spec(&done)?
        .artifact
        .ok_or_else(|| anyhow::anyhow!("receipt missing"))?;
    assert_eq!(receipt.rows, 1002);
    let response = response(
        &f,
        f.actor,
        &format!("/exports/jobs/{}/file", job.job_id),
        None,
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = std::pin::pin!(response.into_body());
    let mut hash = Sha256::new();
    let mut chunks = 0;
    let mut bytes = 0;
    while let Some(frame) = body.frame().await {
        let frame = frame?;
        if let Some(data) = frame.data_ref() {
            assert!(data.len() <= 65536);
            hash.update(data);
            bytes += data.len();
            chunks += 1;
        }
    }
    assert!(chunks >= 3);
    assert_eq!(bytes as u64, receipt.bytes);
    assert_eq!(hex::encode(hash.finalize()), receipt.sha256);
    let raw = tokio::fs::read(path(&done)?).await?;
    let mut csv = csv::Reader::from_reader(raw.as_slice());
    let mut ids = std::collections::HashSet::new();
    for row in csv.records() {
        let row = row?;
        assert!(ids.insert(row[0].to_owned()));
        assert_eq!(&row[5], "0.1");
        assert_eq!(&row[6], "0.9");
    }
    assert_eq!(ids.len(), 1002);
    assert_eq!(source_facts(&pool, &f).await?, before);
    eprintln!(
        "QA inventory export large job={} file={} rows={} bytes={} download_chunks={chunks}",
        done.job_id,
        path(&done)?.display(),
        receipt.rows,
        receipt.bytes
    );
    Ok(())
}

#[tokio::test]
async fn inventory_export_requires_source_permission() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-INVENTORY-EXPORT')")
        .bind(tenant)
        .execute(&pool)
        .await?;
    let jwt = JwtService::new("synthetic-inventory-export-key".into());
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        Uuid::new_v4().to_string(),
        tenant.to_string(),
        vec!["exports.write".into()],
        false,
        now,
        now + 300,
    ))?;
    let app = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        Arc::new(jwt),
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/exports/jobs")
                .method("POST")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"type":"INVENTORY_EXPORT","params":{"request_id":Uuid::new_v4()}})
                        .to_string(),
                ))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!("QA inventory export permission tenant={tenant} status={status} body={body}");
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "inventory source permission required: {body}"
    );
    Ok(())
}
