use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsMatchHit {
    pub hit_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub rule_id: Option<uuid::Uuid>,
    pub shipment_id: Option<uuid::Uuid>,
    pub order_id: Option<uuid::Uuid>,
    pub product_id: Option<uuid::Uuid>,
    pub hit_reason: Option<String>,
    pub result_json: Option<Value>,
    pub success: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct LogisticsMatchHitCreate {
    pub tenant_id: TenantId,
    pub rule_id: Option<uuid::Uuid>,
    pub shipment_id: Option<uuid::Uuid>,
    pub order_id: Option<uuid::Uuid>,
    pub product_id: Option<uuid::Uuid>,
    pub hit_reason: Option<String>,
    pub result_json: Option<Value>,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct LogisticsMatchHitListFilters {
    pub rule_id: Option<uuid::Uuid>,
    pub shipment_id: Option<uuid::Uuid>,
    pub order_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct LogisticsMatchHitListResult {
    pub items: Vec<LogisticsMatchHit>,
    pub total: i64,
}

pub struct LogisticsMatchHitsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> LogisticsMatchHitsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_hit(&self, input: LogisticsMatchHitCreate) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            insert into logistics_match_hits(
                tenant_id, rule_id, shipment_id, order_id, product_id,
                hit_reason, result_json, success
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.rule_id)
        .bind(input.shipment_id)
        .bind(input.order_id)
        .bind(input.product_id)
        .bind(input.hit_reason)
        .bind(input.result_json)
        .bind(input.success)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_hits(
        &self,
        tenant_id: TenantId,
        filters: LogisticsMatchHitListFilters,
    ) -> anyhow::Result<LogisticsMatchHitListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from logistics_match_hits where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                hit_id, rule_id, shipment_id, order_id, product_id,
                hit_reason, result_json, success, created_at
            from logistics_match_hits
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

        let rows: Vec<LogisticsMatchHitRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_hit(tenant_id))
            .collect();

        Ok(LogisticsMatchHitListResult { items, total })
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &LogisticsMatchHitListFilters) {
    if let Some(rule_id) = filters.rule_id {
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
    }
    if let Some(shipment_id) = filters.shipment_id {
        qb.push(" and shipment_id = ");
        qb.push_bind(shipment_id);
    }
    if let Some(order_id) = filters.order_id {
        qb.push(" and order_id = ");
        qb.push_bind(order_id);
    }
}

#[derive(sqlx::FromRow)]
struct LogisticsMatchHitRow {
    hit_id: uuid::Uuid,
    rule_id: Option<uuid::Uuid>,
    shipment_id: Option<uuid::Uuid>,
    order_id: Option<uuid::Uuid>,
    product_id: Option<uuid::Uuid>,
    hit_reason: Option<String>,
    result_json: Option<Value>,
    success: bool,
    created_at: DateTime<Utc>,
}

impl LogisticsMatchHitRow {
    fn into_hit(self, tenant_id: TenantId) -> LogisticsMatchHit {
        LogisticsMatchHit {
            hit_id: self.hit_id,
            tenant_id,
            rule_id: self.rule_id,
            shipment_id: self.shipment_id,
            order_id: self.order_id,
            product_id: self.product_id,
            hit_reason: self.hit_reason,
            result_json: self.result_json,
            success: self.success,
            created_at: self.created_at,
        }
    }
}
