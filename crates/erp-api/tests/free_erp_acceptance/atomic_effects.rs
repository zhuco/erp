//! Atomic manufacturing must derive admission from real business and authenticated scope.
use super::{
    Result,
    inspection_outcome_edges::owned_facts,
    isolated_pool,
    production_receipt_release::{Receipt, http, setup},
};
use axum::http::StatusCode;
use chrono::Utc;
use erp_domain::quantity::parse_qty;
use erp_repo::{
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
use uuid::Uuid;

async fn context(pool: &PgPool, case: &str) -> Result<Receipt> {
    let r = setup(pool, case).await?;
    sqlx::query("update tenants set name='QA-ERP-ATOMIC-EFFECTS' where tenant_id=$1")
        .bind(r.f.tenant.0)
        .execute(pool)
        .await?;
    sqlx::query("update skus set cost_cent=1200 where tenant_id=$1 and sku_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.f.sku)
        .execute(pool)
        .await?;
    // Owned database graph preparation, not a public BOM publish/release acceptance.
    let source = Uuid::new_v4();
    let version = Uuid::new_v4();
    sqlx::query("insert into mfg_boms(tenant_id,bom_id,product_code,status,version,base_qty_micros) values($1,$2,'QA-P','PUBLISHED',1,1000000)")
        .bind(r.f.tenant.0).bind(source).execute(pool).await?;
    sqlx::query("insert into mfg_bom_items(tenant_id,bom_id,line_no,component_code,qty_per_micros) values($1,$2,1,'QA-COMMAND',1000000)")
        .bind(r.f.tenant.0).bind(source).execute(pool).await?;
    sqlx::query("insert into shared_mfg_bom_versions(tenant_id,version_id,series_code,version_no,status,start_date,source_bom_id) values($1,$2,'QA-P',1,'PUBLISHED','2026-10-01',$3)")
        .bind(r.f.tenant.0).bind(version).bind(source).execute(pool).await?;
    sqlx::query("insert into shared_mfg_bom_version_lines(tenant_id,version_id,line_no,parent_code,component_code,qty_per_micros,unit_code,scrap_ppm,supply_kind) values($1,$2,1,'QA-P','QA-COMMAND',1000000,'kg',0,'PURCHASE')")
        .bind(r.f.tenant.0).bind(version).execute(pool).await?;
    sqlx::query("update mfg_work_orders set bom_id=$3 where tenant_id=$1 and work_order_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.work_order)
        .bind(version)
        .execute(pool)
        .await?;
    eprintln!(
        "QA atomic_effect_fixture tenant={} case={case} actor={} warehouse={} sku={} work_order={}",
        r.f.tenant, r.f.actor, r.f.warehouse, r.f.sku, r.work_order
    );
    Ok(r)
}

async fn facts(pool: &PgPool, r: &Receipt) -> Result<Value> {
    let mut result = owned_facts(pool, r).await?;
    for table in [
        "atomic_source_documents",
        "atomic_posting_events",
        "mfg_material_moves",
        "mfg_qty_balances",
        "inventory_reservations",
        "skus",
        "user_warehouse_access",
        "atomic_material_values",
        "shared_mfg_bom_versions",
        "shared_mfg_bom_version_lines",
        "mfg_boms",
        "mfg_bom_items",
    ] {
        result[table] = sqlx::query_scalar(&format!(
            "select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from {table} x where tenant_id=$1"
        ))
        .bind(r.f.tenant.0)
        .fetch_one(pool)
        .await?;
    }
    Ok(result)
}

fn body(r: &Receipt, request: &str, warehouse: Uuid, amount: i64) -> Value {
    json!({
        "request_id":request,"warehouse_id":warehouse,"sku_id":r.f.sku,
        "source_action":"ISSUE","action_no":Uuid::new_v4().to_string(),
        "qty":"1","amount_cent":amount,"prepared_lines":1,"failing_line":false
    })
}

async fn refuse(r: &Receipt, case: &str, input: Value, expected: StatusCode) -> Result<()> {
    let independent = isolated_pool().await?;
    let before = facts(&independent, r).await?;
    eprintln!(
        "QA atomic_effect_before tenant={} facts={before}",
        r.f.tenant
    );
    let (status, response) = http(
        &r.f,
        &format!("work-orders/{}/atomic-effects", r.work_order),
        input.clone(),
    )
    .await?;
    let after = facts(&independent, r).await?;
    eprintln!(
        "QA atomic_effect_result tenant={} case={case} request={} status={} unchanged={} input={input} response={response} after={after}",
        r.f.tenant,
        input["request_id"].as_str().unwrap_or_default(),
        status.as_u16(),
        before == after
    );
    assert_eq!(status, expected, "{case}: actual HTTP {response}");
    let error = response["text"]
        .as_str()
        .or_else(|| response["message"].as_str())
        .unwrap_or_default();
    match case {
        "terminal-order" => assert!(error.contains("not open"), "{response}"),
        "cost-claim" => assert!(error.contains("server cost source"), "{response}"),
        "warehouse-scope" => assert!(error.contains("warehouse access denied"), "{response}"),
        "quality-source" => assert!(error.contains("inspection is incomplete"), "{response}"),
        _ => {}
    }
    assert_eq!(
        before, after,
        "{case}: every selected owned fact remains unchanged"
    );
    Ok(())
}

#[tokio::test]
async fn terminal_order_cannot_issue() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "terminal-order").await?;
    sqlx::query(
        "update mfg_work_orders set status='CLOSED' where tenant_id=$1 and work_order_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(r.work_order)
    .execute(&pool)
    .await?;
    refuse(
        &r,
        "terminal-order",
        body(&r, &Uuid::new_v4().to_string(), r.f.warehouse, 1200),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await
}

#[tokio::test]
async fn client_amount_cannot_set_material_value() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "cost-claim").await?;
    refuse(
        &r,
        "cost-claim",
        body(&r, &Uuid::new_v4().to_string(), r.f.warehouse, 999999),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await
}

#[tokio::test]
async fn current_warehouse_scope_cannot_be_bypassed() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "warehouse-scope").await?;
    let other = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Atomic ungranted warehouse',$3)")
        .bind(r.f.tenant.0).bind(other).bind(other.to_string()).execute(&pool).await?;
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: r.f.tenant,
                warehouse_id: other,
                sku_id: r.f.sku,
            },
            on_hand: parse_qty("4")?,
            reserved: parse_qty("0")?,
            frozen: parse_qty("0")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_ATOMIC_OPENING".into(),
            ref_type: Some("QA_ATOMIC_OPENING".into()),
            ref_id: Some(Uuid::new_v4().to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    refuse(
        &r,
        "warehouse-scope",
        body(&r, &Uuid::new_v4().to_string(), other, 1200),
        StatusCode::FORBIDDEN,
    )
    .await
}

#[tokio::test]
async fn required_quality_cannot_be_bypassed() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "quality-source").await?;
    let (status, template) = http(&r.f, "quality/inspection-templates", json!({
        "request_id":Uuid::new_v4().to_string(),"template_code":"QA-ATOMIC-STANDARD","kind":"FINAL",
        "lines":[{"item_code":"WIDTH","required":true,"min_qty":"1","max_qty":"2","sample_count":1}]
    })).await?;
    assert_eq!(status, StatusCode::OK, "actual standard {template}");
    let (status, freeze) = http(&r.f, "quality/inspection-freezes", json!({
        "request_id":Uuid::new_v4().to_string(),"template_id":template["effect_id"],
        "source_kind":"WORK_ORDER","source_id":r.work_order,"item_code":"QA-P","batch_code":"QA-ATOMIC"
    })).await?;
    assert_eq!(status, StatusCode::OK, "actual work-order freeze {freeze}");
    refuse(
        &r,
        "quality-source",
        body(&r, &Uuid::new_v4().to_string(), r.f.warehouse, 1200),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await
}

async fn post(r: &Receipt, input: Value, expected: StatusCode, readonly: bool) -> Result<Value> {
    let pool = isolated_pool().await?;
    let before = facts(&pool, r).await?;
    let (status, response) = http(
        &r.f,
        &format!("work-orders/{}/atomic-effects", r.work_order),
        input.clone(),
    )
    .await?;
    let after = facts(&pool, r).await?;
    eprintln!(
        "QA atomic_effect_observation tenant={} request={} status={} unchanged={} input={input} response={response}",
        r.f.tenant,
        input["request_id"],
        status.as_u16(),
        before == after
    );
    assert_eq!(status, expected, "{response}");
    if readonly {
        assert_eq!(
            before, after,
            "original facts cannot change on a refusal/replay"
        );
    }
    Ok(response)
}

async fn final_facts(r: &Receipt, case: &str) -> Result<Value> {
    let pool = isolated_pool().await?;
    let result = facts(&pool, r).await?;
    eprintln!(
        "QA atomic_effect_final tenant={} case={case} facts={result}",
        r.f.tenant
    );
    Ok(result)
}

fn quantity(r: &Receipt, qty: &str, amount: i64) -> Value {
    let mut result = body(r, &Uuid::new_v4().to_string(), r.f.warehouse, amount);
    result["qty"] = json!(qty);
    result
}

#[tokio::test]
async fn fractional_value_and_original_replay_are_precise() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "fractional-original").await?;
    let previous: i64 = sqlx::query_scalar("select on_hand_micros from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
        .bind(r.f.tenant.0).bind(r.f.warehouse).bind(r.f.sku).fetch_one(&pool).await?;
    let input = quantity(&r, "0.25", 300);
    let original = post(&r, input.clone(), StatusCode::OK, false).await?;
    assert_eq!(original["snapshot"]["qty_micros"], "250000");
    assert_eq!(original["snapshot"]["value_cent_micros"], "300000000");
    assert_eq!(
        original["snapshot"]["stock"]["on_hand_micros"],
        (previous - 250000).to_string()
    );
    assert_eq!(original["snapshot"]["material"]["shop_micros"], "250000");
    let mut changed = input.clone();
    changed["qty"] = json!("0.3");
    changed["amount_cent"] = json!(360);
    post(&r, changed, StatusCode::CONFLICT, true).await?;
    let mut duplicate = input.clone();
    duplicate["request_id"] = json!(Uuid::new_v4().to_string());
    post(&r, duplicate, StatusCode::CONFLICT, true).await?;
    sqlx::query("update skus set cost_cent=2400 where tenant_id=$1 and sku_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.f.sku)
        .execute(&pool)
        .await?;
    sqlx::query(
        "update mfg_work_orders set status='CLOSED' where tenant_id=$1 and work_order_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(r.work_order)
    .execute(&pool)
    .await?;
    let replay = post(&r, input, StatusCode::OK, true).await?;
    assert_eq!(replay["replay"], true);
    assert_eq!(original["snapshot"], replay["snapshot"]);
    let rows = final_facts(&r, "fractional-original").await?;
    assert_eq!(
        rows["atomic_material_values"].as_array().map(Vec::len),
        Some(1)
    );
    Ok(())
}

#[tokio::test]
async fn minimum_quantity_keeps_subcent_value() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "minimum-quantity").await?;
    let result = post(&r, quantity(&r, "0.000001", 0), StatusCode::OK, false).await?;
    assert_eq!(result["snapshot"]["value_cent_micros"], "1200");
    assert_eq!(result["snapshot"]["amount_cent"], 0);
    assert_eq!(result["snapshot"]["material"]["issued_micros"], "1");
    final_facts(&r, "minimum-quantity").await?;
    Ok(())
}

#[tokio::test]
async fn known_zero_cost_is_distinct_from_unknown() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "zero-cost").await?;
    sqlx::query("update skus set cost_cent=0 where tenant_id=$1 and sku_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.f.sku)
        .execute(&pool)
        .await?;
    let result = post(&r, quantity(&r, "0.2", 0), StatusCode::OK, false).await?;
    assert_eq!(result["snapshot"]["unit_cost_cent"], 0);
    assert_eq!(result["snapshot"]["value_cent_micros"], "0");
    final_facts(&r, "zero-cost").await?;
    Ok(())
}

async fn invalid_cost(cost: Option<i64>, case: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, case).await?;
    sqlx::query("update skus set cost_cent=$3 where tenant_id=$1 and sku_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.f.sku)
        .bind(cost)
        .execute(&pool)
        .await?;
    let response = post(&r, quantity(&r, "1", 1200), StatusCode::CONFLICT, true).await?;
    assert_eq!(response["code"], "atomic_material_reconciliation_required");
    final_facts(&r, case).await?;
    Ok(())
}
#[tokio::test]
async fn unknown_cost_cannot_be_filled_by_the_client() -> Result<()> {
    invalid_cost(None, "unknown-cost").await
}
#[tokio::test]
async fn negative_cost_cannot_post() -> Result<()> {
    invalid_cost(Some(-1), "negative-cost").await
}

#[tokio::test]
async fn actual_work_order_bom_is_required() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "missing-bom").await?;
    sqlx::query("update mfg_work_orders set bom_id=null where tenant_id=$1 and work_order_id=$2")
        .bind(r.f.tenant.0)
        .bind(r.work_order)
        .execute(&pool)
        .await?;
    let result = post(&r, quantity(&r, "1", 1200), StatusCode::CONFLICT, true).await?;
    assert_eq!(result["code"], "atomic_material_reconciliation_required");
    final_facts(&r, "missing-bom").await?;
    Ok(())
}

#[tokio::test]
async fn work_order_demand_is_not_a_client_claim() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "excess-demand").await?;
    sqlx::query("update mfg_work_orders set order_qty_micros=100000 where tenant_id=$1 and work_order_id=$2").bind(r.f.tenant.0).bind(r.work_order).execute(&pool).await?;
    let result = post(
        &r,
        quantity(&r, "0.25", 300),
        StatusCode::UNPROCESSABLE_ENTITY,
        true,
    )
    .await?;
    assert!(
        result["text"]
            .as_str()
            .is_some_and(|value| value.contains("exceeds"))
    );
    final_facts(&r, "excess-demand").await?;
    Ok(())
}

#[tokio::test]
async fn another_component_cannot_be_issued() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "wrong-component").await?;
    sqlx::query(
        "update shared_mfg_bom_version_lines set component_code='OTHER' where tenant_id=$1",
    )
    .bind(r.f.tenant.0)
    .execute(&pool)
    .await?;
    post(&r, quantity(&r, "1", 1200), StatusCode::CONFLICT, true).await?;
    final_facts(&r, "wrong-component").await?;
    Ok(())
}

#[tokio::test]
async fn client_preflight_cannot_claim_a_whole_document() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "claimed-lines").await?;
    let mut input = quantity(&r, "1", 1200);
    input["prepared_lines"] = json!(100);
    post(&r, input, StatusCode::BAD_REQUEST, true).await?;
    final_facts(&r, "claimed-lines").await?;
    Ok(())
}

#[tokio::test]
async fn fake_action_cannot_create_an_issue() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "fake-action").await?;
    let mut input = quantity(&r, "1", 1200);
    input["source_action"] = json!("CONSUME");
    post(&r, input, StatusCode::UNPROCESSABLE_ENTITY, true).await?;
    final_facts(&r, "fake-action").await?;
    Ok(())
}

#[tokio::test]
async fn revoked_warehouse_cannot_read_original_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "revoked-replay").await?;
    let input = quantity(&r, "0.25", 300);
    post(&r, input.clone(), StatusCode::OK, false).await?;
    let other = Uuid::new_v4();
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'Atomic replacement grant',$3)").bind(r.f.tenant.0).bind(other).bind(other.to_string()).execute(&pool).await?;
    sqlx::query(
        "update user_warehouse_access set warehouse_id=$3 where tenant_id=$1 and user_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(r.f.actor)
    .bind(other)
    .execute(&pool)
    .await?;
    post(&r, input, StatusCode::FORBIDDEN, true).await?;
    final_facts(&r, "revoked-replay").await?;
    Ok(())
}

#[tokio::test]
async fn own_reservation_is_spent_without_free_stock() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "own-reservation").await?;
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: r.f.tenant,
                warehouse_id: r.f.warehouse,
                sku_id: r.f.sku,
            },
            on_hand: parse_qty("0")?,
            reserved: parse_qty("3.1")?,
            frozen: parse_qty("0")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_ATOMIC_RESERVATION".into(),
            ref_type: Some("QA_ATOMIC_RESERVATION".into()),
            ref_id: Some(r.work_order.to_string()),
            occurred_at: Utc::now(),
        })
        .await?;
    sqlx::query("insert into inventory_reservations(tenant_id,warehouse_id,sku_id,qty,qty_micros,status,source_type,source_id) values($1,$2,$3,0,250000,'RESERVED','MFG_WORK_ORDER',$4)")
        .bind(r.f.tenant.0).bind(r.f.warehouse).bind(r.f.sku).bind(r.work_order.to_string()).execute(&pool).await?;
    let result = post(&r, quantity(&r, "0.25", 300), StatusCode::OK, false).await?;
    assert_eq!(result["snapshot"]["stock"]["reserved_micros"], "3750000");
    assert_eq!(result["snapshot"]["stock"]["available_micros"], "0");
    let rows = final_facts(&r, "own-reservation").await?;
    assert_eq!(rows["inventory_reservations"][0]["qty_micros"], 0);
    assert_eq!(rows["inventory_reservations"][0]["status"], "CONSUMED");
    Ok(())
}

#[tokio::test]
async fn twenty_real_commands_post_exactly_once() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = Arc::new(context(&pool, "twenty-commands").await?);
    let input = quantity(&r, "0.25", 300);
    let barrier = Arc::new(Barrier::new(20));
    let mut jobs = JoinSet::new();
    for _ in 0..20 {
        let r = r.clone();
        let input = input.clone();
        let barrier = barrier.clone();
        jobs.spawn(async move {
            barrier.wait().await;
            http(
                &r.f,
                &format!("work-orders/{}/atomic-effects", r.work_order),
                input,
            )
            .await
        });
    }
    let mut snapshots = Vec::new();
    let mut originals = 0;
    while let Some(result) = jobs.join_next().await {
        let (status, response) = result??;
        assert_eq!(status, StatusCode::OK, "{response}");
        originals += usize::from(response["replay"] == false);
        snapshots.push(response["snapshot"].clone());
    }
    assert_eq!(originals, 1);
    assert_eq!(snapshots.len(), 20);
    assert!(snapshots.iter().all(|snapshot| snapshot == &snapshots[0]));
    let rows = final_facts(&r, "twenty-commands").await?;
    assert_eq!(rows["mfg_material_moves"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        rows["atomic_source_documents"].as_array().map(Vec::len),
        Some(1)
    );
    Ok(())
}

async fn fault(table: &str) -> Result<()> {
    let pool = isolated_pool().await?;
    let case = format!("fault-{table}");
    let r = context(&pool, &case).await?;
    let name = format!("qa_atomic_{}", Uuid::new_v4().simple());
    let sequence = format!("{name}_hit");
    let trigger = format!("{name}_trigger");
    sqlx::query(&format!("create sequence {sequence}"))
        .execute(&pool)
        .await?;
    sqlx::query(&format!("create function {name}() returns trigger language plpgsql as $$ begin if new.tenant_id='{}'::uuid then perform nextval('{sequence}'); raise exception 'QA_ATOMIC_MANDATORY_INSERT_FAILURE'; end if; return new; end $$",r.f.tenant)).execute(&pool).await?;
    sqlx::query(&format!(
        "create trigger {trigger} before insert on {table} for each row execute function {name}()"
    ))
    .execute(&pool)
    .await?;
    let input = quantity(&r, "0.25", 300);
    let before = facts(&pool, &r).await?;
    let result = http(
        &r.f,
        &format!("work-orders/{}/atomic-effects", r.work_order),
        input.clone(),
    )
    .await;
    let actual: bool = sqlx::query_scalar(&format!("select is_called from {sequence}"))
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
    eprintln!(
        "QA atomic_effect_fault tenant={} table={table} actual_marker={actual}",
        r.f.tenant
    );
    let (status, response) = result?;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{response}");
    assert_eq!(
        facts(&pool, &r).await?,
        before,
        "all effects roll back after the actual mandatory insert fails"
    );
    assert!(actual, "actual mandatory INSERT reached its failure");
    post(&r, input.clone(), StatusCode::OK, false).await?;
    post(&r, input, StatusCode::OK, true).await?;
    final_facts(&r, &case).await?;
    Ok(())
}
#[tokio::test]
async fn operation_log_failure_rolls_back_all_effects() -> Result<()> {
    fault("operation_logs").await
}
#[tokio::test]
async fn audit_log_failure_rolls_back_all_effects() -> Result<()> {
    fault("audit_logs").await
}
#[tokio::test]
async fn outbox_failure_rolls_back_all_effects() -> Result<()> {
    fault("outbox_events").await
}
#[tokio::test]
async fn pending_event_failure_rolls_back_all_effects() -> Result<()> {
    fault("atomic_posting_events").await
}
#[tokio::test]
async fn receipt_failure_rolls_back_all_effects() -> Result<()> {
    fault("command_receipts").await
}
