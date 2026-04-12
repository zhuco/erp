use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WmsQcSamplingRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub doc_type: String,
    pub sample_rate: i32,
    pub enabled: bool,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WmsQcSamplingRuleCreate {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub doc_type: String,
    pub sample_rate: i32,
    pub enabled: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WmsQcSamplingRuleUpdate {
    pub doc_type: Option<String>,
    pub sample_rate: Option<i32>,
    pub enabled: Option<bool>,
    pub note: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct WmsQcSamplingRuleListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub doc_type: Option<String>,
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct WmsQcSamplingRuleListResult {
    pub items: Vec<WmsQcSamplingRule>,
    pub total: i64,
}

pub struct WmsQcSamplingRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WmsQcSamplingRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: WmsQcSamplingRuleListFilters,
    ) -> anyhow::Result<WmsQcSamplingRuleListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from wms_qc_sampling_rules where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                rule_id, warehouse_id, doc_type, sample_rate,
                enabled, note, created_at, updated_at
            from wms_qc_sampling_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by updated_at desc, rule_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        let rows: Vec<WmsQcSamplingRuleRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_rule(tenant_id))
            .collect();

        Ok(WmsQcSamplingRuleListResult { items, total })
    }

    pub async fn get_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WmsQcSamplingRule>> {
        let row = sqlx::query_as::<_, WmsQcSamplingRuleRow>(
            r#"
            select
                rule_id, warehouse_id, doc_type, sample_rate,
                enabled, note, created_at, updated_at
            from wms_qc_sampling_rules
            where tenant_id = $1 and rule_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_rule(tenant_id)))
    }

    pub async fn create_rule(
        &self,
        input: WmsQcSamplingRuleCreate,
    ) -> anyhow::Result<WmsQcSamplingRule> {
        let row = sqlx::query_as::<_, WmsQcSamplingRuleRow>(
            r#"
            insert into wms_qc_sampling_rules(
                tenant_id, warehouse_id, doc_type, sample_rate, enabled, note
            ) values ($1,$2,$3,$4,$5,$6)
            on conflict (tenant_id, warehouse_id, doc_type)
            do update set
                sample_rate = excluded.sample_rate,
                enabled = excluded.enabled,
                note = excluded.note,
                updated_at = now()
            returning
                rule_id, warehouse_id, doc_type, sample_rate,
                enabled, note, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(&input.doc_type)
        .bind(input.sample_rate)
        .bind(input.enabled)
        .bind(&input.note)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_rule(input.tenant_id))
    }

    pub async fn update_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: WmsQcSamplingRuleUpdate,
    ) -> anyhow::Result<Option<WmsQcSamplingRule>> {
        let mut qb = QueryBuilder::<Postgres>::new("update wms_qc_sampling_rules set ");
        let mut separated = qb.separated(", ");

        if let Some(doc_type) = input.doc_type {
            separated.push("doc_type = ");
            separated.push_bind(doc_type);
        }
        if let Some(sample_rate) = input.sample_rate {
            separated.push("sample_rate = ");
            separated.push_bind(sample_rate);
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
        }
        if let Some(note) = input.note {
            separated.push("note = ");
            separated.push_bind(note);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(
            " returning rule_id, warehouse_id, doc_type, sample_rate, enabled, note, created_at, updated_at",
        );

        let row: Option<WmsQcSamplingRuleRow> =
            qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_rule(tenant_id)))
    }

    pub async fn delete_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WmsQcSamplingRule>> {
        let row = sqlx::query_as::<_, WmsQcSamplingRuleRow>(
            r#"
            delete from wms_qc_sampling_rules
            where tenant_id = $1 and rule_id = $2
            returning
                rule_id, warehouse_id, doc_type, sample_rate,
                enabled, note, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_rule(tenant_id)))
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a WmsQcSamplingRuleListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(doc_type) = &filters.doc_type {
        qb.push(" and doc_type = ");
        qb.push_bind(doc_type);
    }
    if let Some(enabled) = filters.enabled {
        qb.push(" and enabled = ");
        qb.push_bind(enabled);
    }
}

#[derive(sqlx::FromRow)]
struct WmsQcSamplingRuleRow {
    rule_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    doc_type: String,
    sample_rate: i32,
    enabled: bool,
    note: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WmsQcSamplingRuleRow {
    fn into_rule(self, tenant_id: TenantId) -> WmsQcSamplingRule {
        WmsQcSamplingRule {
            rule_id: self.rule_id,
            tenant_id,
            warehouse_id: self.warehouse_id,
            doc_type: self.doc_type,
            sample_rate: self.sample_rate,
            enabled: self.enabled,
            note: self.note,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
