use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductLifecycle {
    pub lifecycle_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub sku_name: Option<String>,
    pub status: String,
    pub inventory_qty: i32,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProductLifecycleListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ProductLifecycleListResult {
    pub items: Vec<ProductLifecycle>,
    pub total: i64,
}

pub struct ProductLifecycleCreate {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub status: String,
    pub inventory_qty: i32,
    pub remark: Option<String>,
}

pub struct ProductLifecycleUpdate {
    pub status: Option<String>,
    pub inventory_qty: Option<i32>,
    pub remark: Option<Option<String>>,
}

pub struct ProductLifecyclesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductLifecyclesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_lifecycles(
        &self,
        tenant_id: TenantId,
        filters: ProductLifecycleListFilters,
    ) -> anyhow::Result<ProductLifecycleListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from product_lifecycles pl where pl.tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pl.lifecycle_id,
                pl.sku_id,
                s.sku_code,
                s.name as sku_name,
                pl.status,
                pl.inventory_qty,
                pl.remark,
                pl.created_at,
                pl.updated_at
            from product_lifecycles pl
            join skus s
              on s.sku_id = pl.sku_id
             and s.tenant_id = pl.tenant_id
            where pl.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by pl.updated_at desc, pl.lifecycle_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            lifecycle_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            status: String,
            inventory_qty: i32,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProductLifecycle {
                lifecycle_id: row.lifecycle_id,
                tenant_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                sku_name: row.sku_name,
                status: row.status,
                inventory_qty: row.inventory_qty,
                remark: row.remark,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProductLifecycleListResult { items, total })
    }

    pub async fn get_lifecycle(
        &self,
        tenant_id: TenantId,
        lifecycle_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductLifecycle>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            lifecycle_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            status: String,
            inventory_qty: i32,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                pl.lifecycle_id,
                pl.sku_id,
                s.sku_code,
                s.name as sku_name,
                pl.status,
                pl.inventory_qty,
                pl.remark,
                pl.created_at,
                pl.updated_at
            from product_lifecycles pl
            join skus s
              on s.sku_id = pl.sku_id
             and s.tenant_id = pl.tenant_id
            where pl.tenant_id = $1 and pl.lifecycle_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(lifecycle_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductLifecycle {
            lifecycle_id: row.lifecycle_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            status: row.status,
            inventory_qty: row.inventory_qty,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_lifecycle(
        &self,
        input: ProductLifecycleCreate,
    ) -> anyhow::Result<ProductLifecycle> {
        #[derive(sqlx::FromRow)]
        struct Row {
            lifecycle_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            status: String,
            inventory_qty: i32,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into product_lifecycles(
                tenant_id, sku_id, status, inventory_qty, remark
            ) values ($1,$2,$3,$4,$5)
            on conflict (tenant_id, sku_id)
            do update set
                status = excluded.status,
                inventory_qty = excluded.inventory_qty,
                remark = excluded.remark,
                updated_at = now()
            returning
                lifecycle_id,
                sku_id,
                (select sku_code from skus where sku_id = $2) as sku_code,
                (select name from skus where sku_id = $2) as sku_name,
                status,
                inventory_qty,
                remark,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.status)
        .bind(input.inventory_qty)
        .bind(&input.remark)
        .fetch_one(self.pool)
        .await?;

        Ok(ProductLifecycle {
            lifecycle_id: row.lifecycle_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            status: row.status,
            inventory_qty: row.inventory_qty,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_lifecycle(
        &self,
        tenant_id: TenantId,
        lifecycle_id: uuid::Uuid,
        input: ProductLifecycleUpdate,
    ) -> anyhow::Result<Option<ProductLifecycle>> {
        let mut qb = QueryBuilder::<Postgres>::new("update product_lifecycles set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(inventory_qty) = input.inventory_qty {
            separated.push("inventory_qty = ");
            separated.push_bind(inventory_qty);
            has_updates = true;
        }
        if let Some(remark) = input.remark {
            separated.push("remark = ");
            separated.push_bind(remark);
            has_updates = true;
        }

        if !has_updates {
            return self.get_lifecycle(tenant_id, lifecycle_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and lifecycle_id = ");
        qb.push_bind(lifecycle_id);
        qb.push(
            r#"
            returning
                lifecycle_id,
                sku_id,
                (select sku_code from skus where sku_id = product_lifecycles.sku_id) as sku_code,
                (select name from skus where sku_id = product_lifecycles.sku_id) as sku_name,
                status,
                inventory_qty,
                remark,
                created_at,
                updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            lifecycle_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            status: String,
            inventory_qty: i32,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| ProductLifecycle {
            lifecycle_id: row.lifecycle_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            status: row.status,
            inventory_qty: row.inventory_qty,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_lifecycle(
        &self,
        tenant_id: TenantId,
        lifecycle_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductLifecycle>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            lifecycle_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            status: String,
            inventory_qty: i32,
            remark: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from product_lifecycles
            where tenant_id = $1 and lifecycle_id = $2
            returning
                lifecycle_id,
                sku_id,
                (select sku_code from skus where sku_id = product_lifecycles.sku_id) as sku_code,
                (select name from skus where sku_id = product_lifecycles.sku_id) as sku_name,
                status,
                inventory_qty,
                remark,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(lifecycle_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductLifecycle {
            lifecycle_id: row.lifecycle_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            status: row.status,
            inventory_qty: row.inventory_qty,
            remark: row.remark,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}

fn apply_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ProductLifecycleListFilters,
) {
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and pl.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(status) = filters.status.as_ref() {
        qb.push(" and pl.status = ");
        qb.push_bind(status);
    }
}
