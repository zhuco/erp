use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sku {
    pub sku_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub product_id: uuid::Uuid,
    pub sku_code: String,
    pub barcode: Option<String>,
    pub name: Option<String>,
    pub note: Option<String>,
    pub spec: Value,
    pub tags: Vec<String>,
    pub image_urls: Vec<String>,
    pub weight_gram: Option<i32>,
    pub length_mm: Option<i32>,
    pub width_mm: Option<i32>,
    pub height_mm: Option<i32>,
    pub volume_cm3: Option<i32>,
    pub sku_kind: String,
    pub cost_cent: Option<i64>,
    pub is_active: bool,
    pub status: String,
    pub batch_enabled: bool,
    pub serial_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SkuCursor {
    pub created_at: DateTime<Utc>,
    pub sku_id: uuid::Uuid,
}

impl SkuCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.sku_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let sku_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self { created_at, sku_id })
    }
}

pub struct SkuListFilters {
    pub product_id: Option<uuid::Uuid>,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub keyword: Option<String>,
    pub is_active: Option<bool>,
    pub cursor: Option<SkuCursor>,
    pub limit: i64,
}

pub struct SkuListResult {
    pub items: Vec<Sku>,
    pub next_cursor: Option<SkuCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuSearchItem {
    pub sku_id: uuid::Uuid,
    pub sku_code: String,
    pub name: Option<String>,
    pub barcode: Option<String>,
    pub sku_kind: Option<String>,
}

pub struct SkuSearchFilters {
    pub keyword: Option<String>,
    pub sku_kind: Option<String>,
    pub limit: i64,
}

pub struct SkuCreate {
    pub tenant_id: TenantId,
    pub product_id: uuid::Uuid,
    pub sku_code: String,
    pub barcode: Option<String>,
    pub name: Option<String>,
    pub note: Option<String>,
    pub spec: Value,
    pub tags: Vec<String>,
    pub image_urls: Vec<String>,
    pub weight_gram: Option<i32>,
    pub length_mm: Option<i32>,
    pub width_mm: Option<i32>,
    pub height_mm: Option<i32>,
    pub volume_cm3: Option<i32>,
    pub sku_kind: String,
    pub cost_cent: Option<i64>,
    pub is_active: bool,
    pub status: Option<String>,
    pub batch_enabled: bool,
    pub serial_enabled: bool,
}

pub struct SkuUpdate {
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub name: Option<String>,
    pub note: Option<String>,
    pub spec: Option<Value>,
    pub tags: Option<Vec<String>>,
    pub image_urls: Option<Vec<String>>,
    pub weight_gram: Option<i32>,
    pub length_mm: Option<i32>,
    pub width_mm: Option<i32>,
    pub height_mm: Option<i32>,
    pub volume_cm3: Option<i32>,
    pub sku_kind: Option<String>,
    pub cost_cent: Option<i64>,
    pub is_active: Option<bool>,
    pub status: Option<String>,
    pub batch_enabled: Option<bool>,
    pub serial_enabled: Option<bool>,
}

pub struct SkusRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SkusRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Sku>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            product_id: uuid::Uuid,
            sku_code: String,
            barcode: Option<String>,
            name: Option<String>,
            note: Option<String>,
            spec: Value,
            tags: Vec<String>,
            image_urls: Vec<String>,
            weight_gram: Option<i32>,
            length_mm: Option<i32>,
            width_mm: Option<i32>,
            height_mm: Option<i32>,
            volume_cm3: Option<i32>,
            sku_kind: String,
            cost_cent: Option<i64>,
            is_active: bool,
            status: String,
            batch_enabled: bool,
            serial_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                s.sku_id, s.product_id, s.sku_code, s.barcode, s.name, s.note, s.spec,
                s.tags, s.image_urls,
                s.weight_gram, s.length_mm, s.width_mm, s.height_mm, s.volume_cm3,
                s.sku_kind,
                case
                    when s.sku_kind = 'KIT' then (
                        select coalesce(sum(b.qty * coalesce(c.cost_cent, 0)), 0)
                        from sku_boms b
                        join skus c
                            on c.sku_id = b.component_sku_id
                            and c.tenant_id = s.tenant_id
                        where b.tenant_id = s.tenant_id
                          and b.bundle_sku_id = s.sku_id
                    )
                    else s.cost_cent
                end as cost_cent,
                s.is_active, s.status, s.batch_enabled, s.serial_enabled,
                s.created_at, s.updated_at
            from skus s
            where s.tenant_id = $1 and s.sku_id = $2 and s.status <> 'DELETED'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Sku {
            sku_id: row.sku_id,
            tenant_id,
            product_id: row.product_id,
            sku_code: row.sku_code,
            barcode: row.barcode,
            name: row.name,
            note: row.note,
            spec: row.spec,
            tags: row.tags,
            image_urls: row.image_urls,
            weight_gram: row.weight_gram,
            length_mm: row.length_mm,
            width_mm: row.width_mm,
            height_mm: row.height_mm,
            volume_cm3: row.volume_cm3,
            sku_kind: row.sku_kind,
            cost_cent: row.cost_cent,
            is_active: row.is_active,
            status: row.status,
            batch_enabled: row.batch_enabled,
            serial_enabled: row.serial_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_skus(
        &self,
        tenant_id: TenantId,
        filters: SkuListFilters,
    ) -> anyhow::Result<SkuListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                s.sku_id, s.product_id, s.sku_code, s.barcode, s.name, s.note, s.spec,
                s.tags, s.image_urls,
                s.weight_gram, s.length_mm, s.width_mm, s.height_mm, s.volume_cm3,
                s.sku_kind,
                case
                    when s.sku_kind = 'KIT' then (
                        select coalesce(sum(b.qty * coalesce(c.cost_cent, 0)), 0)
                        from sku_boms b
                        join skus c
                            on c.sku_id = b.component_sku_id
                            and c.tenant_id = s.tenant_id
                        where b.tenant_id = s.tenant_id
                          and b.bundle_sku_id = s.sku_id
                    )
                    else s.cost_cent
                end as cost_cent,
                s.is_active, s.status, s.batch_enabled, s.serial_enabled,
                s.created_at, s.updated_at
            from skus s
            where s.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and s.status <> 'DELETED'");

        if let Some(product_id) = filters.product_id {
            qb.push(" and s.product_id = ");
            qb.push_bind(product_id);
        }
        if let Some(sku_code) = filters.sku_code {
            qb.push(" and s.sku_code = ");
            qb.push_bind(sku_code);
        }
        if let Some(barcode) = filters.barcode {
            qb.push(" and s.barcode = ");
            qb.push_bind(barcode);
        }
        if let Some(keyword) = &keyword {
            qb.push(" and (s.sku_code ilike ");
            qb.push_bind(keyword);
            qb.push(" or s.barcode ilike ");
            qb.push_bind(keyword);
            qb.push(" or s.name ilike ");
            qb.push_bind(keyword);
            qb.push(" or exists (select 1 from unnest(s.tags) tag where tag ilike ");
            qb.push_bind(keyword);
            qb.push("))");
        }
        if let Some(is_active) = filters.is_active {
            qb.push(" and s.is_active = ");
            qb.push_bind(is_active);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (s.created_at, s.sku_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.sku_id);
            qb.push(")");
        }

        qb.push(" order by s.created_at desc, s.sku_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            product_id: uuid::Uuid,
            sku_code: String,
            barcode: Option<String>,
            name: Option<String>,
            note: Option<String>,
            spec: Value,
            tags: Vec<String>,
            image_urls: Vec<String>,
            weight_gram: Option<i32>,
            length_mm: Option<i32>,
            width_mm: Option<i32>,
            height_mm: Option<i32>,
            volume_cm3: Option<i32>,
            sku_kind: String,
            cost_cent: Option<i64>,
            is_active: bool,
            status: String,
            batch_enabled: bool,
            serial_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| SkuCursor {
            created_at: row.created_at,
            sku_id: row.sku_id,
        });

        let items = rows
            .into_iter()
            .map(|row| Sku {
                sku_id: row.sku_id,
                tenant_id,
                product_id: row.product_id,
                sku_code: row.sku_code,
                barcode: row.barcode,
                name: row.name,
                note: row.note,
                spec: row.spec,
                tags: row.tags,
                image_urls: row.image_urls,
                weight_gram: row.weight_gram,
                length_mm: row.length_mm,
                width_mm: row.width_mm,
                height_mm: row.height_mm,
                volume_cm3: row.volume_cm3,
                sku_kind: row.sku_kind,
                cost_cent: row.cost_cent,
                is_active: row.is_active,
                status: row.status,
                batch_enabled: row.batch_enabled,
                serial_enabled: row.serial_enabled,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SkuListResult { items, next_cursor })
    }

    pub async fn search_skus(
        &self,
        tenant_id: TenantId,
        filters: SkuSearchFilters,
    ) -> anyhow::Result<Vec<SkuSearchItem>> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select sku_id, sku_code, name, barcode, sku_kind
            from skus
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and status <> 'DELETED'");

        if let Some(keyword) = &keyword {
            qb.push(" and (sku_code ilike ");
            qb.push_bind(keyword);
            qb.push(" or barcode ilike ");
            qb.push_bind(keyword);
            qb.push(" or name ilike ");
            qb.push_bind(keyword);
            qb.push(" or exists (select 1 from unnest(tags) tag where tag ilike ");
            qb.push_bind(keyword);
            qb.push("))");
        }
        if let Some(sku_kind) = filters.sku_kind {
            qb.push(" and sku_kind = ");
            qb.push_bind(sku_kind);
        }

        qb.push(" order by updated_at desc, sku_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            sku_code: String,
            name: Option<String>,
            barcode: Option<String>,
            sku_kind: Option<String>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| SkuSearchItem {
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                name: row.name,
                barcode: row.barcode,
                sku_kind: row.sku_kind,
            })
            .collect())
    }

    pub async fn create_sku(&self, input: SkuCreate) -> anyhow::Result<Sku> {
        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            product_id: uuid::Uuid,
            sku_code: String,
            barcode: Option<String>,
            name: Option<String>,
            note: Option<String>,
            spec: Value,
            tags: Vec<String>,
            image_urls: Vec<String>,
            weight_gram: Option<i32>,
            length_mm: Option<i32>,
            width_mm: Option<i32>,
            height_mm: Option<i32>,
            volume_cm3: Option<i32>,
            sku_kind: String,
            cost_cent: Option<i64>,
            is_active: bool,
            status: String,
            batch_enabled: bool,
            serial_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let status = input.status.unwrap_or_else(|| {
            if input.is_active {
                "ACTIVE"
            } else {
                "INACTIVE"
            }
            .to_string()
        });
        let row: Row = sqlx::query_as(
            r#"
            insert into skus(
                tenant_id, product_id, sku_code, barcode, name, note, spec, tags, image_urls,
                weight_gram, length_mm, width_mm, height_mm, volume_cm3,
                sku_kind, cost_cent, is_active, status, batch_enabled, serial_enabled
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)
            returning
                sku_id, product_id, sku_code, barcode, name, note, spec, tags, image_urls,
                weight_gram, length_mm, width_mm, height_mm, volume_cm3,
                sku_kind, cost_cent, is_active, status, batch_enabled, serial_enabled,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.product_id)
        .bind(&input.sku_code)
        .bind(&input.barcode)
        .bind(&input.name)
        .bind(&input.note)
        .bind(&input.spec)
        .bind(&input.tags)
        .bind(&input.image_urls)
        .bind(input.weight_gram)
        .bind(input.length_mm)
        .bind(input.width_mm)
        .bind(input.height_mm)
        .bind(input.volume_cm3)
        .bind(&input.sku_kind)
        .bind(input.cost_cent)
        .bind(input.is_active)
        .bind(&status)
        .bind(input.batch_enabled)
        .bind(input.serial_enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(Sku {
            sku_id: row.sku_id,
            tenant_id: input.tenant_id,
            product_id: row.product_id,
            sku_code: row.sku_code,
            barcode: row.barcode,
            name: row.name,
            note: row.note,
            spec: row.spec,
            tags: row.tags,
            image_urls: row.image_urls,
            weight_gram: row.weight_gram,
            length_mm: row.length_mm,
            width_mm: row.width_mm,
            height_mm: row.height_mm,
            volume_cm3: row.volume_cm3,
            sku_kind: row.sku_kind,
            cost_cent: row.cost_cent,
            is_active: row.is_active,
            status: row.status,
            batch_enabled: row.batch_enabled,
            serial_enabled: row.serial_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        input: SkuUpdate,
    ) -> anyhow::Result<Option<Sku>> {
        let mut qb = QueryBuilder::<Postgres>::new("update skus set ");
        let mut separated = qb.separated(", ");

        if let Some(sku_code) = input.sku_code {
            separated.push("sku_code = ");
            separated.push_bind(sku_code);
        }
        if let Some(barcode) = input.barcode {
            separated.push("barcode = ");
            separated.push_bind(barcode);
        }
        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(note) = input.note {
            separated.push("note = ");
            separated.push_bind(note);
        }
        if let Some(spec) = input.spec {
            separated.push("spec = ");
            separated.push_bind(spec);
        }
        if let Some(tags) = input.tags {
            separated.push("tags = ");
            separated.push_bind(tags);
        }
        if let Some(image_urls) = input.image_urls {
            separated.push("image_urls = ");
            separated.push_bind(image_urls);
        }
        if let Some(weight_gram) = input.weight_gram {
            separated.push("weight_gram = ");
            separated.push_bind(weight_gram);
        }
        if let Some(length_mm) = input.length_mm {
            separated.push("length_mm = ");
            separated.push_bind(length_mm);
        }
        if let Some(width_mm) = input.width_mm {
            separated.push("width_mm = ");
            separated.push_bind(width_mm);
        }
        if let Some(height_mm) = input.height_mm {
            separated.push("height_mm = ");
            separated.push_bind(height_mm);
        }
        if let Some(volume_cm3) = input.volume_cm3 {
            separated.push("volume_cm3 = ");
            separated.push_bind(volume_cm3);
        }
        if let Some(sku_kind) = input.sku_kind {
            separated.push("sku_kind = ");
            separated.push_bind(sku_kind);
        }
        if let Some(cost_cent) = input.cost_cent {
            separated.push("cost_cent = ");
            separated.push_bind(cost_cent);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status.clone());
            separated.push("is_active = ");
            separated.push_bind(status.eq_ignore_ascii_case("ACTIVE"));
        } else if let Some(is_active) = input.is_active {
            separated.push("is_active = ");
            separated.push_bind(is_active);
            separated.push("status = ");
            separated.push_bind(if is_active { "ACTIVE" } else { "INACTIVE" });
        }
        if let Some(batch_enabled) = input.batch_enabled {
            separated.push("batch_enabled = ");
            separated.push_bind(batch_enabled);
        }
        if let Some(serial_enabled) = input.serial_enabled {
            separated.push("serial_enabled = ");
            separated.push_bind(serial_enabled);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and sku_id = ");
        qb.push_bind(sku_id);
        qb.push(" and status <> 'DELETED'");
        qb.push(
            r#"
            returning
                sku_id, product_id, sku_code, barcode, name, note, spec, tags, image_urls,
                weight_gram, length_mm, width_mm, height_mm, volume_cm3,
                sku_kind, cost_cent, is_active, status, batch_enabled, serial_enabled,
                created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            product_id: uuid::Uuid,
            sku_code: String,
            barcode: Option<String>,
            name: Option<String>,
            note: Option<String>,
            spec: Value,
            tags: Vec<String>,
            image_urls: Vec<String>,
            weight_gram: Option<i32>,
            length_mm: Option<i32>,
            width_mm: Option<i32>,
            height_mm: Option<i32>,
            volume_cm3: Option<i32>,
            sku_kind: String,
            cost_cent: Option<i64>,
            is_active: bool,
            status: String,
            batch_enabled: bool,
            serial_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| Sku {
            sku_id: row.sku_id,
            tenant_id,
            product_id: row.product_id,
            sku_code: row.sku_code,
            barcode: row.barcode,
            name: row.name,
            note: row.note,
            spec: row.spec,
            tags: row.tags,
            image_urls: row.image_urls,
            weight_gram: row.weight_gram,
            length_mm: row.length_mm,
            width_mm: row.width_mm,
            height_mm: row.height_mm,
            volume_cm3: row.volume_cm3,
            sku_kind: row.sku_kind,
            cost_cent: row.cost_cent,
            is_active: row.is_active,
            status: row.status,
            batch_enabled: row.batch_enabled,
            serial_enabled: row.serial_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_sku(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Sku>> {
        let sku_kind: Option<String> = sqlx::query_scalar(
            r#"
            select sku_kind
            from skus
            where tenant_id = $1 and sku_id = $2 and status <> 'DELETED'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_optional(self.pool)
        .await?;

        if let Some(kind) = sku_kind {
            if kind.eq_ignore_ascii_case("KIT") {
                let count: i64 = sqlx::query_scalar(
                    r#"
                    select count(*)
                    from order_items oi
                    join orders o
                      on o.tenant_id = oi.tenant_id
                     and o.order_id = oi.order_id
                     and o.order_month = oi.order_month
                    where oi.tenant_id = $1
                      and oi.sku_id = $2
                      and o.status not in ('COMPLETED', 'CANCELLED')
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(sku_id.to_string())
                .fetch_one(self.pool)
                .await?;

                if count > 0 {
                    return Err(anyhow!("kit has active orders"));
                }
            }
        }

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_id: uuid::Uuid,
            product_id: uuid::Uuid,
            sku_code: String,
            barcode: Option<String>,
            name: Option<String>,
            note: Option<String>,
            spec: Value,
            tags: Vec<String>,
            image_urls: Vec<String>,
            weight_gram: Option<i32>,
            length_mm: Option<i32>,
            width_mm: Option<i32>,
            height_mm: Option<i32>,
            volume_cm3: Option<i32>,
            sku_kind: String,
            cost_cent: Option<i64>,
            is_active: bool,
            status: String,
            batch_enabled: bool,
            serial_enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update skus
            set status = 'DELETED',
                is_active = false,
                updated_at = now()
            where tenant_id = $1 and sku_id = $2 and status <> 'DELETED'
            returning
                sku_id, product_id, sku_code, barcode, name, note, spec, tags, image_urls,
                weight_gram, length_mm, width_mm, height_mm, volume_cm3,
                sku_kind, cost_cent, is_active, status, batch_enabled, serial_enabled,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Sku {
            sku_id: row.sku_id,
            tenant_id,
            product_id: row.product_id,
            sku_code: row.sku_code,
            barcode: row.barcode,
            name: row.name,
            note: row.note,
            spec: row.spec,
            tags: row.tags,
            image_urls: row.image_urls,
            weight_gram: row.weight_gram,
            length_mm: row.length_mm,
            width_mm: row.width_mm,
            height_mm: row.height_mm,
            volume_cm3: row.volume_cm3,
            sku_kind: row.sku_kind,
            cost_cent: row.cost_cent,
            is_active: row.is_active,
            status: row.status,
            batch_enabled: row.batch_enabled,
            serial_enabled: row.serial_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
