use anyhow::{anyhow, Context};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

pub const WORKFLOW_STATUS_PENDING: &str = "PENDING";
pub const WORKFLOW_STATUS_APPROVED: &str = "APPROVED";
pub const WORKFLOW_STATUS_REJECTED: &str = "REJECTED";
pub const WORKFLOW_STATUS_WITHDRAWN: &str = "WITHDRAWN";

pub const WORKFLOW_CALLBACK_STATUS_PENDING: &str = "PENDING";
pub const WORKFLOW_CALLBACK_STATUS_FAILED: &str = "FAILED";
pub const WORKFLOW_CALLBACK_STATUS_SUCCEEDED: &str = "SUCCEEDED";

const WORKFLOW_ENTITY_TYPE_INSTANCE: &str = "WORKFLOW_INSTANCE";
const WORKFLOW_ENTITY_TYPE_CALLBACK: &str = "WORKFLOW_CALLBACK";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    pub instance_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub workflow_code: String,
    pub biz_domain: String,
    pub biz_id: String,
    pub status: String,
    pub submitter_id: UserId,
    pub current_step_seq: i32,
    pub total_steps: i32,
    pub callback_target_approved: String,
    pub callback_target_rejected: String,
    pub callback_target_withdrawn: String,
    pub callback_payload: Value,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTodo {
    pub todo_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub instance_id: uuid::Uuid,
    pub step_seq: i32,
    pub assignee_user_id: UserId,
    pub todo_type: String,
    pub status: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionLog {
    pub action_log_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub instance_id: uuid::Uuid,
    pub action: String,
    pub request_id: String,
    pub actor_user_id: Option<UserId>,
    pub from_status: String,
    pub to_status: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCallback {
    pub callback_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub instance_id: uuid::Uuid,
    pub biz_domain: String,
    pub biz_id: String,
    pub contract_version: String,
    pub target_status: String,
    pub payload: Value,
    pub status: String,
    pub retry_count: i32,
    pub timeout_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WorkflowNodeInput {
    pub step_seq: i32,
    pub node_code: String,
    pub assignees: Vec<UserId>,
}

#[derive(Debug, Clone)]
pub struct WorkflowCallbackContract {
    pub target_on_approved: String,
    pub target_on_rejected: String,
    pub target_on_withdrawn: String,
    pub extra_payload: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct WorkflowSubmitInput {
    pub tenant_id: TenantId,
    pub workflow_code: String,
    pub biz_domain: String,
    pub biz_id: String,
    pub submitter_id: UserId,
    pub request_id: String,
    pub callback_contract: WorkflowCallbackContract,
    pub nodes: Vec<WorkflowNodeInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowAction {
    Approve,
    Reject,
    Withdraw,
    Transfer,
    AddSign,
}

impl WorkflowAction {
    fn as_str(self) -> &'static str {
        match self {
            WorkflowAction::Approve => "APPROVE",
            WorkflowAction::Reject => "REJECT",
            WorkflowAction::Withdraw => "WITHDRAW",
            WorkflowAction::Transfer => "TRANSFER",
            WorkflowAction::AddSign => "ADD_SIGN",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowActionCommand {
    pub tenant_id: TenantId,
    pub instance_id: uuid::Uuid,
    pub action: WorkflowAction,
    pub actor_user_id: UserId,
    pub target_user_id: Option<UserId>,
    pub note: Option<String>,
    pub request_id: String,
    pub expected_version: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct WorkflowActionResult {
    pub instance: WorkflowInstance,
    pub idempotent: bool,
}

#[derive(Debug, Clone)]
pub struct WorkflowCallbackResultInput {
    pub tenant_id: TenantId,
    pub callback_id: uuid::Uuid,
    pub request_id: String,
    pub success: bool,
    pub timed_out: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowCallbackResult {
    pub callback: WorkflowCallback,
    pub idempotent: bool,
}

pub struct WorkflowEngineRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WorkflowEngineRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn submit(&self, mut input: WorkflowSubmitInput) -> anyhow::Result<WorkflowInstance> {
        ensure_non_blank("workflow_code", &input.workflow_code)?;
        ensure_non_blank("biz_domain", &input.biz_domain)?;
        ensure_non_blank("biz_id", &input.biz_id)?;
        ensure_non_blank("request_id", &input.request_id)?;
        ensure_non_blank(
            "target_on_approved",
            &input.callback_contract.target_on_approved,
        )?;
        ensure_non_blank(
            "target_on_rejected",
            &input.callback_contract.target_on_rejected,
        )?;
        ensure_non_blank(
            "target_on_withdrawn",
            &input.callback_contract.target_on_withdrawn,
        )?;

        if input.nodes.is_empty() {
            return Err(anyhow!("workflow nodes required"));
        }

        input.nodes.sort_by_key(|node| node.step_seq);
        for node in &input.nodes {
            if node.step_seq <= 0 {
                return Err(anyhow!("step_seq must be positive"));
            }
            ensure_non_blank("node_code", &node.node_code)?;
            if node.assignees.is_empty() {
                return Err(anyhow!("node assignees required"));
            }
        }

        let first_step = input
            .nodes
            .first()
            .map(|node| node.step_seq)
            .ok_or_else(|| anyhow!("workflow nodes required"))?;
        let total_steps = input
            .nodes
            .iter()
            .map(|node| node.step_seq)
            .max()
            .unwrap_or(first_step);

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let proposed_instance_id = uuid::Uuid::new_v4();
        let inserted_instance: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            insert into workflow_submit_dedup(
                tenant_id, biz_domain, biz_id, workflow_code, request_id, instance_id
            ) values ($1,$2,$3,$4,$5,$6)
            on conflict (tenant_id, biz_domain, biz_id, workflow_code, request_id)
            do nothing
            returning instance_id
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.biz_domain)
        .bind(&input.biz_id)
        .bind(&input.workflow_code)
        .bind(&input.request_id)
        .bind(proposed_instance_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(instance_id) = inserted_instance {
            let callback_payload = input
                .callback_contract
                .extra_payload
                .clone()
                .unwrap_or_else(|| serde_json::json!({}));

            sqlx::query(
                r#"
                insert into workflow_instances(
                    instance_id, tenant_id, workflow_code, biz_domain, biz_id, status,
                    submitter_id, current_step_seq, total_steps,
                    callback_target_approved, callback_target_rejected, callback_target_withdrawn,
                    callback_payload, version
                ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,0)
                "#,
            )
            .bind(instance_id)
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(&input.workflow_code)
            .bind(&input.biz_domain)
            .bind(&input.biz_id)
            .bind(WORKFLOW_STATUS_PENDING)
            .bind(uuid::Uuid::from(input.submitter_id))
            .bind(first_step)
            .bind(total_steps)
            .bind(&input.callback_contract.target_on_approved)
            .bind(&input.callback_contract.target_on_rejected)
            .bind(&input.callback_contract.target_on_withdrawn)
            .bind(&callback_payload)
            .execute(&mut *tx)
            .await?;

            for node in &input.nodes {
                for assignee in &node.assignees {
                    sqlx::query(
                        r#"
                        insert into workflow_steps(
                            tenant_id, instance_id, step_seq, node_code, assignee_user_id, status
                        ) values ($1,$2,$3,$4,$5,'PENDING')
                        on conflict (tenant_id, instance_id, step_seq, assignee_user_id)
                        do nothing
                        "#,
                    )
                    .bind(uuid::Uuid::from(input.tenant_id))
                    .bind(instance_id)
                    .bind(node.step_seq)
                    .bind(&node.node_code)
                    .bind(uuid::Uuid::from(*assignee))
                    .execute(&mut *tx)
                    .await?;

                    if node.step_seq == first_step {
                        sqlx::query(
                            r#"
                            insert into workflow_todos(
                                tenant_id, instance_id, step_seq, assignee_user_id, todo_type, status
                            ) values ($1,$2,$3,$4,'APPROVAL','OPEN')
                            on conflict (tenant_id, instance_id, step_seq, assignee_user_id, todo_type)
                            do update set status = 'OPEN', updated_at = now(), completed_at = null
                            "#,
                        )
                        .bind(uuid::Uuid::from(input.tenant_id))
                        .bind(instance_id)
                        .bind(node.step_seq)
                        .bind(uuid::Uuid::from(*assignee))
                        .execute(&mut *tx)
                        .await?;

                        insert_notification(
                            &mut tx,
                            input.tenant_id,
                            instance_id,
                            *assignee,
                            "WORKFLOW.TODO.ASSIGNED",
                            serde_json::json!({
                                "instance_id": instance_id,
                                "step_seq": node.step_seq,
                                "biz_domain": input.biz_domain,
                                "biz_id": input.biz_id,
                            }),
                        )
                        .await?;
                    }
                }
            }

            let action_log_id = uuid::Uuid::new_v4();
            insert_action_log(
                &mut tx,
                WorkflowActionLogCreate {
                    action_log_id,
                    tenant_id: input.tenant_id,
                    instance_id,
                    action: "SUBMIT",
                    request_id: &input.request_id,
                    actor_user_id: Some(input.submitter_id),
                    from_status: "DRAFT",
                    to_status: WORKFLOW_STATUS_PENDING,
                    metadata: serde_json::json!({
                        "current_step_seq": first_step,
                        "total_steps": total_steps,
                    }),
                },
            )
            .await?;

            insert_audit_log(
                &mut tx,
                input.tenant_id,
                Some(input.submitter_id),
                "WORKFLOW.SUBMIT",
                WORKFLOW_ENTITY_TYPE_INSTANCE,
                Some(&instance_id.to_string()),
                None,
                Some(serde_json::json!({
                    "status": WORKFLOW_STATUS_PENDING,
                    "workflow_code": input.workflow_code,
                })),
            )
            .await?;

            insert_operation_log(
                &mut tx,
                input.tenant_id,
                Some(input.submitter_id),
                "WORKFLOW.SUBMIT",
                Some("WORKFLOW_INSTANCE"),
                Some(&instance_id.to_string()),
                serde_json::json!({
                    "request_id": input.request_id,
                    "biz_domain": input.biz_domain,
                    "biz_id": input.biz_id,
                }),
            )
            .await?;

            tx.commit().await?;
            let instance = self
                .get_instance(input.tenant_id, instance_id)
                .await?
                .context("created workflow instance missing")?;
            return Ok(instance);
        }

        let existing_instance_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            select instance_id from workflow_submit_dedup
            where tenant_id = $1 and biz_domain = $2 and biz_id = $3 and workflow_code = $4 and request_id = $5
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.biz_domain)
        .bind(&input.biz_id)
        .bind(&input.workflow_code)
        .bind(&input.request_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_instance(input.tenant_id, existing_instance_id)
            .await?
            .context("workflow instance not found for submit dedup")
    }
    pub async fn execute_action(
        &self,
        input: WorkflowActionCommand,
    ) -> anyhow::Result<WorkflowActionResult> {
        ensure_non_blank("request_id", &input.request_id)?;

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let action_name = input.action.as_str();
        let candidate_action_log_id = uuid::Uuid::new_v4();

        let inserted: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            insert into workflow_action_dedup(
                tenant_id, instance_id, action, request_id, action_log_id
            ) values ($1,$2,$3,$4,$5)
            on conflict (tenant_id, instance_id, action, request_id)
            do nothing
            returning action_log_id
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.instance_id)
        .bind(action_name)
        .bind(&input.request_id)
        .bind(candidate_action_log_id)
        .fetch_optional(&mut *tx)
        .await?;

        if inserted.is_none() {
            let existing = get_instance_tx(&mut tx, input.tenant_id, input.instance_id)
                .await?
                .context("workflow instance not found")?;
            tx.commit().await?;
            return Ok(WorkflowActionResult {
                instance: existing,
                idempotent: true,
            });
        }

        let mut instance = get_instance_for_update(&mut tx, input.tenant_id, input.instance_id)
            .await?
            .context("workflow instance not found")?;

        if let Some(expected_version) = input.expected_version {
            if expected_version != instance.version {
                return Err(anyhow!(
                    "workflow version conflict: expected {}, actual {}",
                    expected_version,
                    instance.version
                ));
            }
        }

        let from_status = instance.status.clone();
        let mut to_status = from_status.clone();
        let action_metadata: Value;

        match input.action {
            WorkflowAction::Approve => {
                if instance.status != WORKFLOW_STATUS_PENDING {
                    return Err(anyhow!("workflow status does not allow approve"));
                }

                consume_actor_todo(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    instance.current_step_seq,
                    input.actor_user_id,
                )
                .await?;

                update_step_action(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    instance.current_step_seq,
                    input.actor_user_id,
                    "APPROVED",
                    input.note.as_deref(),
                )
                .await?;

                let remaining = count_open_todos(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    instance.current_step_seq,
                )
                .await?;

                if remaining == 0 {
                    let next_step = find_next_step(
                        &mut tx,
                        input.tenant_id,
                        instance.instance_id,
                        instance.current_step_seq,
                    )
                    .await?;
                    if let Some(next_step_seq) = next_step {
                        open_step_todos_and_notify(
                            &mut tx,
                            input.tenant_id,
                            instance.instance_id,
                            next_step_seq,
                            &instance.biz_domain,
                            &instance.biz_id,
                        )
                        .await?;
                        instance.current_step_seq = next_step_seq;
                        instance.version += 1;
                        update_instance_core(
                            &mut tx,
                            input.tenant_id,
                            instance.instance_id,
                            &instance.status,
                            instance.current_step_seq,
                            instance.version,
                        )
                        .await?;
                    } else {
                        to_status = WORKFLOW_STATUS_APPROVED.to_string();
                        instance.status = to_status.clone();
                        instance.version += 1;
                        update_instance_core(
                            &mut tx,
                            input.tenant_id,
                            instance.instance_id,
                            &instance.status,
                            instance.current_step_seq,
                            instance.version,
                        )
                        .await?;

                        enqueue_callback_for_terminal(
                            &mut tx,
                            &instance,
                            WORKFLOW_STATUS_APPROVED,
                            "APPROVE",
                        )
                        .await?;
                    }
                }

                action_metadata = serde_json::json!({
                    "step_seq": instance.current_step_seq,
                    "note": input.note,
                });
            }
            WorkflowAction::Reject => {
                if instance.status != WORKFLOW_STATUS_PENDING {
                    return Err(anyhow!("workflow status does not allow reject"));
                }

                consume_actor_todo(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    instance.current_step_seq,
                    input.actor_user_id,
                )
                .await?;

                update_step_action(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    instance.current_step_seq,
                    input.actor_user_id,
                    "REJECTED",
                    input.note.as_deref(),
                )
                .await?;

                close_all_open_todos(&mut tx, input.tenant_id, instance.instance_id).await?;

                to_status = WORKFLOW_STATUS_REJECTED.to_string();
                instance.status = to_status.clone();
                instance.version += 1;

                update_instance_core(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    &instance.status,
                    instance.current_step_seq,
                    instance.version,
                )
                .await?;

                enqueue_callback_for_terminal(
                    &mut tx,
                    &instance,
                    WORKFLOW_STATUS_REJECTED,
                    "REJECT",
                )
                .await?;

                action_metadata = serde_json::json!({
                    "note": input.note,
                });
            }
            WorkflowAction::Withdraw => {
                if instance.status != WORKFLOW_STATUS_PENDING {
                    return Err(anyhow!("workflow status does not allow withdraw"));
                }
                if instance.submitter_id != input.actor_user_id {
                    return Err(anyhow!("only submitter can withdraw workflow"));
                }

                close_all_open_todos(&mut tx, input.tenant_id, instance.instance_id).await?;
                to_status = WORKFLOW_STATUS_WITHDRAWN.to_string();
                instance.status = to_status.clone();
                instance.version += 1;
                update_instance_core(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    &instance.status,
                    instance.current_step_seq,
                    instance.version,
                )
                .await?;

                enqueue_callback_for_terminal(
                    &mut tx,
                    &instance,
                    WORKFLOW_STATUS_WITHDRAWN,
                    "WITHDRAW",
                )
                .await?;

                action_metadata = serde_json::json!({
                    "note": input.note,
                });
            }
            WorkflowAction::Transfer => {
                if instance.status != WORKFLOW_STATUS_PENDING {
                    return Err(anyhow!("workflow status does not allow transfer"));
                }
                let target_user = input
                    .target_user_id
                    .ok_or_else(|| anyhow!("target_user_id required for transfer"))?;

                transfer_todo(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    instance.current_step_seq,
                    input.actor_user_id,
                    target_user,
                    input.note.as_deref(),
                    &instance.biz_domain,
                    &instance.biz_id,
                )
                .await?;

                instance.version += 1;
                update_instance_core(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    &instance.status,
                    instance.current_step_seq,
                    instance.version,
                )
                .await?;

                action_metadata = serde_json::json!({
                    "target_user_id": target_user,
                    "note": input.note,
                });
            }
            WorkflowAction::AddSign => {
                if instance.status != WORKFLOW_STATUS_PENDING {
                    return Err(anyhow!("workflow status does not allow add-sign"));
                }
                let target_user = input
                    .target_user_id
                    .ok_or_else(|| anyhow!("target_user_id required for add-sign"))?;

                ensure_actor_has_open_todo(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    instance.current_step_seq,
                    input.actor_user_id,
                )
                .await?;

                add_sign_todo(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    instance.current_step_seq,
                    target_user,
                    &instance.biz_domain,
                    &instance.biz_id,
                )
                .await?;

                instance.version += 1;
                update_instance_core(
                    &mut tx,
                    input.tenant_id,
                    instance.instance_id,
                    &instance.status,
                    instance.current_step_seq,
                    instance.version,
                )
                .await?;

                action_metadata = serde_json::json!({
                    "target_user_id": target_user,
                    "note": input.note,
                });
            }
        }

        insert_action_log(
            &mut tx,
            WorkflowActionLogCreate {
                action_log_id: candidate_action_log_id,
                tenant_id: input.tenant_id,
                instance_id: input.instance_id,
                action: action_name,
                request_id: &input.request_id,
                actor_user_id: Some(input.actor_user_id),
                from_status: &from_status,
                to_status: &to_status,
                metadata: action_metadata,
            },
        )
        .await?;

        insert_audit_log(
            &mut tx,
            input.tenant_id,
            Some(input.actor_user_id),
            &format!("WORKFLOW.{action_name}"),
            WORKFLOW_ENTITY_TYPE_INSTANCE,
            Some(&input.instance_id.to_string()),
            Some(serde_json::json!({
                "status": from_status,
                "version": instance.version.saturating_sub(1),
            })),
            Some(serde_json::json!({
                "status": instance.status,
                "version": instance.version,
            })),
        )
        .await?;

        insert_operation_log(
            &mut tx,
            input.tenant_id,
            Some(input.actor_user_id),
            &format!("WORKFLOW.{action_name}"),
            Some("WORKFLOW_INSTANCE"),
            Some(&input.instance_id.to_string()),
            serde_json::json!({
                "request_id": input.request_id,
            }),
        )
        .await?;

        let result_instance = get_instance_tx(&mut tx, input.tenant_id, input.instance_id)
            .await?
            .context("workflow instance disappeared during action")?;

        tx.commit().await?;

        Ok(WorkflowActionResult {
            instance: result_instance,
            idempotent: false,
        })
    }

    pub async fn get_instance(
        &self,
        tenant_id: TenantId,
        instance_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WorkflowInstance>> {
        let mut tx = self.pool.begin().await?;
        let instance = get_instance_tx(&mut tx, tenant_id, instance_id).await?;
        tx.commit().await?;
        Ok(instance)
    }

    pub async fn list_action_logs(
        &self,
        tenant_id: TenantId,
        instance_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<WorkflowActionLog>> {
        let rows: Vec<WorkflowActionLogRow> = sqlx::query_as(
            r#"
            select
              action_log_id, tenant_id, instance_id, action, request_id, actor_user_id,
              from_status, to_status, metadata, created_at
            from workflow_action_logs
            where tenant_id = $1 and instance_id = $2
            order by created_at asc, action_log_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(instance_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(WorkflowActionLogRow::into_model)
            .collect())
    }

    pub async fn list_callbacks(
        &self,
        tenant_id: TenantId,
        instance_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<WorkflowCallback>> {
        let rows: Vec<WorkflowCallbackRow> = sqlx::query_as(
            r#"
            select
              callback_id, tenant_id, instance_id, biz_domain, biz_id, contract_version,
              target_status, payload, status, retry_count, timeout_at, last_error,
              created_at, updated_at
            from workflow_callbacks
            where tenant_id = $1 and instance_id = $2
            order by created_at asc, callback_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(instance_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(WorkflowCallbackRow::into_model)
            .collect())
    }

    pub async fn list_open_todos(
        &self,
        tenant_id: TenantId,
        assignee_user_id: UserId,
    ) -> anyhow::Result<Vec<WorkflowTodo>> {
        let rows: Vec<WorkflowTodoRow> = sqlx::query_as(
            r#"
            select
              todo_id, tenant_id, instance_id, step_seq, assignee_user_id,
              todo_type, status, completed_at, created_at, updated_at
            from workflow_todos
            where tenant_id = $1 and assignee_user_id = $2 and status = 'OPEN'
            order by created_at asc, todo_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(assignee_user_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(WorkflowTodoRow::into_model).collect())
    }
    pub async fn record_callback_result(
        &self,
        input: WorkflowCallbackResultInput,
    ) -> anyhow::Result<WorkflowCallbackResult> {
        ensure_non_blank("request_id", &input.request_id)?;

        let mut tx = self.pool.begin().await?;
        let candidate_action_log_id = uuid::Uuid::new_v4();

        let inserted: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            insert into workflow_callback_receipts(
                tenant_id, callback_id, request_id, action_log_id
            ) values ($1,$2,$3,$4)
            on conflict (tenant_id, callback_id, request_id)
            do nothing
            returning action_log_id
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.callback_id)
        .bind(&input.request_id)
        .bind(candidate_action_log_id)
        .fetch_optional(&mut *tx)
        .await?;

        if inserted.is_none() {
            let callback = get_callback_for_update(&mut tx, input.tenant_id, input.callback_id)
                .await?
                .context("workflow callback not found")?;
            tx.commit().await?;
            return Ok(WorkflowCallbackResult {
                callback,
                idempotent: true,
            });
        }

        let mut callback = get_callback_for_update(&mut tx, input.tenant_id, input.callback_id)
            .await?
            .context("workflow callback not found")?;

        let action_name = if input.success {
            "CALLBACK_SUCCEEDED"
        } else {
            "CALLBACK_FAILED"
        };

        let last_error = if input.success {
            None
        } else {
            let base = input
                .error_message
                .clone()
                .unwrap_or_else(|| "callback failed".to_string());
            if input.timed_out {
                Some(format!("timeout:{base}"))
            } else {
                Some(base)
            }
        };

        if input.success {
            callback.status = WORKFLOW_CALLBACK_STATUS_SUCCEEDED.to_string();
        } else {
            callback.status = WORKFLOW_CALLBACK_STATUS_FAILED.to_string();
            callback.retry_count += 1;
        }
        callback.last_error = last_error.clone();

        sqlx::query(
            r#"
            update workflow_callbacks
            set status = $3,
                retry_count = $4,
                last_error = $5,
                updated_at = now()
            where tenant_id = $1 and callback_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.callback_id)
        .bind(&callback.status)
        .bind(callback.retry_count)
        .bind(&callback.last_error)
        .execute(&mut *tx)
        .await?;

        let instance = get_instance_tx(&mut tx, input.tenant_id, callback.instance_id)
            .await?
            .context("workflow instance not found for callback")?;

        insert_action_log(
            &mut tx,
            WorkflowActionLogCreate {
                action_log_id: candidate_action_log_id,
                tenant_id: input.tenant_id,
                instance_id: callback.instance_id,
                action: action_name,
                request_id: &input.request_id,
                actor_user_id: None,
                from_status: &instance.status,
                to_status: &instance.status,
                metadata: serde_json::json!({
                    "callback_id": callback.callback_id,
                    "timed_out": input.timed_out,
                    "error_message": last_error,
                }),
            },
        )
        .await?;

        insert_audit_log(
            &mut tx,
            input.tenant_id,
            None,
            &format!("WORKFLOW.{action_name}"),
            WORKFLOW_ENTITY_TYPE_CALLBACK,
            Some(&callback.callback_id.to_string()),
            None,
            Some(serde_json::json!({
                "status": callback.status,
                "instance_id": callback.instance_id,
            })),
        )
        .await?;

        insert_operation_log(
            &mut tx,
            input.tenant_id,
            None,
            &format!("WORKFLOW.{action_name}"),
            Some("WORKFLOW_CALLBACK"),
            Some(&callback.callback_id.to_string()),
            serde_json::json!({
                "request_id": input.request_id,
                "timed_out": input.timed_out,
            }),
        )
        .await?;

        callback = get_callback_for_update(&mut tx, input.tenant_id, input.callback_id)
            .await?
            .context("workflow callback missing after update")?;

        tx.commit().await?;

        Ok(WorkflowCallbackResult {
            callback,
            idempotent: false,
        })
    }

    pub async fn retry_callback(
        &self,
        tenant_id: TenantId,
        callback_id: uuid::Uuid,
        request_id: String,
    ) -> anyhow::Result<WorkflowCallback> {
        ensure_non_blank("request_id", &request_id)?;
        let mut tx = self.pool.begin().await?;

        let callback = get_callback_for_update(&mut tx, tenant_id, callback_id)
            .await?
            .context("workflow callback not found")?;
        if callback.status != WORKFLOW_CALLBACK_STATUS_FAILED {
            return Err(anyhow!("workflow callback is not retryable"));
        }

        sqlx::query(
            r#"
            update workflow_callbacks
            set status = 'PENDING', updated_at = now()
            where tenant_id = $1 and callback_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(callback_id)
        .execute(&mut *tx)
        .await?;

        enqueue_callback_outbox(
            &mut tx,
            tenant_id,
            callback_id,
            &callback.payload,
            &format!("WF_CALLBACK_RETRY:{callback_id}:{request_id}"),
        )
        .await?;

        insert_action_log(
            &mut tx,
            WorkflowActionLogCreate {
                action_log_id: uuid::Uuid::new_v4(),
                tenant_id,
                instance_id: callback.instance_id,
                action: "CALLBACK_RETRY",
                request_id: &request_id,
                actor_user_id: None,
                from_status: WORKFLOW_STATUS_PENDING,
                to_status: WORKFLOW_STATUS_PENDING,
                metadata: serde_json::json!({
                    "callback_id": callback_id,
                }),
            },
        )
        .await?;

        let updated = get_callback_for_update(&mut tx, tenant_id, callback_id)
            .await?
            .context("workflow callback not found after retry")?;

        tx.commit().await?;
        Ok(updated)
    }
}

fn ensure_non_blank(field: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{field} must not be blank"));
    }
    Ok(())
}

fn month_start(dt: DateTime<Utc>) -> NaiveDate {
    let d = dt.date_naive();
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
}

async fn get_instance_for_update(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
) -> anyhow::Result<Option<WorkflowInstance>> {
    let row: Option<WorkflowInstanceRow> = sqlx::query_as(
        r#"
        select
          instance_id, tenant_id, workflow_code, biz_domain, biz_id, status,
          submitter_id, current_step_seq, total_steps,
          callback_target_approved, callback_target_rejected, callback_target_withdrawn,
          callback_payload, version, created_at, updated_at
        from workflow_instances
        where tenant_id = $1 and instance_id = $2
        for update
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(WorkflowInstanceRow::into_model))
}

async fn get_instance_tx(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
) -> anyhow::Result<Option<WorkflowInstance>> {
    let row: Option<WorkflowInstanceRow> = sqlx::query_as(
        r#"
        select
          instance_id, tenant_id, workflow_code, biz_domain, biz_id, status,
          submitter_id, current_step_seq, total_steps,
          callback_target_approved, callback_target_rejected, callback_target_withdrawn,
          callback_payload, version, created_at, updated_at
        from workflow_instances
        where tenant_id = $1 and instance_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(WorkflowInstanceRow::into_model))
}

async fn update_instance_core(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    status: &str,
    current_step_seq: i32,
    version: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        update workflow_instances
        set status = $3,
            current_step_seq = $4,
            version = $5,
            updated_at = now()
        where tenant_id = $1 and instance_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(status)
    .bind(current_step_seq)
    .bind(version)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn consume_actor_todo(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    step_seq: i32,
    actor_user_id: UserId,
) -> anyhow::Result<()> {
    let affected = sqlx::query(
        r#"
        update workflow_todos
        set status = 'DONE', completed_at = now(), updated_at = now()
        where tenant_id = $1
          and instance_id = $2
          and step_seq = $3
          and assignee_user_id = $4
          and status = 'OPEN'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(actor_user_id))
    .execute(&mut **tx)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(anyhow!("actor has no open todo in current step"));
    }
    Ok(())
}

async fn ensure_actor_has_open_todo(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    step_seq: i32,
    actor_user_id: UserId,
) -> anyhow::Result<()> {
    let exists: Option<uuid::Uuid> = sqlx::query_scalar(
        r#"
        select todo_id from workflow_todos
        where tenant_id = $1
          and instance_id = $2
          and step_seq = $3
          and assignee_user_id = $4
          and status = 'OPEN'
        limit 1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(actor_user_id))
    .fetch_optional(&mut **tx)
    .await?;

    if exists.is_none() {
        return Err(anyhow!("actor has no open todo in current step"));
    }
    Ok(())
}
async fn update_step_action(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    step_seq: i32,
    actor_user_id: UserId,
    status: &str,
    note: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        update workflow_steps
        set status = $6,
            acted_by = $5,
            acted_at = now(),
            action_note = $7,
            updated_at = now()
        where tenant_id = $1
          and instance_id = $2
          and step_seq = $3
          and assignee_user_id = $4
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(actor_user_id))
    .bind(uuid::Uuid::from(actor_user_id))
    .bind(status)
    .bind(note)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn count_open_todos(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    step_seq: i32,
) -> anyhow::Result<i64> {
    let count: i64 = sqlx::query_scalar(
        r#"
        select count(*)
        from workflow_todos
        where tenant_id = $1
          and instance_id = $2
          and step_seq = $3
          and status = 'OPEN'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

async fn find_next_step(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    current_step_seq: i32,
) -> anyhow::Result<Option<i32>> {
    let next: Option<i32> = sqlx::query_scalar(
        r#"
        select min(step_seq)
        from workflow_steps
        where tenant_id = $1
          and instance_id = $2
          and step_seq > $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(current_step_seq)
    .fetch_one(&mut **tx)
    .await?;
    Ok(next)
}

async fn open_step_todos_and_notify(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    step_seq: i32,
    biz_domain: &str,
    biz_id: &str,
) -> anyhow::Result<()> {
    let assignees: Vec<uuid::Uuid> = sqlx::query_scalar(
        r#"
        select assignee_user_id
        from workflow_steps
        where tenant_id = $1 and instance_id = $2 and step_seq = $3
        order by assignee_user_id asc
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .fetch_all(&mut **tx)
    .await?;

    for assignee in assignees {
        sqlx::query(
            r#"
            insert into workflow_todos(
                tenant_id, instance_id, step_seq, assignee_user_id, todo_type, status
            ) values ($1,$2,$3,$4,'APPROVAL','OPEN')
            on conflict (tenant_id, instance_id, step_seq, assignee_user_id, todo_type)
            do update set status = 'OPEN', completed_at = null, updated_at = now()
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(instance_id)
        .bind(step_seq)
        .bind(assignee)
        .execute(&mut **tx)
        .await?;

        insert_notification(
            tx,
            tenant_id,
            instance_id,
            UserId(assignee),
            "WORKFLOW.TODO.ASSIGNED",
            serde_json::json!({
                "instance_id": instance_id,
                "step_seq": step_seq,
                "biz_domain": biz_domain,
                "biz_id": biz_id,
            }),
        )
        .await?;
    }

    Ok(())
}

async fn close_all_open_todos(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        update workflow_todos
        set status = 'CANCELED', completed_at = coalesce(completed_at, now()), updated_at = now()
        where tenant_id = $1 and instance_id = $2 and status = 'OPEN'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn transfer_todo(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    step_seq: i32,
    actor_user_id: UserId,
    target_user_id: UserId,
    note: Option<&str>,
    biz_domain: &str,
    biz_id: &str,
) -> anyhow::Result<()> {
    let affected = sqlx::query(
        r#"
        update workflow_todos
        set status = 'CANCELED', completed_at = now(), updated_at = now()
        where tenant_id = $1
          and instance_id = $2
          and step_seq = $3
          and assignee_user_id = $4
          and status = 'OPEN'
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(actor_user_id))
    .execute(&mut **tx)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(anyhow!("actor has no open todo in current step"));
    }

    sqlx::query(
        r#"
        update workflow_steps
        set status = 'TRANSFERRED',
            acted_by = $5,
            acted_at = now(),
            action_note = $6,
            updated_at = now()
        where tenant_id = $1 and instance_id = $2 and step_seq = $3 and assignee_user_id = $4
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(actor_user_id))
    .bind(uuid::Uuid::from(actor_user_id))
    .bind(note)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        insert into workflow_steps(tenant_id, instance_id, step_seq, node_code, assignee_user_id, status)
        select tenant_id, instance_id, step_seq, node_code, $5, 'PENDING'
        from workflow_steps
        where tenant_id = $1 and instance_id = $2 and step_seq = $3 and assignee_user_id = $4
        on conflict (tenant_id, instance_id, step_seq, assignee_user_id)
        do update set status = 'PENDING', acted_by = null, acted_at = null, action_note = null, updated_at = now()
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(actor_user_id))
    .bind(uuid::Uuid::from(target_user_id))
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        insert into workflow_todos(tenant_id, instance_id, step_seq, assignee_user_id, todo_type, status)
        values ($1,$2,$3,$4,'APPROVAL','OPEN')
        on conflict (tenant_id, instance_id, step_seq, assignee_user_id, todo_type)
        do update set status = 'OPEN', completed_at = null, updated_at = now()
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(target_user_id))
    .execute(&mut **tx)
    .await?;

    insert_notification(
        tx,
        tenant_id,
        instance_id,
        target_user_id,
        "WORKFLOW.TODO.TRANSFERRED",
        serde_json::json!({
            "instance_id": instance_id,
            "step_seq": step_seq,
            "biz_domain": biz_domain,
            "biz_id": biz_id,
        }),
    )
    .await?;

    Ok(())
}

async fn add_sign_todo(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    step_seq: i32,
    target_user_id: UserId,
    biz_domain: &str,
    biz_id: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        insert into workflow_steps(tenant_id, instance_id, step_seq, node_code, assignee_user_id, status)
        values ($1,$2,$3,'ADDED_SIGN',$4,'PENDING')
        on conflict (tenant_id, instance_id, step_seq, assignee_user_id)
        do update set status = 'PENDING', acted_by = null, acted_at = null, action_note = null, updated_at = now()
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(target_user_id))
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        insert into workflow_todos(tenant_id, instance_id, step_seq, assignee_user_id, todo_type, status)
        values ($1,$2,$3,$4,'CO_SIGN','OPEN')
        on conflict (tenant_id, instance_id, step_seq, assignee_user_id, todo_type)
        do update set status = 'OPEN', completed_at = null, updated_at = now()
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(step_seq)
    .bind(uuid::Uuid::from(target_user_id))
    .execute(&mut **tx)
    .await?;

    insert_notification(
        tx,
        tenant_id,
        instance_id,
        target_user_id,
        "WORKFLOW.TODO.COSIGN",
        serde_json::json!({
            "instance_id": instance_id,
            "step_seq": step_seq,
            "biz_domain": biz_domain,
            "biz_id": biz_id,
        }),
    )
    .await?;

    Ok(())
}
async fn enqueue_callback_for_terminal(
    tx: &mut Transaction<'_, Postgres>,
    instance: &WorkflowInstance,
    terminal_status: &str,
    trigger_action: &str,
) -> anyhow::Result<uuid::Uuid> {
    let target_status = match terminal_status {
        WORKFLOW_STATUS_APPROVED => &instance.callback_target_approved,
        WORKFLOW_STATUS_REJECTED => &instance.callback_target_rejected,
        WORKFLOW_STATUS_WITHDRAWN => &instance.callback_target_withdrawn,
        other => return Err(anyhow!("unsupported terminal status for callback: {other}")),
    };

    let payload = serde_json::json!({
        "tenant_id": instance.tenant_id,
        "workflow_instance_id": instance.instance_id,
        "biz_domain": instance.biz_domain,
        "biz_id": instance.biz_id,
        "target_status": target_status,
        "trigger_action": trigger_action,
        "extra": instance.callback_payload,
    });

    let callback_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        insert into workflow_callbacks(
            tenant_id, instance_id, biz_domain, biz_id, contract_version,
            target_status, payload, status, timeout_at
        ) values ($1,$2,$3,$4,'v1',$5,$6,'PENDING', now() + interval '30 seconds')
        on conflict (tenant_id, instance_id, target_status)
        do update set payload = excluded.payload, updated_at = now()
        returning callback_id
        "#,
    )
    .bind(uuid::Uuid::from(instance.tenant_id))
    .bind(instance.instance_id)
    .bind(&instance.biz_domain)
    .bind(&instance.biz_id)
    .bind(target_status)
    .bind(&payload)
    .fetch_one(&mut **tx)
    .await?;

    enqueue_callback_outbox(
        tx,
        instance.tenant_id,
        callback_id,
        &payload,
        &format!("WF_CALLBACK:{callback_id}"),
    )
    .await?;

    Ok(callback_id)
}

async fn enqueue_callback_outbox(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    callback_id: uuid::Uuid,
    payload: &Value,
    dedup_key: &str,
) -> anyhow::Result<()> {
    let now = Utc::now();
    let event_month = month_start(now);
    sqlx::query(
        r#"
        insert into outbox_events(
            tenant_id, event_month, event_type, payload, dedup_key
        )
        select $1, $2, 'WORKFLOW_CALLBACK_PENDING', $3, $4
        where not exists (
            select 1
            from outbox_events
            where tenant_id = $1
              and event_month = $2
              and dedup_key = $4
        )
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(event_month)
    .bind(serde_json::json!({
        "callback_id": callback_id,
        "contract": payload,
    }))
    .bind(dedup_key)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

struct WorkflowActionLogCreate<'a> {
    action_log_id: uuid::Uuid,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    action: &'a str,
    request_id: &'a str,
    actor_user_id: Option<UserId>,
    from_status: &'a str,
    to_status: &'a str,
    metadata: Value,
}

async fn insert_action_log(
    tx: &mut Transaction<'_, Postgres>,
    input: WorkflowActionLogCreate<'_>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        insert into workflow_action_logs(
            action_log_id, tenant_id, instance_id, action, request_id,
            actor_user_id, from_status, to_status, metadata
        ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        "#,
    )
    .bind(input.action_log_id)
    .bind(uuid::Uuid::from(input.tenant_id))
    .bind(input.instance_id)
    .bind(input.action)
    .bind(input.request_id)
    .bind(input.actor_user_id.map(uuid::Uuid::from))
    .bind(input.from_status)
    .bind(input.to_status)
    .bind(input.metadata)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_notification(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    instance_id: uuid::Uuid,
    user_id: UserId,
    template_code: &str,
    payload: Value,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        insert into workflow_notifications(
            tenant_id, instance_id, user_id, channel, template_code, payload, status
        ) values ($1,$2,$3,'INBOX',$4,$5,'PENDING')
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(instance_id)
    .bind(uuid::Uuid::from(user_id))
    .bind(template_code)
    .bind(payload)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_audit_log(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    user_id: Option<UserId>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    before_json: Option<Value>,
    after_json: Option<Value>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        insert into audit_logs(
            tenant_id, user_id, action, entity_type, entity_id, before_json, after_json
        ) values ($1,$2,$3,$4,$5,$6,$7)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(user_id.map(uuid::Uuid::from))
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(before_json)
    .bind(after_json)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_operation_log(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    actor_user_id: Option<UserId>,
    action: &str,
    ref_type: Option<&str>,
    ref_id: Option<&str>,
    metadata: Value,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        insert into operation_logs(
            tenant_id, actor_type, actor_user_id, action, ref_type, ref_id, metadata
        ) values ($1,'USER',$2,$3,$4,$5,$6)
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(actor_user_id.map(uuid::Uuid::from))
    .bind(action)
    .bind(ref_type)
    .bind(ref_id)
    .bind(metadata)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn get_callback_for_update(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    callback_id: uuid::Uuid,
) -> anyhow::Result<Option<WorkflowCallback>> {
    let row: Option<WorkflowCallbackRow> = sqlx::query_as(
        r#"
        select
          callback_id, tenant_id, instance_id, biz_domain, biz_id, contract_version,
          target_status, payload, status, retry_count, timeout_at, last_error,
          created_at, updated_at
        from workflow_callbacks
        where tenant_id = $1 and callback_id = $2
        for update
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(callback_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(WorkflowCallbackRow::into_model))
}

#[derive(sqlx::FromRow)]
struct WorkflowInstanceRow {
    instance_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    workflow_code: String,
    biz_domain: String,
    biz_id: String,
    status: String,
    submitter_id: uuid::Uuid,
    current_step_seq: i32,
    total_steps: i32,
    callback_target_approved: String,
    callback_target_rejected: String,
    callback_target_withdrawn: String,
    callback_payload: Value,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WorkflowInstanceRow {
    fn into_model(self) -> WorkflowInstance {
        WorkflowInstance {
            instance_id: self.instance_id,
            tenant_id: TenantId(self.tenant_id),
            workflow_code: self.workflow_code,
            biz_domain: self.biz_domain,
            biz_id: self.biz_id,
            status: self.status,
            submitter_id: UserId(self.submitter_id),
            current_step_seq: self.current_step_seq,
            total_steps: self.total_steps,
            callback_target_approved: self.callback_target_approved,
            callback_target_rejected: self.callback_target_rejected,
            callback_target_withdrawn: self.callback_target_withdrawn,
            callback_payload: self.callback_payload,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WorkflowTodoRow {
    todo_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    instance_id: uuid::Uuid,
    step_seq: i32,
    assignee_user_id: uuid::Uuid,
    todo_type: String,
    status: String,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WorkflowTodoRow {
    fn into_model(self) -> WorkflowTodo {
        WorkflowTodo {
            todo_id: self.todo_id,
            tenant_id: TenantId(self.tenant_id),
            instance_id: self.instance_id,
            step_seq: self.step_seq,
            assignee_user_id: UserId(self.assignee_user_id),
            todo_type: self.todo_type,
            status: self.status,
            completed_at: self.completed_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WorkflowActionLogRow {
    action_log_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    instance_id: uuid::Uuid,
    action: String,
    request_id: String,
    actor_user_id: Option<uuid::Uuid>,
    from_status: String,
    to_status: String,
    metadata: Value,
    created_at: DateTime<Utc>,
}

impl WorkflowActionLogRow {
    fn into_model(self) -> WorkflowActionLog {
        WorkflowActionLog {
            action_log_id: self.action_log_id,
            tenant_id: TenantId(self.tenant_id),
            instance_id: self.instance_id,
            action: self.action,
            request_id: self.request_id,
            actor_user_id: self.actor_user_id.map(UserId),
            from_status: self.from_status,
            to_status: self.to_status,
            metadata: self.metadata,
            created_at: self.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WorkflowCallbackRow {
    callback_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    instance_id: uuid::Uuid,
    biz_domain: String,
    biz_id: String,
    contract_version: String,
    target_status: String,
    payload: Value,
    status: String,
    retry_count: i32,
    timeout_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WorkflowCallbackRow {
    fn into_model(self) -> WorkflowCallback {
        WorkflowCallback {
            callback_id: self.callback_id,
            tenant_id: TenantId(self.tenant_id),
            instance_id: self.instance_id,
            biz_domain: self.biz_domain,
            biz_id: self.biz_id,
            contract_version: self.contract_version,
            target_status: self.target_status,
            payload: self.payload,
            status: self.status,
            retry_count: self.retry_count,
            timeout_at: self.timeout_at,
            last_error: self.last_error,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
