use chrono::{DateTime, Utc};
use erp_types::{OrderId, TenantId, UserId};
use serde_json::Value;
use sqlx::{PgPool, Postgres};

#[derive(Debug, Clone)]
pub struct OrderActionLog {
    pub action_log_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub action_code: String,
    pub idempotency_key: String,
    pub status: String,
    pub operator_id: Option<UserId>,
    pub request_json: Value,
    pub response_json: Value,
    pub error_message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrderActionLogCreate {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub action_code: String,
    pub idempotency_key: String,
    pub status: Option<String>,
    pub operator_id: Option<UserId>,
    pub request_json: Option<Value>,
    pub response_json: Option<Value>,
    pub error_message: Option<String>,
}

pub struct OrderActionLogsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> OrderActionLogsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, input: OrderActionLogCreate) -> anyhow::Result<OrderActionLog> {
        Self::insert_with_tx(self.pool, input).await
    }

    pub async fn insert_with_tx<'t, T>(
        executor: T,
        input: OrderActionLogCreate,
    ) -> anyhow::Result<OrderActionLog>
    where
        T: sqlx::Executor<'t, Database = Postgres>,
    {
        let status = input.status.unwrap_or_else(|| "SUCCESS".to_string());
        let request_json = input.request_json.unwrap_or_else(|| serde_json::json!({}));
        let response_json = input.response_json.unwrap_or_else(|| serde_json::json!({}));
        let error_message = input.error_message.unwrap_or_default();
        let row: OrderActionLogRow = sqlx::query_as(
            r#"
            insert into order_action_log(
                tenant_id,
                order_id,
                action_code,
                idempotency_key,
                status,
                operator_id,
                request_json,
                response_json,
                error_message
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            returning
                action_log_id,
                tenant_id,
                order_id,
                action_code,
                idempotency_key,
                status,
                operator_id,
                request_json,
                response_json,
                error_message,
                created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.order_id))
        .bind(&input.action_code)
        .bind(&input.idempotency_key)
        .bind(&status)
        .bind(input.operator_id.map(uuid::Uuid::from))
        .bind(request_json)
        .bind(response_json)
        .bind(&error_message)
        .fetch_one(executor)
        .await?;

        Ok(row.into_log())
    }

    pub async fn find_by_idempotency(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        action_code: &str,
        idempotency_key: &str,
    ) -> anyhow::Result<Option<OrderActionLog>> {
        let row: Option<OrderActionLogRow> = sqlx::query_as(
            r#"
            select
                action_log_id,
                tenant_id,
                order_id,
                action_code,
                idempotency_key,
                status,
                operator_id,
                request_json,
                response_json,
                error_message,
                created_at
            from order_action_log
            where tenant_id = $1
              and order_id = $2
              and action_code = $3
              and idempotency_key = $4
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(action_code)
        .bind(idempotency_key)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(OrderActionLogRow::into_log))
    }
}

#[derive(sqlx::FromRow)]
struct OrderActionLogRow {
    action_log_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    order_id: uuid::Uuid,
    action_code: String,
    idempotency_key: String,
    status: String,
    operator_id: Option<uuid::Uuid>,
    request_json: Value,
    response_json: Value,
    error_message: String,
    created_at: DateTime<Utc>,
}

impl OrderActionLogRow {
    fn into_log(self) -> OrderActionLog {
        OrderActionLog {
            action_log_id: self.action_log_id,
            tenant_id: TenantId(self.tenant_id),
            order_id: OrderId(self.order_id),
            action_code: self.action_code,
            idempotency_key: self.idempotency_key,
            status: self.status,
            operator_id: self.operator_id.map(UserId),
            request_json: self.request_json,
            response_json: self.response_json,
            error_message: self.error_message,
            created_at: self.created_at,
        }
    }
}
