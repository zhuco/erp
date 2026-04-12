use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockCountSuggestion {
    pub suggestion_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub strategy_id: Option<uuid::Uuid>,
    pub strategy_name: Option<String>,
    pub reason: Option<String>,
    pub item_count: i32,
    pub items_json: Value,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StockCountSuggestionCreate {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub strategy_id: Option<uuid::Uuid>,
    pub strategy_name: Option<String>,
    pub reason: Option<String>,
    pub item_count: i32,
    pub items_json: Value,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct StockCountSuggestionListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct StockCountSuggestionListResult {
    pub items: Vec<StockCountSuggestion>,
    pub total: i64,
}

pub struct StockCountSuggestionsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> StockCountSuggestionsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn clear_open(&self, tenant_id: TenantId) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            delete from stock_count_suggestions
            where tenant_id = $1 and status = 'OPEN'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_suggestion(
        &self,
        input: StockCountSuggestionCreate,
    ) -> anyhow::Result<StockCountSuggestion> {
        let row: StockCountSuggestionRow = sqlx::query_as(
            r#"
            insert into stock_count_suggestions(
                tenant_id, warehouse_id, strategy_id, strategy_name,
                reason, item_count, items_json, status
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                suggestion_id, warehouse_id, strategy_id, strategy_name,
                reason, item_count, items_json, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(input.strategy_id)
        .bind(&input.strategy_name)
        .bind(&input.reason)
        .bind(input.item_count)
        .bind(input.items_json)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_suggestion(input.tenant_id))
    }

    pub async fn list_suggestions(
        &self,
        tenant_id: TenantId,
        filters: StockCountSuggestionListFilters,
    ) -> anyhow::Result<StockCountSuggestionListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from stock_count_suggestions where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                suggestion_id, warehouse_id, strategy_id, strategy_name,
                reason, item_count, items_json, status, created_at, updated_at
            from stock_count_suggestions
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, suggestion_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        let rows: Vec<StockCountSuggestionRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_suggestion(tenant_id))
            .collect();

        Ok(StockCountSuggestionListResult { items, total })
    }

    pub async fn get_suggestion(
        &self,
        tenant_id: TenantId,
        suggestion_id: uuid::Uuid,
    ) -> anyhow::Result<Option<StockCountSuggestion>> {
        let row: Option<StockCountSuggestionRow> = sqlx::query_as(
            r#"
            select
                suggestion_id, warehouse_id, strategy_id, strategy_name,
                reason, item_count, items_json, status, created_at, updated_at
            from stock_count_suggestions
            where tenant_id = $1 and suggestion_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(suggestion_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_suggestion(tenant_id)))
    }

    pub async fn mark_applied(
        &self,
        tenant_id: TenantId,
        suggestion_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update stock_count_suggestions
            set status = 'APPLIED',
                updated_at = now()
            where tenant_id = $1 and suggestion_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(suggestion_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a StockCountSuggestionListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
}

#[derive(sqlx::FromRow)]
struct StockCountSuggestionRow {
    suggestion_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    strategy_id: Option<uuid::Uuid>,
    strategy_name: Option<String>,
    reason: Option<String>,
    item_count: i32,
    items_json: Value,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl StockCountSuggestionRow {
    fn into_suggestion(self, tenant_id: TenantId) -> StockCountSuggestion {
        StockCountSuggestion {
            suggestion_id: self.suggestion_id,
            tenant_id,
            warehouse_id: self.warehouse_id,
            strategy_id: self.strategy_id,
            strategy_name: self.strategy_name,
            reason: self.reason,
            item_count: self.item_count,
            items_json: self.items_json,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
