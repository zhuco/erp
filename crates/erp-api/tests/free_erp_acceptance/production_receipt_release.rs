//! Real production receipt HTTP admission, exact stock sources and original outcomes.
use super::{
    Result,
    inventory_commands::{Fixture, fixture},
    isolated_pool,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{AppState, build_app};
use erp_repo::db::Db;
use erp_repo::shared_contract::{SharedContractRepo, SharedWrite};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{
    sync::Barrier,
    task::JoinSet,
    time::{Duration, Instant},
};
use tower::ServiceExt;
use uuid::Uuid;

pub(super) struct Receipt {
    pub(super) f: Fixture,
    pub(super) work_order: Uuid,
    pub(super) receipt: Uuid,
    report: Uuid,
    inspection: Uuid,
    case: String,
}

pub(super) async fn http(f: &Fixture, path: &str, body: Value) -> Result<(StatusCode, Value)> {
    let jwt = f.login.jwt.clone();
    let token = f.login.token.clone();
    let database = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let joiner = if database.contains('?') { '&' } else { '?' };
    let database = format!("{database}{joiner}application_name=qa_release_{}", f.tenant);
    let response = build_app(AppState::new(Db::connect_lazy(&database)?, jwt))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(if path.starts_with("quality/") {
                    format!("/v1/tenants/{}/{path}", f.tenant)
                } else {
                    format!("/v1/tenants/{}/manufacturing/{path}", f.tenant)
                })
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let response = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA production_release_http tenant={} path={path} input={body} status={} response={response}",
        f.tenant,
        status.as_u16()
    );
    Ok((status, response))
}

pub(super) async fn setup(pool: &PgPool, case: &str) -> Result<Receipt> {
    let f = fixture(pool).await?;
    sqlx::query("update tenants set name='QA-ERP-PRODUCTION-RELEASE' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    let work_order = Uuid::new_v4();
    let report = Uuid::new_v4();
    let inspection = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-RELEASE','QA-P','RELEASED',10000000)")
        .bind(f.tenant.0).bind(work_order).execute(pool).await?;
    sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id) values($1,$2,$3,'FINAL',10000000,0,0,1,'QA-WORKER','QA-LOGIN','QA-SESSION')")
        .bind(f.tenant.0).bind(report).bind(work_order).execute(pool).await?;
    let (status, body) = http(
        &f,
        &format!("work-orders/{work_order}/receipts"),
        json!({"request_id":Uuid::new_v4().to_string(),"qty":"1.25","status":"HOLD",
            "report_id":report,"batch_code":"QA-PRODUCTION","inspection_id":inspection,
            "warehouse_id":f.warehouse,"sku_id":f.sku}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "receipt fixture: {body}");
    let receipt = Uuid::parse_str(
        body["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing receipt effect"))?,
    )?;
    eprintln!(
        "QA production_release_fixture tenant={} case={case} actor={} warehouse={} sku={} work_order={work_order} receipt={receipt} report={report} inspection={inspection}",
        f.tenant, f.actor, f.warehouse, f.sku
    );
    Ok(Receipt {
        f,
        work_order,
        receipt,
        report,
        inspection,
        case: case.into(),
    })
}

pub(super) async fn release(
    r: &Receipt,
    request: &str,
    work_order: Uuid,
) -> Result<(StatusCode, Value)> {
    http(
        &r.f,
        &format!("work-orders/{work_order}/receipts/{}/release", r.receipt),
        json!({"request_id":request}),
    )
    .await
}

pub(super) async fn facts(pool: &PgPool, r: &Receipt) -> Result<Value> {
    let mut result = serde_json::Map::new();
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "inventory_quantity_activations",
        "inventory_quantity_source_maps",
        "shared_stock_ledgers",
        "mfg_qty_ledgers",
        "mfg_receipts",
        "mfg_reports",
        "mfg_inspections",
        "mfg_quality_items",
        "shared_inspection_freezes",
        "shared_inspection_freeze_lines",
        "shared_inspection_source_bindings",
        "shared_inspection_templates",
        "shared_inspection_template_lines",
        "shared_sales_revisions",
        "shared_sales_lines",
        "mfg_quality_releases",
        "mfg_quality_release_proofs",
        "command_receipts",
        "operation_logs",
        "audit_logs",
        "outbox_events",
        "mfg_receipt_release_commands",
        "mfg_receipt_release_effects",
    ] {
        let exists: bool = sqlx::query_scalar("select to_regclass($1) is not null")
            .bind(format!("public.{table}"))
            .fetch_one(pool)
            .await?;
        let value = if exists {
            sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from public.{table} x where tenant_id=$1"))
                .bind(r.f.tenant.0).fetch_one(pool).await?
        } else {
            Value::Null
        };
        result.insert(table.into(), value);
    }
    Ok(Value::Object(result))
}

pub(super) async fn rejected(
    pool: &PgPool,
    r: &Receipt,
    work_order: Uuid,
    expected: StatusCode,
) -> Result<()> {
    let before = facts(pool, r).await?;
    let id = Uuid::new_v4().to_string();
    let (status, body) = release(r, &id, work_order).await?;
    let after = facts(pool, r).await?;
    eprintln!(
        "QA production_release_rejected tenant={} case={} request_id={id} expected={} actual={} unchanged={}",
        r.f.tenant,
        r.case,
        expected.as_u16(),
        status.as_u16(),
        before == after
    );
    assert_eq!(status, expected, "{}: {body}", r.case);
    assert_eq!(
        after, before,
        "{} refusal must leave every effect unchanged",
        r.case
    );
    Ok(())
}

async fn reading(
    pool: &PgPool,
    r: &Receipt,
    required: bool,
    present: bool,
    passed: bool,
    value: i64,
) -> Result<()> {
    sqlx::query("insert into mfg_quality_items(tenant_id,item_id,work_order_id,batch_code,item_code,required,min_micros,max_micros,reading_micros,reading_present,passed,inspector) values($1,$2,$3,'QA-PRODUCTION','DIMENSION',$4,1,100,$5,$6,$7,'QA-INSPECTOR')")
        .bind(r.f.tenant.0).bind(Uuid::new_v4()).bind(r.work_order).bind(required)
        .bind(value).bind(present).bind(passed).execute(pool).await?;
    Ok(())
}

#[tokio::test]
async fn production_release_open_required_quality_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "open-required").await?;
    reading(&pool, &r, true, false, false, 0).await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_release_forged_pass_outside_bounds_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "forged-pass").await?;
    reading(&pool, &r, true, true, true, 101).await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_release_wrong_work_order_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "wrong-work-order").await?;
    let wrong = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-OTHER','QA-P','RELEASED',10000000)")
        .bind(r.f.tenant.0).bind(wrong).execute(&pool).await?;
    rejected(&pool, &r, wrong, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_release_outside_warehouse_scope_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "warehouse-scope").await?;
    let other = Uuid::new_v4();
    sqlx::query(
        "insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'QA Other',$3)",
    )
    .bind(r.f.tenant.0)
    .bind(other)
    .bind(other.to_string())
    .execute(&pool)
    .await?;
    sqlx::query(
        "update user_warehouse_access set warehouse_id=$3 where tenant_id=$1 and user_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(r.f.actor)
    .bind(other)
    .execute(&pool)
    .await?;
    rejected(&pool, &r, r.work_order, StatusCode::FORBIDDEN).await
}

#[tokio::test]
async fn production_release_mismatched_ledger_source_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "source-quantity").await?;
    sqlx::query("update mfg_qty_ledgers set delta_micros=2000000 where tenant_id=$1 and move_id=$2 and reason_type='MFG_RECEIPT'")
        .bind(r.f.tenant.0).bind(r.receipt).execute(&pool).await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_release_missing_inspection_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "missing-inspection").await?;
    sqlx::query("update mfg_receipts set inspection_id=null where tenant_id=$1 and receipt_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.receipt)
        .execute(&pool)
        .await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_release_held_available_corruption_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "held-available").await?;
    sqlx::query(
        "update mfg_receipts set available_micros=1250000 where tenant_id=$1 and receipt_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(r.receipt)
    .execute(&pool)
    .await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_release_original_retry_keeps_original_total() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "original-result").await?;
    let id = Uuid::new_v4().to_string();
    let (first_status, first) = release(&r, &id, r.work_order).await?;
    assert_eq!(first_status, StatusCode::OK, "{first}");
    let (status, second) = http(
        &r.f,
        &format!("work-orders/{}/receipts", r.work_order),
        json!({"request_id":Uuid::new_v4().to_string(),"qty":"0.5","status":"RELEASED",
            "report_id":r.report,"batch_code":"QA-PRODUCTION-SECOND","inspection_id":Uuid::new_v4(),
            "warehouse_id":r.f.warehouse,"sku_id":r.f.sku}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{second}");
    let before = facts(&pool, &r).await?;
    let (retry_status, retry) = release(&r, &id, r.work_order).await?;
    assert_eq!(retry_status, StatusCode::OK, "{retry}");
    assert_eq!(
        retry["available_micros"], first["available_micros"],
        "later receipts cannot replace original result"
    );
    assert_eq!(retry["replay"], true);
    assert_eq!(
        facts(&pool, &r).await?,
        before,
        "retry cannot create effects"
    );
    Ok(())
}

#[tokio::test]
async fn production_release_valid_fractional_has_trusted_actor_and_one_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "valid-fractional").await?;
    reading(&pool, &r, true, true, true, 50).await?;
    let id = Uuid::new_v4().to_string();
    let (status, response) = release(&r, &id, r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{response}");
    let (hand, frozen): (i64,i64) = sqlx::query_as("select on_hand_micros,frozen_micros from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(r.f.tenant.0).bind(r.f.warehouse).bind(r.f.sku).fetch_one(&pool).await?;
    assert_eq!((hand, frozen), (6_150_000, 900_000));
    let (operations,audits):(i64,i64)=sqlx::query_as("select (select count(*) from operation_logs where tenant_id=$1 and action='MFG_RECEIPT_RELEASE' and actor_type='USER' and actor_user_id=$2 and ref_id=$3), (select count(*) from audit_logs where tenant_id=$1 and action='mfg.receipt.release' and user_id=$2 and entity_id=$3)")
        .bind(r.f.tenant.0).bind(r.f.actor).bind(r.receipt.to_string()).fetch_one(&pool).await?;
    eprintln!(
        "QA production_release_valid tenant={} request_id={id} hand={hand} frozen={frozen} operations={operations} audits={audits} inspection={}",
        r.f.tenant, r.inspection
    );
    assert_eq!(
        (operations, audits),
        (1, 1),
        "stock release requires trusted mandatory records"
    );
    let before = facts(&pool, &r).await?;
    let (status, replay) = release(&r, &id, r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replay"], true);
    assert_eq!(facts(&pool, &r).await?, before);
    Ok(())
}

async fn freeze(pool: &PgPool, r: &Receipt, batch: &str) -> Result<Uuid> {
    let repo = SharedContractRepo::new(pool);
    let template_request = Uuid::new_v4().to_string();
    let (_, template, _) = repo
        .save_inspection_template(
            r.f.tenant,
            &template_request,
            &template_request,
            "QA-RELEASE-STANDARD",
            "INCOMING",
            &[erp_domain::manufacturing::FrozenInspectionLine {
                item_code: "DIMENSION".into(),
                required: true,
                min_micros: 1,
                max_micros: 100,
                sample_count: 1,
            }],
        )
        .await??;
    if batch.is_empty() {
        // The legacy global copy is a separate owned fixture, never an edit to a new binding.
        let id = Uuid::new_v4();
        let mut tx = pool.begin().await?;
        sqlx::query("insert into shared_inspection_freezes(tenant_id,freeze_id,template_id,version_no,source_kind,source_id,item_code,batch_code) values($1,$2,$3,1,'WORK_ORDER',$4,'QA-P','')")
            .bind(r.f.tenant.0).bind(id).bind(template).bind(r.work_order).execute(&mut *tx).await?;
        sqlx::query("insert into shared_inspection_freeze_lines(tenant_id,freeze_id,item_code,required,min_micros,max_micros,sample_count) select tenant_id,$2,item_code,required,min_micros,max_micros,sample_count from shared_inspection_template_lines where tenant_id=$1 and template_id=$3")
            .bind(r.f.tenant.0).bind(id).bind(template).execute(&mut *tx).await?;
        tx.commit().await?;
        return Ok(id);
    }
    let freeze_request = Uuid::new_v4().to_string();
    let stored = repo
        .freeze_inspection_template(
            r.f.tenant,
            &freeze_request,
            &freeze_request,
            template,
            "WORK_ORDER",
            r.work_order,
            "QA-P",
            batch,
        )
        .await??;
    let id = match stored {
        SharedWrite::Stored(c) | SharedWrite::Replay(c) => Uuid::parse_str(&c.effect_id)?,
    };
    Ok(id)
}

#[tokio::test]
async fn production_release_global_frozen_requirement_blocks_missing_reading() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "global-frozen-open").await?;
    freeze(&pool, &r, "").await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_release_actual_frozen_reading_uses_original_limits() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "frozen-passed").await?;
    let frozen = freeze(&pool, &r, "QA-PRODUCTION").await?;
    let reading_request = Uuid::new_v4().to_string();
    SharedContractRepo::new(&pool)
        .save_frozen_inspection_reading(
            r.f.tenant,
            &reading_request,
            &reading_request,
            frozen,
            "DIMENSION",
            Some(50),
            "QA-INSPECTOR",
            "QA-REVIEWER",
        )
        .await??;
    let id = Uuid::new_v4().to_string();
    let (status, body) = release(&r, &id, r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let proof: Value = sqlx::query_scalar(
        "select snapshot from mfg_receipt_release_commands where tenant_id=$1 and request_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(proof["quality"]["required_item_count"], 1);
    assert_eq!(proof["quality"]["frozen"][0]["line"]["max_micros"], 100);
    Ok(())
}

#[tokio::test]
async fn production_release_unknown_physical_balance_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let mut r = setup(&pool, "unknown-physical").await?;
    // Build a new legacy key with unreconciled balance history. Existing proven
    // ledgers are immutable; the earlier attempted NULL rewrite correctly failed.
    let warehouse = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let receipt = Uuid::new_v4();
    let inspection = Uuid::new_v4();
    let quantity_ledger = Uuid::new_v4();
    let stock = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'QA Legacy Receipt',$3)")
        .bind(r.f.tenant.0).bind(warehouse).bind(warehouse.to_string()).execute(&pool).await?;
    super::masters::sku(&pool, r.f.tenant.0, sku, "QA-LEGACY-RECEIPT").await?;
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(r.f.tenant.0)
    .bind(r.f.actor)
    .bind(warehouse)
    .execute(&pool)
    .await?;
    sqlx::query("insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand,frozen) values($1,$2,$3,1,0)")
        .bind(r.f.tenant.0).bind(warehouse).bind(sku).execute(&pool).await?;
    for (kind, reference) in [
        ("shared_stock_ledger", stock),
        ("mfg_material_move", receipt),
    ] {
        sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,ref_type,ref_id,occurred_at) values($1,$2,$3,1,0,1,0,1250000,0,1250000,0,'MFG_RECEIPT',$4,$5,now())")
            .bind(r.f.tenant.0).bind(warehouse).bind(sku).bind(kind).bind(reference.to_string()).execute(&pool).await?;
    }
    sqlx::query("insert into mfg_qty_ledgers(tenant_id,ledger_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,$5,1250000,'MFG_RECEIPT')")
        .bind(r.f.tenant.0).bind(quantity_ledger).bind(warehouse).bind(sku).bind(receipt).execute(&pool).await?;
    sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,'QA-PRODUCTION','HOLD','ON_HAND',1250000,$6,'QA-LEGACY-RECEIPT')")
        .bind(r.f.tenant.0).bind(stock).bind(warehouse).bind(Uuid::nil()).bind(sku).bind(receipt.to_string()).execute(&pool).await?;
    sqlx::query("insert into mfg_inspections(tenant_id,inspection_id,execution_id,result,good_micros,rejected_micros) values($1,$2,$3,'HOLD',1250000,0)")
        .bind(r.f.tenant.0).bind(inspection).bind(r.work_order).execute(&pool).await?;
    sqlx::query("insert into mfg_receipts(tenant_id,receipt_id,work_order_id,qty_micros,status,available_micros,report_id,batch_code,inspection_id,ledger_id) values($1,$2,$3,1250000,'HOLD',0,$4,'QA-PRODUCTION',$5,$6)")
        .bind(r.f.tenant.0).bind(receipt).bind(r.work_order).bind(r.report).bind(inspection).bind(quantity_ledger).execute(&pool).await?;
    r.f.warehouse = warehouse;
    r.f.sku = sku;
    r.receipt = receipt;
    r.inspection = inspection;
    eprintln!(
        "QA production_release_fixture tenant={} case=unknown-physical actor={} warehouse={} sku={} work_order={} receipt={} report={} inspection={}",
        r.f.tenant,
        r.f.actor,
        r.f.warehouse,
        r.f.sku,
        r.work_order,
        r.receipt,
        r.report,
        r.inspection
    );
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

#[tokio::test]
async fn production_release_report_reversal_removes_source_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "report-reversed").await?;
    sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id,reverses_report_id) values($1,$2,$3,'FINAL',-10000000,0,0,1,'QA-WORKER','QA-LOGIN','QA-SESSION',$4)")
        .bind(r.f.tenant.0).bind(Uuid::new_v4()).bind(r.work_order).bind(r.report).execute(&pool).await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

async fn inspection_budget(pool: &PgPool, r: &Receipt, count: i32) -> Result<()> {
    sqlx::query("insert into mfg_quality_items(tenant_id,item_id,work_order_id,batch_code,item_code,required,min_micros,max_micros,reading_micros,reading_present,passed,inspector) select $1,gen_random_uuid(),$2,'QA-PRODUCTION','OPTIONAL-'||n,false,1,100,0,false,false,'' from generate_series(1,$3) n")
        .bind(r.f.tenant.0).bind(r.work_order).bind(count).execute(pool).await?;
    Ok(())
}

#[tokio::test]
async fn production_release_one_thousand_inspection_rows_are_complete() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "inspection-1000").await?;
    inspection_budget(&pool, &r, 1000).await?;
    let id = Uuid::new_v4().to_string();
    let (status, body) = release(&r, &id, r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let count:i32=sqlx::query_scalar("select jsonb_array_length(snapshot#>'{quality,readings}') from mfg_receipt_release_commands where tenant_id=$1 and request_id=$2")
        .bind(r.f.tenant.0).bind(id).fetch_one(&pool).await?;
    assert_eq!(
        count, 1000,
        "complete original quality snapshot, not a truncated pass"
    );
    Ok(())
}

#[tokio::test]
async fn production_release_one_thousand_one_inspection_rows_are_refused() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "inspection-1001").await?;
    inspection_budget(&pool, &r, 1001).await?;
    rejected(&pool, &r, r.work_order, StatusCode::CONFLICT).await
}

async fn fault(table: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, &format!("fault-{table}")).await?;
    let name = format!("qa_prod_release_{}", Uuid::new_v4().simple());
    let sequence = format!("{name}_hit");
    let trigger = format!("{name}_trigger");
    sqlx::query(&format!("create sequence {sequence}"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then perform nextval('{sequence}'); raise exception 'QA_PRODUCTION_REQUIRED_INSERT_FAILURE'; end if; return new; end $$",r.f.tenant))
        .execute(&pool).await?;
    sqlx::query(&format!(
        "create trigger {trigger} before insert on {table} for each row execute function {name}()"
    ))
    .execute(&pool)
    .await?;
    let before = facts(&pool, &r).await?;
    let id = Uuid::new_v4().to_string();
    let result = release(&r, &id, r.work_order).await;
    let actual_marker: bool = sqlx::query_scalar(&format!("select is_called from {sequence}"))
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
    let (status, body) = result?;
    let unchanged = facts(&pool, &r).await? == before;
    eprintln!(
        "QA production_release_fault tenant={} table={table} request_id={id} actual_marker={actual_marker} unchanged={unchanged}",
        r.f.tenant
    );
    assert!(actual_marker, "real required INSERT must reach the fault");
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert!(
        unchanged,
        "all stock, quality, original command and logs roll back"
    );
    let (status, body) = release(&r, &id, r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = facts(&pool, &r).await?;
    let (status, body) = release(&r, &id, r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["replay"], true);
    assert_eq!(facts(&pool, &r).await?, before);
    Ok(())
}

#[tokio::test]
async fn production_release_operation_insert_failure_is_atomic() -> Result<()> {
    fault("operation_logs").await
}
#[tokio::test]
async fn production_release_audit_insert_failure_is_atomic() -> Result<()> {
    fault("audit_logs").await
}

pub(super) async fn blocker_count(
    pool: &PgPool,
    r: &Receipt,
    gate: i32,
    expected: i64,
) -> Result<i64> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let count:i64=sqlx::query_scalar("with recursive waiting as (select pid from pg_stat_activity where application_name=$1 and wait_event_type='Lock'), chain(pid,blocker,path) as (select w.pid,b,array[w.pid,b] from waiting w cross join lateral unnest(pg_blocking_pids(w.pid)) b union all select c.pid,b,c.path||b from chain c cross join lateral unnest(pg_blocking_pids(c.blocker)) b where not b=any(c.path) and cardinality(c.path)<32) select count(distinct pid) from chain where blocker=$2")
            .bind(format!("qa_release_{}",r.f.tenant)).bind(gate).fetch_one(pool).await?;
        if count >= expected || Instant::now() >= deadline {
            return Ok(count);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn concurrency(same: bool) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = Arc::new(
        setup(
            &pool,
            if same {
                "twenty-original"
            } else {
                "twenty-competing"
            },
        )
        .await?,
    );
    let mut gate = pool.begin().await?;
    let gate_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await?;
    sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
        .bind(r.f.tenant.0).bind(r.work_order).execute(&mut *gate).await?;
    let original = Uuid::new_v4().to_string();
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let r = Arc::clone(&r);
        let barrier = Arc::clone(&barrier);
        let id = if same {
            original.clone()
        } else {
            Uuid::new_v4().to_string()
        };
        tasks.spawn(async move {
            barrier.wait().await;
            release(&r, &id, r.work_order).await
        });
    }
    let waiting = blocker_count(&independent, &r, gate_pid, 20).await;
    gate.rollback().await?;
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result);
    }
    let actual_waiting = waiting?;
    let responses: Vec<_> = results
        .into_iter()
        .map(|result| result.map_err(anyhow::Error::from).and_then(|value| value))
        .collect::<Result<_>>()?;
    let statuses: Vec<_> = responses
        .iter()
        .map(|(status, _)| status.as_u16())
        .collect();
    let commands: i64 =
        sqlx::query_scalar("select count(*) from mfg_receipt_release_commands where tenant_id=$1")
            .bind(r.f.tenant.0)
            .fetch_one(&independent)
            .await?;
    eprintln!(
        "QA production_release_concurrency tenant={} same={same} actual_waiting={actual_waiting} results={} commands={commands} statuses={statuses:?}",
        r.f.tenant,
        responses.len()
    );
    assert_eq!(
        actual_waiting, 20,
        "all twenty requests must be proven in the real PG blocker chain"
    );
    assert_eq!(
        responses.len(),
        20,
        "collect every actual outcome before checking"
    );
    assert_eq!(commands, 1);
    assert_eq!(
        statuses.iter().filter(|status| **status == 200).count(),
        if same { 20 } else { 1 }
    );
    assert!(
        statuses
            .iter()
            .all(|status| *status == 200 || (!same && *status == 409))
    );
    Ok(())
}

#[tokio::test]
async fn production_release_twenty_original_requests_have_one_effect() -> Result<()> {
    concurrency(true).await
}
#[tokio::test]
async fn production_release_twenty_competing_requests_have_one_effect() -> Result<()> {
    concurrency(false).await
}

#[tokio::test]
async fn production_release_waiting_user_is_rechecked_after_revocation() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = Arc::new(setup(&pool, "actor-revoked-waiting").await?);
    let mut gate = pool.begin().await?;
    let gate_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await?;
    sqlx::query("select user_id from users where tenant_id=$1 and user_id=$2 for update")
        .bind(r.f.tenant.0)
        .bind(r.f.actor)
        .execute(&mut *gate)
        .await?;
    let request = Uuid::new_v4().to_string();
    let pending = Arc::clone(&r);
    let task = tokio::spawn(async move { release(&pending, &request, pending.work_order).await });
    let waiting = blocker_count(&independent, &r, gate_pid, 1).await;
    sqlx::query("update users set status='DISABLED' where tenant_id=$1 and user_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.f.actor)
        .execute(&mut *gate)
        .await?;
    gate.commit().await?;
    let (status, body) = task.await??;
    let waiting = waiting?;
    eprintln!(
        "QA production_release_authority_wait tenant={} actual_waiting={waiting} status={}",
        r.f.tenant,
        status.as_u16()
    );
    assert_eq!(
        waiting, 1,
        "release must actually wait on the current actor"
    );
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    let count: i64 =
        sqlx::query_scalar("select count(*) from mfg_receipt_release_commands where tenant_id=$1")
            .bind(r.f.tenant.0)
            .fetch_one(&independent)
            .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[tokio::test]
async fn production_release_original_sources_and_effects_are_immutable() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "immutable-sources").await?;
    let id = Uuid::new_v4().to_string();
    let (status, body) = release(&r, &id, r.work_order).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let before = facts(&pool, &r).await?;
    let mut rejected_count = 0;
    for sql in [
        "update mfg_receipt_release_commands set snapshot='{}'::jsonb where tenant_id=$1",
        "delete from mfg_receipt_release_effects where tenant_id=$1",
        "update command_receipts set effect_id='forged' where tenant_id=$1 and request_id in(select request_id from mfg_receipt_release_commands where tenant_id=$1)",
        "update inventory_ledgers set delta_on_hand_micros=0 where tenant_id=$1 and reason_type in('MFG_RECEIPT','MFG_RECEIPT_RELEASE')",
        "update mfg_qty_ledgers set delta_micros=0 where tenant_id=$1",
        "update shared_stock_ledgers set delta_micros=0 where tenant_id=$1",
    ] {
        let result = sqlx::query(sql).bind(r.f.tenant.0).execute(&pool).await;
        if result.is_err() {
            rejected_count += 1;
        }
    }
    eprintln!(
        "QA production_release_immutable tenant={} actual_rejected={rejected_count}",
        r.f.tenant
    );
    assert_eq!(rejected_count, 6);
    assert_eq!(facts(&pool, &r).await?, before);
    Ok(())
}

#[tokio::test]
async fn production_release_unknown_original_snapshot_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "unknown-original-result").await?;
    let id = Uuid::new_v4().to_string();
    let payload = serde_json::to_string(&(1, r.work_order, r.receipt))?;
    sqlx::query("insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id) values($1,$2,$3,'SUCCEEDED',$4)")
        .bind(r.f.tenant.0).bind(&id).bind(payload).bind(r.receipt.to_string()).execute(&pool).await?;
    let before = facts(&pool, &r).await?;
    let (status, body) = release(&r, &id, r.work_order).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "unknown history cannot invent an original result: {body}"
    );
    assert_eq!(facts(&pool, &r).await?, before);
    Ok(())
}

#[tokio::test]
async fn production_release_waiting_work_order_rechecks_committed_quality() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = Arc::new(setup(&pool, "quality-changed-waiting").await?);
    reading(&pool, &r, false, false, false, 0).await?;
    let mut gate = pool.begin().await?;
    let gate_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await?;
    sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
        .bind(r.f.tenant.0).bind(r.work_order).execute(&mut *gate).await?;
    let request = Uuid::new_v4().to_string();
    let pending = Arc::clone(&r);
    let task = tokio::spawn(async move { release(&pending, &request, pending.work_order).await });
    let waiting = blocker_count(&independent, &r, gate_pid, 1).await;
    sqlx::query(
        "update mfg_quality_items set required=true where tenant_id=$1 and work_order_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(r.work_order)
    .execute(&mut *gate)
    .await?;
    gate.commit().await?;
    let (status, body) = task.await??;
    let waiting = waiting?;
    eprintln!(
        "QA production_release_quality_wait tenant={} actual_waiting={waiting} status={}",
        r.f.tenant,
        status.as_u16()
    );
    assert_eq!(waiting, 1, "release must actually wait on its work order");
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "new committed requirement must be checked: {body}"
    );
    let (held,commands):(i64,i64)=sqlx::query_as("select (select available_micros from mfg_receipts where tenant_id=$1 and receipt_id=$2),(select count(*) from mfg_receipt_release_commands where tenant_id=$1)")
        .bind(r.f.tenant.0).bind(r.receipt).fetch_one(&independent).await?;
    assert_eq!((held, commands), (0, 0));
    Ok(())
}

#[tokio::test]
async fn production_release_internal_entry_has_same_quality_guard() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = setup(&pool, "internal-quality-guard").await?;
    reading(&pool, &r, true, false, false, 0).await?;
    let before = facts(&pool, &r).await?;
    let id = Uuid::new_v4().to_string();
    let failed = SharedContractRepo::new(&pool)
        .release_production_receipt(r.f.tenant, &id, &id, r.receipt)
        .await;
    assert!(
        failed.is_err(),
        "the legacy repository entry must not bypass required quality"
    );
    assert_eq!(facts(&pool, &r).await?, before);
    sqlx::query("update mfg_quality_items set reading_micros=50,reading_present=true,passed=true where tenant_id=$1")
        .bind(r.f.tenant.0).execute(&pool).await?;
    let (write, total) = SharedContractRepo::new(&pool)
        .release_production_receipt(r.f.tenant, &id, &id, r.receipt)
        .await??;
    assert!(matches!(write, SharedWrite::Stored(_)));
    assert_eq!(total, 1250000);
    let actor: String = sqlx::query_scalar(
        "select actor_type from operation_logs where tenant_id=$1 and action='MFG_RECEIPT_RELEASE'",
    )
    .bind(r.f.tenant.0)
    .fetch_one(&pool)
    .await?;
    assert_eq!(actor, "SYSTEM");
    eprintln!(
        "QA production_release_internal tenant={} quality_blocked=true system_actor=true request_id={id}",
        r.f.tenant
    );
    Ok(())
}
