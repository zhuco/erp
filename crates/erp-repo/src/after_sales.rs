use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{
    AfterSalesStatus, AfterSalesType, OrderId, RefundChannel, RefundStatus, ReturnReceiptStatus,
    TenantId,
};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::str::FromStr;

use crate::inventory::{InventoryDelta, InventoryService};

pub const REFUND_REQUESTED_EVENT: &str = "REFUND_REQUESTED";

const RETURN_INBOUND_DOC_TYPE: &str = "RETURN";
const RETURN_REF_TYPE: &str = "AFTER_SALES";
const RETURN_LEDGER_KIND: &str = "RETURN_IN";
const RETURN_LEDGER_CANCEL_KIND: &str = "RETURN_IN_CANCEL";

#[derive(Debug, Clone)]
pub struct AfterSales {
    pub id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub after_sales_type: AfterSalesType,
    pub status: AfterSalesStatus,
    pub after_sale_status: String,
    pub compensation_status: Option<String>,
    pub evidence_status: Option<String>,
    pub reason_code: Option<String>,
    pub amount_cent: i64,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AfterSalesItem {
    pub after_sales_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub refund_cent: i64,
}

#[derive(Debug, Clone)]
pub struct ReturnReceipt {
    pub id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub status: ReturnReceiptStatus,
    pub return_status: String,
    pub received_at: Option<DateTime<Utc>>,
    pub putaway_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AfterSalesShipment {
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub shipment_id: uuid::Uuid,
    pub shipment_type: String,
    pub origin_shipment_id: Option<uuid::Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RefundDoc {
    pub id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub amount_cent: i64,
    pub status: RefundStatus,
    pub refund_status: String,
    pub channel: RefundChannel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ExchangeDoc {
    pub exchange_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub exchange_status: String,
    pub outbound_doc_id: Option<uuid::Uuid>,
    pub shipment_id: Option<uuid::Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AfterSalesDetail {
    pub after_sales: AfterSales,
    pub items: Vec<AfterSalesItem>,
    pub return_receipt: Option<ReturnReceipt>,
    pub refund_doc: Option<RefundDoc>,
    pub shipments: Vec<AfterSalesShipment>,
}

pub struct AfterSalesListFilters {
    pub status: Option<AfterSalesStatus>,
    pub after_sales_type: Option<AfterSalesType>,
    pub order_id: Option<OrderId>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct AfterSalesListResult {
    pub items: Vec<AfterSales>,
    pub total: i64,
}

pub struct RefundDocListFilters {
    pub status: Option<RefundStatus>,
    pub after_sales_id: Option<uuid::Uuid>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct RefundDocListResult {
    pub items: Vec<RefundDoc>,
    pub total: i64,
}

pub struct AfterSalesCreateInput {
    pub after_sales_id: Option<uuid::Uuid>,
    pub tenant_id: TenantId,
    pub order_id: OrderId,
    pub after_sales_type: AfterSalesType,
    pub reason_code: Option<String>,
    pub amount_cent: i64,
    pub raw_json: Option<Value>,
    pub compensation_status: Option<String>,
    pub evidence_status: Option<String>,
    pub items: Vec<AfterSalesItemCreate>,
}

pub struct AfterSalesItemCreate {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub refund_cent: i64,
}

pub struct ReturnReceiptCreateInput {
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub putaway_done: bool,
    pub received_at: Option<DateTime<Utc>>,
    pub link_inbound: bool,
}

pub struct AfterSalesShipmentCreate {
    pub shipment_id: uuid::Uuid,
    pub shipment_type: String,
    pub origin_shipment_id: Option<uuid::Uuid>,
}

pub struct RefundDocCreateInput {
    pub tenant_id: TenantId,
    pub after_sales_id: uuid::Uuid,
    pub amount_cent: i64,
    pub channel: RefundChannel,
}

pub struct ExchangeDocCreateInput {
    pub outbound_doc_id: Option<uuid::Uuid>,
    pub shipment_id: Option<uuid::Uuid>,
}

pub struct AfterSalesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> AfterSalesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_after_sales(
        &self,
        input: AfterSalesCreateInput,
    ) -> anyhow::Result<AfterSalesDetail> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let after_sales_id = input.after_sales_id.unwrap_or_else(uuid::Uuid::new_v4);

        let inserted = sqlx::query(
            r#"
            insert into after_sales(
                id, tenant_id, order_id, type, status, after_sale_status, reason_code, amount_cent, raw_json,
                compensation_status, evidence_status
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            on conflict (id) do nothing
            "#,
        )
        .bind(after_sales_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.order_id))
        .bind(input.after_sales_type.to_string())
        .bind(AfterSalesStatus::Open.to_string())
        .bind(map_after_sale_status(AfterSalesStatus::Open))
        .bind(&input.reason_code)
        .bind(input.amount_cent)
        .bind(input.raw_json.unwrap_or_else(|| serde_json::json!({})))
        .bind(&input.compensation_status)
        .bind(&input.evidence_status)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;

        if inserted {
            for item in &input.items {
                sqlx::query(
                    r#"
                    insert into after_sales_items(
                        tenant_id, after_sales_id, sku_id, qty, refund_cent
                    ) values ($1,$2,$3,$4,$5)
                    on conflict (tenant_id, after_sales_id, sku_id)
                    do update set
                        qty = excluded.qty,
                        refund_cent = excluded.refund_cent,
                        updated_at = now()
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(after_sales_id)
                .bind(item.sku_id)
                .bind(item.qty)
                .bind(item.refund_cent)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        let detail = self
            .get_detail(input.tenant_id, after_sales_id)
            .await?
            .ok_or_else(|| anyhow!("after_sales not found after insert"))?;
        Ok(detail)
    }

    pub async fn list_after_sales(
        &self,
        tenant_id: TenantId,
        filters: AfterSalesListFilters,
    ) -> anyhow::Result<AfterSalesListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                id, order_id, type as after_sales_type, status, after_sale_status,
                compensation_status, evidence_status, reason_code, amount_cent, raw_json,
                created_at, updated_at
            from after_sales
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_after_sales_filters(&mut qb, &filters);
        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<AfterSalesRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(row.into_after_sales(tenant_id)?);
        }

        let mut count_qb =
            QueryBuilder::<Postgres>::new("select count(*) from after_sales where tenant_id = ");
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_after_sales_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(AfterSalesListResult { items, total })
    }

    pub async fn get_after_sales(
        &self,
        tenant_id: TenantId,
        after_sales_id: uuid::Uuid,
    ) -> anyhow::Result<Option<AfterSales>> {
        let row = sqlx::query_as::<_, AfterSalesRow>(
            r#"
            select
                id, order_id, type as after_sales_type, status, after_sale_status,
                compensation_status, evidence_status, reason_code, amount_cent, raw_json,
                created_at, updated_at
            from after_sales
            where tenant_id = $1 and id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(match row {
            Some(row) => Some(row.into_after_sales(tenant_id)?),
            None => None,
        })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        after_sales_id: uuid::Uuid,
    ) -> anyhow::Result<Option<AfterSalesDetail>> {
        let Some(after_sales) = self.get_after_sales(tenant_id, after_sales_id).await? else {
            return Ok(None);
        };

        let item_rows: Vec<AfterSalesItemRow> = sqlx::query_as(
            r#"
            select sku_id, qty, refund_cent
            from after_sales_items
            where tenant_id = $1 and after_sales_id = $2
            order by sku_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_all(self.pool)
        .await?;

        let items = item_rows
            .into_iter()
            .map(|row| AfterSalesItem {
                tenant_id,
                after_sales_id,
                sku_id: row.sku_id,
                qty: row.qty,
                refund_cent: row.refund_cent,
            })
            .collect();

        let receipt_row = sqlx::query_as::<_, ReturnReceiptRow>(
            r#"
            select
                id, warehouse_id, status, return_status,
                received_at, putaway_at, created_at, updated_at
            from return_receipts
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_optional(self.pool)
        .await?;

        let return_receipt = match receipt_row {
            Some(row) => Some(row.into_receipt(tenant_id, after_sales_id)?),
            None => None,
        };

        let refund_row = sqlx::query_as::<_, RefundDocRow>(
            r#"
            select
                id, after_sales_id, amount_cent, status, refund_status, channel,
                created_at, updated_at
            from refund_docs
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_optional(self.pool)
        .await?;

        let refund_doc = match refund_row {
            Some(row) => Some(row.into_refund_doc(tenant_id)?),
            None => None,
        };

        let shipment_rows: Vec<AfterSalesShipmentRow> = sqlx::query_as(
            r#"
            select shipment_id, shipment_type, origin_shipment_id, created_at
            from after_sales_shipments
            where tenant_id = $1 and after_sales_id = $2
            order by created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_all(self.pool)
        .await?;
        let shipments = shipment_rows
            .into_iter()
            .map(|row| AfterSalesShipment {
                tenant_id,
                after_sales_id,
                shipment_id: row.shipment_id,
                shipment_type: row.shipment_type,
                origin_shipment_id: row.origin_shipment_id,
                created_at: row.created_at,
            })
            .collect();

        Ok(Some(AfterSalesDetail {
            after_sales,
            items,
            return_receipt,
            refund_doc,
            shipments,
        }))
    }

    pub async fn set_status(
        &self,
        tenant_id: TenantId,
        after_sales_id: uuid::Uuid,
        status: AfterSalesStatus,
        allowed: &[AfterSalesStatus],
    ) -> anyhow::Result<AfterSales> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let current_status: Option<String> = sqlx::query_scalar(
            r#"
            select status
            from after_sales
            where tenant_id = $1 and id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(current_status) = current_status else {
            return Err(anyhow!("after_sales not found"));
        };
        let current = AfterSalesStatus::from_str(&current_status).unwrap_or(AfterSalesStatus::Open);

        if current == status {
            tx.commit().await?;
            return self
                .get_after_sales(tenant_id, after_sales_id)
                .await?
                .ok_or_else(|| anyhow!("after_sales not found"));
        }

        if !allowed.contains(&current) {
            return Err(anyhow!("after_sales status not allowed"));
        }

        let row: AfterSalesRow = sqlx::query_as(
            r#"
            update after_sales
            set status = $3,
                after_sale_status = $4,
                updated_at = now()
            where tenant_id = $1 and id = $2
            returning
                id, order_id, type as after_sales_type, status, after_sale_status,
                compensation_status, evidence_status, reason_code, amount_cent, raw_json,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .bind(status.to_string())
        .bind(map_after_sale_status(status))
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        row.into_after_sales(tenant_id)
    }

    pub async fn receive_return(
        &self,
        input: ReturnReceiptCreateInput,
    ) -> anyhow::Result<AfterSalesDetail> {
        self.receive_return_impl(input, false).await
    }

    pub async fn redo_return_receipt(
        &self,
        input: ReturnReceiptCreateInput,
    ) -> anyhow::Result<AfterSalesDetail> {
        self.receive_return_impl(input, true).await
    }

    pub async fn cancel_return_receipt(
        &self,
        tenant_id: TenantId,
        after_sales_id: uuid::Uuid,
    ) -> anyhow::Result<AfterSalesDetail> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct StatusRow {
            status: String,
        }

        let status_row = sqlx::query_as::<_, StatusRow>(
            r#"
            select status
            from after_sales
            where tenant_id = $1 and id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(status_row) = status_row else {
            return Err(anyhow!("after_sales not found"));
        };
        let current =
            AfterSalesStatus::from_str(&status_row.status).unwrap_or(AfterSalesStatus::Open);
        if matches!(
            current,
            AfterSalesStatus::Refunded | AfterSalesStatus::Closed | AfterSalesStatus::Rejected
        ) {
            return Err(anyhow!("after_sales status not cancellable"));
        }

        let existing_receipt = sqlx::query_as::<_, ReturnReceiptRow>(
            r#"
            select
                id, warehouse_id, status, return_status,
                received_at, putaway_at, created_at, updated_at
            from return_receipts
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(receipt_row) = existing_receipt else {
            return Err(anyhow!("return receipt not found"));
        };
        let receipt = receipt_row.into_receipt(tenant_id, after_sales_id)?;
        if receipt.status == ReturnReceiptStatus::Cancelled {
            tx.commit().await?;
            return self
                .get_detail(tenant_id, after_sales_id)
                .await?
                .ok_or_else(|| anyhow!("after_sales not found"));
        }

        let items: Vec<AfterSalesItemRow> = sqlx::query_as(
            r#"
            select sku_id, qty, refund_cent
            from after_sales_items
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_all(&mut *tx)
        .await?;

        if items.is_empty() {
            return Err(anyhow!("after_sales items empty"));
        }

        if receipt.status == ReturnReceiptStatus::PutawayDone {
            for item in &items {
                if item.qty <= 0 {
                    continue;
                }
                let has_ledger: bool = sqlx::query_scalar(
                    r#"
                    select exists(
                        select 1
                        from inventory_ledgers
                        where tenant_id = $1
                          and ref_type = $2
                          and ref_id = $3
                          and sku_id = $4
                          and reason_type = $5
                    )
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(RETURN_REF_TYPE)
                .bind(after_sales_id.to_string())
                .bind(item.sku_id)
                .bind(RETURN_LEDGER_KIND)
                .fetch_one(&mut *tx)
                .await?;
                if !has_ledger {
                    continue;
                }
                apply_inventory_adjustment_with_reason(
                    self.pool,
                    &mut tx,
                    tenant_id,
                    receipt.warehouse_id,
                    item.sku_id,
                    -item.qty,
                    after_sales_id,
                    RETURN_LEDGER_CANCEL_KIND,
                )
                .await?;
            }
        }

        sqlx::query(
            r#"
            update return_receipts
            set status = $3,
                return_status = $4,
                updated_at = now()
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .bind(ReturnReceiptStatus::Cancelled.to_string())
        .bind(map_return_status(ReturnReceiptStatus::Cancelled))
        .execute(&mut *tx)
        .await?;

        let doc_no = format!("RETURN-{}", after_sales_id);
        sqlx::query(
            r#"
            update inbound_docs
            set status = 'CANCELLED',
                updated_at = now()
            where tenant_id = $1 and doc_no = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&doc_no)
        .execute(&mut *tx)
        .await?;

        if current == AfterSalesStatus::Received {
            sqlx::query(
                r#"
                update after_sales
                set status = $3,
                    after_sale_status = $4,
                    updated_at = now()
                where tenant_id = $1 and id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(after_sales_id)
            .bind(AfterSalesStatus::Approved.to_string())
            .bind(map_after_sale_status(AfterSalesStatus::Approved))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.get_detail(tenant_id, after_sales_id)
            .await?
            .ok_or_else(|| anyhow!("after_sales not found"))
    }

    pub async fn putaway_return_receipt(
        &self,
        tenant_id: TenantId,
        after_sales_id: uuid::Uuid,
    ) -> anyhow::Result<AfterSalesDetail> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct StatusRow {
            status: String,
        }

        let status_row = sqlx::query_as::<_, StatusRow>(
            r#"
            select status
            from after_sales
            where tenant_id = $1 and id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(status_row) = status_row else {
            return Err(anyhow!("after_sales not found"));
        };

        let current =
            AfterSalesStatus::from_str(&status_row.status).unwrap_or(AfterSalesStatus::Open);
        if !matches!(
            current,
            AfterSalesStatus::Approved | AfterSalesStatus::Received
        ) {
            return Err(anyhow!("after_sales status not receivable"));
        }

        let receipt_row = sqlx::query_as::<_, ReturnReceiptRow>(
            r#"
            select
                id, warehouse_id, status, return_status,
                received_at, putaway_at, created_at, updated_at
            from return_receipts
            where tenant_id = $1 and after_sales_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(receipt_row) = receipt_row else {
            return Err(anyhow!("return receipt not found"));
        };
        let receipt = receipt_row.into_receipt(tenant_id, after_sales_id)?;
        if receipt.status == ReturnReceiptStatus::Cancelled {
            return Err(anyhow!("return receipt cancelled"));
        }
        if receipt.status == ReturnReceiptStatus::PutawayDone {
            tx.commit().await?;
            return self
                .get_detail(tenant_id, after_sales_id)
                .await?
                .ok_or_else(|| anyhow!("after_sales not found"));
        }

        let items: Vec<AfterSalesItemRow> = sqlx::query_as(
            r#"
            select sku_id, qty, refund_cent
            from after_sales_items
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .fetch_all(&mut *tx)
        .await?;

        if items.is_empty() {
            return Err(anyhow!("after_sales items empty"));
        }

        for item in &items {
            if item.qty <= 0 {
                continue;
            }
            let has_ledger: bool = sqlx::query_scalar(
                r#"
                select exists(
                    select 1
                    from inventory_ledgers
                    where tenant_id = $1
                      and ref_type = $2
                      and ref_id = $3
                      and sku_id = $4
                      and reason_type = $5
                )
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(RETURN_REF_TYPE)
            .bind(after_sales_id.to_string())
            .bind(item.sku_id)
            .bind(RETURN_LEDGER_KIND)
            .fetch_one(&mut *tx)
            .await?;
            if has_ledger {
                continue;
            }
            apply_inventory_adjustment(
                self.pool,
                &mut tx,
                tenant_id,
                receipt.warehouse_id,
                item.sku_id,
                item.qty,
                after_sales_id,
            )
            .await?;
        }

        let now = Utc::now();
        sqlx::query(
            r#"
            update return_receipts
            set status = $3,
                return_status = $4,
                putaway_at = $5,
                updated_at = now()
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .bind(ReturnReceiptStatus::PutawayDone.to_string())
        .bind(map_return_status(ReturnReceiptStatus::PutawayDone))
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            update after_sales
            set status = $3,
                after_sale_status = $4,
                updated_at = now()
            where tenant_id = $1 and id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(after_sales_id)
        .bind(AfterSalesStatus::Received.to_string())
        .bind(map_after_sale_status(AfterSalesStatus::Received))
        .execute(&mut *tx)
        .await?;

        let doc_no = format!("RETURN-{}", after_sales_id);
        let doc_id: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select doc_id
            from inbound_docs
            where tenant_id = $1 and doc_no = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&doc_no)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(doc_id) = doc_id {
            sqlx::query(
                r#"
                update inbound_doc_items
                set received_qty = planned_qty,
                    putaway_qty = planned_qty,
                    updated_at = now()
                where tenant_id = $1 and doc_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(doc_id)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                update inbound_docs
                set status = $3,
                    inbound_status = 'COMPLETED',
                    receipt_status = 'RECEIVED',
                    putaway_status = 'PUTAWAYED',
                    updated_at = now()
                where tenant_id = $1 and doc_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(doc_id)
            .bind("PUTAWAY_DONE")
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.get_detail(tenant_id, after_sales_id)
            .await?
            .ok_or_else(|| anyhow!("after_sales not found"))
    }

    async fn receive_return_impl(
        &self,
        input: ReturnReceiptCreateInput,
        allow_cancelled: bool,
    ) -> anyhow::Result<AfterSalesDetail> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct StatusRow {
            status: String,
        }

        let status_row = sqlx::query_as::<_, StatusRow>(
            r#"
            select status
            from after_sales
            where tenant_id = $1 and id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.after_sales_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(status_row) = status_row else {
            return Err(anyhow!("after_sales not found"));
        };
        let current =
            AfterSalesStatus::from_str(&status_row.status).unwrap_or(AfterSalesStatus::Open);
        if !matches!(
            current,
            AfterSalesStatus::Approved | AfterSalesStatus::Received
        ) {
            return Err(anyhow!("after_sales status not receivable"));
        }

        let existing_receipt = sqlx::query_as::<_, ReturnReceiptRow>(
            r#"
            select
                id, warehouse_id, status, return_status,
                received_at, putaway_at, created_at, updated_at
            from return_receipts
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.after_sales_id)
        .fetch_optional(&mut *tx)
        .await?;

        let mut allow_update = false;
        if let Some(row) = existing_receipt {
            let receipt = row.into_receipt(input.tenant_id, input.after_sales_id)?;
            if receipt.status == ReturnReceiptStatus::Cancelled && allow_cancelled {
                allow_update = true;
            } else {
                tx.commit().await?;
                return self
                    .get_detail(input.tenant_id, input.after_sales_id)
                    .await?
                    .ok_or_else(|| anyhow!("after_sales not found"));
            }
        }

        let items: Vec<AfterSalesItemRow> = sqlx::query_as(
            r#"
            select sku_id, qty, refund_cent
            from after_sales_items
            where tenant_id = $1 and after_sales_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.after_sales_id)
        .fetch_all(&mut *tx)
        .await?;

        if items.is_empty() {
            return Err(anyhow!("after_sales items empty"));
        }

        let now = input.received_at.unwrap_or_else(Utc::now);
        let receipt_status = if input.putaway_done {
            ReturnReceiptStatus::PutawayDone
        } else {
            ReturnReceiptStatus::Receiving
        };
        let putaway_at = if input.putaway_done { Some(now) } else { None };
        let received_at = Some(now);

        if input.link_inbound {
            let doc_no = format!("RETURN-{}", input.after_sales_id);
            let doc_status = receipt_status.to_string();
            let doc_id: uuid::Uuid = sqlx::query_scalar(
                r#"
                insert into inbound_docs(
                    tenant_id, warehouse_id, doc_type, doc_no, status, ref_type, ref_id
                ) values ($1,$2,$3,$4,$5,$6,$7)
                on conflict (tenant_id, doc_no)
                do update set status = excluded.status, updated_at = now()
                returning doc_id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.warehouse_id)
            .bind(RETURN_INBOUND_DOC_TYPE)
            .bind(&doc_no)
            .bind(&doc_status)
            .bind(RETURN_REF_TYPE)
            .bind(input.after_sales_id.to_string())
            .fetch_one(&mut *tx)
            .await?;

            for item in &items {
                let received_qty = if input.putaway_done { item.qty } else { 0 };
                let putaway_qty = if input.putaway_done { item.qty } else { 0 };
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
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(doc_id)
                .bind(item.sku_id)
                .bind(item.qty)
                .bind(received_qty)
                .bind(putaway_qty)
                .execute(&mut *tx)
                .await?;
            }
        }

        let receipt_id = if allow_update {
            let id: uuid::Uuid = sqlx::query_scalar(
                r#"
                update return_receipts
                set warehouse_id = $3,
                    status = $4,
                    return_status = $5,
                    received_at = $6,
                    putaway_at = $7,
                    updated_at = now()
                where tenant_id = $1 and after_sales_id = $2
                returning id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.after_sales_id)
            .bind(input.warehouse_id)
            .bind(receipt_status.to_string())
            .bind(map_return_status(receipt_status))
            .bind(received_at)
            .bind(putaway_at)
            .fetch_one(&mut *tx)
            .await?;
            id
        } else {
            let id: uuid::Uuid = sqlx::query_scalar(
                r#"
                insert into return_receipts(
                    tenant_id, after_sales_id, warehouse_id, status, return_status, received_at, putaway_at
                ) values ($1,$2,$3,$4,$5,$6,$7)
                returning id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.after_sales_id)
            .bind(input.warehouse_id)
            .bind(receipt_status.to_string())
            .bind(map_return_status(receipt_status))
            .bind(received_at)
            .bind(putaway_at)
            .fetch_one(&mut *tx)
            .await?;
            id
        };

        if input.putaway_done {
            if input.link_inbound {
                for item in &items {
                    if item.qty <= 0 {
                        continue;
                    }
                    apply_inventory_adjustment(
                        self.pool,
                        &mut tx,
                        input.tenant_id,
                        input.warehouse_id,
                        item.sku_id,
                        item.qty,
                        input.after_sales_id,
                    )
                    .await?;
                }
            }

            sqlx::query(
                r#"
            update after_sales
            set status = $3,
                after_sale_status = $4,
                updated_at = now()
            where tenant_id = $1 and id = $2
            "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.after_sales_id)
            .bind(AfterSalesStatus::Received.to_string())
            .bind(map_after_sale_status(AfterSalesStatus::Received))
            .execute(&mut *tx)
            .await?;
        }

        if receipt_id.is_nil() {
            return Err(anyhow!("return receipt insert failed"));
        }

        tx.commit().await?;

        self.get_detail(input.tenant_id, input.after_sales_id)
            .await?
            .ok_or_else(|| anyhow!("after_sales not found"))
    }

    pub async fn upsert_refund_doc(
        &self,
        input: RefundDocCreateInput,
    ) -> anyhow::Result<(RefundDoc, bool)> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let existing = sqlx::query_as::<_, RefundDocRow>(
            r#"
            select
                id, after_sales_id, amount_cent, status, refund_status, channel,
                created_at, updated_at
            from refund_docs
            where tenant_id = $1 and after_sales_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.after_sales_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = existing {
            let status = RefundStatus::from_str(&row.status).unwrap_or(RefundStatus::Pending);
            if status == RefundStatus::Failed {
                let updated: RefundDocRow = sqlx::query_as(
                    r#"
                    update refund_docs
                    set status = 'PENDING',
                        refund_status = 'PENDING_REFUND',
                        amount_cent = $3,
                        channel = $4,
                        updated_at = now()
                    where tenant_id = $1 and after_sales_id = $2
                    returning
                        id, after_sales_id, amount_cent, status, refund_status, channel,
                        created_at, updated_at
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(input.after_sales_id)
                .bind(input.amount_cent)
                .bind(input.channel.to_string())
                .fetch_one(&mut *tx)
                .await?;

                tx.commit().await?;
                return Ok((updated.into_refund_doc(input.tenant_id)?, true));
            }

            tx.commit().await?;
            return Ok((row.into_refund_doc(input.tenant_id)?, false));
        }

        let inserted: RefundDocRow = sqlx::query_as(
            r#"
            insert into refund_docs(
                tenant_id, after_sales_id, amount_cent, status, refund_status, channel
            ) values ($1,$2,$3,'PENDING','PENDING_APPROVAL',$4)
            returning
                id, after_sales_id, amount_cent, status, refund_status, channel,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.after_sales_id)
        .bind(input.amount_cent)
        .bind(input.channel.to_string())
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((inserted.into_refund_doc(input.tenant_id)?, true))
    }

    pub async fn list_refund_docs(
        &self,
        tenant_id: TenantId,
        filters: RefundDocListFilters,
    ) -> anyhow::Result<RefundDocListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                id, after_sales_id, amount_cent, status, refund_status, channel,
                created_at, updated_at
            from refund_docs
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_refund_filters(&mut qb, &filters);
        qb.push(" order by created_at desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<RefundDocRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(row.into_refund_doc(tenant_id)?);
        }

        let mut count_qb =
            QueryBuilder::<Postgres>::new("select count(*) from refund_docs where tenant_id = ");
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_refund_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        Ok(RefundDocListResult { items, total })
    }

    pub async fn mark_refund_done(
        &self,
        tenant_id: TenantId,
        refund_doc_id: uuid::Uuid,
    ) -> anyhow::Result<RefundDoc> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let row = sqlx::query_as::<_, RefundDocRow>(
            r#"
            select
                id, after_sales_id, amount_cent, status, refund_status, channel,
                created_at, updated_at
            from refund_docs
            where tenant_id = $1 and id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(refund_doc_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Err(anyhow!("refund_doc not found"));
        };

        let status = RefundStatus::from_str(&row.status).unwrap_or(RefundStatus::Pending);
        let updated = if status == RefundStatus::Done {
            row
        } else {
            sqlx::query_as(
                r#"
                update refund_docs
                set status = 'DONE',
                    refund_status = 'REFUNDED',
                    updated_at = now()
                where tenant_id = $1 and id = $2
                returning
                    id, after_sales_id, amount_cent, status, refund_status, channel,
                    created_at, updated_at
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(refund_doc_id)
            .fetch_one(&mut *tx)
            .await?
        };

        sqlx::query(
            r#"
            update after_sales
            set status = $3,
                after_sale_status = $4,
                updated_at = now()
            where tenant_id = $1 and id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(updated.after_sales_id)
        .bind(AfterSalesStatus::Refunded.to_string())
        .bind(map_after_sale_status(AfterSalesStatus::Refunded))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        updated.into_refund_doc(tenant_id)
    }

    pub async fn record_shipments(
        &self,
        tenant_id: TenantId,
        after_sales_id: uuid::Uuid,
        shipments: &[AfterSalesShipmentCreate],
    ) -> anyhow::Result<()> {
        if shipments.is_empty() {
            return Ok(());
        }
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into after_sales_shipments(
                tenant_id, after_sales_id, shipment_id, shipment_type, origin_shipment_id
            )
            "#,
        );
        qb.push_values(shipments.iter(), |mut b, item| {
            b.push_bind(uuid::Uuid::from(tenant_id))
                .push_bind(after_sales_id)
                .push_bind(item.shipment_id)
                .push_bind(&item.shipment_type)
                .push_bind(item.origin_shipment_id);
        });
        qb.push(" on conflict (tenant_id, after_sales_id, shipment_id) do nothing");
        qb.build().execute(self.pool).await?;
        Ok(())
    }

    pub async fn create_exchange_docs(
        &self,
        tenant_id: TenantId,
        after_sales_id: uuid::Uuid,
        docs: &[ExchangeDocCreateInput],
    ) -> anyhow::Result<Vec<ExchangeDoc>> {
        if docs.is_empty() {
            return Ok(Vec::new());
        }
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into exchange_docs(
                tenant_id, after_sales_id, exchange_status, outbound_doc_id, shipment_id
            )
            "#,
        );
        qb.push_values(docs, |mut b, doc| {
            b.push_bind(uuid::Uuid::from(tenant_id))
                .push_bind(after_sales_id)
                .push_bind("CREATED")
                .push_bind(doc.outbound_doc_id)
                .push_bind(doc.shipment_id);
        });
        qb.push(
            r#"
            returning exchange_id, tenant_id, after_sales_id, exchange_status,
                      outbound_doc_id, shipment_id, created_at, updated_at
            "#,
        );
        let rows: Vec<ExchangeDocRow> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows.into_iter().map(ExchangeDocRow::into_doc).collect())
    }
}

fn apply_after_sales_filters(qb: &mut QueryBuilder<Postgres>, filters: &AfterSalesListFilters) {
    if let Some(status) = filters.status {
        qb.push(" and status = ");
        qb.push_bind(status.to_string());
    }
    if let Some(after_sales_type) = filters.after_sales_type {
        qb.push(" and type = ");
        qb.push_bind(after_sales_type.to_string());
    }
    if let Some(order_id) = filters.order_id {
        qb.push(" and order_id = ");
        qb.push_bind(uuid::Uuid::from(order_id));
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_refund_filters(qb: &mut QueryBuilder<Postgres>, filters: &RefundDocListFilters) {
    if let Some(status) = filters.status {
        qb.push(" and status = ");
        qb.push_bind(status.to_string());
    }
    if let Some(after_sales_id) = filters.after_sales_id {
        qb.push(" and after_sales_id = ");
        qb.push_bind(after_sales_id);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and created_at <= ");
        qb.push_bind(created_to);
    }
}

async fn apply_inventory_adjustment(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty_delta: i64,
    after_sales_id: uuid::Uuid,
) -> anyhow::Result<()> {
    apply_inventory_adjustment_with_reason(
        pool,
        tx,
        tenant_id,
        warehouse_id,
        sku_id,
        qty_delta,
        after_sales_id,
        RETURN_LEDGER_KIND,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn apply_inventory_adjustment_with_reason(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty_delta: i64,
    after_sales_id: uuid::Uuid,
    reason_type: &str,
) -> anyhow::Result<()> {
    let service = InventoryService::new(pool);
    service
        .apply_with_tx(
            tx,
            InventoryDelta {
                tenant_id,
                warehouse_id,
                sku_id,
                delta_on_hand: qty_delta,
                delta_reserved: 0,
                delta_in_transit: 0,
                delta_frozen: 0,
                reason_type: reason_type.to_string(),
                ref_type: Some(RETURN_REF_TYPE.to_string()),
                ref_id: Some(after_sales_id.to_string()),
                occurred_at: Utc::now(),
            },
        )
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct AfterSalesRow {
    id: uuid::Uuid,
    order_id: uuid::Uuid,
    after_sales_type: String,
    status: String,
    after_sale_status: String,
    compensation_status: Option<String>,
    evidence_status: Option<String>,
    reason_code: Option<String>,
    amount_cent: i64,
    raw_json: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AfterSalesRow {
    fn into_after_sales(self, tenant_id: TenantId) -> anyhow::Result<AfterSales> {
        let after_sales_type = AfterSalesType::from_str(&self.after_sales_type)
            .map_err(|e| anyhow!("bad after_sales type: {e}"))?;
        let status = AfterSalesStatus::from_str(&self.status)
            .map_err(|e| anyhow!("bad after_sales status: {e}"))?;

        Ok(AfterSales {
            id: self.id,
            tenant_id,
            order_id: OrderId(self.order_id),
            after_sales_type,
            status,
            after_sale_status: self.after_sale_status,
            compensation_status: self.compensation_status,
            evidence_status: self.evidence_status,
            reason_code: self.reason_code,
            amount_cent: self.amount_cent,
            raw_json: self.raw_json,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct AfterSalesItemRow {
    sku_id: uuid::Uuid,
    qty: i64,
    refund_cent: i64,
}

#[derive(sqlx::FromRow)]
struct ReturnReceiptRow {
    id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    status: String,
    return_status: String,
    received_at: Option<DateTime<Utc>>,
    putaway_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ReturnReceiptRow {
    fn into_receipt(
        self,
        tenant_id: TenantId,
        after_sales_id: uuid::Uuid,
    ) -> anyhow::Result<ReturnReceipt> {
        let status = ReturnReceiptStatus::from_str(&self.status)
            .map_err(|e| anyhow!("bad return_receipt status: {e}"))?;
        Ok(ReturnReceipt {
            id: self.id,
            tenant_id,
            after_sales_id,
            warehouse_id: self.warehouse_id,
            status,
            return_status: self.return_status,
            received_at: self.received_at,
            putaway_at: self.putaway_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct RefundDocRow {
    id: uuid::Uuid,
    after_sales_id: uuid::Uuid,
    amount_cent: i64,
    status: String,
    refund_status: String,
    channel: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ExchangeDocRow {
    exchange_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    after_sales_id: uuid::Uuid,
    exchange_status: String,
    outbound_doc_id: Option<uuid::Uuid>,
    shipment_id: Option<uuid::Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct AfterSalesShipmentRow {
    shipment_id: uuid::Uuid,
    shipment_type: String,
    origin_shipment_id: Option<uuid::Uuid>,
    created_at: DateTime<Utc>,
}

impl RefundDocRow {
    fn into_refund_doc(self, tenant_id: TenantId) -> anyhow::Result<RefundDoc> {
        let status =
            RefundStatus::from_str(&self.status).map_err(|e| anyhow!("bad refund status: {e}"))?;
        let channel = RefundChannel::from_str(&self.channel)
            .map_err(|e| anyhow!("bad refund channel: {e}"))?;
        Ok(RefundDoc {
            id: self.id,
            tenant_id,
            after_sales_id: self.after_sales_id,
            amount_cent: self.amount_cent,
            status,
            refund_status: self.refund_status,
            channel,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

impl ExchangeDocRow {
    fn into_doc(self) -> ExchangeDoc {
        ExchangeDoc {
            exchange_id: self.exchange_id,
            tenant_id: TenantId(self.tenant_id),
            after_sales_id: self.after_sales_id,
            exchange_status: self.exchange_status,
            outbound_doc_id: self.outbound_doc_id,
            shipment_id: self.shipment_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn map_after_sale_status(status: AfterSalesStatus) -> &'static str {
    match status {
        AfterSalesStatus::Open => "SUBMITTED",
        AfterSalesStatus::Approved => "ACCEPTED",
        AfterSalesStatus::Received => "ACCEPTED",
        AfterSalesStatus::Refunded => "CLOSED",
        AfterSalesStatus::Closed => "CLOSED",
        AfterSalesStatus::Rejected => "REJECTED",
    }
}

fn map_return_status(status: ReturnReceiptStatus) -> &'static str {
    match status {
        ReturnReceiptStatus::Receiving => "QC_CHECKING",
        ReturnReceiptStatus::PutawayDone => "QC_PASSED",
        ReturnReceiptStatus::Cancelled => "QC_FAILED",
    }
}
