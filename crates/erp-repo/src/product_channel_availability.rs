use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductChannelAvailability {
    pub availability_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub sku_name: Option<String>,
    pub channel_id: Option<uuid::Uuid>,
    pub channel_platform: Option<String>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub warehouse_name: Option<String>,
    pub status: String,
    pub available_qty: i32,
    pub rule: Option<String>,
    pub on_shelf_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProductChannelAvailabilityListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub channel_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ProductChannelAvailabilityListResult {
    pub items: Vec<ProductChannelAvailability>,
    pub total: i64,
}

pub struct ProductChannelAvailabilityCreate {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub channel_id: Option<uuid::Uuid>,
    pub warehouse_id: Option<uuid::Uuid>,
    pub status: String,
    pub available_qty: i32,
    pub rule: Option<String>,
    pub on_shelf_at: Option<DateTime<Utc>>,
}

pub struct ProductChannelAvailabilityUpdate {
    pub channel_id: Option<Option<uuid::Uuid>>,
    pub warehouse_id: Option<Option<uuid::Uuid>>,
    pub status: Option<String>,
    pub available_qty: Option<i32>,
    pub rule: Option<Option<String>>,
    pub on_shelf_at: Option<Option<DateTime<Utc>>>,
}

pub struct ProductChannelAvailabilityRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductChannelAvailabilityRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_availability(
        &self,
        tenant_id: TenantId,
        filters: ProductChannelAvailabilityListFilters,
    ) -> anyhow::Result<ProductChannelAvailabilityListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from product_channel_availability pca where pca.tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pca.availability_id,
                pca.sku_id,
                s.sku_code,
                s.name as sku_name,
                pca.channel_id,
                c.platform as channel_platform,
                pca.warehouse_id,
                w.name as warehouse_name,
                pca.status,
                pca.available_qty,
                pca.rule,
                pca.on_shelf_at,
                pca.created_at,
                pca.updated_at
            from product_channel_availability pca
            join skus s
              on s.sku_id = pca.sku_id
             and s.tenant_id = pca.tenant_id
            left join channels c
              on c.channel_id = pca.channel_id
             and c.tenant_id = pca.tenant_id
            left join warehouses w
              on w.warehouse_id = pca.warehouse_id
             and w.tenant_id = pca.tenant_id
            where pca.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by pca.updated_at desc, pca.availability_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            availability_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            channel_id: Option<uuid::Uuid>,
            channel_platform: Option<String>,
            warehouse_id: Option<uuid::Uuid>,
            warehouse_name: Option<String>,
            status: String,
            available_qty: i32,
            rule: Option<String>,
            on_shelf_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProductChannelAvailability {
                availability_id: row.availability_id,
                tenant_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                sku_name: row.sku_name,
                channel_id: row.channel_id,
                channel_platform: row.channel_platform,
                warehouse_id: row.warehouse_id,
                warehouse_name: row.warehouse_name,
                status: row.status,
                available_qty: row.available_qty,
                rule: row.rule,
                on_shelf_at: row.on_shelf_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProductChannelAvailabilityListResult { items, total })
    }

    pub async fn get_availability(
        &self,
        tenant_id: TenantId,
        availability_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductChannelAvailability>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            availability_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            channel_id: Option<uuid::Uuid>,
            channel_platform: Option<String>,
            warehouse_id: Option<uuid::Uuid>,
            warehouse_name: Option<String>,
            status: String,
            available_qty: i32,
            rule: Option<String>,
            on_shelf_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                pca.availability_id,
                pca.sku_id,
                s.sku_code,
                s.name as sku_name,
                pca.channel_id,
                c.platform as channel_platform,
                pca.warehouse_id,
                w.name as warehouse_name,
                pca.status,
                pca.available_qty,
                pca.rule,
                pca.on_shelf_at,
                pca.created_at,
                pca.updated_at
            from product_channel_availability pca
            join skus s
              on s.sku_id = pca.sku_id
             and s.tenant_id = pca.tenant_id
            left join channels c
              on c.channel_id = pca.channel_id
             and c.tenant_id = pca.tenant_id
            left join warehouses w
              on w.warehouse_id = pca.warehouse_id
             and w.tenant_id = pca.tenant_id
            where pca.tenant_id = $1 and pca.availability_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(availability_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductChannelAvailability {
            availability_id: row.availability_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            channel_id: row.channel_id,
            channel_platform: row.channel_platform,
            warehouse_id: row.warehouse_id,
            warehouse_name: row.warehouse_name,
            status: row.status,
            available_qty: row.available_qty,
            rule: row.rule,
            on_shelf_at: row.on_shelf_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_availability(
        &self,
        input: ProductChannelAvailabilityCreate,
    ) -> anyhow::Result<ProductChannelAvailability> {
        #[derive(sqlx::FromRow)]
        struct Row {
            availability_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            channel_id: Option<uuid::Uuid>,
            channel_platform: Option<String>,
            warehouse_id: Option<uuid::Uuid>,
            warehouse_name: Option<String>,
            status: String,
            available_qty: i32,
            rule: Option<String>,
            on_shelf_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into product_channel_availability(
                tenant_id, sku_id, channel_id, warehouse_id, status, available_qty, rule, on_shelf_at
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                availability_id,
                sku_id,
                (select sku_code from skus where sku_id = $2) as sku_code,
                (select name from skus where sku_id = $2) as sku_name,
                channel_id,
                (select platform from channels where channel_id = product_channel_availability.channel_id) as channel_platform,
                warehouse_id,
                (select name from warehouses where warehouse_id = product_channel_availability.warehouse_id) as warehouse_name,
                status,
                available_qty,
                rule,
                on_shelf_at,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(input.channel_id)
        .bind(input.warehouse_id)
        .bind(&input.status)
        .bind(input.available_qty)
        .bind(&input.rule)
        .bind(input.on_shelf_at)
        .fetch_one(self.pool)
        .await?;

        Ok(ProductChannelAvailability {
            availability_id: row.availability_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            channel_id: row.channel_id,
            channel_platform: row.channel_platform,
            warehouse_id: row.warehouse_id,
            warehouse_name: row.warehouse_name,
            status: row.status,
            available_qty: row.available_qty,
            rule: row.rule,
            on_shelf_at: row.on_shelf_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_availability(
        &self,
        tenant_id: TenantId,
        availability_id: uuid::Uuid,
        input: ProductChannelAvailabilityUpdate,
    ) -> anyhow::Result<Option<ProductChannelAvailability>> {
        let mut qb = QueryBuilder::<Postgres>::new("update product_channel_availability set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(channel_id) = input.channel_id {
            separated.push("channel_id = ");
            separated.push_bind(channel_id);
            has_updates = true;
        }
        if let Some(warehouse_id) = input.warehouse_id {
            separated.push("warehouse_id = ");
            separated.push_bind(warehouse_id);
            has_updates = true;
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(available_qty) = input.available_qty {
            separated.push("available_qty = ");
            separated.push_bind(available_qty);
            has_updates = true;
        }
        if let Some(rule) = input.rule {
            separated.push("rule = ");
            separated.push_bind(rule);
            has_updates = true;
        }
        if let Some(on_shelf_at) = input.on_shelf_at {
            separated.push("on_shelf_at = ");
            separated.push_bind(on_shelf_at);
            has_updates = true;
        }

        if !has_updates {
            return self.get_availability(tenant_id, availability_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and availability_id = ");
        qb.push_bind(availability_id);
        qb.push(
            r#"
            returning
                availability_id,
                sku_id,
                (select sku_code from skus where sku_id = product_channel_availability.sku_id) as sku_code,
                (select name from skus where sku_id = product_channel_availability.sku_id) as sku_name,
                channel_id,
                (select platform from channels where channel_id = product_channel_availability.channel_id) as channel_platform,
                warehouse_id,
                (select name from warehouses where warehouse_id = product_channel_availability.warehouse_id) as warehouse_name,
                status,
                available_qty,
                rule,
                on_shelf_at,
                created_at,
                updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            availability_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            channel_id: Option<uuid::Uuid>,
            channel_platform: Option<String>,
            warehouse_id: Option<uuid::Uuid>,
            warehouse_name: Option<String>,
            status: String,
            available_qty: i32,
            rule: Option<String>,
            on_shelf_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ProductChannelAvailability {
            availability_id: row.availability_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            channel_id: row.channel_id,
            channel_platform: row.channel_platform,
            warehouse_id: row.warehouse_id,
            warehouse_name: row.warehouse_name,
            status: row.status,
            available_qty: row.available_qty,
            rule: row.rule,
            on_shelf_at: row.on_shelf_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_availability(
        &self,
        tenant_id: TenantId,
        availability_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductChannelAvailability>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            availability_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            channel_id: Option<uuid::Uuid>,
            channel_platform: Option<String>,
            warehouse_id: Option<uuid::Uuid>,
            warehouse_name: Option<String>,
            status: String,
            available_qty: i32,
            rule: Option<String>,
            on_shelf_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from product_channel_availability
            where tenant_id = $1 and availability_id = $2
            returning
                availability_id,
                sku_id,
                (select sku_code from skus where sku_id = product_channel_availability.sku_id) as sku_code,
                (select name from skus where sku_id = product_channel_availability.sku_id) as sku_name,
                channel_id,
                (select platform from channels where channel_id = product_channel_availability.channel_id) as channel_platform,
                warehouse_id,
                (select name from warehouses where warehouse_id = product_channel_availability.warehouse_id) as warehouse_name,
                status,
                available_qty,
                rule,
                on_shelf_at,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(availability_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductChannelAvailability {
            availability_id: row.availability_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            channel_id: row.channel_id,
            channel_platform: row.channel_platform,
            warehouse_id: row.warehouse_id,
            warehouse_name: row.warehouse_name,
            status: row.status,
            available_qty: row.available_qty,
            rule: row.rule,
            on_shelf_at: row.on_shelf_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProductChannelAvailabilityListFilters,
) {
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and pca.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(channel_id) = filters.channel_id {
        qb.push(" and pca.channel_id = ");
        qb.push_bind(channel_id);
    }
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and pca.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = filters.status.as_ref() {
        qb.push(" and pca.status = ");
        qb.push_bind(status);
    }
}
