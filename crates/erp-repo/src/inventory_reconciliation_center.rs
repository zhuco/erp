use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

pub const INVENTORY_RECONCILIATION_STATUS_MATCH: &str = "MATCH";
pub const INVENTORY_RECONCILIATION_STATUS_MISMATCH: &str = "MISMATCH";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryReconciliationRecord {
    pub reconciliation_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub expected_on_hand: i64,
    pub actual_on_hand: i64,
    pub diff_qty: i64,
    pub status: String,
    pub source_ref: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InventoryReconciliationCheckInput {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub expected_on_hand: i64,
    pub source_ref: Option<String>,
}

pub struct InventoryReconciliationCenterRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InventoryReconciliationCenterRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn check_and_record(
        &self,
        input: InventoryReconciliationCheckInput,
    ) -> anyhow::Result<InventoryReconciliationRecord> {
        let actual_on_hand: i64 = sqlx::query_scalar(
            r#"
            select coalesce((
                select on_hand
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
            ), 0)
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(input.sku_id)
        .fetch_one(self.pool)
        .await?;

        let diff_qty = actual_on_hand - input.expected_on_hand;
        let status = if diff_qty == 0 {
            INVENTORY_RECONCILIATION_STATUS_MATCH
        } else {
            INVENTORY_RECONCILIATION_STATUS_MISMATCH
        };

        #[derive(sqlx::FromRow)]
        struct Row {
            reconciliation_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            expected_on_hand: i64,
            actual_on_hand: i64,
            diff_qty: i64,
            status: String,
            source_ref: Option<String>,
            created_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into inventory_reconciliation_center(
                tenant_id,
                warehouse_id,
                sku_id,
                expected_on_hand,
                actual_on_hand,
                diff_qty,
                status,
                source_ref
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                reconciliation_id,
                warehouse_id,
                sku_id,
                expected_on_hand,
                actual_on_hand,
                diff_qty,
                status,
                source_ref,
                created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(input.sku_id)
        .bind(input.expected_on_hand)
        .bind(actual_on_hand)
        .bind(diff_qty)
        .bind(status)
        .bind(input.source_ref)
        .fetch_one(self.pool)
        .await?;

        Ok(InventoryReconciliationRecord {
            reconciliation_id: row.reconciliation_id,
            tenant_id: input.tenant_id,
            warehouse_id: row.warehouse_id,
            sku_id: row.sku_id,
            expected_on_hand: row.expected_on_hand,
            actual_on_hand: row.actual_on_hand,
            diff_qty: row.diff_qty,
            status: row.status,
            source_ref: row.source_ref,
            created_at: row.created_at,
        })
    }
}
