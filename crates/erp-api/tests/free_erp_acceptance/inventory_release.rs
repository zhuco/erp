//! Real fulfillment reservation release and independent, held-connection readback.

use super::{Result, TenantId, isolated_pool};
use erp_repo::fulfillment::FulfillmentRepo;
use erp_types::OrderId;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

async fn fixture(pool: &PgPool) -> Result<(TenantId, Uuid, Uuid, OrderId)> {
    let tenant = TenantId(Uuid::new_v4());
    let warehouse = Uuid::new_v4();
    let sku = Uuid::new_v4();
    let order = OrderId(Uuid::new_v4());
    sqlx::query("insert into tenants(tenant_id,name) values($1,'QA-INVENTORY-RELEASE')")
        .bind(Uuid::from(tenant))
        .execute(pool)
        .await?;
    sqlx::query("insert into warehouses(tenant_id,warehouse_id,name,code) values($1,$2,'QA warehouse','QA-RELEASE')")
        .bind(Uuid::from(tenant)).bind(warehouse).execute(pool).await?;
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-RELEASE',(select product_id from qa_parent))")
        .bind(Uuid::from(tenant))
        .bind(sku)
        .execute(pool)
        .await?;
    sqlx::query("insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand,reserved) values($1,$2,$3,3,2)")
        .bind(Uuid::from(tenant)).bind(warehouse).bind(sku).execute(pool).await?;
    sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_in_transit,delta_frozen,reason_type) values($1,$2,$3,3,2,0,0,'QA_OPENING')")
        .bind(Uuid::from(tenant)).bind(warehouse).bind(sku).execute(pool).await?;
    for source_order in [order, OrderId(Uuid::new_v4())] {
        sqlx::query("insert into inventory_reservations(tenant_id,warehouse_id,sku_id,order_id,order_item_id,qty,status) values($1,$2,$3,$4,$5,1,'RESERVED')")
            .bind(Uuid::from(tenant)).bind(warehouse).bind(sku)
            .bind(Uuid::from(source_order)).bind(Uuid::new_v4()).execute(pool).await?;
    }
    eprintln!(
        "QA release tenant={} order={}",
        Uuid::from(tenant),
        Uuid::from(order)
    );
    Ok((tenant, warehouse, sku, order))
}

async fn readback(reader: &mut PgConnection, tenant: TenantId) -> Result<()> {
    let balance: (i64, i64) =
        sqlx::query_as("select on_hand,reserved from inventory_balances where tenant_id=$1")
            .bind(Uuid::from(tenant))
            .fetch_one(&mut *reader)
            .await?;
    assert_eq!(balance, (3, 1), "one source releases one unit exactly once");
    let ledger: (i64, i64, i64) = sqlx::query_as("select sum(delta_on_hand)::bigint,sum(delta_reserved)::bigint,count(*) filter(where reason_type='RESERVATION_RELEASE') from inventory_ledgers where tenant_id=$1")
        .bind(Uuid::from(tenant)).fetch_one(&mut *reader).await?;
    assert_eq!(ledger, (3, 1, 1), "balance and append-only ledger agree");
    let states: (i64, i64) = sqlx::query_as("select count(*) filter(where status='RESERVED'),count(*) filter(where status='RELEASED') from inventory_reservations where tenant_id=$1")
        .bind(Uuid::from(tenant)).fetch_one(&mut *reader).await?;
    assert_eq!(states, (1, 1), "the other source remains reserved");
    Ok(())
}

#[tokio::test]
async fn fulfillment_release_once_replay_and_other_source_readback() -> Result<()> {
    let pool = isolated_pool().await?;
    let (tenant, _warehouse, _sku, order) = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let repo = FulfillmentRepo::new(&pool);
    let cross_tenant = repo
        .release_order_reservations_keep_order(TenantId(Uuid::new_v4()), order)
        .await?;
    assert_eq!(cross_tenant.released, 0);
    assert_eq!(
        repo.release_order_reservations_keep_order(tenant, order)
            .await?
            .released,
        1
    );
    assert_eq!(
        repo.release_order_reservations_keep_order(tenant, order)
            .await?
            .released,
        0
    );
    readback(&mut reader, tenant).await
}

#[tokio::test]
async fn fulfillment_release_twenty_concurrent_calls_keep_one_effect() -> Result<()> {
    let pool = isolated_pool().await?;
    let (tenant, _warehouse, _sku, order) = fixture(&pool).await?;
    let mut reader = pool.acquire().await?;
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let pool = pool.clone();
        tasks.spawn(async move {
            FulfillmentRepo::new(&pool)
                .release_order_reservations_keep_order(tenant, order)
                .await
        });
    }
    let mut released = 0;
    while let Some(result) = tasks.join_next().await {
        released += result??.released;
    }
    assert_eq!(released, 1);
    readback(&mut reader, tenant).await
}

#[tokio::test]
async fn fulfillment_release_later_failure_rolls_back_all_sources() -> Result<()> {
    let pool = isolated_pool().await?;
    let (tenant, warehouse, first_sku, order) = fixture(&pool).await?;
    // The release query sorts SKU keys; this failing source is processed second.
    let second_sku = Uuid::from_u128(
        first_sku
            .as_u128()
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("fixture SKU identity overflow"))?,
    );
    sqlx::query("with qa_parent as (insert into products(tenant_id,product_id,name) values($1,gen_random_uuid(),'QA owned SKU parent') returning product_id) insert into skus(tenant_id,sku_id,sku_code,product_id) values($1,$2,'QA-BROKEN-RELEASE',(select product_id from qa_parent))")
        .bind(Uuid::from(tenant))
        .bind(second_sku)
        .execute(&pool)
        .await?;
    sqlx::query("insert into inventory_balances(tenant_id,warehouse_id,sku_id,on_hand,reserved) values($1,$2,$3,1,0)")
        .bind(Uuid::from(tenant)).bind(warehouse).bind(second_sku).execute(&pool).await?;
    sqlx::query("insert into inventory_reservations(tenant_id,warehouse_id,sku_id,order_id,order_item_id,qty,status) values($1,$2,$3,$4,$5,1,'RESERVED')")
        .bind(Uuid::from(tenant)).bind(warehouse).bind(second_sku)
        .bind(Uuid::from(order)).bind(Uuid::new_v4()).execute(&pool).await?;
    let mut reader = pool.acquire().await?;
    assert!(
        FulfillmentRepo::new(&pool)
            .release_order_reservations_keep_order(tenant, order)
            .await
            .is_err()
    );
    let balance: i64 = sqlx::query_scalar(
        "select reserved from inventory_balances where tenant_id=$1 and sku_id=$2",
    )
    .bind(Uuid::from(tenant))
    .bind(first_sku)
    .fetch_one(&mut *reader)
    .await?;
    assert_eq!(balance, 2);
    let (released, ledger): (i64, i64) = sqlx::query_as("select (select count(*) from inventory_reservations where tenant_id=$1 and status='RELEASED'), (select count(*) from inventory_ledgers where tenant_id=$1 and reason_type='RESERVATION_RELEASE')")
        .bind(Uuid::from(tenant)).fetch_one(&mut *reader).await?;
    assert_eq!((released, ledger), (0, 0));
    Ok(())
}
