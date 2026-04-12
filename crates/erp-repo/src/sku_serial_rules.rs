use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuSerialRule {
    pub rule_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: Option<uuid::Uuid>,
    pub name: String,
    pub prefix: String,
    pub seq_length: i32,
    pub increment_step: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SkuSerialRuleListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub enabled: Option<bool>,
    pub offset: i64,
    pub limit: i64,
}

pub struct SkuSerialRuleListResult {
    pub items: Vec<SkuSerialRule>,
    pub total: i64,
}

pub struct SkuSerialRuleCreate {
    pub tenant_id: TenantId,
    pub sku_id: Option<uuid::Uuid>,
    pub name: String,
    pub prefix: String,
    pub seq_length: i32,
    pub increment_step: i32,
    pub enabled: bool,
}

pub struct SkuSerialRuleUpdate {
    pub sku_id: Option<Option<uuid::Uuid>>,
    pub name: Option<String>,
    pub prefix: Option<String>,
    pub seq_length: Option<i32>,
    pub increment_step: Option<i32>,
    pub enabled: Option<bool>,
}

pub struct SkuSerialRulesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SkuSerialRulesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_rules(
        &self,
        tenant_id: TenantId,
        filters: SkuSerialRuleListFilters,
    ) -> anyhow::Result<SkuSerialRuleListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from sku_serial_rules where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                rule_id, sku_id, name, prefix, seq_length, increment_step,
                enabled, created_at, updated_at
            from sku_serial_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by updated_at desc, rule_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            prefix: String,
            seq_length: i32,
            increment_step: i32,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| SkuSerialRule {
                rule_id: row.rule_id,
                tenant_id,
                sku_id: row.sku_id,
                name: row.name,
                prefix: row.prefix,
                seq_length: row.seq_length,
                increment_step: row.increment_step,
                enabled: row.enabled,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SkuSerialRuleListResult { items, total })
    }

    pub async fn create_rule(&self, input: SkuSerialRuleCreate) -> anyhow::Result<SkuSerialRule> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            prefix: String,
            seq_length: i32,
            increment_step: i32,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into sku_serial_rules(
                tenant_id, sku_id, name, prefix, seq_length, increment_step, enabled
            ) values ($1,$2,$3,$4,$5,$6,$7)
            returning
                rule_id, sku_id, name, prefix, seq_length, increment_step,
                enabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.name)
        .bind(&input.prefix)
        .bind(input.seq_length)
        .bind(input.increment_step)
        .bind(input.enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(SkuSerialRule {
            rule_id: row.rule_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            prefix: row.prefix,
            seq_length: row.seq_length,
            increment_step: row.increment_step,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: SkuSerialRuleUpdate,
    ) -> anyhow::Result<Option<SkuSerialRule>> {
        let mut qb = QueryBuilder::<Postgres>::new("update sku_serial_rules set ");
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
        if let Some(prefix) = input.prefix {
            separated.push("prefix = ");
            separated.push_bind(prefix);
            has_updates = true;
        }
        if let Some(seq_length) = input.seq_length {
            separated.push("seq_length = ");
            separated.push_bind(seq_length);
            has_updates = true;
        }
        if let Some(increment_step) = input.increment_step {
            separated.push("increment_step = ");
            separated.push_bind(increment_step);
            has_updates = true;
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
            has_updates = true;
        }

        if !has_updates {
            return self.get_rule(tenant_id, rule_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(
            " returning rule_id, sku_id, name, prefix, seq_length, increment_step, enabled, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            prefix: String,
            seq_length: i32,
            increment_step: i32,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| SkuSerialRule {
            rule_id: row.rule_id,
            tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            prefix: row.prefix,
            seq_length: row.seq_length,
            increment_step: row.increment_step,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuSerialRule>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            prefix: String,
            seq_length: i32,
            increment_step: i32,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from sku_serial_rules
            where tenant_id = $1 and rule_id = $2
            returning
                rule_id, sku_id, name, prefix, seq_length, increment_step,
                enabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SkuSerialRule {
            rule_id: row.rule_id,
            tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            prefix: row.prefix,
            seq_length: row.seq_length,
            increment_step: row.increment_step,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn get_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuSerialRule>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            prefix: String,
            seq_length: i32,
            increment_step: i32,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                rule_id, sku_id, name, prefix, seq_length, increment_step,
                enabled, created_at, updated_at
            from sku_serial_rules
            where tenant_id = $1 and rule_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SkuSerialRule {
            rule_id: row.rule_id,
            tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            prefix: row.prefix,
            seq_length: row.seq_length,
            increment_step: row.increment_step,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn find_enabled_rule_for_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SkuSerialRule>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            sku_id: Option<uuid::Uuid>,
            name: String,
            prefix: String,
            seq_length: i32,
            increment_step: i32,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                rule_id, sku_id, name, prefix, seq_length, increment_step,
                enabled, created_at, updated_at
            from sku_serial_rules
            where tenant_id = $1
              and enabled = true
              and (sku_id = $2 or sku_id is null)
            order by sku_id desc nulls last, updated_at desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SkuSerialRule {
            rule_id: row.rule_id,
            tenant_id,
            sku_id: row.sku_id,
            name: row.name,
            prefix: row.prefix,
            seq_length: row.seq_length,
            increment_step: row.increment_step,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a SkuSerialRuleListFilters) {
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(enabled) = filters.enabled {
        qb.push(" and enabled = ");
        qb.push_bind(enabled);
    }
}
