//! Focused acceptance uses the existing protected QA database and adds its own tenant.
//! It never seeds over another fixture and fails when the database is unavailable.

#[path = "free_erp_acceptance/masters.rs"]
mod masters;

#[path = "free_erp_acceptance/qa_database.rs"]
mod qa_database;

#[path = "free_erp_acceptance/atomic_effects.rs"]
mod atomic_effects;
#[path = "free_erp_acceptance/material_documents.rs"]
mod material_documents;
#[path = "free_erp_acceptance/material_confirmation.rs"]
mod material_confirmation;
#[path = "free_erp_acceptance/material_source_review.rs"]
mod material_source_review;
#[path = "free_erp_acceptance/manufacturing_catalog.rs"]
mod manufacturing_catalog;
#[path = "free_erp_acceptance/material_rejections.rs"]
mod material_rejections;
#[path = "free_erp_acceptance/material_batch_options.rs"]
mod material_batch_options;
#[path = "free_erp_acceptance/operation_reports.rs"]
mod operation_reports;
#[path = "free_erp_acceptance/report_sources.rs"]
mod report_sources;
#[path = "free_erp_acceptance/quality_rows.rs"]
mod quality_rows;

#[path = "free_erp_acceptance/account_login.rs"]
mod account_login;

#[path = "free_erp_acceptance/session_pool.rs"]
mod session_pool;

#[path = "free_erp_acceptance/trade_facts.rs"]
mod trade_facts;

#[path = "free_erp_acceptance/trade_lines.rs"]
mod trade_lines;

#[path = "free_erp_acceptance/trade_documents.rs"]
mod trade_documents;

#[path = "free_erp_acceptance/receipt_values.rs"]
mod receipt_values;

#[path = "free_erp_acceptance/receipt_value_edges.rs"]
mod receipt_value_edges;

#[path = "free_erp_acceptance/receipt_authority.rs"]
mod receipt_authority;

#[path = "free_erp_acceptance/purchase_arrival_guard.rs"]
mod purchase_arrival_guard;
#[path = "free_erp_acceptance/source_dispositions.rs"]
mod source_dispositions;

#[path = "free_erp_acceptance/command_readback.rs"]
mod command_readback;

#[path = "free_erp_acceptance/inspection_gates.rs"]
mod inspection_gates;

#[path = "free_erp_acceptance/inspection_outcomes.rs"]
mod inspection_outcomes;

#[path = "free_erp_acceptance/inspection_outcome_edges.rs"]
mod inspection_outcome_edges;

#[path = "free_erp_acceptance/inspection_outcome_commands.rs"]
mod inspection_outcome_commands;

#[path = "free_erp_acceptance/inspection_outcome_sales.rs"]
mod inspection_outcome_sales;

#[path = "free_erp_acceptance/production_quality_sources.rs"]
mod production_quality_sources;

#[path = "free_erp_acceptance/bom.rs"]
mod bom;

#[path = "free_erp_acceptance/inventory_release.rs"]
mod inventory_release;

#[path = "free_erp_acceptance/item_cost.rs"]
mod item_cost;

#[path = "free_erp_acceptance/material_identity.rs"]
mod material_identity;

#[path = "free_erp_acceptance/inventory_precision.rs"]
mod inventory_precision;

#[path = "free_erp_acceptance/auth_scope.rs"]
mod auth_scope;

#[path = "free_erp_acceptance/inventory_reads.rs"]
mod inventory_reads;

#[path = "free_erp_acceptance/inventory_exports.rs"]
mod inventory_exports;

#[path = "free_erp_acceptance/inventory_consumers.rs"]
mod inventory_consumers;

#[path = "free_erp_acceptance/inventory_mutation_atomicity.rs"]
mod inventory_mutation_atomicity;

#[path = "free_erp_acceptance/inventory_commands.rs"]
mod inventory_commands;

#[path = "free_erp_acceptance/inventory_command_edges.rs"]
mod inventory_command_edges;

#[path = "free_erp_acceptance/inventory_location_commands.rs"]
mod inventory_location_commands;

#[path = "free_erp_acceptance/inventory_location_outbound.rs"]
mod inventory_location_outbound;

#[path = "free_erp_acceptance/inventory_location_reservations.rs"]
mod inventory_location_reservations;

#[path = "free_erp_acceptance/inventory_location_reservation_edges.rs"]
mod inventory_location_reservation_edges;

#[path = "free_erp_acceptance/inventory_legacy_reservation_atomicity.rs"]
mod inventory_legacy_reservation_atomicity;

#[path = "free_erp_acceptance/inventory_legacy_reservation_edges.rs"]
mod inventory_legacy_reservation_edges;

#[path = "free_erp_acceptance/inventory_kit_sources.rs"]
mod inventory_kit_sources;
#[path = "free_erp_acceptance/inventory_kit_source_edges.rs"]
mod inventory_kit_source_edges;

#[path = "free_erp_acceptance/fulfillment_sources.rs"]
mod fulfillment_sources;

#[path = "free_erp_acceptance/fulfillment_atomicity.rs"]
mod fulfillment_atomicity;

#[path = "free_erp_acceptance/fulfillment_cost_batches.rs"]
mod fulfillment_cost_batches;

#[path = "free_erp_acceptance/login_sessions.rs"]
mod login_sessions;

#[path = "free_erp_acceptance/inventory_batch_writes.rs"]
mod inventory_batch_writes;

#[path = "free_erp_acceptance/production_receipt_release.rs"]
mod production_receipt_release;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{body::Body, http::{Request, StatusCode}};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_domain::manufacturing::{
    BomItem, BucketDemand, BucketStock, BucketSupply, BucketSuggestion, plan_bucket_mrp,
};
use erp_domain::quantity::{ScaledQty, parse_qty};
use erp_repo::db::Db;
use erp_repo::shared_contract::{MaterialMoveInput, SharedContractRepo, SharedWrite};
use erp_types::TenantId;
use http_body_util::BodyExt;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;

fn qty(value: &str) -> ScaledQty {
    parse_qty(value).expect("valid fixture quantity")
}

fn policy(code: &str, supply: &str) -> BucketStock {
    BucketStock {
        code: code.into(), warehouse: "QA-WH".into(), on_hand: ScaledQty::ZERO,
        allocated_elsewhere: ScaledQty::ZERO, wip: ScaledQty::ZERO,
        safety: ScaledQty::ZERO, min_lot: ScaledQty::ZERO,
        lot_multiple: ScaledQty::ZERO, supply: supply.into(), lead_days: 0,
    }
}

fn demand(code: &str, quantity: &str, day: &str) -> BucketDemand {
    BucketDemand {
        code: code.into(), qty: qty(quantity), need_date: day.into(),
        warehouse: "QA-WH".into(), frozen: true,
    }
}

fn plan(bom: &[BomItem], stock: &[BucketStock], supply: &[BucketSupply],
    demand: &[BucketDemand], warehouse: &str, rest_days: &[String]) -> Result<Vec<BucketSuggestion>> {
    plan_bucket_mrp(bom, stock, supply, demand, warehouse, rest_days)
        .map_err(|error| anyhow::anyhow!("MRP rejected: {error:?}"))
}

#[test]
fn mrp_lot_surplus_covers_the_next_bucket() -> Result<()> {
    let mut item = policy("RAW", "BUY");
    item.min_lot = qty("10");
    let rows = plan(&[], &[item], &[], &[
        demand("RAW", "5", "2026-10-08"), demand("RAW", "5", "2026-10-09"),
    ], "QA-WH", &[])?;
    assert_eq!(rows.len(), 1, "the unused five units cover the second bucket");
    assert_eq!(rows[0].suggested, qty("10"));
    Ok(())
}

#[test]
fn mrp_shared_component_is_aggregated_before_lot_rounding() -> Result<()> {
    let mut component = policy("S", "BUY");
    component.min_lot = qty("10");
    let bom = [
        BomItem { parent: "P".into(), component: "S".into(), qty_per: qty("1") },
        BomItem { parent: "Q".into(), component: "S".into(), qty_per: qty("1") },
    ];
    let rows = plan(&bom,
        &[policy("P", "MAKE"), policy("Q", "MAKE"), component], &[],
        &[demand("P", "1", "2026-10-08"), demand("Q", "1", "2026-10-08")],
        "QA-WH", &[])?;
    let shared: Vec<_> = rows.iter().filter(|row| row.code == "S").collect();
    assert_eq!(shared.len(), 1);
    assert_eq!(shared[0].gross, qty("2"));
    assert_eq!(shared[0].suggested, qty("10"));
    Ok(())
}

#[test]
fn mrp_child_material_is_needed_when_the_parent_starts() -> Result<()> {
    let mut parent = policy("P", "MAKE");
    parent.lead_days = 2;
    let bom = [BomItem { parent: "P".into(), component: "A".into(), qty_per: qty("1") }];
    let rows = plan(&bom, &[parent, policy("A", "BUY")], &[
        BucketSupply { code: "A".into(), warehouse: "QA-WH".into(), qty: qty("1"),
            arrive_date: "2026-10-09".into(), accepted: true },
    ], &[demand("P", "1", "2026-10-12")], "QA-WH", &[])?;
    let parent = rows.iter().find(|row| row.code == "P").context("parent suggestion")?;
    let material = rows.iter().find(|row| row.code == "A").context("material suggestion")?;
    assert_eq!(parent.release_date, "2026-10-08");
    assert_eq!(material.need_date, parent.release_date);
    assert_eq!(material.net, qty("1"), "arrival after production starts is late");
    Ok(())
}

#[test]
fn mrp_calendar_excludes_rest_days_inside_the_lead_time() -> Result<()> {
    let mut item = policy("RAW", "BUY");
    item.lead_days = 2;
    let rows = plan(&[], &[item], &[], &[demand("RAW", "1", "2026-10-12")],
        "QA-WH", &["2026-10-09".into()])?;
    assert_eq!(rows[0].release_date, "2026-10-07");
    Ok(())
}

#[test]
fn mrp_calendar_handles_weekends_with_monday_based_weekdays() -> Result<()> {
    for (need, expected) in [
        ("2026-10-12", "2026-10-09"),
        ("2026-10-10", "2026-10-09"),
        ("2026-10-11", "2026-10-09"),
        ("2026-10-13", "2026-10-12"),
        ("2026-10-09", "2026-10-08"),
    ] {
        let mut item = policy("RAW", "BUY");
        item.lead_days = 1;
        let rows = plan(&[], &[item], &[], &[demand("RAW", "1", need)], "QA-WH", &[])?;
        assert_eq!(rows[0].release_date, expected, "need date {need}");
    }
    Ok(())
}

#[test]
fn mrp_rejects_negative_quantities_and_duplicate_stock_policies() {
    let mut negative = policy("RAW", "BUY");
    negative.on_hand = qty("-1");
    assert!(plan_bucket_mrp(&[], &[negative], &[], &[demand("RAW", "1", "2026-10-08")], "QA-WH", &[]).is_err());
    assert!(plan_bucket_mrp(&[], &[policy("RAW", "BUY")], &[], &[demand("RAW", "-1", "2026-10-08")], "QA-WH", &[]).is_err());
    assert!(plan_bucket_mrp(&[], &[policy("RAW", "BUY"), policy("RAW", "BUY")], &[],
        &[demand("RAW", "1", "2026-10-08")], "QA-WH", &[]).is_err());
}

#[test]
fn mrp_intermediate_multiplication_does_not_overflow_a_valid_result() -> Result<()> {
    let bom = [BomItem { parent: "P".into(), component: "A".into(), qty_per: qty("100") }];
    let rows = plan(&bom, &[policy("P", "MAKE"), policy("A", "BUY")], &[],
        &[demand("P", "1000000", "2026-10-08")], "QA-WH", &[])?;
    assert_eq!(rows.iter().find(|row| row.code == "A").context("material row")?.gross,
        qty("100000000"));
    Ok(())
}

async fn isolated_pool() -> Result<PgPool> {
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")
        .context("QA database unavailable: acceptance NOT RUN")?;
    let parsed = approved_database_url(&url)?;
    if !parsed { anyhow::bail!("refusing an unapproved QA database URL"); }
    let pool = PgPoolOptions::new().max_connections(4).connect(&url).await?;
    qa_database::verify(&pool).await?;
    Ok(pool)
}

fn approved_database_url(url: &str) -> Result<bool> {
    qa_database::approved_url(url)
}

#[tokio::test]
async fn manufacturing_write_endpoints_deny_read_only_users_before_any_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = uuid::Uuid::new_v4();
    let jwt = JwtService::new("synthetic-free-erp-acceptance-key".into());
    let now = Utc::now().timestamp() as usize;
    let claims = Claims::bearer(uuid::Uuid::new_v4().to_string(), tenant.to_string(),
        vec!["orders.read".into(), "skus.read".into()], false, now, now + 300);
    let token = jwt.encode(&claims)?;
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let app = build_app(AppState::new(Db::connect_lazy(&url)?, Arc::new(jwt)));
    let paths = ["boms", "bom-structures", "bom-versions", "routings", "operation-routes",
        "work-centers", "production-plans", "mrp-buckets", "mrp-runs", "executions", "work-orders",
        "work-orders/00000000-0000-0000-0000-000000000001/actions",
        "work-orders/00000000-0000-0000-0000-000000000001/materials"];
    for path in paths {
        for key in [None, Some("same-key-cached-response")] {
            let mut request = Request::builder().method("POST")
                .uri(format!("/v1/tenants/{tenant}/manufacturing/{path}"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json");
            if let Some(key) = key { request = request.header("idempotency-key", key); }
            let response = app.clone().oneshot(request.body(Body::from("{}"))?).await?;
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "route {path}, key {key:?}");
        }
    }
    let writes: i64 = sqlx::query_scalar("select count(*) from command_receipts where tenant_id = $1")
        .bind(tenant).fetch_one(&pool).await?;
    assert_eq!(writes, 0);
    Ok(())
}

#[tokio::test]
async fn manufacturing_bom_authorized_write_replay_and_scope_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let tenant = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id, name) values ($1, 'QA-MFG-HTTP')")
        .bind(tenant).execute(&pool).await?;
    let jwt = JwtService::new("synthetic-free-erp-acceptance-key".into());
    let now = Utc::now().timestamp() as usize;
    let claims = Claims::bearer(uuid::Uuid::new_v4().to_string(), tenant.to_string(),
        vec!["manufacturing.write".into()], false, now, now + 300);
    let token = jwt.encode(&claims)?;
    let app = build_app(AppState::new(Db::connect_lazy(&std::env::var("ERP_GRID_TEST_DATABASE_URL")?)?, Arc::new(jwt)));
    let payload = serde_json::json!({"request_id":"qa-bom", "product_code":"QA-P",
        "items":[{"parent":"QA-P", "component":"QA-A", "qty_per":"0.125"}]});
    let mut first_effect = None;
    for (destination, body, expected) in [
        (tenant, payload.clone(), StatusCode::OK),
        (tenant, payload.clone(), StatusCode::OK),
        (tenant, serde_json::json!({"request_id":"qa-bom", "product_code":"QA-P",
            "items":[{"parent":"QA-P", "component":"QA-A", "qty_per":"0.250"}]}), StatusCode::CONFLICT),
        (uuid::Uuid::new_v4(), payload, StatusCode::FORBIDDEN),
    ] {
        let response = app.clone().oneshot(Request::builder().method("POST")
            .uri(format!("/v1/tenants/{destination}/manufacturing/boms"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))?).await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        assert_eq!(status, expected, "{}", String::from_utf8_lossy(&bytes));
        if expected == StatusCode::OK {
            let json: serde_json::Value = serde_json::from_slice(&bytes)?;
            let effect = json["effect_id"].as_str().context("BOM effect identity")?.to_string();
            if let Some(first) = &first_effect { assert_eq!(&effect, first); }
            else { first_effect = Some(effect); }
        }
    }
    let (boms, lines, micros, receipts): (i64, i64, i64, i64) = sqlx::query_as(
        "select (select count(*) from mfg_boms where tenant_id=$1),
        (select count(*) from mfg_bom_items where tenant_id=$1),
        (select qty_per_micros from mfg_bom_items where tenant_id=$1),
        (select count(*) from command_receipts where tenant_id=$1)",
    ).bind(tenant).fetch_one(&pool).await?;
    assert_eq!((boms, lines, micros, receipts), (1, 1, 125_000, 1));
    Ok(())
}

#[tokio::test]
async fn manufacturing_fractional_return_restores_balance_and_replays_once() -> Result<()> {
    use erp_domain::manufacturing::{MaterialMove, MaterialMoveKind};
    let pool = isolated_pool().await?;
    let tenant = TenantId(uuid::Uuid::new_v4());
    let warehouse = uuid::Uuid::new_v4();
    let sku = uuid::Uuid::new_v4();
    let order = uuid::Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id, name) values ($1, 'QA-MFG-RETURN')")
        .bind(uuid::Uuid::from(tenant)).execute(&pool).await?;
    material_identity::seed_identity(&pool, uuid::Uuid::from(tenant), warehouse, sku).await?;
    sqlx::query("insert into mfg_work_orders(tenant_id, work_order_id, work_order_no,
        product_code, status, order_qty_micros) values($1,$2,'QA-WO','QA-P','RELEASED',1000000)")
        .bind(uuid::Uuid::from(tenant)).bind(order).execute(&pool).await?;
    sqlx::query("insert into mfg_qty_balances(tenant_id,warehouse_id,sku_id,on_hand_micros,reserved_micros)
        values($1,$2,$3,1000000,0)")
        .bind(uuid::Uuid::from(tenant)).bind(warehouse).bind(sku).execute(&pool).await?;
    sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type)
        select $1,$2,$3,ref_id::uuid,1000000,'QA_OPENING' from inventory_ledgers where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 and reason_type='QA_OPENING' and ref_type='mfg_material_move'")
        .bind(uuid::Uuid::from(tenant)).bind(warehouse).bind(sku).execute(&pool).await?;
    let repo = SharedContractRepo::new(&pool);
    let issue=repo.post_material_move(tenant, "qa-issue", "qa-issue", MaterialMoveInput {
        work_order_id:order, warehouse_id:warehouse, sku_id:sku, batch_id:None, original_issue_id:None,
        movement:MaterialMove { kind:MaterialMoveKind::Issue,qty:qty("0.250"),supplement_reason_required:false },reason:None,
    }).await?.map_err(|e|anyhow::anyhow!("issue receipt rejected: {e:?}"))?;
    for replay in [false, true] {
        let result=repo.post_material_move(tenant, "qa-return", "qa-return", MaterialMoveInput {
            work_order_id:order,warehouse_id:warehouse,sku_id:sku,batch_id:None,original_issue_id:Some(issue.move_id),
            movement:MaterialMove {kind:MaterialMoveKind::Return,qty:qty("0.125"),supplement_reason_required:false},reason:None,
        }).await?.map_err(|e|anyhow::anyhow!("return receipt rejected: {e:?}"))?;
        assert_eq!(matches!(result.effect, SharedWrite::Replay(_)), replay);
        assert_eq!(result.balance.shop_qty, qty("0.125"));
    }
    let (on_hand, reserved, ledger_total, moves): (i64, i64, i64, i64) = sqlx::query_as(
        "select on_hand_micros,reserved_micros,
        (select sum(delta_on_hand_micros)::bigint from inventory_ledgers where tenant_id=$1),
        (select count(*) from mfg_material_moves where tenant_id=$1)
        from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3",
    ).bind(uuid::Uuid::from(tenant)).bind(warehouse).bind(sku).fetch_one(&pool).await?;
    assert_eq!((on_hand, reserved, ledger_total, moves), (875_000, 0, 875_000, 2));
    Ok(())
}

#[tokio::test]
async fn manufacturing_concurrent_consumption_never_exceeds_issued_material() -> Result<()> {
    use erp_domain::manufacturing::{MaterialMove, MaterialMoveKind};
    let pool = isolated_pool().await?;
    let tenant = TenantId(uuid::Uuid::new_v4());
    let order = uuid::Uuid::new_v4();
    let warehouse=uuid::Uuid::new_v4();
    let sku=uuid::Uuid::new_v4();
    sqlx::query("insert into tenants(tenant_id, name) values($1,'QA-MFG-CONSUME')")
        .bind(uuid::Uuid::from(tenant)).execute(&pool).await?;
    material_identity::seed_identity(&pool,uuid::Uuid::from(tenant),warehouse,sku).await?;
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,status,order_qty_micros) values($1,$2,'QA-WO','QA-P','RELEASED',1000000)")
        .bind(uuid::Uuid::from(tenant)).bind(order).execute(&pool).await?;
    sqlx::query("insert into mfg_qty_balances(tenant_id,warehouse_id,sku_id,on_hand_micros) values($1,$2,$3,1000000)")
        .bind(uuid::Uuid::from(tenant)).bind(warehouse).bind(sku).execute(&pool).await?;
    let issue=SharedContractRepo::new(&pool).post_material_move(tenant,"qa-seed","qa-seed",MaterialMoveInput {
        work_order_id:order,warehouse_id:warehouse,sku_id:sku,batch_id:None,original_issue_id:None,
        movement:MaterialMove {kind:MaterialMoveKind::Issue,qty:qty("0.500"),supplement_reason_required:false},reason:None,
    }).await?.map_err(|e|anyhow::anyhow!("seed refused: {e:?}"))?;
    let issue_id=issue.move_id;
    // Hold the order row while the first three writers reach it. An unlocked
    // read-before-update implementation lets all three spend the same material.
    let mut blocker = pool.begin().await?;
    sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
        .bind(uuid::Uuid::from(tenant)).bind(order).execute(&mut *blocker).await?;
    let blocker_pid:i32=sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *blocker).await?;
    let mut tasks = Vec::new();
    for index in 0..20 {
        let pool = pool.clone();
        tasks.push(tokio::spawn(async move {
            let command = format!("qa-consume-{index}");
            SharedContractRepo::new(&pool).post_material_move(tenant,&command,&command,MaterialMoveInput {
                work_order_id:order,warehouse_id:warehouse,sku_id:sku,batch_id:None,original_issue_id:Some(issue_id),
                movement:MaterialMove {kind:MaterialMoveKind::Consume,qty:qty("0.400"),supplement_reason_required:false},reason:None,
            }).await
        }));
    }
    let mut waiting = 0;
    for _ in 0..100 {
        sqlx::query("select pg_stat_clear_snapshot()")
            .execute(&mut *blocker).await?;
        waiting = material_identity::waiting_descendants(&mut *blocker,blocker_pid).await?;
        if waiting >= 3 { break; }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    blocker.rollback().await?;
    let mut accepted = 0;
    let mut outcomes = Vec::new();
    for task in tasks {
        outcomes.push(task.await);
    }
    assert_eq!(outcomes.len(),20,"all actual concurrent outcomes must be collected");
    for outcome in outcomes {
        match outcome? {
            Ok(Ok(_)) => accepted += 1,
            Ok(Err(error)) => anyhow::bail!("unexpected receipt conflict: {error:?}"),
            Err(error) => assert!(
                error.to_string().starts_with("material move rejected:"),
                "unexpected failure instead of an insufficient-material rejection: {error}",
            ),
        }
    }
    eprintln!("QA material_consume tenant={tenant} blocker_pid={blocker_pid} waiting_descendants={waiting} results=20 accepted={accepted}");
    assert!(waiting >= 3, "concurrency fixture did not reach contention");
    assert_eq!(accepted, 1, "only one 0.400 issue fits in 0.500 on the shop floor");
    let consumed: i64 = sqlx::query_scalar(
        "select coalesce(sum(qty_micros),0)::bigint from mfg_material_moves
        where tenant_id=$1 and work_order_id=$2 and kind='CONSUME'",
    ).bind(uuid::Uuid::from(tenant)).bind(order).fetch_one(&pool).await?;
    assert_eq!(consumed, 400_000);
    Ok(())
}
