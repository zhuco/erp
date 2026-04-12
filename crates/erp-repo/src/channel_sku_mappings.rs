use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::{Platform, ShopId, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSkuMapping {
    pub mapping_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_sku_id: String,
    pub platform_outer_sku_id: Option<String>,
    pub sku_id: uuid::Uuid,
    pub status: String,
    pub conflict_reason: String,
    pub confidence: i32,
    pub match_source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSkuMappingView {
    pub mapping_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_sku_id: String,
    pub platform_outer_sku_id: Option<String>,
    pub sku_id: uuid::Uuid,
    pub sku_code: String,
    pub barcode: Option<String>,
    pub status: String,
    pub conflict_reason: String,
    pub match_source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ChannelSkuMappingCursor {
    pub created_at: DateTime<Utc>,
    pub mapping_id: uuid::Uuid,
}

impl ChannelSkuMappingCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.mapping_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let mapping_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            mapping_id,
        })
    }
}

pub struct ChannelSkuMappingListFilters {
    pub shop_id: Option<ShopId>,
    pub platform: Option<Platform>,
    pub sku_id: Option<uuid::Uuid>,
    pub platform_sku_id: Option<String>,
    pub platform_outer_sku_id: Option<String>,
    pub status: Option<String>,
    pub cursor: Option<ChannelSkuMappingCursor>,
    pub limit: i64,
}

pub struct ChannelSkuMappingListResult {
    pub items: Vec<ChannelSkuMapping>,
    pub next_cursor: Option<ChannelSkuMappingCursor>,
}

pub struct ChannelSkuMappingViewListFilters {
    pub shop_id: Option<ShopId>,
    pub platform: Option<Platform>,
    pub platform_sku_id: Option<String>,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub status: Option<String>,
    pub conflict_reason: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ChannelSkuMappingViewListResult {
    pub items: Vec<ChannelSkuMappingView>,
    pub total: i64,
}

pub struct ChannelSkuMappingCreate {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub platform: Platform,
    pub platform_sku_id: String,
    pub platform_outer_sku_id: Option<String>,
    pub sku_id: uuid::Uuid,
    pub status: String,
    pub conflict_reason: Option<String>,
    pub confidence: i32,
    pub match_source: String,
}

pub struct ChannelSkuMappingUpdate {
    pub platform_sku_id: Option<String>,
    pub platform_outer_sku_id: Option<String>,
    pub sku_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub conflict_reason: Option<String>,
    pub confidence: Option<i32>,
    pub match_source: Option<String>,
}

pub struct ChannelSkuMappingsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ChannelSkuMappingsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_mappings(
        &self,
        tenant_id: TenantId,
        filters: ChannelSkuMappingListFilters,
    ) -> anyhow::Result<ChannelSkuMappingListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                mapping_id, shop_id, platform, platform_sku_id,
                platform_outer_sku_id, sku_id, status, conflict_reason, confidence, match_source,
                created_at, updated_at
            from channel_sku_mappings
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = filters.shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(uuid::Uuid::from(shop_id));
        }
        if let Some(platform) = filters.platform {
            qb.push(" and platform = ");
            qb.push_bind(platform.to_string());
        }
        if let Some(sku_id) = filters.sku_id {
            qb.push(" and sku_id = ");
            qb.push_bind(sku_id);
        }
        if let Some(platform_sku_id) = filters.platform_sku_id {
            qb.push(" and platform_sku_id = ");
            qb.push_bind(platform_sku_id);
        }
        if let Some(platform_outer_sku_id) = filters.platform_outer_sku_id {
            qb.push(" and platform_outer_sku_id = ");
            qb.push_bind(platform_outer_sku_id);
        }
        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, mapping_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.mapping_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, mapping_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_sku_id: String,
            platform_outer_sku_id: Option<String>,
            sku_id: uuid::Uuid,
            status: String,
            conflict_reason: String,
            confidence: i32,
            match_source: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| ChannelSkuMappingCursor {
            created_at: row.created_at,
            mapping_id: row.mapping_id,
        });

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let platform = Platform::from_str(&row.platform)
                .map_err(|e| anyhow!("bad platform in db: {e}"))?;
            items.push(ChannelSkuMapping {
                mapping_id: row.mapping_id,
                tenant_id,
                shop_id: ShopId(row.shop_id),
                platform,
                platform_sku_id: row.platform_sku_id,
                platform_outer_sku_id: row.platform_outer_sku_id,
                sku_id: row.sku_id,
                status: row.status,
                conflict_reason: row.conflict_reason,
                confidence: row.confidence,
                match_source: row.match_source,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(ChannelSkuMappingListResult { items, next_cursor })
    }

    pub async fn list_mapping_views(
        &self,
        tenant_id: TenantId,
        filters: ChannelSkuMappingViewListFilters,
    ) -> anyhow::Result<ChannelSkuMappingViewListResult> {
        let mut total_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from channel_sku_mappings m
            join skus s
              on s.sku_id = m.sku_id
             and s.tenant_id = m.tenant_id
            where m.tenant_id =
            "#,
        );
        total_qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = filters.shop_id {
            total_qb.push(" and m.shop_id = ");
            total_qb.push_bind(uuid::Uuid::from(shop_id));
        }
        if let Some(platform) = filters.platform {
            total_qb.push(" and m.platform = ");
            total_qb.push_bind(platform.to_string());
        }
        if let Some(platform_sku_id) = filters.platform_sku_id.as_deref() {
            total_qb.push(" and m.platform_sku_id = ");
            total_qb.push_bind(platform_sku_id);
        }
        if let Some(sku_code) = filters.sku_code.as_deref() {
            total_qb.push(" and s.sku_code = ");
            total_qb.push_bind(sku_code);
        }
        if let Some(barcode) = filters.barcode.as_deref() {
            total_qb.push(" and s.barcode = ");
            total_qb.push_bind(barcode);
        }
        if let Some(status) = filters.status.as_deref() {
            total_qb.push(" and m.status = ");
            total_qb.push_bind(status);
        }
        if let Some(conflict_reason) = filters.conflict_reason.as_deref() {
            total_qb.push(" and m.conflict_reason = ");
            total_qb.push_bind(conflict_reason);
        }

        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                m.mapping_id, m.shop_id, m.platform, m.platform_sku_id,
                m.platform_outer_sku_id, m.sku_id, m.status, m.conflict_reason, m.match_source,
                s.sku_code, s.barcode,
                m.created_at, m.updated_at
            from channel_sku_mappings m
            join skus s
              on s.sku_id = m.sku_id
             and s.tenant_id = m.tenant_id
            where m.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = filters.shop_id {
            qb.push(" and m.shop_id = ");
            qb.push_bind(uuid::Uuid::from(shop_id));
        }
        if let Some(platform) = filters.platform {
            qb.push(" and m.platform = ");
            qb.push_bind(platform.to_string());
        }
        if let Some(platform_sku_id) = filters.platform_sku_id {
            qb.push(" and m.platform_sku_id = ");
            qb.push_bind(platform_sku_id);
        }
        if let Some(sku_code) = filters.sku_code {
            qb.push(" and s.sku_code = ");
            qb.push_bind(sku_code);
        }
        if let Some(barcode) = filters.barcode {
            qb.push(" and s.barcode = ");
            qb.push_bind(barcode);
        }
        if let Some(status) = filters.status {
            qb.push(" and m.status = ");
            qb.push_bind(status);
        }
        if let Some(conflict_reason) = filters.conflict_reason {
            qb.push(" and m.conflict_reason = ");
            qb.push_bind(conflict_reason);
        }

        qb.push(" order by m.updated_at desc, m.mapping_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_sku_id: String,
            platform_outer_sku_id: Option<String>,
            sku_id: uuid::Uuid,
            sku_code: String,
            barcode: Option<String>,
            status: String,
            conflict_reason: String,
            match_source: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let platform = Platform::from_str(&row.platform)
                .map_err(|e| anyhow!("bad platform in db: {e}"))?;
            items.push(ChannelSkuMappingView {
                mapping_id: row.mapping_id,
                tenant_id,
                shop_id: ShopId(row.shop_id),
                platform,
                platform_sku_id: row.platform_sku_id,
                platform_outer_sku_id: row.platform_outer_sku_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                barcode: row.barcode,
                status: row.status,
                conflict_reason: row.conflict_reason,
                match_source: row.match_source,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(ChannelSkuMappingViewListResult { items, total })
    }

    pub async fn get_mapping_by_platform_sku(
        &self,
        tenant_id: TenantId,
        shop_id: ShopId,
        platform: Platform,
        platform_sku_id: &str,
    ) -> anyhow::Result<Option<ChannelSkuMapping>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_sku_id: String,
            platform_outer_sku_id: Option<String>,
            sku_id: uuid::Uuid,
            status: String,
            conflict_reason: String,
            confidence: i32,
            match_source: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                mapping_id, shop_id, platform, platform_sku_id,
                platform_outer_sku_id, sku_id, status, conflict_reason, confidence, match_source,
                created_at, updated_at
            from channel_sku_mappings
            where tenant_id = $1
              and shop_id = $2
              and platform = $3
              and platform_sku_id = $4
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .bind(platform.to_string())
        .bind(platform_sku_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let platform =
            Platform::from_str(&row.platform).map_err(|e| anyhow!("bad platform in db: {e}"))?;

        Ok(Some(ChannelSkuMapping {
            mapping_id: row.mapping_id,
            tenant_id,
            shop_id: ShopId(row.shop_id),
            platform,
            platform_sku_id: row.platform_sku_id,
            platform_outer_sku_id: row.platform_outer_sku_id,
            sku_id: row.sku_id,
            status: row.status,
            conflict_reason: row.conflict_reason,
            confidence: row.confidence,
            match_source: row.match_source,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_mapping(
        &self,
        input: ChannelSkuMappingCreate,
    ) -> anyhow::Result<ChannelSkuMapping> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_sku_id: String,
            platform_outer_sku_id: Option<String>,
            sku_id: uuid::Uuid,
            status: String,
            conflict_reason: String,
            confidence: i32,
            match_source: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into channel_sku_mappings(
                tenant_id, shop_id, platform, platform_sku_id, platform_outer_sku_id,
                sku_id, status, conflict_reason, confidence, match_source
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            returning
                mapping_id, shop_id, platform, platform_sku_id,
                platform_outer_sku_id, sku_id, status, conflict_reason, confidence, match_source,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(input.platform.to_string())
        .bind(&input.platform_sku_id)
        .bind(&input.platform_outer_sku_id)
        .bind(input.sku_id)
        .bind(&input.status)
        .bind(input.conflict_reason.unwrap_or_default())
        .bind(input.confidence)
        .bind(&input.match_source)
        .fetch_one(self.pool)
        .await?;

        let platform =
            Platform::from_str(&row.platform).map_err(|e| anyhow!("bad platform in db: {e}"))?;

        Ok(ChannelSkuMapping {
            mapping_id: row.mapping_id,
            tenant_id: input.tenant_id,
            shop_id: ShopId(row.shop_id),
            platform,
            platform_sku_id: row.platform_sku_id,
            platform_outer_sku_id: row.platform_outer_sku_id,
            sku_id: row.sku_id,
            status: row.status,
            conflict_reason: row.conflict_reason,
            confidence: row.confidence,
            match_source: row.match_source,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_mapping(
        &self,
        tenant_id: TenantId,
        mapping_id: uuid::Uuid,
        input: ChannelSkuMappingUpdate,
    ) -> anyhow::Result<Option<ChannelSkuMapping>> {
        let mut qb = QueryBuilder::<Postgres>::new("update channel_sku_mappings set ");
        let mut separated = qb.separated(", ");

        if let Some(platform_sku_id) = input.platform_sku_id {
            separated.push("platform_sku_id = ");
            separated.push_bind(platform_sku_id);
        }
        if let Some(platform_outer_sku_id) = input.platform_outer_sku_id {
            separated.push("platform_outer_sku_id = ");
            separated.push_bind(platform_outer_sku_id);
        }
        if let Some(sku_id) = input.sku_id {
            separated.push("sku_id = ");
            separated.push_bind(sku_id);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(conflict_reason) = input.conflict_reason {
            separated.push("conflict_reason = ");
            separated.push_bind(conflict_reason);
        }
        if let Some(confidence) = input.confidence {
            separated.push("confidence = ");
            separated.push_bind(confidence);
        }
        if let Some(match_source) = input.match_source {
            separated.push("match_source = ");
            separated.push_bind(match_source);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and mapping_id = ");
        qb.push_bind(mapping_id);
        qb.push(
            r#"
            returning
                mapping_id, shop_id, platform, platform_sku_id,
                platform_outer_sku_id, sku_id, status, conflict_reason, confidence, match_source,
                created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_sku_id: String,
            platform_outer_sku_id: Option<String>,
            sku_id: uuid::Uuid,
            status: String,
            conflict_reason: String,
            confidence: i32,
            match_source: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let platform =
            Platform::from_str(&row.platform).map_err(|e| anyhow!("bad platform in db: {e}"))?;
        Ok(Some(ChannelSkuMapping {
            mapping_id: row.mapping_id,
            tenant_id,
            shop_id: ShopId(row.shop_id),
            platform,
            platform_sku_id: row.platform_sku_id,
            platform_outer_sku_id: row.platform_outer_sku_id,
            sku_id: row.sku_id,
            status: row.status,
            conflict_reason: row.conflict_reason,
            confidence: row.confidence,
            match_source: row.match_source,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_mapping(
        &self,
        tenant_id: TenantId,
        mapping_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ChannelSkuMapping>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            mapping_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_sku_id: String,
            platform_outer_sku_id: Option<String>,
            sku_id: uuid::Uuid,
            status: String,
            conflict_reason: String,
            confidence: i32,
            match_source: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from channel_sku_mappings
            where tenant_id = $1 and mapping_id = $2
            returning
                mapping_id, shop_id, platform, platform_sku_id,
                platform_outer_sku_id, sku_id, status, conflict_reason, confidence, match_source,
                created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(mapping_id)
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let platform =
            Platform::from_str(&row.platform).map_err(|e| anyhow!("bad platform in db: {e}"))?;

        Ok(Some(ChannelSkuMapping {
            mapping_id: row.mapping_id,
            tenant_id,
            shop_id: ShopId(row.shop_id),
            platform,
            platform_sku_id: row.platform_sku_id,
            platform_outer_sku_id: row.platform_outer_sku_id,
            sku_id: row.sku_id,
            status: row.status,
            conflict_reason: row.conflict_reason,
            confidence: row.confidence,
            match_source: row.match_source,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn resolve_internal_sku(
        &self,
        tenant_id: TenantId,
        shop_id: ShopId,
        platform: Platform,
        platform_sku_id: &str,
        outer_sku_id: Option<&str>,
    ) -> anyhow::Result<Option<uuid::Uuid>> {
        let sku_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            select sku_id
            from channel_sku_mappings
            where tenant_id = $1
              and shop_id = $2
              and platform = $3
              and platform_sku_id = $4
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .bind(platform.to_string())
        .bind(platform_sku_id)
        .fetch_optional(self.pool)
        .await?;

        if sku_id.is_some() {
            return Ok(sku_id);
        }

        let Some(outer_sku_id) = outer_sku_id.filter(|v| !v.trim().is_empty()) else {
            return Ok(None);
        };

        let sku_id = sqlx::query_scalar::<_, uuid::Uuid>(
            r#"
            select sku_id
            from channel_sku_mappings
            where tenant_id = $1
              and shop_id = $2
              and platform = $3
              and platform_outer_sku_id = $4
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .bind(platform.to_string())
        .bind(outer_sku_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(sku_id)
    }
}
