//! Complete-source quotes and admission must not consume unproved report totals.
use super::{
    Result,
    inventory_commands::Fixture,
    isolated_pool,
    operation_reports::{body, count, get, post, start},
};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

async fn fixture(pool: &PgPool) -> Result<(Fixture, Uuid)> {
    let (f, order) = start(pool).await?;
    sqlx::query("update tenants set name='QA-REPORT-SOURCES' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    eprintln!("QA report_sources tenant={} order={order}", f.tenant);
    Ok((f, order))
}
async fn context(f: &Fixture, order: Uuid) -> Result<(StatusCode, Value)> {
    let result = f
        .login
        .request(
            "GET",
            &format!(
                "/v1/tenants/{}/manufacturing/work-orders/{order}/reporting-context",
                f.tenant
            ),
            Value::Null,
        )
        .await?;
    eprintln!(
        "QA reporting_context tenant={} order={order} status={} response={}",
        f.tenant,
        result.0.as_u16(),
        result.1
    );
    Ok(result)
}

#[tokio::test]
async fn real_context_quotes_empty_then_posted_and_reversed_sources_exactly() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = fixture(&pool).await?;
    let (status, empty) = context(&f, order).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["session_id"], json!(f.login.session));
    assert_eq!(empty["actor_user_id"], json!(f.actor));
    assert_eq!(empty["operations"][0]["remaining_micros"], "1000000");
    assert_eq!(empty["operations"][1]["remaining_micros"], "0");
    let (status, original) = post(&f, order, body(&f, "CUT", "0.6")).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        post(&f, order, body(&f, "WELD", "0.25")).await?.0,
        StatusCode::OK
    );
    let (_, actual) = context(&f, order).await?;
    assert_eq!(actual["operations"][0]["good_micros"], "600000");
    assert_eq!(actual["operations"][1]["remaining_micros"], "350000");
    let mut reverse = body(&f, "CUT", "-0.6");
    reverse["minutes"] = json!(-10);
    reverse["reverses_report_id"] = original["effect_id"].clone();
    assert_eq!(
        post(&f, order, reverse).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        context(&f, order).await?.1["operations"],
        actual["operations"]
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 2);
    Ok(())
}

#[tokio::test]
async fn unproved_other_operation_blocks_whole_context_totals_and_new_output_but_not_original_read()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = fixture(&pool).await?;
    let input = body(&f, "CUT", "0.25");
    let (_, original) = post(&f, order, input.clone()).await?;
    sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id) values($1,$2,$3,'WELD',50000,0,0,1,'QA-LEGACY-WORKER',$4,$5)")
        .bind(f.tenant.0).bind(Uuid::new_v4()).bind(order).bind(f.actor.to_string()).bind(f.login.session.to_string()).execute(&pool).await?;
    assert_eq!(context(&f, order).await?.0, StatusCode::CONFLICT);
    assert_eq!(
        get(&f, order, "?operation_code=CUT").await?.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        post(&f, order, body(&f, "CUT", "0.1")).await?.0,
        StatusCode::CONFLICT
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
        .1,
        original
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 2);
    assert_eq!(count(&pool, &f, "mfg_operation_report_outcomes").await?, 1);
    Ok(())
}

#[tokio::test]
async fn changed_report_or_receipt_cannot_become_new_predecessor_capacity() -> Result<()> {
    let pool = isolated_pool().await?;
    for kind in ["quantity", "receipt"] {
        let (f, order) = fixture(&pool).await?;
        let input = body(&f, "CUT", "0.25");
        let (_, original) = post(&f, order, input.clone()).await?;
        if kind == "quantity" {
            sqlx::query(
                "update mfg_reports set good_micros=900000 where tenant_id=$1 and report_id=$2",
            )
            .bind(f.tenant.0)
            .bind(Uuid::parse_str(
                original["effect_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("missing effect"))?,
            )?)
            .execute(&pool)
            .await?;
        } else {
            sqlx::query("update command_receipts set payload_hash='QA-declared-corruption' where tenant_id=$1 and request_id=$2").bind(f.tenant.0).bind(input["request_id"].as_str()).execute(&pool).await?;
        }
        eprintln!(
            "QA report_source_corruption tenant={} kind={kind}",
            f.tenant
        );
        assert_eq!(context(&f, order).await?.0, StatusCode::CONFLICT);
        assert_eq!(
            get(&f, order, "?operation_code=WELD").await?.0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            post(&f, order, body(&f, "WELD", "0.5")).await?.0,
            StatusCode::CONFLICT
        );
        assert_eq!(count(&pool, &f, "mfg_reports").await?, 1);
        assert_eq!(count(&pool, &f, "mfg_operation_report_outcomes").await?, 1);
    }
    Ok(())
}

#[tokio::test]
async fn complete_report_budget_rejects_1001_before_returning_a_selected_operation() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = fixture(&pool).await?;
    sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id) select $1,gen_random_uuid(),$2,'CUT',0,0,0,0,'QA-BUDGET-WORKER',$3,$4 from generate_series(1,1001)")
        .bind(f.tenant.0).bind(order).bind(f.actor.to_string()).bind(f.login.session.to_string()).execute(&pool).await?;
    let (status, response) = context(&f, order).await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response["code"], "report_context_limit");
    assert_eq!(
        get(&f, order, "?operation_code=WELD").await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        post(&f, order, body(&f, "CUT", "0.1")).await?.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 1001);
    assert_eq!(count(&pool, &f, "mfg_operation_report_outcomes").await?, 0);
    Ok(())
}

#[tokio::test]
async fn frozen_parent_and_sequence_faults_are_not_selectable_context() -> Result<()> {
    let pool = isolated_pool().await?;
    for kind in ["parent", "cycle", "sequence", "step"] {
        let (f, order) = fixture(&pool).await?;
        match kind {
            "parent" => {
                sqlx::query("update shared_work_order_routes set route_id=$3 where tenant_id=$1 and work_order_id=$2 and operation_code='WELD'").bind(f.tenant.0).bind(order).bind(Uuid::new_v4()).execute(&pool).await?;
            }
            "cycle" => {
                sqlx::query("update shared_work_order_routes set predecessor_code='WELD' where tenant_id=$1 and work_order_id=$2 and operation_code='CUT'").bind(f.tenant.0).bind(order).execute(&pool).await?;
            }
            "sequence" => {
                sqlx::query("update shared_work_order_routes set sequence_no=1 where tenant_id=$1 and work_order_id=$2 and operation_code='WELD'").bind(f.tenant.0).bind(order).execute(&pool).await?;
            }
            _ => {
                sqlx::query("delete from shared_operation_steps where tenant_id=$1 and operation_code='WELD'").bind(f.tenant.0).execute(&pool).await?;
            }
        }
        assert_eq!(context(&f, order).await?.0, StatusCode::CONFLICT);
        assert_eq!(
            post(&f, order, body(&f, "CUT", "0.1")).await?.0,
            StatusCode::CONFLICT
        );
        assert_eq!(count(&pool, &f, "mfg_reports").await?, 0);
    }
    Ok(())
}

#[tokio::test]
async fn context_rechecks_current_read_permission_and_foreign_order_scope() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = fixture(&pool).await?;
    assert_eq!(context(&f, Uuid::new_v4()).await?.0, StatusCode::NOT_FOUND);
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in('manufacturing.read','MFG.PRODUCTION.READ')").bind(f.tenant.0).execute(&pool).await?;
    assert_eq!(context(&f, order).await?.0, StatusCode::FORBIDDEN);
    assert_eq!(count(&pool, &f, "mfg_reports").await?, 0);
    Ok(())
}
