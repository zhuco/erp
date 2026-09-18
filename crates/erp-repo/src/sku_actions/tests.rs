use super::*;
use sqlx::Executor;

fn command(source: u128, version: &str, column: u64, value: &str) -> ActionCommand {
    ActionCommand {
        operation_id: RowId(Uuid::new_v4().as_u128()),
        action: UPDATE.into(),
        source: RowId(source),
        expected_version: Some(version.into()),
        values: [(ColumnId(column), value.into())].into(),
    }
}

#[test]
fn sku_action_validation_enforces_columns_bounds_boolean_and_exact_money() {
    let version = "2026-09-10T00:00:00Z";
    assert!(validate(&command(1, version, 0, "SKU-1")).is_ok());
    assert!(validate(&command(1, version, 0, " ")).is_err());
    assert!(validate(&command(1, version, 0, &"x".repeat(256))).is_err());
    assert!(validate(&command(1, version, 3, "KIT")).is_err());
    assert!(validate(&command(1, version, 4, "12.34")).is_ok());
    assert!(validate(&command(1, version, 4, "12.345")).is_err());
    assert!(validate(&command(1, version, 4, "-0.01")).is_err());
    assert!(validate(&command(1, version, 5, "true")).is_ok());
    assert!(validate(&command(1, version, 5, "TRUE")).is_err());
    assert!(validate(&command(1, "not-a-date", 2, "name")).is_err());
    assert_eq!(format_cost(Some(-50)), "-0.50");
}

async fn setup() -> (PgPool, PgPool, String) {
    let url = std::env::var("ERP_GRID_TEST_DATABASE_URL").expect("explicit QA database required");
    let admin = PgPool::connect(&url).await.unwrap();
    let schema = format!("sku_action_test_{}", Uuid::new_v4().simple());
    admin
        .execute(format!("create schema {schema}").as_str())
        .await
        .unwrap();
    let search_path = format!("set search_path={schema},pg_catalog");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
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
    pool.execute("create table tenants(tenant_id uuid primary key); create table skus(sku_id uuid primary key,tenant_id uuid not null references tenants,product_id uuid not null,sku_code text not null,barcode text,name text,sku_kind text not null,cost_cent bigint,is_active boolean not null default true,status text not null default 'ACTIVE',created_at timestamptz not null default now(),updated_at timestamptz not null default now(),unique(tenant_id,sku_code)); create table sku_boms(tenant_id uuid,bundle_sku_id uuid,component_sku_id uuid,qty bigint); create table operation_logs(log_id uuid primary key default gen_random_uuid(),tenant_id uuid,actor_type text,actor_user_id uuid,action text,ref_type text,ref_id text,metadata jsonb); create table audit_logs(audit_id uuid primary key default gen_random_uuid(),tenant_id uuid,user_id uuid,action text,entity_type text,entity_id text,before_json jsonb,after_json jsonb); ").await.unwrap();
    pool.execute(include_str!(
        "../../../../migrations/0098_grid_sku_operations.sql"
    ))
    .await
    .unwrap();
    sqlx::query("insert into tenants values($1)")
        .bind(Uuid::from_u128(1))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("insert into skus(sku_id,tenant_id,product_id,sku_code,name,sku_kind,cost_cent) values($1,$3,$4,'NORMAL','Old','NORMAL',100),($2,$3,$4,'KIT','Kit','KIT',null)")
        .bind(Uuid::from_u128(10)).bind(Uuid::from_u128(11)).bind(Uuid::from_u128(1)).bind(Uuid::from_u128(20)).execute(&pool).await.unwrap();
    (admin, pool, schema)
}

#[tokio::test]
#[ignore = "Requires explicit ERP_GRID_TEST_DATABASE_URL; creates and removes only a fresh sku_action_test_* schema"]
async fn sku_action_postgres_cas_replay_identity_kit_and_audit_rollback() {
    let (admin, pool, schema) = setup().await;
    let tenant = TenantId(Uuid::from_u128(1));
    let actor = UserId(Uuid::from_u128(99));
    let version: DateTime<Utc> = sqlx::query_scalar("select updated_at from skus where sku_id=$1")
        .bind(Uuid::from_u128(10))
        .fetch_one(&pool)
        .await
        .unwrap();
    let save = command(10, &version.to_rfc3339(), 4, "12.34");
    let first = execute(&pool, tenant, actor, &save).await.unwrap();
    assert!(
        matches!(first.outcome, ActionOutcome::Applied { ref values, .. } if values[&ColumnId(4)] == "12.34")
    );
    assert_eq!(execute(&pool, tenant, actor, &save).await.unwrap(), first);
    assert_eq!(
        receipt(&pool, tenant, actor, save.operation_id)
            .await
            .unwrap(),
        Some(first.clone())
    );
    let mut changed = save.clone();
    changed.values.insert(ColumnId(2), "different".into());
    assert!(matches!(
        execute(&pool, tenant, actor, &changed).await,
        Err(ActionError::IdentityConflict)
    ));
    let stale = command(10, &version.to_rfc3339(), 2, "stale");
    assert!(
        matches!(execute(&pool, tenant, actor, &stale).await.unwrap().outcome, ActionOutcome::Rejected { code, .. } if code == "VERSION_CONFLICT")
    );
    let kit_version: DateTime<Utc> =
        sqlx::query_scalar("select updated_at from skus where sku_id=$1")
            .bind(Uuid::from_u128(11))
            .fetch_one(&pool)
            .await
            .unwrap();
    let kit = command(11, &kit_version.to_rfc3339(), 4, "1.00");
    assert!(
        matches!(execute(&pool, tenant, actor, &kit).await.unwrap().outcome, ActionOutcome::Rejected { code, .. } if code == "KIT_COST_READ_ONLY")
    );
    pool.execute("alter table audit_logs add constraint reject_sku_audit check(action <> 'skus.update') not valid").await.unwrap();
    let latest: DateTime<Utc> = sqlx::query_scalar("select updated_at from skus where sku_id=$1")
        .bind(Uuid::from_u128(10))
        .fetch_one(&pool)
        .await
        .unwrap();
    let interrupted = command(10, &latest.to_rfc3339(), 2, "must roll back");
    assert!(execute(&pool, tenant, actor, &interrupted).await.is_err());
    assert!(
        receipt(&pool, tenant, actor, interrupted.operation_id)
            .await
            .unwrap()
            .is_none()
    );
    let name: Option<String> = sqlx::query_scalar("select name from skus where sku_id=$1")
        .bind(Uuid::from_u128(10))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name.as_deref(), Some("Old"));
    pool.close().await;
    assert!(
        schema.starts_with("sku_action_test_")
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

#[test]
fn sku_action_historical_receipt_does_not_reapply_current_admission_rules() {
    let mut original = command(10, "2026-09-10T00:00:00Z", 2, "Old value");
    original.action = "skus.retired_action".into();
    assert!(validate(&original).is_err());
    let receipt = ActionReceipt {
        operation_id: original.operation_id,
        source: original.source,
        command_fingerprint: original.fingerprint(),
        outcome: rejected("REJECTED", "Original result"),
    };
    assert_eq!(
        decode_receipt(
            serde_json::to_value(&original).unwrap(),
            serde_json::to_value(&receipt).unwrap()
        )
        .unwrap(),
        receipt
    );
}
