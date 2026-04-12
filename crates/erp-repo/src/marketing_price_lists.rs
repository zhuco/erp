use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingPriceList {
    pub price_list_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub channel_id: Option<uuid::Uuid>,
    pub currency: String,
    pub includes_tax: bool,
    pub status: String,
    pub version: i32,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketingPriceItem {
    pub item_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub price_list_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub base_price_cent: Option<i64>,
    pub channel_price_cent: Option<i64>,
    pub member_price_cent: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MarketingPriceListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub channel_id: Option<uuid::Uuid>,
    pub currency: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct MarketingPriceListResult {
    pub items: Vec<MarketingPriceList>,
    pub total: i64,
}

pub struct MarketingPriceListCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub channel_id: Option<uuid::Uuid>,
    pub currency: String,
    pub includes_tax: bool,
    pub status: String,
    pub version: i32,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
}

pub struct MarketingPriceListUpdate {
    pub name: Option<String>,
    pub channel_id: Option<uuid::Uuid>,
    pub currency: Option<String>,
    pub includes_tax: Option<bool>,
    pub status: Option<String>,
    pub version: Option<i32>,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_to: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

pub struct MarketingPriceItemCreate {
    pub tenant_id: TenantId,
    pub price_list_id: uuid::Uuid,
    pub sku_id: uuid::Uuid,
    pub base_price_cent: Option<i64>,
    pub channel_price_cent: Option<i64>,
    pub member_price_cent: Option<i64>,
}

pub struct MarketingPriceItemUpdate {
    pub base_price_cent: Option<i64>,
    pub channel_price_cent: Option<i64>,
    pub member_price_cent: Option<i64>,
}

pub struct MarketingPriceListsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MarketingPriceListsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_price_lists(
        &self,
        tenant_id: TenantId,
        filters: MarketingPriceListFilters,
    ) -> anyhow::Result<MarketingPriceListResult> {
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
                    from marketing_price_lists
                    where tenant_id = $1
                      and name ilike $2
                      and ($3::text is null or status = $3)
                      and ($4::uuid is null or channel_id = $4)
                      and ($5::text is null or currency = $5)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .bind(filters.channel_id)
                .bind(filters.currency.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from marketing_price_lists
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                      and ($3::uuid is null or channel_id = $3)
                      and ($4::text is null or currency = $4)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .bind(filters.channel_id)
                .bind(filters.currency.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select price_list_id, name, channel_id, currency, includes_tax, status, version,
                   effective_from, effective_to, notes, created_by, created_at, updated_at
            from marketing_price_lists
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(channel_id) = filters.channel_id {
            qb.push(" and channel_id = ");
            qb.push_bind(channel_id);
        }
        if let Some(currency) = &filters.currency {
            qb.push(" and currency = ");
            qb.push_bind(currency);
        }

        qb.push(" order by updated_at desc, price_list_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            price_list_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            currency: String,
            includes_tax: bool,
            status: String,
            version: i32,
            effective_from: Option<DateTime<Utc>>,
            effective_to: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| MarketingPriceList {
                price_list_id: row.price_list_id,
                tenant_id,
                name: row.name,
                channel_id: row.channel_id,
                currency: row.currency,
                includes_tax: row.includes_tax,
                status: row.status,
                version: row.version,
                effective_from: row.effective_from,
                effective_to: row.effective_to,
                notes: row.notes,
                created_by: row.created_by,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(MarketingPriceListResult { items, total })
    }

    pub async fn get_price_list(
        &self,
        tenant_id: TenantId,
        price_list_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MarketingPriceList>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            price_list_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            currency: String,
            includes_tax: bool,
            status: String,
            version: i32,
            effective_from: Option<DateTime<Utc>>,
            effective_to: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select price_list_id, name, channel_id, currency, includes_tax, status, version,
                   effective_from, effective_to, notes, created_by, created_at, updated_at
            from marketing_price_lists
            where tenant_id = $1 and price_list_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(price_list_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MarketingPriceList {
            price_list_id: row.price_list_id,
            tenant_id,
            name: row.name,
            channel_id: row.channel_id,
            currency: row.currency,
            includes_tax: row.includes_tax,
            status: row.status,
            version: row.version,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_price_items(
        &self,
        tenant_id: TenantId,
        price_list_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<MarketingPriceItem>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: uuid::Uuid,
            price_list_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            base_price_cent: Option<i64>,
            channel_price_cent: Option<i64>,
            member_price_cent: Option<i64>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select item_id, price_list_id, sku_id, base_price_cent, channel_price_cent,
                   member_price_cent, created_at, updated_at
            from marketing_price_items
            where tenant_id = $1 and price_list_id = $2
            order by updated_at desc, item_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(price_list_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| MarketingPriceItem {
                item_id: row.item_id,
                tenant_id,
                price_list_id: row.price_list_id,
                sku_id: row.sku_id,
                base_price_cent: row.base_price_cent,
                channel_price_cent: row.channel_price_cent,
                member_price_cent: row.member_price_cent,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn create_price_list(
        &self,
        input: MarketingPriceListCreate,
    ) -> anyhow::Result<MarketingPriceList> {
        #[derive(sqlx::FromRow)]
        struct Row {
            price_list_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            currency: String,
            includes_tax: bool,
            status: String,
            version: i32,
            effective_from: Option<DateTime<Utc>>,
            effective_to: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into marketing_price_lists(
                tenant_id, name, channel_id, currency, includes_tax, status, version,
                effective_from, effective_to, notes, created_by
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            returning price_list_id, name, channel_id, currency, includes_tax, status, version,
                      effective_from, effective_to, notes, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.channel_id)
        .bind(&input.currency)
        .bind(input.includes_tax)
        .bind(&input.status)
        .bind(input.version)
        .bind(input.effective_from)
        .bind(input.effective_to)
        .bind(&input.notes)
        .bind(&input.created_by)
        .fetch_one(self.pool)
        .await?;

        Ok(MarketingPriceList {
            price_list_id: row.price_list_id,
            tenant_id: input.tenant_id,
            name: row.name,
            channel_id: row.channel_id,
            currency: row.currency,
            includes_tax: row.includes_tax,
            status: row.status,
            version: row.version,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_price_list(
        &self,
        tenant_id: TenantId,
        price_list_id: uuid::Uuid,
        input: MarketingPriceListUpdate,
    ) -> anyhow::Result<Option<MarketingPriceList>> {
        let mut qb = QueryBuilder::<Postgres>::new("update marketing_price_lists set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(channel_id) = input.channel_id {
            separated.push("channel_id = ");
            separated.push_bind(channel_id);
        }
        if let Some(currency) = input.currency {
            separated.push("currency = ");
            separated.push_bind(currency);
        }
        if let Some(includes_tax) = input.includes_tax {
            separated.push("includes_tax = ");
            separated.push_bind(includes_tax);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(version) = input.version {
            separated.push("version = ");
            separated.push_bind(version);
        }
        if let Some(effective_from) = input.effective_from {
            separated.push("effective_from = ");
            separated.push_bind(effective_from);
        }
        if let Some(effective_to) = input.effective_to {
            separated.push("effective_to = ");
            separated.push_bind(effective_to);
        }
        if let Some(notes) = input.notes {
            separated.push("notes = ");
            separated.push_bind(notes);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and price_list_id = ");
        qb.push_bind(price_list_id);
        qb.push(
            " returning price_list_id, name, channel_id, currency, includes_tax, status, version,
              effective_from, effective_to, notes, created_by, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            price_list_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            currency: String,
            includes_tax: bool,
            status: String,
            version: i32,
            effective_from: Option<DateTime<Utc>>,
            effective_to: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(MarketingPriceList {
            price_list_id: row.price_list_id,
            tenant_id,
            name: row.name,
            channel_id: row.channel_id,
            currency: row.currency,
            includes_tax: row.includes_tax,
            status: row.status,
            version: row.version,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_price_list(
        &self,
        tenant_id: TenantId,
        price_list_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MarketingPriceList>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            price_list_id: uuid::Uuid,
            name: String,
            channel_id: Option<uuid::Uuid>,
            currency: String,
            includes_tax: bool,
            status: String,
            version: i32,
            effective_from: Option<DateTime<Utc>>,
            effective_to: Option<DateTime<Utc>>,
            notes: Option<String>,
            created_by: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from marketing_price_lists
            where tenant_id = $1 and price_list_id = $2
            returning price_list_id, name, channel_id, currency, includes_tax, status, version,
                      effective_from, effective_to, notes, created_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(price_list_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MarketingPriceList {
            price_list_id: row.price_list_id,
            tenant_id,
            name: row.name,
            channel_id: row.channel_id,
            currency: row.currency,
            includes_tax: row.includes_tax,
            status: row.status,
            version: row.version,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            notes: row.notes,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_price_item(
        &self,
        input: MarketingPriceItemCreate,
    ) -> anyhow::Result<MarketingPriceItem> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: uuid::Uuid,
            price_list_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            base_price_cent: Option<i64>,
            channel_price_cent: Option<i64>,
            member_price_cent: Option<i64>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into marketing_price_items(
                tenant_id, price_list_id, sku_id, base_price_cent, channel_price_cent,
                member_price_cent
            ) values ($1, $2, $3, $4, $5, $6)
            on conflict (price_list_id, sku_id)
            do update set
                base_price_cent = excluded.base_price_cent,
                channel_price_cent = excluded.channel_price_cent,
                member_price_cent = excluded.member_price_cent,
                updated_at = now()
            returning item_id, price_list_id, sku_id, base_price_cent, channel_price_cent,
                      member_price_cent, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.price_list_id)
        .bind(input.sku_id)
        .bind(input.base_price_cent)
        .bind(input.channel_price_cent)
        .bind(input.member_price_cent)
        .fetch_one(self.pool)
        .await?;

        Ok(MarketingPriceItem {
            item_id: row.item_id,
            tenant_id: input.tenant_id,
            price_list_id: row.price_list_id,
            sku_id: row.sku_id,
            base_price_cent: row.base_price_cent,
            channel_price_cent: row.channel_price_cent,
            member_price_cent: row.member_price_cent,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_price_item(
        &self,
        tenant_id: TenantId,
        item_id: uuid::Uuid,
        input: MarketingPriceItemUpdate,
    ) -> anyhow::Result<Option<MarketingPriceItem>> {
        let mut qb = QueryBuilder::<Postgres>::new("update marketing_price_items set ");
        let mut separated = qb.separated(", ");

        if let Some(base_price_cent) = input.base_price_cent {
            separated.push("base_price_cent = ");
            separated.push_bind(base_price_cent);
        }
        if let Some(channel_price_cent) = input.channel_price_cent {
            separated.push("channel_price_cent = ");
            separated.push_bind(channel_price_cent);
        }
        if let Some(member_price_cent) = input.member_price_cent {
            separated.push("member_price_cent = ");
            separated.push_bind(member_price_cent);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and item_id = ");
        qb.push_bind(item_id);
        qb.push(
            " returning item_id, price_list_id, sku_id, base_price_cent, channel_price_cent,
              member_price_cent, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: uuid::Uuid,
            price_list_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            base_price_cent: Option<i64>,
            channel_price_cent: Option<i64>,
            member_price_cent: Option<i64>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(MarketingPriceItem {
            item_id: row.item_id,
            tenant_id,
            price_list_id: row.price_list_id,
            sku_id: row.sku_id,
            base_price_cent: row.base_price_cent,
            channel_price_cent: row.channel_price_cent,
            member_price_cent: row.member_price_cent,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_price_item(
        &self,
        tenant_id: TenantId,
        item_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MarketingPriceItem>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            item_id: uuid::Uuid,
            price_list_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            base_price_cent: Option<i64>,
            channel_price_cent: Option<i64>,
            member_price_cent: Option<i64>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from marketing_price_items
            where tenant_id = $1 and item_id = $2
            returning item_id, price_list_id, sku_id, base_price_cent, channel_price_cent,
                      member_price_cent, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(item_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MarketingPriceItem {
            item_id: row.item_id,
            tenant_id,
            price_list_id: row.price_list_id,
            sku_id: row.sku_id,
            base_price_cent: row.base_price_cent,
            channel_price_cent: row.channel_price_cent,
            member_price_cent: row.member_price_cent,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
