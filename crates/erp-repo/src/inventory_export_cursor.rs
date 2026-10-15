//! Bounded inventory export from one reconciled PostgreSQL snapshot.
use crate::inventory_export_jobs::{InventoryExportSpec, MAX_EXPORT_ROWS};
use crate::inventory_reads::{
    InventoryReadQuantities, QuantityBounds, ReadScope, exact_quantities, quantity_filters,
    require_reconciled, source_filters,
};
use chrono::{DateTime, Utc};
use erp_domain::quantity::parse_qty;
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct InventoryExportRow {
    pub sku_id: Uuid,
    pub sku_code: Option<String>,
    pub warehouse_id: Uuid,
    pub on_hand: String,
    pub reserved: String,
    pub frozen: String,
    pub in_transit: String,
}

impl InventoryExportRow {
    pub fn quantities(&self) -> anyhow::Result<InventoryReadQuantities> {
        exact_quantities(
            &self.on_hand,
            &self.reserved,
            &self.frozen,
            &self.in_transit,
        )
    }
}

pub struct InventoryExportCursor {
    tx: Option<Transaction<'static, Postgres>>,
    cursor: String,
    total: u64,
    snapshot_at: DateTime<Utc>,
}

fn predicates<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    tenant: TenantId,
    spec: &'a InventoryExportSpec,
) -> anyhow::Result<()> {
    qb.push(" from inventory_balances ib left join skus s on s.tenant_id=ib.tenant_id and s.sku_id=ib.sku_id where ib.tenant_id=").push_bind(Uuid::from(tenant));
    source_filters(qb, &scope(spec));
    let parse = |v: &Option<String>| {
        v.as_deref()
            .map(parse_qty)
            .transpose()
            .map(|value| value.map(|qty| i128::from(qty.as_micros())))
    };
    quantity_filters(
        qb,
        "ib.on_hand_micros",
        "(ib.on_hand_micros::numeric - ib.reserved_micros::numeric - ib.frozen_micros::numeric)",
        QuantityBounds::new(
            false,
            None,
            parse(&spec.query.available_min)?,
            parse(&spec.query.available_max)?,
        ),
    )
}

fn scope(spec: &InventoryExportSpec) -> ReadScope<'_> {
    ReadScope {
        warehouse: spec.query.warehouse_id,
        warehouses: Some(&spec.warehouse_ids),
        sku: spec.query.sku_id,
        code: spec.query.sku_code.as_deref(),
        barcode: spec.query.barcode.as_deref(),
        tag: spec.query.tag.as_deref(),
        search: None,
    }
}

impl InventoryExportCursor {
    pub async fn open(
        pool: &PgPool,
        tenant: TenantId,
        spec: &InventoryExportSpec,
    ) -> anyhow::Result<Self> {
        let mut tx = pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read, read only")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local statement_timeout='10s'")
            .execute(&mut *tx)
            .await?;
        sqlx::query("set local idle_in_transaction_session_timeout='30s'")
            .execute(&mut *tx)
            .await?;
        let snapshot_at = sqlx::query_scalar("select statement_timestamp()")
            .fetch_one(&mut *tx)
            .await?;
        require_reconciled(&mut tx, tenant, &scope(spec)).await?;
        let mut text_check = QueryBuilder::<Postgres>::new("select count(*)");
        predicates(&mut text_check, tenant, spec)?;
        text_check.push(" and octet_length(s.sku_code)>1024");
        let oversized: i64 = text_check.build_query_scalar().fetch_one(&mut *tx).await?;
        if oversized != 0 {
            anyhow::bail!("inventory export SKU code exceeds 1024 UTF-8 bytes");
        }
        let mut count = QueryBuilder::<Postgres>::new("select count(*) from (select 1");
        predicates(&mut count, tenant, spec)?;
        count
            .push(" limit ")
            .push_bind((MAX_EXPORT_ROWS + 1) as i64)
            .push(") bounded_inventory_export");
        let total: i64 = count.build_query_scalar().fetch_one(&mut *tx).await?;
        let total = u64::try_from(total)?;
        if total > MAX_EXPORT_ROWS {
            anyhow::bail!("inventory export exceeds {MAX_EXPORT_ROWS} rows");
        }
        let cursor = format!("inventory_export_{}", Uuid::new_v4().simple());
        let mut declaration = QueryBuilder::<Postgres>::new("declare ");
        declaration.push(&cursor).push(" no scroll cursor for select ib.sku_id,s.sku_code,ib.warehouse_id,ib.on_hand_micros::text as on_hand,ib.reserved_micros::text as reserved,ib.frozen_micros::text as frozen,ib.in_transit_micros::text as in_transit");
        predicates(&mut declaration, tenant, spec)?;
        declaration.push(" order by ib.warehouse_id,ib.sku_id");
        declaration.build().execute(&mut *tx).await?;
        Ok(Self {
            tx: Some(tx),
            cursor,
            total,
            snapshot_at,
        })
    }
    pub fn total(&self) -> u64 {
        self.total
    }
    pub fn snapshot_at(&self) -> DateTime<Utc> {
        self.snapshot_at
    }
    pub async fn fetch_next(&mut self, rows: u16) -> anyhow::Result<Vec<InventoryExportRow>> {
        if !(1..=1000).contains(&rows) {
            anyhow::bail!("inventory export fetch size must be 1..1000");
        }
        let tx = self
            .tx
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("inventory export cursor closed"))?;
        let query = format!("fetch forward {rows} from {}", self.cursor);
        Ok(sqlx::query_as(&query).fetch_all(&mut **tx).await?)
    }
    pub async fn finish(mut self) -> anyhow::Result<()> {
        if let Some(tx) = self.tx.take() {
            tx.rollback().await?;
        }
        Ok(())
    }
}
