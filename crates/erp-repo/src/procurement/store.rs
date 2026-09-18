use super::models::*;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::HashMap;

use crate::inventory::InventoryService;

const PROCUREMENT_APPROVAL_SETTING_KEY: &str = "procurement.approval_required";
const PROCUREMENT_SALES_WINDOW_SETTING_KEY: &str = "procurement.sales_window_days";
const DEFAULT_SALES_WINDOW_DAYS: i64 = 30;

pub struct PurchaseOrdersStore<'a> {
    pool: &'a PgPool,
}

impl<'a> PurchaseOrdersStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_orders(
        &self,
        tenant_id: TenantId,
        filters: PurchaseOrderListFilters,
    ) -> anyhow::Result<PurchaseOrderListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from purchase_orders po
            where po.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_order_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                po.po_id,
                po.supplier_id,
                s.name as supplier_name,
                po.warehouse_id,
                po.expected_at,
                po.remark,
                po.status,
                po.purchase_order_status,
                po.total_cent,
                po.created_at,
                po.updated_at
            from purchase_orders po
            left join suppliers s
              on s.supplier_id = po.supplier_id
             and s.tenant_id = po.tenant_id
            where po.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_order_filters(&mut qb, &filters);
        qb.push(" order by po.created_at desc, po.po_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            po_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            warehouse_id: Option<uuid::Uuid>,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            status: String,
            purchase_order_status: String,
            total_cent: i64,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| PurchaseOrder {
                po_id: row.po_id,
                tenant_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                warehouse_id: row.warehouse_id,
                expected_at: row.expected_at,
                remark: row.remark,
                status: row.status,
                purchase_order_status: row.purchase_order_status,
                total_cent: row.total_cent,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(PurchaseOrderListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PurchaseOrderDetail>> {
        let order = self.get_order(tenant_id, po_id, self.pool).await?;
        let Some(order) = order else {
            return Ok(None);
        };

        let items: Vec<PurchaseOrderItem> = sqlx::query_as(
            r#"
            select i.sku_id, s.sku_code, i.qty, i.price_cent, i.received_qty
            from purchase_order_items i
            left join skus s
              on s.sku_id = i.sku_id
             and s.tenant_id = i.tenant_id
            where i.tenant_id = $1 and i.po_id = $2
            order by i.sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .fetch_all(self.pool)
        .await?;

        let receipts_rows: Vec<PurchaseReceiptSummaryRow> = sqlx::query_as(
            r#"
            select r.receipt_id,
                   r.po_id,
                   po.supplier_id,
                   s.name as supplier_name,
                   r.warehouse_id,
                   r.status,
                   r.receiving_status,
                   r.confirmed_at,
                   r.created_at
            from purchase_receipts r
            join purchase_orders po
              on po.po_id = r.po_id
             and po.tenant_id = r.tenant_id
            left join suppliers s
              on s.supplier_id = po.supplier_id
             and s.tenant_id = po.tenant_id
            where r.tenant_id = $1 and r.po_id = $2
            order by r.created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .fetch_all(self.pool)
        .await?;

        let receipts = receipts_rows
            .into_iter()
            .map(PurchaseReceiptSummaryRow::into_summary)
            .collect();

        Ok(Some(PurchaseOrderDetail {
            order,
            items,
            receipts,
        }))
    }

    pub async fn create_order(
        &self,
        input: PurchaseOrderCreate,
    ) -> anyhow::Result<PurchaseOrderDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("purchase order items required"));
        }
        let total_cent = compute_total(&input.items)?;

        let po_id = uuid::Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r#"
            insert into purchase_orders(
                po_id, tenant_id, supplier_id, warehouse_id, expected_at, remark,
                status, purchase_order_status, total_cent
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(po_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.supplier_id)
        .bind(input.warehouse_id)
        .bind(input.expected_at)
        .bind(input.remark.as_deref())
        .bind(PURCHASE_ORDER_STATUS_DRAFT)
        .bind(PURCHASE_ORDER_STATUS_DRAFT)
        .bind(total_cent)
        .execute(&mut *tx)
        .await?;

        insert_order_items(&mut tx, input.tenant_id, po_id, &input.items).await?;
        tx.commit().await?;
        self.get_detail(input.tenant_id, po_id)
            .await?
            .ok_or_else(|| anyhow!("purchase order not found"))
    }

    pub async fn update_order(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
        input: PurchaseOrderUpdate,
    ) -> anyhow::Result<Option<PurchaseOrderDetail>> {
        let mut tx = self.pool.begin().await?;
        #[derive(sqlx::FromRow)]
        struct Row {
            supplier_id: uuid::Uuid,
            warehouse_id: Option<uuid::Uuid>,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            status: String,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select supplier_id, warehouse_id, expected_at, remark, status
            from purchase_orders
            where tenant_id = $1 and po_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        if row.status != PURCHASE_ORDER_STATUS_DRAFT {
            return Err(anyhow!("purchase order not editable"));
        }

        let supplier_id = input.supplier_id.unwrap_or(row.supplier_id);
        let warehouse_id = input.warehouse_id.or(row.warehouse_id);
        let expected_at = input.expected_at.or(row.expected_at);
        let remark = input.remark.or(row.remark);
        let items = match input.items {
            Some(items) => {
                if items.is_empty() {
                    return Err(anyhow!("purchase order items required"));
                }
                sqlx::query(
                    r#"
                    delete from purchase_order_items
                    where tenant_id = $1 and po_id = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(po_id)
                .execute(&mut *tx)
                .await?;
                insert_order_items(&mut tx, tenant_id, po_id, &items).await?;
                items
            }
            None => fetch_order_items(&mut tx, tenant_id, po_id).await?,
        };

        let total_cent = compute_total(&items)?;
        sqlx::query(
            r#"
            update purchase_orders
            set supplier_id = $3,
                warehouse_id = $4,
                expected_at = $5,
                remark = $6,
                total_cent = $7,
                updated_at = now()
            where tenant_id = $1 and po_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .bind(supplier_id)
        .bind(warehouse_id)
        .bind(expected_at)
        .bind(remark.as_deref())
        .bind(total_cent)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_detail(tenant_id, po_id).await
    }

    pub async fn delete_order(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PurchaseOrder>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            po_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            warehouse_id: Option<uuid::Uuid>,
            expected_at: Option<DateTime<Utc>>,
            remark: Option<String>,
            status: String,
            purchase_order_status: String,
            total_cent: i64,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from purchase_orders
            where tenant_id = $1
              and po_id = $2
              and status = $3
            returning po_id, supplier_id, warehouse_id, expected_at, remark, status,
                      purchase_order_status, total_cent, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .bind(PURCHASE_ORDER_STATUS_DRAFT)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PurchaseOrder {
            po_id: row.po_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: None,
            warehouse_id: row.warehouse_id,
            expected_at: row.expected_at,
            remark: row.remark,
            status: row.status,
            purchase_order_status: row.purchase_order_status,
            total_cent: row.total_cent,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn mark_sent(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PurchaseOrder>> {
        let row: Option<PurchaseOrderRow> = sqlx::query_as(
            r#"
            update purchase_orders
            set status = $3,
                purchase_order_status = $5,
                updated_at = now()
            where tenant_id = $1 and po_id = $2 and status = $4
            returning po_id, supplier_id, warehouse_id, expected_at, remark, status,
                      purchase_order_status, total_cent, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .bind(PURCHASE_ORDER_STATUS_SENT)
        .bind(PURCHASE_ORDER_STATUS_DRAFT)
        .bind(PURCHASE_ORDER_STATUS_PENDING_APPROVAL)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    pub async fn confirm_order(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PurchaseOrder>> {
        let row: Option<PurchaseOrderRow> = sqlx::query_as(
            r#"
            update purchase_orders
            set status = $3,
                purchase_order_status = $5,
                updated_at = now()
            where tenant_id = $1 and po_id = $2 and status = $4
            returning po_id, supplier_id, warehouse_id, expected_at, remark, status,
                      purchase_order_status, total_cent, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .bind(PURCHASE_ORDER_STATUS_CONFIRMED)
        .bind(PURCHASE_ORDER_STATUS_SENT)
        .bind(PURCHASE_ORDER_STATUS_PLACED)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    pub async fn receive_order(
        &self,
        input: PurchaseOrderReceiveInput,
    ) -> anyhow::Result<PurchaseOrderReceiveResult> {
        if input.items.is_empty() {
            return Err(anyhow!("receipt items required"));
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct OrderRow {
            supplier_id: uuid::Uuid,
            warehouse_id: Option<uuid::Uuid>,
            status: String,
        }

        let order_row: Option<OrderRow> = sqlx::query_as(
            r#"
            select supplier_id, warehouse_id, status
            from purchase_orders
            where tenant_id = $1 and po_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.po_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(order_row) = order_row else {
            return Err(anyhow!("purchase order not found"));
        };
        if order_row.status == PURCHASE_ORDER_STATUS_CANCELLED {
            return Err(anyhow!("purchase order cancelled"));
        }
        if order_row.status == PURCHASE_ORDER_STATUS_DRAFT {
            return Err(anyhow!("purchase order not sent"));
        }
        let approval_required =
            fetch_procurement_approval_required(self.pool, input.tenant_id).await?;
        if approval_required && order_row.status == PURCHASE_ORDER_STATUS_SENT {
            return Err(anyhow!("purchase order not confirmed"));
        }
        if let Some(warehouse_id) = order_row.warehouse_id {
            if warehouse_id != input.warehouse_id {
                return Err(anyhow!("warehouse mismatch"));
            }
        }

        let receipt_id = uuid::Uuid::new_v4();
        let mut receiving_status = PURCHASE_RECEIPT_STATUS_PART_RECEIVED;
        sqlx::query(
            r#"
            insert into purchase_receipts(
                receipt_id, tenant_id, po_id, warehouse_id, status, receiving_status, confirmed_at
            ) values ($1,$2,$3,$4,$5,$6,now())
            "#,
        )
        .bind(receipt_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.po_id)
        .bind(input.warehouse_id)
        .bind(PURCHASE_RECEIPT_STATUS_CONFIRMED)
        .bind(receiving_status)
        .execute(&mut *tx)
        .await?;

        let mut amount_cent: i64 = 0;
        let mut receipt_items = Vec::with_capacity(input.items.len());
        let mut receipt_item_prices: HashMap<uuid::Uuid, i64> =
            HashMap::with_capacity(input.items.len());
        for item in &input.items {
            if item.qty <= 0 {
                return Err(anyhow!("received qty must be positive"));
            }
            #[derive(sqlx::FromRow)]
            struct ItemRow {
                qty: i64,
                received_qty: i64,
                price_cent: i64,
            }

            let row: ItemRow = sqlx::query_as(
                r#"
                select qty, received_qty, price_cent
                from purchase_order_items
                where tenant_id = $1 and po_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.po_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("purchase order item not found"))?;

            let next_received = row
                .received_qty
                .checked_add(item.qty)
                .ok_or_else(|| anyhow!("received qty overflow"))?;
            if next_received > row.qty {
                return Err(anyhow!("received qty exceeds ordered qty"));
            }

            sqlx::query(
                r#"
                update purchase_order_items
                set received_qty = $4,
                    updated_at = now()
                where tenant_id = $1 and po_id = $2 and sku_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.po_id)
            .bind(item.sku_id)
            .bind(next_received)
            .execute(&mut *tx)
            .await?;

            amount_cent = amount_cent
                .checked_add(item.qty.saturating_mul(row.price_cent))
                .ok_or_else(|| anyhow!("amount overflow"))?;

            sqlx::query(
                r#"
                insert into receipt_items(
                    tenant_id, receipt_id, sku_id, qty
                ) values ($1,$2,$3,$4)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(receipt_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;

            receipt_items.push(PurchaseReceiptItem {
                sku_id: item.sku_id,
                qty: item.qty,
            });
            receipt_item_prices.insert(item.sku_id, row.price_cent);
        }

        let all_received: bool = sqlx::query_scalar::<_, Option<bool>>(
            r#"
            select bool_and(received_qty >= qty)
            from purchase_order_items
            where tenant_id = $1 and po_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.po_id)
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(false);

        receiving_status = if all_received {
            PURCHASE_RECEIPT_STATUS_RECEIVED
        } else {
            PURCHASE_RECEIPT_STATUS_PART_RECEIVED
        };
        sqlx::query(
            r#"
            update purchase_receipts
            set receiving_status = $3,
                updated_at = now()
            where tenant_id = $1 and receipt_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(receipt_id)
        .bind(receiving_status)
        .execute(&mut *tx)
        .await?;

        let next_status = if all_received {
            PURCHASE_ORDER_STATUS_DONE
        } else {
            PURCHASE_ORDER_STATUS_RECEIVING
        };
        let next_purchase_status = if all_received {
            PURCHASE_ORDER_STATUS_RECEIVED
        } else {
            PURCHASE_ORDER_STATUS_PARTIALLY_RECEIVED
        };

        sqlx::query(
            r#"
            update purchase_orders
            set status = $3,
                purchase_order_status = $4,
                updated_at = now()
            where tenant_id = $1 and po_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.po_id)
        .bind(next_status)
        .bind(next_purchase_status)
        .execute(&mut *tx)
        .await?;

        let inbound_doc_id = uuid::Uuid::new_v4();
        let inbound_doc_no = format!("PO-{}-{}", input.po_id, receipt_id);
        sqlx::query(
            r#"
            insert into inbound_docs(
                doc_id, tenant_id, warehouse_id, doc_type, doc_no, status, ref_type, ref_id
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(inbound_doc_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(INBOUND_DOC_TYPE_PURCHASE)
        .bind(&inbound_doc_no)
        .bind(INBOUND_STATUS_PUTAWAY_DONE)
        .bind("PURCHASE_RECEIPT")
        .bind(receipt_id.to_string())
        .execute(&mut *tx)
        .await?;

        let inventory_service = InventoryService::new(self.pool);
        for item in &receipt_items {
            let price_cent = receipt_item_prices.get(&item.sku_id).copied().unwrap_or(0);
            sqlx::query(
                r#"
                insert into inbound_doc_items(
                    tenant_id, doc_id, sku_id, planned_qty, received_qty, putaway_qty
                ) values ($1,$2,$3,$4,$5,$6)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(inbound_doc_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .bind(item.qty)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;

            inventory_service
                .apply_with_tx(
                    &mut tx,
                    crate::inventory::InventoryDelta {
                        tenant_id: input.tenant_id,
                        warehouse_id: input.warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: item.qty,
                        delta_reserved: 0,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: INVENTORY_REASON_PURCHASE_RECEIPT.to_string(),
                        ref_type: Some("PURCHASE_RECEIPT".to_string()),
                        ref_id: Some(receipt_id.to_string()),
                        occurred_at: Utc::now(),
                    },
                )
                .await?;

            if input.update_cost {
                let previous_cost: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
                    r#"
                    select cost_cent
                    from skus
                    where tenant_id = $1 and sku_id = $2
                    for update
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(item.sku_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| anyhow!("sku not found"))?;

                sqlx::query(
                    r#"
                    update skus
                    set cost_cent = $3,
                        updated_at = now()
                    where tenant_id = $1 and sku_id = $2
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(item.sku_id)
                .bind(price_cent)
                .execute(&mut *tx)
                .await?;

                sqlx::query(
                    r#"
                    insert into sku_cost_ledgers(
                        tenant_id, sku_id, previous_cost_cent, new_cost_cent, ref_type, ref_id
                    ) values ($1,$2,$3,$4,$5,$6)
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(item.sku_id)
                .bind(previous_cost)
                .bind(price_cent)
                .bind("PURCHASE_RECEIPT")
                .bind(receipt_id.to_string())
                .execute(&mut *tx)
                .await?;
            }

            sqlx::query(
                r#"
                insert into inbound_putaway_tasks(
                    tenant_id, doc_id, sku_id, qty, from_area, status
                ) values ($1,$2,$3,$4,$5,$6)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(inbound_doc_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .bind("RECEIVING")
            .bind("DONE")
            .execute(&mut *tx)
            .await?;
        }

        let ap_doc_id: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            insert into ap_docs(
                tenant_id, supplier_id, ref_type, ref_id, amount_cent, status, settlement_status
            ) values ($1,$2,$3,$4,$5,$6,$7)
            on conflict (tenant_id, ref_type, ref_id)
            do nothing
            returning ap_id
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(order_row.supplier_id)
        .bind("RECEIPT")
        .bind(receipt_id.to_string())
        .bind(amount_cent)
        .bind(AP_STATUS_OPEN)
        .bind(AP_SETTLEMENT_STATUS_UNSETTLED)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        let order = self
            .get_order(input.tenant_id, input.po_id, self.pool)
            .await?
            .ok_or_else(|| anyhow!("purchase order not found"))?;
        let receipt = PurchaseReceipt {
            receipt_id,
            tenant_id: input.tenant_id,
            po_id: input.po_id,
            supplier_id: Some(order_row.supplier_id),
            supplier_name: order.supplier_name.clone(),
            warehouse_id: input.warehouse_id,
            status: PURCHASE_RECEIPT_STATUS_CONFIRMED.to_string(),
            receiving_status: receiving_status.to_string(),
            confirmed_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        Ok(PurchaseOrderReceiveResult {
            order,
            receipt,
            receipt_items,
            inbound_doc_id,
            ap_doc_id,
        })
    }

    async fn get_order<'t, T>(
        &self,
        tenant_id: TenantId,
        po_id: uuid::Uuid,
        executor: T,
    ) -> anyhow::Result<Option<PurchaseOrder>>
    where
        T: sqlx::Executor<'t, Database = Postgres>,
    {
        let row: Option<PurchaseOrderRow> = sqlx::query_as(
            r#"
            select po.po_id,
                   po.supplier_id,
                   s.name as supplier_name,
                   po.warehouse_id,
                   po.expected_at,
                   po.remark,
                   po.status,
                   po.purchase_order_status,
                   po.total_cent,
                   po.created_at,
                   po.updated_at
            from purchase_orders po
            left join suppliers s
              on s.supplier_id = po.supplier_id
             and s.tenant_id = po.tenant_id
            where po.tenant_id = $1 and po.po_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.map(|row| row.into_order(tenant_id)))
    }
}

#[derive(sqlx::FromRow)]
struct PurchaseOrderRow {
    po_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    supplier_name: Option<String>,
    warehouse_id: Option<uuid::Uuid>,
    expected_at: Option<DateTime<Utc>>,
    remark: Option<String>,
    status: String,
    purchase_order_status: String,
    total_cent: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PurchaseOrderRow {
    fn into_order(self, tenant_id: TenantId) -> PurchaseOrder {
        PurchaseOrder {
            po_id: self.po_id,
            tenant_id,
            supplier_id: self.supplier_id,
            supplier_name: self.supplier_name,
            warehouse_id: self.warehouse_id,
            expected_at: self.expected_at,
            remark: self.remark,
            status: self.status,
            purchase_order_status: self.purchase_order_status,
            total_cent: self.total_cent,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

pub struct PurchaseReceiptsStore<'a> {
    pool: &'a PgPool,
}

impl<'a> PurchaseReceiptsStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_receipts(
        &self,
        tenant_id: TenantId,
        filters: PurchaseReceiptListFilters,
    ) -> anyhow::Result<PurchaseReceiptListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from purchase_receipts r
            join purchase_orders po
              on po.po_id = r.po_id
             and po.tenant_id = r.tenant_id
            where r.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_receipt_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                r.receipt_id,
                r.po_id,
                po.supplier_id,
                s.name as supplier_name,
                r.warehouse_id,
                r.status,
                r.receiving_status,
                r.confirmed_at,
                r.created_at
            from purchase_receipts r
            join purchase_orders po
              on po.po_id = r.po_id
             and po.tenant_id = r.tenant_id
            left join suppliers s
              on s.supplier_id = po.supplier_id
             and s.tenant_id = po.tenant_id
            where r.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_receipt_filters(&mut qb, &filters);
        qb.push(" order by r.created_at desc, r.receipt_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            receipt_id: uuid::Uuid,
            po_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            warehouse_id: uuid::Uuid,
            status: String,
            receiving_status: String,
            confirmed_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| PurchaseReceiptSummary {
                receipt_id: row.receipt_id,
                po_id: row.po_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                warehouse_id: row.warehouse_id,
                status: row.status,
                receiving_status: row.receiving_status,
                confirmed_at: row.confirmed_at,
                created_at: row.created_at,
            })
            .collect();

        Ok(PurchaseReceiptListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        receipt_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PurchaseReceiptDetail>> {
        #[derive(sqlx::FromRow)]
        struct ReceiptRow {
            receipt_id: uuid::Uuid,
            po_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            warehouse_id: uuid::Uuid,
            status: String,
            receiving_status: String,
            confirmed_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<ReceiptRow> = sqlx::query_as(
            r#"
            select
                r.receipt_id,
                r.po_id,
                po.supplier_id,
                s.name as supplier_name,
                r.warehouse_id,
                r.status,
                r.receiving_status,
                r.confirmed_at,
                r.created_at,
                r.updated_at
            from purchase_receipts r
            join purchase_orders po
              on po.po_id = r.po_id
             and po.tenant_id = r.tenant_id
            left join suppliers s
              on s.supplier_id = po.supplier_id
             and s.tenant_id = po.tenant_id
            where r.tenant_id = $1 and r.receipt_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(receipt_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let items: Vec<PurchaseReceiptItem> = sqlx::query_as(
            r#"
            select sku_id, qty
            from receipt_items
            where tenant_id = $1 and receipt_id = $2
            order by sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(receipt_id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(PurchaseReceiptDetail {
            receipt: PurchaseReceipt {
                receipt_id: row.receipt_id,
                tenant_id,
                po_id: row.po_id,
                supplier_id: Some(row.supplier_id),
                supplier_name: row.supplier_name,
                warehouse_id: row.warehouse_id,
                status: row.status,
                receiving_status: row.receiving_status,
                confirmed_at: row.confirmed_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            },
            items,
        }))
    }
}

pub struct PurchaseReturnsStore<'a> {
    pool: &'a PgPool,
}

impl<'a> PurchaseReturnsStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_returns(
        &self,
        tenant_id: TenantId,
        filters: PurchaseReturnListFilters,
    ) -> anyhow::Result<PurchaseReturnListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from purchase_returns pr
            join purchase_orders po
              on po.po_id = pr.po_id
             and po.tenant_id = pr.tenant_id
            where pr.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_return_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pr.return_id,
                pr.po_id,
                po.supplier_id,
                s.name as supplier_name,
                pr.warehouse_id,
                pr.status,
                pr.confirmed_at,
                pr.created_at,
                pr.updated_at
            from purchase_returns pr
            join purchase_orders po
              on po.po_id = pr.po_id
             and po.tenant_id = pr.tenant_id
            left join suppliers s
              on s.supplier_id = po.supplier_id
             and s.tenant_id = po.tenant_id
            where pr.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_return_filters(&mut qb, &filters);
        qb.push(" order by pr.created_at desc, pr.return_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<PurchaseReturnRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_return(tenant_id))
            .collect();

        Ok(PurchaseReturnListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        return_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PurchaseReturnDetail>> {
        let row: Option<PurchaseReturnRow> = sqlx::query_as(
            r#"
            select
                pr.return_id,
                pr.po_id,
                po.supplier_id,
                s.name as supplier_name,
                pr.warehouse_id,
                pr.status,
                pr.confirmed_at,
                pr.created_at,
                pr.updated_at
            from purchase_returns pr
            join purchase_orders po
              on po.po_id = pr.po_id
             and po.tenant_id = pr.tenant_id
            left join suppliers s
              on s.supplier_id = po.supplier_id
             and s.tenant_id = po.tenant_id
            where pr.tenant_id = $1 and pr.return_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(return_id)
        .fetch_optional(self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let items: Vec<PurchaseReturnItem> = sqlx::query_as(
            r#"
            select sku_id, qty
            from return_items
            where tenant_id = $1 and return_id = $2
            order by sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(return_id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(PurchaseReturnDetail {
            purchase_return: row.into_return(tenant_id),
            items,
        }))
    }

    pub async fn create_return(
        &self,
        input: PurchaseReturnCreate,
    ) -> anyhow::Result<PurchaseReturnDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("return items required"));
        }
        let mut tx = self.pool.begin().await?;
        let return_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"
            insert into purchase_returns(
                return_id, tenant_id, po_id, warehouse_id, status
            ) values ($1,$2,$3,$4,$5)
            "#,
        )
        .bind(return_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.po_id)
        .bind(input.warehouse_id)
        .bind(PURCHASE_RETURN_STATUS_DRAFT)
        .execute(&mut *tx)
        .await?;

        for item in &input.items {
            if item.qty <= 0 {
                return Err(anyhow!("return qty must be positive"));
            }
            sqlx::query(
                r#"
                insert into return_items(tenant_id, return_id, sku_id, qty)
                values ($1,$2,$3,$4)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(return_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get_detail(input.tenant_id, return_id)
            .await?
            .ok_or_else(|| anyhow!("purchase return not found"))
    }

    pub async fn confirm_return(
        &self,
        tenant_id: TenantId,
        return_id: uuid::Uuid,
    ) -> anyhow::Result<PurchaseReturnConfirmResult> {
        let mut tx = self.pool.begin().await?;
        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            status: String,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select warehouse_id, status
            from purchase_returns
            where tenant_id = $1 and return_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(return_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Err(anyhow!("purchase return not found"));
        };
        if row.status != PURCHASE_RETURN_STATUS_DRAFT {
            return Err(anyhow!("purchase return not confirmable"));
        }

        let items: Vec<PurchaseReturnItem> = sqlx::query_as(
            r#"
            select sku_id, qty
            from return_items
            where tenant_id = $1 and return_id = $2
            order by sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(return_id)
        .fetch_all(&mut *tx)
        .await?;

        let outbound_doc_id = uuid::Uuid::new_v4();
        let doc_no = format!("PR-{}", return_id);
        sqlx::query(
            r#"
            insert into outbound_docs(
                outbound_doc_id, tenant_id, warehouse_id, doc_type, shipment_id, doc_no, status
            ) values ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(outbound_doc_id)
        .bind(uuid::Uuid::from(tenant_id))
        .bind(row.warehouse_id)
        .bind(OUTBOUND_DOC_TYPE_PURCHASE_RETURN)
        .bind(Option::<uuid::Uuid>::None)
        .bind(doc_no)
        .bind(OUTBOUND_STATUS_SHIPPED)
        .execute(&mut *tx)
        .await?;

        let inventory_service = InventoryService::new(self.pool);
        for item in &items {
            sqlx::query(
                r#"
                insert into outbound_doc_items(
                    tenant_id, outbound_doc_id, sku_id, planned_qty, picked_qty, packed_qty
                ) values ($1,$2,$3,$4,$5,$6)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(outbound_doc_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .bind(item.qty)
            .bind(item.qty)
            .execute(&mut *tx)
            .await?;

            inventory_service
                .apply_with_tx(
                    &mut tx,
                    crate::inventory::InventoryDelta {
                        tenant_id,
                        warehouse_id: row.warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: -item.qty,
                        delta_reserved: 0,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: INVENTORY_REASON_PURCHASE_RETURN.to_string(),
                        ref_type: Some("PURCHASE_RETURN".to_string()),
                        ref_id: Some(return_id.to_string()),
                        occurred_at: Utc::now(),
                    },
                )
                .await?;
        }

        sqlx::query(
            r#"
            update purchase_returns
            set status = $3,
                confirmed_at = now(),
                updated_at = now()
            where tenant_id = $1 and return_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(return_id)
        .bind(PURCHASE_RETURN_STATUS_DONE)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        let detail = self
            .get_detail(tenant_id, return_id)
            .await?
            .ok_or_else(|| anyhow!("purchase return not found"))?;

        Ok(PurchaseReturnConfirmResult {
            purchase_return: detail.purchase_return,
            outbound_doc_id,
        })
    }
}

#[derive(sqlx::FromRow)]
struct PurchaseReturnRow {
    return_id: uuid::Uuid,
    po_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    supplier_name: Option<String>,
    warehouse_id: uuid::Uuid,
    status: String,
    confirmed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PurchaseReturnRow {
    fn into_return(self, tenant_id: TenantId) -> PurchaseReturn {
        PurchaseReturn {
            return_id: self.return_id,
            tenant_id,
            po_id: self.po_id,
            supplier_id: Some(self.supplier_id),
            supplier_name: self.supplier_name,
            warehouse_id: self.warehouse_id,
            status: self.status,
            confirmed_at: self.confirmed_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

pub struct PurchaseSuggestionsStore<'a> {
    pool: &'a PgPool,
}

impl<'a> PurchaseSuggestionsStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn refresh(&self, tenant_id: TenantId) -> anyhow::Result<Vec<PurchaseSuggestion>> {
        let safety_stock = fetch_safety_stock(self.pool, tenant_id).await?;
        let sales_window_days = fetch_sales_window_days(self.pool, tenant_id).await?;

        let sales_rows: Vec<(uuid::Uuid, i64)> = sqlx::query_as(
            r#"
            select (oi.sku_id::uuid) as sku_id, sum(oi.qty)::bigint as qty
            from order_items oi
            join orders o
              on o.tenant_id = oi.tenant_id
             and o.order_id = oi.order_id
             and o.order_month = oi.order_month
            where oi.tenant_id = $1
              and oi.sku_id ~* '^[0-9a-f-]{36}$'
              and o.order_created_at >= now() - make_interval(days => $2)
            group by oi.sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sales_window_days)
        .fetch_all(self.pool)
        .await?;
        let sales_map: HashMap<uuid::Uuid, i64> = sales_rows.into_iter().collect();

        #[derive(sqlx::FromRow)]
        struct InventoryRow {
            sku_id: uuid::Uuid,
            on_hand: i64,
            reserved: i64,
            in_transit: i64,
        }

        let inventory_rows: Vec<InventoryRow> = sqlx::query_as(
            r#"
            select sku_id,
                   sum(on_hand)::bigint as on_hand,
                   sum(reserved)::bigint as reserved,
                   sum(in_transit)::bigint as in_transit
            from inventory_balances
            where tenant_id = $1
            group by sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;
        let inventory_map: HashMap<uuid::Uuid, InventoryRow> = inventory_rows
            .into_iter()
            .map(|row| (row.sku_id, row))
            .collect();

        #[derive(sqlx::FromRow)]
        struct SkuRow {
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
        }

        let sku_rows: Vec<SkuRow> = sqlx::query_as(
            r#"
            select sku_id, sku_code
            from skus
            where tenant_id = $1 and is_active = true
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;

        let computed_at = Utc::now();
        let mut suggestions = Vec::with_capacity(sku_rows.len());
        let mut tx = self.pool.begin().await?;

        for sku in sku_rows {
            let sales_qty = *sales_map.get(&sku.sku_id).unwrap_or(&0);
            let inventory = inventory_map.get(&sku.sku_id);
            let on_hand = inventory.map(|row| row.on_hand).unwrap_or(0);
            let reserved = inventory.map(|row| row.reserved).unwrap_or(0);
            let in_transit = inventory.map(|row| row.in_transit).unwrap_or(0);
            let available = on_hand + in_transit - reserved;
            let target_stock = sales_qty.saturating_add(safety_stock);
            let suggested_qty = (target_stock - available).max(0);

            let reason = if sales_qty > 0 {
                "sales_velocity"
            } else if safety_stock > 0 {
                "safety_stock"
            } else {
                "low_stock"
            };

            sqlx::query(
                r#"
                insert into purchase_suggestions(
                    tenant_id, sku_id, suggested_qty, reason, computed_at
                ) values ($1,$2,$3,$4,$5)
                on conflict (tenant_id, sku_id)
                do update set
                    suggested_qty = excluded.suggested_qty,
                    reason = excluded.reason,
                    computed_at = excluded.computed_at
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(sku.sku_id)
            .bind(suggested_qty)
            .bind(reason)
            .bind(computed_at)
            .execute(&mut *tx)
            .await?;

            suggestions.push(PurchaseSuggestion {
                tenant_id,
                sku_id: sku.sku_id,
                sku_code: sku.sku_code,
                suggested_qty,
                reason: reason.to_string(),
                computed_at,
            });
        }

        tx.commit().await?;
        Ok(suggestions)
    }

    pub async fn list_suggestions(
        &self,
        tenant_id: TenantId,
        filters: PurchaseSuggestionListFilters,
    ) -> anyhow::Result<PurchaseSuggestionListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from purchase_suggestions ps
            where ps.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_suggestion_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                ps.sku_id,
                s.sku_code,
                ps.suggested_qty,
                ps.reason,
                ps.computed_at
            from purchase_suggestions ps
            left join skus s
              on s.sku_id = ps.sku_id
             and s.tenant_id = ps.tenant_id
            where ps.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_suggestion_filters(&mut qb, &filters);
        qb.push(" order by ps.suggested_qty desc, ps.sku_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            suggested_qty: i64,
            reason: String,
            computed_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| PurchaseSuggestion {
                tenant_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                suggested_qty: row.suggested_qty,
                reason: row.reason,
                computed_at: row.computed_at,
            })
            .collect();

        Ok(PurchaseSuggestionListResult { items, total })
    }
}

pub struct ApDocsStore<'a> {
    pool: &'a PgPool,
}

impl<'a> ApDocsStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_docs(
        &self,
        tenant_id: TenantId,
        filters: ApDocListFilters,
    ) -> anyhow::Result<ApDocListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from ap_docs ap
            where ap.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_ap_doc_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                ap.ap_id,
                ap.supplier_id,
                s.name as supplier_name,
                ap.ref_type,
                ap.ref_id,
                ap.amount_cent,
                ap.status,
                ap.settlement_status,
                ap.paid_at,
                ap.created_at,
                ap.updated_at
            from ap_docs ap
            left join suppliers s
              on s.supplier_id = ap.supplier_id
             and s.tenant_id = ap.tenant_id
            where ap.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_ap_doc_filters(&mut qb, &filters);
        qb.push(" order by ap.created_at desc, ap.ap_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            ap_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            status: String,
            settlement_status: String,
            paid_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ApDoc {
                ap_id: row.ap_id,
                tenant_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                ref_type: row.ref_type,
                ref_id: row.ref_id,
                amount_cent: row.amount_cent,
                status: row.status,
                settlement_status: row.settlement_status,
                paid_at: row.paid_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ApDocListResult { items, total })
    }

    pub async fn get_doc(
        &self,
        tenant_id: TenantId,
        ap_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ApDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ap_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            status: String,
            settlement_status: String,
            paid_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                ap.ap_id,
                ap.supplier_id,
                s.name as supplier_name,
                ap.ref_type,
                ap.ref_id,
                ap.amount_cent,
                ap.status,
                ap.settlement_status,
                ap.paid_at,
                ap.created_at,
                ap.updated_at
            from ap_docs ap
            left join suppliers s
              on s.supplier_id = ap.supplier_id
             and s.tenant_id = ap.tenant_id
            where ap.tenant_id = $1 and ap.ap_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ap_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ApDoc {
            ap_id: row.ap_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            status: row.status,
            settlement_status: row.settlement_status,
            paid_at: row.paid_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn mark_paid(
        &self,
        tenant_id: TenantId,
        ap_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ApDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ap_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            status: String,
            settlement_status: String,
            paid_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update ap_docs ap
            set status = $3,
                settlement_status = $4,
                paid_at = now(),
                updated_at = now()
            from suppliers s
            where ap.tenant_id = $1
              and ap.ap_id = $2
              and s.supplier_id = ap.supplier_id
              and s.tenant_id = ap.tenant_id
            returning ap.ap_id,
                      ap.supplier_id,
                      s.name as supplier_name,
                      ap.ref_type,
                      ap.ref_id,
                      ap.amount_cent,
                      ap.status,
                      ap.settlement_status,
                      ap.paid_at,
                      ap.created_at,
                      ap.updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ap_id)
        .bind(AP_STATUS_PAID)
        .bind(AP_SETTLEMENT_STATUS_SETTLED)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ApDoc {
            ap_id: row.ap_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            status: row.status,
            settlement_status: row.settlement_status,
            paid_at: row.paid_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn close_doc(
        &self,
        tenant_id: TenantId,
        ap_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ApDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ap_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            status: String,
            settlement_status: String,
            paid_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update ap_docs ap
            set status = $3,
                settlement_status = $4,
                updated_at = now()
            from suppliers s
            where ap.tenant_id = $1
              and ap.ap_id = $2
              and ap.status = $5
              and s.supplier_id = ap.supplier_id
              and s.tenant_id = ap.tenant_id
            returning ap.ap_id,
                      ap.supplier_id,
                      s.name as supplier_name,
                      ap.ref_type,
                      ap.ref_id,
                      ap.amount_cent,
                      ap.status,
                      ap.settlement_status,
                      ap.paid_at,
                      ap.created_at,
                      ap.updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ap_id)
        .bind(AP_STATUS_CLOSED)
        .bind(AP_SETTLEMENT_STATUS_SETTLED)
        .bind(AP_STATUS_PAID)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ApDoc {
            ap_id: row.ap_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            status: row.status,
            settlement_status: row.settlement_status,
            paid_at: row.paid_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn update_settlement_status(
        &self,
        tenant_id: TenantId,
        ap_id: uuid::Uuid,
        settlement_status: &str,
    ) -> anyhow::Result<Option<ApDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            ap_id: uuid::Uuid,
            supplier_id: uuid::Uuid,
            supplier_name: Option<String>,
            ref_type: String,
            ref_id: String,
            amount_cent: i64,
            status: String,
            settlement_status: String,
            paid_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update ap_docs ap
            set settlement_status = $3,
                updated_at = now()
            from suppliers s
            where ap.tenant_id = $1
              and ap.ap_id = $2
              and s.supplier_id = ap.supplier_id
              and s.tenant_id = ap.tenant_id
            returning ap.ap_id,
                      ap.supplier_id,
                      s.name as supplier_name,
                      ap.ref_type,
                      ap.ref_id,
                      ap.amount_cent,
                      ap.status,
                      ap.settlement_status,
                      ap.paid_at,
                      ap.created_at,
                      ap.updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ap_id)
        .bind(settlement_status)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ApDoc {
            ap_id: row.ap_id,
            tenant_id,
            supplier_id: row.supplier_id,
            supplier_name: row.supplier_name,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            amount_cent: row.amount_cent,
            status: row.status,
            settlement_status: row.settlement_status,
            paid_at: row.paid_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

#[derive(sqlx::FromRow)]
struct PurchaseReceiptSummaryRow {
    receipt_id: uuid::Uuid,
    po_id: uuid::Uuid,
    supplier_id: uuid::Uuid,
    supplier_name: Option<String>,
    warehouse_id: uuid::Uuid,
    status: String,
    receiving_status: String,
    confirmed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl PurchaseReceiptSummaryRow {
    fn into_summary(self) -> PurchaseReceiptSummary {
        PurchaseReceiptSummary {
            receipt_id: self.receipt_id,
            po_id: self.po_id,
            supplier_id: self.supplier_id,
            supplier_name: self.supplier_name,
            warehouse_id: self.warehouse_id,
            status: self.status,
            receiving_status: self.receiving_status,
            confirmed_at: self.confirmed_at,
            created_at: self.created_at,
        }
    }
}

fn apply_order_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a PurchaseOrderListFilters,
) {
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and po.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and po.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and po.purchase_order_status = ");
        qb.push_bind(status);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and po.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and po.created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_receipt_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a PurchaseReceiptListFilters,
) {
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and po.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and r.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and r.receiving_status = ");
        qb.push_bind(status);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and r.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and r.created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_return_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a PurchaseReturnListFilters,
) {
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and po.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and pr.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and pr.status = ");
        qb.push_bind(status);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and pr.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and pr.created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_ap_doc_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a ApDocListFilters) {
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(" and ap.supplier_id = ");
        qb.push_bind(supplier_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and ap.status = ");
        qb.push_bind(status);
    }
}

async fn insert_order_items(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    po_id: uuid::Uuid,
    items: &[PurchaseOrderItemInput],
) -> anyhow::Result<()> {
    for item in items {
        if item.qty <= 0 {
            return Err(anyhow!("qty must be positive"));
        }
        if item.price_cent < 0 {
            return Err(anyhow!("price_cent must be >= 0"));
        }
        sqlx::query(
            r#"
            insert into purchase_order_items(
                tenant_id, po_id, sku_id, qty, price_cent, received_qty
            ) values ($1,$2,$3,$4,$5,0)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(po_id)
        .bind(item.sku_id)
        .bind(item.qty)
        .bind(item.price_cent)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn fetch_order_items(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    po_id: uuid::Uuid,
) -> anyhow::Result<Vec<PurchaseOrderItemInput>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        sku_id: uuid::Uuid,
        qty: i64,
        price_cent: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        select sku_id, qty, price_cent
        from purchase_order_items
        where tenant_id = $1 and po_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(po_id)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| PurchaseOrderItemInput {
            sku_id: row.sku_id,
            qty: row.qty,
            price_cent: row.price_cent,
        })
        .collect())
}

fn compute_total(items: &[PurchaseOrderItemInput]) -> anyhow::Result<i64> {
    let mut total: i64 = 0;
    for item in items {
        let line_total = item
            .qty
            .checked_mul(item.price_cent)
            .ok_or_else(|| anyhow!("total overflow"))?;
        total = total
            .checked_add(line_total)
            .ok_or_else(|| anyhow!("total overflow"))?;
    }
    Ok(total)
}

async fn fetch_safety_stock(pool: &PgPool, tenant_id: TenantId) -> anyhow::Result<i64> {
    let value: Option<serde_json::Value> = sqlx::query_scalar(
        r#"
        select value_json
        from settings
        where tenant_id = $1
          and scope_type = 'TENANT'
          and scope_id is null
          and key = 'inventory.safety_stock'
        order by updated_at desc
        limit 1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .fetch_optional(pool)
    .await?;

    Ok(extract_numeric_setting(value).unwrap_or(0))
}

async fn fetch_procurement_approval_required(
    pool: &PgPool,
    tenant_id: TenantId,
) -> anyhow::Result<bool> {
    let value: Option<serde_json::Value> = sqlx::query_scalar(
        r#"
        select value_json
        from settings
        where tenant_id = $1
          and scope_type = 'TENANT'
          and scope_id is null
          and key = $2
        order by updated_at desc
        limit 1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(PROCUREMENT_APPROVAL_SETTING_KEY)
    .fetch_optional(pool)
    .await?;

    Ok(extract_bool_setting(value).unwrap_or(false))
}

async fn fetch_sales_window_days(pool: &PgPool, tenant_id: TenantId) -> anyhow::Result<i64> {
    let value: Option<serde_json::Value> = sqlx::query_scalar(
        r#"
        select value_json
        from settings
        where tenant_id = $1
          and scope_type = 'TENANT'
          and scope_id is null
          and key = $2
        order by updated_at desc
        limit 1
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(PROCUREMENT_SALES_WINDOW_SETTING_KEY)
    .fetch_optional(pool)
    .await?;

    let days = extract_numeric_setting(value).unwrap_or(DEFAULT_SALES_WINDOW_DAYS);
    Ok(days.max(1))
}

fn extract_numeric_setting(value: Option<serde_json::Value>) -> Option<i64> {
    let value = value?;
    if let Some(num) = value.as_i64() {
        return Some(num);
    }
    if let Some(obj) = value.as_object() {
        if let Some(v) = obj.get("value").and_then(|v| v.as_i64()) {
            return Some(v);
        }
    }
    None
}

fn extract_bool_setting(value: Option<serde_json::Value>) -> Option<bool> {
    let value = value?;
    if let Some(v) = value.as_bool() {
        return Some(v);
    }
    if let Some(num) = value.as_i64() {
        return Some(num != 0);
    }
    if let Some(text) = value.as_str() {
        let normalized = text.trim().to_ascii_lowercase();
        return match normalized.as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        };
    }
    if let Some(obj) = value.as_object() {
        if let Some(v) = obj.get("value") {
            return extract_bool_setting(Some(v.clone()));
        }
    }
    None
}

fn apply_suggestion_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a PurchaseSuggestionListFilters,
) {
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and ps.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(reason) = &filters.reason {
        qb.push(" and ps.reason = ");
        qb.push_bind(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::{PurchaseOrderItemInput, compute_total};

    #[test]
    fn compute_total_sums_items() {
        let items = vec![
            PurchaseOrderItemInput {
                sku_id: uuid::Uuid::new_v4(),
                qty: 2,
                price_cent: 150,
            },
            PurchaseOrderItemInput {
                sku_id: uuid::Uuid::new_v4(),
                qty: 3,
                price_cent: 200,
            },
        ];
        let total = compute_total(&items).expect("total should compute");
        assert_eq!(total, 2 * 150 + 3 * 200);
    }
}
