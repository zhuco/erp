use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};
use erp_repo::workflow_engine::{
    WORKFLOW_CALLBACK_STATUS_FAILED, WORKFLOW_CALLBACK_STATUS_PENDING,
    WORKFLOW_CALLBACK_STATUS_SUCCEEDED, WORKFLOW_STATUS_APPROVED, WORKFLOW_STATUS_PENDING,
    WorkflowAction, WorkflowActionCommand, WorkflowCallbackContract, WorkflowCallbackResultInput,
    WorkflowEngineRepo, WorkflowNodeInput, WorkflowSubmitInput,
};
use erp_types::{TenantId, UserId};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

fn month_start() -> NaiveDate {
    let now = Utc::now().date_naive();
    NaiveDate::from_ymd_opt(now.year(), now.month(), 1).expect("valid date")
}

async fn try_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/postgres".to_string());
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect(&url)
        .await
        .ok()
}

async fn setup_schema(pool: &PgPool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("select pg_advisory_xact_lock(9919001)")
        .execute(&mut *tx)
        .await?;

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
        create table if not exists audit_logs (
          audit_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
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
        r#"
        create table if not exists operation_logs (
          log_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
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
        create table if not exists outbox_events (
          tenant_id uuid not null,
          event_month date not null,
          outbox_id uuid not null default gen_random_uuid(),
          event_type text not null,
          payload jsonb not null,
          trace_id text,
          dedup_key text,
          status text not null default 'PENDING',
          attempts int not null default 0,
          next_retry_at timestamptz not null default now(),
          last_error text not null default '',
          last_attempt_at timestamptz not null default now(),
          published_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          primary key (tenant_id, event_month, outbox_id)
        )
        "#,
        r#"
        create unique index if not exists idx_outbox_dedup
        on outbox_events (tenant_id, event_month, dedup_key)
        where dedup_key is not null
        "#,
        r#"
        create table if not exists workflow_instances (
          instance_id uuid primary key,
          tenant_id uuid not null,
          workflow_code text not null,
          biz_domain text not null,
          biz_id text not null,
          status text not null,
          submitter_id uuid not null,
          current_step_seq int not null,
          total_steps int not null,
          callback_target_approved text not null,
          callback_target_rejected text not null,
          callback_target_withdrawn text not null,
          callback_payload jsonb not null default '{}'::jsonb,
          version bigint not null default 0,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists workflow_submit_dedup (
          tenant_id uuid not null,
          biz_domain text not null,
          biz_id text not null,
          workflow_code text not null,
          request_id text not null,
          instance_id uuid not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, biz_domain, biz_id, workflow_code, request_id)
        )
        "#,
        r#"
        create table if not exists workflow_steps (
          step_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          instance_id uuid not null,
          step_seq int not null,
          node_code text not null,
          assignee_user_id uuid not null,
          status text not null default 'PENDING',
          acted_by uuid,
          acted_at timestamptz,
          action_note text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, instance_id, step_seq, assignee_user_id)
        )
        "#,
        r#"
        create table if not exists workflow_todos (
          todo_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          instance_id uuid not null,
          step_seq int not null,
          assignee_user_id uuid not null,
          todo_type text not null,
          status text not null default 'OPEN',
          completed_at timestamptz,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, instance_id, step_seq, assignee_user_id, todo_type)
        )
        "#,
        r#"
        create table if not exists workflow_notifications (
          notification_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          instance_id uuid not null,
          user_id uuid not null,
          channel text not null,
          template_code text not null,
          payload jsonb not null default '{}'::jsonb,
          status text not null default 'PENDING',
          created_at timestamptz not null default now()
        )
        "#,
        r#"
        create table if not exists workflow_action_dedup (
          tenant_id uuid not null,
          instance_id uuid not null,
          action text not null,
          request_id text not null,
          action_log_id uuid not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, instance_id, action, request_id)
        )
        "#,
        r#"
        create table if not exists workflow_action_logs (
          action_log_id uuid primary key,
          tenant_id uuid not null,
          instance_id uuid not null,
          action text not null,
          request_id text not null,
          actor_user_id uuid,
          from_status text not null,
          to_status text not null,
          metadata jsonb not null default '{}'::jsonb,
          created_at timestamptz not null default now(),
          unique (tenant_id, instance_id, action, request_id)
        )
        "#,
        r#"
        create table if not exists workflow_callbacks (
          callback_id uuid primary key default gen_random_uuid(),
          tenant_id uuid not null,
          instance_id uuid not null,
          biz_domain text not null,
          biz_id text not null,
          contract_version text not null,
          target_status text not null,
          payload jsonb not null default '{}'::jsonb,
          status text not null default 'PENDING',
          retry_count int not null default 0,
          timeout_at timestamptz,
          last_error text,
          created_at timestamptz not null default now(),
          updated_at timestamptz not null default now(),
          unique (tenant_id, instance_id, target_status)
        )
        "#,
        r#"
        create table if not exists workflow_callback_receipts (
          tenant_id uuid not null,
          callback_id uuid not null,
          request_id text not null,
          action_log_id uuid not null,
          created_at timestamptz not null default now(),
          primary key (tenant_id, callback_id, request_id)
        )
        "#,
    ];

    for stmt in statements {
        sqlx::query(stmt).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn seed_tenant(pool: &PgPool, tenant_id: TenantId) -> Result<()> {
    sqlx::query("insert into tenants (tenant_id, name, status) values ($1,'wf-test','ACTIVE')")
        .bind(uuid::Uuid::from(tenant_id))
        .execute(pool)
        .await?;
    Ok(())
}

fn submit_input(
    tenant_id: TenantId,
    submitter_id: UserId,
    approver_id: UserId,
    biz_domain: &str,
    biz_id: &str,
    request_id: &str,
) -> WorkflowSubmitInput {
    WorkflowSubmitInput {
        tenant_id,
        workflow_code: "WF-UNIVERSAL-001".to_string(),
        biz_domain: biz_domain.to_string(),
        biz_id: biz_id.to_string(),
        submitter_id,
        request_id: request_id.to_string(),
        callback_contract: WorkflowCallbackContract {
            target_on_approved: "APPROVED".to_string(),
            target_on_rejected: "REJECTED".to_string(),
            target_on_withdrawn: "WITHDRAWN".to_string(),
            extra_payload: Some(serde_json::json!({"source": "workflow-test"})),
        },
        nodes: vec![WorkflowNodeInput {
            step_seq: 1,
            node_code: "manager_approval".to_string(),
            assignees: vec![approver_id],
        }],
    }
}

#[tokio::test]
async fn duplicate_approval_is_idempotent_and_emits_single_callback() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!(
            "skipping duplicate_approval_is_idempotent_and_emits_single_callback: db unavailable"
        );
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let submitter = UserId(uuid::Uuid::new_v4());
    let approver = UserId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = WorkflowEngineRepo::new(&pool);
    let submitted = repo
        .submit(submit_input(
            tenant_id,
            submitter,
            approver,
            "ORDER",
            "ORDER-1001",
            "submit-1001",
        ))
        .await?;
    assert_eq!(submitted.status, WORKFLOW_STATUS_PENDING);

    let first = repo
        .execute_action(WorkflowActionCommand {
            tenant_id,
            instance_id: submitted.instance_id,
            action: WorkflowAction::Approve,
            actor_user_id: approver,
            target_user_id: None,
            note: Some("approve once".to_string()),
            request_id: "approve-1001".to_string(),
            expected_version: Some(submitted.version),
        })
        .await?;
    assert!(!first.idempotent);
    assert_eq!(first.instance.status, WORKFLOW_STATUS_APPROVED);

    let second = repo
        .execute_action(WorkflowActionCommand {
            tenant_id,
            instance_id: submitted.instance_id,
            action: WorkflowAction::Approve,
            actor_user_id: approver,
            target_user_id: None,
            note: Some("duplicate".to_string()),
            request_id: "approve-1001".to_string(),
            expected_version: None,
        })
        .await?;
    assert!(second.idempotent);
    assert_eq!(second.instance.status, WORKFLOW_STATUS_APPROVED);

    let callbacks = repo
        .list_callbacks(tenant_id, submitted.instance_id)
        .await?;
    assert_eq!(callbacks.len(), 1);
    assert_eq!(callbacks[0].status, WORKFLOW_CALLBACK_STATUS_PENDING);

    let callback_done = repo
        .record_callback_result(WorkflowCallbackResultInput {
            tenant_id,
            callback_id: callbacks[0].callback_id,
            request_id: "cb-ok-1001".to_string(),
            success: true,
            timed_out: false,
            error_message: None,
        })
        .await?;
    assert!(!callback_done.idempotent);
    assert_eq!(
        callback_done.callback.status,
        WORKFLOW_CALLBACK_STATUS_SUCCEEDED
    );

    let callback_done_repeat = repo
        .record_callback_result(WorkflowCallbackResultInput {
            tenant_id,
            callback_id: callbacks[0].callback_id,
            request_id: "cb-ok-1001".to_string(),
            success: true,
            timed_out: false,
            error_message: None,
        })
        .await?;
    assert!(callback_done_repeat.idempotent);
    assert_eq!(
        callback_done_repeat.callback.status,
        WORKFLOW_CALLBACK_STATUS_SUCCEEDED
    );

    let logs = repo
        .list_action_logs(tenant_id, submitted.instance_id)
        .await?;
    let approve_count = logs.iter().filter(|item| item.action == "APPROVE").count();
    let callback_success_count = logs
        .iter()
        .filter(|item| item.action == "CALLBACK_SUCCEEDED")
        .count();
    assert_eq!(approve_count, 1);
    assert_eq!(callback_success_count, 1);

    let audit_count: i64 = sqlx::query_scalar(
        "select count(*) from audit_logs where tenant_id = $1 and (entity_id = $2 or entity_id = $3)",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(submitted.instance_id.to_string())
    .bind(callbacks[0].callback_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert!(
        audit_count >= 3,
        "audit trail should include submit/approve/callback records"
    );

    let outbox_callback_count: i64 = sqlx::query_scalar(
        "select count(*) from outbox_events where tenant_id = $1 and event_type = 'WORKFLOW_CALLBACK_PENDING' and event_month = $2",
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(month_start())
    .fetch_one(&pool)
    .await?;
    assert_eq!(outbox_callback_count, 1);

    Ok(())
}

#[tokio::test]
async fn concurrent_approvals_only_one_can_advance_status() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping concurrent_approvals_only_one_can_advance_status: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let submitter = UserId(uuid::Uuid::new_v4());
    let approver = UserId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = WorkflowEngineRepo::new(&pool);
    let submitted = repo
        .submit(submit_input(
            tenant_id,
            submitter,
            approver,
            "PROCUREMENT",
            "PO-2002",
            "submit-2002",
        ))
        .await?;

    let pool_a = pool.clone();
    let pool_b = pool.clone();
    let instance_id = submitted.instance_id;

    let f1 = async move {
        let repo = WorkflowEngineRepo::new(&pool_a);
        repo.execute_action(WorkflowActionCommand {
            tenant_id,
            instance_id,
            action: WorkflowAction::Approve,
            actor_user_id: approver,
            target_user_id: None,
            note: Some("parallel-1".to_string()),
            request_id: "approve-concurrent-a".to_string(),
            expected_version: None,
        })
        .await
    };

    let f2 = async move {
        let repo = WorkflowEngineRepo::new(&pool_b);
        repo.execute_action(WorkflowActionCommand {
            tenant_id,
            instance_id,
            action: WorkflowAction::Approve,
            actor_user_id: approver,
            target_user_id: None,
            note: Some("parallel-2".to_string()),
            request_id: "approve-concurrent-b".to_string(),
            expected_version: None,
        })
        .await
    };

    let (r1, r2) = tokio::join!(f1, f2);
    assert!(r1.is_ok() ^ r2.is_ok());

    let final_instance = repo.get_instance(tenant_id, instance_id).await?.unwrap();
    assert_eq!(final_instance.status, WORKFLOW_STATUS_APPROVED);
    let approve_count = repo
        .list_action_logs(tenant_id, instance_id)
        .await?
        .into_iter()
        .filter(|log| log.action == "APPROVE")
        .count();
    assert_eq!(approve_count, 1);

    Ok(())
}

#[tokio::test]
async fn withdraw_conflict_after_approval_is_rejected() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping withdraw_conflict_after_approval_is_rejected: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let submitter = UserId(uuid::Uuid::new_v4());
    let approver = UserId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = WorkflowEngineRepo::new(&pool);
    let submitted = repo
        .submit(submit_input(
            tenant_id,
            submitter,
            approver,
            "AFTER_SALES",
            "AS-3003",
            "submit-3003",
        ))
        .await?;

    repo.execute_action(WorkflowActionCommand {
        tenant_id,
        instance_id: submitted.instance_id,
        action: WorkflowAction::Approve,
        actor_user_id: approver,
        target_user_id: None,
        note: None,
        request_id: "approve-3003".to_string(),
        expected_version: None,
    })
    .await?;

    let withdraw_err = repo
        .execute_action(WorkflowActionCommand {
            tenant_id,
            instance_id: submitted.instance_id,
            action: WorkflowAction::Withdraw,
            actor_user_id: submitter,
            target_user_id: None,
            note: Some("late withdraw".to_string()),
            request_id: "withdraw-3003".to_string(),
            expected_version: None,
        })
        .await
        .expect_err("withdraw after approval should conflict");
    assert!(
        withdraw_err
            .to_string()
            .contains("workflow status does not allow withdraw"),
        "unexpected error: {withdraw_err}"
    );

    Ok(())
}

#[tokio::test]
async fn callback_timeout_can_retry_and_succeeds_once() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping callback_timeout_can_retry_and_succeeds_once: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let submitter = UserId(uuid::Uuid::new_v4());
    let approver = UserId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = WorkflowEngineRepo::new(&pool);
    let submitted = repo
        .submit(submit_input(
            tenant_id,
            submitter,
            approver,
            "EXPENSE",
            "EXP-4004",
            "submit-4004",
        ))
        .await?;

    repo.execute_action(WorkflowActionCommand {
        tenant_id,
        instance_id: submitted.instance_id,
        action: WorkflowAction::Approve,
        actor_user_id: approver,
        target_user_id: None,
        note: None,
        request_id: "approve-4004".to_string(),
        expected_version: None,
    })
    .await?;

    let callbacks = repo
        .list_callbacks(tenant_id, submitted.instance_id)
        .await?;
    assert_eq!(callbacks.len(), 1);

    let failed = repo
        .record_callback_result(WorkflowCallbackResultInput {
            tenant_id,
            callback_id: callbacks[0].callback_id,
            request_id: "cb-timeout-4004".to_string(),
            success: false,
            timed_out: true,
            error_message: Some("timeout".to_string()),
        })
        .await?;
    assert_eq!(failed.callback.status, WORKFLOW_CALLBACK_STATUS_FAILED);

    repo.retry_callback(
        tenant_id,
        callbacks[0].callback_id,
        "cb-retry-4004".to_string(),
    )
    .await?;

    let success = repo
        .record_callback_result(WorkflowCallbackResultInput {
            tenant_id,
            callback_id: callbacks[0].callback_id,
            request_id: "cb-success-4004".to_string(),
            success: true,
            timed_out: false,
            error_message: None,
        })
        .await?;
    assert_eq!(success.callback.status, WORKFLOW_CALLBACK_STATUS_SUCCEEDED);

    let repeat_success = repo
        .record_callback_result(WorkflowCallbackResultInput {
            tenant_id,
            callback_id: callbacks[0].callback_id,
            request_id: "cb-success-4004".to_string(),
            success: true,
            timed_out: false,
            error_message: None,
        })
        .await?;
    assert!(repeat_success.idempotent);
    assert_eq!(
        repeat_success.callback.status,
        WORKFLOW_CALLBACK_STATUS_SUCCEEDED
    );

    Ok(())
}

#[tokio::test]
async fn callback_contract_links_order_procurement_after_sales() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!("skipping callback_contract_links_order_procurement_after_sales: db unavailable");
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let submitter = UserId(uuid::Uuid::new_v4());
    let approver = UserId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = WorkflowEngineRepo::new(&pool);

    for (idx, (domain, biz_id)) in [
        ("ORDER", "ORDER-5001"),
        ("PROCUREMENT", "PO-5002"),
        ("AFTER_SALES", "AS-5003"),
    ]
    .iter()
    .enumerate()
    {
        let submitted = repo
            .submit(submit_input(
                tenant_id,
                submitter,
                approver,
                domain,
                biz_id,
                &format!("submit-link-{idx}"),
            ))
            .await?;

        let approved = repo
            .execute_action(WorkflowActionCommand {
                tenant_id,
                instance_id: submitted.instance_id,
                action: WorkflowAction::Approve,
                actor_user_id: approver,
                target_user_id: None,
                note: None,
                request_id: format!("approve-link-{idx}"),
                expected_version: None,
            })
            .await?;

        assert_eq!(approved.instance.status, WORKFLOW_STATUS_APPROVED);

        let callbacks = repo
            .list_callbacks(tenant_id, submitted.instance_id)
            .await?;
        assert_eq!(callbacks.len(), 1);
        assert_eq!(callbacks[0].biz_domain, *domain);
        assert_eq!(callbacks[0].biz_id, *biz_id);
        assert_eq!(callbacks[0].target_status, "APPROVED");
        let expected_instance_id = submitted.instance_id.to_string();
        assert_eq!(
            callbacks[0]
                .payload
                .get("workflow_instance_id")
                .and_then(|v| v.as_str()),
            Some(expected_instance_id.as_str())
        );
    }

    Ok(())
}

#[tokio::test]
async fn transfer_and_add_sign_update_todo_center_and_gate_approval() -> Result<()> {
    let Some(pool) = try_pool().await else {
        eprintln!(
            "skipping transfer_and_add_sign_update_todo_center_and_gate_approval: db unavailable"
        );
        return Ok(());
    };
    setup_schema(&pool).await?;

    let tenant_id = TenantId(uuid::Uuid::new_v4());
    let submitter = UserId(uuid::Uuid::new_v4());
    let approver_a = UserId(uuid::Uuid::new_v4());
    let approver_b = UserId(uuid::Uuid::new_v4());
    let approver_c = UserId(uuid::Uuid::new_v4());
    seed_tenant(&pool, tenant_id).await?;

    let repo = WorkflowEngineRepo::new(&pool);
    let submitted = repo
        .submit(submit_input(
            tenant_id,
            submitter,
            approver_a,
            "ORDER",
            "ORDER-6001",
            "submit-6001",
        ))
        .await?;

    repo.execute_action(WorkflowActionCommand {
        tenant_id,
        instance_id: submitted.instance_id,
        action: WorkflowAction::AddSign,
        actor_user_id: approver_a,
        target_user_id: Some(approver_b),
        note: Some("need co-sign".to_string()),
        request_id: "add-sign-6001".to_string(),
        expected_version: None,
    })
    .await?;

    repo.execute_action(WorkflowActionCommand {
        tenant_id,
        instance_id: submitted.instance_id,
        action: WorkflowAction::Transfer,
        actor_user_id: approver_a,
        target_user_id: Some(approver_c),
        note: Some("delegate".to_string()),
        request_id: "transfer-6001".to_string(),
        expected_version: None,
    })
    .await?;

    let todos_a = repo.list_open_todos(tenant_id, approver_a).await?;
    let todos_b = repo.list_open_todos(tenant_id, approver_b).await?;
    let todos_c = repo.list_open_todos(tenant_id, approver_c).await?;
    assert!(
        todos_a.is_empty(),
        "original approver should be transferred out"
    );
    assert_eq!(todos_b.len(), 1, "co-signer should have one open todo");
    assert_eq!(
        todos_c.len(),
        1,
        "transfer target should have one open todo"
    );

    let approve_c = repo
        .execute_action(WorkflowActionCommand {
            tenant_id,
            instance_id: submitted.instance_id,
            action: WorkflowAction::Approve,
            actor_user_id: approver_c,
            target_user_id: None,
            note: None,
            request_id: "approve-c-6001".to_string(),
            expected_version: None,
        })
        .await?;
    assert_eq!(
        approve_c.instance.status, WORKFLOW_STATUS_PENDING,
        "workflow must wait for co-signer approval"
    );

    let approve_b = repo
        .execute_action(WorkflowActionCommand {
            tenant_id,
            instance_id: submitted.instance_id,
            action: WorkflowAction::Approve,
            actor_user_id: approver_b,
            target_user_id: None,
            note: None,
            request_id: "approve-b-6001".to_string(),
            expected_version: None,
        })
        .await?;
    assert_eq!(approve_b.instance.status, WORKFLOW_STATUS_APPROVED);

    Ok(())
}
