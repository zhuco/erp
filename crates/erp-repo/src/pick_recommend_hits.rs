use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickRecommendHit {
    pub hit_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub strategy_id: Option<uuid::Uuid>,
    pub pick_task_id: Option<uuid::Uuid>,
    pub sku_id: Option<uuid::Uuid>,
    pub from_location_id: Option<uuid::Uuid>,
    pub reason: Option<String>,
    pub success: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PickRecommendHitCreate {
    pub tenant_id: TenantId,
    pub strategy_id: Option<uuid::Uuid>,
    pub pick_task_id: Option<uuid::Uuid>,
    pub sku_id: Option<uuid::Uuid>,
    pub from_location_id: Option<uuid::Uuid>,
    pub reason: Option<String>,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct PickRecommendHitListFilters {
    pub strategy_id: Option<uuid::Uuid>,
    pub pick_task_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct PickRecommendHitListResult {
    pub items: Vec<PickRecommendHit>,
    pub total: i64,
}

pub struct PickRecommendHitsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PickRecommendHitsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_hit(&self, input: PickRecommendHitCreate) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            insert into pick_recommend_hits(
                tenant_id, strategy_id, pick_task_id, sku_id,
                from_location_id, reason, success
            ) values ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.strategy_id)
        .bind(input.pick_task_id)
        .bind(input.sku_id)
        .bind(input.from_location_id)
        .bind(input.reason)
        .bind(input.success)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_hits(
        &self,
        tenant_id: TenantId,
        filters: PickRecommendHitListFilters,
    ) -> anyhow::Result<PickRecommendHitListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from pick_recommend_hits where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                hit_id, strategy_id, pick_task_id, sku_id,
                from_location_id, reason, success, created_at
            from pick_recommend_hits
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, hit_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        let rows: Vec<PickRecommendHitRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_hit(tenant_id))
            .collect();

        Ok(PickRecommendHitListResult { items, total })
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &PickRecommendHitListFilters) {
    if let Some(strategy_id) = filters.strategy_id {
        qb.push(" and strategy_id = ");
        qb.push_bind(strategy_id);
    }
    if let Some(pick_task_id) = filters.pick_task_id {
        qb.push(" and pick_task_id = ");
        qb.push_bind(pick_task_id);
    }
}

#[derive(sqlx::FromRow)]
struct PickRecommendHitRow {
    hit_id: uuid::Uuid,
    strategy_id: Option<uuid::Uuid>,
    pick_task_id: Option<uuid::Uuid>,
    sku_id: Option<uuid::Uuid>,
    from_location_id: Option<uuid::Uuid>,
    reason: Option<String>,
    success: bool,
    created_at: DateTime<Utc>,
}

impl PickRecommendHitRow {
    fn into_hit(self, tenant_id: TenantId) -> PickRecommendHit {
        PickRecommendHit {
            hit_id: self.hit_id,
            tenant_id,
            strategy_id: self.strategy_id,
            pick_task_id: self.pick_task_id,
            sku_id: self.sku_id,
            from_location_id: self.from_location_id,
            reason: self.reason,
            success: self.success,
            created_at: self.created_at,
        }
    }
}
