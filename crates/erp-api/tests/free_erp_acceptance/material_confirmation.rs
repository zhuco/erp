//! Real material documents post stock/value once and roll back every actual line.
use super::{
    Result,
    inventory_commands::Fixture,
    isolated_pool,
    material_documents::{context, read, save},
    production_receipt_release::http,
};
use axum::http::StatusCode;
use erp_domain::quantity::parse_qty;
use erp_repo::{
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
    shared_contract::{CostReceiptInput, SharedContractRepo},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn material_confirmation_known_zero_value_is_a_real_posted_source() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    SharedContractRepo::new(&pool)
        .post_cost_adjustment(erp_repo::shared_contract::CostAdjustmentInput {
            tenant_id: f.tenant,
            request_id: "known-zero",
            payload_hash: "known-zero",
            item_code: "QA-COMMAND",
            amount_cent: -1200,
            period_code: &chrono::Utc::now().format("%Y-%m").to_string(),
        })
        .await?
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let doc = draft(&f, order, vec![line(&f, "ISSUE", "0.25", None)]).await?;
    let (status, value) = confirm(&f, order, doc, "zero-confirm").await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["snapshot"]["effects"][0]["value"]["state"], "VALUED");
    assert_eq!(value["snapshot"]["effects"][0]["value"]["amount_cent"], "0");
    assert_eq!(
        SharedContractRepo::new(&pool)
            .read_item_cost(f.tenant, "QA-COMMAND")
            .await?,
        (750000, 0, 0, false)
    );
    assert_eq!(
        read(&f, order, doc, None).await?.1["snapshot"],
        value["snapshot"]
    );
    Ok(())
}

#[tokio::test]
async fn material_confirmation_opposite_sku_orders_wait_and_complete_without_deadlock() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let second = Uuid::new_v4();
    super::masters::sku(&pool, f.tenant.0, second, "QA-COMMAND-B").await?;
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: second,
            },
            on_hand: parse_qty("1")?,
            reserved: parse_qty("0")?,
            frozen: parse_qty("0")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_REVERSE_MATERIAL_KEYS".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: chrono::Utc::now(),
        })
        .await?;
    SharedContractRepo::new(&pool)
        .post_cost_receipt(CostReceiptInput {
            tenant_id: f.tenant,
            request_id: "cost-open-b",
            payload_hash: "cost-open-b",
            item_code: "QA-COMMAND-B",
            qty_micros: 1000000,
            amount_cent: 1200,
            period_code: &chrono::Utc::now().format("%Y-%m").to_string(),
        })
        .await?
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let version: Uuid = sqlx::query_scalar(
        "select bom_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2",
    )
    .bind(f.tenant.0)
    .bind(order)
    .fetch_one(&pool)
    .await?;
    sqlx::query("insert into shared_mfg_bom_version_lines(tenant_id,version_id,line_no,parent_code,component_code,qty_per_micros,unit_code,scrap_ppm,supply_kind) values($1,$2,2,'QA-P','QA-COMMAND-B',1000000,'kg',0,'PURCHASE')")
        .bind(f.tenant.0).bind(version).execute(&pool).await?;
    let other = Uuid::new_v4();
    sqlx::query("insert into mfg_work_orders(tenant_id,work_order_id,work_order_no,product_code,bom_id,status,order_qty_micros) values($1,$2,'QA-REVERSE-WORK-ORDER','QA-P',$3,'RELEASED',10000000)")
        .bind(f.tenant.0).bind(other).bind(version).execute(&pool).await?;
    let a = line(&f, "ISSUE", "0.25", None);
    let mut b = line(&f, "ISSUE", "0.25", None);
    b["sku_id"] = json!(second);
    let first_doc = draft(&f, order, vec![a.clone(), b.clone()]).await?;
    let second_doc = draft(&f, other, vec![b, a]).await?;
    let mut blocker = pool.begin().await?;
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext('item-cost:QA-COMMAND'))")
        .bind(f.tenant.0.to_string())
        .execute(&mut *blocker)
        .await?;
    let mut jobs = tokio::task::JoinSet::new();
    for (owner, doc, request) in [
        (order, first_doc, "forward"),
        (other, second_doc, "reverse"),
    ] {
        let f = Fixture {
            tenant: f.tenant,
            actor: f.actor,
            warehouse: f.warehouse,
            sku: f.sku,
            login: f.login.clone(),
        };
        jobs.spawn(async move { confirm(&f, owner, doc, request).await });
    }
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut waiting = 0_i64;
    while waiting < 2 && tokio::time::Instant::now() < deadline {
        waiting=sqlx::query_scalar("select count(*) from pg_locks where locktype='advisory' and not granted and classid::bigint=(hashtext($1)::bigint & 4294967295) and objid::bigint=(hashtext('item-cost:QA-COMMAND')::bigint & 4294967295)")
            .bind(f.tenant.0.to_string()).fetch_one(&pool).await?;
        if waiting < 2 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
    blocker.commit().await?;
    assert_eq!(
        waiting, 2,
        "two actual postgres writers reached the same cost fence"
    );
    let mut completed = 0;
    while let Some(result) = jobs.join_next().await {
        let (status, value) = result??;
        assert_eq!(status, StatusCode::OK, "{value}");
        completed += 1;
    }
    assert_eq!(completed, 2);
    for code in ["QA-COMMAND", "QA-COMMAND-B"] {
        assert_eq!(
            SharedContractRepo::new(&pool)
                .read_item_cost(f.tenant, code)
                .await?,
            (500000, 600, 1200, false)
        );
    }
    let rows: i64 =
        sqlx::query_scalar("select count(*) from mfg_material_document_effects where tenant_id=$1")
            .bind(f.tenant.0)
            .fetch_one(&pool)
            .await?;
    assert_eq!(rows, 4);
    Ok(())
}

pub(super) async fn setup(pool: &PgPool) -> Result<(Fixture, Uuid)> {
    let (f, order) = context(pool).await?;
    sqlx::query("update tenants set name='QA-MATERIAL-DOCUMENT-CONFIRM' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    InventoryService::new(pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.tenant,
                warehouse_id: f.warehouse,
                sku_id: f.sku,
            },
            on_hand: parse_qty("-3.9")?,
            reserved: parse_qty("-0.9")?,
            frozen: parse_qty("-0.9")?,
            in_transit: parse_qty("-2.3")?,
            reason_type: "QA_MATERIAL_CONFIRM_OPENING".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: chrono::Utc::now(),
        })
        .await?;
    SharedContractRepo::new(pool)
        .post_cost_receipt(CostReceiptInput {
            tenant_id: f.tenant,
            request_id: "cost-opening",
            payload_hash: "cost-opening",
            item_code: "QA-COMMAND",
            qty_micros: 1000000,
            amount_cent: 1200,
            period_code: &chrono::Utc::now().format("%Y-%m").to_string(),
        })
        .await?
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let source = Uuid::new_v4();
    let version = Uuid::new_v4();
    sqlx::query("insert into mfg_boms(tenant_id,bom_id,product_code,status,version,base_qty_micros) values($1,$2,'QA-P','PUBLISHED',1,1000000)").bind(f.tenant.0).bind(source).execute(pool).await?;
    sqlx::query("insert into shared_mfg_bom_versions(tenant_id,version_id,series_code,version_no,status,start_date,source_bom_id) values($1,$2,'QA-P',1,'PUBLISHED','2026-10-01',$3)").bind(f.tenant.0).bind(version).bind(source).execute(pool).await?;
    sqlx::query("insert into shared_mfg_bom_version_lines(tenant_id,version_id,line_no,parent_code,component_code,qty_per_micros,unit_code,scrap_ppm,supply_kind) values($1,$2,1,'QA-P','QA-COMMAND',1000000,'kg',0,'PURCHASE')").bind(f.tenant.0).bind(version).execute(pool).await?;
    sqlx::query("update mfg_work_orders set bom_id=$3 where tenant_id=$1 and work_order_id=$2")
        .bind(f.tenant.0)
        .bind(order)
        .bind(version)
        .execute(pool)
        .await?;
    eprintln!("QA material_confirm tenant={} order={order}", f.tenant);
    Ok((f, order))
}
pub(super) fn line(f: &Fixture, kind: &str, qty: &str, source: Option<Uuid>) -> Value {
    json!({"line_id":Uuid::new_v4(),"kind":kind,"qty":qty,"warehouse_id":f.warehouse,"sku_id":f.sku,"unit_code":"kg",
        "original_issue_id":source,"reason":if kind=="SUPPLEMENT"{Some("real supplement")}else{None}})
}
pub(super) async fn draft(f: &Fixture, order: Uuid, lines: Vec<Value>) -> Result<Uuid> {
    let doc = Uuid::new_v4();
    let (status, value) = save(
        f,
        order,
        json!({"request_id":doc.to_string(),"document_id":doc,"expected_version":0,"lines":lines}),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    Ok(doc)
}
async fn confirm(
    f: &Fixture,
    order: Uuid,
    doc: Uuid,
    request: &str,
) -> Result<(StatusCode, Value)> {
    http(
        f,
        &format!("work-orders/{order}/material-documents/{doc}/confirm"),
        json!({"request_id":request,"expected_version":1}),
    )
    .await
}
pub(super) async fn owned(pool: &PgPool, f: &Fixture) -> Result<Value> {
    let mut rows = serde_json::Map::new();
    for table in [
        "mfg_material_documents",
        "mfg_material_document_revisions",
        "mfg_material_document_lines",
        "mfg_material_document_effects",
        "mfg_material_moves",
        "mfg_material_issue_values",
        "mfg_material_value_allocations",
        "inventory_balances",
        "inventory_ledgers",
        "shared_item_costs",
        "command_receipts",
        "operation_logs",
        "audit_logs",
        "outbox_events",
        "mfg_work_orders",
    ] {
        let value:Value=sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(t) order by to_jsonb(t)::text),'[]'::jsonb) from {table} t where tenant_id=$1"))
            .bind(f.tenant.0).fetch_one(pool).await?;
        rows.insert(table.into(), value);
    }
    Ok(Value::Object(rows))
}

#[tokio::test]
async fn material_confirmation_issue_consume_return_conserve_quantity_and_original_value()
-> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let issue_doc = draft(&f, order, vec![line(&f, "ISSUE", "0.25", None)]).await?;
    let (status, original) = confirm(&f, order, issue_doc, "issue").await?;
    assert_eq!(status, StatusCode::OK, "{original}");
    assert_eq!(original["snapshot"]["status"], "POSTED");
    assert_eq!(
        original["snapshot"]["effects"][0]["value"]["amount_cent"],
        "300"
    );
    let issue = Uuid::parse_str(
        original["snapshot"]["effects"][0]["move_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing move"))?,
    )?;
    let consume = draft(&f, order, vec![line(&f, "CONSUME", "0.125", Some(issue))]).await?;
    let (status, value) = confirm(&f, order, consume, "consume").await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(
        value["snapshot"]["effects"][0]["stock"]["on_hand_micros"],
        "750000"
    );
    assert_eq!(
        value["snapshot"]["effects"][0]["value"]["amount_cent"],
        "150"
    );
    let returned = draft(&f, order, vec![line(&f, "RETURN", "0.125", Some(issue))]).await?;
    let (status, value) = confirm(&f, order, returned, "return").await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(
        value["snapshot"]["effects"][0]["stock"]["on_hand_micros"],
        "875000"
    );
    assert_eq!(
        value["snapshot"]["effects"][0]["value"]["amount_cent"],
        "150"
    );
    assert_eq!(
        SharedContractRepo::new(&pool)
            .read_item_cost(f.tenant, "QA-COMMAND")
            .await?,
        (875000, 1050, 1200, false)
    );
    let before = owned(&pool, &f).await?;
    let (status, replay) = confirm(&f, order, issue_doc, "issue").await?;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["snapshot"], original["snapshot"]);
    assert_eq!(
        read(&f, order, issue_doc, None).await?.1["snapshot"],
        original["snapshot"]
    );
    assert_eq!(
        read(&f, order, issue_doc, Some(&issue_doc.to_string()))
            .await?
            .1["snapshot"]["status"],
        "DRAFT"
    );
    assert_eq!(
        confirm(&f, order, issue_doc, "duplicate-business").await?.0,
        StatusCode::CONFLICT
    );
    assert_eq!(owned(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn material_confirmation_supplement_and_hundred_real_lines_use_server_cost() -> Result<()> {
    for hundred in [false, true] {
        let pool = isolated_pool().await?;
        let (f, order) = setup(&pool).await?;
        let lines = if hundred {
            (0..100).map(|_| line(&f, "ISSUE", "0.01", None)).collect()
        } else {
            vec![
                line(&f, "ISSUE", "0.1", None),
                line(&f, "SUPPLEMENT", "0.15", None),
            ]
        };
        let doc = draft(&f, order, lines).await?;
        let (status, value) = confirm(&f, order, doc, "batch").await?;
        assert_eq!(status, StatusCode::OK, "{value}");
        assert_eq!(
            value["snapshot"]["effects"].as_array().map(Vec::len),
            Some(if hundred { 100 } else { 2 })
        );
        let amount:i64=sqlx::query_scalar("select coalesce(sum(total_cent),0)::bigint from mfg_material_issue_values where tenant_id=$1").bind(f.tenant.0).fetch_one(&pool).await?;
        assert_eq!(amount, if hundred { 1200 } else { 300 });
        let balance:i64=sqlx::query_scalar("select on_hand_micros from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3").bind(f.tenant.0).bind(f.warehouse).bind(f.sku).fetch_one(&pool).await?;
        assert_eq!(balance, if hundred { 0 } else { 750000 });
    }
    Ok(())
}

#[tokio::test]
async fn material_confirmation_last_real_stock_line_failure_rolls_back_every_effect() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let doc = draft(
        &f,
        order,
        vec![
            line(&f, "ISSUE", "0.25", None),
            line(&f, "SUPPLEMENT", "0.8", None),
        ],
    )
    .await?;
    let before = owned(&pool, &f).await?;
    let (status, value) = confirm(&f, order, doc, "last-fails").await?;
    assert!(!status.is_success(), "{value}");
    assert_eq!(owned(&pool, &f).await?, before);
    Ok(())
}

#[tokio::test]
async fn material_confirmation_real_audit_and_outbox_faults_roll_back_and_retry_original_id()
-> Result<()> {
    for outbox in [false, true] {
        let pool = isolated_pool().await?;
        let (f, order) = setup(&pool).await?;
        let doc = draft(&f, order, vec![line(&f, "ISSUE", "0.25", None)]).await?;
        let before = owned(&pool, &f).await?;
        let name = format!("qa_mfg_confirm_{}", Uuid::new_v4().simple());
        let table = if outbox {
            "outbox_events"
        } else {
            "audit_logs"
        };
        let condition = if outbox {
            "NEW.event_type='manufacturing.material.document.confirmed'"
        } else {
            "NEW.action='manufacturing.material.document.confirm'"
        };
        sqlx::query(&format!("create function {name}() returns trigger language plpgsql as $$ begin if NEW.tenant_id='{}'::uuid and {condition} then raise exception 'owned document confirm fault'; end if; return NEW; end $$",f.tenant.0)).execute(&pool).await?;
        sqlx::query(&format!(
            "create trigger {name} before insert on {table} for each row execute function {name}()"
        ))
        .execute(&pool)
        .await?;
        let result = confirm(&f, order, doc, "original-confirm").await;
        sqlx::query(&format!("drop trigger {name} on {table}"))
            .execute(&pool)
            .await?;
        sqlx::query(&format!("drop function {name}()"))
            .execute(&pool)
            .await?;
        assert_eq!(result?.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(owned(&pool, &f).await?, before);
        assert_eq!(
            confirm(&f, order, doc, "original-confirm").await?.0,
            StatusCode::OK
        );
    }
    Ok(())
}

#[tokio::test]
async fn material_confirmation_twenty_same_requests_have_one_material_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let doc = draft(&f, order, vec![line(&f, "ISSUE", "0.25", None)]).await?;
    let mut jobs = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let f = Fixture {
            tenant: f.tenant,
            actor: f.actor,
            warehouse: f.warehouse,
            sku: f.sku,
            login: f.login.clone(),
        };
        jobs.spawn(async move { confirm(&f, order, doc, "same").await });
    }
    let mut original = None;
    let mut writes = 0;
    while let Some(result) = jobs.join_next().await {
        let (status, value) = result??;
        assert_eq!(status, StatusCode::OK, "{value}");
        writes += usize::from(value["replay"] == false);
        if let Some(original) = &original {
            assert_eq!(&value["snapshot"], original);
        } else {
            original = Some(value["snapshot"].clone());
        }
    }
    assert_eq!(writes, 1);
    let rows: i64 =
        sqlx::query_scalar("select count(*) from mfg_material_moves where tenant_id=$1")
            .bind(f.tenant.0)
            .fetch_one(&pool)
            .await?;
    assert_eq!(rows, 1);
    Ok(())
}

#[tokio::test]
async fn material_confirmation_wrong_unit_unknown_cost_and_revoked_replay_cannot_post() -> Result<()>
{
    let pool = isolated_pool().await?;
    let (f, order) = setup(&pool).await?;
    let mut invalid = line(&f, "ISSUE", "0.25", None);
    invalid["unit_code"] = json!("other-unit");
    let doc = draft(&f, order, vec![invalid]).await?;
    let before = owned(&pool, &f).await?;
    assert_eq!(
        confirm(&f, order, doc, "bad-unit").await?.0,
        StatusCode::CONFLICT
    );
    assert_eq!(owned(&pool, &f).await?, before);
    let (uncertain, uncertain_order) = setup(&pool).await?;
    SharedContractRepo::new(&pool)
        .post_estimated_cost(
            uncertain.tenant,
            "unknown-value",
            "unknown-value",
            "QA-COMMAND",
            1,
            &chrono::Utc::now().format("%Y-%m").to_string(),
        )
        .await?
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    let uncertain_doc = draft(
        &uncertain,
        uncertain_order,
        vec![line(&uncertain, "ISSUE", "0.25", None)],
    )
    .await?;
    let uncertain_before = owned(&pool, &uncertain).await?;
    let (status, value) = confirm(&uncertain, uncertain_order, uncertain_doc, "unvalued").await?;
    assert_eq!(status, StatusCode::CONFLICT, "{value}");
    assert_eq!(value["code"], "MATERIAL_DOCUMENT_RECONCILIATION_REQUIRED");
    assert_eq!(owned(&pool, &uncertain).await?, uncertain_before);
    let valid = draft(&f, order, vec![line(&f, "ISSUE", "0.25", None)]).await?;
    let (status, value) = confirm(&f, order, valid, "posted").await?;
    assert_eq!(status, StatusCode::OK, "{value}");
    sqlx::query("delete from user_warehouse_access where tenant_id=$1 and user_id=$2")
        .bind(f.tenant.0)
        .bind(f.actor)
        .execute(&pool)
        .await?;
    assert_eq!(
        confirm(&f, order, valid, "posted").await?.0,
        StatusCode::NOT_FOUND
    );
    Ok(())
}
