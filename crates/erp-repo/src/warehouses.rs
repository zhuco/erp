use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    pub warehouse_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub code: String,
    pub is_default: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct WarehouseListFilters {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct WarehouseListResult {
    pub items: Vec<Warehouse>,
    pub total: i64,
}

pub struct WarehouseCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub code: String,
    pub status: String,
    pub is_default: bool,
}

pub struct WarehouseUpdate {
    pub name: Option<String>,
    pub code: Option<String>,
    pub status: Option<String>,
}

pub struct WarehousesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WarehousesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_warehouses(
        &self,
        tenant_id: TenantId,
        filters: WarehouseListFilters,
    ) -> anyhow::Result<WarehouseListResult> {
        let _ = self.ensure_default_warehouse(tenant_id).await?;
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
                    from warehouses
                    where tenant_id = $1
                      and (name ilike $2 or code ilike $2)
                      and ($3::text is null or status = $3)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(keyword)
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    select count(*)
                    from warehouses
                    where tenant_id = $1
                      and ($2::text is null or status = $2)
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(filters.status.as_deref())
                .fetch_one(self.pool)
                .await?
            }
        };

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                warehouse_id, name, code, is_default, status,
                created_at, updated_at
            from warehouses
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(keyword) = &keyword {
            qb.push(" and (name ilike ");
            qb.push_bind(keyword);
            qb.push(" or code ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }
        if let Some(status) = &filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }

        qb.push(" order by updated_at desc, warehouse_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            name: String,
            code: String,
            is_default: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| Warehouse {
                warehouse_id: row.warehouse_id,
                tenant_id,
                name: row.name,
                code: row.code,
                is_default: row.is_default,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(WarehouseListResult { items, total })
    }

    pub async fn create_warehouse(&self, input: WarehouseCreate) -> anyhow::Result<Warehouse> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let has_default: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select warehouse_id
            from warehouses
            where tenant_id = $1 and is_default = true
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .fetch_optional(&mut *tx)
        .await?;

        let should_default = if has_default.is_some() {
            input.is_default
        } else {
            true
        };

        if should_default {
            sqlx::query(
                r#"
                update warehouses
                set is_default = false
                where tenant_id = $1
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .execute(&mut *tx)
            .await?;
        }

        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            name: String,
            code: String,
            is_default: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into warehouses (tenant_id, name, code, is_default, status)
            values ($1, $2, $3, $4, $5)
            on conflict (tenant_id, code)
            do update set
                name = excluded.name,
                status = excluded.status,
                is_default = excluded.is_default,
                updated_at = now()
            returning warehouse_id, name, code, is_default, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.code)
        .bind(should_default)
        .bind(&input.status)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Warehouse {
            warehouse_id: row.warehouse_id,
            tenant_id: input.tenant_id,
            name: row.name,
            code: row.code,
            is_default: row.is_default,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        input: WarehouseUpdate,
    ) -> anyhow::Result<Option<Warehouse>> {
        let mut qb = QueryBuilder::<Postgres>::new("update warehouses set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(code) = input.code {
            separated.push("code = ");
            separated.push_bind(code);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
        qb.push(" returning warehouse_id, name, code, is_default, status, created_at, updated_at");

        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            name: String,
            code: String,
            is_default: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(Warehouse {
            warehouse_id: row.warehouse_id,
            tenant_id,
            name: row.name,
            code: row.code,
            is_default: row.is_default,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Warehouse>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            name: String,
            code: String,
            is_default: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from warehouses
            where tenant_id = $1 and warehouse_id = $2
            returning warehouse_id, name, code, is_default, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Warehouse {
            warehouse_id: row.warehouse_id,
            tenant_id,
            name: row.name,
            code: row.code,
            is_default: row.is_default,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn set_default_warehouse(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Warehouse>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        sqlx::query(
            r#"
            update warehouses
            set is_default = false
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .execute(&mut *tx)
        .await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            name: String,
            code: String,
            is_default: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update warehouses
            set is_default = true,
                status = 'ACTIVE',
                updated_at = now()
            where tenant_id = $1 and warehouse_id = $2
            returning warehouse_id, name, code, is_default, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row.map(|row| Warehouse {
            warehouse_id: row.warehouse_id,
            tenant_id,
            name: row.name,
            code: row.code,
            is_default: row.is_default,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn get_default_warehouse(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Option<Warehouse>> {
        let warehouse = self.ensure_default_warehouse(tenant_id).await?;
        Ok(Some(warehouse))
    }

    pub async fn ensure_default_warehouse(&self, tenant_id: TenantId) -> anyhow::Result<Warehouse> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            name: String,
            code: String,
            is_default: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let default_row: Option<Row> = sqlx::query_as(
            r#"
            select warehouse_id, name, code, is_default, status, created_at, updated_at
            from warehouses
            where tenant_id = $1
              and is_default = true
              and status = 'ACTIVE'
            order by updated_at desc, warehouse_id desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = default_row {
            tx.commit().await?;
            return Ok(Warehouse {
                warehouse_id: row.warehouse_id,
                tenant_id,
                name: row.name,
                code: row.code,
                is_default: row.is_default,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        let existing: Option<Row> = sqlx::query_as(
            r#"
            select warehouse_id, name, code, is_default, status, created_at, updated_at
            from warehouses
            where tenant_id = $1
              and status = 'ACTIVE'
            order by updated_at desc, warehouse_id desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(&mut *tx)
        .await?;

        let row: Row = if let Some(existing) = existing {
            sqlx::query(
                r#"
                update warehouses
                set is_default = false
                where tenant_id = $1
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .execute(&mut *tx)
            .await?;

            sqlx::query_as(
                r#"
                update warehouses
                set is_default = true,
                    updated_at = now()
                where tenant_id = $1 and warehouse_id = $2
                returning warehouse_id, name, code, is_default, status, created_at, updated_at
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(existing.warehouse_id)
            .fetch_one(&mut *tx)
            .await?
        } else {
            sqlx::query_as(
                r#"
                insert into warehouses (tenant_id, name, code, is_default, status)
                values ($1, $2, $3, true, 'ACTIVE')
                on conflict (tenant_id, code)
                do update set is_default = true, status = 'ACTIVE', updated_at = now()
                returning warehouse_id, name, code, is_default, status, created_at, updated_at
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind("默认仓库")
            .bind("DEFAULT")
            .fetch_one(&mut *tx)
            .await?
        };

        tx.commit().await?;
        Ok(Warehouse {
            warehouse_id: row.warehouse_id,
            tenant_id,
            name: row.name,
            code: row.code,
            is_default: row.is_default,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}
