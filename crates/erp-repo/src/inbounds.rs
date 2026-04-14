use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use crate::inventory::{InventoryDelta, InventoryService};

pub const INBOUND_STATUS_DRAFT: &str = "DRAFT";
pub const INBOUND_STATUS_AUDITED: &str = "AUDITED";
pub const INBOUND_STATUS_RECEIVING: &str = "RECEIVING";
pub const INBOUND_STATUS_PUTAWAY_DONE: &str = "PUTAWAY_DONE";
pub const INBOUND_STATUS_CANCELLED: &str = "CANCELLED";

pub const INBOUND_ACTION_RECEIVE: &str = "RECEIVE";
pub const INBOUND_ACTION_PUTAWAY: &str = "PUTAWAY";
pub const INBOUND_ACTION_INSPECT: &str = "INSPECT";

pub const INBOUND_LEDGER_KIND: &str = "PURCHASE_RECEIPT";
pub const INBOUND_REF_TYPE: &str = "INBOUND_DOC";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundDoc {
    pub doc_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub doc_type: String,
    pub doc_no: String,
    pub status: String,
    pub inbound_status: String,
    pub receipt_status: String,
    pub inbound_qc_status: String,
    pub putaway_status: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InboundDocItem {
    pub doc_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub planned_qty: i64,
    pub received_qty: i64,
    pub putaway_qty: i64,
}

#[derive(Debug, Clone)]
pub struct InboundDocSummary {
    pub doc_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub doc_type: String,
    pub doc_no: String,
    pub status: String,
    pub inbound_status: String,
    pub receipt_status: String,
    pub inbound_qc_status: String,
    pub putaway_status: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub supplier_id: Option<uuid::Uuid>,
    pub supplier_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundDocDetail {
    pub doc: InboundDoc,
    pub items: Vec<InboundDocItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundInspection {
    pub inspection_id: uuid::Uuid,
    pub doc_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub result: String,
    pub note: Option<String>,
    pub operator_id: Option<uuid::Uuid>,
    pub operation_id: uuid::Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InboundDocItemCreate {
    pub sku_id: uuid::Uuid,
    pub planned_qty: i64,
}

#[derive(Debug, Clone)]
pub struct InboundDocCreate {
    pub doc_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub doc_type: String,
    pub doc_no: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub items: Vec<InboundDocItemCreate>,
}

#[derive(Debug, Clone)]
pub struct InboundDocListFilters {
    pub status: Option<String>,
    pub inbound_status: Option<String>,
    pub receipt_status: Option<String>,
    pub inbound_qc_status: Option<String>,
    pub putaway_status: Option<String>,
    pub doc_type: Option<String>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub doc_no: Option<String>,
    pub doc_no_keyword: Option<String>,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub supplier_id: Option<uuid::Uuid>,
    pub sku_id: Option<uuid::Uuid>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct InboundDocListResult {
    pub items: Vec<InboundDocSummary>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct InboundReceiveItem {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub operation_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct InboundReceiveInput {
    pub tenant_id: TenantId,
    pub doc_id: uuid::Uuid,
    pub items: Vec<InboundReceiveItem>,
}

#[derive(Debug, Clone)]
pub struct InboundInspectItem {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub result: String,
    pub note: Option<String>,
    pub operation_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct InboundInspectInput {
    pub tenant_id: TenantId,
    pub doc_id: uuid::Uuid,
    pub items: Vec<InboundInspectItem>,
    pub operator_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct InboundPutawayItem {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub operation_id: uuid::Uuid,
    pub from_area: Option<String>,
    pub to_location_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct InboundPutawayInput {
    pub tenant_id: TenantId,
    pub doc_id: uuid::Uuid,
    pub items: Vec<InboundPutawayItem>,
}

pub struct InboundsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InboundsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_doc(&self, input: InboundDocCreate) -> anyhow::Result<InboundDocDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("inbound items required"));
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        sqlx::query(
            r#"
            insert into inbound_docs(
                doc_id, tenant_id, warehouse_id, doc_type, doc_no, status,
                inbound_status, receipt_status, inbound_qc_status, putaway_status,
                ref_type, ref_id
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(input.doc_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(&input.doc_type)
        .bind(&input.doc_no)
        .bind(INBOUND_STATUS_DRAFT)
        .bind("DRAFT")
        .bind("NOT_RECEIVED")
        .bind("NOT_REQUIRED")
        .bind("NOT_PUTAWAY")
        .bind(&input.ref_type)
        .bind(&input.ref_id)
        .execute(&mut *tx)
        .await?;

        for item in &input.items {
            if item.planned_qty < 0 {
                return Err(anyhow!("planned_qty must be >= 0"));
            }
            sqlx::query(
                r#"
                insert into inbound_doc_items(
                    tenant_id, doc_id, sku_id, planned_qty
                ) values ($1,$2,$3,$4)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(item.sku_id)
            .bind(item.planned_qty)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get_detail(input.tenant_id, input.doc_id)
            .await?
            .ok_or_else(|| anyhow!("inbound doc not found"))
    }

    pub async fn has_unpassed_inspections(
        &self,
        tenant_id: TenantId,
        doc_id: uuid::Uuid,
    ) -> anyhow::Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from inbound_doc_items i
            left join lateral (
                select result
                from inbound_inspections ins
                where ins.tenant_id = i.tenant_id
                  and ins.doc_id = i.doc_id
                  and ins.sku_id = i.sku_id
                order by ins.created_at desc, ins.inspection_id desc
                limit 1
            ) latest on true
            where i.tenant_id = $1
              and i.doc_id = $2
              and (latest.result is null or latest.result <> 'PASSED')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(doc_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count > 0)
    }

    pub async fn list_docs(
        &self,
        tenant_id: TenantId,
        filters: InboundDocListFilters,
    ) -> anyhow::Result<InboundDocListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from inbound_docs d
            where d.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_inbound_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                d.doc_id,
                d.warehouse_id,
                d.doc_type,
                d.doc_no,
                d.status,
                d.inbound_status,
                d.receipt_status,
                d.inbound_qc_status,
                d.putaway_status,
                d.ref_type,
                d.ref_id,
                d.created_at,
                d.updated_at,
                s.supplier_id,
                s.name as supplier_name
            from inbound_docs d
            left join purchase_receipts pr
              on pr.tenant_id = d.tenant_id
             and d.ref_type = 'PURCHASE_RECEIPT'
             and pr.receipt_id::text = d.ref_id
            left join purchase_orders po
              on po.tenant_id = pr.tenant_id
             and po.po_id = pr.po_id
            left join suppliers s
              on s.tenant_id = po.tenant_id
             and s.supplier_id = po.supplier_id
            where d.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_inbound_filters(&mut qb, &filters);
        qb.push(" order by d.created_at desc, d.doc_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            doc_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            doc_type: String,
            doc_no: String,
            status: String,
            inbound_status: String,
            receipt_status: String,
            inbound_qc_status: String,
            putaway_status: String,
            ref_type: Option<String>,
            ref_id: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            supplier_id: Option<uuid::Uuid>,
            supplier_name: Option<String>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| InboundDocSummary {
                doc_id: row.doc_id,
                tenant_id,
                warehouse_id: row.warehouse_id,
                doc_type: row.doc_type,
                doc_no: row.doc_no,
                status: row.status,
                inbound_status: row.inbound_status,
                receipt_status: row.receipt_status,
                inbound_qc_status: row.inbound_qc_status,
                putaway_status: row.putaway_status,
                ref_type: row.ref_type,
                ref_id: row.ref_id,
                supplier_id: row.supplier_id,
                supplier_name: row.supplier_name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(InboundDocListResult { items, total })
    }

    pub async fn get_detail(
        &self,
        tenant_id: TenantId,
        doc_id: uuid::Uuid,
    ) -> anyhow::Result<Option<InboundDocDetail>> {
        let doc = self.get_doc(tenant_id, doc_id, self.pool).await?;
        let Some(doc) = doc else {
            return Ok(None);
        };

        let items: Vec<InboundDocItem> = sqlx::query_as(
            r#"
            select
                i.doc_id,
                i.sku_id,
                s.sku_code,
                s.barcode,
                i.planned_qty,
                i.received_qty,
                i.putaway_qty
            from inbound_doc_items i
            left join skus s on s.sku_id = i.sku_id
            where i.tenant_id = $1 and i.doc_id = $2
            order by i.sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(doc_id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(InboundDocDetail { doc, items }))
    }

    pub async fn list_inspections(
        &self,
        tenant_id: TenantId,
        doc_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<InboundInspection>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            inspection_id: uuid::Uuid,
            doc_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            qty: i64,
            result: String,
            note: Option<String>,
            operator_id: Option<uuid::Uuid>,
            operation_id: uuid::Uuid,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select inspection_id, doc_id, sku_id, qty, result, note, operator_id, operation_id, created_at
            from inbound_inspections
            where tenant_id = $1 and doc_id = $2
            order by created_at desc, inspection_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(doc_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| InboundInspection {
                inspection_id: row.inspection_id,
                doc_id: row.doc_id,
                sku_id: row.sku_id,
                qty: row.qty,
                result: row.result,
                note: row.note,
                operator_id: row.operator_id,
                operation_id: row.operation_id,
                created_at: row.created_at,
            })
            .collect())
    }

    pub async fn audit_doc(
        &self,
        tenant_id: TenantId,
        doc_id: uuid::Uuid,
    ) -> anyhow::Result<InboundDocDetail> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let doc = self.get_doc_for_update(tenant_id, doc_id, &mut tx).await?;
        let Some(doc) = doc else {
            return Err(anyhow!("inbound doc not found"));
        };

        if doc.status == INBOUND_STATUS_CANCELLED {
            return Err(anyhow!("inbound doc cancelled"));
        }
        if doc.status == INBOUND_STATUS_PUTAWAY_DONE {
            return Err(anyhow!("inbound doc already putaway done"));
        }
        if doc.status == INBOUND_STATUS_AUDITED {
            tx.commit().await?;
            return self
                .get_detail(tenant_id, doc_id)
                .await?
                .ok_or_else(|| anyhow!("inbound doc not found"));
        }
        if doc.status != INBOUND_STATUS_DRAFT {
            return Err(anyhow!("inbound doc not auditable"));
        }

        sqlx::query(
            r#"
            update inbound_docs
            set status = $3,
                inbound_status = 'PENDING_RECEIPT',
                updated_at = now()
            where tenant_id = $1 and doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(doc_id)
        .bind(INBOUND_STATUS_AUDITED)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_detail(tenant_id, doc_id)
            .await?
            .ok_or_else(|| anyhow!("inbound doc not found"))
    }

    pub async fn receive(&self, input: InboundReceiveInput) -> anyhow::Result<InboundDocDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("receive items required"));
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let doc = self
            .get_doc_for_update(input.tenant_id, input.doc_id, &mut tx)
            .await?;
        let Some(doc) = doc else {
            return Err(anyhow!("inbound doc not found"));
        };
        if doc.status == INBOUND_STATUS_CANCELLED || doc.status == INBOUND_STATUS_PUTAWAY_DONE {
            return Err(anyhow!("inbound doc not receivable"));
        }

        let mut any_applied = false;

        for item in &input.items {
            if item.qty <= 0 {
                return Err(anyhow!("received qty must be positive"));
            }

            let inserted = insert_inbound_op(
                &mut tx,
                input.tenant_id,
                item.operation_id,
                input.doc_id,
                item.sku_id,
                INBOUND_ACTION_RECEIVE,
                item.qty,
            )
            .await?;

            if !inserted {
                continue;
            }
            any_applied = true;

            #[derive(sqlx::FromRow)]
            struct ItemRow {
                planned_qty: i64,
                received_qty: i64,
                putaway_qty: i64,
            }

            let row: ItemRow = sqlx::query_as(
                r#"
                select planned_qty, received_qty, putaway_qty
                from inbound_doc_items
                where tenant_id = $1 and doc_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("inbound item not found"))?;

            let new_received = row
                .received_qty
                .checked_add(item.qty)
                .ok_or_else(|| anyhow!("received qty overflow"))?;
            if row.planned_qty > 0 && new_received > row.planned_qty {
                return Err(anyhow!("received qty exceeds planned"));
            }
            if new_received < row.putaway_qty {
                return Err(anyhow!("received qty below putaway qty"));
            }

            sqlx::query(
                r#"
                update inbound_doc_items
                set received_qty = $4,
                    updated_at = now()
                where tenant_id = $1 and doc_id = $2 and sku_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(item.sku_id)
            .bind(new_received)
            .execute(&mut *tx)
            .await?;
        }

        if any_applied {
            #[derive(sqlx::FromRow)]
            struct ReceiptSummary {
                total_planned: i64,
                total_received: i64,
                received_items: i64,
            }

            let summary: ReceiptSummary = sqlx::query_as(
                r#"
                select
                    coalesce(sum(planned_qty), 0)::bigint as total_planned,
                    coalesce(sum(received_qty), 0)::bigint as total_received,
                    (count(*) filter (where received_qty > 0))::bigint as received_items
                from inbound_doc_items
                where tenant_id = $1 and doc_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .fetch_one(&mut *tx)
            .await?;

            let receipt_status = if summary.total_received == 0 && summary.received_items == 0 {
                "NOT_RECEIVED"
            } else if summary.total_planned > 0 && summary.total_received < summary.total_planned {
                "PART_RECEIVED"
            } else {
                "RECEIVED"
            };
            let inbound_status = if receipt_status == "RECEIVED" {
                if doc.inbound_qc_status == "PASSED" || doc.inbound_qc_status == "NOT_REQUIRED" {
                    "PENDING_PUTAWAY"
                } else {
                    "QC_PENDING"
                }
            } else if receipt_status == "PART_RECEIVED" {
                "RECEIVING"
            } else {
                "PENDING_RECEIPT"
            };

            sqlx::query(
                r#"
                update inbound_docs
                set status = $3,
                    inbound_status = $4,
                    receipt_status = $5,
                    updated_at = now()
                where tenant_id = $1
                  and doc_id = $2
                  and status = any($6)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(INBOUND_STATUS_RECEIVING)
            .bind(inbound_status)
            .bind(receipt_status)
            .bind(vec![
                INBOUND_STATUS_DRAFT,
                INBOUND_STATUS_AUDITED,
                INBOUND_STATUS_RECEIVING,
            ])
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get_detail(input.tenant_id, input.doc_id)
            .await?
            .ok_or_else(|| anyhow!("inbound doc not found"))
    }

    pub async fn inspect(&self, input: InboundInspectInput) -> anyhow::Result<()> {
        if input.items.is_empty() {
            return Err(anyhow!("inspection items required"));
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let doc = self
            .get_doc_for_update(input.tenant_id, input.doc_id, &mut tx)
            .await?;
        let Some(doc) = doc else {
            return Err(anyhow!("inbound doc not found"));
        };
        if doc.status == INBOUND_STATUS_CANCELLED || doc.status == INBOUND_STATUS_PUTAWAY_DONE {
            return Err(anyhow!("inbound doc not inspectable"));
        }

        for item in &input.items {
            if item.qty <= 0 {
                return Err(anyhow!("inspection qty must be positive"));
            }
            let exists: Option<i64> = sqlx::query_scalar(
                r#"
                select 1
                from inbound_doc_items
                where tenant_id = $1 and doc_id = $2 and sku_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?;
            if exists.is_none() {
                return Err(anyhow!("inbound item not found"));
            }
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into inbound_inspections(
                tenant_id, doc_id, sku_id, qty, result, note, operator_id, operation_id
            )
            "#,
        );
        qb.push_values(input.items.iter(), |mut b, item| {
            b.push_bind(uuid::Uuid::from(input.tenant_id))
                .push_bind(input.doc_id)
                .push_bind(item.sku_id)
                .push_bind(item.qty)
                .push_bind(&item.result)
                .push_bind(item.note.as_deref())
                .push_bind(input.operator_id)
                .push_bind(item.operation_id);
        });
        qb.push(" on conflict (tenant_id, operation_id) do nothing");
        qb.build().execute(&mut *tx).await?;

        #[derive(sqlx::FromRow)]
        struct InspectionSummary {
            failed: i64,
            passed: i64,
            total: i64,
        }

        let summary: InspectionSummary = sqlx::query_as(
            r#"
            select
                count(*) filter (where latest.result = 'FAILED') as failed,
                count(*) filter (where latest.result = 'PASSED') as passed,
                count(*) as total
            from inbound_doc_items i
            left join lateral (
                select result
                from inbound_inspections ins
                where ins.tenant_id = i.tenant_id
                  and ins.doc_id = i.doc_id
                  and ins.sku_id = i.sku_id
                order by ins.created_at desc, ins.inspection_id desc
                limit 1
            ) latest on true
            where i.tenant_id = $1 and i.doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.doc_id)
        .fetch_one(&mut *tx)
        .await?;

        let inbound_qc_status = if summary.failed > 0 {
            "FAILED"
        } else if summary.total > 0 && summary.passed == summary.total {
            "PASSED"
        } else {
            "PENDING"
        };

        let inbound_status = if doc.receipt_status == "RECEIVED" {
            if inbound_qc_status == "PASSED" || inbound_qc_status == "NOT_REQUIRED" {
                "PENDING_PUTAWAY"
            } else {
                "QC_PENDING"
            }
        } else if doc.receipt_status == "PART_RECEIVED" {
            "RECEIVING"
        } else {
            "PENDING_RECEIPT"
        };

        sqlx::query(
            r#"
            update inbound_docs
            set inbound_qc_status = $3,
                inbound_status = $4,
                updated_at = now()
            where tenant_id = $1 and doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.doc_id)
        .bind(inbound_qc_status)
        .bind(inbound_status)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn putaway(&self, input: InboundPutawayInput) -> anyhow::Result<InboundDocDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("putaway items required"));
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let doc = self
            .get_doc_for_update(input.tenant_id, input.doc_id, &mut tx)
            .await?;
        let Some(doc) = doc else {
            return Err(anyhow!("inbound doc not found"));
        };
        if doc.status == INBOUND_STATUS_CANCELLED {
            return Err(anyhow!("inbound doc cancelled"));
        }

        let allow_updates = doc.status != INBOUND_STATUS_PUTAWAY_DONE;
        let mut any_applied = false;

        for item in &input.items {
            if item.qty <= 0 {
                return Err(anyhow!("putaway qty must be positive"));
            }

            let inserted = insert_inbound_op(
                &mut tx,
                input.tenant_id,
                item.operation_id,
                input.doc_id,
                item.sku_id,
                INBOUND_ACTION_PUTAWAY,
                item.qty,
            )
            .await?;

            if !inserted {
                continue;
            }

            if !allow_updates {
                return Err(anyhow!("inbound doc already putaway done"));
            }
            any_applied = true;

            #[derive(sqlx::FromRow)]
            struct ItemRow {
                received_qty: i64,
                putaway_qty: i64,
            }

            let row: ItemRow = sqlx::query_as(
                r#"
                select received_qty, putaway_qty
                from inbound_doc_items
                where tenant_id = $1 and doc_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("inbound item not found"))?;

            let new_putaway = row
                .putaway_qty
                .checked_add(item.qty)
                .ok_or_else(|| anyhow!("putaway qty overflow"))?;
            if new_putaway > row.received_qty {
                return Err(anyhow!("putaway qty exceeds received"));
            }

            sqlx::query(
                r#"
                update inbound_doc_items
                set putaway_qty = $4,
                    updated_at = now()
                where tenant_id = $1 and doc_id = $2 and sku_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(item.sku_id)
            .bind(new_putaway)
            .execute(&mut *tx)
            .await?;

            apply_inventory_inbound(
                self.pool,
                &mut tx,
                input.tenant_id,
                doc.warehouse_id,
                item.sku_id,
                item.qty,
                input.doc_id,
            )
            .await?;

            sqlx::query(
                r#"
                insert into inbound_putaway_tasks(
                    tenant_id, doc_id, sku_id, qty, from_area, to_location_id, status
                ) values ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(item.sku_id)
            .bind(item.qty)
            .bind(item.from_area.as_deref().unwrap_or("RECEIVING"))
            .bind(item.to_location_id)
            .bind("PUTAWAYED")
            .execute(&mut *tx)
            .await?;
        }

        if any_applied {
            sqlx::query(
                r#"
                update inbound_docs
                set status = $3,
                    inbound_status = 'PUTAWAYING',
                    putaway_status = 'PART_PUTAWAY',
                    updated_at = now()
                where tenant_id = $1
                  and doc_id = $2
                  and status = any($4)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .bind(INBOUND_STATUS_RECEIVING)
            .bind(vec![
                INBOUND_STATUS_DRAFT,
                INBOUND_STATUS_AUDITED,
                INBOUND_STATUS_RECEIVING,
            ])
            .execute(&mut *tx)
            .await?;

            let complete: Option<bool> = sqlx::query_scalar(
                r#"
                select bool_and(received_qty > 0 and putaway_qty >= received_qty)
                from inbound_doc_items
                where tenant_id = $1 and doc_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.doc_id)
            .fetch_optional(&mut *tx)
            .await?;

            if complete.unwrap_or(false) {
                sqlx::query(
                    r#"
                    update inbound_docs
                    set status = $3,
                        inbound_status = 'COMPLETED',
                        putaway_status = 'PUTAWAYED',
                        updated_at = now()
                    where tenant_id = $1 and doc_id = $2
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(input.doc_id)
                .bind(INBOUND_STATUS_PUTAWAY_DONE)
                .execute(&mut *tx)
                .await?;

                if let (Some(ref_type), Some(ref_id)) = (&doc.ref_type, &doc.ref_id) {
                    if ref_type == "AFTER_SALES" {
                        let after_sales_id = uuid::Uuid::parse_str(ref_id)
                            .map_err(|_| anyhow!("bad after_sales ref_id"))?;
                        let now = Utc::now();
                        sqlx::query(
                            r#"
                            update return_receipts
                            set status = $4,
                                putaway_at = $5,
                                updated_at = now()
                            where tenant_id = $1 and after_sales_id = $2
                            "#,
                        )
                        .bind(uuid::Uuid::from(input.tenant_id))
                        .bind(after_sales_id)
                        .bind("PUTAWAY_DONE")
                        .bind(now)
                        .execute(&mut *tx)
                        .await?;

                        sqlx::query(
                            r#"
                            update after_sales
                            set status = $3,
                                updated_at = now()
                            where tenant_id = $1 and id = $2
                            "#,
                        )
                        .bind(uuid::Uuid::from(input.tenant_id))
                        .bind(after_sales_id)
                        .bind("RECEIVED")
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
        }

        tx.commit().await?;
        self.get_detail(input.tenant_id, input.doc_id)
            .await?
            .ok_or_else(|| anyhow!("inbound doc not found"))
    }

    async fn get_doc<'t, T>(
        &self,
        tenant_id: TenantId,
        doc_id: uuid::Uuid,
        executor: T,
    ) -> anyhow::Result<Option<InboundDoc>>
    where
        T: sqlx::Executor<'t, Database = Postgres>,
    {
        #[derive(sqlx::FromRow)]
        struct Row {
            doc_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            doc_type: String,
            doc_no: String,
            status: String,
            inbound_status: String,
            receipt_status: String,
            inbound_qc_status: String,
            putaway_status: String,
            ref_type: Option<String>,
            ref_id: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row = sqlx::query_as::<_, Row>(
            r#"
            select
                doc_id, warehouse_id, doc_type, doc_no, status,
                inbound_status, receipt_status, inbound_qc_status, putaway_status,
                ref_type, ref_id, created_at, updated_at
            from inbound_docs
            where tenant_id = $1 and doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(doc_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.map(|row| InboundDoc {
            doc_id: row.doc_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            doc_type: row.doc_type,
            doc_no: row.doc_no,
            status: row.status,
            inbound_status: row.inbound_status,
            receipt_status: row.receipt_status,
            inbound_qc_status: row.inbound_qc_status,
            putaway_status: row.putaway_status,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn get_doc_for_update(
        &self,
        tenant_id: TenantId,
        doc_id: uuid::Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> anyhow::Result<Option<InboundDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            doc_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            doc_type: String,
            doc_no: String,
            status: String,
            inbound_status: String,
            receipt_status: String,
            inbound_qc_status: String,
            putaway_status: String,
            ref_type: Option<String>,
            ref_id: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row = sqlx::query_as::<_, Row>(
            r#"
            select
                doc_id, warehouse_id, doc_type, doc_no, status,
                inbound_status, receipt_status, inbound_qc_status, putaway_status,
                ref_type, ref_id, created_at, updated_at
            from inbound_docs
            where tenant_id = $1 and doc_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(doc_id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(row.map(|row| InboundDoc {
            doc_id: row.doc_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            doc_type: row.doc_type,
            doc_no: row.doc_no,
            status: row.status,
            inbound_status: row.inbound_status,
            receipt_status: row.receipt_status,
            inbound_qc_status: row.inbound_qc_status,
            putaway_status: row.putaway_status,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

async fn insert_inbound_op(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    operation_id: uuid::Uuid,
    doc_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    action: &str,
    qty: i64,
) -> anyhow::Result<bool> {
    let inserted: Option<uuid::Uuid> = sqlx::query_scalar(
        r#"
        insert into inbound_item_ops(
            tenant_id, operation_id, doc_id, sku_id, action, qty
        ) values ($1,$2,$3,$4,$5,$6)
        on conflict (tenant_id, operation_id) do nothing
        returning operation_id
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(operation_id)
    .bind(doc_id)
    .bind(sku_id)
    .bind(action)
    .bind(qty)
    .fetch_optional(&mut **tx)
    .await?;

    if inserted.is_some() {
        return Ok(true);
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        doc_id: uuid::Uuid,
        sku_id: uuid::Uuid,
        action: String,
        qty: i64,
    }

    let existing: Option<Row> = sqlx::query_as(
        r#"
        select doc_id, sku_id, action, qty
        from inbound_item_ops
        where tenant_id = $1 and operation_id = $2
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(operation_id)
    .fetch_optional(&mut **tx)
    .await?;

    let Some(existing) = existing else {
        return Err(anyhow!("operation lookup failed"));
    };

    if existing.doc_id != doc_id
        || existing.sku_id != sku_id
        || existing.action != action
        || existing.qty != qty
    {
        return Err(anyhow!("operation id conflict"));
    }

    Ok(false)
}

async fn apply_inventory_inbound(
    pool: &PgPool,
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    warehouse_id: uuid::Uuid,
    sku_id: uuid::Uuid,
    qty: i64,
    doc_id: uuid::Uuid,
) -> anyhow::Result<()> {
    let service = InventoryService::new(pool);
    service
        .apply_with_tx(
            tx,
            InventoryDelta {
                tenant_id,
                warehouse_id,
                sku_id,
                delta_on_hand: qty,
                delta_reserved: 0,
                delta_in_transit: 0,
                delta_frozen: 0,
                reason_type: INBOUND_LEDGER_KIND.to_string(),
                ref_type: Some(INBOUND_REF_TYPE.to_string()),
                ref_id: Some(doc_id.to_string()),
                occurred_at: Utc::now(),
            },
        )
        .await?;
    Ok(())
}

fn apply_inbound_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a InboundDocListFilters,
) {
    if let Some(status) = &filters.status {
        qb.push(" and d.status = ");
        qb.push_bind(status);
    }
    if let Some(inbound_status) = &filters.inbound_status {
        qb.push(" and d.inbound_status = ");
        qb.push_bind(inbound_status);
    }
    if let Some(receipt_status) = &filters.receipt_status {
        qb.push(" and d.receipt_status = ");
        qb.push_bind(receipt_status);
    }
    if let Some(inbound_qc_status) = &filters.inbound_qc_status {
        qb.push(" and d.inbound_qc_status = ");
        qb.push_bind(inbound_qc_status);
    }
    if let Some(putaway_status) = &filters.putaway_status {
        qb.push(" and d.putaway_status = ");
        qb.push_bind(putaway_status);
    }
    if let Some(doc_type) = &filters.doc_type {
        qb.push(" and d.doc_type = ");
        qb.push_bind(doc_type);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and d.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(doc_no) = &filters.doc_no {
        qb.push(" and d.doc_no = ");
        qb.push_bind(doc_no);
    }
    if let Some(keyword) = &filters.doc_no_keyword {
        qb.push(" and d.doc_no ilike ");
        qb.push_bind(format!("%{}%", keyword));
    }
    if let Some(ref_type) = &filters.ref_type {
        qb.push(" and d.ref_type = ");
        qb.push_bind(ref_type);
    }
    if let Some(ref_id) = &filters.ref_id {
        qb.push(" and d.ref_id = ");
        qb.push_bind(ref_id);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and d.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and d.created_at <= ");
        qb.push_bind(created_to);
    }
    if let Some(sku_id) = filters.sku_id {
        qb.push(
            " and exists (select 1 from inbound_doc_items i where i.tenant_id = d.tenant_id and i.doc_id = d.doc_id and i.sku_id = ",
        );
        qb.push_bind(sku_id);
        qb.push(")");
    }
    if let Some(supplier_id) = filters.supplier_id {
        qb.push(
            " and exists (select 1 from purchase_receipts pr join purchase_orders po on po.tenant_id = pr.tenant_id and po.po_id = pr.po_id where pr.tenant_id = d.tenant_id and d.ref_type = 'PURCHASE_RECEIPT' and pr.receipt_id::text = d.ref_id and po.supplier_id = ",
        );
        qb.push_bind(supplier_id);
        qb.push(")");
    }
}
