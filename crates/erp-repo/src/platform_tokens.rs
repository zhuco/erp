use chrono::{DateTime, Utc};
use erp_types::{Platform, ShopId, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformToken {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub token_status: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<String>,
    pub last_refreshed_at: Option<DateTime<Utc>>,
    pub refresh_fail_count: i32,
    pub last_error: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PlatformTokenUpsert {
    pub tenant_id: TenantId,
    pub shop_id: ShopId,
    pub token_status: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<String>,
    pub last_refreshed_at: Option<DateTime<Utc>>,
    pub refresh_fail_count: Option<i32>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenRefreshCandidate {
    pub token: PlatformToken,
    pub platform: Platform,
}

pub struct PlatformTokensRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PlatformTokensRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_token(
        &self,
        tenant_id: TenantId,
        shop_id: ShopId,
    ) -> anyhow::Result<Option<PlatformToken>> {
        let row: Option<TokenRow> = sqlx::query_as(
            r#"
            select tenant_id, shop_id, token_status, access_token, refresh_token, expires_at, scopes,
                   last_refreshed_at, refresh_fail_count, last_error, created_at, updated_at
            from platform_tokens
            where tenant_id = $1 and shop_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(TokenRow::into_token))
    }

    pub async fn list_tokens(
        &self,
        tenant_id: TenantId,
        shop_ids: &[ShopId],
    ) -> anyhow::Result<Vec<PlatformToken>> {
        if shop_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<uuid::Uuid> = shop_ids.iter().map(|id| uuid::Uuid::from(*id)).collect();
        let rows: Vec<TokenRow> = sqlx::query_as(
            r#"
            select tenant_id, shop_id, token_status, access_token, refresh_token, expires_at, scopes,
                   last_refreshed_at, refresh_fail_count, last_error, created_at, updated_at
            from platform_tokens
            where tenant_id = $1 and shop_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(ids)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(TokenRow::into_token).collect())
    }

    pub async fn upsert_token(&self, input: PlatformTokenUpsert) -> anyhow::Result<PlatformToken> {
        let last_error = input.last_error.unwrap_or_default();
        let now = Utc::now();
        let token_status = input
            .token_status
            .unwrap_or_else(|| derive_token_status(input.expires_at, now));
        let refresh_fail_count = input.refresh_fail_count.unwrap_or(0);
        let row: TokenRow = sqlx::query_as(
            r#"
            insert into platform_tokens (
              tenant_id, shop_id, token_status, access_token, refresh_token, expires_at,
              scopes, last_refreshed_at, refresh_fail_count, last_error
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            on conflict (tenant_id, shop_id)
            do update set
              token_status = excluded.token_status,
              access_token = excluded.access_token,
              refresh_token = excluded.refresh_token,
              expires_at = excluded.expires_at,
              scopes = excluded.scopes,
              last_refreshed_at = excluded.last_refreshed_at,
              refresh_fail_count = excluded.refresh_fail_count,
              last_error = excluded.last_error,
              updated_at = now()
            returning tenant_id, shop_id, token_status, access_token, refresh_token, expires_at, scopes,
                      last_refreshed_at, refresh_fail_count, last_error, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(uuid::Uuid::from(input.shop_id))
        .bind(token_status)
        .bind(input.access_token)
        .bind(input.refresh_token)
        .bind(input.expires_at)
        .bind(input.scopes)
        .bind(input.last_refreshed_at)
        .bind(refresh_fail_count)
        .bind(last_error)
        .fetch_one(self.pool)
        .await?;

        Ok(row.into_token())
    }

    pub async fn mark_refresh_failed(
        &self,
        tenant_id: TenantId,
        shop_id: ShopId,
        last_error: String,
    ) -> anyhow::Result<Option<PlatformToken>> {
        let row: Option<TokenRow> = sqlx::query_as(
            r#"
            update platform_tokens
            set token_status = 'EXPIRED',
                refresh_fail_count = refresh_fail_count + 1,
                last_error = $3,
                last_refreshed_at = now(),
                updated_at = now()
            where tenant_id = $1 and shop_id = $2
            returning tenant_id, shop_id, token_status, access_token, refresh_token, expires_at, scopes,
                      last_refreshed_at, refresh_fail_count, last_error, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(shop_id))
        .bind(last_error)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(TokenRow::into_token))
    }

    pub async fn list_expiring_tokens(
        &self,
        before: DateTime<Utc>,
        limit: i64,
    ) -> anyhow::Result<Vec<TokenRefreshCandidate>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tenant_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            token_status: String,
            access_token: Option<String>,
            refresh_token: Option<String>,
            expires_at: Option<DateTime<Utc>>,
            scopes: Option<String>,
            last_refreshed_at: Option<DateTime<Utc>>,
            refresh_fail_count: i32,
            last_error: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            platform: String,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select pt.tenant_id, pt.shop_id, pt.token_status, pt.access_token, pt.refresh_token,
                   pt.expires_at, pt.scopes, pt.last_refreshed_at, pt.refresh_fail_count,
                   pt.last_error, pt.created_at, pt.updated_at, s.platform
            from platform_tokens pt
            join shops s
              on s.tenant_id = pt.tenant_id and s.shop_id = pt.shop_id
            where s.status <> 'DISABLED'
              and pt.expires_at is not null
              and pt.expires_at <= $1
            order by pt.expires_at asc
            limit $2
            "#,
        )
        .bind(before)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let platform = Platform::from_str(&row.platform)
                .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
            let token = PlatformToken {
                tenant_id: TenantId(row.tenant_id),
                shop_id: ShopId(row.shop_id),
                token_status: row.token_status,
                access_token: row.access_token,
                refresh_token: row.refresh_token,
                expires_at: row.expires_at,
                scopes: row.scopes,
                last_refreshed_at: row.last_refreshed_at,
                refresh_fail_count: row.refresh_fail_count,
                last_error: row.last_error,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };
            out.push(TokenRefreshCandidate { token, platform });
        }

        Ok(out)
    }
}

#[derive(sqlx::FromRow)]
struct TokenRow {
    tenant_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    token_status: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    scopes: Option<String>,
    last_refreshed_at: Option<DateTime<Utc>>,
    refresh_fail_count: i32,
    last_error: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TokenRow {
    fn into_token(self) -> PlatformToken {
        PlatformToken {
            tenant_id: TenantId(self.tenant_id),
            shop_id: ShopId(self.shop_id),
            token_status: self.token_status,
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: self.expires_at,
            scopes: self.scopes,
            last_refreshed_at: self.last_refreshed_at,
            refresh_fail_count: self.refresh_fail_count,
            last_error: self.last_error,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn derive_token_status(expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> String {
    match expires_at {
        Some(ts) if ts > now => "ACTIVE".to_string(),
        _ => "EXPIRED".to_string(),
    }
}
