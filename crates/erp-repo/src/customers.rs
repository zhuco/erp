use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerCategory {
    pub category_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CustomerCategoryListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct CustomerCategoryListResult {
    pub items: Vec<CustomerCategory>,
    pub total: i64,
}

pub struct CustomerCategoryCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
}

pub struct CustomerCategoryUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberLevel {
    pub level_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub level_rank: i32,
    pub discount_rate: Option<f64>,
    pub rules: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MemberLevelListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct MemberLevelListResult {
    pub items: Vec<MemberLevel>,
    pub total: i64,
}

pub struct MemberLevelCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub level_rank: i32,
    pub discount_rate: Option<f64>,
    pub rules: Option<String>,
    pub status: String,
}

pub struct MemberLevelUpdate {
    pub name: Option<String>,
    pub level_rank: Option<i32>,
    pub discount_rate: Option<f64>,
    pub rules: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerTag {
    pub tag_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CustomerTagListFilters {
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct CustomerTagListResult {
    pub items: Vec<CustomerTag>,
    pub total: i64,
}

pub struct CustomerTagCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub color: Option<String>,
}

pub struct CustomerTagUpdate {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub customer_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub note: Option<String>,
    pub status: String,
    pub category_id: Option<uuid::Uuid>,
    pub category_name: Option<String>,
    pub member_level_id: Option<uuid::Uuid>,
    pub member_level_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CustomerDetail {
    pub customer: Customer,
    pub tags: Vec<CustomerTag>,
}

pub struct CustomerListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub category_id: Option<uuid::Uuid>,
    pub member_level_id: Option<uuid::Uuid>,
    pub tag_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

pub struct CustomerListResult {
    pub items: Vec<Customer>,
    pub total: i64,
}

pub struct CustomerCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub note: Option<String>,
    pub status: String,
    pub category_id: Option<uuid::Uuid>,
    pub member_level_id: Option<uuid::Uuid>,
}

pub struct CustomerUpdate {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub note: Option<String>,
    pub status: Option<String>,
    pub category_id: Option<uuid::Uuid>,
    pub member_level_id: Option<uuid::Uuid>,
}

pub struct CustomerCategoriesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> CustomerCategoriesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_categories(
        &self,
        tenant_id: TenantId,
        filters: CustomerCategoryListFilters,
    ) -> anyhow::Result<CustomerCategoryListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from customer_categories where tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        if let Some(keyword) = keyword.as_ref() {
            count_qb.push(" and name ilike ");
            count_qb.push_bind(keyword);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select category_id, name, description, status, created_at, updated_at
            from customer_categories
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(keyword) = keyword.as_ref() {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }
        qb.push(" order by updated_at desc, category_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            category_id: uuid::Uuid,
            name: String,
            description: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| CustomerCategory {
                category_id: row.category_id,
                tenant_id,
                name: row.name,
                description: row.description,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(CustomerCategoryListResult { items, total })
    }

    pub async fn create_category(
        &self,
        input: CustomerCategoryCreate,
    ) -> anyhow::Result<CustomerCategory> {
        #[derive(sqlx::FromRow)]
        struct Row {
            category_id: uuid::Uuid,
            name: String,
            description: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into customer_categories(tenant_id, name, description, status)
            values ($1, $2, $3, $4)
            on conflict (tenant_id, name)
            do update set
                description = excluded.description,
                status = excluded.status,
                updated_at = now()
            returning category_id, name, description, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(CustomerCategory {
            category_id: row.category_id,
            tenant_id: input.tenant_id,
            name: row.name,
            description: row.description,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_category(
        &self,
        tenant_id: TenantId,
        category_id: uuid::Uuid,
        input: CustomerCategoryUpdate,
    ) -> anyhow::Result<Option<CustomerCategory>> {
        let mut qb = QueryBuilder::<Postgres>::new("update customer_categories set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(description) = input.description {
            separated.push("description = ");
            separated.push_bind(description);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and category_id = ");
        qb.push_bind(category_id);
        qb.push(" returning category_id, name, description, status, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            category_id: uuid::Uuid,
            name: String,
            description: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(CustomerCategory {
            category_id: row.category_id,
            tenant_id,
            name: row.name,
            description: row.description,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_category(
        &self,
        tenant_id: TenantId,
        category_id: uuid::Uuid,
    ) -> anyhow::Result<Option<CustomerCategory>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            category_id: uuid::Uuid,
            name: String,
            description: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from customer_categories
            where tenant_id = $1 and category_id = $2
            returning category_id, name, description, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(category_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| CustomerCategory {
            category_id: row.category_id,
            tenant_id,
            name: row.name,
            description: row.description,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

pub struct MemberLevelsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MemberLevelsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_levels(
        &self,
        tenant_id: TenantId,
        filters: MemberLevelListFilters,
    ) -> anyhow::Result<MemberLevelListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb =
            QueryBuilder::<Postgres>::new("select count(*) from member_levels where tenant_id = ");
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        if let Some(keyword) = keyword.as_ref() {
            count_qb.push(" and name ilike ");
            count_qb.push_bind(keyword);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select level_id, name, level_rank, discount_rate, rules, status, created_at, updated_at
            from member_levels
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(keyword) = keyword.as_ref() {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }
        qb.push(" order by level_rank desc, updated_at desc, level_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            level_id: uuid::Uuid,
            name: String,
            level_rank: i32,
            discount_rate: Option<f64>,
            rules: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| MemberLevel {
                level_id: row.level_id,
                tenant_id,
                name: row.name,
                level_rank: row.level_rank,
                discount_rate: row.discount_rate,
                rules: row.rules,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(MemberLevelListResult { items, total })
    }

    pub async fn create_level(&self, input: MemberLevelCreate) -> anyhow::Result<MemberLevel> {
        #[derive(sqlx::FromRow)]
        struct Row {
            level_id: uuid::Uuid,
            name: String,
            level_rank: i32,
            discount_rate: Option<f64>,
            rules: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into member_levels(tenant_id, name, level_rank, discount_rate, rules, status)
            values ($1, $2, $3, $4, $5, $6)
            on conflict (tenant_id, name)
            do update set
                level_rank = excluded.level_rank,
                discount_rate = excluded.discount_rate,
                rules = excluded.rules,
                status = excluded.status,
                updated_at = now()
            returning level_id, name, level_rank, discount_rate, rules, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(input.level_rank)
        .bind(input.discount_rate)
        .bind(&input.rules)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(MemberLevel {
            level_id: row.level_id,
            tenant_id: input.tenant_id,
            name: row.name,
            level_rank: row.level_rank,
            discount_rate: row.discount_rate,
            rules: row.rules,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_level(
        &self,
        tenant_id: TenantId,
        level_id: uuid::Uuid,
        input: MemberLevelUpdate,
    ) -> anyhow::Result<Option<MemberLevel>> {
        let mut qb = QueryBuilder::<Postgres>::new("update member_levels set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(rank) = input.level_rank {
            separated.push("level_rank = ");
            separated.push_bind(rank);
        }
        if let Some(rate) = input.discount_rate {
            separated.push("discount_rate = ");
            separated.push_bind(rate);
        }
        if let Some(rules) = input.rules {
            separated.push("rules = ");
            separated.push_bind(rules);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and level_id = ");
        qb.push_bind(level_id);
        qb.push(" returning level_id, name, level_rank, discount_rate, rules, status, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            level_id: uuid::Uuid,
            name: String,
            level_rank: i32,
            discount_rate: Option<f64>,
            rules: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(MemberLevel {
            level_id: row.level_id,
            tenant_id,
            name: row.name,
            level_rank: row.level_rank,
            discount_rate: row.discount_rate,
            rules: row.rules,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_level(
        &self,
        tenant_id: TenantId,
        level_id: uuid::Uuid,
    ) -> anyhow::Result<Option<MemberLevel>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            level_id: uuid::Uuid,
            name: String,
            level_rank: i32,
            discount_rate: Option<f64>,
            rules: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from member_levels
            where tenant_id = $1 and level_id = $2
            returning level_id, name, level_rank, discount_rate, rules, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(level_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| MemberLevel {
            level_id: row.level_id,
            tenant_id,
            name: row.name,
            level_rank: row.level_rank,
            discount_rate: row.discount_rate,
            rules: row.rules,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

pub struct CustomerTagsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> CustomerTagsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_tags(
        &self,
        tenant_id: TenantId,
        filters: CustomerTagListFilters,
    ) -> anyhow::Result<CustomerTagListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb =
            QueryBuilder::<Postgres>::new("select count(*) from customer_tags where tenant_id = ");
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(keyword) = keyword.as_ref() {
            count_qb.push(" and name ilike ");
            count_qb.push_bind(keyword);
        }
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select tag_id, name, color, created_at, updated_at
            from customer_tags
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(keyword) = keyword.as_ref() {
            qb.push(" and name ilike ");
            qb.push_bind(keyword);
        }
        qb.push(" order by updated_at desc, tag_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| CustomerTag {
                tag_id: row.tag_id,
                tenant_id,
                name: row.name,
                color: row.color,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(CustomerTagListResult { items, total })
    }

    pub async fn list_tags_by_ids(
        &self,
        tenant_id: TenantId,
        tag_ids: Vec<uuid::Uuid>,
    ) -> anyhow::Result<Vec<CustomerTag>> {
        if tag_ids.is_empty() {
            return Ok(vec![]);
        }

        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select tag_id, name, color, created_at, updated_at
            from customer_tags
            where tenant_id = $1 and tag_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&tag_ids)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| CustomerTag {
                tag_id: row.tag_id,
                tenant_id,
                name: row.name,
                color: row.color,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn create_tag(&self, input: CustomerTagCreate) -> anyhow::Result<CustomerTag> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into customer_tags(tenant_id, name, color)
            values ($1, $2, $3)
            on conflict (tenant_id, name)
            do update set
                color = excluded.color,
                updated_at = now()
            returning tag_id, name, color, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.color)
        .fetch_one(self.pool)
        .await?;

        Ok(CustomerTag {
            tag_id: row.tag_id,
            tenant_id: input.tenant_id,
            name: row.name,
            color: row.color,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_tag(
        &self,
        tenant_id: TenantId,
        tag_id: uuid::Uuid,
        input: CustomerTagUpdate,
    ) -> anyhow::Result<Option<CustomerTag>> {
        let mut qb = QueryBuilder::<Postgres>::new("update customer_tags set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(color) = input.color {
            separated.push("color = ");
            separated.push_bind(color);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and tag_id = ");
        qb.push_bind(tag_id);
        qb.push(" returning tag_id, name, color, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(CustomerTag {
            tag_id: row.tag_id,
            tenant_id,
            name: row.name,
            color: row.color,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_tag(
        &self,
        tenant_id: TenantId,
        tag_id: uuid::Uuid,
    ) -> anyhow::Result<Option<CustomerTag>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from customer_tags
            where tenant_id = $1 and tag_id = $2
            returning tag_id, name, color, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(tag_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| CustomerTag {
            tag_id: row.tag_id,
            tenant_id,
            name: row.name,
            color: row.color,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

pub struct CustomersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> CustomersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_customers(
        &self,
        tenant_id: TenantId,
        filters: CustomerListFilters,
    ) -> anyhow::Result<CustomerListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb = QueryBuilder::<Postgres>::new("select count(*) from customers c");
        if filters.tag_id.is_some() {
            count_qb.push(" join customer_tag_links ctl on ctl.tenant_id = c.tenant_id and ctl.customer_id = c.customer_id");
        }
        count_qb.push(" where c.tenant_id = ");
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &filters.status {
            count_qb.push(" and c.status = ");
            count_qb.push_bind(status);
        }
        if let Some(category_id) = filters.category_id {
            count_qb.push(" and c.category_id = ");
            count_qb.push_bind(category_id);
        }
        if let Some(member_level_id) = filters.member_level_id {
            count_qb.push(" and c.member_level_id = ");
            count_qb.push_bind(member_level_id);
        }
        if let Some(tag_id) = filters.tag_id {
            count_qb.push(" and ctl.tag_id = ");
            count_qb.push_bind(tag_id);
        }
        if let Some(keyword) = keyword.as_ref() {
            count_qb.push(" and (c.name ilike ");
            count_qb.push_bind(keyword);
            count_qb.push(" or c.phone ilike ");
            count_qb.push_bind(keyword);
            count_qb.push(")");
        }

        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                c.customer_id,
                c.name,
                c.phone,
                c.address,
                c.note,
                c.status,
                c.category_id,
                cat.name as category_name,
                c.member_level_id,
                lvl.name as member_level_name,
                c.created_at,
                c.updated_at
            from customers c
            left join customer_categories cat
              on cat.tenant_id = c.tenant_id and cat.category_id = c.category_id
            left join member_levels lvl
              on lvl.tenant_id = c.tenant_id and lvl.level_id = c.member_level_id
            "#,
        );
        if filters.tag_id.is_some() {
            qb.push(" join customer_tag_links ctl on ctl.tenant_id = c.tenant_id and ctl.customer_id = c.customer_id");
        }
        qb.push(" where c.tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = filters.status {
            qb.push(" and c.status = ");
            qb.push_bind(status);
        }
        if let Some(category_id) = filters.category_id {
            qb.push(" and c.category_id = ");
            qb.push_bind(category_id);
        }
        if let Some(member_level_id) = filters.member_level_id {
            qb.push(" and c.member_level_id = ");
            qb.push_bind(member_level_id);
        }
        if let Some(tag_id) = filters.tag_id {
            qb.push(" and ctl.tag_id = ");
            qb.push_bind(tag_id);
        }
        if let Some(keyword) = keyword.as_ref() {
            qb.push(" and (c.name ilike ");
            qb.push_bind(keyword);
            qb.push(" or c.phone ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }
        qb.push(" order by c.updated_at desc, c.customer_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            customer_id: uuid::Uuid,
            name: String,
            phone: Option<String>,
            address: Option<String>,
            note: Option<String>,
            status: String,
            category_id: Option<uuid::Uuid>,
            category_name: Option<String>,
            member_level_id: Option<uuid::Uuid>,
            member_level_name: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| Customer {
                customer_id: row.customer_id,
                tenant_id,
                name: row.name,
                phone: row.phone,
                address: row.address,
                note: row.note,
                status: row.status,
                category_id: row.category_id,
                category_name: row.category_name,
                member_level_id: row.member_level_id,
                member_level_name: row.member_level_name,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(CustomerListResult { items, total })
    }

    pub async fn get_customer(
        &self,
        tenant_id: TenantId,
        customer_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Customer>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            customer_id: uuid::Uuid,
            name: String,
            phone: Option<String>,
            address: Option<String>,
            note: Option<String>,
            status: String,
            category_id: Option<uuid::Uuid>,
            category_name: Option<String>,
            member_level_id: Option<uuid::Uuid>,
            member_level_name: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                c.customer_id,
                c.name,
                c.phone,
                c.address,
                c.note,
                c.status,
                c.category_id,
                cat.name as category_name,
                c.member_level_id,
                lvl.name as member_level_name,
                c.created_at,
                c.updated_at
            from customers c
            left join customer_categories cat
              on cat.tenant_id = c.tenant_id and cat.category_id = c.category_id
            left join member_levels lvl
              on lvl.tenant_id = c.tenant_id and lvl.level_id = c.member_level_id
            where c.tenant_id = $1 and c.customer_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(customer_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Customer {
            customer_id: row.customer_id,
            tenant_id,
            name: row.name,
            phone: row.phone,
            address: row.address,
            note: row.note,
            status: row.status,
            category_id: row.category_id,
            category_name: row.category_name,
            member_level_id: row.member_level_id,
            member_level_name: row.member_level_name,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn get_customer_detail(
        &self,
        tenant_id: TenantId,
        customer_id: uuid::Uuid,
    ) -> anyhow::Result<Option<CustomerDetail>> {
        let Some(customer) = self.get_customer(tenant_id, customer_id).await? else {
            return Ok(None);
        };
        let tags = self.list_customer_tags(tenant_id, customer_id).await?;
        Ok(Some(CustomerDetail { customer, tags }))
    }

    pub async fn create_customer(&self, input: CustomerCreate) -> anyhow::Result<Customer> {
        #[derive(sqlx::FromRow)]
        struct Row {
            customer_id: uuid::Uuid,
            name: String,
            phone: Option<String>,
            address: Option<String>,
            note: Option<String>,
            status: String,
            category_id: Option<uuid::Uuid>,
            member_level_id: Option<uuid::Uuid>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into customers(
                tenant_id, name, phone, address, note, status, category_id, member_level_id
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            on conflict (tenant_id, phone)
            do update set
                name = excluded.name,
                address = excluded.address,
                note = excluded.note,
                status = excluded.status,
                category_id = excluded.category_id,
                member_level_id = excluded.member_level_id,
                updated_at = now()
            returning
                customer_id, name, phone, address, note, status,
                category_id, member_level_id, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.phone)
        .bind(&input.address)
        .bind(&input.note)
        .bind(&input.status)
        .bind(input.category_id)
        .bind(input.member_level_id)
        .fetch_one(self.pool)
        .await?;

        Ok(Customer {
            customer_id: row.customer_id,
            tenant_id: input.tenant_id,
            name: row.name,
            phone: row.phone,
            address: row.address,
            note: row.note,
            status: row.status,
            category_id: row.category_id,
            category_name: None,
            member_level_id: row.member_level_id,
            member_level_name: None,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_customer(
        &self,
        tenant_id: TenantId,
        customer_id: uuid::Uuid,
        input: CustomerUpdate,
    ) -> anyhow::Result<Option<Customer>> {
        let mut qb = QueryBuilder::<Postgres>::new("update customers set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(phone) = input.phone {
            separated.push("phone = ");
            separated.push_bind(phone);
        }
        if let Some(address) = input.address {
            separated.push("address = ");
            separated.push_bind(address);
        }
        if let Some(note) = input.note {
            separated.push("note = ");
            separated.push_bind(note);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(category_id) = input.category_id {
            separated.push("category_id = ");
            separated.push_bind(category_id);
        }
        if let Some(level_id) = input.member_level_id {
            separated.push("member_level_id = ");
            separated.push_bind(level_id);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and customer_id = ");
        qb.push_bind(customer_id);
        qb.push(" returning customer_id, name, phone, address, note, status, category_id, member_level_id, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            customer_id: uuid::Uuid,
            name: String,
            phone: Option<String>,
            address: Option<String>,
            note: Option<String>,
            status: String,
            category_id: Option<uuid::Uuid>,
            member_level_id: Option<uuid::Uuid>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(Customer {
            customer_id: row.customer_id,
            tenant_id,
            name: row.name,
            phone: row.phone,
            address: row.address,
            note: row.note,
            status: row.status,
            category_id: row.category_id,
            category_name: None,
            member_level_id: row.member_level_id,
            member_level_name: None,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_customer(
        &self,
        tenant_id: TenantId,
        customer_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Customer>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            customer_id: uuid::Uuid,
            name: String,
            phone: Option<String>,
            address: Option<String>,
            note: Option<String>,
            status: String,
            category_id: Option<uuid::Uuid>,
            member_level_id: Option<uuid::Uuid>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from customers
            where tenant_id = $1 and customer_id = $2
            returning customer_id, name, phone, address, note, status, category_id, member_level_id, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(customer_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Customer {
            customer_id: row.customer_id,
            tenant_id,
            name: row.name,
            phone: row.phone,
            address: row.address,
            note: row.note,
            status: row.status,
            category_id: row.category_id,
            category_name: None,
            member_level_id: row.member_level_id,
            member_level_name: None,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_customer_tags(
        &self,
        tenant_id: TenantId,
        customer_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<CustomerTag>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select t.tag_id, t.name, t.color, t.created_at, t.updated_at
            from customer_tag_links l
            join customer_tags t
              on t.tenant_id = l.tenant_id and t.tag_id = l.tag_id
            where l.tenant_id = $1 and l.customer_id = $2
            order by t.name asc, t.tag_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(customer_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| CustomerTag {
                tag_id: row.tag_id,
                tenant_id,
                name: row.name,
                color: row.color,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn replace_customer_tags(
        &self,
        tenant_id: TenantId,
        customer_id: uuid::Uuid,
        tag_ids: Vec<uuid::Uuid>,
    ) -> anyhow::Result<Vec<CustomerTag>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        sqlx::query(
            r#"
            delete from customer_tag_links
            where tenant_id = $1 and customer_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(customer_id)
        .execute(&mut *tx)
        .await?;

        if !tag_ids.is_empty() {
            let mut qb = QueryBuilder::<Postgres>::new(
                "insert into customer_tag_links(tenant_id, customer_id, tag_id) ",
            );
            qb.push_values(tag_ids.iter(), |mut row, tag_id| {
                row.push_bind(uuid::Uuid::from(tenant_id));
                row.push_bind(customer_id);
                row.push_bind(tag_id);
            });
            qb.push(" on conflict do nothing");
            qb.build().execute(&mut *tx).await?;
        }

        #[derive(sqlx::FromRow)]
        struct Row {
            tag_id: uuid::Uuid,
            name: String,
            color: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select t.tag_id, t.name, t.color, t.created_at, t.updated_at
            from customer_tag_links l
            join customer_tags t
              on t.tenant_id = l.tenant_id and t.tag_id = l.tag_id
            where l.tenant_id = $1 and l.customer_id = $2
            order by t.name asc, t.tag_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(customer_id)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(rows
            .into_iter()
            .map(|row| CustomerTag {
                tag_id: row.tag_id,
                tenant_id,
                name: row.name,
                color: row.color,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }
}
