use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuCostLedger {
    pub ledger_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub previous_cost_cent: Option<i64>,
    pub new_cost_cent: i64,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct SkuCostLedgerListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct SkuCostLedgerListResult {
    pub items: Vec<SkuCostLedger>,
    pub total: i64,
}

pub struct SkuCostAdjustInput {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub new_cost_cent: i64,
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
}

pub struct SkuCostsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SkuCostsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_ledgers(
        &self,
        tenant_id: TenantId,
        filters: SkuCostLedgerListFilters,
    ) -> anyhow::Result<SkuCostLedgerListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from sku_cost_ledgers where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(sku_id) = filters.sku_id {
            count_qb.push(" and sku_id = ");
            count_qb.push_bind(sku_id);
        }
        if let Some(start_at) = filters.start_at {
            count_qb.push(" and created_at >= ");
            count_qb.push_bind(start_at);
        }
        if let Some(end_at) = filters.end_at {
            count_qb.push(" and created_at <= ");
            count_qb.push_bind(end_at);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select ledger_id, sku_id, previous_cost_cent, new_cost_cent,
                   ref_type, ref_id, created_at
            from sku_cost_ledgers
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(sku_id) = filters.sku_id {
            qb.push(" and sku_id = ");
            qb.push_bind(sku_id);
        }
        if let Some(start_at) = filters.start_at {
            qb.push(" and created_at >= ");
            qb.push_bind(start_at);
        }
        if let Some(end_at) = filters.end_at {
            qb.push(" and created_at <= ");
            qb.push_bind(end_at);
        }
        qb.push(" order by created_at desc, ledger_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        #[derive(sqlx::FromRow)]
        struct Row {
            ledger_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            previous_cost_cent: Option<i64>,
            new_cost_cent: i64,
            ref_type: Option<String>,
            ref_id: Option<String>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| SkuCostLedger {
                ledger_id: row.ledger_id,
                tenant_id,
                sku_id: row.sku_id,
                previous_cost_cent: row.previous_cost_cent,
                new_cost_cent: row.new_cost_cent,
                ref_type: row.ref_type,
                ref_id: row.ref_id,
                created_at: row.created_at,
            })
            .collect();

        Ok(SkuCostLedgerListResult { items, total })
    }

    pub async fn adjust_cost(&self, input: SkuCostAdjustInput) -> anyhow::Result<SkuCostLedger> {
        if input.new_cost_cent < 0 {
            return Err(anyhow::anyhow!("new_cost_cent must be >= 0"));
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let previous_cost: Option<i64> = sqlx::query_scalar(
            r#"
            select cost_cent
            from skus
            where tenant_id = $1 and sku_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("sku not found"))?;

        sqlx::query(
            r#"
            update skus
            set cost_cent = $3,
                updated_at = now()
            where tenant_id = $1 and sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(input.new_cost_cent)
        .execute(&mut *tx)
        .await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            ledger_id: uuid::Uuid,
            previous_cost_cent: Option<i64>,
            new_cost_cent: i64,
            ref_type: Option<String>,
            ref_id: Option<String>,
            created_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into sku_cost_ledgers(
                tenant_id, sku_id, previous_cost_cent, new_cost_cent, ref_type, ref_id
            ) values ($1,$2,$3,$4,$5,$6)
            returning ledger_id, previous_cost_cent, new_cost_cent, ref_type, ref_id, created_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(previous_cost)
        .bind(input.new_cost_cent)
        .bind(input.ref_type.clone())
        .bind(input.ref_id.clone())
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(SkuCostLedger {
            ledger_id: row.ledger_id,
            tenant_id: input.tenant_id,
            sku_id: input.sku_id,
            previous_cost_cent: row.previous_cost_cent,
            new_cost_cent: row.new_cost_cent,
            ref_type: row.ref_type,
            ref_id: row.ref_id,
            created_at: row.created_at,
        })
    }
}
