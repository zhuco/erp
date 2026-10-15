//! Current authenticated reporting, stored ceilings, reversals and original outcomes.
use super::{
    Result, inventory_commands::Fixture, isolated_pool, material_confirmation::setup,
    production_receipt_release::http,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use erp_api::{AppState, build_app};
use erp_repo::db::Db;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub(super) async fn start(pool: &PgPool) -> Result<(Fixture, Uuid)> {
    let (f, order) = setup(pool).await?;
    sqlx::query("update mfg_work_orders set order_qty_micros=1000000 where tenant_id=$1 and work_order_id=$2").bind(f.tenant.0).bind(order).execute(pool).await?;
    sqlx::query("update tenants set name='QA-OPERATION-REPORT-AUTHORITY' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    let route = Uuid::new_v4();
    sqlx::query("insert into shared_operation_routes(tenant_id,route_id,product_code,version_no) values($1,$2,'QA-P',1)").bind(f.tenant.0).bind(route).execute(pool).await?;
    for (code, sequence, previous) in [("CUT", 1, ""), ("WELD", 2, "CUT")] {
        sqlx::query("insert into shared_operation_steps(tenant_id,route_id,operation_code,sequence_no,predecessor_code,work_center,setup_minutes,minutes_per,inspection_required,report_field) values($1,$2,$3,$4,$5,'QA-CENTER',0,1,false,'GOOD')")
            .bind(f.tenant.0).bind(route).bind(code).bind(sequence).bind(previous).execute(pool).await?;
        sqlx::query("insert into shared_work_order_routes(tenant_id,work_order_id,route_id,version_no,operation_code,sequence_no,predecessor_code,work_center,setup_minutes,minutes_per,inspection_required,report_field) select tenant_id,$3,route_id,1,operation_code,sequence_no,predecessor_code,work_center,setup_minutes,minutes_per,inspection_required,report_field from shared_operation_steps where tenant_id=$1 and route_id=$2 and operation_code=$4")
            .bind(f.tenant.0).bind(route).bind(order).bind(code).execute(pool).await?;
    }
    eprintln!(
        "QA operation_reports tenant={} order={order} actor={} session={}",
        f.tenant, f.actor, f.login.session
    );
    Ok((f, order))
}
pub(super) fn body(f: &Fixture, operation: &str, qty: &str) -> Value {
    json!({"request_id":Uuid::new_v4().to_string(),"operation_code":operation,"good_qty":qty,"rejected_qty":"0","rework_qty":"0","minutes":10,"operator_id":"QA-WORKER-001","session_id":f.login.session,"crew_code":"QA-CREW","started_at":"2026-10-10T00:00:00Z","ended_at":"2026-10-10T00:10:00Z","previous_good_qty":"999999","reverses_report_id":null})
}
pub(super) async fn post(f: &Fixture, order: Uuid, body: Value) -> Result<(StatusCode, Value)> {
    http(f, &format!("work-orders/{order}/reports"), body).await
}
pub(super) async fn get(f: &Fixture, order: Uuid, tail: &str) -> Result<(StatusCode, Value)> {
    let path = format!(
        "/v1/tenants/{}/manufacturing/work-orders/{order}/reports{tail}",
        f.tenant
    );
    let response = build_app(AppState::new(
        Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?,
        f.login.jwt.clone(),
    ))
    .oneshot(
        Request::builder()
            .uri(path)
            .header("authorization", format!("Bearer {}", f.login.token))
            .body(Body::empty())?,
    )
    .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA operation_report_get tenant={} order={order} tail={tail} status={} response={value}",
        f.tenant,
        status.as_u16()
    );
    Ok((status, value))
}
pub(super) async fn count(pool: &PgPool, f: &Fixture, table: &str) -> Result<i64> {
    Ok(
        sqlx::query_scalar(&format!("select count(*) from {table} where tenant_id=$1"))
            .bind(f.tenant.0)
            .fetch_one(pool)
            .await?,
    )
}

#[tokio::test]
async fn current_session_and_permissions_are_required_for_new_replay_and_reads() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    let mut bad = body(&f, "CUT", "0.25");
    bad["session_id"] = json!(Uuid::new_v4());
    assert_eq!(post(&f, order, bad).await?.0, StatusCode::FORBIDDEN);
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 0);
    let input = body(&f, "CUT", "0.25");
    let (status, original) = post(&f, order, input.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        get(
            &f,
            order,
            &format!(
                "/commands/{}",
                input["request_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("request missing"))?
            )
        )
        .await?
        .1,
        original
    );
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in('manufacturing.read','MFG.PRODUCTION.READ','manufacturing.write','MFG.PRODUCTION.WRITE')").bind(f.tenant.0).execute(&pool).await?;
    assert_eq!(
        post(&f, order, input.clone()).await?.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&f, order, "?operation_code=CUT").await?.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(
            &f,
            order,
            &format!(
                "/commands/{}",
                input["request_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("request missing"))?
            )
        )
        .await?
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 1);
    Ok(())
}

#[tokio::test]
async fn frozen_order_and_predecessor_bound_every_operation_and_concurrent_report() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    assert_eq!(
        post(&f, order, body(&f, "CUT", "1.000001")).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(&f, order, body(&f, "NOT-FROZEN", "0.1")).await?.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        post(&f, order, body(&f, "WELD", "0.1")).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (a, b) = tokio::join!(
        post(&f, order, body(&f, "CUT", "0.6")),
        post(&f, order, body(&f, "CUT", "0.6"))
    );
    let statuses = [a?.0, b?.0];
    assert_eq!(statuses.iter().filter(|s| **s == StatusCode::OK).count(), 1);
    assert_eq!(
        statuses
            .iter()
            .filter(|s| **s == StatusCode::UNPROCESSABLE_ENTITY)
            .count(),
        1
    );
    assert_eq!(
        post(&f, order, body(&f, "WELD", "0.600001")).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(&f, order, body(&f, "WELD", "0.6")).await?.0,
        StatusCode::OK
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 2);
    Ok(())
}

#[tokio::test]
async fn original_result_survives_later_reports_and_real_reversal() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    let input = body(&f, "CUT", "0.25");
    let (status, original) = post(&f, order, input.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        post(&f, order, body(&f, "CUT", "0.5")).await?.0,
        StatusCode::OK
    );
    let (mut_status, mut replay) = post(&f, order, input.clone()).await?;
    assert_eq!(mut_status, StatusCode::OK);
    assert_eq!(replay["replay"], true);
    replay["replay"] = json!(false);
    assert_eq!(replay, original);
    let mut changed = input.clone();
    changed["good_qty"] = json!("0.2");
    assert_eq!(post(&f, order, changed).await?.0, StatusCode::CONFLICT);
    let mut reverse = body(&f, "CUT", "-0.25");
    reverse["minutes"] = json!(-10);
    reverse["reverses_report_id"] = original["effect_id"].clone();
    assert_eq!(post(&f, order, reverse.clone()).await?.0, StatusCode::OK);
    reverse["request_id"] = json!(Uuid::new_v4());
    assert_eq!(
        post(&f, order, reverse).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        get(&f, order, "?operation_code=CUT").await?.1["good_micros"],
        500000
    );
    assert_eq!(
        get(
            &f,
            order,
            &format!(
                "/commands/{}",
                input["request_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("request missing"))?
            )
        )
        .await?
        .1,
        original
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 3);
    let protected =
        sqlx::query("update mfg_operation_report_outcomes set snapshot='{}' where tenant_id=$1")
            .bind(f.tenant.0)
            .execute(&pool)
            .await;
    assert!(protected.is_err());
    Ok(())
}

#[tokio::test]
async fn reversal_cannot_offset_another_order_operation_or_reversal() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    let (_, original) = post(&f, order, body(&f, "CUT", "0.25")).await?;
    let other = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) select tenant_id,$2,'QA-OTHER',product_code,'RELEASED',order_qty_micros from mfg_work_orders where tenant_id=$1 and work_order_id=$3").bind(f.tenant.0).bind(other).bind(order).execute(&pool).await?;
    sqlx::query("insert into shared_work_order_routes select tenant_id,$2,route_id,version_no,operation_code,sequence_no,predecessor_code,work_center,setup_minutes,minutes_per,inspection_required,report_field,created_at from shared_work_order_routes where tenant_id=$1 and work_order_id=$3").bind(f.tenant.0).bind(other).bind(order).execute(&pool).await?;
    let mut reverse = body(&f, "CUT", "-0.25");
    reverse["minutes"] = json!(-10);
    reverse["reverses_report_id"] = original["effect_id"].clone();
    assert_eq!(
        post(&f, other, reverse.clone()).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    reverse["operation_code"] = json!("WELD");
    assert_eq!(
        post(&f, order, reverse.clone()).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 1);
    Ok(())
}

#[tokio::test]
async fn outcome_storage_failure_rolls_back_the_report_and_original_receipt() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    let name = format!("qa_report_failure_{}", f.tenant.0.simple());
    sqlx::query(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then raise exception 'owned report outcome failure'; end if; return new; end $$",f.tenant.0)).execute(&pool).await?;
    sqlx::query(&format!("create trigger {name} before insert on mfg_operation_report_outcomes for each row execute function {name}()")).execute(&pool).await?;
    let result = post(&f, order, body(&f, "CUT", "0.25")).await;
    sqlx::query(&format!(
        "drop trigger {name} on mfg_operation_report_outcomes"
    ))
    .execute(&pool)
    .await?;
    sqlx::query(&format!("drop function {name}()"))
        .execute(&pool)
        .await?;
    assert_eq!(result?.0, StatusCode::INTERNAL_SERVER_ERROR);
    for table in ["mfg_reports", "mfg_operation_report_outcomes"] {
        assert_eq!(count(&pool, &f, table).await?, 0);
    }
    assert_eq!(count(&pool, &f, "command_receipts").await?, 1);
    assert_eq!(
        post(&f, order, body(&f, "CUT", "0.25")).await?.0,
        StatusCode::OK
    );
    Ok(())
}
#[tokio::test]
async fn invalid_times_closed_orders_and_missing_context_never_create_reports() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    for (start, end) in [
        ("not-a-date", "2026-10-10T00:10:00Z"),
        ("2026-10-10T00:11:00Z", "2026-10-10T00:10:00Z"),
    ] {
        let mut input = body(&f, "CUT", "0.25");
        input["started_at"] = json!(start);
        input["ended_at"] = json!(end);
        assert_eq!(
            post(&f, order, input).await?.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        get(&f, Uuid::new_v4(), "?operation_code=CUT").await?.0,
        StatusCode::NOT_FOUND
    );
    sqlx::query(
        "update mfg_work_orders set status='CLOSED' where tenant_id=$1 and work_order_id=$2",
    )
    .bind(f.tenant.0)
    .bind(order)
    .execute(&pool)
    .await?;
    assert_eq!(
        post(&f, order, body(&f, "CUT", "0.25")).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 0);
    Ok(())
}

#[tokio::test]
async fn reversals_preserve_effective_downstream_output_and_real_pending_receipts() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    assert_eq!(
        post(&f, order, body(&f, "CUT", "0.4")).await?.0,
        StatusCode::OK
    );
    let (_, cut) = post(&f, order, body(&f, "CUT", "0.5")).await?;
    let (status, weld) = post(&f, order, body(&f, "WELD", "0.8")).await?;
    assert_eq!(status, StatusCode::OK);
    let mut reverse = body(&f, "CUT", "-0.5");
    reverse["minutes"] = json!(-10);
    reverse["reverses_report_id"] = cut["effect_id"].clone();
    assert_eq!(
        post(&f, order, reverse).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let(status,receipt)=http(&f,&format!("work-orders/{order}/receipts"),json!({"request_id":Uuid::new_v4(),"qty":"0.1","status":"HOLD","report_id":weld["effect_id"],"batch_code":"QA-REPORT-SOURCE","inspection_id":Uuid::new_v4(),"warehouse_id":f.warehouse,"sku_id":f.sku})).await?;
    assert_eq!(status, StatusCode::OK, "{receipt}");
    let mut reverse = body(&f, "WELD", "-0.8");
    reverse["minutes"] = json!(-10);
    reverse["reverses_report_id"] = weld["effect_id"].clone();
    assert_eq!(
        post(&f, order, reverse).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 3);
    assert_eq!(count(&pool, &f, "mfg_receipts").await?, 1);
    Ok(())
}

#[tokio::test]
async fn source_mismatch_requires_reconciliation_without_replaying_business() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    let input = body(&f, "CUT", "0.25");
    let (status, original) = post(&f, order, input.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let id = Uuid::parse_str(
        original["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing effect"))?,
    )?;
    sqlx::query("update mfg_reports set good_micros=200000 where tenant_id=$1 and report_id=$2")
        .bind(f.tenant.0)
        .bind(id)
        .execute(&pool)
        .await?;
    eprintln!(
        "QA operation_report_source_corruption tenant={} report={id} actual_good=200000 original_good=250000",
        f.tenant
    );
    let (status, response) = post(&f, order, input.clone()).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(response["code"], "report_reconciliation_required");
    let (status, response) = get(
        &f,
        order,
        &format!(
            "/commands/{}",
            input["request_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing request"))?
        ),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(response["code"], "report_reconciliation_required");
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 1);
    Ok(())
}

#[tokio::test]
async fn expired_revoked_or_stale_credentials_reject_writes_and_original_reads() -> Result<()> {
    let pool = isolated_pool().await?;
    for mutation in ["expiry", "revoke", "epoch"] {
        let (f, order) = start(&pool).await?;
        let input = body(&f, "CUT", "0.25");
        assert_eq!(post(&f, order, input.clone()).await?.0, StatusCode::OK);
        match mutation {
            "expiry" => {
                sqlx::query("update auth_sessions set expires_at=clock_timestamp()-interval '1 second' where tenant_id=$1 and session_id=$2").bind(f.tenant.0).bind(f.login.session).execute(&pool).await?;
            }
            "revoke" => {
                sqlx::query("update auth_sessions set status='REVOKED' where tenant_id=$1 and session_id=$2").bind(f.tenant.0).bind(f.login.session).execute(&pool).await?;
            }
            _ => {
                sqlx::query(
                    "update tenants set credentials_epoch=credentials_epoch+1 where tenant_id=$1",
                )
                .bind(f.tenant.0)
                .execute(&pool)
                .await?;
            }
        }
        assert_eq!(
            post(&f, order, input.clone()).await?.0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get(
                &f,
                order,
                &format!(
                    "/commands/{}",
                    input["request_id"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("missing request"))?
                )
            )
            .await?
            .0,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(count(&pool, &f, "mfg_reports").await?, 1);
    }
    Ok(())
}
#[tokio::test]
async fn original_outcome_reads_compare_timestamp_instants_across_database_timezones() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order) = start(&pool).await?;
    let input = body(&f, "CUT", "0.25");
    let (status, original) = post(&f, order, input.clone()).await?;
    assert_eq!(status, StatusCode::OK);
    let zoned = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("set timezone='Asia/Shanghai'")
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)
        .await?;
    super::qa_database::verify(&zoned).await?;
    let timezone: String = sqlx::query_scalar("show timezone")
        .fetch_one(&zoned)
        .await?;
    assert_eq!(timezone, "Asia/Shanghai");
    let result = erp_repo::shared_contract::SharedContractRepo::new(&zoned)
        .read_authorized_operation_report(
            f.tenant,
            order,
            input["request_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing request"))?,
            erp_repo::shared_contract::ReceiptValueActor {
                authentication_tenant: f.tenant,
                user_id: erp_types::UserId(f.actor),
                session_id: f.login.session,
                credential_epoch: 0,
                warehouse_id: Uuid::nil(),
            },
        )
        .await?;
    assert_eq!(result, original);
    zoned.close().await;
    Ok(())
}
