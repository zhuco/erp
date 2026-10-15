//! A root intent and its applied components survive later catalog changes.
use super::inventory_legacy_reservation_atomicity::{body, fixture as stock_fixture, saved};
use super::inventory_location_commands::{Fixture, api};
use super::{Result, isolated_pool};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) struct Kit {
    pub(super) f: Fixture,
    pub(super) root: Uuid,
    pub(super) first: Uuid,
    pub(super) second: Uuid,
}

pub(super) async fn fixture(pool: &PgPool) -> Result<Kit> {
    let (mut f, root) = stock_fixture(pool).await?;
    sqlx::query("update tenants set name='QA-INVENTORY-KIT-SOURCES' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(pool)
        .await?;
    let first = f.sku;
    let second = Uuid::new_v4();
    let parent = Uuid::new_v4();
    for (sku, code, kind) in [
        (second, "QA-KIT-SECOND", "NORMAL"),
        (parent, "QA-KIT-PARENT", "KIT"),
    ] {
        sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,sku_kind,product_id) values($1,$2,$3,$4,(select product_id from qa_parent))")
            .bind(f.tenant.0)
            .bind(sku)
            .bind(code)
            .bind(kind)
            .execute(pool)
            .await?;
    }
    let opening = erp_repo::inventory_precision::InventoryPreciseDelta {
        key: erp_repo::inventory_precision::InventoryQuantityKey {
            tenant_id: f.tenant,
            warehouse_id: f.warehouse,
            sku_id: second,
        },
        on_hand: erp_domain::quantity::parse_qty("2.9")?,
        reserved: erp_domain::quantity::parse_qty("0.9")?,
        frozen: erp_domain::quantity::parse_qty("0.9")?,
        in_transit: erp_domain::quantity::parse_qty("2.3")?,
        reason_type: "QA_KIT_SOURCE_OPENING".into(),
        ref_type: Some("QA_KIT_SOURCE_OPENING".into()),
        ref_id: None,
        occurred_at: chrono::Utc::now(),
    };
    erp_repo::inventory::InventoryService::new(pool)
        .apply_precise(opening)
        .await?;
    for sku in [first, second] {
        sqlx::query(
            "insert into sku_boms(tenant_id,bundle_sku_id,component_sku_id,qty) values($1,$2,$3,1)",
        )
        .bind(f.tenant.0)
        .bind(parent)
        .bind(sku)
        .execute(pool)
        .await?;
        sqlx::query(
            "with qa_shop as (insert into shops(tenant_id,shop_id,platform,platform_shop_id,shop_name) values($1,gen_random_uuid(),'qa',gen_random_uuid()::text,'QA mapping shop') returning shop_id) insert into channel_sku_mappings(tenant_id,sku_id,status,shop_id,platform,platform_sku_id) values($1,$2,'ACTIVE',(select shop_id from qa_shop),'qa',$2::text)",
        )
        .bind(f.tenant.0)
        .bind(sku)
        .execute(pool)
        .await?;
    }
    f.sku = parent;
    eprintln!(
        "QA kit source fixture tenant={} actor={} warehouse={} parent={parent} root={root} first={first} second={second}",
        f.tenant, f.actor, f.warehouse
    );
    Ok(Kit {
        f,
        root,
        first,
        second,
    })
}

pub(super) async fn facts(pool: &PgPool, kit: &Kit) -> Result<Value> {
    let mut facts = saved(pool, &kit.f).await?;
    let present: bool = sqlx::query_scalar(
        "select to_regclass('public.inventory_order_reservation_sources') is not null",
    )
    .fetch_one(pool)
    .await?;
    facts["snapshot_schema_present"] = json!(present);
    if present {
        let source:Value=sqlx::query_scalar("select coalesce(jsonb_agg(to_jsonb(s) order by source_id),'[]'::jsonb) from inventory_order_reservation_sources s where tenant_id=$1")
            .bind(kit.f.tenant.0).fetch_one(pool).await?;
        let components:Value=sqlx::query_scalar("select coalesce(jsonb_agg(to_jsonb(c) order by sku_id),'[]'::jsonb) from inventory_order_reservation_components c where tenant_id=$1")
            .bind(kit.f.tenant.0).fetch_one(pool).await?;
        facts["sources"] = source;
        facts["components"] = components;
    }
    Ok(facts)
}

pub(super) async fn reserve(kit: &Kit) -> Result<()> {
    let (status, result) = api(&kit.f, "reservations", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_order_components_reject_changed_parent_quantity() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    let order = Uuid::new_v4();
    for sku in [kit.first, kit.second] {
        sqlx::query("insert into order_item_components(tenant_id,order_id,order_item_id,bundle_sku_id,component_sku_id,component_qty,bom_version,bom_snapshot) values($1,$2,$3,$4,$5,1,7,'[]')")
            .bind(kit.f.tenant.0).bind(order).bind(kit.root).bind(kit.f.sku).bind(sku).execute(&pool).await?;
    }
    let mut request = body(&kit.f, kit.root);
    request["order_id"] = json!(order);
    let (status, result) = api(&kit.f, "reservations", request.clone(), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    let before = facts(&independent, &kit).await?;
    request["quantity"] = json!(2);
    let (status, result) = api(&kit.f, "reservations", request, None, None).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "changed parent quantity must not be accepted: {result}"
    );
    assert_eq!(facts(&independent, &kit).await?, before);
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_deleted_bom_releases_original_components() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    reserve(&kit).await?;
    sqlx::query("delete from sku_boms where tenant_id=$1 and bundle_sku_id=$2")
        .bind(kit.f.tenant.0)
        .bind(kit.f.sku)
        .execute(&pool)
        .await?;
    let (status, result) = api(&kit.f, "releases", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "original applied list must survive deleted BOM: {result}"
    );
    let after = facts(&independent, &kit).await?;
    assert!(
        after["reservations"]
            .as_array()
            .is_some_and(|rows| rows.len() == 2 && rows.iter().all(|r| r["status"] == "RELEASED"))
    );
    assert!(
        after["stock"]["balances"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|r| r["reserved_micros"] == 900_000))
    );
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_changed_bom_retry_does_not_add_new_component() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let kit = fixture(&pool).await?;
    reserve(&kit).await?;
    let third = Uuid::new_v4();
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-KIT-THIRD',(select product_id from qa_parent))")
        .bind(kit.f.tenant.0)
        .bind(third)
        .execute(&pool)
        .await?;
    erp_repo::inventory::InventoryService::new(&pool)
        .apply(erp_repo::inventory::InventoryDelta {
            tenant_id: kit.f.tenant,
            warehouse_id: kit.f.warehouse,
            sku_id: third,
            delta_on_hand: 5,
            delta_reserved: 0,
            delta_frozen: 0,
            delta_in_transit: 0,
            reason_type: "QA_KIT_SOURCE_THIRD".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: chrono::Utc::now(),
        })
        .await?;
    sqlx::query(
        "delete from sku_boms where tenant_id=$1 and bundle_sku_id=$2 and component_sku_id=$3",
    )
    .bind(kit.f.tenant.0)
    .bind(kit.f.sku)
    .bind(kit.second)
    .execute(&pool)
    .await?;
    sqlx::query("insert into sku_boms(tenant_id,bundle_sku_id,component_sku_id,qty,bom_version) values($1,$2,$3,1,8)")
        .bind(kit.f.tenant.0).bind(kit.f.sku).bind(third).execute(&pool).await?;
    let before = facts(&independent, &kit).await?;
    reserve(&kit).await?;
    assert_eq!(
        facts(&independent, &kit).await?,
        before,
        "catalog changes must not append a new source effect/log"
    );
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_different_parent_sku_cannot_borrow_original_children() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let mut kit = fixture(&pool).await?;
    reserve(&kit).await?;
    let other = Uuid::new_v4();
    sqlx::query(
        "with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,sku_kind,product_id) values($1,$2,'QA-KIT-OTHER','KIT',(select product_id from qa_parent))",
    )
    .bind(kit.f.tenant.0)
    .bind(other)
    .execute(&pool)
    .await?;
    for sku in [kit.first, kit.second] {
        sqlx::query(
            "insert into sku_boms(tenant_id,bundle_sku_id,component_sku_id,qty) values($1,$2,$3,1)",
        )
        .bind(kit.f.tenant.0)
        .bind(other)
        .bind(sku)
        .execute(&pool)
        .await?;
    }
    kit.f.sku = other;
    let before = facts(&independent, &kit).await?;
    let (status, result) = api(&kit.f, "reservations", body(&kit.f, kit.root), None, None).await?;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "different parent must conflict even with identical components: {result}"
    );
    assert_eq!(facts(&independent, &kit).await?, before);
    Ok(())
}

#[tokio::test]
async fn kit_snapshot_reclassification_keeps_original_direct_source() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let (f, root) = stock_fixture(&pool).await?;
    sqlx::query("update tenants set name='QA-INVENTORY-KIT-SOURCES' where tenant_id=$1")
        .bind(f.tenant.0)
        .execute(&pool)
        .await?;
    let (status, result) = api(&f, "reservations", body(&f, root), None, None).await?;
    assert_eq!(status, StatusCode::OK, "{result}");
    sqlx::query("update skus set sku_kind='KIT' where tenant_id=$1 and sku_id=$2")
        .bind(f.tenant.0)
        .bind(f.sku)
        .execute(&pool)
        .await?;
    eprintln!(
        "QA kit source direct tenant={} actor={} warehouse={} sku={} root={root}",
        f.tenant, f.actor, f.warehouse, f.sku
    );
    let (status, result) = api(&f, "releases", body(&f, root), None, None).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "catalog kind change cannot re-expand an existing direct source: {result}"
    );
    assert_eq!(
        saved(&independent, &f).await?["stock"]["balances"][0]["reserved_micros"],
        900_000
    );
    Ok(())
}
