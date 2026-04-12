use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsWarehouseStrategy {
    pub strategy_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub priority: String,
    pub enabled: bool,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsWarehouseStrategyHistory {
    pub record_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub strategy_id: uuid::Uuid,
    pub summary: String,
    pub updated_by: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsWarehouseSuggestion {
    pub suggestion_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub order_id: String,
    pub destination: String,
    pub recommended_warehouse: String,
    pub eta_hours: i32,
    pub cost_cent: i64,
    pub savings_cent: i64,
    pub strategy_id: Option<uuid::Uuid>,
    pub applied: bool,
    pub applied_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LogisticsWarehouseStrategyCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub priority: String,
    pub enabled: bool,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogisticsWarehouseStrategyUpdate {
    pub name: Option<String>,
    pub priority: Option<String>,
    pub enabled: Option<bool>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogisticsWarehouseStrategyListFilters {
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct LogisticsWarehouseStrategyListResult {
    pub items: Vec<LogisticsWarehouseStrategy>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct LogisticsWarehouseStrategyHistoryCreate {
    pub tenant_id: TenantId,
    pub strategy_id: uuid::Uuid,
    pub summary: String,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogisticsWarehouseSuggestionListFilters {
    pub applied: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct LogisticsWarehouseSuggestionListResult {
    pub items: Vec<LogisticsWarehouseSuggestion>,
    pub total: i64,
}

pub struct LogisticsWarehouseStrategiesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsWarehouseStrategiesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_strategies(
        &self,
        tenant_id: TenantId,
        filters: LogisticsWarehouseStrategyListFilters,
    ) -> anyhow::Result<LogisticsWarehouseStrategyListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from logistics_warehouse_strategies where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(enabled) = filters.enabled {
            count_qb.push(" and enabled = ");
            count_qb.push_bind(enabled);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                strategy_id, name, priority, enabled, updated_by, created_at, updated_at
            from logistics_warehouse_strategies
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

        let rows: Vec<LogisticsWarehouseStrategyRow> =
            qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_strategy(tenant_id))
            .collect();

        Ok(LogisticsWarehouseStrategyListResult { items, total })
    }

    pub async fn create_strategy(
        &self,
        input: LogisticsWarehouseStrategyCreate,
    ) -> anyhow::Result<LogisticsWarehouseStrategy> {
        let row: LogisticsWarehouseStrategyRow = sqlx::query_as(
            r#"
            insert into logistics_warehouse_strategies(
                tenant_id, name, priority, enabled, updated_by
            ) values ($1, $2, $3, $4, $5)
            returning strategy_id, name, priority, enabled, updated_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.priority)
        .bind(input.enabled)
        .bind(&input.updated_by)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_strategy(input.tenant_id))
    }

    pub async fn update_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
        input: LogisticsWarehouseStrategyUpdate,
    ) -> anyhow::Result<Option<LogisticsWarehouseStrategy>> {
        let mut qb = QueryBuilder::<Postgres>::new("update logistics_warehouse_strategies set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(priority) = input.priority {
            separated.push("priority = ");
            separated.push_bind(priority);
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
        }
        if let Some(updated_by) = input.updated_by {
            separated.push("updated_by = ");
            separated.push_bind(updated_by);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and strategy_id = ");
        qb.push_bind(strategy_id);
        qb.push(
            " returning strategy_id, name, priority, enabled, updated_by, created_at, updated_at",
        );

        let row: Option<LogisticsWarehouseStrategyRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_strategy(tenant_id)))
    }

    pub async fn insert_history(
        &self,
        input: LogisticsWarehouseStrategyHistoryCreate,
    ) -> anyhow::Result<LogisticsWarehouseStrategyHistory> {
        #[derive(sqlx::FromRow)]
        struct Row {
            record_id: uuid::Uuid,
            strategy_id: uuid::Uuid,
            summary: String,
            updated_by: Option<String>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into logistics_warehouse_strategy_history(
                tenant_id, strategy_id, summary, updated_by
            ) values ($1, $2, $3, $4)
            returning record_id, strategy_id, summary, updated_by, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.strategy_id)
        .bind(&input.summary)
        .bind(&input.updated_by)
        .fetch_one(self.pool)
        .await?;

        Ok(LogisticsWarehouseStrategyHistory {
            record_id: row.record_id,
            tenant_id: input.tenant_id,
            strategy_id: row.strategy_id,
            summary: row.summary,
            updated_by: row.updated_by,
            updated_at: row.updated_at,
        })
    }

    pub async fn list_history(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<LogisticsWarehouseStrategyHistory>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            record_id: uuid::Uuid,
            strategy_id: uuid::Uuid,
            summary: String,
            updated_by: Option<String>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select record_id, strategy_id, summary, updated_by, updated_at
            from logistics_warehouse_strategy_history
            where tenant_id = $1 and strategy_id = $2
            order by updated_at desc, record_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(strategy_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| LogisticsWarehouseStrategyHistory {
                record_id: row.record_id,
                tenant_id,
                strategy_id: row.strategy_id,
                summary: row.summary,
                updated_by: row.updated_by,
                updated_at: row.updated_at,
            })
            .collect())
    }
}

pub struct LogisticsWarehouseSuggestionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsWarehouseSuggestionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_suggestions(
        &self,
        tenant_id: TenantId,
        filters: LogisticsWarehouseSuggestionListFilters,
    ) -> anyhow::Result<LogisticsWarehouseSuggestionListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from logistics_warehouse_suggestions where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(applied) = filters.applied {
            count_qb.push(" and applied = ");
            count_qb.push_bind(applied);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                suggestion_id, order_id, destination, recommended_warehouse, eta_hours,
                cost_cent, savings_cent, strategy_id, applied, applied_at, created_at
            from logistics_warehouse_suggestions
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(applied) = filters.applied {
            qb.push(" and applied = ");
            qb.push_bind(applied);
        }
        qb.push(" order by created_at desc, suggestion_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            suggestion_id: uuid::Uuid,
            order_id: String,
            destination: String,
            recommended_warehouse: String,
            eta_hours: i32,
            cost_cent: i64,
            savings_cent: i64,
            strategy_id: Option<uuid::Uuid>,
            applied: bool,
            applied_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| LogisticsWarehouseSuggestion {
                suggestion_id: row.suggestion_id,
                tenant_id,
                order_id: row.order_id,
                destination: row.destination,
                recommended_warehouse: row.recommended_warehouse,
                eta_hours: row.eta_hours,
                cost_cent: row.cost_cent,
                savings_cent: row.savings_cent,
                strategy_id: row.strategy_id,
                applied: row.applied,
                applied_at: row.applied_at,
                created_at: row.created_at,
            })
            .collect();

        Ok(LogisticsWarehouseSuggestionListResult { items, total })
    }

    pub async fn apply_suggestion(
        &self,
        tenant_id: TenantId,
        suggestion_id: uuid::Uuid,
    ) -> anyhow::Result<Option<LogisticsWarehouseSuggestion>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            suggestion_id: uuid::Uuid,
            order_id: String,
            destination: String,
            recommended_warehouse: String,
            eta_hours: i32,
            cost_cent: i64,
            savings_cent: i64,
            strategy_id: Option<uuid::Uuid>,
            applied: bool,
            applied_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update logistics_warehouse_suggestions
            set applied = true,
                applied_at = now()
            where tenant_id = $1 and suggestion_id = $2
            returning
                suggestion_id, order_id, destination, recommended_warehouse, eta_hours,
                cost_cent, savings_cent, strategy_id, applied, applied_at, created_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(suggestion_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| LogisticsWarehouseSuggestion {
            suggestion_id: row.suggestion_id,
            tenant_id,
            order_id: row.order_id,
            destination: row.destination,
            recommended_warehouse: row.recommended_warehouse,
            eta_hours: row.eta_hours,
            cost_cent: row.cost_cent,
            savings_cent: row.savings_cent,
            strategy_id: row.strategy_id,
            applied: row.applied,
            applied_at: row.applied_at,
            created_at: row.created_at,
        }))
    }
}

#[derive(sqlx::FromRow)]
struct LogisticsWarehouseStrategyRow {
    strategy_id: uuid::Uuid,
    name: String,
    priority: String,
    enabled: bool,
    updated_by: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl LogisticsWarehouseStrategyRow {
    fn into_strategy(self, tenant_id: TenantId) -> LogisticsWarehouseStrategy {
        LogisticsWarehouseStrategy {
            strategy_id: self.strategy_id,
            tenant_id,
            name: self.name,
            priority: self.priority,
            enabled: self.enabled,
            updated_by: self.updated_by,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
