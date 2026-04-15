use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuBatch {
    pub batch_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub batch_no: String,
    pub production_date: Option<DateTime<Utc>>,
    pub expire_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SkuBatchCursor {
    pub created_at: DateTime<Utc>,
    pub batch_id: uuid::Uuid,
}

impl SkuBatchCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.batch_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let batch_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            batch_id,
        })
    }
}

pub struct SkuBatchListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub batch_no: Option<String>,
    pub cursor: Option<SkuBatchCursor>,
    pub limit: i64,
}

pub struct SkuBatchListResult {
    pub items: Vec<SkuBatch>,
    pub next_cursor: Option<SkuBatchCursor>,
}

pub struct SkuBatchCreate {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub batch_no: String,
    pub production_date: Option<DateTime<Utc>>,
    pub expire_at: Option<DateTime<Utc>>,
}

pub struct SkuBatchUpdate {
    pub batch_no: Option<String>,
    pub production_date: Option<DateTime<Utc>>,
    pub expire_at: Option<DateTime<Utc>>,
}

pub struct SkuBatchesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SkuBatchesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_batches(
        &self,
        tenant_id: TenantId,
        filters: SkuBatchListFilters,
    ) -> anyhow::Result<SkuBatchListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select batch_id, sku_id, batch_no, production_date, expire_at, created_at, updated_at
            from sku_batches
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(sku_id) = filters.sku_id {
            qb.push(" and sku_id = ");
            qb.push_bind(sku_id);
        }
        if let Some(batch_no) = filters.batch_no {
            qb.push(" and batch_no = ");
            qb.push_bind(batch_no);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, batch_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.batch_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, batch_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            batch_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            batch_no: String,
            production_date: Option<DateTime<Utc>>,
            expire_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| SkuBatchCursor {
            created_at: row.created_at,
            batch_id: row.batch_id,
        });
        let items = rows
            .into_iter()
            .map(|row| SkuBatch {
                batch_id: row.batch_id,
                tenant_id,
                sku_id: row.sku_id,
                batch_no: row.batch_no,
                production_date: row.production_date,
                expire_at: row.expire_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SkuBatchListResult { items, next_cursor })
    }

    pub async fn get_batch(
        &self,
        tenant_id: TenantId,
        batch_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuBatch>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            batch_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            batch_no: String,
            production_date: Option<DateTime<Utc>>,
            expire_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select batch_id, sku_id, batch_no, production_date, expire_at, created_at, updated_at
            from sku_batches
            where tenant_id = $1 and batch_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(batch_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SkuBatch {
            batch_id: row.batch_id,
            tenant_id,
            sku_id: row.sku_id,
            batch_no: row.batch_no,
            production_date: row.production_date,
            expire_at: row.expire_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_batch(&self, input: SkuBatchCreate) -> anyhow::Result<SkuBatch> {
        #[derive(sqlx::FromRow)]
        struct Row {
            batch_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            batch_no: String,
            production_date: Option<DateTime<Utc>>,
            expire_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into sku_batches(tenant_id, sku_id, batch_no, production_date, expire_at)
            values ($1,$2,$3,$4,$5)
            returning batch_id, sku_id, batch_no, production_date, expire_at, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.batch_no)
        .bind(input.production_date)
        .bind(input.expire_at)
        .fetch_one(self.pool)
        .await?;

        Ok(SkuBatch {
            batch_id: row.batch_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            batch_no: row.batch_no,
            production_date: row.production_date,
            expire_at: row.expire_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_batch(
        &self,
        tenant_id: TenantId,
        batch_id: uuid::Uuid,
        input: SkuBatchUpdate,
    ) -> anyhow::Result<Option<SkuBatch>> {
        let mut qb = QueryBuilder::<Postgres>::new("update sku_batches set ");
        let mut separated = qb.separated(", ");

        if let Some(batch_no) = input.batch_no {
            separated.push("batch_no = ");
            separated.push_bind(batch_no);
        }
        if let Some(production_date) = input.production_date {
            separated.push("production_date = ");
            separated.push_bind(production_date);
        }
        if let Some(expire_at) = input.expire_at {
            separated.push("expire_at = ");
            separated.push_bind(expire_at);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and batch_id = ");
        qb.push_bind(batch_id);
        qb.push(
            " returning batch_id, sku_id, batch_no, production_date, expire_at, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            batch_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            batch_no: String,
            production_date: Option<DateTime<Utc>>,
            expire_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| SkuBatch {
            batch_id: row.batch_id,
            tenant_id,
            sku_id: row.sku_id,
            batch_no: row.batch_no,
            production_date: row.production_date,
            expire_at: row.expire_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
