//! Real password/session quality decisions and independently read exact stock.
use super::{
    Result,
    inventory_commands::{Fixture, fixture},
    isolated_pool,
};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

struct Source {
    f: Fixture,
    order: Uuid,
    receipt: Uuid,
}
async fn setup(pool: &PgPool, qty: &str, unit: Option<i64>, case: &str) -> Result<Source> {
    let f = fixture(pool).await?;
    let order = Uuid::new_v4();
    let report = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-SOURCE-DISPOSITION','QA-COMMAND','RELEASED',10000000)")
        .bind(f.tenant.0).bind(order).execute(pool).await?;
    sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id,started_at) values($1,$2,$3,'FINAL',10000000,0,0,1,'QA-WORKER','QA-LOGIN','QA-SID','2026-10-08T01:00:00Z')")
        .bind(f.tenant.0).bind(report).bind(order).execute(pool).await?;
    let (status,body)=f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{order}/receipts",f.tenant),
        json!({"request_id":Uuid::new_v4(),"qty":qty,"status":"HOLD","report_id":report,
        "batch_code":"QA-DISPOSITION","inspection_id":Uuid::new_v4(),"warehouse_id":f.warehouse,"sku_id":f.sku})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let receipt = Uuid::parse_str(
        body["effect_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing original receipt"))?,
    )?;
    if let Some(unit) = unit {
        let qty = erp_domain::quantity::parse_qty(qty)?.as_micros();
        let total = i64::try_from(i128::from(qty) * i128::from(unit) / 1_000_000)?;
        sqlx::query("insert into mfg_cost_runs(tenant_id,work_order_id,material_cent,labor_cent,overhead_cent,total_cent,unit_cent,completed_qty_micros,estimated,cost_run_id) values($1,$2,$3,0,0,$3,$4,$5,false,$6)")
            .bind(f.tenant.0).bind(order).bind(total).bind(unit).bind(qty).bind(Uuid::new_v4()).execute(pool).await?;
    }
    eprintln!(
        "QA source_disposition_fixture tenant={} case={case} receipt={receipt} order={order}",
        f.tenant
    );
    Ok(Source { f, order, receipt })
}
async fn post(
    s: &Source,
    id: &str,
    kind: &str,
    qty: &str,
    parent: Option<Uuid>,
    rework: Option<Uuid>,
) -> Result<(StatusCode, Value)> {
    s.f.login.request("POST",&format!("/v1/tenants/{}/quality/dispositions",s.f.tenant),json!({
        "request_id":id,"receipt_id":s.receipt,"parent_disposition_id":parent,
        "location_id":Uuid::nil(),"warehouse_id":s.f.warehouse,"sku_id":s.f.sku,
        "kind":kind,"qty":qty,"reason":"QA actual decision","rework_work_order_id":rework.map(|id|id.to_string()).unwrap_or_default()
    })).await
}
async fn facts(pool: &PgPool, s: &Source) -> Result<Value> {
    let mut facts = serde_json::Map::new();
    for table in [
        "inventory_balances",
        "inventory_ledgers",
        "shared_stock_ledgers",
        "mfg_qty_ledgers",
        "mfg_receipts",
        "mfg_receipt_disposition_commands",
        "mfg_receipt_disposition_costs",
        "mfg_quality_dispositions",
        "mfg_material_moves",
        "mfg_work_orders",
        "mfg_rework_material_values",
        "mfg_material_cost_bases",
        "mfg_cost_commit_authorities",
        "mfg_cost_allocations",
        "mfg_cost_labor_bases",
        "mfg_cost_center_bindings",
        "mfg_material_issue_values",
        "mfg_material_value_allocations",
        "mfg_cost_runs",
        "command_receipts",
        "operation_logs",
        "audit_logs",
        "outbox_events",
    ] {
        let rows:Value=sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(t) order by to_jsonb(t)::text),'[]') from {table} t where tenant_id=$1"))
            .bind(s.f.tenant.0).fetch_one(pool).await?;
        facts.insert(table.into(), rows);
    }
    Ok(Value::Object(facts))
}
async fn balance(pool: &PgPool, s: &Source) -> Result<(i64, i64)> {
    Ok(sqlx::query_as("select on_hand_micros,frozen_micros from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(s.f.tenant.0).bind(s.f.warehouse).bind(s.f.sku).fetch_one(pool).await?)
}

async fn state(s: &Source) -> Result<Value> {
    let (status, value) =
        s.f.login
            .request(
                "GET",
                &format!(
                    "/v1/tenants/{}/quality/receipt-dispositions/{}?warehouse_id={}",
                    s.f.tenant, s.receipt, s.f.warehouse
                ),
                Value::Null,
            )
            .await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    Ok(value)
}

#[tokio::test]
async fn source_dispositions_partial_acceptance_and_original_replay() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1.25", None, "accept-replay").await?;
    let a = post(&s, "QA-source-A", "ACCEPT", "0.5", None, None).await?;
    assert_eq!(a.0, StatusCode::OK, "{}", a.1);
    assert_eq!(a.1["result"]["source_remaining_micros"], 750_000);
    assert_eq!(balance(&read, &s).await?, (6_150_000, 1_650_000));
    let before = facts(&read, &s).await?;
    let release =
        s.f.login
            .request(
                "POST",
                &format!(
                    "/v1/tenants/{}/manufacturing/work-orders/{}/receipts/{}/release",
                    s.f.tenant, s.order, s.receipt
                ),
                json!({"request_id":"QA-double-release"}),
            )
            .await?;
    assert_eq!(release.0, StatusCode::CONFLICT, "{}", release.1);
    assert_eq!(facts(&read, &s).await?, before);
    let b = post(&s, "QA-source-B", "ACCEPT", "0.75", None, None).await?;
    assert_eq!(b.0, StatusCode::OK, "{}", b.1);
    assert_eq!(balance(&read, &s).await?, (6_150_000, 900_000));
    let current = state(&s).await?;
    assert_eq!(current["held_micros"], 0);
    assert_eq!(current["available_micros"], 1_250_000);
    let before = facts(&read, &s).await?;
    let replay = post(&s, "QA-source-A", "ACCEPT", "0.5", None, None).await?;
    assert_eq!(replay.0, StatusCode::OK, "{}", replay.1);
    assert_eq!(replay.1["result"], a.1["result"]);
    assert_eq!(replay.1["replay"], true);
    assert_eq!(facts(&read, &s).await?, before);
    let changed = post(&s, "QA-source-A", "ACCEPT", "0.4", None, None).await?;
    assert_eq!(changed.0, StatusCode::CONFLICT);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_isolation_is_not_disposal_and_resumes_from_its_parent() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1.25", None, "isolate-parent").await?;
    let a = post(&s, "QA-isolate", "ISOLATE", "1", None, None).await?;
    assert_eq!(a.0, StatusCode::OK, "{}", a.1);
    assert_eq!(balance(&read, &s).await?, (6_150_000, 2_150_000));
    let parent = Uuid::parse_str(
        a.1["result"]["disposition_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing parent"))?,
    )?;
    let b = post(&s, "QA-parent-accept", "ACCEPT", "0.75", Some(parent), None).await?;
    assert_eq!(b.0, StatusCode::OK, "{}", b.1);
    assert_eq!(b.1["result"]["source_remaining_micros"], 250_000);
    assert_eq!(balance(&read, &s).await?, (6_150_000, 1_400_000));
    let current = state(&s).await?;
    assert_eq!(current["held_micros"], 250_000);
    assert_eq!(current["isolated_micros"], 250_000);
    assert_eq!(current["available_micros"], 750_000);
    let before = facts(&read, &s).await?;
    let over = post(&s, "QA-parent-over", "ACCEPT", "0.3", Some(parent), None).await?;
    assert_eq!(over.0, StatusCode::CONFLICT);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_two_half_scraps_share_one_original_cent() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", Some(1), "scrap-tail").await?;
    let a = post(&s, "QA-half-A", "SCRAP", "0.5", None, None).await?;
    let b = post(&s, "QA-half-B", "SCRAP", "0.5", None, None).await?;
    assert_eq!(a.0, StatusCode::OK, "{}", a.1);
    assert_eq!(b.0, StatusCode::OK, "{}", b.1);
    assert_eq!(a.1["result"]["cost_cent"], 0);
    assert_eq!(b.1["result"]["cost_cent"], 1);
    assert_eq!(balance(&read, &s).await?, (4_900_000, 900_000));
    let before = facts(&read, &s).await?;
    let repeat = post(&s, "QA-half-A", "SCRAP", "0.5", None, None).await?;
    assert_eq!(repeat.0, StatusCode::OK);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_missing_cost_or_revoked_grant_has_zero_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1.25", None, "missing-cost-grant").await?;
    let before = facts(&read, &s).await?;
    let a = post(&s, "QA-no-cost", "SCRAP", "0.5", None, None).await?;
    assert_eq!(a.0, StatusCode::CONFLICT);
    assert_eq!(facts(&read, &s).await?, before);
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code='quality.write'")
        .bind(s.f.tenant.0)
        .execute(&pool)
        .await?;
    let before = facts(&read, &s).await?;
    let b = post(&s, "QA-no-grant", "ACCEPT", "0.5", None, None).await?;
    assert_eq!(b.0, StatusCode::FORBIDDEN, "{}", b.1);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_is_a_real_issued_material_and_value() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1.25", Some(200), "rework").await?;
    let rework = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-ACTUAL-REWORK','QA-COMMAND','RELEASED',1000000)")
        .bind(s.f.tenant.0).bind(rework).execute(&pool).await?;
    sqlx::query("insert into mfg_exceptions(tenant_id,exception_id,work_order_id,kind,qty_micros,reason,source_work_order_id,added_minutes) values($1,$2,$3,'REWORK',1000000,'QA explicit original rework',$4,1)")
        .bind(s.f.tenant.0).bind(Uuid::new_v4()).bind(rework).bind(s.order).execute(&pool).await?;
    let result = post(&s, "QA-real-rework", "REWORK", "0.5", None, Some(rework)).await?;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    assert_eq!(result.1["result"]["cost_cent"], 100);
    let issued:(i64,i64)=sqlx::query_as("select w.issued_micros,m.material_shop_micros from mfg_work_orders w join mfg_material_moves m on m.tenant_id=w.tenant_id and m.work_order_id=w.work_order_id where w.tenant_id=$1 and w.work_order_id=$2")
        .bind(s.f.tenant.0).bind(rework).fetch_one(&read).await?;
    assert_eq!(issued, (500_000, 500_000));
    assert_eq!(balance(&read, &s).await?, (5_650_000, 1_650_000));
    Ok(())
}

async fn rework_source(pool: &PgPool, case: &str, unit: i64) -> Result<(Source, Uuid, Uuid)> {
    let s = setup(pool, "1", Some(unit), case).await?;
    let order = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-REWORK-FOLLOWUP','QA-COMMAND','RELEASED',1000000)")
        .bind(s.f.tenant.0).bind(order).execute(pool).await?;
    sqlx::query("insert into mfg_exceptions(tenant_id,exception_id,work_order_id,kind,qty_micros,reason,source_work_order_id,added_minutes) values($1,$2,$3,'REWORK',1000000,'QA actual rework',$4,1)")
        .bind(s.f.tenant.0).bind(Uuid::new_v4()).bind(order).bind(s.order).execute(pool).await?;
    let issued = post(&s, "QA-followup-issue", "REWORK", "1", None, Some(order)).await?;
    assert_eq!(issued.0, StatusCode::OK, "{}", issued.1);
    let issue = Uuid::parse_str(
        issued.1["result"]["material_move_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing issued material"))?,
    )?;
    let route = Uuid::new_v4();
    sqlx::query("insert into shared_operation_routes(tenant_id,route_id,product_code,version_no) values($1,$2,'QA-COMMAND',1)").bind(s.f.tenant.0).bind(route).execute(pool).await?;
    sqlx::query("insert into shared_operation_steps(tenant_id,route_id,operation_code,sequence_no,predecessor_code,work_center,setup_minutes,minutes_per,inspection_required,report_field) values($1,$2,'FINAL',1,'','QA-REWORK-LINE',0,0,false,'GOOD')").bind(s.f.tenant.0).bind(route).execute(pool).await?;
    sqlx::query("insert into shared_work_order_routes(tenant_id,work_order_id,route_id,version_no,operation_code,sequence_no,predecessor_code,work_center,setup_minutes,minutes_per,inspection_required,report_field) select tenant_id,$3,route_id,1,operation_code,sequence_no,predecessor_code,work_center,setup_minutes,minutes_per,inspection_required,report_field from shared_operation_steps where tenant_id=$1 and route_id=$2").bind(s.f.tenant.0).bind(route).bind(order).execute(pool).await?;
    let report=s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{order}/reports",s.f.tenant),json!({
        "request_id":"QA-actual-rework-output","operation_code":"FINAL","good_qty":"1","rejected_qty":"0","rework_qty":"0","minutes":0,"operator_id":"QA-REWORK-WORKER","session_id":s.f.login.session,"crew_code":"QA-CREW","started_at":"2026-10-08T01:00:00Z","ended_at":"2026-10-08T01:01:00Z","previous_good_qty":"1"
    })).await?;
    assert_eq!(report.0, StatusCode::OK, "{}", report.1);
    Ok((s, order, issue))
}
async fn material(
    s: &Source,
    order: Uuid,
    issue: Uuid,
    id: &str,
    kind: &str,
    qty: &str,
) -> Result<(StatusCode, Value)> {
    s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{order}/materials",s.f.tenant),json!({
        "request_id":id,"warehouse_id":s.f.warehouse,"sku_id":s.f.sku,"original_issue_id":issue,"kind":kind,"qty":qty,"reason":"QA source followup"
    })).await
}
async fn returned_decision(
    s: &Source,
    returned: Uuid,
    id: &str,
    kind: &str,
    qty: &str,
) -> Result<(StatusCode, Value)> {
    s.f.login.request("POST",&format!("/v1/tenants/{}/quality/dispositions",s.f.tenant),json!({
        "request_id":id,"receipt_id":s.receipt,"returned_source_move_id":returned,"warehouse_id":s.f.warehouse,
        "location_id":Uuid::nil(),"sku_id":s.f.sku,"kind":kind,"qty":qty,"reason":"QA returned original material"
    })).await
}

#[tokio::test]
async fn source_dispositions_rework_consume_return_preserves_quality_and_value() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-consume-return", 200).await?;
    let before = balance(&read, &s).await?;
    let consumed = material(&s, order, issue, "QA-consume-quarter", "CONSUME", "0.25").await?;
    assert_eq!(consumed.0, StatusCode::OK, "{}", consumed.1);
    assert_eq!(balance(&read, &s).await?, before);
    let returned = material(&s, order, issue, "QA-return-half", "RETURN", "0.5").await?;
    assert_eq!(returned.0, StatusCode::OK, "{}", returned.1);
    let move_id = Uuid::parse_str(
        returned.1["move_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing return"))?,
    )?;
    assert_eq!(balance(&read, &s).await?, (5_400_000, 1_400_000));
    let values:Vec<(String,i64)>=sqlx::query_as("select kind,cost_cent from mfg_rework_material_values where tenant_id=$1 order by created_at,move_id")
        .bind(s.f.tenant.0).fetch_all(&read).await?;
    assert_eq!(values, vec![("CONSUME".into(), 50), ("RETURN".into(), 100)]);
    let current = state(&s).await?;
    assert_eq!(current["isolated_micros"], 500_000);
    assert_eq!(current["rework_micros"], 500_000);
    let facts_before = facts(&read, &s).await?;
    let replay = material(&s, order, issue, "QA-return-half", "RETURN", "0.5").await?;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1["replay"], true);
    assert_eq!(facts(&read, &s).await?, facts_before);
    let excessive = material(&s, order, issue, "QA-return-too-much", "RETURN", "0.3").await?;
    assert_eq!(excessive.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(facts(&read, &s).await?, facts_before);
    let accepted = returned_decision(&s, move_id, "QA-return-accept", "ACCEPT", "0.5").await?;
    assert_eq!(accepted.0, StatusCode::OK, "{}", accepted.1);
    assert_eq!(balance(&read, &s).await?, (5_400_000, 900_000));
    let current = state(&s).await?;
    assert_eq!(current["isolated_micros"], 0);
    assert_eq!(current["available_micros"], 500_000);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_returned_cost_is_not_charged_twice() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-return-rounding", 1).await?;
    let returned = material(&s, order, issue, "QA-one-cent-return", "RETURN", "1").await?;
    assert_eq!(returned.0, StatusCode::OK, "{}", returned.1);
    let returned = Uuid::parse_str(
        returned.1["move_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing return"))?,
    )?;
    for (id, amount) in [("QA-return-scrap-A", 0), ("QA-return-scrap-B", 1)] {
        let scrap = returned_decision(&s, returned, id, "SCRAP", "0.5").await?;
        assert_eq!(scrap.0, StatusCode::OK, "{}", scrap.1);
        assert_eq!(scrap.1["result"]["cost_cent"], amount);
    }
    assert_eq!(balance(&read, &s).await?, (4_900_000, 900_000));
    assert_eq!(state(&s).await?["scrapped_micros"], 1_000_000);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_revoked_material_grant_has_zero_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-current-grant", 200).await?;
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in('manufacturing.write','MFG.PRODUCTION.WRITE')")
        .bind(s.f.tenant.0).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let denied = material(&s, order, issue, "QA-rework-revoked", "RETURN", "0.5").await?;
    assert_eq!(denied.0, StatusCode::FORBIDDEN, "{}", denied.1);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_return_isolation_keeps_original_cost_and_scope() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-return-isolate", 1).await?;
    let returned = material(&s, order, issue, "QA-isolate-return", "RETURN", "1").await?;
    assert_eq!(returned.0, StatusCode::OK, "{}", returned.1);
    let returned = Uuid::parse_str(
        returned.1["move_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing return"))?,
    )?;
    let isolated = returned_decision(&s, returned, "QA-return-isolate", "ISOLATE", "1").await?;
    assert_eq!(isolated.0, StatusCode::OK, "{}", isolated.1);
    let parent = Uuid::parse_str(
        isolated.1["result"]["disposition_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing isolate"))?,
    )?;
    for (id, value) in [("QA-isolated-scrap-A", 0), ("QA-isolated-scrap-B", 1)] {
        let scrap = post(&s, id, "SCRAP", "0.5", Some(parent), None).await?;
        assert_eq!(scrap.0, StatusCode::OK, "{}", scrap.1);
        assert_eq!(scrap.1["result"]["cost_cent"], value);
    }
    let before = facts(&read, &s).await?;
    let consumed_source =
        returned_decision(&s, issue, "QA-invalid-return-source", "ACCEPT", "0.1").await?;
    assert_eq!(consumed_source.0, StatusCode::CONFLICT);
    assert_eq!(facts(&read, &s).await?, before);
    assert_eq!(state(&s).await?["isolated_micros"], 0);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_value_failure_rolls_back_stock_and_same_id_retries()
-> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-value-rollback", 200).await?;
    let name = format!("qa_rework_value_fault_{}", s.f.tenant.0.simple());
    sqlx::raw_sql(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then raise exception 'owned rework failure'; end if; return new; end $$; create trigger {name} before insert on mfg_rework_material_values for each row execute function {name}();",s.f.tenant.0)).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let failed = material(&s, order, issue, "QA-rework-fault", "RETURN", "0.5").await?;
    sqlx::raw_sql(&format!(
        "drop trigger {name} on mfg_rework_material_values;drop function {name}();"
    ))
    .execute(&pool)
    .await?;
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR, "{}", failed.1);
    assert_eq!(facts(&read, &s).await?, before);
    let retry = material(&s, order, issue, "QA-rework-fault", "RETURN", "0.5").await?;
    assert_eq!(retry.0, StatusCode::OK, "{}", retry.1);
    assert_eq!(balance(&read, &s).await?, (5_400_000, 1_400_000));
    Ok(())
}

async fn cost_center(pool: &PgPool, s: &Source) -> Result<Uuid> {
    let repo = erp_repo::shared_contract::SharedContractRepo::new(pool);
    let (_, center) = repo
        .save_work_center_calendar(
            s.f.tenant,
            "QA-cost-center",
            "QA-cost-center",
            "QA-REWORK-LINE",
            1,
            &[erp_domain::manufacturing::CalendarDay {
                day: "2026-10-08".into(),
                rest: false,
                shift_minutes: 480,
            }],
        )
        .await?
        .map_err(|e| anyhow::anyhow!("center rejected {e:?}"))?;
    repo.save_work_center_rate(
        s.f.tenant,
        "QA-cost-rate",
        "QA-cost-rate",
        center,
        1,
        "2026-10-01",
        "",
        "2026-10-08",
    )
    .await?
    .map_err(|e| anyhow::anyhow!("rate rejected {e:?}"))?;
    let orders: Vec<Uuid> = sqlx::query_scalar(
        "select work_order_id from mfg_work_orders where tenant_id=$1 order by work_order_id",
    )
    .bind(s.f.tenant.0)
    .fetch_all(pool)
    .await?;
    for order in orders {
        let result =
            s.f.login
                .request(
                    "POST",
                    &format!(
                        "/v1/tenants/{}/manufacturing/work-orders/{order}/cost-center-binding",
                        s.f.tenant
                    ),
                    json!({"center_id":center,"reason":"QA explicit work-order labor center"}),
                )
                .await?;
        assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    }
    Ok(center)
}
async fn confirm_cost(
    s: &Source,
    order: Uuid,
    center: Uuid,
    id: &str,
) -> Result<(StatusCode, Value)> {
    s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{order}/costs",s.f.tenant),json!({
        "request_id":id,"item_code":"QA-UNPRICED-CLIENT-CODE","center_id":center,"as_of":"2026-10-08",
        "overhead_cent":0,"completed_qty":if order==s.order {"10"}else{"1"},"rejected_qty":"0","unfinished_qty":"0"
    })).await
}

#[tokio::test]
async fn source_dispositions_rework_fractional_cost_uses_consumed_value_not_return_or_client_price()
-> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-cost-confirm", 200).await?;
    let center = cost_center(&pool, &s).await?;
    assert_eq!(
        material(&s, order, issue, "QA-cost-consume", "CONSUME", "0.25")
            .await?
            .0,
        StatusCode::OK
    );
    assert_eq!(
        material(&s, order, issue, "QA-cost-return", "RETURN", "0.5")
            .await?
            .0,
        StatusCode::OK
    );
    let stock = balance(&read, &s).await?;
    let confirmed = confirm_cost(&s, order, center, "QA-frozen-cost-confirm").await?;
    assert_eq!(confirmed.0, StatusCode::OK, "{}", confirmed.1);
    assert_eq!(confirmed.1["total_cent"], 50);
    assert_eq!(confirmed.1["estimated"], false);
    assert_eq!(balance(&read, &s).await?, stock);
    let before = facts(&read, &s).await?;
    let replay = confirm_cost(&s, order, center, "QA-frozen-cost-confirm").await?;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1["replay"], true);
    assert_eq!(facts(&read, &s).await?, before);
    let basis:Value=sqlx::query_scalar("select b.snapshot from mfg_material_cost_bases b join mfg_cost_runs c on c.tenant_id=b.tenant_id and c.cost_run_id=b.cost_run_id where c.tenant_id=$1 and c.work_order_id=$2")
        .bind(s.f.tenant.0).bind(order).fetch_one(&read).await?;
    assert_eq!(basis["material_cent"], 50);
    assert_eq!(basis["legacy_quantity_micros"], 0);
    assert_eq!(
        basis["rework_consumptions"].as_array().map(Vec::len),
        Some(1)
    );
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_consumption_cost_tail_is_one_cent() -> Result<()> {
    let pool = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-cost-tail", 1).await?;
    let center = cost_center(&pool, &s).await?;
    for id in ["QA-cost-tail-A", "QA-cost-tail-B"] {
        assert_eq!(
            material(&s, order, issue, id, "CONSUME", "0.5").await?.0,
            StatusCode::OK
        );
    }
    let confirmed = confirm_cost(&s, order, center, "QA-cost-tail-confirm").await?;
    assert_eq!(confirmed.0, StatusCode::OK, "{}", confirmed.1);
    assert_eq!(confirmed.1["total_cent"], 1);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_unvalued_legacy_consume_requires_reconciliation() -> Result<()>
{
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-cost-missing-proof", 200).await?;
    let center = cost_center(&pool, &s).await?;
    sqlx::query("insert into mfg_material_moves(tenant_id,move_id,work_order_id,kind,qty_micros,warehouse_id,sku_id,original_issue_id) values($1,$2,$3,'CONSUME',250000,$4,$5,$6)")
        .bind(s.f.tenant.0).bind(Uuid::new_v4()).bind(order).bind(s.f.warehouse).bind(s.f.sku).bind(issue).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let refused = confirm_cost(&s, order, center, "QA-cost-no-proof").await?;
    assert_eq!(refused.0, StatusCode::CONFLICT, "{}", refused.1);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_cost_estimate_and_leased_batch_keep_original_value()
-> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    for mode in ["estimate", "batch"] {
        let (s, order, issue) = rework_source(&pool, &format!("rework-cost-{mode}"), 200).await?;
        let center = cost_center(&pool, &s).await?;
        assert_eq!(
            material(&s, order, issue, "QA-mode-consume", "CONSUME", "0.25")
                .await?
                .0,
            StatusCode::OK
        );
        if mode == "estimate" {
            let result=s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{order}/cost-estimates",s.f.tenant),json!({
                "request_id":"QA-source-estimate","item_code":"QA-UNPRICED","center_id":center,"as_of":"2026-10-08","completed_qty":"1"
            })).await?;
            assert_eq!(result.0, StatusCode::OK, "{}", result.1);
            assert_eq!(result.1["total_cent"], 50);
            assert_eq!(result.1["estimated"], true);
        } else {
            let repo = erp_repo::shared_contract::SharedContractRepo::new(&pool);
            let (_, task) = repo
                .enqueue_cost_batch(
                    s.f.tenant,
                    "QA-source-cost-task",
                    "QA-source-cost-task",
                    order,
                )
                .await?
                .map_err(|e| anyhow::anyhow!("task rejected {e:?}"))?;
            sqlx::query("update durable_tasks set status='LEASED',lease_owner='QA-owned-cost-worker',lease_until=now()+interval '5 minutes' where tenant_id=$1 and task_id=$2")
                .bind(s.f.tenant.0).bind(task).execute(&pool).await?;
            let result = repo
                .complete_cost_batch(
                    s.f.tenant,
                    "QA-source-cost-done",
                    "QA-source-cost-done",
                    task,
                    "QA-owned-cost-worker",
                    "QA-UNPRICED",
                    center,
                    "2026-10-08",
                    Some(0),
                    super::qty("1"),
                    super::qty("0"),
                    super::qty("0"),
                    1,
                )
                .await?
                .map_err(|e| anyhow::anyhow!("completion rejected {e:?}"))?;
            assert_eq!(result.2, 50);
            assert!(!result.4);
        }
        let cent:i64=sqlx::query_scalar("select b.rework_cent from mfg_material_cost_bases b join mfg_cost_runs c on c.tenant_id=b.tenant_id and c.cost_run_id=b.cost_run_id where c.tenant_id=$1 and c.work_order_id=$2")
            .bind(s.f.tenant.0).bind(order).fetch_one(&read).await?;
        assert_eq!(cent, 50);
    }
    Ok(())
}

#[tokio::test]
async fn source_dispositions_rework_cost_basis_failure_rolls_back_run_and_same_id_retries()
-> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "rework-cost-basis-fault", 200).await?;
    let center = cost_center(&pool, &s).await?;
    assert_eq!(
        material(&s, order, issue, "QA-basis-consume", "CONSUME", "0.25")
            .await?
            .0,
        StatusCode::OK
    );
    let name = format!("qa_cost_basis_fault_{}", s.f.tenant.0.simple());
    sqlx::raw_sql(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then raise exception 'owned cost basis fault'; end if; return new; end $$; create trigger {name} before insert on mfg_material_cost_bases for each row execute function {name}();",s.f.tenant.0)).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let failed = confirm_cost(&s, order, center, "QA-cost-basis-retry").await?;
    sqlx::raw_sql(&format!(
        "drop trigger {name} on mfg_material_cost_bases; drop function {name}();"
    ))
    .execute(&pool)
    .await?;
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR, "{}", failed.1);
    assert_eq!(facts(&read, &s).await?, before);
    let retry = confirm_cost(&s, order, center, "QA-cost-basis-retry").await?;
    assert_eq!(retry.0, StatusCode::OK, "{}", retry.1);
    assert_eq!(retry.1["total_cent"], 50);
    Ok(())
}

async fn material_price(pool: &PgPool, s: &Source, code: &str, cent: i64, id: &str) -> Result<()> {
    erp_repo::shared_contract::SharedContractRepo::new(pool)
        .post_cost_receipt(erp_repo::shared_contract::CostReceiptInput {
            tenant_id: s.f.tenant,
            request_id: id,
            payload_hash: id,
            item_code: code,
            qty_micros: 10_000_000,
            amount_cent: cent,
            period_code: "2026-10",
        })
        .await?
        .map_err(|e| anyhow::anyhow!("cost book receipt rejected {e:?}"))?;
    Ok(())
}
async fn ordinary(
    s: &Source,
    sku: Uuid,
    kind: &str,
    qty: &str,
    source: Option<Uuid>,
    id: &str,
) -> Result<(StatusCode, Value)> {
    s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{}/materials",s.f.tenant,s.order),json!({
        "request_id":id,"warehouse_id":s.f.warehouse,"sku_id":sku,"kind":kind,"qty":qty,"original_issue_id":source,"reason":"QA per-source material"
    })).await
}
fn movement_id(body: &Value) -> Result<Uuid> {
    Ok(Uuid::parse_str(body["move_id"].as_str().ok_or_else(
        || anyhow::anyhow!("missing material movement"),
    )?)?)
}

#[tokio::test]
async fn source_dispositions_ordinary_two_skus_keep_issue_prices_after_cost_changes() -> Result<()>
{
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "ordinary-two-sku").await?;
    let center = cost_center(&pool, &s).await?;
    material_price(&pool, &s, "QA-COMMAND", 1000, "QA-price-A").await?;
    let second = Uuid::new_v4();
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-SECOND-MATERIAL',(select product_id from qa_parent))")
        .bind(s.f.tenant.0)
        .bind(second)
        .execute(&pool)
        .await?;
    erp_repo::inventory::InventoryService::new(&pool)
        .apply_precise(erp_repo::inventory_precision::InventoryPreciseDelta {
            key: erp_repo::inventory_precision::InventoryQuantityKey {
                tenant_id: s.f.tenant,
                warehouse_id: s.f.warehouse,
                sku_id: second,
            },
            on_hand: super::qty("1"),
            reserved: super::qty("0"),
            frozen: super::qty("0"),
            in_transit: super::qty("0"),
            reason_type: "QA_SECOND_MATERIAL_OPENING".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: chrono::Utc::now(),
        })
        .await?;
    material_price(&pool, &s, "QA-SECOND-MATERIAL", 3000, "QA-price-B").await?;
    let a = ordinary(&s, s.f.sku, "ISSUE", "0.5", None, "QA-normal-A").await?;
    let b = ordinary(&s, second, "ISSUE", "0.5", None, "QA-normal-B").await?;
    assert_eq!(a.0, StatusCode::OK, "{}", a.1);
    assert_eq!(b.0, StatusCode::OK, "{}", b.1);
    let a = movement_id(&a.1)?;
    let b = movement_id(&b.1)?;
    material_price(&pool, &s, "QA-COMMAND", 9000, "QA-reprice-A").await?;
    for (sku, issue, id) in [(s.f.sku, a, "QA-consume-A"), (second, b, "QA-consume-B")] {
        assert_eq!(
            ordinary(&s, sku, "CONSUME", "0.25", Some(issue), id)
                .await?
                .0,
            StatusCode::OK
        );
        assert_eq!(
            ordinary(
                &s,
                sku,
                "RETURN",
                "0.25",
                Some(issue),
                &format!("{id}-return")
            )
            .await?
            .0,
            StatusCode::OK
        );
    }
    let confirmed = confirm_cost(&s, s.order, center, "QA-normal-cost-confirm").await?;
    assert_eq!(confirmed.0, StatusCode::OK, "{}", confirmed.1);
    assert_eq!(confirmed.1["total_cent"], 101);
    let material: i64 = sqlx::query_scalar(
        "select material_cent from mfg_cost_runs where tenant_id=$1 and work_order_id=$2",
    )
    .bind(s.f.tenant.0)
    .bind(s.order)
    .fetch_one(&read)
    .await?;
    assert_eq!(material, 100);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_ordinary_unvalued_issue_never_uses_later_or_client_price() -> Result<()>
{
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "ordinary-unvalued").await?;
    let center = cost_center(&pool, &s).await?;
    let issue = ordinary(&s, s.f.sku, "ISSUE", "0.5", None, "QA-unvalued-issue").await?;
    assert_eq!(issue.0, StatusCode::OK, "{}", issue.1);
    let issue = movement_id(&issue.1)?;
    material_price(&pool, &s, "QA-COMMAND", 1000, "QA-too-late-price").await?;
    assert_eq!(
        ordinary(
            &s,
            s.f.sku,
            "CONSUME",
            "0.25",
            Some(issue),
            "QA-unvalued-consume"
        )
        .await?
        .0,
        StatusCode::OK
    );
    let before = facts(&read, &s).await?;
    let refused = confirm_cost(&s, s.order, center, "QA-refuse-unvalued-cost").await?;
    assert_eq!(refused.0, StatusCode::CONFLICT, "{}", refused.1);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_ordinary_one_cent_and_explicit_zero_remain_distinct() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    for (price, expected, case) in [(10, 1, "ordinary-tail"), (0, 0, "ordinary-zero")] {
        let s = setup(&pool, "1", None, case).await?;
        material_price(&pool, &s, "QA-COMMAND", price, "QA-small-price").await?;
        let issue = ordinary(&s, s.f.sku, "ISSUE", "1", None, "QA-small-issue").await?;
        assert_eq!(issue.0, StatusCode::OK, "{}", issue.1);
        let issue = movement_id(&issue.1)?;
        for id in ["QA-small-A", "QA-small-B"] {
            assert_eq!(
                ordinary(&s, s.f.sku, "CONSUME", "0.5", Some(issue), id)
                    .await?
                    .0,
                StatusCode::OK
            );
        }
        let total:i64=sqlx::query_scalar("select sum(cost_cent)::bigint from mfg_material_value_allocations where tenant_id=$1 and issue_id=$2")
            .bind(s.f.tenant.0).bind(issue).fetch_one(&read).await?;
        assert_eq!(total, expected);
    }
    Ok(())
}

#[tokio::test]
async fn source_dispositions_ordinary_average_residual_and_return_restore_exact_book_value()
-> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "ordinary-average-residual").await?;
    material_price(&pool, &s, "QA-COMMAND", 1, "QA-residual-book").await?;
    erp_repo::inventory::InventoryService::new(&pool)
        .apply_precise(erp_repo::inventory_precision::InventoryPreciseDelta {
            key: erp_repo::inventory_precision::InventoryQuantityKey {
                tenant_id: s.f.tenant,
                warehouse_id: s.f.warehouse,
                sku_id: s.f.sku,
            },
            on_hand: super::qty("10"),
            reserved: super::qty("0"),
            frozen: super::qty("0"),
            in_transit: super::qty("0"),
            reason_type: "QA_RESIDUAL_OPENING".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: chrono::Utc::now(),
        })
        .await?;
    let issue = ordinary(&s, s.f.sku, "ISSUE", "10", None, "QA-residual-issue").await?;
    assert_eq!(issue.0, StatusCode::OK, "{}", issue.1);
    let issue = movement_id(&issue.1)?;
    let value: i64 = sqlx::query_scalar(
        "select total_cent from mfg_material_issue_values where tenant_id=$1 and issue_id=$2",
    )
    .bind(s.f.tenant.0)
    .bind(issue)
    .fetch_one(&read)
    .await?;
    assert_eq!(value, 1);
    assert_eq!(
        ordinary(
            &s,
            s.f.sku,
            "CONSUME",
            "5",
            Some(issue),
            "QA-residual-consume"
        )
        .await?
        .0,
        StatusCode::OK
    );
    assert_eq!(
        ordinary(
            &s,
            s.f.sku,
            "RETURN",
            "5",
            Some(issue),
            "QA-residual-return"
        )
        .await?
        .0,
        StatusCode::OK
    );
    let book = erp_repo::shared_contract::SharedContractRepo::new(&read)
        .read_item_cost(s.f.tenant, "QA-COMMAND")
        .await?;
    assert_eq!((book.0, book.1), (5_000_000, 1));
    Ok(())
}

#[tokio::test]
async fn source_dispositions_ordinary_audit_failure_rolls_back_stock_book_and_value() -> Result<()>
{
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "ordinary-audit-rollback").await?;
    material_price(&pool, &s, "QA-COMMAND", 1000, "QA-fault-book").await?;
    let name = format!("qa_material_log_fault_{}", s.f.tenant.0.simple());
    sqlx::raw_sql(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid and new.action='MATERIAL_SOURCE_VALUE' then raise exception 'owned material log fault'; end if; return new; end $$; create trigger {name} before insert on operation_logs for each row execute function {name}();",s.f.tenant.0)).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let book = erp_repo::shared_contract::SharedContractRepo::new(&read)
        .read_item_cost(s.f.tenant, "QA-COMMAND")
        .await?;
    let failed = ordinary(&s, s.f.sku, "ISSUE", "0.5", None, "QA-material-fault").await?;
    sqlx::raw_sql(&format!(
        "drop trigger {name} on operation_logs; drop function {name}();"
    ))
    .execute(&pool)
    .await?;
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR, "{}", failed.1);
    assert_eq!(facts(&read, &s).await?, before);
    assert_eq!(
        erp_repo::shared_contract::SharedContractRepo::new(&read)
            .read_item_cost(s.f.tenant, "QA-COMMAND")
            .await?,
        book
    );
    let retry = ordinary(&s, s.f.sku, "ISSUE", "0.5", None, "QA-material-fault").await?;
    assert_eq!(retry.0, StatusCode::OK, "{}", retry.1);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_twenty_writers_cannot_overconsume_one_source() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "concurrent").await?;
    let mut jobs = tokio::task::JoinSet::new();
    for i in 0..20 {
        let account = s.f.login.clone();
        let path = format!("/v1/tenants/{}/quality/dispositions", s.f.tenant);
        let body = json!({"request_id":format!("QA-race-{i}"),"receipt_id":s.receipt,"location_id":Uuid::nil(),
            "warehouse_id":s.f.warehouse,"sku_id":s.f.sku,"kind":"ACCEPT","qty":"0.1","reason":"QA actual race"});
        jobs.spawn(async move { account.request("POST", &path, body).await });
    }
    let mut successes = 0;
    let mut conflicts = 0;
    while let Some(result) = jobs.join_next().await {
        let (status, body) = result??;
        match status {
            StatusCode::OK => successes += 1,
            StatusCode::CONFLICT => conflicts += 1,
            _ => anyhow::bail!("unexpected race response {status}: {body}"),
        }
    }
    assert_eq!((successes, conflicts), (10, 10));
    assert_eq!(balance(&read, &s).await?, (5_900_000, 900_000));
    let count:(i64,i64)=sqlx::query_as("select count(*)::bigint,sum(qty_micros)::bigint from mfg_receipt_disposition_commands where tenant_id=$1")
        .bind(s.f.tenant.0).fetch_one(&read).await?;
    assert_eq!(count, (10, 1_000_000));
    Ok(())
}

#[tokio::test]
async fn source_dispositions_storage_failure_rolls_back_all_effects_and_same_id_retries()
-> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "rollback").await?;
    let name = format!("qa_disposition_fault_{}", s.f.tenant.0.simple());
    let create = format!(
        "create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then raise exception 'owned quality fault'; end if; return new; end $$; create trigger {name} before insert on mfg_receipt_disposition_commands for each row execute function {name}();",
        s.f.tenant.0
    );
    sqlx::raw_sql(&create).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let failure = post(&s, "QA-retry-fault", "ACCEPT", "0.5", None, None).await?;
    sqlx::raw_sql(&format!(
        "drop trigger {name} on mfg_receipt_disposition_commands; drop function {name}();"
    ))
    .execute(&pool)
    .await?;
    assert_eq!(
        failure.0,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        failure.1
    );
    assert_eq!(facts(&read, &s).await?, before);
    let retry = post(&s, "QA-retry-fault", "ACCEPT", "0.5", None, None).await?;
    assert_eq!(retry.0, StatusCode::OK, "{}", retry.1);
    assert_eq!(balance(&read, &s).await?, (5_900_000, 1_400_000));
    Ok(())
}

#[tokio::test]
async fn source_dispositions_current_revocation_after_source_wait_is_rechecked() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "wait-revoke").await?;
    let mut lock = pool.begin().await?;
    let owner_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *lock)
        .await?;
    sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
        .bind(s.f.tenant.0).bind(s.order).execute(&mut *lock).await?;
    let account = s.f.login.clone();
    let path = format!("/v1/tenants/{}/quality/dispositions", s.f.tenant);
    let body = json!({"request_id":"QA-wait-revoke","receipt_id":s.receipt,"location_id":Uuid::nil(),"warehouse_id":s.f.warehouse,
        "sku_id":s.f.sku,"kind":"ACCEPT","qty":"0.5","reason":"QA observed queue"});
    let job = tokio::spawn(async move { account.request("POST", &path, body).await });
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        let waiting: bool = sqlx::query_scalar(
            "select exists(select 1 from pg_stat_activity where $1=any(pg_blocking_pids(pid)))",
        )
        .bind(owner_pid)
        .fetch_one(&read)
        .await?;
        if waiting {
            break;
        }
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("actual source wait not observed");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code='quality.write'")
        .bind(s.f.tenant.0)
        .execute(&read)
        .await?;
    let before = facts(&read, &s).await?;
    lock.rollback().await?;
    let response = job.await??;
    assert_eq!(response.0, StatusCode::FORBIDDEN, "{}", response.1);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_concession_needs_measured_evidence_and_current_override_grant()
-> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1.25", None, "concession").await?;
    let before = facts(&read, &s).await?;
    let no_grant = post(&s, "QA-concession-grant", "CONCESSION", "0.5", None, None).await?;
    assert_eq!(no_grant.0, StatusCode::FORBIDDEN);
    assert_eq!(facts(&read, &s).await?, before);
    sqlx::query("insert into role_permissions(tenant_id,role_id,perm_code) select tenant_id,role_id,'QUALITY.CONCESSION' from user_roles where tenant_id=$1 and user_id=$2 on conflict do nothing")
        .bind(s.f.tenant.0).bind(s.f.actor).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let missing = post(&s, "QA-concession-missing", "CONCESSION", "0.5", None, None).await?;
    assert_eq!(missing.0, StatusCode::CONFLICT);
    assert_eq!(facts(&read, &s).await?, before);
    let (status,template)=s.f.login.request("POST",&format!("/v1/tenants/{}/quality/inspection-templates",s.f.tenant),json!({
        "request_id":"QA-concession-standard","template_code":"QA-CURRENT-STANDARD","kind":"FINAL",
        "lines":[{"item_code":"WIDTH","required":true,"min_qty":"1","max_qty":"2","sample_count":1}]
    })).await?;
    assert_eq!(status, StatusCode::OK, "{template}");
    let (status,freeze)=s.f.login.request("POST",&format!("/v1/tenants/{}/quality/inspection-freezes",s.f.tenant),json!({
        "request_id":"QA-concession-freeze","template_id":template["effect_id"],"source_kind":"WORK_ORDER","source_id":s.order,
        "item_code":"QA-COMMAND","batch_code":"QA-DISPOSITION"
    })).await?;
    assert_eq!(status, StatusCode::OK, "{freeze}");
    let (status,reading)=s.f.login.request("POST",&format!("/v1/tenants/{}/quality/inspection-freezes/{}/readings",s.f.tenant,freeze["effect_id"].as_str().ok_or_else(||anyhow::anyhow!("missing freeze"))?),json!({
        "request_id":"QA-concession-reading","item_code":"WIDTH","reading_qty":"3","inspector":"QA actual inspector","reviewer":"QA actual reviewer"
    })).await?;
    assert_eq!(status, StatusCode::OK, "{reading}");
    let before = facts(&read, &s).await?;
    let blocked = post(&s, "QA-no-ordinary-accept", "ACCEPT", "0.5", None, None).await?;
    assert_eq!(blocked.0, StatusCode::CONFLICT);
    assert_eq!(facts(&read, &s).await?, before);
    let result = post(&s, "QA-actual-concession", "CONCESSION", "0.5", None, None).await?;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    assert_eq!(balance(&read, &s).await?, (6_150_000, 1_650_000));
    Ok(())
}

#[tokio::test]
async fn source_dispositions_cost_commit_revoked_role_rejects_new_and_cached_result() -> Result<()>
{
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "cost-current-role", 200).await?;
    let center = cost_center(&pool, &s).await?;
    assert_eq!(
        material(&s, order, issue, "QA-role-consume", "CONSUME", "0.25")
            .await?
            .0,
        StatusCode::OK
    );
    let cost = confirm_cost(&s, order, center, "QA-role-cost").await?;
    assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in('manufacturing.write','MFG.PRODUCTION.WRITE')")
        .bind(s.f.tenant.0).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    for id in ["QA-role-cost", "QA-new-role-cost"] {
        let denied = confirm_cost(&s, order, center, id).await?;
        assert_eq!(denied.0, StatusCode::FORBIDDEN, "{}", denied.1);
        assert_eq!(facts(&read, &s).await?, before);
    }
    Ok(())
}

#[tokio::test]
async fn source_dispositions_cost_commit_rechecks_role_after_observed_order_wait() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "cost-role-wait", 200).await?;
    let center = cost_center(&pool, &s).await?;
    assert_eq!(
        material(&s, order, issue, "QA-wait-cost-consume", "CONSUME", "0.25")
            .await?
            .0,
        StatusCode::OK
    );
    let mut lock = pool.begin().await?;
    let pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *lock)
        .await?;
    sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
        .bind(s.f.tenant.0).bind(order).execute(&mut *lock).await?;
    let account = s.f.login.clone();
    let path = format!(
        "/v1/tenants/{}/manufacturing/work-orders/{order}/costs",
        s.f.tenant
    );
    let job = tokio::spawn(async move {
        account.request("POST",&path,json!({"request_id":"QA-observed-cost-wait","item_code":"QA-NOT-A-PRICE","center_id":center,"as_of":"2026-10-08","overhead_cent":0,"completed_qty":"1","rejected_qty":"0","unfinished_qty":"0"})).await
    });
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        let blocked: bool = sqlx::query_scalar(
            "select exists(select 1 from pg_stat_activity where $1=any(pg_blocking_pids(pid)))",
        )
        .bind(pid)
        .fetch_one(&read)
        .await?;
        if blocked {
            break;
        }
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("actual cost order wait not observed");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in('manufacturing.write','MFG.PRODUCTION.WRITE')")
        .bind(s.f.tenant.0).execute(&read).await?;
    let before = facts(&read, &s).await?;
    lock.rollback().await?;
    let result = job.await??;
    assert_eq!(result.0, StatusCode::FORBIDDEN, "{}", result.1);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_cost_commit_final_operation_not_sum_or_client_quantity() -> Result<()>
{
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "cost-final-quantity").await?;
    let center = cost_center(&pool, &s).await?;
    let route = Uuid::new_v4();
    for (operation, sequence) in [("CUT", 1), ("FINAL", 2)] {
        sqlx::query("insert into shared_work_order_routes(tenant_id,work_order_id,route_id,version_no,operation_code,sequence_no,predecessor_code,work_center,setup_minutes,minutes_per,inspection_required,report_field) values($1,$2,$3,1,$4,$5,'','QA-REWORK-LINE',0,0,false,'')")
            .bind(s.f.tenant.0).bind(s.order).bind(route).bind(operation).bind(sequence).execute(&pool).await?;
    }
    sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id,started_at) values($1,$2,$3,'CUT',10000000,0,0,0,'QA-CUT-WORKER','QA-LOGIN','QA-STATION','2026-10-08T01:00:00Z')")
        .bind(s.f.tenant.0).bind(Uuid::new_v4()).bind(s.order).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let wrong=s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{}/costs",s.f.tenant,s.order),json!({"request_id":"QA-invented-cost-output","item_code":"QA-NOT-A-PRICE","center_id":center,"as_of":"2026-10-08","overhead_cent":0,"completed_qty":"20","rejected_qty":"0","unfinished_qty":"0"})).await?;
    assert_eq!(wrong.0, StatusCode::CONFLICT, "{}", wrong.1);
    assert_eq!(facts(&read, &s).await?, before);
    let correct = confirm_cost(&s, s.order, center, "QA-final-cost-output").await?;
    assert_eq!(correct.0, StatusCode::OK, "{}", correct.1);
    let proof:Value=sqlx::query_scalar("select snapshot->'quantities' from mfg_cost_commit_authorities where tenant_id=$1 and request_id='QA-final-cost-output'")
        .bind(s.f.tenant.0).fetch_one(&read).await?;
    assert_eq!(proof["final_operation"], "FINAL");
    assert_eq!(proof["good_micros"], 10_000_000);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_cost_commit_authority_failure_rolls_back_whole_cost() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, order, issue) = rework_source(&pool, "cost-authority-rollback", 200).await?;
    let center = cost_center(&pool, &s).await?;
    assert_eq!(
        material(&s, order, issue, "QA-authority-consume", "CONSUME", "0.25")
            .await?
            .0,
        StatusCode::OK
    );
    let name = format!("qa_cost_authority_fault_{}", s.f.tenant.0.simple());
    sqlx::raw_sql(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then raise exception 'owned cost authority fault'; end if; return new; end $$; create trigger {name} before insert on mfg_cost_commit_authorities for each row execute function {name}();",s.f.tenant.0)).execute(&pool).await?;
    let before = facts(&read, &s).await?;
    let failed = confirm_cost(&s, order, center, "QA-authority-retry").await?;
    sqlx::raw_sql(&format!(
        "drop trigger {name} on mfg_cost_commit_authorities;drop function {name}();"
    ))
    .execute(&pool)
    .await?;
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR, "{}", failed.1);
    assert_eq!(facts(&read, &s).await?, before);
    let retry = confirm_cost(&s, order, center, "QA-authority-retry").await?;
    assert_eq!(retry.0, StatusCode::OK, "{}", retry.1);
    Ok(())
}

async fn allocation_source(
    pool: &PgPool,
    case: &str,
    good: i64,
    wip: i64,
    bad: i64,
    amount: i64,
) -> Result<(Source, Uuid)> {
    let f = fixture(pool).await?;
    let order = Uuid::new_v4();
    let planned = good + wip + bad;
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-COST-ALLOCATION','QA-COMMAND','RELEASED',$3)")
        .bind(f.tenant.0).bind(order).bind(planned).execute(pool).await?;
    sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id,started_at) values($1,$2,$3,'FINAL',$4,$5,0,0,'QA-ALLOC-WORKER','QA-LOGIN','QA-STATION','2026-10-08T01:00:00Z')")
        .bind(f.tenant.0).bind(Uuid::new_v4()).bind(order).bind(good).bind(bad).execute(pool).await?;
    if wip > 0 {
        sqlx::query("insert into operation_wip(tenant_id,wip_id,work_order_id,location_code,owner_code,qty_micros) values($1,$2,$3,'QA-IN-PROCESS','QA-OWNER',$4)")
            .bind(f.tenant.0).bind(Uuid::new_v4()).bind(order).bind(wip).execute(pool).await?;
    }
    let s = Source {
        f,
        order,
        receipt: Uuid::nil(),
    };
    let center = cost_center(pool, &s).await?;
    material_price(pool, &s, "QA-COMMAND", amount * 10, "QA-allocation-book").await?;
    let issue = ordinary(&s, s.f.sku, "ISSUE", "1", None, "QA-allocation-issue").await?;
    assert_eq!(issue.0, StatusCode::OK, "{}", issue.1);
    let issue = movement_id(&issue.1)?;
    assert_eq!(
        ordinary(
            &s,
            s.f.sku,
            "CONSUME",
            "1",
            Some(issue),
            "QA-allocation-consume"
        )
        .await?
        .0,
        StatusCode::OK
    );
    eprintln!(
        "QA source_disposition_fixture tenant={} case={case} receipt={} order={order}",
        s.f.tenant, s.receipt
    );
    Ok((s, center))
}
async fn allocated_cost(
    s: &Source,
    center: Uuid,
    id: &str,
    good: &str,
    wip: &str,
    bad: &str,
) -> Result<(StatusCode, Value)> {
    s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{}/costs",s.f.tenant,s.order),json!({
        "request_id":id,"item_code":"QA-UNRELATED-CLIENT-CODE","center_id":center,"as_of":"2026-10-08","overhead_cent":0,"completed_qty":good,"unfinished_qty":wip,"rejected_qty":bad
    })).await
}
#[tokio::test]
async fn source_dispositions_cost_allocation_finished_wip_is_one_pool() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, center) = allocation_source(
        &pool,
        "cost-allocation-partial",
        1_000_000,
        1_000_000,
        0,
        100,
    )
    .await?;
    let cost = allocated_cost(&s, center, "QA-partial-allocation", "1", "1", "0").await?;
    assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
    assert_eq!(cost.1["total_cent"], 100);
    assert_eq!(cost.1["unit_cent"], 50);
    let amounts: (i64, i64, i64) = sqlx::query_as(
        "select finished_cent,wip_cent,rejected_cent from mfg_cost_allocations where tenant_id=$1",
    )
    .bind(s.f.tenant.0)
    .fetch_one(&read)
    .await?;
    assert_eq!(amounts, (50, 50, 0));
    let before = facts(&read, &s).await?;
    let replay = allocated_cost(&s, center, "QA-partial-allocation", "1", "1", "0").await?;
    assert_eq!(replay.0, StatusCode::OK);
    assert_eq!(replay.1["replay"], true);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}
#[tokio::test]
async fn source_dispositions_cost_allocation_fractional_finished_and_cent_tail() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    for (good, wip, bad, total, expected) in [
        (500_000, 0, 0, 100, (100, 0, 0)),
        (250_000, 250_000, 500_000, 1, (0, 0, 1)),
    ] {
        let (s, center) =
            allocation_source(&pool, "cost-allocation-fraction", good, wip, bad, total).await?;
        let cost = allocated_cost(
            &s,
            center,
            "QA-fraction-allocation",
            &erp_domain::quantity::format_qty(erp_domain::quantity::ScaledQty::from_micros(good)),
            &erp_domain::quantity::format_qty(erp_domain::quantity::ScaledQty::from_micros(wip)),
            &erp_domain::quantity::format_qty(erp_domain::quantity::ScaledQty::from_micros(bad)),
        )
        .await?;
        assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
        let amounts:(i64,i64,i64)=sqlx::query_as("select finished_cent,wip_cent,rejected_cent from mfg_cost_allocations where tenant_id=$1")
            .bind(s.f.tenant.0).fetch_one(&read).await?;
        assert_eq!(amounts, expected);
        assert_eq!(amounts.0 + amounts.1 + amounts.2, total);
    }
    Ok(())
}
#[tokio::test]
async fn source_dispositions_cost_allocation_wip_only_has_no_unit_price() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, center) =
        allocation_source(&pool, "cost-allocation-wip-only", 0, 1_000_000, 0, 100).await?;
    let cost = allocated_cost(&s, center, "QA-wip-allocation", "0", "1", "0").await?;
    assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
    assert!(cost.1["unit_cent"].is_null());
    let unit: Option<i64> =
        sqlx::query_scalar("select unit_cent from mfg_cost_runs where tenant_id=$1")
            .bind(s.f.tenant.0)
            .fetch_one(&read)
            .await?;
    assert_eq!(unit, None);
    Ok(())
}
#[tokio::test]
async fn source_dispositions_cost_allocation_estimate_derives_wip_and_keeps_fee_unknown()
-> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let (s, center) = allocation_source(
        &pool,
        "cost-allocation-estimate",
        1_000_000,
        1_000_000,
        0,
        100,
    )
    .await?;
    let cost=s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{}/cost-estimates",s.f.tenant,s.order),json!({"request_id":"QA-estimated-allocation","item_code":"QA-UNRELATED","center_id":center,"as_of":"2026-10-08","completed_qty":"1"})).await?;
    assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
    assert_eq!(cost.1["unit_cent"], 50);
    assert_eq!(cost.1["estimated"], true);
    let fee: Option<i64> =
        sqlx::query_scalar("select overhead_cent from mfg_cost_runs where tenant_id=$1")
            .bind(s.f.tenant.0)
            .fetch_one(&read)
            .await?;
    assert_eq!(fee, None);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_labor_cost_uses_work_date_not_client_rate_date() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "labor-work-date").await?;
    let center = cost_center(&pool, &s).await?;
    let repo = erp_repo::shared_contract::SharedContractRepo::new(&pool);
    repo.save_work_center_rate(
        s.f.tenant,
        "QA-future-rate",
        "QA-future-rate",
        center,
        100,
        "2026-11-01",
        "",
        "2026-11-01",
    )
    .await?
    .map_err(|e| anyhow::anyhow!("future rate {e:?}"))?;
    let cost=s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{}/costs",s.f.tenant,s.order),json!({"request_id":"QA-work-date-cost","item_code":"QA-NOT-A-PRICE","center_id":center,"as_of":"2026-11-01","overhead_cent":0,"completed_qty":"10","rejected_qty":"0","unfinished_qty":"0"})).await?;
    assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
    assert_eq!(cost.1["total_cent"], 1);
    let basis: Value =
        sqlx::query_scalar("select snapshot from mfg_cost_labor_bases where tenant_id=$1")
            .bind(s.f.tenant.0)
            .fetch_one(&read)
            .await?;
    assert_eq!(basis["reports"][0]["work_date"], "2026-10-08");
    assert_eq!(basis["reports"][0]["rate"]["rate_cent"], 1);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_labor_binding_cannot_switch_to_another_cheap_center() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "labor-center-binding").await?;
    let center = cost_center(&pool, &s).await?;
    let repo = erp_repo::shared_contract::SharedContractRepo::new(&pool);
    let (_, other) = repo
        .save_work_center_calendar(
            s.f.tenant,
            "QA-other-center",
            "QA-other-center",
            "QA-OTHER-CENTER",
            1,
            &[erp_domain::manufacturing::CalendarDay {
                day: "2026-10-08".into(),
                rest: false,
                shift_minutes: 480,
            }],
        )
        .await?
        .map_err(|e| anyhow::anyhow!("other center {e:?}"))?;
    let before = facts(&read, &s).await?;
    let rebind =
        s.f.login
            .request(
                "POST",
                &format!(
                    "/v1/tenants/{}/manufacturing/work-orders/{}/cost-center-binding",
                    s.f.tenant, s.order
                ),
                json!({"center_id":other,"reason":"QA attempted cost replacement"}),
            )
            .await?;
    assert_eq!(rebind.0, StatusCode::CONFLICT, "{}", rebind.1);
    assert_eq!(facts(&read, &s).await?, before);
    let wrong=s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{}/costs",s.f.tenant,s.order),json!({"request_id":"QA-wrong-cost-center","item_code":"QA-NOT-A-PRICE","center_id":other,"as_of":"2026-10-08","overhead_cent":0,"completed_qty":"10","rejected_qty":"0","unfinished_qty":"0"})).await?;
    assert_eq!(wrong.0, StatusCode::CONFLICT, "{}", wrong.1);
    assert_eq!(facts(&read, &s).await?, before);
    let early=s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{}/costs",s.f.tenant,s.order),json!({"request_id":"QA-before-work-date","item_code":"QA-NOT-A-PRICE","center_id":center,"as_of":"2026-10-01","overhead_cent":0,"completed_qty":"10","rejected_qty":"0","unfinished_qty":"0"})).await?;
    assert_eq!(early.0, StatusCode::CONFLICT, "{}", early.1);
    assert_eq!(facts(&read, &s).await?, before);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_labor_reports_keep_distinct_dated_rates() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let f = fixture(&pool).await?;
    let order = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-DATED-LABOR','QA-COMMAND','RELEASED',2000000)").bind(f.tenant.0).bind(order).execute(&pool).await?;
    for (date, id) in [
        ("2026-10-01T10:00:00Z", "QA-day-A"),
        ("2026-10-02T10:00:00Z", "QA-day-B"),
    ] {
        sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id,started_at) values($1,$2,$3,'FINAL',1000000,0,0,10,$4,'QA-LOGIN','QA-STATION',$5)").bind(f.tenant.0).bind(Uuid::new_v4()).bind(order).bind(id).bind(date).execute(&pool).await?;
    }
    let s = Source {
        f,
        order,
        receipt: Uuid::nil(),
    };
    let repo = erp_repo::shared_contract::SharedContractRepo::new(&pool);
    let (_, center) = repo
        .save_work_center_calendar(
            s.f.tenant,
            "QA-dated-center",
            "QA-dated-center",
            "QA-DATED-CENTER",
            1,
            &[erp_domain::manufacturing::CalendarDay {
                day: "2026-10-01".into(),
                rest: false,
                shift_minutes: 480,
            }],
        )
        .await?
        .map_err(|e| anyhow::anyhow!("dated center {e:?}"))?;
    repo.save_work_center_rate(
        s.f.tenant,
        "QA-day-A-rate",
        "QA-day-A-rate",
        center,
        2,
        "2026-10-01",
        "2026-10-01",
        "2026-10-01",
    )
    .await?
    .map_err(|e| anyhow::anyhow!("A rate {e:?}"))?;
    repo.save_work_center_rate(
        s.f.tenant,
        "QA-day-B-rate",
        "QA-day-B-rate",
        center,
        3,
        "2026-10-02",
        "",
        "2026-10-02",
    )
    .await?
    .map_err(|e| anyhow::anyhow!("B rate {e:?}"))?;
    let bound =
        s.f.login
            .request(
                "POST",
                &format!(
                    "/v1/tenants/{}/manufacturing/work-orders/{order}/cost-center-binding",
                    s.f.tenant
                ),
                json!({"center_id":center,"reason":"QA explicit dated work center"}),
            )
            .await?;
    assert_eq!(bound.0, StatusCode::OK, "{}", bound.1);
    let cost=s.f.login.request("POST",&format!("/v1/tenants/{}/manufacturing/work-orders/{order}/costs",s.f.tenant),json!({"request_id":"QA-multi-day-cost","item_code":"QA-NOT-A-PRICE","center_id":center,"as_of":"2026-10-02","overhead_cent":0,"completed_qty":"2","rejected_qty":"0","unfinished_qty":"0"})).await?;
    assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
    assert_eq!(cost.1["total_cent"], 50);
    let labour: i64 =
        sqlx::query_scalar("select labor_cent from mfg_cost_labor_bases where tenant_id=$1")
            .bind(s.f.tenant.0)
            .fetch_one(&read)
            .await?;
    assert_eq!(labour, 50);
    eprintln!(
        "QA source_disposition_fixture tenant={} case=labor-multi-day receipt={} order={order}",
        s.f.tenant, s.receipt
    );
    Ok(())
}

#[tokio::test]
async fn source_dispositions_labor_reversal_keeps_original_work_date_and_rate() -> Result<()> {
    let pool = isolated_pool().await?;
    let read = isolated_pool().await?;
    let s = setup(&pool, "1", None, "labor-source-reversal").await?;
    let center = cost_center(&pool, &s).await?;
    let original: Uuid = sqlx::query_scalar(
        "select report_id from mfg_reports where tenant_id=$1 and work_order_id=$2",
    )
    .bind(s.f.tenant.0)
    .bind(s.order)
    .fetch_one(&pool)
    .await?;
    let repo = erp_repo::shared_contract::SharedContractRepo::new(&pool);
    repo.save_work_center_rate(
        s.f.tenant,
        "QA-retro-rate",
        "QA-retro-rate",
        center,
        100,
        "2026-10-01",
        "",
        "2026-10-08",
    )
    .await?
    .map_err(|e| anyhow::anyhow!("retro rate {e:?}"))?;
    sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id,started_at,reverses_report_id) values($1,$2,$3,'FINAL',-10000000,0,0,-1,'QA-REV-WORKER','QA-LOGIN','QA-STATION','2026-11-01T10:00:00Z',$4)").bind(s.f.tenant.0).bind(Uuid::new_v4()).bind(s.order).bind(original).execute(&pool).await?;
    sqlx::query("insert into operation_wip(tenant_id,wip_id,work_order_id,location_code,owner_code,qty_micros) values($1,$2,$3,'QA-IN-PROCESS','QA-OWNER',10000000)").bind(s.f.tenant.0).bind(Uuid::new_v4()).bind(s.order).execute(&pool).await?;
    let cost = allocated_cost(&s, center, "QA-reversed-labor-cost", "0", "10", "0").await?;
    assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
    assert_eq!(cost.1["total_cent"], 0);
    assert!(cost.1["unit_cent"].is_null());
    let snapshot: Value =
        sqlx::query_scalar("select snapshot from mfg_cost_labor_bases where tenant_id=$1")
            .bind(s.f.tenant.0)
            .fetch_one(&read)
            .await?;
    assert_eq!(snapshot["labor_cent"], 0);
    for row in snapshot["reports"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("labor reports missing"))?
    {
        assert_eq!(row["work_date"], "2026-10-08");
        assert_eq!(row["rate"]["rate_cent"], 1);
    }
    Ok(())
}

async fn read_cost_evidence(s: &Source, run: Uuid) -> Result<(StatusCode, Value)> {
    let result =
        s.f.login
            .request(
                "GET",
                &format!(
                    "/v1/tenants/{}/manufacturing/work-orders/{}/costs/{run}",
                    s.f.tenant, s.order
                ),
                json!({}),
            )
            .await?;
    eprintln!(
        "QA cost_read HTTP tenant={} order={} run={run} status={} body={}",
        s.f.tenant,
        s.order,
        result.0.as_u16(),
        result.1
    );
    Ok(result)
}

#[tokio::test]
async fn source_dispositions_cost_read_original_proofs_and_separate_adjustments() -> Result<()> {
    let pool = isolated_pool().await?;
    let (s, center) =
        allocation_source(&pool, "cost-read-proof", 1_000_000, 1_000_000, 0, 100).await?;
    let cost = allocated_cost(&s, center, "QA-read-cost", "1", "1", "0").await?;
    assert_eq!(cost.0, StatusCode::OK, "{}", cost.1);
    let run: Uuid = sqlx::query_scalar(
        "select cost_run_id from mfg_cost_runs where tenant_id=$1 and work_order_id=$2",
    )
    .bind(s.f.tenant.0)
    .bind(s.order)
    .fetch_one(&pool)
    .await?;
    let before = read_cost_evidence(&s, run).await?;
    assert_eq!(before.0, StatusCode::OK, "{}", before.1);
    assert_eq!(before.1["reconciliation_required"], false, "{}", before.1);
    assert_eq!(before.1["original"]["total_cent"], 100);
    assert_eq!(before.1["evidence"]["allocation"]["finished_cent"], 50);
    assert_eq!(
        before.1["evidence"]["allocation"]["unit_cent_micros"],
        "50000000"
    );
    assert_eq!(
        before.1["evidence"]["authority"]["user_id"],
        s.f.actor.to_string()
    );
    let adjustment =
        s.f.login
            .request(
                "POST",
                &format!(
                    "/v1/tenants/{}/manufacturing/work-orders/{}/cost-adjustments",
                    s.f.tenant, s.order
                ),
                json!({"request_id":"QA-read-late-fee","fee_cent":7}),
            )
            .await?;
    assert_eq!(adjustment.0, StatusCode::OK, "{}", adjustment.1);
    let after = read_cost_evidence(&s, run).await?;
    assert_eq!(after.0, StatusCode::OK);
    assert_eq!(after.1["original"], before.1["original"]);
    assert_eq!(after.1["evidence"], before.1["evidence"]);
    assert_eq!(after.1["work_order_adjustments"][0]["fee_cent"], 7);
    let counts: (i64,i64,i64) = sqlx::query_as("select (select count(*) from mfg_cost_runs where tenant_id=$1),(select count(*) from operation_logs where tenant_id=$1 and action='MFG_COST_COMMIT'),(select count(*) from mfg_cost_commit_authorities where tenant_id=$1)").bind(s.f.tenant.0).fetch_one(&pool).await?;
    assert_eq!(counts, (1, 1, 1));
    Ok(())
}

#[tokio::test]
async fn source_dispositions_cost_read_current_read_grant_warehouse_and_session() -> Result<()> {
    let pool = isolated_pool().await?;
    let (s, center) = allocation_source(&pool, "cost-read-authority", 1_000_000, 0, 0, 100).await?;
    let cost = allocated_cost(&s, center, "QA-read-authority", "1", "0", "0").await?;
    assert_eq!(cost.0, StatusCode::OK);
    let run: Uuid = sqlx::query_scalar("select cost_run_id from mfg_cost_runs where tenant_id=$1")
        .bind(s.f.tenant.0)
        .fetch_one(&pool)
        .await?;
    sqlx::query("delete from role_permissions where tenant_id=$1 and perm_code in('manufacturing.write','MFG.PRODUCTION.WRITE')").bind(s.f.tenant.0).execute(&pool).await?;
    assert_eq!(read_cost_evidence(&s, run).await?.0, StatusCode::OK);
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(s.f.tenant.0)
        .bind(s.f.actor)
        .execute(&pool)
        .await?;
    assert_eq!(read_cost_evidence(&s, run).await?.0, StatusCode::NOT_FOUND);
    sqlx::query(
        "insert into user_warehouse_access(tenant_id,user_id,warehouse_id) values($1,$2,$3)",
    )
    .bind(s.f.tenant.0)
    .bind(s.f.actor)
    .bind(s.f.warehouse)
    .execute(&pool)
    .await?;
    sqlx::query(
        "delete from role_permissions where tenant_id=$1 and perm_code='manufacturing.read'",
    )
    .bind(s.f.tenant.0)
    .execute(&pool)
    .await?;
    assert_eq!(read_cost_evidence(&s, run).await?.0, StatusCode::FORBIDDEN);
    sqlx::query("insert into role_permissions(tenant_id,role_id,perm_code) select tenant_id,role_id,'manufacturing.read' from user_roles where tenant_id=$1 and user_id=$2").bind(s.f.tenant.0).bind(s.f.actor).execute(&pool).await?;
    sqlx::query("update auth_sessions set status='REVOKED' where tenant_id=$1 and session_id=$2")
        .bind(s.f.tenant.0)
        .bind(s.f.login.session)
        .execute(&pool)
        .await?;
    assert_eq!(
        read_cost_evidence(&s, run).await?.0,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[tokio::test]
async fn source_dispositions_cost_read_legacy_missing_and_unrelated_identity() -> Result<()> {
    let pool = isolated_pool().await?;
    let s = setup(&pool, "1", Some(9), "cost-read-legacy").await?;
    let run: Uuid = sqlx::query_scalar("select cost_run_id from mfg_cost_runs where tenant_id=$1")
        .bind(s.f.tenant.0)
        .fetch_one(&pool)
        .await?;
    let old = read_cost_evidence(&s, run).await?;
    assert_eq!(old.0, StatusCode::OK, "{}", old.1);
    assert_eq!(old.1["original"]["total_cent"], 9);
    assert_eq!(old.1["reconciliation_required"], true);
    assert_eq!(old.1["evidence_issues"].as_array().map(Vec::len), Some(5));
    assert!(old.1["evidence"]["material"].is_null());
    assert_eq!(
        read_cost_evidence(&s, Uuid::new_v4()).await?.0,
        StatusCode::NOT_FOUND
    );
    let other = setup(&pool, "1", None, "cost-read-other").await?;
    assert_eq!(
        read_cost_evidence(&other, run).await?.0,
        StatusCode::NOT_FOUND
    );
    let wrong_order =
        s.f.login
            .request(
                "GET",
                &format!(
                    "/v1/tenants/{}/manufacturing/work-orders/{}/costs/{run}",
                    s.f.tenant, other.order
                ),
                json!({}),
            )
            .await?;
    assert_eq!(wrong_order.0, StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_cost_read_rejects_oversized_evidence() -> Result<()> {
    let pool = isolated_pool().await?;
    let s = setup(&pool, "1", Some(9), "cost-read-budget").await?;
    let run: Uuid = sqlx::query_scalar("select cost_run_id from mfg_cost_runs where tenant_id=$1")
        .bind(s.f.tenant.0)
        .fetch_one(&pool)
        .await?;
    sqlx::query("insert into mfg_cost_labor_bases(tenant_id,cost_run_id,labor_cent,snapshot) values($1,$2,0,jsonb_build_object('labor_cent',0,'reports','[]'::jsonb,'padding',repeat('x',4194304)))").bind(s.f.tenant.0).bind(run).execute(&pool).await?;
    let result = read_cost_evidence(&s, run).await?;
    assert_eq!(result.0, StatusCode::CONFLICT, "{}", result.1);
    assert_eq!(result.1["reconciliation_required"], true);
    Ok(())
}

#[tokio::test]
async fn source_dispositions_cost_read_flags_inconsistent_legacy_totals() -> Result<()> {
    let pool = isolated_pool().await?;
    let s = setup(&pool, "1", Some(9), "cost-read-inconsistent-legacy").await?;
    let run: Uuid = sqlx::query_scalar("select cost_run_id from mfg_cost_runs where tenant_id=$1")
        .bind(s.f.tenant.0)
        .fetch_one(&pool)
        .await?;
    sqlx::query("update mfg_cost_runs set total_cent=10 where tenant_id=$1 and cost_run_id=$2")
        .bind(s.f.tenant.0)
        .bind(run)
        .execute(&pool)
        .await?;
    let result = read_cost_evidence(&s, run).await?;
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    assert_eq!(result.1["original"]["total_cent"], 10);
    assert_eq!(result.1["reconciliation_required"], true);
    assert!(
        result.1["evidence_issues"]
            .as_array()
            .is_some_and(|issues| issues
                .iter()
                .any(|issue| issue == "inconsistent_original_total"))
    );
    Ok(())
}
