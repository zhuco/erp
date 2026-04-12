use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickRecommendStrategy {
    pub strategy_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub method: String,
    pub prefer_location_priority: bool,
    pub prefer_pick_zone: bool,
    pub prefer_sufficient: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PickRecommendStrategyCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub method: String,
    pub prefer_location_priority: bool,
    pub prefer_pick_zone: bool,
    pub prefer_sufficient: bool,
}

#[derive(Debug, Clone)]
pub struct PickRecommendStrategyUpdate {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub method: Option<String>,
    pub prefer_location_priority: Option<bool>,
    pub prefer_pick_zone: Option<bool>,
    pub prefer_sufficient: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PickRecommendStrategyListFilters {
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct PickRecommendStrategyListResult {
    pub items: Vec<PickRecommendStrategy>,
    pub total: i64,
}

pub struct PickRecommendStrategiesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PickRecommendStrategiesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_strategies(
        &self,
        tenant_id: TenantId,
        filters: PickRecommendStrategyListFilters,
    ) -> anyhow::Result<PickRecommendStrategyListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from pick_recommend_strategies where tenant_id = ",
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
                strategy_id, name, enabled, priority, method,
                prefer_location_priority, prefer_pick_zone, prefer_sufficient,
                created_at, updated_at
            from pick_recommend_strategies
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(enabled) = filters.enabled {
            qb.push(" and enabled = ");
            qb.push_bind(enabled);
        }
        qb.push(" order by priority desc, updated_at desc, strategy_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        let rows: Vec<PickRecommendStrategyRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_strategy(tenant_id))
            .collect();

        Ok(PickRecommendStrategyListResult { items, total })
    }

    pub async fn list_enabled_strategies(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<PickRecommendStrategy>> {
        let rows: Vec<PickRecommendStrategyRow> = sqlx::query_as(
            r#"
            select
                strategy_id, name, enabled, priority, method,
                prefer_location_priority, prefer_pick_zone, prefer_sufficient,
                created_at, updated_at
            from pick_recommend_strategies
            where tenant_id = $1 and enabled = true
            order by priority desc, updated_at desc, strategy_id desc
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
    ) -> anyhow::Result<Option<PickRecommendStrategy>> {
        let row: Option<PickRecommendStrategyRow> = sqlx::query_as(
            r#"
            select
                strategy_id, name, enabled, priority, method,
                prefer_location_priority, prefer_pick_zone, prefer_sufficient,
                created_at, updated_at
            from pick_recommend_strategies
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
        input: PickRecommendStrategyCreate,
    ) -> anyhow::Result<PickRecommendStrategy> {
        let row: PickRecommendStrategyRow = sqlx::query_as(
            r#"
            insert into pick_recommend_strategies(
                tenant_id, name, enabled, priority, method,
                prefer_location_priority, prefer_pick_zone, prefer_sufficient
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            on conflict (tenant_id, name)
            do update set
                enabled = excluded.enabled,
                priority = excluded.priority,
                method = excluded.method,
                prefer_location_priority = excluded.prefer_location_priority,
                prefer_pick_zone = excluded.prefer_pick_zone,
                prefer_sufficient = excluded.prefer_sufficient,
                updated_at = now()
            returning
                strategy_id, name, enabled, priority, method,
                prefer_location_priority, prefer_pick_zone, prefer_sufficient,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.enabled)
        .bind(input.priority)
        .bind(&input.method)
        .bind(input.prefer_location_priority)
        .bind(input.prefer_pick_zone)
        .bind(input.prefer_sufficient)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_strategy(input.tenant_id))
    }

    pub async fn update_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
        input: PickRecommendStrategyUpdate,
    ) -> anyhow::Result<Option<PickRecommendStrategy>> {
        let mut qb = QueryBuilder::<Postgres>::new("update pick_recommend_strategies set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
        }
        if let Some(priority) = input.priority {
            separated.push("priority = ");
            separated.push_bind(priority);
        }
        if let Some(method) = input.method {
            separated.push("method = ");
            separated.push_bind(method);
        }
        if let Some(prefer_location_priority) = input.prefer_location_priority {
            separated.push("prefer_location_priority = ");
            separated.push_bind(prefer_location_priority);
        }
        if let Some(prefer_pick_zone) = input.prefer_pick_zone {
            separated.push("prefer_pick_zone = ");
            separated.push_bind(prefer_pick_zone);
        }
        if let Some(prefer_sufficient) = input.prefer_sufficient {
            separated.push("prefer_sufficient = ");
            separated.push_bind(prefer_sufficient);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and strategy_id = ");
        qb.push_bind(strategy_id);
        qb.push(
            " returning strategy_id, name, enabled, priority, method, prefer_location_priority, prefer_pick_zone, prefer_sufficient, created_at, updated_at",
        );

        let row: Option<PickRecommendStrategyRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_strategy(tenant_id)))
    }

    pub async fn delete_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
    ) -> anyhow::Result<Option<PickRecommendStrategy>> {
        let row: Option<PickRecommendStrategyRow> = sqlx::query_as(
            r#"
            delete from pick_recommend_strategies
            where tenant_id = $1 and strategy_id = $2
            returning
                strategy_id, name, enabled, priority, method,
                prefer_location_priority, prefer_pick_zone, prefer_sufficient,
                created_at, updated_at
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
struct PickRecommendStrategyRow {
    strategy_id: uuid::Uuid,
    name: String,
    enabled: bool,
    priority: i32,
    method: String,
    prefer_location_priority: bool,
    prefer_pick_zone: bool,
    prefer_sufficient: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl PickRecommendStrategyRow {
    fn into_strategy(self, tenant_id: TenantId) -> PickRecommendStrategy {
        PickRecommendStrategy {
            strategy_id: self.strategy_id,
            tenant_id,
            name: self.name,
            enabled: self.enabled,
            priority: self.priority,
            method: self.method,
            prefer_location_priority: self.prefer_location_priority,
            prefer_pick_zone: self.prefer_pick_zone,
            prefer_sufficient: self.prefer_sufficient,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
