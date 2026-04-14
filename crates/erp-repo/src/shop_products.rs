use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{Platform, ShopId, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopProduct {
    pub product_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_product_id: String,
    pub merchant_code: Option<String>,
    pub title: String,
    pub product_type: String,
    pub status: String,
    pub platform_updated_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopProductSku {
    pub sku_record_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_product_id: String,
    pub platform_sku_id: String,
    pub merchant_sku_code: Option<String>,
    pub title: Option<String>,
    pub spec_text: Option<String>,
    pub price_cent: Option<i64>,
    pub stock_qty: Option<i64>,
    pub status: String,
    pub barcode: Option<String>,
    pub image_url: Option<String>,
    pub platform_updated_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopProductSkuView {
    pub sku_record_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_product_id: String,
    pub platform_sku_id: String,
    pub merchant_code: Option<String>,
    pub merchant_sku_code: Option<String>,
    pub product_title: String,
    pub sku_title: Option<String>,
    pub spec_text: Option<String>,
    pub price_cent: Option<i64>,
    pub stock_qty: Option<i64>,
    pub status: String,
    pub barcode: Option<String>,
    pub image_url: Option<String>,
    pub platform_updated_at: Option<DateTime<Utc>>,
    pub mapping_id: Option<uuid::Uuid>,
    pub mapped_sku_id: Option<uuid::Uuid>,
    pub mapped_sku_code: Option<String>,
    pub mapped_sku_name: Option<String>,
    pub match_source: Option<String>,
    pub mapping_status: Option<String>,
    pub conflict_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ShopProductUpsert {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_product_id: String,
    pub merchant_code: Option<String>,
    pub title: String,
    pub product_type: String,
    pub status: String,
    pub platform_updated_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
}

#[derive(Debug, Clone)]
pub struct ShopProductSkuUpsert {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_product_id: String,
    pub platform_sku_id: String,
    pub merchant_sku_code: Option<String>,
    pub title: Option<String>,
    pub spec_text: Option<String>,
    pub price_cent: Option<i64>,
    pub stock_qty: Option<i64>,
    pub status: String,
    pub barcode: Option<String>,
    pub image_url: Option<String>,
    pub platform_updated_at: Option<DateTime<Utc>>,
    pub raw_json: Value,
}

#[derive(Debug, Clone)]
pub struct ShopProductSkuListFilters {
    pub platform: Option<Platform>,
    pub shop_ids: Option<Vec<ShopId>>,
    pub product_type: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub match_status: Option<String>,
    pub mapping_status: Option<String>,
    pub conflict_reason: Option<String>,
    pub match_source: Option<String>,
    pub updated_from: Option<DateTime<Utc>>,
    pub updated_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopProductSkuListResult {
    pub items: Vec<ShopProductSkuView>,
    pub total: i64,
}

pub struct ShopProductsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ShopProductsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_product(&self, input: ShopProductUpsert) -> anyhow::Result<ShopProduct> {
        #[derive(sqlx::FromRow)]
        struct Row {
            product_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_product_id: String,
            merchant_code: Option<String>,
            title: String,
            product_type: String,
            status: String,
            platform_updated_at: Option<DateTime<Utc>>,
            raw_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into shop_products(
                tenant_id, shop_id, platform, platform_product_id, merchant_code,
                title, product_type, status, platform_updated_at, raw_json
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            on conflict (tenant_id, shop_id, platform, platform_product_id)
            do update set
                merchant_code = excluded.merchant_code,
                title = excluded.title,
                product_type = excluded.product_type,
                status = excluded.status,
                platform_updated_at = excluded.platform_updated_at,
                raw_json = excluded.raw_json,
                updated_at = now()
            returning
                product_id, tenant_id, shop_id, platform, platform_product_id,
                merchant_code, title, product_type, status, platform_updated_at,
                raw_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(input.platform.to_string())
        .bind(&input.platform_product_id)
        .bind(&input.merchant_code)
        .bind(&input.title)
        .bind(&input.product_type)
        .bind(&input.status)
        .bind(input.platform_updated_at)
        .bind(&input.raw_json)
        .fetch_one(self.pool)
        .await?;

        let platform =
            Platform::from_str(&row.platform).map_err(|e| anyhow!("bad platform in db: {e}"))?;
        Ok(ShopProduct {
            product_id: row.product_id,
            tenant_id: TenantId(row.tenant_id),
            shop_id: ShopId(row.shop_id),
            platform,
            platform_product_id: row.platform_product_id,
            merchant_code: row.merchant_code,
            title: row.title,
            product_type: row.product_type,
            status: row.status,
            platform_updated_at: row.platform_updated_at,
            raw_json: row.raw_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn upsert_sku(&self, input: ShopProductSkuUpsert) -> anyhow::Result<ShopProductSku> {
        #[derive(sqlx::FromRow)]
        struct Row {
            sku_record_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_product_id: String,
            platform_sku_id: String,
            merchant_sku_code: Option<String>,
            title: Option<String>,
            spec_text: Option<String>,
            price_cent: Option<i64>,
            stock_qty: Option<i64>,
            status: String,
            barcode: Option<String>,
            image_url: Option<String>,
            platform_updated_at: Option<DateTime<Utc>>,
            raw_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into shop_product_skus(
                tenant_id, shop_id, platform, platform_product_id, platform_sku_id,
                merchant_sku_code, title, spec_text, price_cent, stock_qty, status,
                barcode, image_url, platform_updated_at, raw_json
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
            on conflict (tenant_id, shop_id, platform, platform_sku_id)
            do update set
                platform_product_id = excluded.platform_product_id,
                merchant_sku_code = excluded.merchant_sku_code,
                title = excluded.title,
                spec_text = excluded.spec_text,
                price_cent = excluded.price_cent,
                stock_qty = excluded.stock_qty,
                status = excluded.status,
                barcode = excluded.barcode,
                image_url = excluded.image_url,
                platform_updated_at = excluded.platform_updated_at,
                raw_json = excluded.raw_json,
                updated_at = now()
            returning
                sku_record_id, tenant_id, shop_id, platform, platform_product_id, platform_sku_id,
                merchant_sku_code, title, spec_text, price_cent, stock_qty, status, barcode,
                image_url, platform_updated_at, raw_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(input.platform.to_string())
        .bind(&input.platform_product_id)
        .bind(&input.platform_sku_id)
        .bind(&input.merchant_sku_code)
        .bind(&input.title)
        .bind(&input.spec_text)
        .bind(input.price_cent)
        .bind(input.stock_qty)
        .bind(&input.status)
        .bind(&input.barcode)
        .bind(&input.image_url)
        .bind(input.platform_updated_at)
        .bind(&input.raw_json)
        .fetch_one(self.pool)
        .await?;

        let platform =
            Platform::from_str(&row.platform).map_err(|e| anyhow!("bad platform in db: {e}"))?;
        Ok(ShopProductSku {
            sku_record_id: row.sku_record_id,
            tenant_id: TenantId(row.tenant_id),
            shop_id: ShopId(row.shop_id),
            platform,
            platform_product_id: row.platform_product_id,
            platform_sku_id: row.platform_sku_id,
            merchant_sku_code: row.merchant_sku_code,
            title: row.title,
            spec_text: row.spec_text,
            price_cent: row.price_cent,
            stock_qty: row.stock_qty,
            status: row.status,
            barcode: row.barcode,
            image_url: row.image_url,
            platform_updated_at: row.platform_updated_at,
            raw_json: row.raw_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn list_sku_views(
        &self,
        tenant_id: TenantId,
        filters: ShopProductSkuListFilters,
    ) -> anyhow::Result<ShopProductSkuListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from shop_product_skus s
            join shop_products p
              on p.tenant_id = s.tenant_id
             and p.shop_id = s.shop_id
             and p.platform = s.platform
             and p.platform_product_id = s.platform_product_id
            left join channel_sku_mappings m
              on m.tenant_id = s.tenant_id
             and m.shop_id = s.shop_id
             and m.platform = s.platform
             and m.platform_sku_id = s.platform_sku_id
            where s.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_sku_filters(&mut count_qb, &filters, keyword.as_deref());
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                s.sku_record_id, s.shop_id, s.platform, s.platform_product_id, s.platform_sku_id,
                p.merchant_code, s.merchant_sku_code, p.title as product_title, s.title as sku_title,
                s.spec_text, s.price_cent, s.stock_qty, s.status, s.barcode, s.image_url,
                s.platform_updated_at,
                m.mapping_id, m.sku_id as mapped_sku_id, m.match_source, m.status as mapping_status,
                m.conflict_reason,
                k.sku_code as mapped_sku_code, k.name as mapped_sku_name,
                s.created_at, s.updated_at
            from shop_product_skus s
            join shop_products p
              on p.tenant_id = s.tenant_id
             and p.shop_id = s.shop_id
             and p.platform = s.platform
             and p.platform_product_id = s.platform_product_id
            left join channel_sku_mappings m
              on m.tenant_id = s.tenant_id
             and m.shop_id = s.shop_id
             and m.platform = s.platform
             and m.platform_sku_id = s.platform_sku_id
            left join skus k
              on k.tenant_id = s.tenant_id
             and k.sku_id = m.sku_id
            where s.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_sku_filters(&mut qb, &filters, keyword.as_deref());
        qb.push(
            " order by coalesce(s.platform_updated_at, s.updated_at) desc, s.sku_record_id desc",
        );
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_record_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_product_id: String,
            platform_sku_id: String,
            merchant_code: Option<String>,
            merchant_sku_code: Option<String>,
            product_title: String,
            sku_title: Option<String>,
            spec_text: Option<String>,
            price_cent: Option<i64>,
            stock_qty: Option<i64>,
            status: String,
            barcode: Option<String>,
            image_url: Option<String>,
            platform_updated_at: Option<DateTime<Utc>>,
            mapping_id: Option<uuid::Uuid>,
            mapped_sku_id: Option<uuid::Uuid>,
            match_source: Option<String>,
            mapping_status: Option<String>,
            conflict_reason: Option<String>,
            mapped_sku_code: Option<String>,
            mapped_sku_name: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let platform = Platform::from_str(&row.platform)
                .map_err(|e| anyhow!("bad platform in db: {e}"))?;
            items.push(ShopProductSkuView {
                sku_record_id: row.sku_record_id,
                tenant_id,
                shop_id: ShopId(row.shop_id),
                platform,
                platform_product_id: row.platform_product_id,
                platform_sku_id: row.platform_sku_id,
                merchant_code: row.merchant_code,
                merchant_sku_code: row.merchant_sku_code,
                product_title: row.product_title,
                sku_title: row.sku_title,
                spec_text: row.spec_text,
                price_cent: row.price_cent,
                stock_qty: row.stock_qty,
                status: row.status,
                barcode: row.barcode,
                image_url: row.image_url,
                platform_updated_at: row.platform_updated_at,
                mapping_id: row.mapping_id,
                mapped_sku_id: row.mapped_sku_id,
                mapped_sku_code: row.mapped_sku_code,
                mapped_sku_name: row.mapped_sku_name,
                match_source: row.match_source,
                mapping_status: row.mapping_status,
                conflict_reason: row.conflict_reason,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(ShopProductSkuListResult { items, total })
    }

    pub async fn list_sku_views_by_ids(
        &self,
        tenant_id: TenantId,
        sku_record_ids: &[uuid::Uuid],
    ) -> anyhow::Result<Vec<ShopProductSkuView>> {
        if sku_record_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                s.sku_record_id, s.shop_id, s.platform, s.platform_product_id, s.platform_sku_id,
                p.merchant_code, s.merchant_sku_code, p.title as product_title, s.title as sku_title,
                s.spec_text, s.price_cent, s.stock_qty, s.status, s.barcode, s.image_url,
                s.platform_updated_at,
                m.mapping_id, m.sku_id as mapped_sku_id, m.match_source, m.status as mapping_status,
                m.conflict_reason,
                k.sku_code as mapped_sku_code, k.name as mapped_sku_name,
                s.created_at, s.updated_at
            from shop_product_skus s
            join shop_products p
              on p.tenant_id = s.tenant_id
             and p.shop_id = s.shop_id
             and p.platform = s.platform
             and p.platform_product_id = s.platform_product_id
            left join channel_sku_mappings m
              on m.tenant_id = s.tenant_id
             and m.shop_id = s.shop_id
             and m.platform = s.platform
             and m.platform_sku_id = s.platform_sku_id
            left join skus k
              on k.tenant_id = s.tenant_id
             and k.sku_id = m.sku_id
            where s.tenant_id = 
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and s.sku_record_id = any(");
        qb.push_bind(sku_record_ids);
        qb.push(")");
        qb.push(" order by s.created_at desc");

        #[derive(sqlx::FromRow)]
        struct Row {
            sku_record_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_product_id: String,
            platform_sku_id: String,
            merchant_code: Option<String>,
            merchant_sku_code: Option<String>,
            product_title: String,
            sku_title: Option<String>,
            spec_text: Option<String>,
            price_cent: Option<i64>,
            stock_qty: Option<i64>,
            status: String,
            barcode: Option<String>,
            image_url: Option<String>,
            platform_updated_at: Option<DateTime<Utc>>,
            mapping_id: Option<uuid::Uuid>,
            mapped_sku_id: Option<uuid::Uuid>,
            match_source: Option<String>,
            mapping_status: Option<String>,
            conflict_reason: Option<String>,
            mapped_sku_code: Option<String>,
            mapped_sku_name: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let platform = Platform::from_str(&row.platform)
                .map_err(|e| anyhow!("bad platform in db: {e}"))?;
            items.push(ShopProductSkuView {
                sku_record_id: row.sku_record_id,
                tenant_id,
                shop_id: ShopId(row.shop_id),
                platform,
                platform_product_id: row.platform_product_id,
                platform_sku_id: row.platform_sku_id,
                merchant_code: row.merchant_code,
                merchant_sku_code: row.merchant_sku_code,
                product_title: row.product_title,
                sku_title: row.sku_title,
                spec_text: row.spec_text,
                price_cent: row.price_cent,
                stock_qty: row.stock_qty,
                status: row.status,
                barcode: row.barcode,
                image_url: row.image_url,
                platform_updated_at: row.platform_updated_at,
                mapping_id: row.mapping_id,
                mapped_sku_id: row.mapped_sku_id,
                mapped_sku_code: row.mapped_sku_code,
                mapped_sku_name: row.mapped_sku_name,
                match_source: row.match_source,
                mapping_status: row.mapping_status,
                conflict_reason: row.conflict_reason,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(items)
    }
}

fn apply_sku_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ShopProductSkuListFilters,
    keyword: Option<&'a str>,
) {
    if let Some(platform) = filters.platform {
        qb.push(" and s.platform = ");
        qb.push_bind(platform.to_string());
    }
    if let Some(shop_ids) = filters.shop_ids.as_ref() {
        if !shop_ids.is_empty() {
            let ids: Vec<uuid::Uuid> = shop_ids.iter().copied().map(uuid::Uuid::from).collect();
            qb.push(" and s.shop_id = any(");
            qb.push_bind(ids);
            qb.push(")");
        }
    }
    if let Some(status) = filters.status.as_deref() {
        qb.push(" and s.status = ");
        qb.push_bind(status);
    }
    if let Some(product_type) = filters.product_type.as_deref() {
        qb.push(" and p.product_type = ");
        qb.push_bind(product_type);
    }
    if let Some(keyword) = keyword {
        qb.push(" and (");
        qb.push("p.title ilike ");
        qb.push_bind(keyword);
        qb.push(" or p.merchant_code ilike ");
        qb.push_bind(keyword);
        qb.push(" or s.platform_sku_id ilike ");
        qb.push_bind(keyword);
        qb.push(" or s.merchant_sku_code ilike ");
        qb.push_bind(keyword);
        qb.push(" or s.title ilike ");
        qb.push_bind(keyword);
        qb.push(")");
    }
    if let Some(from) = filters.updated_from {
        qb.push(" and coalesce(s.platform_updated_at, s.updated_at) >= ");
        qb.push_bind(from);
    }
    if let Some(to) = filters.updated_to {
        qb.push(" and coalesce(s.platform_updated_at, s.updated_at) <= ");
        qb.push_bind(to);
    }
    if let Some(match_status) = filters.match_status.as_deref() {
        match match_status {
            "MATCHED" => {
                qb.push(" and m.mapping_id is not null");
            }
            "UNMATCHED" => {
                qb.push(" and m.mapping_id is null");
            }
            "AUTO" => {
                qb.push(" and m.mapping_id is not null and m.match_source = ");
                qb.push_bind("AUTO");
            }
            "MANUAL" => {
                qb.push(" and m.mapping_id is not null and m.match_source = ");
                qb.push_bind("MANUAL");
            }
            _ => {}
        }
    }
    if let Some(mapping_status) = filters.mapping_status.as_deref() {
        qb.push(" and m.status = ");
        qb.push_bind(mapping_status);
    }
    if let Some(conflict_reason) = filters.conflict_reason.as_deref() {
        qb.push(" and m.conflict_reason = ");
        qb.push_bind(conflict_reason);
    }
    if let Some(match_source) = filters.match_source.as_deref() {
        qb.push(" and m.match_source = ");
        qb.push_bind(match_source);
    }
}
