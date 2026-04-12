use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCountStrategy {
    pub strategy_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub strategy_type: String,
    pub config_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockCountStrategyCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub strategy_type: String,
    pub config_json: Value,
}

#[derive(Debug, Clone)]
pub struct StockCountStrategyUpdate {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub strategy_type: Option<String>,
    pub config_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct StockCountStrategyListFilters {
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct StockCountStrategyListResult {
    pub items: Vec<StockCountStrategy>,
    pub total: i64,
}

pub struct StockCountStrategiesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> StockCountStrategiesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_strategies(
        &self,
        tenant_id: TenantId,
        filters: StockCountStrategyListFilters,
    ) -> anyhow::Result<StockCountStrategyListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from stock_count_strategies where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(enabled) = filters.enabled {
            count_qb.push(" and enabled = ");
            count_qb.push_bind(enabled);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select strategy_id, name, enabled, strategy_type, config_json, created_at, updated_at
            from stock_count_strategies
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(enabled) = filters.enabled {
            qb.push(" and enabled = ");
            qb.push_bind(enabled);
        }
        qb.push(" order by updated_at desc, strategy_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        let rows: Vec<StockCountStrategyRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_strategy(tenant_id))
            .collect();

        Ok(StockCountStrategyListResult { items, total })
    }

    pub async fn list_enabled_strategies(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<StockCountStrategy>> {
        let rows: Vec<StockCountStrategyRow> = sqlx::query_as(
            r#"
            select strategy_id, name, enabled, strategy_type, config_json, created_at, updated_at
            from stock_count_strategies
            where tenant_id = $1 and enabled = true
            order by updated_at desc, strategy_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| row.into_strategy(tenant_id))
            .collect())
    }

    pub async fn get_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockCountStrategy>> {
        let row: Option<StockCountStrategyRow> = sqlx::query_as(
            r#"
            select strategy_id, name, enabled, strategy_type, config_json, created_at, updated_at
            from stock_count_strategies
            where tenant_id = $1 and strategy_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(strategy_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_strategy(tenant_id)))
    }

    pub async fn create_strategy(
        &self,
        input: StockCountStrategyCreate,
    ) -> anyhow::Result<StockCountStrategy> {
        let row: StockCountStrategyRow = sqlx::query_as(
            r#"
            insert into stock_count_strategies(
                tenant_id, name, enabled, strategy_type, config_json
            ) values ($1,$2,$3,$4,$5)
            on conflict (tenant_id, name)
            do update set
                enabled = excluded.enabled,
                strategy_type = excluded.strategy_type,
                config_json = excluded.config_json,
                updated_at = now()
            returning strategy_id, name, enabled, strategy_type, config_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.enabled)
        .bind(&input.strategy_type)
        .bind(input.config_json)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_strategy(input.tenant_id))
    }

    pub async fn update_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
        input: StockCountStrategyUpdate,
    ) -> anyhow::Result<Option<StockCountStrategy>> {
        let mut qb = QueryBuilder::<Postgres>::new("update stock_count_strategies set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
        }
        if let Some(strategy_type) = input.strategy_type {
            separated.push("strategy_type = ");
            separated.push_bind(strategy_type);
        }
        if let Some(config_json) = input.config_json {
            separated.push("config_json = ");
            separated.push_bind(config_json);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and strategy_id = ");
        qb.push_bind(strategy_id);
        qb.push(
            " returning strategy_id, name, enabled, strategy_type, config_json, created_at, updated_at",
        );

        let row: Option<StockCountStrategyRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_strategy(tenant_id)))
    }

    pub async fn delete_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockCountStrategy>> {
        let row: Option<StockCountStrategyRow> = sqlx::query_as(
            r#"
            delete from stock_count_strategies
            where tenant_id = $1 and strategy_id = $2
            returning strategy_id, name, enabled, strategy_type, config_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(strategy_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_strategy(tenant_id)))
    }
}

#[derive(sqlx::FromRow)]
struct StockCountStrategyRow {
    strategy_id: uuid::Uuid,
    name: String,
    enabled: bool,
    strategy_type: String,
    config_json: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockCountStrategyRow {
    fn into_strategy(self, tenant_id: TenantId) -> StockCountStrategy {
        StockCountStrategy {
            strategy_id: self.strategy_id,
            tenant_id,
            name: self.name,
            enabled: self.enabled,
            strategy_type: self.strategy_type,
            config_json: self.config_json,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
