use chrono::{DateTime, Utc};
use erp_adapter::ShopAuthContext;
use erp_types::{Platform, ShopId, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shop {
    pub shop_id: ShopId,
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub platform_shop_id: String,
    pub name: String,
    pub status: String,
    pub auth_meta_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ShopListFilters {
    pub platform: Option<Platform>,
    pub status: Option<String>,
    pub shop_ids: Option<Vec<ShopId>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ShopListResult {
    pub items: Vec<Shop>,
    pub total: i64,
}

pub struct ShopCreate {
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub platform_shop_id: String,
    pub name: String,
    pub status: Option<String>,
    pub credentials: Option<Value>,
    pub auth_meta_json: Option<Value>,
}

pub struct ShopUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
    pub credentials: Option<Value>,
    pub auth_meta_json: Option<Value>,
}

pub struct ShopsRepo<'a> {
    pool: &'a PgPool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShopSummary {
    pub shop_id: ShopId,
    pub tenant_id: TenantId,
    pub platform: Platform,
    pub shop_name: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl<'a> ShopsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_shops(
        &self,
        tenant_id: TenantId,
        filters: ShopListFilters,
    ) -> anyhow::Result<ShopListResult> {
        let ShopListFilters {
            platform,
            status,
            shop_ids,
            offset,
            limit,
        } = filters;
        if let Some(ids) = &shop_ids
            && ids.is_empty()
        {
            return Ok(ShopListResult {
                items: vec![],
                total: 0,
            });
        }
        let mut total_qb =
            QueryBuilder::<Postgres>::new("select count(*) from shops where tenant_id = ");
        total_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(platform) = platform {
            total_qb.push(" and platform = ");
            total_qb.push_bind(platform.to_string());
        }
        if let Some(status) = &status {
            total_qb.push(" and status = ");
            total_qb.push_bind(status);
        }
        if let Some(shop_ids) = &shop_ids {
            total_qb.push(" and shop_id = any(");
            total_qb.push_bind(
                shop_ids
                    .iter()
                    .map(|id| uuid::Uuid::from(*id))
                    .collect::<Vec<_>>(),
            );
            total_qb.push(")");
        }
        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select shop_id, platform, platform_shop_id, name, status, auth_meta_json, created_at, updated_at
            from shops
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(platform) = platform {
            qb.push(" and platform = ");
            qb.push_bind(platform.to_string());
        }
        if let Some(status) = status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(shop_ids) = shop_ids {
            qb.push(" and shop_id = any(");
            qb.push_bind(
                shop_ids
                    .iter()
                    .map(|id| uuid::Uuid::from(*id))
                    .collect::<Vec<_>>(),
            );
            qb.push(")");
        }

        qb.push(" order by updated_at desc, shop_id desc");
        qb.push(" offset ");
        qb.push_bind(offset);
        qb.push(" limit ");
        qb.push_bind(limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            shop_id: uuid::Uuid,
            platform: String,
            platform_shop_id: String,
            name: String,
            status: String,
            auth_meta_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| {
                let platform = Platform::from_str(&row.platform)
                    .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
                Ok(Shop {
                    shop_id: ShopId(row.shop_id),
                    tenant_id,
                    platform,
                    platform_shop_id: row.platform_shop_id,
                    name: row.name,
                    status: row.status,
                    auth_meta_json: row.auth_meta_json,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(ShopListResult { items, total })
    }

    pub async fn get_shop(
        &self,
        tenant_id: TenantId,
        shop_id: ShopId,
    ) -> anyhow::Result<Option<Shop>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            shop_id: uuid::Uuid,
            platform: String,
            platform_shop_id: String,
            name: String,
            status: String,
            auth_meta_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select shop_id, platform, platform_shop_id, name, status, auth_meta_json, created_at, updated_at
            from shops
            where tenant_id = $1 and shop_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        let platform = Platform::from_str(&row.platform)
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
        Ok(Some(Shop {
            shop_id: ShopId(row.shop_id),
            tenant_id,
            platform,
            platform_shop_id: row.platform_shop_id,
            name: row.name,
            status: row.status,
            auth_meta_json: row.auth_meta_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_shop(&self, input: ShopCreate) -> anyhow::Result<Shop> {
        let status = input.status.unwrap_or_else(|| "AUTHORIZED".to_string());
        let credentials = input.credentials.unwrap_or_else(|| serde_json::json!({}));
        let auth_meta_json = input
            .auth_meta_json
            .unwrap_or_else(|| serde_json::json!({}));

        #[derive(sqlx::FromRow)]
        struct Row {
            shop_id: uuid::Uuid,
            platform: String,
            platform_shop_id: String,
            name: String,
            status: String,
            auth_meta_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into shops (
              tenant_id, platform, platform_shop_id, name, shop_name, status, credentials, auth_meta_json
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            on conflict (tenant_id, platform, platform_shop_id)
            do update set name = excluded.name, shop_name = excluded.shop_name, updated_at = now()
            returning shop_id, platform, platform_shop_id, name, status, auth_meta_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.platform.to_string())
        .bind(&input.platform_shop_id)
        .bind(&input.name)
        .bind(&input.name)
        .bind(&status)
        .bind(credentials)
        .bind(auth_meta_json)
        .fetch_one(self.pool)
        .await?;

        let platform = Platform::from_str(&row.platform)
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
        Ok(Shop {
            shop_id: ShopId(row.shop_id),
            tenant_id: input.tenant_id,
            platform,
            platform_shop_id: row.platform_shop_id,
            name: row.name,
            status: row.status,
            auth_meta_json: row.auth_meta_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_shop(
        &self,
        tenant_id: TenantId,
        shop_id: ShopId,
        input: ShopUpdate,
    ) -> anyhow::Result<Option<Shop>> {
        let mut qb = QueryBuilder::<Postgres>::new("update shops set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name.clone());
            separated.push("shop_name = ");
            separated.push_bind(name);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(credentials) = input.credentials {
            separated.push("credentials = ");
            separated.push_bind(credentials);
        }
        if let Some(auth_meta_json) = input.auth_meta_json {
            separated.push("auth_meta_json = ");
            separated.push_bind(auth_meta_json);
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and shop_id = ");
        qb.push_bind(uuid::Uuid::from(shop_id));
        qb.push(
            " returning shop_id, platform, platform_shop_id, name, status, auth_meta_json, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            shop_id: uuid::Uuid,
            platform: String,
            platform_shop_id: String,
            name: String,
            status: String,
            auth_meta_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let platform = Platform::from_str(&row.platform)
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
        Ok(Some(Shop {
            shop_id: ShopId(row.shop_id),
            tenant_id,
            platform,
            platform_shop_id: row.platform_shop_id,
            name: row.name,
            status: row.status,
            auth_meta_json: row.auth_meta_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn get_shop_auth(&self, shop_id: ShopId) -> anyhow::Result<ShopAuthContext> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tenant_id: uuid::Uuid,
            platform: String,
            platform_shop_id: String,
            credentials: Value,
            auth_meta_json: Value,
            access_token: Option<String>,
            refresh_token: Option<String>,
            expires_at: Option<DateTime<Utc>>,
            scopes: Option<String>,
        }

        let r: Row = sqlx::query_as::<_, Row>(
            r#"
            select s.tenant_id, s.platform, s.platform_shop_id, s.credentials, s.auth_meta_json,
                   t.access_token, t.refresh_token, t.expires_at, t.scopes
            from shops s
            left join platform_tokens t
              on t.tenant_id = s.tenant_id and t.shop_id = s.shop_id
            where s.shop_id = $1 and s.status in ('AUTHORIZED', 'EXPIRED')
            "#,
        )
        .bind(uuid::Uuid::from(shop_id))
        .fetch_one(self.pool)
        .await?;

        let platform = Platform::from_str(&r.platform)
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;

        let mut credentials = r.credentials;
        let map = match credentials.as_object_mut() {
            Some(map) => map,
            None => {
                credentials = Value::Object(Map::new());
                credentials.as_object_mut().unwrap()
            }
        };
        if let Some(access_token) = r.access_token {
            map.insert("access_token".to_string(), Value::String(access_token));
        }
        if let Some(refresh_token) = r.refresh_token {
            map.insert("refresh_token".to_string(), Value::String(refresh_token));
        }
        if let Some(scopes) = r.scopes {
            map.insert("scopes".to_string(), Value::String(scopes));
        }
        if let Some(expires_at) = r.expires_at {
            map.insert(
                "expires_at".to_string(),
                Value::String(expires_at.to_rfc3339()),
            );
        }

        Ok(ShopAuthContext {
            tenant_id: TenantId(r.tenant_id),
            shop_id,
            platform,
            platform_shop_id: r.platform_shop_id,
            credentials,
            auth_meta: r.auth_meta_json,
        })
    }

    pub async fn get_default_carrier_code(
        &self,
        tenant_id: TenantId,
        shop_id: ShopId,
    ) -> anyhow::Result<Option<String>> {
        let carrier: Option<String> = sqlx::query_scalar(
            r#"
            select default_carrier_code
            from shops
            where tenant_id = $1 and shop_id = $2 and status in ('AUTHORIZED', 'EXPIRED')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(carrier.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }))
    }

    pub async fn list_shop_summaries(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Vec<ShopSummary>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            shop_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            platform: String,
            shop_name: String,
            status: String,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select shop_id, tenant_id, platform, shop_name, status, created_at
            from shops
            where tenant_id = $1
            order by created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let platform = Platform::from_str(&row.platform).ok()?;
                Some(ShopSummary {
                    shop_id: ShopId(row.shop_id),
                    tenant_id: TenantId(row.tenant_id),
                    platform,
                    shop_name: row.shop_name,
                    status: row.status,
                    created_at: row.created_at,
                })
            })
            .collect())
    }
}
