use anyhow::Result;
use erp_repo::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    db::Db,
    operation_logs::{OperationLogCreate, OperationLogsRepo},
    order_action_logs::{OrderActionLogCreate, OrderActionLogsRepo},
    order_status_history::{OrderStatusHistoryCreate, OrderStatusHistoryRepo},
};
use erp_types::{OrderId, TenantId, UserId};
use serde_json::json;
use sqlx::PgPool;

fn configured_database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("TEST_DATABASE_URL").ok())
}

async fn setup_schema(pool: &PgPool) -> Result<()> {
    let statements = [
        "create extension if not exists pgcrypto",
        r#"
        create table if not exists tenants (
          tenant_id uuid primary key,
          name text not null,
          status text not null default 'ACTIVE',
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists order_status_history (
          history_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          from_status text,
          to_status text not null,
          action_code text,
          operator_id uuid,
          reason text,
          payload jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists order_action_log (
          action_log_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          order_id uuid not null,
          action_code text not null,
          idempotency_key text not null,
          status text not null default 'SUCCESS',
          operator_id uuid,
          request_json jsonb not null default '{}'::jsonb,
          response_json jsonb not null default '{}'::jsonb,
          error_message text not null default '',
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists operation_logs (
          log_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          actor_type text not null,
          actor_user_id uuid,
          action text not null,
          ref_type text,
          ref_id text,
          metadata jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists audit_logs (
          audit_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null references tenants(tenant_id),
          user_id uuid,
          action text not null,
          entity_type text not null,
          entity_id text,
          before_json jsonb,
          after_json jsonb,
          ip text,
          ua text,
          created_at timestamptz not null default now()
        )
        "#,
    ];

    for statement in statements {
        sqlx::query(statement).execute(pool).await?;
    }

    Ok(())
}

#[tokio::test]
async fn persistence_guard_rejects_blank_required_fields() -> Result<()> {
    let Some(database_url) = configured_database_url() else {
        eprintln!("skipping persistence_guard_rejects_blank_required_fields: DATABASE_URL not set");
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_uuid = uuid::Uuid::new_v4();
    let order_uuid = uuid::Uuid::new_v4();
    let user_uuid = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'guard-tenant')")
        .bind(tenant_uuid)
        .execute(db.pool())
        .await?;

    let tenant_id = TenantId(tenant_uuid);
    let order_id = OrderId(order_uuid);
    let user_id = UserId(user_uuid);

    let action_repo = OrderActionLogsRepo::new(db.pool());
    let action_err = action_repo
        .insert(OrderActionLogCreate {
            tenant_id,
            order_id,
            action_code: "   ".to_string(),
            idempotency_key: " ".to_string(),
            status: Some("SUCCESS".to_string()),
            operator_id: Some(user_id),
            request_json: Some(json!({"source": "test"})),
            response_json: Some(json!({"ok": true})),
            error_message: None,
        })
        .await
        .expect_err("blank action_code/idempotency_key should be rejected");
    assert!(
        action_err.to_string().contains("action_code")
            || action_err.to_string().contains("idempotency_key")
    );

    let history_repo = OrderStatusHistoryRepo::new(db.pool());
    let history_err = history_repo
        .insert(OrderStatusHistoryCreate {
            tenant_id,
            order_id,
            from_status: Some("PENDING_REVIEW".to_string()),
            to_status: " ".to_string(),
            action_code: Some(" ".to_string()),
            operator_id: Some(user_id),
            reason: Some("blank-status".to_string()),
            payload: Some(json!({"source": "test"})),
        })
        .await
        .expect_err("blank to_status/action_code should be rejected");
    assert!(
        history_err.to_string().contains("to_status")
            || history_err.to_string().contains("action_code")
    );

    let operation_repo = OperationLogsRepo::new(db.pool());
    let operation_err = operation_repo
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: " ".to_string(),
            actor_user_id: Some(user_id),
            action: " ".to_string(),
            ref_type: Some("ORDER".to_string()),
            ref_id: Some(order_uuid.to_string()),
            metadata: Some(json!({"scope": "guard"})),
        })
        .await
        .expect_err("blank actor_type/action should be rejected");
    assert!(
        operation_err.to_string().contains("actor_type")
            || operation_err.to_string().contains("action")
    );

    let audit_repo = AuditLogsRepo::new(db.pool());
    let audit_err = audit_repo
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(user_id),
            action: " ".to_string(),
            entity_type: " ".to_string(),
            entity_id: Some(order_uuid.to_string()),
            before_json: Some(json!({"status": "before"})),
            after_json: Some(json!({"status": "after"})),
            ip: Some("127.0.0.1".to_string()),
            ua: Some("task0-test".to_string()),
        })
        .await
        .expect_err("blank audit action/entity_type should be rejected");
    assert!(
        audit_err.to_string().contains("action") || audit_err.to_string().contains("entity_type")
    );

    Ok(())
}

#[tokio::test]
async fn persistence_guard_persists_valid_mutation_rows() -> Result<()> {
    let Some(database_url) = configured_database_url() else {
        eprintln!("skipping persistence_guard_persists_valid_mutation_rows: DATABASE_URL not set");
        return Ok(());
    };

    let db = Db::connect(&database_url).await?;
    setup_schema(db.pool()).await?;

    let tenant_uuid = uuid::Uuid::new_v4();
    let order_uuid = uuid::Uuid::new_v4();
    let user_uuid = uuid::Uuid::new_v4();

    sqlx::query("insert into tenants (tenant_id, name) values ($1, 'guard-tenant-ok')")
        .bind(tenant_uuid)
        .execute(db.pool())
        .await?;

    let tenant_id = TenantId(tenant_uuid);
    let order_id = OrderId(order_uuid);
    let user_id = UserId(user_uuid);

    OrderActionLogsRepo::new(db.pool())
        .insert(OrderActionLogCreate {
            tenant_id,
            order_id,
            action_code: "OMS.REVIEW_PASS".to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            status: Some("SUCCESS".to_string()),
            operator_id: Some(user_id),
            request_json: Some(json!({"request_id": "req-1"})),
            response_json: Some(json!({"state": "APPROVED"})),
            error_message: None,
        })
        .await?;

    OrderStatusHistoryRepo::new(db.pool())
        .insert(OrderStatusHistoryCreate {
            tenant_id,
            order_id,
            from_status: Some("PENDING_REVIEW".to_string()),
            to_status: "PENDING_ALLOCATE".to_string(),
            action_code: Some("OMS.REVIEW_PASS".to_string()),
            operator_id: Some(user_id),
            reason: Some("guard-pass".to_string()),
            payload: Some(json!({"request_id": "req-1"})),
        })
        .await?;

    OperationLogsRepo::new(db.pool())
        .insert(OperationLogCreate {
            tenant_id,
            actor_type: "USER".to_string(),
            actor_user_id: Some(user_id),
            action: "WMS.WAVE_SHIP".to_string(),
            ref_type: Some("OUTBOUND".to_string()),
            ref_id: Some(order_uuid.to_string()),
            metadata: Some(json!({"batch": "WAVE-1"})),
        })
        .await?;

    AuditLogsRepo::new(db.pool())
        .insert(AuditLogCreate {
            tenant_id,
            user_id: Some(user_id),
            action: "OMS.ORDER_EDIT".to_string(),
            entity_type: "ORDER".to_string(),
            entity_id: Some(order_uuid.to_string()),
            before_json: Some(json!({"logistics_company": "OLD"})),
            after_json: Some(json!({"logistics_company": "NEW"})),
            ip: Some("127.0.0.1".to_string()),
            ua: Some("task0-test".to_string()),
        })
        .await?;

    let action_count: i64 = sqlx::query_scalar(
        "select count(*) from order_action_log where tenant_id = $1 and order_id = $2",
    )
    .bind(tenant_uuid)
    .bind(order_uuid)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(action_count, 1);

    let history_count: i64 = sqlx::query_scalar(
        "select count(*) from order_status_history where tenant_id = $1 and order_id = $2",
    )
    .bind(tenant_uuid)
    .bind(order_uuid)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(history_count, 1);

    let operation_count: i64 = sqlx::query_scalar(
        "select count(*) from operation_logs where tenant_id = $1 and action = 'WMS.WAVE_SHIP'",
    )
    .bind(tenant_uuid)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(operation_count, 1);

    let audit_count: i64 = sqlx::query_scalar(
        "select count(*) from audit_logs where tenant_id = $1 and action = 'OMS.ORDER_EDIT'",
    )
    .bind(tenant_uuid)
    .fetch_one(db.pool())
    .await?;
    assert_eq!(audit_count, 1);

    Ok(())
}
