use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{OrderId, TenantId, UserId};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

fn ensure_non_blank(field: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{field} must not be blank"));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct OrderStatusHistory {
    pub history_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub from_status: Option<String>,
    pub to_status: String,
    pub action_code: Option<String>,
    pub operator_id: Option<UserId>,
    pub reason: Option<String>,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OrderStatusHistoryCreate {
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub from_status: Option<String>,
    pub to_status: String,
    pub action_code: Option<String>,
    pub operator_id: Option<UserId>,
    pub reason: Option<String>,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct OrderStatusHistoryCursor {
    pub created_at: DateTime<Utc>,
    pub history_id: uuid::Uuid,
}

impl OrderStatusHistoryCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.history_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw.split_once('|').ok_or_else(|| anyhow!("游标无效"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("游标无效"))?
            .with_timezone(&Utc);
        let history_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("游标无效"))?;
        Ok(Self {
            created_at,
            history_id,
        })
    }
}

pub struct OrderStatusHistoryListResult {
    pub items: Vec<OrderStatusHistory>,
    pub next_cursor: Option<OrderStatusHistoryCursor>,
}

pub struct OrderStatusHistoryRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> OrderStatusHistoryRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        input: OrderStatusHistoryCreate,
    ) -> anyhow::Result<OrderStatusHistory> {
        ensure_non_blank("to_status", &input.to_status)?;
        if let Some(action_code) = input.action_code.as_deref() {
            ensure_non_blank("action_code", action_code)?;
        }
        let payload = input.payload.unwrap_or_else(|| serde_json::json!({}));
        let row: OrderStatusHistoryRow = sqlx::query_as(
            r#"
            insert into order_status_history(
                tenant_id,
                order_id,
                from_status,
                to_status,
                action_code,
                operator_id,
                reason,
                payload
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                history_id,
                tenant_id,
                order_id,
                from_status,
                to_status,
                action_code,
                operator_id,
                reason,
                payload,
                created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.order_id))
        .bind(&input.from_status)
        .bind(&input.to_status)
        .bind(&input.action_code)
        .bind(input.operator_id.map(uuid::Uuid::from))
        .bind(&input.reason)
        .bind(payload)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_history())
    }

    pub async fn list_by_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        cursor: Option<OrderStatusHistoryCursor>,
        limit: i64,
    ) -> anyhow::Result<OrderStatusHistoryListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                history_id,
                tenant_id,
                order_id,
                from_status,
                to_status,
                action_code,
                operator_id,
                reason,
                payload,
                created_at
            from order_status_history
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and order_id = ");
        qb.push_bind(uuid::Uuid::from(order_id));

        if let Some(cursor) = cursor {
            qb.push(" and (created_at, history_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.history_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, history_id desc");
        qb.push(" limit ");
        qb.push_bind(limit);

        let rows: Vec<OrderStatusHistoryRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| OrderStatusHistoryCursor {
            created_at: row.created_at,
            history_id: row.history_id,
        });
        let items = rows
            .into_iter()
            .map(OrderStatusHistoryRow::into_history)
            .collect();

        Ok(OrderStatusHistoryListResult { items, next_cursor })
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_non_blank;

    #[test]
    fn ensure_non_blank_rejects_blank_values() {
        assert!(ensure_non_blank("to_status", " ").is_err());
        assert!(ensure_non_blank("to_status", "\n").is_err());
        assert!(ensure_non_blank("to_status", "PENDING_ALLOCATE").is_ok());
    }
}

#[derive(sqlx::FromRow)]
struct OrderStatusHistoryRow {
    history_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    order_id: uuid::Uuid,
    from_status: Option<String>,
    to_status: String,
    action_code: Option<String>,
    operator_id: Option<uuid::Uuid>,
    reason: Option<String>,
    payload: Value,
    created_at: DateTime<Utc>,
}

impl OrderStatusHistoryRow {
    fn into_history(self) -> OrderStatusHistory {
        OrderStatusHistory {
            history_id: self.history_id,
            tenant_id: TenantId(self.tenant_id),
            order_id: OrderId(self.order_id),
            from_status: self.from_status,
            to_status: self.to_status,
            action_code: self.action_code,
            operator_id: self.operator_id.map(UserId),
            reason: self.reason,
            payload: self.payload,
            created_at: self.created_at,
        }
    }
}
