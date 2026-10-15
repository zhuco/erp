//! Root snapshots, all children and real stock either commit together or roll back.
use super::inventory_kit_sources::{Kit, facts, fixture, reserve};
use super::inventory_legacy_reservation_atomicity::{body, fixture as stock_fixture};
use super::inventory_location_commands::{Fixture, api};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use erp_repo::inventory::{
    InventoryDelta, InventoryRepo, InventoryReservationAction, InventoryService,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{sync::Barrier, task::JoinSet};
use uuid::Uuid;

fn duplicate(f: &Fixture) -> Fixture {
    Fixture {
        tenant: f.tenant,
        actor: f.actor,
        warehouse: f.warehouse,
        sku: f.sku,
        location: f.location,
        login: f.login.clone(),
    }
}

async fn old_source(pool: &PgPool, kit: &Kit, sku: Uuid, source: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("insert into inventory_reservations(tenant_id,warehouse_id,sku_id,order_item_id,qty,status) values($1,$2,$3,$4,1,'RESERVED')")
        .bind(kit.f.tenant.0).bind(kit.f.warehouse).bind(sku).bind(source).execute(&mut *tx).await?;
    InventoryService::new(pool)
        .apply_with_tx(
            &mut tx,
            InventoryDelta {
                tenant_id: kit.f.tenant,
                warehouse_id: kit.f.warehouse,
                sku_id: sku,
                delta_on_hand: 0,
                delta_reserved: 1,
                delta_frozen: 0,
                delta_in_transit: 0,
                reason_type: "RESERVE".into(),
                ref_type: Some("ORDER_ITEM".into()),
                ref_id: Some(source.to_string()),
                occurred_at: chrono::Utc::now(),
            },
        )
        .await?;
    tx.commit().await?;
    eprintln!(
        "QA kit source legacy seed tenant={} sku={sku} source={source}",
        kit.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_database_guards_keep_original_facts() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    reserve(&kit).await?;
    let before = facts(&independent, &kit).await?;
    let source: Uuid = sqlx::query_scalar("select source_id from inventory_order_reservation_sources where tenant_id=$1 and root_order_item_id=$2")
        .bind(kit.f.tenant.0).bind(kit.root).fetch_one(&independent).await?;
    let tenant = kit.f.tenant;
    let probes = [
        (
            "root-quantity",
            format!(
                "update inventory_order_reservation_sources set original_qty=2 where source_id='{source}'"
            ),
        ),
        (
            "root-delete",
            format!("delete from inventory_order_reservation_sources where source_id='{source}'"),
        ),
        (
            "component-quantity",
            format!(
                "update inventory_order_reservation_components set qty=2 where tenant_id='{tenant}' and source_id='{source}'"
            ),
        ),
        (
            "component-delete",
            format!(
                "delete from inventory_order_reservation_components where tenant_id='{tenant}' and source_id='{source}'"
            ),
        ),
        (
            "attachment-clear",
            format!(
                "update inventory_reservations set reservation_source_id=null where tenant_id='{tenant}' and reservation_source_id='{source}'"
            ),
        ),
        (
            "child-order-context",
            format!(
                "update inventory_reservations set order_id='{}' where tenant_id='{tenant}' and reservation_source_id='{source}'",
                Uuid::new_v4()
            ),
        ),
        (
            "child-sku",
            format!(
                "update inventory_reservations set sku_id='{}' where tenant_id='{tenant}' and sku_id='{}' and reservation_source_id='{source}'",
                kit.second, kit.first
            ),
        ),
        (
            "unlisted-child",
            format!(
                "insert into inventory_reservations(tenant_id,warehouse_id,sku_id,order_item_id,qty,status,reservation_source_id) values('{tenant}','{}','{}','{}',1,'RESERVED','{source}')",
                kit.f.warehouse,
                kit.first,
                Uuid::new_v4()
            ),
        ),
        (
            "incomplete-root",
            format!(
                "insert into inventory_order_reservation_sources(tenant_id,root_order_item_id,warehouse_id,sku_id,original_qty,component_count,definition_kind) values('{tenant}','{}','{}','{}',1,1,'DIRECT')",
                Uuid::new_v4(),
                kit.f.warehouse,
                kit.first
            ),
        ),
        (
            "zero-root",
            format!(
                "insert into inventory_order_reservation_sources(tenant_id,root_order_item_id,warehouse_id,sku_id,original_qty,component_count,definition_kind) values('{tenant}','00000000-0000-0000-0000-000000000000','{}','{}',1,1,'DIRECT')",
                kit.f.warehouse, kit.first
            ),
        ),
    ];
    for (name, sql) in probes {
        let mut tx = pool.begin().await?;
        let rejected = match sqlx::query(&sql).execute(&mut *tx).await {
            Ok(_) => tx.commit().await.err(),
            Err(error) => {
                tx.rollback().await?;
                Some(error)
            }
        };
        assert!(rejected.is_some(), "database accepted {name}");
        eprintln!(
            "QA kit source database guard tenant={tenant} case={name} rejected=true error={rejected:?}"
        );
        assert_eq!(facts(&independent, &kit).await?, before, "{name}");
    }
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_late_component_shortage_rolls_back_root_and_list() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    let late = kit.first.max(kit.second);
    InventoryService::new(&pool)
        .apply_precise(erp_repo::inventory_precision::InventoryPreciseDelta {
            key: erp_repo::inventory_precision::InventoryQuantityKey {
                tenant_id: kit.f.tenant,
                warehouse_id: kit.f.warehouse,
                sku_id: late,
            },
            on_hand: erp_domain::quantity::parse_qty("0")?,
            reserved: erp_domain::quantity::parse_qty("0.2")?,
            frozen: erp_domain::quantity::parse_qty("0")?,
            in_transit: erp_domain::quantity::parse_qty("0")?,
            reason_type: "QA_KIT_SOURCE_SHORTAGE".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: chrono::Utc::now(),
        })
        .await?;
    let before = facts(&independent, &kit).await?;
    let (status, result) = api(&kit.f, "reservations", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    let after = facts(&independent, &kit).await?;
    eprintln!(
        "QA kit source shortage tenant={} late={late} before={before} after={after}",
        kit.f.tenant
    );
    assert_eq!(
        after, before,
        "late component failure must undo earlier child, root and list"
    );
    assert_eq!(after["sources"], json!([]));
    assert_eq!(after["components"], json!([]));
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_both_logs_and_actions_roll_back_complete_group() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    for action in ["reservations", "releases"] {
        for table in ["operation_logs", "audit_logs"] {
            let kit = fixture(&pool).await?;
            if action == "releases" {
                reserve(&kit).await?;
            }
            let schema = format!("qa_kit_sources_{}", Uuid::new_v4().simple());
            sqlx::query(&format!("create schema {schema}"))
                .execute(&pool)
                .await?;
            sqlx::query(&format!("create view {schema}.{table} as select * from public.{table} where false with local check option")).execute(&pool).await?;
            let url = std::env::var("ERP_GRID_TEST_DATABASE_URL")?;
            let separator = if url.contains('?') { '&' } else { '?' };
            let url = format!(
                "{url}{separator}options=-c%20search_path%3D{schema}%2Cpublic%2Cpg_catalog"
            );
            assert!(super::approved_database_url(&url)?);
            let before = facts(&independent, &kit).await?;
            let (status, result) =
                api(&kit.f, action, body(&kit.f, kit.root), None, Some(url)).await?;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{result}");
            let message = result["message"]
                .as_str()
                .or_else(|| result["text"].as_str())
                .ok_or_else(|| anyhow::anyhow!("missing real fault message"))?;
            assert!(
                message.contains("check option") && message.contains(table),
                "{message}"
            );
            let after = facts(&independent, &kit).await?;
            eprintln!(
                "QA kit source log rollback tenant={} action={action} table={table} before={before} after={after}",
                kit.f.tenant
            );
            assert_eq!(after, before, "{action}/{table}");
            let (status, result) = api(&kit.f, action, body(&kit.f, kit.root), None, None).await?;
            assert_eq!(status, StatusCode::OK, "{result}");
        }
    }
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_twenty_retries_freeze_one_root_and_release_once() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    for (action, quantity, expected) in [
        ("reservations", 1, StatusCode::OK),
        ("reservations", 2, StatusCode::CONFLICT),
        ("releases", 1, StatusCode::OK),
    ] {
        let before = facts(&independent, &kit).await?;
        let barrier = Arc::new(Barrier::new(20));
        let mut tasks = JoinSet::new();
        for _ in 0..20 {
            let f = duplicate(&kit.f);
            let root = kit.root;
            let barrier = Arc::clone(&barrier);
            tasks.spawn(async move {
                let mut request = body(&f, root);
                request["quantity"] = json!(quantity);
                barrier.wait().await;
                api(&f, action, request, None, None).await
            });
        }
        let mut completed = 0;
        while let Some(result) = tasks.join_next().await {
            let (status, result) = result??;
            assert_eq!(status, expected, "{result}");
            completed += 1;
        }
        assert_eq!(completed, 20);
        let after = facts(&independent, &kit).await?;
        eprintln!(
            "QA kit source twenty same tenant={} action={action} quantity={quantity} completed={completed}",
            kit.f.tenant
        );
        if quantity == 2 {
            assert_eq!(after, before);
        }
        assert_eq!(after["sources"].as_array().map(Vec::len), Some(1));
        assert_eq!(after["components"].as_array().map(Vec::len), Some(2));
        let expected_logs = if action == "releases" { 2 } else { 1 };
        assert_eq!(
            after["stock"]["operations"].as_array().map(Vec::len),
            Some(expected_logs)
        );
        assert_eq!(
            after["stock"]["audits"].as_array().map(Vec::len),
            Some(expected_logs)
        );
    }
    let before = facts(&independent, &kit).await?;
    let (status, result) = api(&kit.f, "releases", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(facts(&independent, &kit).await?, before);
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_twenty_roots_compete_without_partial_groups() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = JoinSet::new();
    for _ in 0..20 {
        let f = duplicate(&kit.f);
        let barrier = Arc::clone(&barrier);
        tasks.spawn(async move {
            let root = Uuid::new_v4();
            barrier.wait().await;
            api(&f, "reservations", body(&f, root), None, None).await
        });
    }
    let mut success = 0;
    let mut conflicts = 0;
    while let Some(result) = tasks.join_next().await {
        let (status, result) = result??;
        match status {
            StatusCode::OK => success += 1,
            StatusCode::CONFLICT => conflicts += 1,
            _ => {
                return Err(anyhow::anyhow!(
                    "unexpected competing root status {status}: {result}"
                ));
            }
        }
    }
    assert_eq!((success, conflicts), (1, 19));
    let after = facts(&independent, &kit).await?;
    assert_eq!(after["sources"].as_array().map(Vec::len), Some(1));
    assert_eq!(after["components"].as_array().map(Vec::len), Some(2));
    assert_eq!(after["reservations"].as_array().map(Vec::len), Some(2));
    eprintln!(
        "QA kit source twenty different tenant={} success={success} conflicts={conflicts}",
        kit.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_unmapped_legacy_components_require_review() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    for sku in [kit.first, kit.second] {
        old_source(&pool, &kit, sku, Uuid::new_v5(&kit.root, sku.as_bytes())).await?;
    }
    sqlx::query("delete from sku_boms where tenant_id=$1 and bundle_sku_id=$2")
        .bind(kit.f.tenant.0)
        .bind(kit.f.sku)
        .execute(&pool)
        .await?;
    let before = facts(&independent, &kit).await?;
    for action in ["reservations", "releases"] {
        let (status, result) = api(&kit.f, action, body(&kit.f, kit.root), None, None).await?;
        assert_eq!(status, StatusCode::CONFLICT, "{result}");
        assert_eq!(result["code"], "reservation_source_reconciliation_required");
        assert_eq!(facts(&independent, &kit).await?, before);
    }
    assert_eq!(before["sources"], json!([]));
    assert_eq!(before["components"], json!([]));
    eprintln!(
        "QA kit source historical-kit tenant={} root={} status=review",
        kit.f.tenant, kit.root
    );
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_unmapped_direct_keeps_original_row_after_reclassification() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, root) = stock_fixture(&pool).await?;
    sqlx::query("update tenants set name='QA-INVENTORY-KIT-SOURCES' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(&pool)
        .await?;
    let kit = Kit {
        first: f.sku,
        second: f.sku,
        f,
        root,
    };
    old_source(&pool, &kit, kit.f.sku, kit.root).await?;
    sqlx::query("update skus set sku_kind='KIT' where tenant_id=$1 and sku_id=$2")
        .bind(kit.f.tenant.0)
        .bind(kit.f.sku)
        .execute(&pool)
        .await?;
    let before = facts(&independent, &kit).await?;
    reserve(&kit).await?;
    assert_eq!(facts(&independent, &kit).await?, before);
    let (status, result) = api(&kit.f, "releases", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let after = facts(&independent, &kit).await?;
    assert_eq!(after["sources"], json!([]));
    assert_eq!(after["components"], json!([]));
    assert_eq!(
        after["reservations"][0]["reservation_source_id"],
        Value::Null
    );
    assert_eq!(after["stock"]["balances"][0]["reserved_micros"], 900_000);
    eprintln!(
        "QA kit source historical-direct tenant={} actor={} warehouse={} sku={} root={root}",
        kit.f.tenant, kit.f.actor, kit.f.warehouse, kit.f.sku
    );
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_consume_uses_original_components_after_catalog_changes() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    reserve(&kit).await?;
    sqlx::query("delete from sku_boms where tenant_id=$1 and bundle_sku_id=$2")
        .bind(kit.f.tenant.0)
        .bind(kit.f.sku)
        .execute(&pool)
        .await?;
    sqlx::query("update skus set sku_kind='NORMAL' where tenant_id=$1 and sku_id=$2")
        .bind(kit.f.tenant.0)
        .bind(kit.f.sku)
        .execute(&pool)
        .await?;
    let action = || InventoryReservationAction {
        tenant_id: kit.f.tenant,
        warehouse_id: kit.f.warehouse,
        sku_id: kit.f.sku,
        order_item_id: kit.root,
        qty: 1,
    };
    let result = InventoryRepo::new(&pool)
        .consume_reservation(action())
        .await?;
    let before = facts(&independent, &kit).await?;
    let replay = InventoryRepo::new(&pool)
        .consume_reservation(action())
        .await?;
    assert_eq!(result.sku_id, replay.sku_id);
    assert_eq!(facts(&independent, &kit).await?, before);
    let (status, result) = api(&kit.f, "releases", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    assert_eq!(facts(&independent, &kit).await?, before);
    assert!(
        before["reservations"].as_array().is_some_and(
            |rows| rows.len() == 2 && rows.iter().all(|row| row["status"] == "CONSUMED")
        )
    );
    assert!(before["stock"]["balances"].as_array().is_some_and(|rows| {
        rows.len() == 2
            && rows
                .iter()
                .all(|row| row["on_hand_micros"] == 1_900_000 && row["reserved_micros"] == 900_000)
    }));
    eprintln!(
        "QA kit source consume tenant={} root={} sku={}",
        kit.f.tenant, kit.root, replay.sku_id
    );
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_order_components_are_frozen_when_original_rows_disappear() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    let order = Uuid::new_v4();
    for sku in [kit.first, kit.second] {
        sqlx::query("insert into order_item_components(tenant_id,order_id,order_item_id,bundle_sku_id,component_sku_id,component_qty,bom_version,bom_snapshot) values($1,$2,$3,$4,$5,1,7,'[]')")
            .bind(kit.f.tenant.0).bind(order).bind(kit.root).bind(kit.f.sku).bind(sku).execute(&pool).await?;
    }
    let before = facts(&independent, &kit).await?;
    let mut request = body(&kit.f, kit.root);
    request["order_id"] = json!(Uuid::new_v4());
    let (status, result) = api(&kit.f, "reservations", request.clone(), None, None).await?;
    assert_eq!(status, StatusCode::CONFLICT, "{result}");
    assert_eq!(facts(&independent, &kit).await?, before);
    request["order_id"] = json!(order);
    let (status, result) = api(&kit.f, "reservations", request.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    sqlx::query("delete from order_item_components where tenant_id=$1 and order_item_id=$2")
        .bind(kit.f.tenant.0)
        .bind(kit.root)
        .execute(&pool)
        .await?;
    sqlx::query("delete from sku_boms where tenant_id=$1 and bundle_sku_id=$2")
        .bind(kit.f.tenant.0)
        .bind(kit.f.sku)
        .execute(&pool)
        .await?;
    let frozen = facts(&independent, &kit).await?;
    let (status, result) = api(&kit.f, "reservations", request, None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(facts(&independent, &kit).await?, frozen);
    let (status, result) = api(&kit.f, "releases", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let after = facts(&independent, &kit).await?;
    assert!(
        after["components"]
            .as_array()
            .is_some_and(|rows| rows.len() == 2 && rows.iter().all(|row| row["bom_version"] == 7))
    );
    assert_eq!(after["sources"][0]["definition_kind"], "ORDER_COMPONENTS");
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_component_limit_is_explicit_and_atomic() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    sqlx::query("delete from sku_boms where tenant_id=$1 and bundle_sku_id=$2")
        .bind(kit.f.tenant.0)
        .bind(kit.f.sku)
        .execute(&pool)
        .await?;
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA bounded kit parent') returning product_id), created as (insert into skus(tenant_id,sku_id,sku_code,product_id) select $1,gen_random_uuid(),'QA-KIT-LIMIT-'||n::text,(select product_id from qa_parent) from generate_series(1,1001) n returning sku_id) insert into sku_boms(tenant_id,bundle_sku_id,component_sku_id,qty) select $1,$2,sku_id,1 from created")
        .bind(kit.f.tenant.0).bind(kit.f.sku).execute(&pool).await?;
    let before = facts(&independent, &kit).await?;
    let (status, result) = api(&kit.f, "reservations", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{result}");
    assert_eq!(result["code"], "reservation_components_limit");
    assert_eq!(facts(&independent, &kit).await?, before);
    eprintln!(
        "QA kit source limit tenant={} components=1001 rejected=true",
        kit.f.tenant
    );
    Ok(())
}
