//! Physical consumers must reconcile the complete frozen quality evidence.
use super::{
    Result,
    inspection_outcome_edges::{effect, owned_facts},
    isolated_pool,
    production_receipt_release::{Receipt, http, setup},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use erp_api::{AppState, Claims, JwtService, build_app};
use erp_domain::quantity::ScaledQty;
use erp_domain::trade::SalesLine;
use erp_repo::{
    db::Db,
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
    shared_contract::SharedContractRepo,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
use tower::ServiceExt;
use uuid::Uuid;

async fn context(pool: &PgPool, case: &str) -> Result<Receipt> {
    let r = setup(pool, case).await?;
    sqlx::query("update tenants set name='QA-ERP-INSPECTION-GATE' where tenant_id=$1")
        .bind(r.f.tenant.0)
        .execute(pool)
        .await?;
    eprintln!(
        "QA inspection_gate_fixture tenant={} case={case} work_order={}",
        r.f.tenant, r.work_order
    );
    Ok(r)
}

async fn facts(pool: &PgPool, r: &Receipt) -> Result<Value> {
    let mut value = owned_facts(pool, r).await?;
    for table in [
        "shared_sales_deliveries",
        "shared_material_issues",
        "mfg_material_moves",
        "mfg_qty_balances",
        "inventory_reservations",
    ] {
        value[table] = sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from {table} x where tenant_id=$1"))
            .bind(r.f.tenant.0).fetch_one(pool).await?;
    }
    Ok(value)
}

async fn standard(r: &Receipt) -> Result<Uuid> {
    let (status,body)=http(&r.f,"quality/inspection-templates",json!({
        "request_id":Uuid::new_v4().to_string(),"template_code":"QA-GATE-STANDARD","kind":"FINAL",
        "lines":[
            {"item_code":"WIDTH","required":true,"min_qty":"1","max_qty":"2","sample_count":1},
            {"item_code":"HEIGHT","required":true,"min_qty":"1","max_qty":"2","sample_count":1}
        ]
    })).await?;
    assert_eq!(status, StatusCode::OK, "actual template: {body}");
    effect(&body)
}

async fn legacy_copy(
    pool: &PgPool,
    r: &Receipt,
    kind: &str,
    source: Uuid,
    partial: bool,
) -> Result<Uuid> {
    let template = standard(r).await?;
    let freeze = Uuid::new_v4();
    // These are new, owned QA legacy rows. No bound row is changed or deleted.
    sqlx::query("insert into shared_inspection_freezes(tenant_id,freeze_id,template_id,version_no,source_kind,source_id,item_code,batch_code) values($1,$2,$3,1,$4,$5,'QA-P','QA-PRODUCTION')")
        .bind(r.f.tenant.0).bind(freeze).bind(template).bind(kind).bind(source).execute(pool).await?;
    if partial {
        sqlx::query("insert into shared_inspection_freeze_lines(tenant_id,freeze_id,item_code,required,min_micros,max_micros,sample_count) select tenant_id,$2,item_code,required,min_micros,max_micros,sample_count from shared_inspection_template_lines where tenant_id=$1 and template_id=$3 and item_code='WIDTH'")
            .bind(r.f.tenant.0).bind(freeze).bind(template).execute(pool).await?;
        let order = if kind.trim().eq_ignore_ascii_case("WORK_ORDER") {
            Some(r.work_order)
        } else {
            None
        };
        sqlx::query("insert into mfg_quality_items(tenant_id,item_id,work_order_id,batch_code,item_code,required,min_micros,max_micros,reading_micros,reading_present,passed,inspector,freeze_id,source_kind,source_id) values($1,$2,$3,'QA-PRODUCTION','WIDTH',true,1000000,2000000,1500000,true,true,'QA-LEGACY-MEASURED',$4,$5,$6)")
            .bind(r.f.tenant.0).bind(Uuid::new_v4()).bind(order).bind(freeze).bind(kind.trim().to_ascii_uppercase()).bind(source).execute(pool).await?;
    }
    eprintln!(
        "QA inspection_gate_legacy tenant={} freeze={freeze} source_kind={kind} source={source} partial={partial}",
        r.f.tenant
    );
    Ok(freeze)
}

fn sale_line() -> SalesLine {
    SalesLine {
        line_no: 1,
        item_code: "QA-P".into(),
        qty_micros: 1_000_000,
        price_cent: 100,
        discount_cent: 0,
        tax_cent: 0,
        due_date: "2026-10-05".into(),
        address: "QA-GATE".into(),
        source: "RESALE".into(),
    }
}

async fn delivery(pool: &PgPool, r: &Receipt) -> Result<(Uuid, Uuid, Uuid)> {
    delivery_config(pool, r, false).await
}

async fn delivery_config(
    pool: &PgPool,
    r: &Receipt,
    zero_available: bool,
) -> Result<(Uuid, Uuid, Uuid)> {
    let repo = SharedContractRepo::new(pool);
    let sale = Uuid::new_v4();
    for (action, version) in [("SUBMIT", None), ("FULFILL", Some(1))] {
        let request = Uuid::new_v4().to_string();
        repo.save_sales_revision(erp_repo::shared_contract::TradeSalesRevisionInput {
            tenant_id: r.f.tenant,
            request_id: &request,
            payload_hash: &request,
            so_id: Some(sale),
            customer_code: "QA-CUSTOMER",
            expected_version: version,
            action,
            lines: &[sale_line()],
        })
        .await??;
    }
    let location = Uuid::new_v4();
    let request = Uuid::new_v4().to_string();
    repo.append_stock_ledger(
        r.f.tenant,
        &request,
        &request,
        r.f.warehouse,
        location,
        r.f.sku,
        "QA-PRODUCTION",
        "GOOD",
        "ON_HAND",
        1_000_000,
    )
    .await??;
    let request = Uuid::new_v4().to_string();
    repo.reserve_stock_line(
        r.f.tenant,
        &request,
        &request,
        r.f.warehouse,
        location,
        r.f.sku,
        "QA-PRODUCTION",
        "GOOD",
        &format!("{sale}:1"),
        1_000_000,
    )
    .await??;
    let request = Uuid::new_v4().to_string();
    let (_, delivery) = repo
        .plan_sales_delivery(erp_repo::shared_contract::SalesDeliveryPlanInput {
            tenant_id: r.f.tenant,
            request_id: &request,
            payload_hash: &request,
            so_id: sale,
            line_no: 1,
            expected_version: 2,
            qty_micros: 1_000_000,
        })
        .await??;
    for action in ["RESERVE", "PICK", "REVIEW", "ISSUE"] {
        if zero_available && action == "REVIEW" {
            let service = InventoryService::new(pool);
            let key = InventoryQuantityKey {
                tenant_id: r.f.tenant,
                warehouse_id: r.f.warehouse,
                sku_id: r.f.sku,
            };
            let balance = service
                .get_precise_balance(key)
                .await?
                .ok_or_else(|| anyhow::anyhow!("actual balance missing"))?;
            let available = balance.available()?;
            if available != ScaledQty::ZERO {
                service
                    .apply_precise(InventoryPreciseDelta {
                        key,
                        on_hand: ScaledQty::ZERO,
                        reserved: ScaledQty::ZERO,
                        frozen: available,
                        in_transit: ScaledQty::ZERO,
                        reason_type: "QA_AVAILABLE_HOLD".into(),
                        ref_type: Some("qa_inventory_hold".into()),
                        ref_id: Some(Uuid::new_v4().to_string()),
                        occurred_at: Utc::now(),
                    })
                    .await?;
            }
            let actual = service
                .get_precise_balance(key)
                .await?
                .ok_or_else(|| anyhow::anyhow!("held balance missing"))?;
            assert_eq!(actual.available()?, ScaledQty::ZERO);
            eprintln!(
                "QA inspection_gate_zero_available tenant={} before_review=0",
                r.f.tenant
            );
        }
        let (status, body) = ship(
            r,
            sale,
            delivery,
            location,
            action,
            &Uuid::new_v4().to_string(),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "real {action}: {body}");
    }
    eprintln!(
        "QA inspection_gate_delivery tenant={} sale={sale} delivery={delivery} location={location}",
        r.f.tenant
    );
    Ok((sale, delivery, location))
}

#[tokio::test]
async fn inspection_gate_trade_review_consumes_its_reservation_at_zero_available() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "gate-review-zero-available").await?;
    let (_, delivery, _) = delivery_config(&pool, &r, true).await?;
    let actual = InventoryService::new(&pool)
        .get_precise_balance(InventoryQuantityKey {
            tenant_id: r.f.tenant,
            warehouse_id: r.f.warehouse,
            sku_id: r.f.sku,
        })
        .await?
        .ok_or_else(|| anyhow::anyhow!("reviewed balance missing"))?;
    assert_eq!(actual.available()?, ScaledQty::ZERO);
    let saved = facts(&pool, &r).await?;
    assert!(
        saved["shared_sales_deliveries"]
            .as_array()
            .is_some_and(|rows| rows
                .iter()
                .any(|row| row["delivery_id"] == delivery.to_string() && row["stage"] == "ISSUED"))
    );
    eprintln!(
        "QA inspection_gate_positive tenant={} case=gate-review-zero-available available=0 stage=ISSUED",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_partial_reservation_release_is_precise_and_original_replay_is_inert()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "gate-reservation-release").await?;
    let repo = SharedContractRepo::new(&pool);
    let location = Uuid::new_v4();
    let source = Uuid::new_v4().to_string();
    let opening = Uuid::new_v4().to_string();
    repo.append_stock_ledger(
        r.f.tenant,
        &opening,
        &opening,
        r.f.warehouse,
        location,
        r.f.sku,
        "QA-PRODUCTION",
        "GOOD",
        "ON_HAND",
        500_000,
    )
    .await??;
    let reservation = Uuid::new_v4().to_string();
    repo.reserve_stock_line(
        r.f.tenant,
        &reservation,
        &reservation,
        r.f.warehouse,
        location,
        r.f.sku,
        "QA-PRODUCTION",
        "GOOD",
        &source,
        500_000,
    )
    .await??;
    let key = InventoryQuantityKey {
        tenant_id: r.f.tenant,
        warehouse_id: r.f.warehouse,
        sku_id: r.f.sku,
    };
    let service = InventoryService::new(&independent);
    let before = service
        .get_precise_balance(key)
        .await?
        .ok_or_else(|| anyhow::anyhow!("reserved balance missing"))?;
    let request = Uuid::new_v4().to_string();
    let (_, effect) = repo
        .release_stock_line(
            r.f.tenant,
            &request,
            &request,
            r.f.warehouse,
            location,
            r.f.sku,
            "QA-PRODUCTION",
            "GOOD",
            &source,
            200_000,
        )
        .await??;
    let after = service
        .get_precise_balance(key)
        .await?
        .ok_or_else(|| anyhow::anyhow!("released balance missing"))?;
    assert_eq!(before.on_hand, after.on_hand);
    assert_eq!(
        before.reserved.try_sub(after.reserved)?,
        ScaledQty::from_micros(200_000)
    );
    let saved = facts(&independent, &r).await?;
    let (_, replay_effect) = repo
        .release_stock_line(
            r.f.tenant,
            &request,
            &request,
            r.f.warehouse,
            location,
            r.f.sku,
            "QA-PRODUCTION",
            "GOOD",
            &source,
            200_000,
        )
        .await??;
    assert_eq!(effect, replay_effect);
    assert_eq!(saved, facts(&independent, &r).await?);
    let remaining:i64=sqlx::query_scalar("select coalesce(sum(delta_micros),0)::bigint from shared_stock_ledgers where tenant_id=$1 and source_line=$2 and bucket='RESERVED'")
        .bind(r.f.tenant.0).bind(&source).fetch_one(&independent).await?;
    assert_eq!(remaining, 300_000);
    eprintln!(
        "QA inspection_gate_positive tenant={} case=gate-reservation-release request={request} source={source} reserved=300000 physical_unchanged=true replay_unchanged=true",
        r.f.tenant
    );
    Ok(())
}

async fn ship(
    r: &Receipt,
    sale: Uuid,
    delivery: Uuid,
    location: Uuid,
    action: &str,
    request: &str,
) -> Result<(StatusCode, Value)> {
    gate_http(r,&format!("sales-orders/{sale}/deliveries"),json!({
        "request_id":request,"line_no":1,"expected_version":2,"qty_micros":1_000_000,
        "action":action,"delivery_id":delivery,"carrier_ref":"QA-GATE-CARRIER",
        "warehouse_id":r.f.warehouse,"location_id":location,"sku_id":r.f.sku,"batch_code":"QA-PRODUCTION"
    })).await
}

async fn gate_http(r: &Receipt, path: &str, input: Value) -> Result<(StatusCode, Value)> {
    let jwt = Arc::new(JwtService::new("synthetic-inspection-gates-key".into()));
    let now = Utc::now().timestamp() as usize;
    let token = jwt.encode(&Claims::bearer(
        r.f.actor.to_string(),
        r.f.tenant.to_string(),
        vec![
            "sales.write".into(),
            "sales.read".into(),
            "quality.write".into(),
            "quality.read".into(),
            "manufacturing.write".into(),
        ],
        false,
        now,
        now + 300,
    ))?;
    let database = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
    let joiner = if database.contains('?') { '&' } else { '?' };
    let database = format!(
        "{database}{joiner}application_name=qa_release_{}",
        r.f.tenant
    );
    let response = build_app(AppState::new(Db::connect_lazy(&database)?, jwt))
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/tenants/{}/{path}", r.f.tenant))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(input.to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"text":String::from_utf8_lossy(&bytes)}));
    eprintln!(
        "QA inspection_gate_http tenant={} path={path} input={input} status={} response={body}",
        r.f.tenant,
        status.as_u16()
    );
    Ok((status, body))
}

async fn blocked_sale(partial: bool) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let case = if partial {
        "gate-sale-partial"
    } else {
        "gate-sale-empty"
    };
    let r = context(&pool, case).await?;
    let (sale, delivery, location) = delivery(&pool, &r).await?;
    legacy_copy(&pool, &r, "SALES_ORDER", sale, partial).await?;
    let before = facts(&independent, &r).await?;
    let request = Uuid::new_v4().to_string();
    let (status, body) = ship(&r, sale, delivery, location, "SHIP", &request).await?;
    let after = facts(&independent, &r).await?;
    eprintln!(
        "QA inspection_gate_result tenant={} case={case} request={request} expected=409 actual={} unchanged={}",
        r.f.tenant,
        status.as_u16(),
        before == after
    );
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "complete legacy quality proof required: {body}"
    );
    assert_eq!(
        before, after,
        "a rejected ship has no stage, command or stock effect"
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_sales_empty_legacy_copy_blocks_actual_ship() -> Result<()> {
    blocked_sale(false).await
}

#[tokio::test]
async fn inspection_gate_sales_partial_required_copy_blocks_actual_ship() -> Result<()> {
    blocked_sale(true).await
}

#[tokio::test]
async fn inspection_gate_material_empty_legacy_copy_blocks_actual_issue() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "gate-material-empty").await?;
    legacy_copy(&pool, &r, "WORK_ORDER", r.work_order, false).await?;
    let before = facts(&independent, &r).await?;
    let request = Uuid::new_v4().to_string();
    let (status,body)=http(&r.f,&format!("work-orders/{}/materials",r.work_order),json!({
        "request_id":request,"kind":"ISSUE","qty":"0.1","warehouse_id":r.f.warehouse,"sku_id":r.f.sku
    })).await?;
    let after = facts(&independent, &r).await?;
    eprintln!(
        "QA inspection_gate_result tenant={} case=gate-material-empty request={request} expected=422 actual={} unchanged={}",
        r.f.tenant,
        status.as_u16(),
        before == after
    );
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "complete legacy quality proof required: {body}"
    );
    assert_eq!(
        before, after,
        "a rejected issue has no material, command or stock effect"
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_direct_released_receipt_empty_copy_has_no_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = context(&pool, "gate-direct-receipt-empty").await?;
    legacy_copy(&pool, &r, "WORK_ORDER", r.work_order, false).await?;
    let report: Uuid = sqlx::query_scalar(
        "select report_id from mfg_reports where tenant_id=$1 and work_order_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(r.work_order)
    .fetch_one(&pool)
    .await?;
    let before = facts(&independent, &r).await?;
    let request = Uuid::new_v4().to_string();
    let (status,body)=http(&r.f,&format!("work-orders/{}/receipts",r.work_order),json!({
        "request_id":request,"qty":"0.1","status":"RELEASED","report_id":report,
        "batch_code":"QA-PRODUCTION","inspection_id":Uuid::new_v4(),"warehouse_id":r.f.warehouse,"sku_id":r.f.sku
    })).await?;
    let after = facts(&independent, &r).await?;
    eprintln!(
        "QA inspection_gate_result tenant={} case=gate-direct-receipt-empty request={request} expected=422 actual={} unchanged={}",
        r.f.tenant,
        status.as_u16(),
        before == after
    );
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "complete legacy quality proof required: {body}"
    );
    assert_eq!(
        before, after,
        "a rejected receipt has no available, command or stock effect"
    );
    Ok(())
}

async fn complete_legacy(pool: &PgPool, r: &Receipt, kind: &str, source: Uuid) -> Result<Uuid> {
    let freeze = legacy_copy(pool, r, kind, source, false).await?;
    sqlx::query("insert into shared_inspection_freeze_lines(tenant_id,freeze_id,item_code,required,min_micros,max_micros,sample_count) select l.tenant_id,$2,l.item_code,l.required,l.min_micros,l.max_micros,l.sample_count from shared_inspection_template_lines l join shared_inspection_freezes f on f.tenant_id=l.tenant_id and f.template_id=l.template_id where f.tenant_id=$1 and f.freeze_id=$2")
        .bind(r.f.tenant.0).bind(freeze).execute(pool).await?;
    let order = if kind.trim().eq_ignore_ascii_case("WORK_ORDER") {
        Some(r.work_order)
    } else {
        None
    };
    for item in ["WIDTH", "HEIGHT"] {
        sqlx::query("insert into mfg_quality_items(tenant_id,item_id,work_order_id,batch_code,item_code,required,min_micros,max_micros,reading_micros,reading_present,passed,inspector,freeze_id,source_kind,source_id) values($1,$2,$3,'QA-PRODUCTION',$4,true,1000000,2000000,1500000,true,true,'QA-LEGACY-MEASURED',$5,$6,$7)")
            .bind(r.f.tenant.0).bind(Uuid::new_v4()).bind(order).bind(item).bind(freeze)
            .bind(kind.trim().to_ascii_uppercase()).bind(source).execute(pool).await?;
    }
    Ok(freeze)
}

async fn material(r: &Receipt, request: &str, kind: &str) -> Result<(StatusCode, Value)> {
    http(
        &r.f,
        &format!("work-orders/{}/materials", r.work_order),
        json!({
            "request_id":request,"kind":kind,"qty":"0.1","reason":"QA-GATE-SUPPLEMENT",
            "warehouse_id":r.f.warehouse,"sku_id":r.f.sku
        }),
    )
    .await
}

async fn direct_receipt(pool: &PgPool, r: &Receipt, request: &str) -> Result<(StatusCode, Value)> {
    let report: Uuid = sqlx::query_scalar(
        "select report_id from mfg_reports where tenant_id=$1 and work_order_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(r.work_order)
    .fetch_one(pool)
    .await?;
    http(&r.f,&format!("work-orders/{}/receipts",r.work_order),json!({
        "request_id":request,"qty":"0.1","status":"RELEASED","report_id":report,
        "batch_code":"QA-PRODUCTION","inspection_id":Uuid::new_v4(),"warehouse_id":r.f.warehouse,"sku_id":r.f.sku
    })).await
}

#[tokio::test]
async fn inspection_gate_sales_complete_lowercase_legacy_copy_ships_and_replays() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "gate-sale-lowercase-complete").await?;
    let (sale, delivery, location) = delivery(&pool, &r).await?;
    complete_legacy(&pool, &r, " sales_order ", sale).await?;
    let request = Uuid::new_v4().to_string();
    let (status, body) = ship(&r, sale, delivery, location, "SHIP", &request).await?;
    assert_eq!(status, StatusCode::OK, "complete legacy ship: {body}");
    assert_eq!(body["stage"], "SHIPPED");
    let before = facts(&pool, &r).await?;
    let (status, replay) = ship(&r, sale, delivery, location, "SHIP", &request).await?;
    assert_eq!(status, StatusCode::OK, "original ship replay: {replay}");
    assert_eq!(replay["stage"], "SHIPPED");
    assert_eq!(replay["replay"], true);
    assert_eq!(
        before,
        facts(&pool, &r).await?,
        "replay has no second stock or stage effect"
    );
    eprintln!(
        "QA inspection_gate_positive tenant={} case=gate-sale-lowercase-complete request={request} replay_unchanged=true",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_material_complete_legacy_copy_allows_issue_and_supplement() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "gate-material-complete").await?;
    complete_legacy(&pool, &r, " work_order ", r.work_order).await?;
    for kind in ["ISSUE", "SUPPLEMENT"] {
        let request = Uuid::new_v4().to_string();
        let (status, body) = material(&r, &request, kind).await?;
        assert_eq!(status, StatusCode::OK, "complete legacy {kind}: {body}");
        let before = facts(&pool, &r).await?;
        let (status, replay) = material(&r, &request, kind).await?;
        assert_eq!(status, StatusCode::OK, "original material replay: {replay}");
        assert_eq!(body["effect_id"], replay["effect_id"]);
        assert_eq!(replay["replay"], true);
        assert_eq!(
            before,
            facts(&pool, &r).await?,
            "original material replay has no second quantity effect"
        );
    }
    let moves = facts(&pool, &r).await?;
    assert_eq!(
        moves["mfg_material_moves"].as_array().map(Vec::len),
        Some(2)
    );
    eprintln!(
        "QA inspection_gate_positive tenant={} case=gate-material-complete moves=2 replay_unchanged=true",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_direct_receipt_complete_legacy_copy_allows_exact_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "gate-direct-receipt-complete").await?;
    complete_legacy(&pool, &r, "WORK_ORDER", r.work_order).await?;
    let (status, body) = direct_receipt(&pool, &r, &Uuid::new_v4().to_string()).await?;
    assert_eq!(status, StatusCode::OK, "complete legacy receipt: {body}");
    assert_eq!(body["available_micros"], 100_000);
    eprintln!(
        "QA inspection_gate_positive tenant={} case=gate-direct-receipt-complete available=100000",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_no_freeze_does_not_invent_a_material_rule() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "gate-no-freeze").await?;
    let (status, body) = material(&r, &Uuid::new_v4().to_string(), "ISSUE").await?;
    assert_eq!(status, StatusCode::OK, "no invented rule: {body}");
    assert!(
        facts(&pool, &r).await?["shared_inspection_freezes"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    eprintln!(
        "QA inspection_gate_positive tenant={} case=gate-no-freeze moves=1",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_legacy_template_version_mismatch_blocks_actual_material() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "gate-template-version").await?;
    let freeze = legacy_copy(&pool, &r, "WORK_ORDER", r.work_order, false).await?;
    sqlx::query(
        "update shared_inspection_freezes set version_no=99 where tenant_id=$1 and freeze_id=$2",
    )
    .bind(r.f.tenant.0)
    .bind(freeze)
    .execute(&pool)
    .await?;
    let before = facts(&pool, &r).await?;
    let request = Uuid::new_v4().to_string();
    let (status, body) = material(&r, &request, "ISSUE").await?;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unknown owned template version: {body}"
    );
    assert_eq!(before, facts(&pool, &r).await?);
    eprintln!(
        "QA inspection_gate_extra_rejected tenant={} case=gate-template-version request={request} status=422 unchanged=true",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_other_batch_legacy_copy_does_not_block_direct_receipt() -> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "gate-other-batch").await?;
    let freeze = legacy_copy(&pool, &r, "WORK_ORDER", r.work_order, false).await?;
    sqlx::query("update shared_inspection_freezes set batch_code='QA-OTHER-BATCH' where tenant_id=$1 and freeze_id=$2")
        .bind(r.f.tenant.0).bind(freeze).execute(&pool).await?;
    let (status, body) = direct_receipt(&pool, &r, &Uuid::new_v4().to_string()).await?;
    assert_eq!(status, StatusCode::OK, "separate batch scope: {body}");
    assert_eq!(body["available_micros"], 100_000);
    eprintln!(
        "QA inspection_gate_positive tenant={} case=gate-other-batch available=100000",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_sales_rejected_reading_retakes_need_actual_approval_before_ship()
-> Result<()> {
    let pool = isolated_pool().await?;
    let r = context(&pool, "gate-sale-review-chain").await?;
    let (sale, delivery, location) = delivery(&pool, &r).await?;
    let template = standard(&r).await?;
    let (status,body)=gate_http(&r,"quality/inspection-freezes",json!({
        "request_id":Uuid::new_v4().to_string(),"template_id":template,"source_kind":"SALES_ORDER",
        "source_id":sale,"item_code":"QA-P","batch_code":"QA-PRODUCTION"
    })).await?;
    assert_eq!(status, StatusCode::OK, "actual freeze: {body}");
    let freeze = effect(&body)?;
    let mut width = Uuid::nil();
    for code in ["HEIGHT", "WIDTH"] {
        let (status, body) = gate_http(
            &r,
            &format!("quality/inspection-freezes/{freeze}/readings"),
            json!({
                "request_id":Uuid::new_v4().to_string(),"item_code":code,"reading_qty":"1.5",
                "inspector":"QA-ACTUAL-MEASUREMENT","reviewer":"Declared reviewer only"
            }),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "actual reading: {body}");
        if code == "WIDTH" {
            width = effect(&body)?;
        }
    }
    let (status,body)=gate_http(&r,&format!("quality/inspection-freezes/{freeze}/readings/{width}/reviews"),json!({
        "request_id":Uuid::new_v4().to_string(),"decision":"REJECT","reason":"actual reinspection needed"
    })).await?;
    assert_eq!(status, StatusCode::OK, "actual review: {body}");
    let ship_request = Uuid::new_v4().to_string();
    let before = facts(&pool, &r).await?;
    let (status, body) = ship(&r, sale, delivery, location, "SHIP", &ship_request).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "rejected reading cannot ship: {body}"
    );
    assert_eq!(before, facts(&pool, &r).await?);
    let (status,body)=gate_http(&r,&format!("quality/inspection-freezes/{freeze}/reinspections"),json!({
        "request_id":Uuid::new_v4().to_string(),"item_code":"WIDTH","reading_qty":"1.5",
        "inspector":"QA-ACTUAL-RETAKE","reviewer":"Declared reviewer only","previous_item_id":width,"reason":"corrected measurement"
    })).await?;
    assert_eq!(status, StatusCode::OK, "actual retake: {body}");
    let retake = effect(&body)?;
    let before = facts(&pool, &r).await?;
    let (status, body) = ship(&r, sale, delivery, location, "SHIP", &ship_request).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "unapproved retake cannot ship: {body}"
    );
    assert_eq!(before, facts(&pool, &r).await?);
    let (status,body)=gate_http(&r,&format!("quality/inspection-freezes/{freeze}/readings/{retake}/reviews"),json!({
        "request_id":Uuid::new_v4().to_string(),"decision":"APPROVE","reason":"actual retake reviewed"
    })).await?;
    assert_eq!(status, StatusCode::OK, "actual approval: {body}");
    let (status, body) = ship(&r, sale, delivery, location, "SHIP", &ship_request).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "approved retake permits actual ship: {body}"
    );
    assert_eq!(body["stage"], "SHIPPED");
    let before = facts(&pool, &r).await?;
    let (status, body) = ship(&r, sale, delivery, location, "SHIP", &ship_request).await?;
    assert_eq!(status, StatusCode::OK, "original ship replay: {body}");
    assert_eq!(body["replay"], true);
    assert_eq!(before, facts(&pool, &r).await?);
    assert_eq!(
        before["shared_inspection_reading_events"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    assert_eq!(
        before["shared_inspection_reading_reviews"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    eprintln!(
        "QA inspection_gate_positive tenant={} case=gate-sale-review-chain request={ship_request} readings=3 reviews=2 replay_unchanged=true",
        r.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn inspection_gate_twenty_ships_observe_a_freeze_committed_under_the_source_lock()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let r = Arc::new(context(&pool, "gate-sale-owner-race").await?);
    let (sale, delivery, location) = delivery(&pool, &r).await?;
    let template = standard(&r).await?;
    let mut blocker = pool.begin().await?;
    let pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;
    sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
        .bind(r.f.tenant.0.to_string())
        .bind(format!("sales-revision:{sale}"))
        .execute(&mut *blocker)
        .await?;
    let barrier = Arc::new(Barrier::new(21));
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let r = Arc::clone(&r);
        let barrier = Arc::clone(&barrier);
        let request = Uuid::new_v4().to_string();
        tasks.spawn(async move {
            barrier.wait().await;
            ship(&r, sale, delivery, location, "SHIP", &request)
                .await
                .map(|result| (request, result))
        });
    }
    barrier.wait().await;
    let waiting = super::production_receipt_release::blocker_count(&independent, &r, pid, 20).await;
    // A new owned legacy import commits while all shipments are proven blocked
    // behind this same real source lock, rather than merely starting together.
    let freeze = Uuid::new_v4();
    let inserted=sqlx::query("insert into shared_inspection_freezes(tenant_id,freeze_id,template_id,version_no,source_kind,source_id,item_code,batch_code) values($1,$2,$3,1,'SALES_ORDER',$4,'QA-P','QA-PRODUCTION')")
        .bind(r.f.tenant.0).bind(freeze).bind(template).bind(sale).execute(&mut *blocker).await;
    blocker.commit().await?;
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result??);
    }
    inserted?;
    assert_eq!(
        waiting?, 20,
        "all twenty actual PG requests must be blocked by the owned source"
    );
    assert_eq!(results.len(), 20);
    let after = facts(&independent, &r).await?;
    for (request, (status, body)) in results {
        assert_eq!(
            status,
            StatusCode::CONFLICT,
            "source-locked fresh copy must block ship: {body}"
        );
        assert!(
            after["command_receipts"]
                .as_array()
                .is_some_and(|rows| !rows.iter().any(|row| row["request_id"] == request))
        );
        eprintln!(
            "QA inspection_gate_race_result tenant={} request={request} status=409 command_present=false",
            r.f.tenant
        );
    }
    assert_eq!(after["shared_sales_deliveries"][0]["stage"], "ISSUED");
    eprintln!(
        "QA inspection_gate_race tenant={} source={sale} freeze={freeze} actual_pg_blocked=20 responses=20 successful_ships=0",
        r.f.tenant
    );
    Ok(())
}
