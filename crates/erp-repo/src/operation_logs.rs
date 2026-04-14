use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

pub const OP_ACTION_SHIPMENT_REQUESTED: &str = "SHIPMENT_REQUESTED";
pub const OP_ACTION_SHIPMENT_CALLBACK: &str = "SHIPMENT_CALLBACK";
pub const OP_ACTION_SHIPMENT_CONFIRMED: &str = "SHIPMENT_CONFIRMED";
pub const OP_ACTION_PRODUCT_CREATED: &str = "PRODUCTS.CREATE";
pub const OP_ACTION_PRODUCT_UPDATED: &str = "PRODUCTS.UPDATE";
pub const OP_ACTION_PRODUCT_DELETED: &str = "PRODUCTS.DELETE";
pub const OP_ACTION_SKU_CREATED: &str = "SKUS.CREATE";
pub const OP_ACTION_SKU_UPDATED: &str = "SKUS.UPDATE";
pub const OP_ACTION_SKU_DELETED: &str = "SKUS.DELETE";
pub const OP_ACTION_SKU_COPIED: &str = "SKUS.COPY";
pub const OP_ACTION_SKU_COST_ADJUSTED: &str = "SKUS.COST_ADJUST";
pub const OP_ACTION_KIT_BOM_SAVED: &str = "KITS.BOM_SAVE";
pub const OP_ACTION_INVENTORY_ADJUSTED: &str = "INVENTORY_ADJUSTED";
pub const OP_ACTION_INVENTORY_INBOUND: &str = "INVENTORY_INBOUND";
pub const OP_ACTION_INVENTORY_OUTBOUND: &str = "INVENTORY_OUTBOUND";
pub const OP_ACTION_INVENTORY_RESERVED: &str = "INVENTORY_RESERVED";
pub const OP_ACTION_INVENTORY_RELEASED: &str = "INVENTORY_RELEASED";
pub const OP_ACTION_INVENTORY_SYNC_RETRY: &str = "INVENTORY_SYNC_RETRY";
pub const OP_ACTION_INVENTORY_SYNC_RUN: &str = "INVENTORY_SYNC_RUN";
pub const OP_ACTION_INVENTORY_FREEZE: &str = "INVENTORY_FREEZE";
pub const OP_ACTION_INVENTORY_UNFREEZE: &str = "INVENTORY_UNFREEZE";
pub const OP_ACTION_INVENTORY_FREEZE_REQUESTED: &str = "INVENTORY_FREEZE_REQUESTED";
pub const OP_ACTION_INVENTORY_UNFREEZE_REQUESTED: &str = "INVENTORY_UNFREEZE_REQUESTED";
pub const OP_ACTION_INVENTORY_FREEZE_APPROVED: &str = "INVENTORY_FREEZE_APPROVED";
pub const OP_ACTION_INVENTORY_UNFREEZE_APPROVED: &str = "INVENTORY_UNFREEZE_APPROVED";
pub const OP_ACTION_INVENTORY_FREEZE_REJECTED: &str = "INVENTORY_FREEZE_REJECTED";
pub const OP_ACTION_INVENTORY_UNFREEZE_REJECTED: &str = "INVENTORY_UNFREEZE_REJECTED";
pub const OP_ACTION_RETURN_STOCKED: &str = "RETURN_STOCKED";
pub const OP_ACTION_OMS_ALLOCATED: &str = "OMS.ALLOCATED";
pub const OP_ACTION_OMS_REVIEW_PASS: &str = "OMS.REVIEW_PASS";
pub const OP_ACTION_OMS_REVIEW_REJECT: &str = "OMS.REVIEW_REJECT";
pub const OP_ACTION_OMS_REVIEW_FLAG: &str = "OMS.REVIEW_FLAG";
pub const OP_ACTION_OMS_REVIEW_AUTO_PASS: &str = "OMS.REVIEW_AUTO_PASS";
pub const OP_ACTION_OMS_REVIEW_AUTO_REJECT: &str = "OMS.REVIEW_AUTO_REJECT";
pub const OP_ACTION_OMS_REVIEW_AUTO_FLAG: &str = "OMS.REVIEW_AUTO_FLAG";
pub const OP_ACTION_OMS_EXCEPTION_RESOLVE: &str = "OMS.EXCEPTION_RESOLVE";
pub const OP_ACTION_OMS_EXCEPTION_REJECT: &str = "OMS.EXCEPTION_REJECT";
pub const OP_ACTION_OMS_EXCEPTION_REVOKE: &str = "OMS.EXCEPTION_REVOKE";
pub const OP_ACTION_OMS_ORDER_CANCEL: &str = "OMS.ORDER_CANCEL";
pub const OP_ACTION_OMS_ORDER_MERGE_SOURCE: &str = "OMS.ORDER_MERGE_SOURCE";
pub const OP_ACTION_OMS_ORDER_MERGE_TARGET: &str = "OMS.ORDER_MERGE_TARGET";
pub const OP_ACTION_OMS_ORDER_SPLIT_SOURCE: &str = "OMS.ORDER_SPLIT_SOURCE";
pub const OP_ACTION_OMS_ORDER_SPLIT_CHILD: &str = "OMS.ORDER_SPLIT_CHILD";
pub const OP_ACTION_OMS_SHIPMENT_CREATED: &str = "OMS.SHIPMENT_CREATED";
pub const OP_ACTION_OMS_SHIPMENT_SHIPPED: &str = "OMS.SHIPMENT_SHIPPED";
pub const OP_ACTION_OMS_RESEND_CREATED: &str = "OMS.RESEND_CREATED";
pub const OP_ACTION_OMS_RESERVATION_RELEASED: &str = "OMS.RESERVATION_RELEASED";
pub const OP_ACTION_OMS_SHIPMENT_CANCEL_REQUESTED: &str = "OMS.SHIPMENT_CANCEL_REQUESTED";
pub const OP_ACTION_SHOP_PRODUCT_AUTO_MATCH: &str = "SHOP_PRODUCTS.AUTO_MATCH";
pub const OP_ACTION_SHOP_PRODUCT_MANUAL_MATCH: &str = "SHOP_PRODUCTS.MANUAL_MATCH";
pub const OP_ACTION_WMS_OUTBOUND_CREATED: &str = "WMS.OUTBOUND_CREATED";
pub const OP_ACTION_WMS_OUTBOUND_PACKED: &str = "WMS.PACKED";
pub const OP_ACTION_WMS_OUTBOUND_INSPECTED: &str = "WMS.INSPECTED";
pub const OP_ACTION_WMS_OUTBOUND_SHIPPED: &str = "WMS.SHIPPED";
pub const OP_ACTION_WMS_OUTBOUND_LOGISTICS_UPDATED: &str = "WMS.LOGISTICS_UPDATED";
pub const OP_ACTION_WMS_OUTBOUND_ADD_PACKAGE: &str = "WMS.ADD_PACKAGE";
pub const OP_ACTION_WMS_TRACKING_RESERVED: &str = "WMS.TRACKING_RESERVED";
pub const OP_ACTION_WMS_SHIPMENT_CANCEL_REQUESTED: &str = "WMS.SHIPMENT_CANCEL_REQUESTED";
pub const OP_ACTION_STOCK_COUNT_CREATED: &str = "STOCK_COUNT_CREATED";
pub const OP_ACTION_STOCK_COUNT_CONFIRMED: &str = "STOCK_COUNT_CONFIRMED";
pub const OP_ACTION_STOCK_COUNT_APPLIED: &str = "STOCK_COUNT_APPLIED";
pub const OP_ACTION_STOCK_COUNT_CANCELLED: &str = "STOCK_COUNT_CANCELLED";
pub const OP_ACTION_STOCK_TRANSFER_CREATED: &str = "STOCK_TRANSFER_CREATED";
pub const OP_ACTION_STOCK_TRANSFER_OUT: &str = "STOCK_TRANSFER_OUT";
pub const OP_ACTION_STOCK_TRANSFER_IN: &str = "STOCK_TRANSFER_IN";
pub const OP_ACTION_STOCK_ADJUST_CREATED: &str = "STOCK_ADJUST_CREATED";
pub const OP_ACTION_STOCK_ADJUST_APPLIED: &str = "STOCK_ADJUST_APPLIED";
pub const OP_ACTION_STOCK_MOVE_CREATED: &str = "STOCK_MOVE_CREATED";
pub const OP_ACTION_STOCK_MOVE_APPLIED: &str = "STOCK_MOVE_APPLIED";
pub const OP_ACTION_WMS_INBOUND_CREATED: &str = "WMS.INBOUND_CREATED";
pub const OP_ACTION_WMS_INBOUND_AUDITED: &str = "WMS.INBOUND_AUDITED";
pub const OP_ACTION_WMS_INBOUND_RECEIVED: &str = "WMS.INBOUND_RECEIVED";
pub const OP_ACTION_WMS_INBOUND_PUTAWAY: &str = "WMS.INBOUND_PUTAWAY";
pub const OP_ACTION_WMS_INBOUND_INSPECTED: &str = "WMS.INBOUND_INSPECTED";
pub const OP_ACTION_WMS_OUTBOUND_AUDITED: &str = "WMS.OUTBOUND_AUDITED";
pub const OP_ACTION_WMS_OUTBOUND_CHECKED: &str = "WMS.OUTBOUND_CHECKED";
pub const OP_ACTION_WMS_OUTBOUND_WEIGHED: &str = "WMS.OUTBOUND_WEIGHED";
pub const OP_ACTION_AP_DOC_PAID: &str = "FINANCE.AP_DOC_PAID";
pub const OP_ACTION_AP_DOC_CLOSED: &str = "FINANCE.AP_DOC_CLOSED";
pub const OP_ACTION_SYSTEM_FRONTEND_ROUTE_DIAGNOSTIC: &str = "SYSTEM.FRONTEND_ROUTE_DIAGNOSTIC";

fn ensure_non_blank(field: &str, value: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{field} must not be blank"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    pub log_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub actor_type: String,
    pub actor_user_id: Option<UserId>,
    pub action: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct OperationLogCreate {
    pub tenant_id: TenantId,
    pub actor_type: String,
    pub actor_user_id: Option<UserId>,
    pub action: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct OperationLogCursor {
    pub created_at: DateTime<Utc>,
    pub log_id: uuid::Uuid,
}

impl OperationLogCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.log_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let log_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self { created_at, log_id })
    }
}

pub struct OperationLogListFilters {
    pub action: Option<String>,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub actor_user_id: Option<UserId>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub cursor: Option<OperationLogCursor>,
    pub limit: i64,
}

pub struct OperationLogListResult {
    pub items: Vec<OperationLog>,
    pub next_cursor: Option<OperationLogCursor>,
}

pub struct OperationLogsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> OperationLogsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, input: OperationLogCreate) -> anyhow::Result<OperationLog> {
        Self::insert_with_tx(self.pool, input).await
    }

    pub async fn insert_with_tx<'t, T>(
        executor: T,
        input: OperationLogCreate,
    ) -> anyhow::Result<OperationLog>
    where
        T: sqlx::Executor<'t, Database = Postgres>,
    {
        ensure_non_blank("actor_type", &input.actor_type)?;
        ensure_non_blank("action", &input.action)?;
        let metadata = input.metadata.unwrap_or_else(|| serde_json::json!({}));
        let row: OperationLogRow = sqlx::query_as(
            r#"
            insert into operation_logs(
                tenant_id, actor_type, actor_user_id, action, ref_type, ref_id, metadata
            ) values ($1,$2,$3,$4,$5,$6,$7)
            returning
                log_id, tenant_id, actor_type, actor_user_id,
                action, ref_type, ref_id, metadata, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.actor_type)
        .bind(input.actor_user_id.map(uuid::Uuid::from))
        .bind(&input.action)
        .bind(&input.ref_type)
        .bind(&input.ref_id)
        .bind(metadata)
        .fetch_one(executor)
        .await?;

        Ok(row.into_log())
    }

    pub async fn list_logs(
        &self,
        tenant_id: TenantId,
        filters: OperationLogListFilters,
    ) -> anyhow::Result<OperationLogListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                log_id, tenant_id, actor_type, actor_user_id,
                action, ref_type, ref_id, metadata, created_at
            from operation_logs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(action) = filters.action {
            qb.push(" and action = ");
            qb.push_bind(action);
        }
        if let Some(ref_type) = filters.ref_type {
            qb.push(" and ref_type = ");
            qb.push_bind(ref_type);
        }
        if let Some(ref_id) = filters.ref_id {
            qb.push(" and ref_id = ");
            qb.push_bind(ref_id);
        }
        if let Some(actor_user_id) = filters.actor_user_id {
            qb.push(" and actor_user_id = ");
            qb.push_bind(uuid::Uuid::from(actor_user_id));
        }
        if let Some(created_from) = filters.created_from {
            qb.push(" and created_at >= ");
            qb.push_bind(created_from);
        }
        if let Some(created_to) = filters.created_to {
            qb.push(" and created_at <= ");
            qb.push_bind(created_to);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, log_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.log_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, log_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<OperationLogRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| OperationLogCursor {
            created_at: row.created_at,
            log_id: row.log_id,
        });
        let items = rows.into_iter().map(OperationLogRow::into_log).collect();

        Ok(OperationLogListResult { items, next_cursor })
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_non_blank;

    #[test]
    fn ensure_non_blank_rejects_blank_values() {
        assert!(ensure_non_blank("actor_type", " ").is_err());
        assert!(ensure_non_blank("actor_type", "\r\n").is_err());
        assert!(ensure_non_blank("actor_type", "USER").is_ok());
    }
}

#[derive(sqlx::FromRow)]
struct OperationLogRow {
    log_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    actor_type: String,
    actor_user_id: Option<uuid::Uuid>,
    action: String,
    ref_type: Option<String>,
    ref_id: Option<String>,
    metadata: Value,
    created_at: DateTime<Utc>,
}

impl OperationLogRow {
    fn into_log(self) -> OperationLog {
        OperationLog {
            log_id: self.log_id,
            tenant_id: TenantId(self.tenant_id),
            actor_type: self.actor_type,
            actor_user_id: self.actor_user_id.map(UserId),
            action: self.action,
            ref_type: self.ref_type,
            ref_id: self.ref_id,
            metadata: self.metadata,
            created_at: self.created_at,
        }
    }
}
