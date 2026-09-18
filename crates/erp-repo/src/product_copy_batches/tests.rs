use super::*;
use sqlx::Executor;

fn request(batch_id: u128, token: u128, excluded: Vec<Uuid>, confirmed_count: u64) -> StartRequest {
    StartRequest {
        batch_id: Uuid::from_u128(batch_id),
        token: Uuid::from_u128(token),
        excluded,
        confirmed_count,
    }
}

#[test]
fn product_copy_batch_request_is_bounded_and_canonical() {
    let left = request(1, 2, vec![Uuid::from_u128(4), Uuid::from_u128(3)], 8);
    let right = request(1, 2, vec![Uuid::from_u128(3), Uuid::from_u128(4)], 8);
    assert_eq!(
        canonical_request(&left).unwrap(),
        canonical_request(&right).unwrap()
    );
    assert!(canonical_request(&request(0, 2, Vec::new(), 1)).is_err());
    assert!(canonical_request(&request(1, 2, Vec::new(), 0)).is_err());
    assert!(canonical_request(&request(1, 2, vec![Uuid::from_u128(3); 2], 1)).is_err());
    assert!(canonical_request(&request(1, 2, Vec::new(), MAX_TARGETS + 1)).is_err());
}

async fn setup() -> (PgPool, PgPool, String) {
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit QA database required");
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("product_copy_batch_test_{}", Uuid::new_v4().simple());
    admin
        .execute(format!("create schema {schema}").as_str())
        .await
        .unwrap();
    let search_path = format!("set search_path = {schema}, pg_catalog");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |conn, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                conn.execute(search_path.as_str()).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    pool.execute("create table tenants(tenant_id uuid primary key); create table shops(shop_id uuid primary key); create table suppliers(supplier_id uuid primary key); create table warehouses(warehouse_id uuid primary key)").await.unwrap();
    for ddl in [
        include_str!("../../../../migrations/0004_product_sku_schema.sql"),
        include_str!("../../../../migrations/0006_kit_bom.sql"),
        include_str!("../../../../migrations/0027_sku_cost.sql"),
        include_str!("../../../../migrations/0029_product_center.sql")
            .split_once("-- Order item BOM expansion trace")
            .unwrap()
            .0,
        include_str!("../../../../migrations/0054_product_module_enhancements.sql")
            .split_once("create table if not exists sku_suppliers")
            .unwrap()
            .0,
        include_str!("../../../../migrations/0070_sku_notes.sql"),
        include_str!("../../../../migrations/0077_products_negative_stock.sql"),
        include_str!("../../../../migrations/0078_product_management.sql"),
        include_str!("../../../../migrations/0017_operation_logs.sql"),
        include_str!("../../../../migrations/0030_system_management.sql")
            .split_once("create table if not exists audit_logs")
            .unwrap()
            .1
            .split_once("create index")
            .unwrap()
            .0,
        include_str!("../../../../migrations/0096_grid_product_operations.sql"),
        include_str!("../../../../migrations/0097_grid_product_selections.sql"),
        include_str!("../../../../migrations/0099_grid_product_copy_batches.sql"),
        include_str!("../../../../migrations/0100_grid_product_copy_batch_retries.sql"),
    ] {
        let ddl = if ddl.trim_start().starts_with('(') {
            format!("create table audit_logs {ddl}")
        } else {
            ddl.into()
        };
        pool.execute(ddl.as_str()).await.unwrap();
    }
    for tenant in [1_u128, 2] {
        sqlx::query("insert into tenants values ($1)")
            .bind(Uuid::from_u128(tenant))
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("insert into products(product_id,tenant_id,product_code,name,attrs,allow_negative_stock) values($1,$3,'ONE','One','{}',false),($2,$3,'TWO','Two','{}',false)")
        .bind(Uuid::from_u128(10))
        .bind(Uuid::from_u128(11))
        .bind(Uuid::from_u128(1))
        .execute(&pool)
        .await
        .unwrap();
    (admin, pool, schema)
}

async fn freeze(pool: &PgPool, token: Uuid, actor: Uuid, products: &[Uuid]) {
    let tenant = Uuid::from_u128(1);
    sqlx::query("insert into grid_product_selections(token,tenant_id,actor_user_id,normalized_query,query_fingerprint,total,expires_at) values($1,$2,$3,'{}','test',$4,clock_timestamp()+interval '5 minutes')")
        .bind(token)
        .bind(tenant)
        .bind(actor)
        .bind(products.len() as i64)
        .execute(pool)
        .await
        .unwrap();
    for (ordinal, product) in products.iter().enumerate() {
        let version: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
            "select updated_at from products where tenant_id=$1 and product_id=$2",
        )
        .bind(tenant)
        .bind(product)
        .fetch_one(pool)
        .await
        .unwrap();
        sqlx::query("insert into grid_product_selection_items(selection_token,ordinal,product_id,product_version,data) values($1,$2,$3,$4,'{}')")
            .bind(token)
            .bind(ordinal as i64)
            .bind(product)
            .bind(version)
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn cleanup(admin: PgPool, pool: PgPool, schema: String) {
    pool.close().await;
    admin
        .execute(format!("drop schema {schema} cascade").as_str())
        .await
        .unwrap();
    admin.close().await;
}

#[tokio::test]
#[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates and removes only a fresh product_copy_batch_test_* schema"]
async fn product_copy_batch_postgres_lifecycle_recovery_cancel_scope_and_cas() {
    let (admin, pool, schema) = setup().await;
    let tenant = TenantId(Uuid::from_u128(1));
    let actor = UserId(Uuid::from_u128(90));
    let actor_id = Uuid::from(actor);

    let token = Uuid::from_u128(100);
    freeze(
        &pool,
        token,
        actor_id,
        &[Uuid::from_u128(10), Uuid::from_u128(11)],
    )
    .await;
    let input = request(200, token.as_u128(), vec![Uuid::from_u128(11)], 1);
    assert_eq!(
        start(&pool, tenant, actor, &input).await.unwrap().pending,
        1
    );
    assert_eq!(
        start(&pool, tenant, actor, &input).await.unwrap().pending,
        1
    );
    let mut changed = input.clone();
    changed.token = Uuid::from_u128(999);
    assert!(matches!(
        start(&pool, tenant, actor, &changed).await,
        Err(BatchError::IdentityConflict)
    ));
    sqlx::query("update grid_product_selections set expires_at=clock_timestamp()-interval '1 second' where token=$1")
        .bind(token)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        start(&pool, tenant, actor, &input).await.unwrap().pending,
        1
    );
    let claim = claim(&pool, Uuid::from(tenant), actor_id, input.batch_id)
        .await
        .unwrap()
        .unwrap();
    let copied = product_actions::execute(&pool, tenant, actor, &claim.command)
        .await
        .unwrap();
    assert!(matches!(copied.outcome, ActionOutcome::Applied { .. }));
    assert!(matches!(
        crate::product_copy_batch_retries::prepare_retry(
            &pool,
            tenant,
            actor,
            input.batch_id,
            Uuid::from_u128(299)
        )
        .await,
        Err(crate::product_copy_batch_retries::RetryError::NotTerminal)
    ));
    let cancelling_unknown = cancel(&pool, tenant, actor, input.batch_id).await.unwrap();
    assert_eq!(
        (cancelling_unknown.pending, cancelling_unknown.unknown),
        (0, 1)
    );
    sqlx::query("update grid_product_copy_batch_items set lease_until=clock_timestamp()-interval '1 second' where batch_id=$1")
        .bind(input.batch_id)
        .execute(&pool)
        .await
        .unwrap();
    let recovered = advance(&pool, tenant, actor, input.batch_id, 1)
        .await
        .unwrap();
    assert_eq!(
        (recovered.applied, recovered.unknown, recovered.pending),
        (1, 0, 0)
    );
    assert_eq!(
        cancel(&pool, tenant, actor, input.batch_id).await.unwrap(),
        recovered
    );
    assert!(matches!(
        crate::product_copy_batch_retries::prepare_retry(
            &pool,
            tenant,
            actor,
            input.batch_id,
            Uuid::from_u128(298)
        )
        .await,
        Err(crate::product_copy_batch_retries::RetryError::NoRejected)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("select count(*) from products where tenant_id=$1")
            .bind(Uuid::from(tenant))
            .fetch_one(&pool)
            .await
            .unwrap(),
        3
    );
    assert!(matches!(
        status(&pool, tenant, UserId(Uuid::from_u128(91)), input.batch_id).await,
        Err(BatchError::Unavailable)
    ));

    let cancel_token = Uuid::from_u128(101);
    freeze(
        &pool,
        cancel_token,
        actor_id,
        &[Uuid::from_u128(10), Uuid::from_u128(11)],
    )
    .await;
    let cancel_input = request(201, cancel_token.as_u128(), Vec::new(), 2);
    start(&pool, tenant, actor, &cancel_input).await.unwrap();
    let cancelled = cancel(&pool, tenant, actor, cancel_input.batch_id)
        .await
        .unwrap();
    assert_eq!(
        (cancelled.cancelled, cancelled.pending, cancelled.unknown),
        (2, 0, 0)
    );
    assert!(cancelled.cancel_requested);
    assert!(matches!(
        crate::product_copy_batch_retries::prepare_retry(
            &pool,
            tenant,
            actor,
            cancel_input.batch_id,
            Uuid::from_u128(300)
        )
        .await,
        Err(crate::product_copy_batch_retries::RetryError::NoRejected)
    ));
    let first = crate::product_copy_batch_results::page(
        &pool,
        tenant,
        actor,
        cancel_input.batch_id,
        None,
        1,
        false,
    )
    .await
    .unwrap();
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.next_after, Some(first.items[0].ordinal));
    let second = crate::product_copy_batch_results::page(
        &pool,
        tenant,
        actor,
        cancel_input.batch_id,
        first.next_after,
        1,
        false,
    )
    .await
    .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.items[0].ordinal > first.items[0].ordinal);
    assert!(second.next_after.is_none());
    assert!(
        crate::product_copy_batch_results::page(
            &pool,
            tenant,
            UserId(Uuid::from_u128(91)),
            cancel_input.batch_id,
            None,
            100,
            false
        )
        .await
        .is_err()
    );

    let stale_token = Uuid::from_u128(102);
    freeze(
        &pool,
        stale_token,
        actor_id,
        &[Uuid::from_u128(10), Uuid::from_u128(11)],
    )
    .await;
    sqlx::query("update products set updated_at=updated_at+interval '1 second' where tenant_id=$1 and product_id=$2")
        .bind(Uuid::from(tenant))
        .bind(Uuid::from_u128(11))
        .execute(&pool)
        .await
        .unwrap();
    let stale_input = request(202, stale_token.as_u128(), Vec::new(), 2);
    start(&pool, tenant, actor, &stale_input).await.unwrap();
    let mut stale = advance(&pool, tenant, actor, stale_input.batch_id, 2)
        .await
        .unwrap();
    if stale.pending > 0 {
        stale = advance(&pool, tenant, actor, stale_input.batch_id, 1)
            .await
            .unwrap();
    }
    assert_eq!(
        (stale.applied, stale.rejected, stale.pending, stale.unknown),
        (1, 1, 0, 0)
    );
    let failed = crate::product_copy_batch_results::page(
        &pool,
        tenant,
        actor,
        stale_input.batch_id,
        None,
        100,
        true,
    )
    .await
    .unwrap();
    assert_eq!(failed.items.len(), 1);
    assert_eq!(failed.items[0].state, "REJECTED");
    assert!(
        matches!(failed.items[0].receipt.as_ref().unwrap().outcome,ActionOutcome::Rejected {ref code,..} if code=="VERSION_CONFLICT")
    );
    let failed_operation = RowId(failed.items[0].operation_id.as_u128());
    let original_receipt = product_actions::receipt(&pool, tenant, actor, failed_operation)
        .await
        .unwrap()
        .unwrap();
    let review_request = Uuid::from_u128(301);
    let review = crate::product_copy_batch_retries::prepare_retry(
        &pool,
        tenant,
        actor,
        stale_input.batch_id,
        review_request,
    )
    .await
    .unwrap();
    assert_eq!(review.selection.total, 1);
    let page = crate::product_selection::page(
        &pool,
        tenant,
        actor,
        review.selection.token,
        0,
        10,
        &[],
    )
    .await
    .unwrap();
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0]["product_id"], Uuid::from_u128(11).to_string());
    assert_eq!(
        product_actions::receipt(&pool, tenant, actor, failed_operation)
            .await
            .unwrap()
            .unwrap(),
        original_receipt
    );

    let retry_input = request(203, review.selection.token.as_u128(), Vec::new(), 1);
    start(&pool, tenant, actor, &retry_input).await.unwrap();
    let retried = advance(&pool, tenant, actor, retry_input.batch_id, 1)
        .await
        .unwrap();
    assert_eq!((retried.applied, retried.rejected), (1, 0));
    sqlx::query("update grid_product_selections set expires_at=clock_timestamp()-interval '1 second' where token=$1")
        .bind(review.selection.token)
        .execute(&pool)
        .await
        .unwrap();
    let replay = crate::product_copy_batch_retries::prepare_retry(
        &pool,
        tenant,
        actor,
        stale_input.batch_id,
        review_request,
    )
    .await
    .unwrap();
    assert_eq!(replay.selection.token, review.selection.token);
    assert_eq!(replay.selection.expires_at, review.selection.expires_at);
    assert!(matches!(
        crate::product_copy_batch_retries::prepare_retry(
            &pool,
            tenant,
            actor,
            input.batch_id,
            review_request
        )
        .await,
        Err(crate::product_copy_batch_retries::RetryError::IdentityConflict)
    ));
    assert!(matches!(
        crate::product_copy_batch_retries::prepare_retry(
            &pool,
            tenant,
            UserId(Uuid::from_u128(91)),
            stale_input.batch_id,
            Uuid::from_u128(302)
        )
        .await,
        Err(crate::product_copy_batch_retries::RetryError::Unavailable)
    ));
    sqlx::query("update products set status='DELETED' where tenant_id=$1 and product_id=$2")
        .bind(Uuid::from(tenant))
        .bind(Uuid::from_u128(11))
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        crate::product_copy_batch_retries::prepare_retry(
            &pool,
            tenant,
            actor,
            stale_input.batch_id,
            Uuid::from_u128(303)
        )
        .await,
        Err(crate::product_copy_batch_retries::RetryError::SourceUnavailable(source))
            if source == Uuid::from_u128(11)
    ));

    let listed = list(&pool, tenant, actor).await.unwrap();
    assert_eq!(listed.len(), 4);
    assert!(
        listed
            .iter()
            .all(|item| item.pending == 0 && item.unknown == 0)
    );
    cleanup(admin, pool, schema).await;
}
