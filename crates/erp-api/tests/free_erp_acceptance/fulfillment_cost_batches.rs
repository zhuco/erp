//! Real OMS cost and batch facts must survive precision, scope and rollback checks.
use super::fulfillment_sources::{Fulfillment, allocated, facts, fixture, request, request_as};
use super::{Result, isolated_pool};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use erp_domain::quantity::parse_qty;
use erp_repo::{
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn setup(pool: &PgPool, case: &str) -> Result<Fulfillment> {
    let f = fixture(pool).await?;
    sqlx::query("update tenants set name='QA-ERP-FULFILLMENT-COST-BATCHES' where tenant_id=$1")
        .bind(f.f.tenant.0)
        .execute(pool)
        .await?;
    eprintln!(
        "QA fulfillment cost_batch_fixture tenant={} case={case} order={} warehouse={} sku={}",
        f.f.tenant, f.order, f.f.warehouse, f.f.sku
    );
    Ok(f)
}

async fn saved(pool: &PgPool, f: &Fulfillment) -> Result<Value> {
    let mut value = facts(pool, f).await?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("object facts required"))?;
    for table in [
        "orders",
        "order_items",
        "order_index",
        "skus",
        "sku_batches",
        "order_item_components",
    ] {
        let rows: Value = sqlx::query_scalar(&format!("select coalesce(jsonb_agg(to_jsonb(x) order by to_jsonb(x)::text),'[]'::jsonb) from {table} x where tenant_id=$1"))
            .bind(f.f.tenant.0).fetch_one(pool).await?;
        object.insert(table.into(), rows);
    }
    Ok(value)
}

async fn reject(
    pool: &PgPool,
    f: &Fulfillment,
    shipment: Uuid,
    case: &str,
    expected: StatusCode,
    code: &str,
) -> Result<()> {
    let before = saved(pool, f).await?;
    let (status, body) = request(f, format!("shipments/{shipment}/ship"), json!({})).await?;
    let after = saved(pool, f).await?;
    eprintln!(
        "QA fulfillment cost_batch_result tenant={} case={case} status={} code={} unchanged={}",
        f.f.tenant,
        status.as_u16(),
        body["code"],
        before == after
    );
    assert_eq!(status, expected, "{case}: {body}");
    if code == "credentials changed" {
        // Current credentials are rejected by authentication before the OMS
        // transaction. The unchanged business facts remain the same contract.
        assert_eq!(body, json!("credentials changed"), "{case}: {body}");
    } else {
        assert_eq!(body["code"], code, "{case}: {body}");
    }
    assert_eq!(
        after, before,
        "{case}: no stock/source/cost/status/log/event effects"
    );
    Ok(())
}

async fn enable_batches(pool: &PgPool, f: &Fulfillment) -> Result<()> {
    sqlx::query("update skus set batch_enabled=true where tenant_id=$1 and sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.f.sku)
        .execute(pool)
        .await?;
    Ok(())
}

async fn batch(
    pool: &PgPool,
    f: &Fulfillment,
    name: &str,
    days: Option<i64>,
    hand: Option<i64>,
    held: Option<i64>,
) -> Result<Uuid> {
    let id = Uuid::new_v4();
    let expiry = days.map(|days| Utc::now() + Duration::days(days));
    sqlx::query("insert into sku_batches(tenant_id,sku_id,batch_id,batch_no,expire_at) values($1,$2,$3,$4,$5)")
        .bind(f.f.tenant.0).bind(f.f.sku).bind(id).bind(name).bind(expiry).execute(pool).await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand,reserved,on_hand_micros,reserved_micros) values($1,$2,$3,$4,2,0,$5,$6)")
        .bind(f.f.tenant.0).bind(f.f.warehouse).bind(f.f.sku).bind(id).bind(hand).bind(held).execute(pool).await?;
    eprintln!(
        "QA fulfillment cost_batch_batch tenant={} batch={id} name={name} hand={hand:?} held={held:?}",
        f.f.tenant
    );
    Ok(id)
}

#[tokio::test]
async fn fulfillment_cost_batch_unknown_cost_has_no_shipment_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "unknown_cost").await?;
    let shipment = allocated(&f).await?;
    sqlx::query("update skus set cost_cent=null where tenant_id=$1 and sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.f.sku)
        .execute(&pool)
        .await?;
    reject(
        &independent,
        &f,
        shipment,
        "unknown_cost",
        StatusCode::CONFLICT,
        "fulfillment_cost_unavailable",
    )
    .await
}

#[tokio::test]
async fn fulfillment_cost_batch_negative_cost_is_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "negative_cost").await?;
    let shipment = allocated(&f).await?;
    sqlx::query("update skus set cost_cent=-1 where tenant_id=$1 and sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.f.sku)
        .execute(&pool)
        .await?;
    reject(
        &independent,
        &f,
        shipment,
        "negative_cost",
        StatusCode::CONFLICT,
        "fulfillment_cost_invalid",
    )
    .await
}

#[tokio::test]
async fn fulfillment_cost_batch_cost_multiplication_overflow_is_rejected() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "cost_overflow").await?;
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.f.tenant,
                warehouse_id: f.f.warehouse,
                sku_id: f.f.sku,
            },
            on_hand: parse_qty("2")?,
            reserved: parse_qty("0")?,
            frozen: parse_qty("0")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_COST_OVERFLOW_OPENING".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;
    sqlx::query("update order_items set qty=2 where tenant_id=$1 and order_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.order)
        .execute(&pool)
        .await?;
    sqlx::query("update skus set cost_cent=$3 where tenant_id=$1 and sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.f.sku)
        .bind(i64::MAX)
        .execute(&pool)
        .await?;
    let shipment = allocated(&f).await?;
    reject(
        &independent,
        &f,
        shipment,
        "cost_overflow",
        StatusCode::UNPROCESSABLE_ENTITY,
        "fulfillment_cost_overflow",
    )
    .await
}

#[tokio::test]
async fn fulfillment_ship_role_session_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "role_session").await?;
    let shipment = allocated(&f).await?;
    eprintln!(
        "role-session-ship-tenant {} actor {} session {} shipment {}",
        f.f.tenant, f.f.actor, f.session_id, shipment
    );
    sqlx::query(
        "delete from role_permissions where tenant_id = $1 and perm_code = 'OMS.SHIPPING.WRITE'",
    )
    .bind(f.f.tenant.0)
    .execute(&pool)
    .await?;
    reject(
        &independent,
        &f,
        shipment,
        "role_revoked",
        StatusCode::FORBIDDEN,
        "fulfillment_role_denied",
    )
    .await?;
    sqlx::query("insert into role_permissions(tenant_id, role_id, perm_code) select $1, role_id, 'OMS.SHIPPING.WRITE' from user_roles where tenant_id = $1 and user_id = $2")
        .bind(f.f.tenant.0)
        .bind(f.f.actor)
        .execute(&pool)
        .await?;
    sqlx::query("update tenants set credentials_epoch = 2 where tenant_id = $1")
        .bind(f.f.tenant.0)
        .execute(&pool)
        .await?;
    reject(
        &independent,
        &f,
        shipment,
        "epoch_stale",
        StatusCode::UNAUTHORIZED,
        "credentials changed",
    )
    .await?;
    sqlx::query("update tenants set credentials_epoch = 1 where tenant_id = $1")
        .bind(f.f.tenant.0)
        .execute(&pool)
        .await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let shipped: (String, i64) = sqlx::query_as(
        "select status, count(*) from shipments where tenant_id = $1 and shipment_id = $2 group by status",
    )
    .bind(f.f.tenant.0)
    .bind(shipment)
    .fetch_one(&independent)
    .await?;
    assert_eq!(shipped.0, "SHIPPED");
    assert_eq!(shipped.1, 1);
    let ledgers: i64 =
        sqlx::query_scalar("select count(*) from inventory_ledgers where tenant_id = $1")
            .bind(f.f.tenant.0)
            .fetch_one(&independent)
            .await?;
    assert!(ledgers >= 1, "ship must consume stock");
    let session_epoch: i32 =
        sqlx::query_scalar("select credentials_epoch from auth_sessions where session_id = $1")
            .bind(f.session_id)
            .fetch_one(&independent)
            .await?;
    assert_eq!(session_epoch, 1);
    eprintln!(
        "role-session-ship-ok tenant={} shipment={shipment} ledgers={ledgers}",
        f.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_ship_other_tenant_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let owner = setup(&pool, "other_tenant_owner").await?;
    let foreign = setup(&pool, "other_tenant_foreign").await?;
    let shipment = allocated(&owner).await?;
    eprintln!(
        "other-tenant-ship owner={} owner_actor={} owner_session={} foreign={} foreign_actor={} foreign_session={} shipment={}",
        owner.f.tenant,
        owner.f.actor,
        owner.session_id,
        foreign.f.tenant,
        foreign.f.actor,
        foreign.session_id,
        shipment
    );
    let before = saved(&independent, &owner).await?;
    let (status, body) = request_as(
        &owner.app,
        owner.f.tenant.0,
        &foreign.token,
        format!("shipments/{shipment}/ship"),
        json!({}),
    )
    .await?;
    let after = saved(&independent, &owner).await?;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "fulfillment_access_denied", "{body}");
    assert_eq!(after, before, "foreign ship must not change owner stock");
    let foreign_shipments: i64 =
        sqlx::query_scalar("select count(*) from shipments where tenant_id = $1")
            .bind(foreign.f.tenant.0)
            .fetch_one(&independent)
            .await?;
    let foreign_sales: i64 = sqlx::query_scalar(
        "select count(*) from inventory_ledgers where tenant_id = $1 and reason_type = 'SALES_SHIP'",
    )
    .bind(foreign.f.tenant.0)
    .fetch_one(&independent)
    .await?;
    assert_eq!(foreign_shipments, 0);
    assert_eq!(foreign_sales, 0);
    let (status, body) = request(&owner, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let shipped: String = sqlx::query_scalar(
        "select status from shipments where tenant_id = $1 and shipment_id = $2",
    )
    .bind(owner.f.tenant.0)
    .bind(shipment)
    .fetch_one(&independent)
    .await?;
    assert_eq!(shipped, "SHIPPED");
    let sales: i64 = sqlx::query_scalar(
        "select count(*) from inventory_ledgers where tenant_id = $1 and reason_type = 'SALES_SHIP'",
    )
    .bind(owner.f.tenant.0)
    .fetch_one(&independent)
    .await?;
    assert_eq!(sales, 1);
    eprintln!(
        "other-tenant-ship-ok owner={} foreign={} shipment={shipment}",
        owner.f.tenant, foreign.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_ship_disabled_account_http_read_back() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let owner = setup(&pool, "disabled_account").await?;
    let shipment = allocated(&owner).await?;
    sqlx::query(
        "insert into role_permissions(tenant_id, role_id, perm_code) select $1, role_id, 'SYS.USER.WRITE' from user_roles where tenant_id = $1 and user_id = $2",
    )
    .bind(owner.f.tenant.0)
    .bind(owner.f.actor)
    .execute(&pool)
    .await?;
    eprintln!(
        "disabled-account-ship tenant={} actor={} session={} shipment={}",
        owner.f.tenant, owner.f.actor, owner.session_id, shipment
    );
    let response = owner
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/v1/tenants/{}/users/{}",
                    owner.f.tenant.0, owner.f.actor
                ))
                .header("authorization", format!("Bearer {}", owner.token))
                .header("content-type", "application/json")
                .body(Body::from(json!({"status":"DISABLED"}).to_string()))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["status"], "DISABLED", "{body}");
    let stored: (String, i32) = sqlx::query_as(
        "select u.status, t.credentials_epoch from users u join tenants t on t.tenant_id = u.tenant_id where u.tenant_id = $1 and u.user_id = $2",
    )
    .bind(owner.f.tenant.0)
    .bind(owner.f.actor)
    .fetch_one(&independent)
    .await?;
    assert_eq!(stored.0, "DISABLED");
    assert_eq!(stored.1, 2);
    let before_denial = saved(&independent, &owner).await?;
    let (status, body) = request(&owner, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body, json!("credentials changed"), "{body}");
    assert_eq!(
        saved(&independent, &owner).await?,
        before_denial,
        "disabled account's stale session creates no business effects"
    );
    let shipment_status: String = sqlx::query_scalar(
        "select status from shipments where tenant_id = $1 and shipment_id = $2",
    )
    .bind(owner.f.tenant.0)
    .bind(shipment)
    .fetch_one(&independent)
    .await?;
    assert_ne!(shipment_status, "SHIPPED");
    let sales: i64 = sqlx::query_scalar(
        "select count(*) from inventory_ledgers where tenant_id = $1 and reason_type = 'SALES_SHIP'",
    )
    .bind(owner.f.tenant.0)
    .fetch_one(&independent)
    .await?;
    assert_eq!(sales, 0);
    let session_epoch: i32 =
        sqlx::query_scalar("select credentials_epoch from auth_sessions where session_id = $1")
            .bind(owner.session_id)
            .fetch_one(&independent)
            .await?;
    assert_eq!(session_epoch, 1);
    eprintln!(
        "disabled-account-ship-ok tenant={} shipment={shipment} sales={sales}",
        owner.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_cost_batch_known_zero_cost_is_preserved() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "known_zero_cost").await?;
    let shipment = allocated(&f).await?;
    sqlx::query("update skus set cost_cent=0 where tenant_id=$1 and sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.f.sku)
        .execute(&pool)
        .await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    let cost: (i64, i64, i64) = sqlx::query_as("select qty,unit_cost_cent,total_cost_cent from order_cogs where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0).bind(shipment).fetch_one(&independent).await?;
    assert_eq!(cost, (1, 0, 0));
    let before = saved(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(saved(&independent, &f).await?, before);
    eprintln!(
        "QA fulfillment cost_batch_result tenant={} case=known_zero_cost status=200 unchanged=true",
        f.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_cost_batch_replay_keeps_original_cost_after_catalog_change() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "original_cost_replay").await?;
    let shipment = allocated(&f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    sqlx::query("update skus set cost_cent=null where tenant_id=$1 and sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(f.f.sku)
        .execute(&pool)
        .await?;
    let before = saved(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(saved(&independent, &f).await?, before);
    let cost: (i64, i64) = sqlx::query_as("select unit_cost_cent,total_cost_cent from order_cogs where tenant_id=$1 and shipment_id=$2")
        .bind(f.f.tenant.0).bind(shipment).fetch_one(&independent).await?;
    assert_eq!(cost, (2500, 2500));
    eprintln!(
        "QA fulfillment cost_batch_result tenant={} case=original_cost_replay status=200 unchanged=true",
        f.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_cost_batch_existing_incompatible_cost_fact_is_not_silently_skipped()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "existing_cost_fact").await?;
    let shipment = allocated(&f).await?;
    let item: Uuid = sqlx::query_scalar(
        "select shipment_item_id from shipment_items where tenant_id=$1 and shipment_id=$2",
    )
    .bind(f.f.tenant.0)
    .bind(shipment)
    .fetch_one(&pool)
    .await?;
    sqlx::query("insert into order_cogs(tenant_id,order_id,shipment_id,order_item_id,sku_id,qty,unit_cost_cent,total_cost_cent,ref_type,ref_id) values($1,$2,$3,$4,$5,99,1,1,'SHIPMENT_ITEM',$6)")
        .bind(f.f.tenant.0).bind(f.order).bind(shipment).bind(f.root).bind(f.f.sku).bind(item.to_string()).execute(&pool).await?;
    reject(
        &independent,
        &f,
        shipment,
        "existing_cost_fact",
        StatusCode::CONFLICT,
        "fulfillment_cost_conflict",
    )
    .await
}

#[tokio::test]
async fn fulfillment_cost_batch_fefo_uses_exact_free_quantities_and_preserves_held_batches()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "exact_fefo").await?;
    let shipment = allocated(&f).await?;
    enable_batches(&pool, &f).await?;
    let early = batch(&pool, &f, "EARLY", Some(1), Some(600_000), Some(100_000)).await?;
    let late = batch(&pool, &f, "NO-EXPIRY", None, Some(1_300_000), Some(500_000)).await?;
    let expired = batch(
        &pool,
        &f,
        "EXPIRED",
        Some(-1),
        Some(1_000_000),
        Some(300_000),
    )
    .await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    for (id, expected) in [
        (early, (0, 0, 100_000, 100_000)),
        (late, (0, 0, 800_000, 500_000)),
        (expired, (1, 0, 1_000_000, 300_000)),
    ] {
        let row: (i64, i64, i64, i64) = sqlx::query_as("select on_hand,reserved,on_hand_micros,reserved_micros from inventory_batch_balances where tenant_id=$1 and batch_id=$2")
            .bind(f.f.tenant.0).bind(id).fetch_one(&independent).await?;
        assert_eq!(row, expected, "batch {id}");
    }
    let before = saved(&independent, &f).await?;
    let (status, body) = request(&f, format!("shipments/{shipment}/ship"), json!({})).await?;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(saved(&independent, &f).await?, before);
    eprintln!(
        "QA fulfillment cost_batch_result tenant={} case=exact_fefo status=200 unchanged=true",
        f.f.tenant
    );
    Ok(())
}

#[tokio::test]
async fn fulfillment_cost_batch_expired_stock_cannot_be_shipped() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "expired_batches").await?;
    let shipment = allocated(&f).await?;
    enable_batches(&pool, &f).await?;
    batch(
        &pool,
        &f,
        "EXPIRED",
        Some(-1),
        Some(2_900_000),
        Some(900_000),
    )
    .await?;
    reject(
        &independent,
        &f,
        shipment,
        "expired_batches",
        StatusCode::CONFLICT,
        "inventory_insufficient",
    )
    .await
}

async fn metadata_mismatch(foreign: bool) -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let case = if foreign {
        "foreign_batch"
    } else {
        "wrong_sku_batch"
    };
    let f = setup(&pool, case).await?;
    let shipment = allocated(&f).await?;
    enable_batches(&pool, &f).await?;
    let id = batch(&pool, &f, "MISMATCH", None, Some(2_900_000), Some(900_000)).await?;
    let other = fixture(&pool).await?;
    if foreign {
        sqlx::query(
            "update sku_batches set tenant_id=$3,sku_id=$4 where tenant_id=$1 and batch_id=$2",
        )
        .bind(f.f.tenant.0)
        .bind(id)
        .bind(other.f.tenant.0)
        .bind(other.f.sku)
        .execute(&pool)
        .await?;
    } else {
        let other_sku = Uuid::new_v4();
        sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,cost_cent,product_id) values($1,$2,$3,2500,(select product_id from qa_parent))")
            .bind(f.f.tenant.0)
            .bind(other_sku)
            .bind(other_sku.to_string())
            .execute(&pool)
            .await?;
        sqlx::query("update sku_batches set sku_id=$3 where tenant_id=$1 and batch_id=$2")
            .bind(f.f.tenant.0)
            .bind(id)
            .bind(other_sku)
            .execute(&pool)
            .await?;
    }
    let foreign_before = saved(&independent, &other).await?;
    reject(
        &independent,
        &f,
        shipment,
        case,
        StatusCode::CONFLICT,
        "inventory_reconciliation_required",
    )
    .await?;
    assert_eq!(saved(&independent, &other).await?, foreign_before);
    Ok(())
}

#[tokio::test]
async fn fulfillment_cost_batch_foreign_enterprise_metadata_is_rejected() -> Result<()> {
    metadata_mismatch(true).await
}

#[tokio::test]
async fn fulfillment_cost_batch_other_sku_metadata_is_rejected() -> Result<()> {
    metadata_mismatch(false).await
}

#[tokio::test]
async fn fulfillment_cost_batch_unknown_precision_is_not_activated_by_shipping() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "unknown_batch_precision").await?;
    let shipment = allocated(&f).await?;
    enable_batches(&pool, &f).await?;
    batch(&pool, &f, "LEGACY", None, None, None).await?;
    reject(
        &independent,
        &f,
        shipment,
        "unknown_batch_precision",
        StatusCode::CONFLICT,
        "inventory_reconciliation_required",
    )
    .await
}

#[tokio::test]
async fn fulfillment_cost_batch_mismatched_batch_total_rolls_back_all_effects() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "batch_total_mismatch").await?;
    let shipment = allocated(&f).await?;
    enable_batches(&pool, &f).await?;
    batch(
        &pool,
        &f,
        "MISMATCH-TOTAL",
        None,
        Some(3_000_000),
        Some(900_000),
    )
    .await?;
    reject(
        &independent,
        &f,
        shipment,
        "batch_total_mismatch",
        StatusCode::CONFLICT,
        "inventory_reconciliation_required",
    )
    .await
}

#[tokio::test]
async fn fulfillment_cost_batch_over_limit_is_rejected_without_truncating_sources() -> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "batch_limit").await?;
    let shipment = allocated(&f).await?;
    enable_batches(&pool, &f).await?;
    sqlx::query("with batches as (insert into sku_batches(tenant_id,sku_id,batch_no) select $1,$2,'QA-LIMIT-'||n from generate_series(1,1001) n returning batch_id,batch_no) insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros) select $1,$3,$2,batch_id,case when batch_no='QA-LIMIT-1' then 1900000 else 1000 end,0 from batches")
        .bind(f.f.tenant.0).bind(f.f.sku).bind(f.f.warehouse).execute(&pool).await?;
    reject(
        &independent,
        &f,
        shipment,
        "batch_limit",
        StatusCode::UNPROCESSABLE_ENTITY,
        "inventory_batch_limit",
    )
    .await
}

#[tokio::test]
async fn fulfillment_cost_batch_later_sku_shortage_rolls_back_prior_stock_and_all_costs()
-> Result<()> {
    let pool = isolated_pool().await?;
    let independent = isolated_pool().await?;
    let f = setup(&pool, "later_batch_shortage").await?;
    let sku = Uuid::from_u128(
        f.f.sku
            .as_u128()
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("ordered QA sku"))?,
    );
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,cost_cent,product_id) values($1,$2,$3,2500,(select product_id from qa_parent))")
        .bind(f.f.tenant.0)
        .bind(sku)
        .bind(sku.to_string())
        .execute(&pool)
        .await?;
    InventoryService::new(&pool)
        .apply_precise(InventoryPreciseDelta {
            key: InventoryQuantityKey {
                tenant_id: f.f.tenant,
                warehouse_id: f.f.warehouse,
                sku_id: sku,
            },
            on_hand: parse_qty("2.9")?,
            reserved: parse_qty("0.9")?,
            frozen: parse_qty("0.9")?,
            in_transit: parse_qty("0")?,
            reason_type: "QA_LATER_BATCH_OPENING".into(),
            ref_type: None,
            ref_id: None,
            occurred_at: Utc::now(),
        })
        .await?;
    sqlx::query("insert into order_items(tenant_id,order_month,order_id,line_id,sku_id,qty) values($1,'2026-10-01',$2,$3,$4,1)")
        .bind(f.f.tenant.0).bind(f.order).bind(Uuid::new_v4().to_string()).bind(sku.to_string()).execute(&pool).await?;
    let shipment = allocated(&f).await?;
    sqlx::query("update skus set batch_enabled=true where tenant_id=$1 and sku_id=$2")
        .bind(f.f.tenant.0)
        .bind(sku)
        .execute(&pool)
        .await?;
    let batch_id = Uuid::new_v4();
    sqlx::query("insert into sku_batches(tenant_id,sku_id,batch_id,batch_no,expire_at) values($1,$2,$3,'LATER-EXPIRED',now()-interval '1 day')")
        .bind(f.f.tenant.0).bind(sku).bind(batch_id).execute(&pool).await?;
    sqlx::query("insert into inventory_batch_balances(tenant_id,warehouse_id,sku_id,batch_id,on_hand_micros,reserved_micros) values($1,$2,$3,$4,2900000,900000)")
        .bind(f.f.tenant.0).bind(f.f.warehouse).bind(sku).bind(batch_id).execute(&pool).await?;
    reject(
        &independent,
        &f,
        shipment,
        "later_batch_shortage",
        StatusCode::CONFLICT,
        "inventory_insufficient",
    )
    .await
}
