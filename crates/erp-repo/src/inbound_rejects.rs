use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundReject {
    pub reject_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub doc_id: uuid::Uuid,
    pub doc_no: String,
    pub doc_type: String,
    pub warehouse_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub qty: i64,
    pub reason: Option<String>,
    pub operator_id: Option<UserId>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct InboundRejectCreateItem {
    pub sku_id: uuid::Uuid,
    pub qty: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InboundRejectCreateInput {
    pub tenant_id: TenantId,
    pub doc_id: uuid::Uuid,
    pub operator_id: Option<UserId>,
    pub items: Vec<InboundRejectCreateItem>,
}

#[derive(Debug, Clone)]
pub struct InboundRejectListFilters {
    pub doc_no_keyword: Option<String>,
    pub sku_id: Option<uuid::Uuid>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct InboundRejectListResult {
    pub items: Vec<InboundReject>,
    pub total: i64,
}

pub struct InboundRejectsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InboundRejectsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_rejects(&self, input: InboundRejectCreateInput) -> anyhow::Result<i64> {
        if input.items.is_empty() {
            return Err(anyhow!("reject items required"));
        }
        for item in &input.items {
            if item.qty <= 0 {
                return Err(anyhow!("reject qty must be positive"));
            }
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into inbound_rejects(
                tenant_id, doc_id, sku_id, qty, reason, operator_id
            )
            "#,
        );
        qb.push_values(input.items.iter(), |mut b, item| {
            b.push_bind(uuid::Uuid::from(input.tenant_id))
                .push_bind(input.doc_id)
                .push_bind(item.sku_id)
                .push_bind(item.qty)
                .push_bind(&item.reason)
                .push_bind(input.operator_id.map(uuid::Uuid::from));
        });
        qb.push(" returning reject_id");

        let rows: Vec<uuid::Uuid> = qb.build_query_scalar().fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(rows.len() as i64)
    }

    pub async fn list_rejects(
        &self,
        tenant_id: TenantId,
        filters: InboundRejectListFilters,
    ) -> anyhow::Result<InboundRejectListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from inbound_rejects r
            join inbound_docs d
              on d.doc_id = r.doc_id
             and d.tenant_id = r.tenant_id
            where r.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_reject_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                r.reject_id,
                r.doc_id,
                r.sku_id,
                r.qty,
                r.reason,
                r.operator_id,
                r.created_at,
                d.doc_no,
                d.doc_type,
                d.warehouse_id,
                s.sku_code,
                s.barcode
            from inbound_rejects r
            join inbound_docs d
              on d.doc_id = r.doc_id
             and d.tenant_id = r.tenant_id
            left join skus s
              on s.sku_id = r.sku_id
             and s.tenant_id = r.tenant_id
            where r.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_reject_filters(&mut qb, &filters);
        qb.push(" order by r.created_at desc, r.reject_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            reject_id: uuid::Uuid,
            doc_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            qty: i64,
            reason: Option<String>,
            operator_id: Option<uuid::Uuid>,
            created_at: DateTime<Utc>,
            doc_no: String,
            doc_type: String,
            warehouse_id: uuid::Uuid,
            sku_code: Option<String>,
            barcode: Option<String>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| InboundReject {
                reject_id: row.reject_id,
                tenant_id,
                doc_id: row.doc_id,
                doc_no: row.doc_no,
                doc_type: row.doc_type,
                warehouse_id: row.warehouse_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                barcode: row.barcode,
                qty: row.qty,
                reason: row.reason,
                operator_id: row.operator_id.map(UserId),
                created_at: row.created_at,
            })
            .collect();

        Ok(InboundRejectListResult { items, total })
    }
}

fn apply_reject_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a InboundRejectListFilters,
) {
    if let Some(keyword) = &filters.doc_no_keyword {
        qb.push(" and d.doc_no ilike ");
        qb.push_bind(format!("%{}%", keyword));
    }
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and r.sku_id = ");
        qb.push_bind(sku_id);
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
