use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub channel_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub platform: String,
    pub name: String,
    pub department: Option<String>,
    pub owner: Option<String>,
    pub allow_manual_order: bool,
    pub allow_manual_after_sales: bool,
    pub default_warehouse_id: Option<uuid::Uuid>,
    pub commission_formula: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ChannelListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ChannelListResult {
    pub items: Vec<Channel>,
    pub total: i64,
}

pub struct ChannelCreate {
    pub tenant_id: TenantId,
    pub platform: String,
    pub name: String,
    pub department: Option<String>,
    pub owner: Option<String>,
    pub allow_manual_order: bool,
    pub allow_manual_after_sales: bool,
    pub default_warehouse_id: Option<uuid::Uuid>,
    pub commission_formula: Option<String>,
    pub status: String,
}

pub struct ChannelUpdate {
    pub platform: Option<String>,
    pub name: Option<String>,
    pub department: Option<String>,
    pub owner: Option<String>,
    pub allow_manual_order: Option<bool>,
    pub allow_manual_after_sales: Option<bool>,
    pub default_warehouse_id: Option<Option<uuid::Uuid>>,
    pub commission_formula: Option<Option<String>>,
    pub status: Option<String>,
}

pub struct ChannelsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ChannelsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_channels(
        &self,
        tenant_id: TenantId,
        filters: ChannelListFilters,
    ) -> anyhow::Result<ChannelListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));
        let status = filters
            .status
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        let mut total_qb =
            QueryBuilder::<Postgres>::new("select count(*) from channels where tenant_id = ");
        total_qb.push_bind(uuid::Uuid::from(tenant_id));
        if let Some(status) = &status {
            total_qb.push(" and status = ");
            total_qb.push_bind(status);
        }
        if let Some(keyword) = &keyword {
            total_qb.push(" and (platform ilike ");
            total_qb.push_bind(keyword);
            total_qb.push(" or name ilike ");
            total_qb.push_bind(keyword);
            total_qb.push(" or department ilike ");
            total_qb.push_bind(keyword);
            total_qb.push(" or owner ilike ");
            total_qb.push_bind(keyword);
            total_qb.push(")");
        }

        let total: i64 = total_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select channel_id, platform, name, department, owner,
                   allow_manual_order, allow_manual_after_sales,
                   default_warehouse_id, commission_formula, status,
                   created_at, updated_at
            from channels
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(status) = &status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        if let Some(keyword) = &keyword {
            qb.push(" and (platform ilike ");
            qb.push_bind(keyword);
            qb.push(" or name ilike ");
            qb.push_bind(keyword);
            qb.push(" or department ilike ");
            qb.push_bind(keyword);
            qb.push(" or owner ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }

        qb.push(" order by updated_at desc, channel_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            channel_id: uuid::Uuid,
            platform: String,
            name: String,
            department: Option<String>,
            owner: Option<String>,
            allow_manual_order: bool,
            allow_manual_after_sales: bool,
            default_warehouse_id: Option<uuid::Uuid>,
            commission_formula: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| Channel {
                channel_id: row.channel_id,
                tenant_id,
                platform: row.platform,
                name: row.name,
                department: row.department,
                owner: row.owner,
                allow_manual_order: row.allow_manual_order,
                allow_manual_after_sales: row.allow_manual_after_sales,
                default_warehouse_id: row.default_warehouse_id,
                commission_formula: row.commission_formula,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ChannelListResult { items, total })
    }

    pub async fn create_channel(&self, input: ChannelCreate) -> anyhow::Result<Channel> {
        #[derive(sqlx::FromRow)]
        struct Row {
            channel_id: uuid::Uuid,
            platform: String,
            name: String,
            department: Option<String>,
            owner: Option<String>,
            allow_manual_order: bool,
            allow_manual_after_sales: bool,
            default_warehouse_id: Option<uuid::Uuid>,
            commission_formula: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into channels (
                tenant_id,
                platform,
                name,
                department,
                owner,
                allow_manual_order,
                allow_manual_after_sales,
                default_warehouse_id,
                commission_formula,
                status
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            returning channel_id, platform, name, department, owner,
                      allow_manual_order, allow_manual_after_sales,
                      default_warehouse_id, commission_formula, status,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.platform)
        .bind(&input.name)
        .bind(&input.department)
        .bind(&input.owner)
        .bind(input.allow_manual_order)
        .bind(input.allow_manual_after_sales)
        .bind(input.default_warehouse_id)
        .bind(&input.commission_formula)
        .bind(&input.status)
        .fetch_one(self.pool)
        .await?;

        Ok(Channel {
            channel_id: row.channel_id,
            tenant_id: input.tenant_id,
            platform: row.platform,
            name: row.name,
            department: row.department,
            owner: row.owner,
            allow_manual_order: row.allow_manual_order,
            allow_manual_after_sales: row.allow_manual_after_sales,
            default_warehouse_id: row.default_warehouse_id,
            commission_formula: row.commission_formula,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_channel(
        &self,
        tenant_id: TenantId,
        channel_id: uuid::Uuid,
        input: ChannelUpdate,
    ) -> anyhow::Result<Option<Channel>> {
        let mut qb = QueryBuilder::<Postgres>::new("update channels set ");
        let mut separated = qb.separated(", ");

        if let Some(platform) = input.platform {
            separated.push("platform = ");
            separated.push_bind(platform);
        }
        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(department) = input.department {
            separated.push("department = ");
            separated.push_bind(department);
        }
        if let Some(owner) = input.owner {
            separated.push("owner = ");
            separated.push_bind(owner);
        }
        if let Some(allow_manual_order) = input.allow_manual_order {
            separated.push("allow_manual_order = ");
            separated.push_bind(allow_manual_order);
        }
        if let Some(allow_manual_after_sales) = input.allow_manual_after_sales {
            separated.push("allow_manual_after_sales = ");
            separated.push_bind(allow_manual_after_sales);
        }
        if let Some(default_warehouse_id) = input.default_warehouse_id {
            separated.push("default_warehouse_id = ");
            separated.push_bind(default_warehouse_id);
        }
        if let Some(commission_formula) = input.commission_formula {
            separated.push("commission_formula = ");
            separated.push_bind(commission_formula);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and channel_id = ");
        qb.push_bind(channel_id);
        qb.push(
            " returning channel_id, platform, name, department, owner, allow_manual_order,
              allow_manual_after_sales, default_warehouse_id, commission_formula, status,
              created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            channel_id: uuid::Uuid,
            platform: String,
            name: String,
            department: Option<String>,
            owner: Option<String>,
            allow_manual_order: bool,
            allow_manual_after_sales: bool,
            default_warehouse_id: Option<uuid::Uuid>,
            commission_formula: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(Channel {
            channel_id: row.channel_id,
            tenant_id,
            platform: row.platform,
            name: row.name,
            department: row.department,
            owner: row.owner,
            allow_manual_order: row.allow_manual_order,
            allow_manual_after_sales: row.allow_manual_after_sales,
            default_warehouse_id: row.default_warehouse_id,
            commission_formula: row.commission_formula,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_channel(
        &self,
        tenant_id: TenantId,
        channel_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Channel>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            channel_id: uuid::Uuid,
            platform: String,
            name: String,
            department: Option<String>,
            owner: Option<String>,
            allow_manual_order: bool,
            allow_manual_after_sales: bool,
            default_warehouse_id: Option<uuid::Uuid>,
            commission_formula: Option<String>,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from channels
            where tenant_id = $1 and channel_id = $2
            returning channel_id, platform, name, department, owner,
                      allow_manual_order, allow_manual_after_sales,
                      default_warehouse_id, commission_formula, status,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(channel_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Channel {
            channel_id: row.channel_id,
            tenant_id,
            platform: row.platform,
            name: row.name,
            department: row.department,
            owner: row.owner,
            allow_manual_order: row.allow_manual_order,
            allow_manual_after_sales: row.allow_manual_after_sales,
            default_warehouse_id: row.default_warehouse_id,
            commission_formula: row.commission_formula,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
