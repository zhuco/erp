use super::*;
use sqlx::Executor;

fn command(action: &str, source: u128) -> ActionCommand {
    ActionCommand {
        operation_id: RowId(Uuid::new_v4().as_u128()),
        action: action.into(),
        source: RowId(source),
        expected_version: None,
        values: Default::default(),
    }
}

#[test]
fn product_actions_validate_identity_version_and_owned_columns() {
    let mut input = command(UPDATE, 1);
    assert!(validate(&input).is_err());
    input.expected_version = Some("2026-09-09T00:00:00Z".into());
    input.values.insert(ColumnId(1), "New name".into());
    assert!(validate(&input).is_ok());
    input.values.insert(ColumnId(5), "2026-09-09".into());
    assert!(validate(&input).is_err());
    input.values.clear();
    input.action = COPY.into();
    assert!(validate(&input).is_ok());
    input.action = "products.delete".into();
    assert!(validate(&input).is_err());
}

#[test]
fn product_actions_enrich_legacy_receipts_only_from_their_stored_command() {
    let input = command(COPY, 1);
    let legacy = serde_json::json!({
        "operation_id": input.operation_id, "source": input.source,
        "outcome": {"Rejected": {"code": "NOT_FOUND", "message": "Missing"}}
    });
    let restored = decode_receipt(serde_json::to_value(&input).unwrap(), legacy.clone()).unwrap();
    assert_eq!(restored.command_fingerprint, input.fingerprint());
    let mut corrupt = legacy.clone();
    corrupt["command_fingerprint"] = serde_json::json!("sha256-v1:wrong");
    assert!(decode_receipt(serde_json::to_value(&input).unwrap(), corrupt).is_err());
    let mut foreign = legacy;
    foreign["source"] = serde_json::json!(RowId(999));
    assert!(decode_receipt(serde_json::to_value(&input).unwrap(), foreign).is_err());
}

async fn setup() -> (PgPool, PgPool, String) {
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit QA database required");
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("grid_action_test_{}", Uuid::new_v4().simple());
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
    // Use actual product-related migration DDL, scoped to a fresh test schema.
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
    ] {
        let ddl = if ddl.trim_start().starts_with('(') {
            format!("create table audit_logs {ddl}")
        } else {
            ddl.into()
        };
        pool.execute(ddl.as_str()).await.unwrap();
    }
    for id in [1, 2] {
        sqlx::query("insert into tenants values ($1)")
            .bind(Uuid::from_u128(id))
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("insert into products(product_id, tenant_id, product_code, name, brand, attrs, primary_barcode, allow_negative_stock) values ($1, $3, 'SOURCE', 'Original', 'Brand', '{\"material\":\"cotton\"}', 'barcode-product', true), ($2, $3, 'OTHER', 'Other', null, '{}', null, false)")
        .bind(Uuid::from_u128(10)).bind(Uuid::from_u128(11)).bind(Uuid::from_u128(1)).execute(&pool).await.unwrap();
    sqlx::query("insert into skus(sku_id, tenant_id, product_id, sku_code, name, barcode, sku_kind, cost_cent, image_urls) values ($1, $4, $5, 'BUNDLE', 'Bundle', 'original-barcode', 'BUNDLE', 1234, array['https://assets.example/sku.png']), ($2, $4, $5, 'PART', 'Part', 'part-barcode', 'NORMAL', 500, '{}'), ($3, $4, $6, 'EXTERNAL', 'External', null, 'NORMAL', 800, '{}')")
        .bind(Uuid::from_u128(20)).bind(Uuid::from_u128(21)).bind(Uuid::from_u128(22)).bind(Uuid::from_u128(1)).bind(Uuid::from_u128(10)).bind(Uuid::from_u128(11)).execute(&pool).await.unwrap();
    sqlx::query("insert into sku_boms(tenant_id,bundle_sku_id,component_sku_id,qty) values ($1,$2,$3,2), ($1,$2,$4,3)")
        .bind(Uuid::from_u128(1)).bind(Uuid::from_u128(20)).bind(Uuid::from_u128(21)).bind(Uuid::from_u128(22)).execute(&pool).await.unwrap();
    (admin, pool, schema)
}

#[tokio::test]
#[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates and removes only a fresh grid_action_test_* schema"]
async fn product_actions_postgres_copy_cas_replay_concurrency_and_audit_atomicity() {
    let (admin, pool, schema) = setup().await;
    let tenant = TenantId(Uuid::from_u128(1));
    let actor = UserId(Uuid::from_u128(99));
    let copy_command = command(COPY, 10);
    let copied = execute(&pool, tenant, actor, &copy_command).await.unwrap();
    assert_eq!(copied.command_fingerprint, copy_command.fingerprint());
    // Simulate an existing 0.18 ledger row. Both lookup and replay must derive
    // the new fingerprint from that row's immutable command, without recopying.
    sqlx::query("update grid_product_operations set receipt = receipt - 'command_fingerprint' where tenant_id=$1 and actor_user_id=$2 and operation_id=$3")
        .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).bind(Uuid::from_u128(copy_command.operation_id.0))
        .execute(&pool).await.unwrap();
    let ActionOutcome::Applied { row: copied_id, .. } = copied.outcome else {
        panic!("copy rejected");
    };
    assert_ne!(copied_id, RowId(10));
    assert_eq!(
        execute(&pool, tenant, actor, &copy_command).await.unwrap(),
        copied
    );
    assert_eq!(
        receipt(&pool, tenant, actor, copy_command.operation_id)
            .await
            .unwrap(),
        Some(copied)
    );
    assert!(
        receipt(
            &pool,
            tenant,
            UserId(Uuid::from_u128(98)),
            copy_command.operation_id
        )
        .await
        .unwrap()
        .is_none()
    );
    let mut reused = copy_command.clone();
    reused.expected_version = Some("2026-09-09T00:00:00Z".into());
    assert!(matches!(
        execute(&pool, tenant, actor, &reused).await,
        Err(ActionError::IdentityConflict)
    ));
    let product: (Option<String>, bool, String, Value) = sqlx::query_as("select primary_barcode, allow_negative_stock, status, attrs from products where product_id=$1")
        .bind(Uuid::from_u128(copied_id.0)).fetch_one(&pool).await.unwrap();
    assert_eq!(
        product,
        (
            None,
            false,
            "INACTIVE".into(),
            serde_json::json!({"material":"cotton"})
        )
    );
    let skus: Vec<(Uuid, String, Option<String>, Option<i64>, bool, Vec<String>)> = sqlx::query_as("select sku_id, sku_kind, barcode, cost_cent, is_active, image_urls from skus where product_id=$1 order by sku_kind")
        .bind(Uuid::from_u128(copied_id.0)).fetch_all(&pool).await.unwrap();
    assert_eq!(skus.len(), 2);
    assert!(skus.iter().all(|s| s.2.is_none() && s.3.is_none() && !s.4));
    assert_eq!(skus[0].5, ["https://assets.example/sku.png"]);
    let components: Vec<(Uuid, i64)> = sqlx::query_as(
        "select component_sku_id,qty from sku_boms where bundle_sku_id=$1 order by qty",
    )
    .bind(skus[0].0)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(components, [(skus[1].0, 2), (Uuid::from_u128(22), 3)]);
    let mapping: Value =
        sqlx::query_scalar("select metadata->'copied_skus' from operation_logs where ref_id=$1")
            .bind(copied_id.to_uuid())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(mapping.as_array().unwrap().len(), 2);
    assert_eq!(mapping[0][0], Uuid::from_u128(20).to_string());
    assert_eq!(mapping[0][1], skus[0].0.to_string());
    let old_version: DateTime<Utc> =
        sqlx::query_scalar("select updated_at from products where product_id=$1")
            .bind(Uuid::from_u128(10))
            .fetch_one(&pool)
            .await
            .unwrap();
    let mut edit = command(UPDATE, 10);
    edit.expected_version = Some(old_version.to_rfc3339());
    edit.values.insert(ColumnId(1), "Updated".into());
    let saved = execute(&pool, tenant, actor, &edit).await.unwrap();
    assert!(matches!(saved.outcome, ActionOutcome::Applied { .. }));
    let mut stale = edit.clone();
    stale.operation_id = RowId(Uuid::new_v4().as_u128());
    stale.values.insert(ColumnId(1), "Lost update".into());
    assert!(
        matches!(execute(&pool, tenant, actor, &stale).await.unwrap().outcome, ActionOutcome::Rejected { code, .. } if code == "VERSION_CONFLICT")
    );
    assert_eq!(execute(&pool, tenant, actor, &edit).await.unwrap(), saved);
    let mut duplicate = command(UPDATE, 10);
    duplicate.expected_version = if let ActionOutcome::Applied { ref version, .. } = saved.outcome {
        Some(version.clone())
    } else {
        None
    };
    duplicate.values.insert(ColumnId(0), "OTHER".into());
    assert!(
        matches!(execute(&pool, tenant, actor, &duplicate).await.unwrap().outcome, ActionOutcome::Rejected { code, .. } if code == "DUPLICATE_CODE")
    );
    let foreign = execute(
        &pool,
        TenantId(Uuid::from_u128(2)),
        actor,
        &command(COPY, 10),
    )
    .await
    .unwrap();
    assert!(matches!(foreign.outcome, ActionOutcome::Rejected { code, .. } if code == "NOT_FOUND"));
    let concurrent = command(COPY, 10);
    let (left, right) = tokio::join!(
        execute(&pool, tenant, actor, &concurrent),
        execute(&pool, tenant, actor, &concurrent)
    );
    assert!(left.is_ok() || right.is_ok());
    let retry = execute(&pool, tenant, actor, &concurrent).await.unwrap();
    for result in [left, right].into_iter().flatten() {
        assert_eq!(result, retry);
    }
    let products_before: i64 = sqlx::query_scalar("select count(*) from products")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        products_before, 4,
        "sequential and concurrent retries must not create extra products"
    );
    pool.execute("alter table audit_logs add constraint reject_new_copy check(action <> 'products.copy') not valid").await.unwrap();
    let interrupted = command(COPY, 10);
    assert!(execute(&pool, tenant, actor, &interrupted).await.is_err());
    assert!(
        receipt(&pool, tenant, actor, interrupted.operation_id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("select count(*) from products")
            .fetch_one(&pool)
            .await
            .unwrap(),
        products_before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("select count(*) from operation_logs")
            .fetch_one(&pool)
            .await
            .unwrap(),
        3
    );
    pool.close().await;
    assert!(
        schema.starts_with("grid_action_test_")
            && schema
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    );
    admin
        .execute(format!("drop schema {schema} cascade").as_str())
        .await
        .unwrap();
    admin.close().await;
}
