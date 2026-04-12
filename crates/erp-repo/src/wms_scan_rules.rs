use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WmsScanRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub scenario: String,
    pub scan_mode: String,
    pub block_on_mismatch: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct WmsScanRuleCreate {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub scenario: String,
    pub scan_mode: String,
    pub block_on_mismatch: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct WmsScanRuleUpdate {
    pub scenario: Option<String>,
    pub scan_mode: Option<String>,
    pub block_on_mismatch: Option<bool>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct WmsScanRuleListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub scenario: Option<String>,
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct WmsScanRuleListResult {
    pub items: Vec<WmsScanRule>,
    pub total: i64,
}

pub struct WmsScanRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WmsScanRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: WmsScanRuleListFilters,
    ) -> anyhow::Result<WmsScanRuleListResult> {
        let mut count_qb =
            QueryBuilder::<Postgres>::new("select count(*) from wms_scan_rules where tenant_id = ");
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                rule_id, warehouse_id, scenario, scan_mode, block_on_mismatch,
                enabled, created_at, updated_at
            from wms_scan_rules
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

        let rows: Vec<WmsScanRuleRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_rule(tenant_id))
            .collect();

        Ok(WmsScanRuleListResult { items, total })
    }

    pub async fn get_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WmsScanRule>> {
        let row = sqlx::query_as::<_, WmsScanRuleRow>(
            r#"
            select
                rule_id, warehouse_id, scenario, scan_mode, block_on_mismatch,
                enabled, created_at, updated_at
            from wms_scan_rules
            where tenant_id = $1 and rule_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_rule(tenant_id)))
    }

    pub async fn create_rule(&self, input: WmsScanRuleCreate) -> anyhow::Result<WmsScanRule> {
        let row = sqlx::query_as::<_, WmsScanRuleRow>(
            r#"
            insert into wms_scan_rules(
                tenant_id, warehouse_id, scenario, scan_mode, block_on_mismatch, enabled
            ) values ($1,$2,$3,$4,$5,$6)
            on conflict (tenant_id, warehouse_id, scenario)
            do update set
                scan_mode = excluded.scan_mode,
                block_on_mismatch = excluded.block_on_mismatch,
                enabled = excluded.enabled,
                updated_at = now()
            returning
                rule_id, warehouse_id, scenario, scan_mode, block_on_mismatch,
                enabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(&input.scenario)
        .bind(&input.scan_mode)
        .bind(input.block_on_mismatch)
        .bind(input.enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_rule(input.tenant_id))
    }

    pub async fn update_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: WmsScanRuleUpdate,
    ) -> anyhow::Result<Option<WmsScanRule>> {
        let mut qb = QueryBuilder::<Postgres>::new("update wms_scan_rules set ");
        let mut separated = qb.separated(", ");

        if let Some(scenario) = input.scenario {
            separated.push("scenario = ");
            separated.push_bind(scenario);
        }
        if let Some(scan_mode) = input.scan_mode {
            separated.push("scan_mode = ");
            separated.push_bind(scan_mode);
        }
        if let Some(block_on_mismatch) = input.block_on_mismatch {
            separated.push("block_on_mismatch = ");
            separated.push_bind(block_on_mismatch);
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(
            " returning rule_id, warehouse_id, scenario, scan_mode, block_on_mismatch, enabled, created_at, updated_at",
        );

        let row: Option<WmsScanRuleRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_rule(tenant_id)))
    }

    pub async fn delete_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WmsScanRule>> {
        let row = sqlx::query_as::<_, WmsScanRuleRow>(
            r#"
            delete from wms_scan_rules
            where tenant_id = $1 and rule_id = $2
            returning
                rule_id, warehouse_id, scenario, scan_mode, block_on_mismatch,
                enabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_rule(tenant_id)))
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a WmsScanRuleListFilters) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(scenario) = &filters.scenario {
        qb.push(" and scenario = ");
        qb.push_bind(scenario);
    }
    if let Some(enabled) = filters.enabled {
        qb.push(" and enabled = ");
        qb.push_bind(enabled);
    }
}

#[derive(sqlx::FromRow)]
struct WmsScanRuleRow {
    rule_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    scenario: String,
    scan_mode: String,
    block_on_mismatch: bool,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WmsScanRuleRow {
    fn into_rule(self, tenant_id: TenantId) -> WmsScanRule {
        WmsScanRule {
            rule_id: self.rule_id,
            tenant_id,
            warehouse_id: self.warehouse_id,
            scenario: self.scenario,
            scan_mode: self.scan_mode,
            block_on_mismatch: self.block_on_mismatch,
            enabled: self.enabled,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
