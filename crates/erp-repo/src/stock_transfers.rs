use anyhow::{anyhow, bail};
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use crate::inventory::{InventoryDelta, InventoryError, InventoryService};

const TRANSFER_INBOUND_DOC_TYPE: &str = "TRANSFER";
const TRANSFER_INBOUND_REF_TYPE: &str = "STOCK_TRANSFER";
const TRANSFER_INBOUND_STATUS: &str = "PUTAWAY_DONE";

#[derive(Debug, Clone)]
pub struct StockTransfer {
    pub transfer_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub from_warehouse_id: uuid::Uuid,
    pub to_warehouse_id: uuid::Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockTransferItem {
    pub transfer_item_id: uuid::Uuid,
    pub transfer_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockTransferDetail {
    pub transfer: StockTransfer,
    pub items: Vec<StockTransferItem>,
}

#[derive(Debug, Clone)]
pub struct StockTransferItemInput {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
}

#[derive(Debug, Clone)]
pub struct StockTransferCreateInput {
    pub tenant_id: TenantId,
    pub from_warehouse_id: uuid::Uuid,
    pub to_warehouse_id: uuid::Uuid,
    pub items: Vec<StockTransferItemInput>,
}

#[derive(Debug, Clone)]
pub struct StockTransferListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct StockTransferListResult {
    pub items: Vec<StockTransfer>,
    pub total: i64,
}

pub struct StockTransfersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> StockTransfersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_transfer(
        &self,
        input: StockTransferCreateInput,
    ) -> anyhow::Result<StockTransfer> {
        if input.items.is_empty() {
            bail!("transfer items required");
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockTransferRow = sqlx::query_as(
            r#"
            insert into stock_transfers(
                tenant_id, from_warehouse_id, to_warehouse_id, status
            ) values ($1,$2,$3,'DRAFT')
            returning transfer_id, tenant_id, from_warehouse_id, to_warehouse_id,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.from_warehouse_id)
        .bind(input.to_warehouse_id)
        .fetch_one(&mut *tx)
        .await?;

        for item in input.items {
            if item.qty <= 0 {
                bail!("qty must be positive");
            }
            sqlx::query(
                r#"
                insert into stock_transfer_items(
                    tenant_id, transfer_id, sku_id, qty
                ) values ($1,$2,$3,$4)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(row.transfer_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(row.into_transfer())
    }

    pub async fn list_transfers(
        &self,
        tenant_id: TenantId,
        filters: StockTransferListFilters,
    ) -> anyhow::Result<StockTransferListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from stock_transfers
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_transfer_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                transfer_id, tenant_id, from_warehouse_id, to_warehouse_id,
                status, created_at, updated_at
            from stock_transfers
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_transfer_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, transfer_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<StockTransferRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(StockTransferRow::into_transfer)
            .collect();

        Ok(StockTransferListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        transfer_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockTransferDetail>> {
        let row: Option<StockTransferRow> = sqlx::query_as(
            r#"
            select
                transfer_id, tenant_id, from_warehouse_id, to_warehouse_id,
                status, created_at, updated_at
            from stock_transfers
            where tenant_id = $1 and transfer_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(transfer_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let items = self.list_items(tenant_id, transfer_id).await?;
        Ok(Some(StockTransferDetail {
            transfer: row.into_transfer(),
            items,
        }))
    }

    pub async fn list_items(
        &self,
        tenant_id: TenantId,
        transfer_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<StockTransferItem>> {
        let rows: Vec<StockTransferItemRow> = sqlx::query_as(
            r#"
            select transfer_item_id, transfer_id, sku_id, qty, created_at, updated_at
            from stock_transfer_items
            where tenant_id = $1 and transfer_id = $2
            order by transfer_item_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(transfer_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(StockTransferItemRow::into_item)
            .collect())
    }

    pub async fn confirm_outbound(
        &self,
        tenant_id: TenantId,
        transfer_id: uuid::Uuid,
    ) -> anyhow::Result<StockTransfer> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockTransferRow = sqlx::query_as(
            r#"
            select
                transfer_id, tenant_id, from_warehouse_id, to_warehouse_id,
                status, created_at, updated_at
            from stock_transfers
            where tenant_id = $1 and transfer_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(transfer_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(anyhow!("transfer not found"))?;

        if row.status == "CANCELLED" {
            return Err(anyhow!("transfer cancelled"));
        }
        if row.status == "OUT_CONFIRMED" || row.status == "IN_CONFIRMED" {
            return Ok(row.into_transfer());
        }

        let items = self.list_items(tenant_id, transfer_id).await?;
        if items.is_empty() {
            return Err(anyhow!("transfer items missing"));
        }

        let occurred_at = Utc::now();
        let service = InventoryService::new(self.pool);
        for item in &items {
            let balance: BalanceRow = sqlx::query_as(
                r#"
                select on_hand, reserved, frozen, in_transit
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.from_warehouse_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(InventoryError::InsufficientAvailable {
                available: 0,
                requested: item.qty,
            })?;

            let available = balance.on_hand - balance.reserved - balance.frozen;
            if available < item.qty {
                return Err(InventoryError::InsufficientAvailable {
                    available,
                    requested: item.qty,
                }
                .into());
            }

            service
                .apply_with_tx(
                    &mut tx,
                    InventoryDelta {
                        tenant_id,
                        warehouse_id: row.from_warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: -item.qty,
                        delta_reserved: 0,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: "TRANSFER_OUT".to_string(),
                        ref_type: Some("STOCK_TRANSFER".to_string()),
                        ref_id: Some(transfer_id.to_string()),
                        occurred_at,
                    },
                )
                .await?;

            service
                .apply_with_tx(
                    &mut tx,
                    InventoryDelta {
                        tenant_id,
                        warehouse_id: row.to_warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: 0,
                        delta_reserved: 0,
                        delta_in_transit: item.qty,
                        delta_frozen: 0,
                        reason_type: "TRANSFER_IN_TRANSIT".to_string(),
                        ref_type: Some("STOCK_TRANSFER".to_string()),
                        ref_id: Some(transfer_id.to_string()),
                        occurred_at,
                    },
                )
                .await?;
        }

        let updated: StockTransferRow = sqlx::query_as(
            r#"
            update stock_transfers
            set status = 'OUT_CONFIRMED',
                updated_at = now()
            where tenant_id = $1 and transfer_id = $2
            returning transfer_id, tenant_id, from_warehouse_id, to_warehouse_id,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(transfer_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated.into_transfer())
    }

    pub async fn confirm_inbound(
        &self,
        tenant_id: TenantId,
        transfer_id: uuid::Uuid,
    ) -> anyhow::Result<StockTransfer> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let row: StockTransferRow = sqlx::query_as(
            r#"
            select
                transfer_id, tenant_id, from_warehouse_id, to_warehouse_id,
                status, created_at, updated_at
            from stock_transfers
            where tenant_id = $1 and transfer_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(transfer_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(anyhow!("transfer not found"))?;

        if row.status == "CANCELLED" {
            return Err(anyhow!("transfer cancelled"));
        }
        if row.status == "IN_CONFIRMED" {
            return Ok(row.into_transfer());
        }
        if row.status != "OUT_CONFIRMED" {
            return Err(anyhow!("transfer not ready"));
        }

        let items = self.list_items(tenant_id, transfer_id).await?;
        if items.is_empty() {
            return Err(anyhow!("transfer items missing"));
        }

        let occurred_at = Utc::now();
        let service = InventoryService::new(self.pool);
        for item in &items {
            let balance: BalanceRow = sqlx::query_as(
                r#"
                select on_hand, reserved, frozen, in_transit
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.to_warehouse_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(anyhow!("missing in-transit balance"))?;

            if balance.in_transit < item.qty {
                return Err(anyhow!("in_transit insufficient"));
            }

            service
                .apply_with_tx(
                    &mut tx,
                    InventoryDelta {
                        tenant_id,
                        warehouse_id: row.to_warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: item.qty,
                        delta_reserved: 0,
                        delta_in_transit: -item.qty,
                        delta_frozen: 0,
                        reason_type: "TRANSFER_IN".to_string(),
                        ref_type: Some("STOCK_TRANSFER".to_string()),
                        ref_id: Some(transfer_id.to_string()),
                        occurred_at,
                    },
                )
                .await?;
        }

        let updated: StockTransferRow = sqlx::query_as(
            r#"
            update stock_transfers
            set status = 'IN_CONFIRMED',
                updated_at = now()
            where tenant_id = $1 and transfer_id = $2
            returning transfer_id, tenant_id, from_warehouse_id, to_warehouse_id,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(transfer_id)
        .fetch_one(&mut *tx)
        .await?;

        let inbound_doc_no = format!("TR-{}", transfer_id);
        let inbound_doc_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            insert into inbound_docs(
                tenant_id, warehouse_id, doc_type, doc_no, status, ref_type, ref_id
            ) values ($1,$2,$3,$4,$5,$6,$7)
            on conflict (tenant_id, doc_no)
            do update set
                warehouse_id = excluded.warehouse_id,
                doc_type = excluded.doc_type,
                status = excluded.status,
                ref_type = excluded.ref_type,
                ref_id = excluded.ref_id,
                updated_at = now()
            returning doc_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(row.to_warehouse_id)
        .bind(TRANSFER_INBOUND_DOC_TYPE)
        .bind(&inbound_doc_no)
        .bind(TRANSFER_INBOUND_STATUS)
        .bind(TRANSFER_INBOUND_REF_TYPE)
        .bind(transfer_id.to_string())
        .fetch_one(&mut *tx)
        .await?;

        for item in &items {
            sqlx::query(
                r#"
                insert into inbound_doc_items(
                    tenant_id, doc_id, sku_id, planned_qty, received_qty, putaway_qty
                ) values ($1,$2,$3,$4,$5,$6)
                on conflict (tenant_id, doc_id, sku_id)
                do update set
                    planned_qty = excluded.planned_qty,
                    received_qty = excluded.received_qty,
                    putaway_qty = excluded.putaway_qty,
                    updated_at = now()
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(inbound_doc_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .bind(item.qty)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(updated.into_transfer())
    }

    pub async fn cancel_transfer(
        &self,
        tenant_id: TenantId,
        transfer_id: uuid::Uuid,
    ) -> anyhow::Result<StockTransfer> {
        let row: StockTransferRow = sqlx::query_as(
            r#"
            update stock_transfers
            set status = 'CANCELLED',
                updated_at = now()
            where tenant_id = $1 and transfer_id = $2
            returning transfer_id, tenant_id, from_warehouse_id, to_warehouse_id,
                      status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(transfer_id)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_transfer())
    }
}

fn apply_transfer_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a StockTransferListFilters,
) {
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and (from_warehouse_id = ");
        qb.push_bind(warehouse_id);
        qb.push(" or to_warehouse_id = ");
        qb.push_bind(warehouse_id);
        qb.push(")");
    }
}

#[derive(sqlx::FromRow)]
struct StockTransferRow {
    transfer_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    from_warehouse_id: uuid::Uuid,
    to_warehouse_id: uuid::Uuid,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockTransferRow {
    fn into_transfer(self) -> StockTransfer {
        StockTransfer {
            transfer_id: self.transfer_id,
            tenant_id: TenantId(self.tenant_id),
            from_warehouse_id: self.from_warehouse_id,
            to_warehouse_id: self.to_warehouse_id,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StockTransferItemRow {
    transfer_item_id: uuid::Uuid,
    transfer_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockTransferItemRow {
    fn into_item(self) -> StockTransferItem {
        StockTransferItem {
            transfer_item_id: self.transfer_item_id,
            transfer_id: self.transfer_id,
            sku_id: self.sku_id,
            qty: self.qty,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct BalanceRow {
    on_hand: i64,
    reserved: i64,
    frozen: i64,
    in_transit: i64,
}
