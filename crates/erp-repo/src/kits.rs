use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitSku {
    pub sku_id: uuid::Uuid,
    pub sku_code: String,
    pub name: Option<String>,
    pub barcode: Option<String>,
    pub sku_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitComponent {
    pub component_sku_id: uuid::Uuid,
    pub component_sku_code: String,
    pub component_name: Option<String>,
    pub component_barcode: Option<String>,
    pub qty: i64,
    pub expire_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitDetail {
    pub kit_sku: KitSku,
    pub components: Vec<KitComponent>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitSummary {
    pub kit_sku_id: uuid::Uuid,
    pub kit_sku_code: String,
    pub kit_name: Option<String>,
    pub component_count: i64,
    pub updated_at: Option<DateTime<Utc>>,
}

pub struct KitListFilters {
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct KitProductListFilters {
    pub product_id: uuid::Uuid,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitListResult {
    pub items: Vec<KitSummary>,
    pub total: i64,
}

pub struct KitBomComponentInput {
    pub component_sku_id: uuid::Uuid,
    pub qty: i64,
    pub expire_at: Option<DateTime<Utc>>,
}

pub struct KitBomSaveInput {
    pub tenant_id: TenantId,
    pub kit_sku_id: uuid::Uuid,
    pub components: Vec<KitBomComponentInput>,
}

pub struct KitsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> KitsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_kits(
        &self,
        tenant_id: TenantId,
        filters: KitListFilters,
    ) -> anyhow::Result<KitListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let total: i64 = match &keyword {
            Some(keyword) => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from skus
                    where tenant_id = $1
                      and sku_kind = 'KIT'
                      and (sku_code ilike $2 or name ilike $2)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from skus
                    where tenant_id = $1 and sku_kind = 'KIT'
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                s.sku_id as kit_sku_id,
                s.sku_code as kit_sku_code,
                s.name as kit_name,
                coalesce(count(b.component_sku_id), 0) as component_count,
                s.updated_at
            from skus s
            left join sku_boms b
                on b.tenant_id = s.tenant_id
                and b.bundle_sku_id = s.sku_id
            where s.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and s.sku_kind = 'KIT'");

        if let Some(keyword) = &keyword {
            qb.push(" and (s.sku_code ilike ");
            qb.push_bind(keyword);
            qb.push(" or s.name ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }

        qb.push(" group by s.sku_id");
        qb.push(" order by s.updated_at desc, s.sku_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            kit_sku_id: uuid::Uuid,
            kit_sku_code: String,
            kit_name: Option<String>,
            component_count: i64,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| KitSummary {
                kit_sku_id: row.kit_sku_id,
                kit_sku_code: row.kit_sku_code,
                kit_name: row.kit_name,
                component_count: row.component_count,
                updated_at: Some(row.updated_at),
            })
            .collect();

        Ok(KitListResult { items, total })
    }

    pub async fn list_kits_by_product(
        &self,
        tenant_id: TenantId,
        filters: KitProductListFilters,
    ) -> anyhow::Result<KitListResult> {
        let total: i64 = sqlx::query_scalar(
            r#"
            select count(distinct s.sku_id)
            from sku_boms b
            join skus s
              on s.sku_id = b.bundle_sku_id
             and s.tenant_id = b.tenant_id
            join skus c
              on c.sku_id = b.component_sku_id
             and c.tenant_id = b.tenant_id
            where b.tenant_id = $1 and c.product_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(filters.product_id)
        .fetch_one(self.pool)
        .await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                s.sku_id as kit_sku_id,
                s.sku_code as kit_sku_code,
                s.name as kit_name,
                coalesce(count(b.component_sku_id), 0) as component_count,
                s.updated_at
            from sku_boms b
            join skus s
              on s.sku_id = b.bundle_sku_id
             and s.tenant_id = b.tenant_id
            join skus c
              on c.sku_id = b.component_sku_id
             and c.tenant_id = b.tenant_id
            where b.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and c.product_id = ");
        qb.push_bind(filters.product_id);
        qb.push(" group by s.sku_id");
        qb.push(" order by s.updated_at desc, s.sku_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            kit_sku_id: uuid::Uuid,
            kit_sku_code: String,
            kit_name: Option<String>,
            component_count: i64,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| KitSummary {
                kit_sku_id: row.kit_sku_id,
                kit_sku_code: row.kit_sku_code,
                kit_name: row.kit_name,
                component_count: row.component_count,
                updated_at: Some(row.updated_at),
            })
            .collect();

        Ok(KitListResult { items, total })
    }

    pub async fn get_kit_detail(
        &self,
        tenant_id: TenantId,
        kit_sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<KitDetail>> {
        #[derive(sqlx::FromRow)]
        struct KitRow {
            sku_id: uuid::Uuid,
            sku_code: String,
            name: Option<String>,
            barcode: Option<String>,
            sku_kind: String,
            updated_at: DateTime<Utc>,
        }

        let row: Option<KitRow> = sqlx::query_as(
            r#"
            select sku_id, sku_code, name, barcode, sku_kind, updated_at
            from skus
            where tenant_id = $1 and sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(kit_sku_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        #[derive(sqlx::FromRow)]
        struct ComponentRow {
            component_sku_id: uuid::Uuid,
            component_sku_code: String,
            component_name: Option<String>,
            component_barcode: Option<String>,
            qty: i64,
            expire_at: Option<DateTime<Utc>>,
        }

        let components: Vec<ComponentRow> = sqlx::query_as(
            r#"
            select
                b.component_sku_id,
                s.sku_code as component_sku_code,
                s.name as component_name,
                s.barcode as component_barcode,
                b.qty,
                b.expire_at
            from sku_boms b
            join skus s
              on s.sku_id = b.component_sku_id
             and s.tenant_id = b.tenant_id
            where b.tenant_id = $1 and b.bundle_sku_id = $2
            order by s.sku_code
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(kit_sku_id)
        .fetch_all(self.pool)
        .await?;

        Ok(Some(KitDetail {
            kit_sku: KitSku {
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                name: row.name,
                barcode: row.barcode,
                sku_kind: Some(row.sku_kind),
            },
            components: components
                .into_iter()
                .map(|component| KitComponent {
                    component_sku_id: component.component_sku_id,
                    component_sku_code: component.component_sku_code,
                    component_name: component.component_name,
                    component_barcode: component.component_barcode,
                    qty: component.qty,
                    expire_at: component.expire_at,
                })
                .collect(),
            updated_at: Some(row.updated_at),
        }))
    }

    pub async fn save_kit_bom(&self, input: KitBomSaveInput) -> anyhow::Result<Option<KitDetail>> {
        let mut merged: HashMap<uuid::Uuid, (i64, Option<DateTime<Utc>>)> = HashMap::new();
        for component in &input.components {
            if component.qty <= 0 {
                return Err(anyhow!("component qty must be greater than 0"));
            }
            if component.component_sku_id == input.kit_sku_id {
                return Err(anyhow!("component sku cannot equal kit sku"));
            }
            let entry = merged
                .entry(component.component_sku_id)
                .or_insert((0, component.expire_at));
            entry.0 = entry
                .0
                .checked_add(component.qty)
                .ok_or_else(|| anyhow!("component qty overflow"))?;
            if let Some(expire_at) = component.expire_at {
                entry.1 = match entry.1 {
                    Some(existing) if existing <= expire_at => Some(existing),
                    _ => Some(expire_at),
                };
            }
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct KitRow {
            sku_kind: String,
        }

        let kit_row: Option<KitRow> = sqlx::query_as(
            r#"
            select sku_kind
            from skus
            where tenant_id = $1 and sku_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.kit_sku_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(kit_row) = kit_row else {
            return Ok(None);
        };

        if !kit_row.sku_kind.eq_ignore_ascii_case("KIT") {
            sqlx::query(
                r#"
                update skus
                set sku_kind = 'KIT',
                    updated_at = now()
                where tenant_id = $1 and sku_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.kit_sku_id)
            .execute(&mut *tx)
            .await?;
        }

        let current_version: i32 = sqlx::query_scalar(
            r#"
            select coalesce(max(bom_version), 0)
            from sku_boms
            where tenant_id = $1 and bundle_sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.kit_sku_id)
        .fetch_one(&mut *tx)
        .await?;
        let next_version = current_version.saturating_add(1);

        sqlx::query(
            r#"
            delete from sku_boms
            where tenant_id = $1 and bundle_sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.kit_sku_id)
        .execute(&mut *tx)
        .await?;

        if !merged.is_empty() {
            let mut qb = QueryBuilder::<Postgres>::new(
                "insert into sku_boms (tenant_id, bundle_sku_id, component_sku_id, qty, bom_version, expire_at) ",
            );
            qb.push_values(
                merged.iter(),
                |mut row, (component_sku_id, (qty, expire_at))| {
                    row.push_bind(uuid::Uuid::from(input.tenant_id));
                    row.push_bind(input.kit_sku_id);
                    row.push_bind(component_sku_id);
                    row.push_bind(*qty);
                    row.push_bind(next_version);
                    row.push_bind(expire_at);
                },
            );
            qb.build().execute(&mut *tx).await?;
        }

        sqlx::query(
            r#"
            update skus
            set updated_at = now()
            where tenant_id = $1 and sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.kit_sku_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_kit_detail(input.tenant_id, input.kit_sku_id).await
    }
}
