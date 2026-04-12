use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuBatchStrategy {
    pub strategy_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: Option<uuid::Uuid>,
    pub name: String,
    pub enabled: bool,
    pub expire_threshold_days: i32,
    pub fifo_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SkuBatchStrategyListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

pub struct SkuBatchStrategyListResult {
    pub items: Vec<SkuBatchStrategy>,
    pub total: i64,
}

pub struct SkuBatchStrategyCreate {
    pub tenant_id: TenantId,
    pub sku_id: Option<uuid::Uuid>,
    pub name: String,
    pub enabled: bool,
    pub expire_threshold_days: i32,
    pub fifo_enabled: bool,
}

pub struct SkuBatchStrategyUpdate {
    pub sku_id: Option<Option<uuid::Uuid>>,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub expire_threshold_days: Option<i32>,
    pub fifo_enabled: Option<bool>,
}

pub struct SkuBatchStrategiesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SkuBatchStrategiesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_strategies(
        &self,
        tenant_id: TenantId,
        filters: SkuBatchStrategyListFilters,
    ) -> anyhow::Result<SkuBatchStrategyListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from sku_batch_strategies where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                strategy_id, sku_id, name, enabled,
                expire_threshold_days, fifo_enabled, created_at, updated_at
            from sku_batch_strategies
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by updated_at desc, strategy_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            strategy_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            enabled: bool,
            expire_threshold_days: i32,
            fifo_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| SkuBatchStrategy {
                strategy_id: row.strategy_id,
                tenant_id,
                sku_id: row.sku_id,
                name: row.name,
                enabled: row.enabled,
                expire_threshold_days: row.expire_threshold_days,
                fifo_enabled: row.fifo_enabled,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SkuBatchStrategyListResult { items, total })
    }

    pub async fn create_strategy(
        &self,
        input: SkuBatchStrategyCreate,
    ) -> anyhow::Result<SkuBatchStrategy> {
        #[derive(sqlx::FromRow)]
        struct Row {
            strategy_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            enabled: bool,
            expire_threshold_days: i32,
            fifo_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into sku_batch_strategies(
                tenant_id, sku_id, name, enabled, expire_threshold_days, fifo_enabled
            ) values ($1,$2,$3,$4,$5,$6)
            returning
                strategy_id, sku_id, name, enabled, expire_threshold_days, fifo_enabled,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.name)
        .bind(input.enabled)
        .bind(input.expire_threshold_days)
        .bind(input.fifo_enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(SkuBatchStrategy {
            strategy_id: row.strategy_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            enabled: row.enabled,
            expire_threshold_days: row.expire_threshold_days,
            fifo_enabled: row.fifo_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
        input: SkuBatchStrategyUpdate,
    ) -> anyhow::Result<Option<SkuBatchStrategy>> {
        let mut qb = QueryBuilder::<Postgres>::new("update sku_batch_strategies set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(sku_id) = input.sku_id {
            separated.push("sku_id = ");
            separated.push_bind(sku_id);
            has_updates = true;
        }
        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
            has_updates = true;
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
            has_updates = true;
        }
        if let Some(expire_threshold_days) = input.expire_threshold_days {
            separated.push("expire_threshold_days = ");
            separated.push_bind(expire_threshold_days);
            has_updates = true;
        }
        if let Some(fifo_enabled) = input.fifo_enabled {
            separated.push("fifo_enabled = ");
            separated.push_bind(fifo_enabled);
            has_updates = true;
        }

        if !has_updates {
            return self.get_strategy(tenant_id, strategy_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and strategy_id = ");
        qb.push_bind(strategy_id);
        qb.push(
            " returning strategy_id, sku_id, name, enabled, expire_threshold_days, fifo_enabled, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            strategy_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            enabled: bool,
            expire_threshold_days: i32,
            fifo_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| SkuBatchStrategy {
            strategy_id: row.strategy_id,
            tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            enabled: row.enabled,
            expire_threshold_days: row.expire_threshold_days,
            fifo_enabled: row.fifo_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuBatchStrategy>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            strategy_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            enabled: bool,
            expire_threshold_days: i32,
            fifo_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from sku_batch_strategies
            where tenant_id = $1 and strategy_id = $2
            returning
                strategy_id, sku_id, name, enabled, expire_threshold_days, fifo_enabled,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(strategy_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SkuBatchStrategy {
            strategy_id: row.strategy_id,
            tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            enabled: row.enabled,
            expire_threshold_days: row.expire_threshold_days,
            fifo_enabled: row.fifo_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn get_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuBatchStrategy>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            strategy_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            enabled: bool,
            expire_threshold_days: i32,
            fifo_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                strategy_id, sku_id, name, enabled, expire_threshold_days, fifo_enabled,
                created_at, updated_at
            from sku_batch_strategies
            where tenant_id = $1 and strategy_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(strategy_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SkuBatchStrategy {
            strategy_id: row.strategy_id,
            tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            enabled: row.enabled,
            expire_threshold_days: row.expire_threshold_days,
            fifo_enabled: row.fifo_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a SkuBatchStrategyListFilters,
) {
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(enabled) = filters.enabled {
        qb.push(" and enabled = ");
        qb.push_bind(enabled);
    }
}
