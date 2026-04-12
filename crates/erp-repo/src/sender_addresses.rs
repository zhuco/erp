use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderAddress {
    pub sender_address_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub district: String,
    pub address: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SenderAddressListFilters {
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct SenderAddressListResult {
    pub items: Vec<SenderAddress>,
    pub total: i64,
}

pub struct SenderAddressCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub phone: String,
    pub province: String,
    pub city: String,
    pub district: String,
    pub address: String,
    pub is_default: bool,
}

pub struct SenderAddressUpdate {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: Option<String>,
    pub is_default: Option<bool>,
}

pub struct SenderAddressesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SenderAddressesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_sender_addresses(
        &self,
        tenant_id: TenantId,
        filters: SenderAddressListFilters,
    ) -> anyhow::Result<SenderAddressListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from sender_addresses
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_sender_filters(&mut count_qb, keyword.as_deref());
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select sender_address_id, name, phone, province, city, district, address,
                   is_default, created_at, updated_at
            from sender_addresses
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_sender_filters(&mut qb, keyword.as_deref());
        qb.push(" order by is_default desc, updated_at desc, sender_address_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            sender_address_id: uuid::Uuid,
            name: String,
            phone: String,
            province: String,
            city: String,
            district: String,
            address: String,
            is_default: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| SenderAddress {
                sender_address_id: row.sender_address_id,
                tenant_id,
                name: row.name,
                phone: row.phone,
                province: row.province,
                city: row.city,
                district: row.district,
                address: row.address,
                is_default: row.is_default,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(SenderAddressListResult { items, total })
    }

    pub async fn create_sender_address(
        &self,
        input: SenderAddressCreate,
    ) -> anyhow::Result<SenderAddress> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        if input.is_default {
            self.clear_default(&mut tx, input.tenant_id).await?;
        }
        let sender = self.create_sender_with_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(sender)
    }

    pub async fn update_sender_address(
        &self,
        tenant_id: TenantId,
        sender_address_id: uuid::Uuid,
        input: SenderAddressUpdate,
    ) -> anyhow::Result<Option<SenderAddress>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        if input.is_default.unwrap_or(false) {
            self.clear_default(&mut tx, tenant_id).await?;
        }

        let mut qb = QueryBuilder::<Postgres>::new("update sender_addresses set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(phone) = input.phone {
            separated.push("phone = ");
            separated.push_bind(phone);
        }
        if let Some(province) = input.province {
            separated.push("province = ");
            separated.push_bind(province);
        }
        if let Some(city) = input.city {
            separated.push("city = ");
            separated.push_bind(city);
        }
        if let Some(district) = input.district {
            separated.push("district = ");
            separated.push_bind(district);
        }
        if let Some(address) = input.address {
            separated.push("address = ");
            separated.push_bind(address);
        }
        if let Some(is_default) = input.is_default {
            separated.push("is_default = ");
            separated.push_bind(is_default);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and sender_address_id = ");
        qb.push_bind(sender_address_id);
        qb.push(
            " returning sender_address_id, name, phone, province, city, district, address, \
             is_default, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            sender_address_id: uuid::Uuid,
            name: String,
            phone: String,
            province: String,
            city: String,
            district: String,
            address: String,
            is_default: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        Ok(row.map(|row| SenderAddress {
            sender_address_id: row.sender_address_id,
            tenant_id,
            name: row.name,
            phone: row.phone,
            province: row.province,
            city: row.city,
            district: row.district,
            address: row.address,
            is_default: row.is_default,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_sender_address(
        &self,
        tenant_id: TenantId,
        sender_address_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SenderAddress>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            sender_address_id: uuid::Uuid,
            name: String,
            phone: String,
            province: String,
            city: String,
            district: String,
            address: String,
            is_default: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from sender_addresses
            where tenant_id = $1 and sender_address_id = $2
            returning sender_address_id, name, phone, province, city, district, address,
                      is_default, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sender_address_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| SenderAddress {
            sender_address_id: row.sender_address_id,
            tenant_id,
            name: row.name,
            phone: row.phone,
            province: row.province,
            city: row.city,
            district: row.district,
            address: row.address,
            is_default: row.is_default,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn create_sender_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: SenderAddressCreate,
    ) -> anyhow::Result<SenderAddress> {
        #[derive(sqlx::FromRow)]
        struct Row {
            sender_address_id: uuid::Uuid,
            name: String,
            phone: String,
            province: String,
            city: String,
            district: String,
            address: String,
            is_default: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into sender_addresses(
                tenant_id, name, phone, province, city, district, address, is_default
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning sender_address_id, name, phone, province, city, district, address,
                      is_default, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.phone)
        .bind(&input.province)
        .bind(&input.city)
        .bind(&input.district)
        .bind(&input.address)
        .bind(input.is_default)
        .fetch_one(&mut **tx)
        .await?;

        Ok(SenderAddress {
            sender_address_id: row.sender_address_id,
            tenant_id: input.tenant_id,
            name: row.name,
            phone: row.phone,
            province: row.province,
            city: row.city,
            district: row.district,
            address: row.address,
            is_default: row.is_default,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    async fn clear_default(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update sender_addresses
            set is_default = false,
                updated_at = now()
            where tenant_id = $1 and is_default = true
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

fn apply_sender_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, keyword: Option<&'a str>) {
    if let Some(keyword) = keyword {
        qb.push(" and (name ilike ");
        qb.push_bind(keyword);
        qb.push(" or phone ilike ");
        qb.push_bind(keyword);
        qb.push(" or address ilike ");
        qb.push_bind(keyword);
        qb.push(")");
    }
}
