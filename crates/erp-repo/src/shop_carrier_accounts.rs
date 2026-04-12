use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopCarrierAccount {
    pub account_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: uuid::Uuid,
    pub carrier_code: String,
    pub account_meta_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ShopCarrierAccountListFilters {
    pub shop_id: Option<uuid::Uuid>,
    pub carrier_code: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ShopCarrierAccountListResult {
    pub items: Vec<ShopCarrierAccount>,
    pub total: i64,
}

pub struct ShopCarrierAccountCreate {
    pub tenant_id: TenantId,
    pub shop_id: uuid::Uuid,
    pub carrier_code: String,
    pub account_meta_json: Value,
}

pub struct ShopCarrierAccountUpdate {
    pub account_meta_json: Option<Value>,
}

pub struct ShopCarrierAccountsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ShopCarrierAccountsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_accounts(
        &self,
        tenant_id: TenantId,
        filters: ShopCarrierAccountListFilters,
    ) -> anyhow::Result<ShopCarrierAccountListResult> {
        let total: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from shop_carrier_accounts
            where tenant_id = $1
              and ($2::uuid is null or shop_id = $2)
              and ($3::text is null or carrier_code = $3)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(filters.shop_id)
        .bind(&filters.carrier_code)
        .fetch_one(self.pool)
        .await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select account_id, shop_id, carrier_code, account_meta_json, created_at, updated_at
            from shop_carrier_accounts
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = filters.shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(carrier_code) = filters.carrier_code {
            qb.push(" and carrier_code = ");
            qb.push_bind(carrier_code);
        }

        qb.push(" order by updated_at desc, account_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            account_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            carrier_code: String,
            account_meta_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ShopCarrierAccount {
                account_id: row.account_id,
                tenant_id,
                shop_id: row.shop_id,
                carrier_code: row.carrier_code,
                account_meta_json: row.account_meta_json,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ShopCarrierAccountListResult { items, total })
    }

    pub async fn create_account(
        &self,
        input: ShopCarrierAccountCreate,
    ) -> anyhow::Result<ShopCarrierAccount> {
        #[derive(sqlx::FromRow)]
        struct Row {
            account_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            carrier_code: String,
            account_meta_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into shop_carrier_accounts(
                tenant_id, shop_id, carrier_code, account_meta_json
            ) values ($1, $2, $3, $4)
            on conflict (tenant_id, shop_id, carrier_code)
            do update set
                account_meta_json = excluded.account_meta_json,
                updated_at = now()
            returning account_id, shop_id, carrier_code, account_meta_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.shop_id)
        .bind(&input.carrier_code)
        .bind(&input.account_meta_json)
        .fetch_one(self.pool)
        .await?;

        Ok(ShopCarrierAccount {
            account_id: row.account_id,
            tenant_id: input.tenant_id,
            shop_id: row.shop_id,
            carrier_code: row.carrier_code,
            account_meta_json: row.account_meta_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_account(
        &self,
        tenant_id: TenantId,
        account_id: uuid::Uuid,
        input: ShopCarrierAccountUpdate,
    ) -> anyhow::Result<Option<ShopCarrierAccount>> {
        let mut qb = QueryBuilder::<Postgres>::new("update shop_carrier_accounts set ");
        let mut separated = qb.separated(", ");

        if let Some(account_meta_json) = input.account_meta_json {
            separated.push("account_meta_json = ");
            separated.push_bind(account_meta_json);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and account_id = ");
        qb.push_bind(account_id);
        qb.push(
            " returning account_id, shop_id, carrier_code, account_meta_json, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            account_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            carrier_code: String,
            account_meta_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(ShopCarrierAccount {
            account_id: row.account_id,
            tenant_id,
            shop_id: row.shop_id,
            carrier_code: row.carrier_code,
            account_meta_json: row.account_meta_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_account(
        &self,
        tenant_id: TenantId,
        account_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ShopCarrierAccount>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            account_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            carrier_code: String,
            account_meta_json: Value,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from shop_carrier_accounts
            where tenant_id = $1 and account_id = $2
            returning account_id, shop_id, carrier_code, account_meta_json, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(account_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ShopCarrierAccount {
            account_id: row.account_id,
            tenant_id,
            shop_id: row.shop_id,
            carrier_code: row.carrier_code,
            account_meta_json: row.account_meta_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
