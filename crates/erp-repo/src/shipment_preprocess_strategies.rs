use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentPreprocessStrategy {
    pub strategy_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub conditions_json: Value,
    pub rules_json: Value,
    pub auto_freeze: bool,
    pub auto_waybill_reserve: bool,
    pub sender_address_id: Option<uuid::Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ShipmentPreprocessStrategyCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub conditions_json: Value,
    pub rules_json: Value,
    pub auto_freeze: bool,
    pub auto_waybill_reserve: bool,
    pub sender_address_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct ShipmentPreprocessStrategyUpdate {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub conditions_json: Option<Value>,
    pub rules_json: Option<Value>,
    pub auto_freeze: Option<bool>,
    pub auto_waybill_reserve: Option<bool>,
    pub sender_address_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct ShipmentPreprocessStrategyListFilters {
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ShipmentPreprocessStrategyListResult {
    pub items: Vec<ShipmentPreprocessStrategy>,
    pub total: i64,
}

pub struct ShipmentPreprocessStrategiesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ShipmentPreprocessStrategiesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_strategies(
        &self,
        tenant_id: TenantId,
        filters: ShipmentPreprocessStrategyListFilters,
    ) -> anyhow::Result<ShipmentPreprocessStrategyListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from shipment_preprocess_strategies where tenant_id = ",
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
                strategy_id, name, enabled, priority, effective_from, effective_to,
                conditions_json, rules_json, auto_freeze, auto_waybill_reserve,
                sender_address_id, created_at, updated_at
            from shipment_preprocess_strategies
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

        let rows: Vec<ShipmentPreprocessStrategyRow> =
            qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_strategy(tenant_id))
            .collect();

        Ok(ShipmentPreprocessStrategyListResult { items, total })
    }

    pub async fn list_enabled_strategies(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<ShipmentPreprocessStrategy>> {
        let rows: Vec<ShipmentPreprocessStrategyRow> = sqlx::query_as(
            r#"
            select
                strategy_id, name, enabled, priority, effective_from, effective_to,
                conditions_json, rules_json, auto_freeze, auto_waybill_reserve,
                sender_address_id, created_at, updated_at
            from shipment_preprocess_strategies
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
    ) -> anyhow::Result<Option<ShipmentPreprocessStrategy>> {
        let row: Option<ShipmentPreprocessStrategyRow> = sqlx::query_as(
            r#"
            select
                strategy_id, name, enabled, priority, effective_from, effective_to,
                conditions_json, rules_json, auto_freeze, auto_waybill_reserve,
                sender_address_id, created_at, updated_at
            from shipment_preprocess_strategies
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
        input: ShipmentPreprocessStrategyCreate,
    ) -> anyhow::Result<ShipmentPreprocessStrategy> {
        let row: ShipmentPreprocessStrategyRow = sqlx::query_as(
            r#"
            insert into shipment_preprocess_strategies(
                tenant_id, name, enabled, priority, effective_from, effective_to,
                conditions_json, rules_json, auto_freeze, auto_waybill_reserve, sender_address_id
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            on conflict (tenant_id, name)
            do update set
                enabled = excluded.enabled,
                priority = excluded.priority,
                effective_from = excluded.effective_from,
                effective_to = excluded.effective_to,
                conditions_json = excluded.conditions_json,
                rules_json = excluded.rules_json,
                auto_freeze = excluded.auto_freeze,
                auto_waybill_reserve = excluded.auto_waybill_reserve,
                sender_address_id = excluded.sender_address_id,
                updated_at = now()
            returning
                strategy_id, name, enabled, priority, effective_from, effective_to,
                conditions_json, rules_json, auto_freeze, auto_waybill_reserve,
                sender_address_id, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.enabled)
        .bind(input.priority)
        .bind(input.effective_from)
        .bind(input.effective_to)
        .bind(input.conditions_json)
        .bind(input.rules_json)
        .bind(input.auto_freeze)
        .bind(input.auto_waybill_reserve)
        .bind(input.sender_address_id)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_strategy(input.tenant_id))
    }

    pub async fn update_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
        input: ShipmentPreprocessStrategyUpdate,
    ) -> anyhow::Result<Option<ShipmentPreprocessStrategy>> {
        let mut qb = QueryBuilder::<Postgres>::new("update shipment_preprocess_strategies set ");
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
        if let Some(effective_from) = input.effective_from {
            separated.push("effective_from = ");
            separated.push_bind(effective_from);
        }
        if let Some(effective_to) = input.effective_to {
            separated.push("effective_to = ");
            separated.push_bind(effective_to);
        }
        if let Some(conditions_json) = input.conditions_json {
            separated.push("conditions_json = ");
            separated.push_bind(conditions_json);
        }
        if let Some(rules_json) = input.rules_json {
            separated.push("rules_json = ");
            separated.push_bind(rules_json);
        }
        if let Some(auto_freeze) = input.auto_freeze {
            separated.push("auto_freeze = ");
            separated.push_bind(auto_freeze);
        }
        if let Some(auto_waybill_reserve) = input.auto_waybill_reserve {
            separated.push("auto_waybill_reserve = ");
            separated.push_bind(auto_waybill_reserve);
        }
        if let Some(sender_address_id) = input.sender_address_id {
            separated.push("sender_address_id = ");
            separated.push_bind(sender_address_id);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and strategy_id = ");
        qb.push_bind(strategy_id);
        qb.push(
            " returning strategy_id, name, enabled, priority, effective_from, effective_to, conditions_json, rules_json, auto_freeze, auto_waybill_reserve, sender_address_id, created_at, updated_at",
        );

        let row: Option<ShipmentPreprocessStrategyRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_strategy(tenant_id)))
    }

    pub async fn delete_strategy(
        &self,
        tenant_id: TenantId,
        strategy_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ShipmentPreprocessStrategy>> {
        let row: Option<ShipmentPreprocessStrategyRow> = sqlx::query_as(
            r#"
            delete from shipment_preprocess_strategies
            where tenant_id = $1 and strategy_id = $2
            returning
                strategy_id, name, enabled, priority, effective_from, effective_to,
                conditions_json, rules_json, auto_freeze, auto_waybill_reserve,
                sender_address_id, created_at, updated_at
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
struct ShipmentPreprocessStrategyRow {
    strategy_id: uuid::Uuid,
    name: String,
    enabled: bool,
    priority: i32,
    effective_from: Option<DateTime<Utc>>,
    effective_to: Option<DateTime<Utc>>,
    conditions_json: Value,
    rules_json: Value,
    auto_freeze: bool,
    auto_waybill_reserve: bool,
    sender_address_id: Option<uuid::Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ShipmentPreprocessStrategyRow {
    fn into_strategy(self, tenant_id: TenantId) -> ShipmentPreprocessStrategy {
        ShipmentPreprocessStrategy {
            strategy_id: self.strategy_id,
            tenant_id,
            name: self.name,
            enabled: self.enabled,
            priority: self.priority,
            effective_from: self.effective_from,
            effective_to: self.effective_to,
            conditions_json: self.conditions_json,
            rules_json: self.rules_json,
            auto_freeze: self.auto_freeze,
            auto_waybill_reserve: self.auto_waybill_reserve,
            sender_address_id: self.sender_address_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
