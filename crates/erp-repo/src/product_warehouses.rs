use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductWarehouse {
    pub tenant_id: TenantId,
    pub product_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub warehouse_code: String,
    pub warehouse_name: String,
    pub is_default: bool,
    pub allow_negative_stock: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProductWarehouseCreate {
    pub tenant_id: TenantId,
    pub product_id: uuid::Uuid,
    pub warehouse_id: uuid::Uuid,
    pub is_default: bool,
    pub allow_negative_stock: bool,
}

pub struct ProductWarehouseUpdate {
    pub is_default: Option<bool>,
    pub allow_negative_stock: Option<bool>,
}

pub struct ProductWarehousesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductWarehousesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_product_warehouses(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<ProductWarehouse>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            product_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            warehouse_code: String,
            warehouse_name: String,
            is_default: bool,
            allow_negative_stock: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select
                pw.product_id,
                pw.warehouse_id,
                w.code as warehouse_code,
                w.name as warehouse_name,
                pw.is_default,
                pw.allow_negative_stock,
                pw.created_at,
                pw.updated_at
            from product_warehouses pw
            join warehouses w
              on w.warehouse_id = pw.warehouse_id
             and w.tenant_id = pw.tenant_id
            where pw.tenant_id = $1 and pw.product_id = $2
            order by pw.is_default desc, w.name
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(product_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ProductWarehouse {
                tenant_id,
                product_id: row.product_id,
                warehouse_id: row.warehouse_id,
                warehouse_code: row.warehouse_code,
                warehouse_name: row.warehouse_name,
                is_default: row.is_default,
                allow_negative_stock: row.allow_negative_stock,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    pub async fn create_product_warehouse(
        &self,
        input: ProductWarehouseCreate,
    ) -> anyhow::Result<ProductWarehouse> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        if input.is_default {
            sqlx::query(
                r#"
                update product_warehouses
                set is_default = false,
                    updated_at = now()
                where tenant_id = $1 and product_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.product_id)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            r#"
            insert into product_warehouses(
                tenant_id, product_id, warehouse_id, is_default, allow_negative_stock
            ) values ($1, $2, $3, $4, $5)
            on conflict (tenant_id, product_id, warehouse_id)
            do update set
                is_default = excluded.is_default,
                allow_negative_stock = excluded.allow_negative_stock,
                updated_at = now()
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.product_id)
        .bind(input.warehouse_id)
        .bind(input.is_default)
        .bind(input.allow_negative_stock)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let record = fetch_product_warehouse(
            self.pool,
            input.tenant_id,
            input.product_id,
            input.warehouse_id,
        )
        .await?
        .ok_or_else(|| anyhow!("product warehouse not found"))?;
        Ok(record)
    }

    pub async fn update_product_warehouse(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        warehouse_id: uuid::Uuid,
        input: ProductWarehouseUpdate,
    ) -> anyhow::Result<Option<ProductWarehouse>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        if let Some(true) = input.is_default {
            sqlx::query(
                r#"
                update product_warehouses
                set is_default = false,
                    updated_at = now()
                where tenant_id = $1 and product_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(product_id)
            .execute(&mut *tx)
            .await?;
        }

        let mut updated = false;
        if input.is_default.is_some() || input.allow_negative_stock.is_some() {
            sqlx::query(
                r#"
                update product_warehouses
                set is_default = coalesce($4, is_default),
                    allow_negative_stock = coalesce($5, allow_negative_stock),
                    updated_at = now()
                where tenant_id = $1 and product_id = $2 and warehouse_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(product_id)
            .bind(warehouse_id)
            .bind(input.is_default)
            .bind(input.allow_negative_stock)
            .execute(&mut *tx)
            .await?;
            updated = true;
        }

        if updated {
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }
        fetch_product_warehouse(self.pool, tenant_id, product_id, warehouse_id).await
    }

    pub async fn delete_product_warehouse(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        warehouse_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductWarehouse>> {
        let sku_ids: Vec<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select sku_id
            from skus
            where tenant_id = $1 and product_id = $2 and status <> 'DELETED'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(product_id)
        .fetch_all(self.pool)
        .await?;

        if !sku_ids.is_empty() {
            let count: i64 = sqlx::query_scalar(
                r#"
                select count(*)
                from inventory_balances
                where tenant_id = $1
                  and warehouse_id = $2
                  and sku_id = any($3)
                  and (
                      coalesce(on_hand, 0) <> 0
                      or coalesce(reserved, 0) <> 0
                      or coalesce(in_transit, 0) <> 0
                      or coalesce(frozen, 0) <> 0
                  )
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .bind(&sku_ids)
            .fetch_one(self.pool)
            .await?;

            if count > 0 {
                return Err(anyhow!("warehouse has inventory"));
            }

            let batch_count: i64 = sqlx::query_scalar(
                r#"
                select count(*)
                from inventory_batch_balances
                where tenant_id = $1
                  and warehouse_id = $2
                  and sku_id = any($3)
                  and (on_hand <> 0 or reserved <> 0)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(warehouse_id)
            .bind(&sku_ids)
            .fetch_one(self.pool)
            .await?;

            if batch_count > 0 {
                return Err(anyhow!("warehouse has batch inventory"));
            }
        }

        let record =
            fetch_product_warehouse(self.pool, tenant_id, product_id, warehouse_id).await?;

        sqlx::query(
            r#"
            delete from product_warehouses
            where tenant_id = $1 and product_id = $2 and warehouse_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(product_id)
        .bind(warehouse_id)
        .execute(self.pool)
        .await?;

        Ok(record)
    }
}

async fn fetch_product_warehouse(
    pool: &PgPool,
    tenant_id: TenantId,
    product_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
) -> anyhow::Result<Option<ProductWarehouse>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        product_id: uuid::Uuid,
        warehouse_id: uuid::Uuid,
        warehouse_code: String,
        warehouse_name: String,
        is_default: bool,
        allow_negative_stock: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    }

    let row: Option<Row> = sqlx::query_as(
        r#"
        select
            pw.product_id,
            pw.warehouse_id,
            w.code as warehouse_code,
            w.name as warehouse_name,
            pw.is_default,
            pw.allow_negative_stock,
            pw.created_at,
            pw.updated_at
        from product_warehouses pw
        join warehouses w
          on w.warehouse_id = pw.warehouse_id
         and w.tenant_id = pw.tenant_id
        where pw.tenant_id = $1
          and pw.product_id = $2
          and pw.warehouse_id = $3
        "#,
    )
    .bind(uuid::Uuid::from(tenant_id))
    .bind(product_id)
    .bind(warehouse_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| ProductWarehouse {
        tenant_id,
        product_id: row.product_id,
        warehouse_id: row.warehouse_id,
        warehouse_code: row.warehouse_code,
        warehouse_name: row.warehouse_name,
        is_default: row.is_default,
        allow_negative_stock: row.allow_negative_stock,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }))
}
