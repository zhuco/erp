use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseArea {
    pub area_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub code: String,
    pub name: String,
    pub area_type: String,
    pub status: String,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct WarehouseAreaListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub area_type: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct WarehouseAreaListResult {
    pub items: Vec<WarehouseArea>,
    pub total: i64,
}

pub struct WarehouseAreaCreate {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub code: String,
    pub name: String,
    pub area_type: String,
    pub status: String,
    pub priority: i32,
}

pub struct WarehouseAreaUpdate {
    pub name: Option<String>,
    pub area_type: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i32>,
}

pub struct WarehouseAreasRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WarehouseAreasRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_areas(
        &self,
        tenant_id: TenantId,
        filters: WarehouseAreaListFilters,
    ) -> anyhow::Result<WarehouseAreaListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from warehouse_areas
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_area_filters(&mut count_qb, &filters, keyword.as_deref());
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select area_id, warehouse_id, code, name, area_type, status, priority,
                   created_at, updated_at
            from warehouse_areas
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_area_filters(&mut qb, &filters, keyword.as_deref());
        qb.push(" order by updated_at desc, area_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            area_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            code: String,
            name: String,
            area_type: String,
            status: String,
            priority: i32,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| WarehouseArea {
                area_id: row.area_id,
                tenant_id,
                warehouse_id: row.warehouse_id,
                code: row.code,
                name: row.name,
                area_type: row.area_type,
                status: row.status,
                priority: row.priority,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(WarehouseAreaListResult { items, total })
    }

    pub async fn create_area(&self, input: WarehouseAreaCreate) -> anyhow::Result<WarehouseArea> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let area = self.create_area_with_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(area)
    }

    pub async fn update_area(
        &self,
        tenant_id: TenantId,
        area_id: uuid::Uuid,
        input: WarehouseAreaUpdate,
    ) -> anyhow::Result<Option<WarehouseArea>> {
        let mut qb = QueryBuilder::<Postgres>::new("update warehouse_areas set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(area_type) = input.area_type {
            separated.push("area_type = ");
            separated.push_bind(area_type);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(priority) = input.priority {
            separated.push("priority = ");
            separated.push_bind(priority);
        }
        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and area_id = ");
        qb.push_bind(area_id);
        qb.push(
            " returning area_id, warehouse_id, code, name, area_type, status, priority, \
             created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            area_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            code: String,
            name: String,
            area_type: String,
            status: String,
            priority: i32,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| WarehouseArea {
            area_id: row.area_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            code: row.code,
            name: row.name,
            area_type: row.area_type,
            status: row.status,
            priority: row.priority,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_area(
        &self,
        tenant_id: TenantId,
        area_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WarehouseArea>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            area_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            code: String,
            name: String,
            area_type: String,
            status: String,
            priority: i32,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from warehouse_areas
            where tenant_id = $1 and area_id = $2
            returning area_id, warehouse_id, code, name, area_type, status, priority,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(area_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| WarehouseArea {
            area_id: row.area_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            code: row.code,
            name: row.name,
            area_type: row.area_type,
            status: row.status,
            priority: row.priority,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn create_area_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: WarehouseAreaCreate,
    ) -> anyhow::Result<WarehouseArea> {
        #[derive(sqlx::FromRow)]
        struct Row {
            area_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            code: String,
            name: String,
            area_type: String,
            status: String,
            priority: i32,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into warehouse_areas(
                tenant_id, warehouse_id, code, name, area_type, status, priority
            ) values ($1,$2,$3,$4,$5,$6,$7)
            on conflict (tenant_id, warehouse_id, code)
            do update set
                name = excluded.name,
                area_type = excluded.area_type,
                status = excluded.status,
                priority = excluded.priority,
                updated_at = now()
            returning area_id, warehouse_id, code, name, area_type, status, priority,
                      created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(&input.code)
        .bind(&input.name)
        .bind(&input.area_type)
        .bind(&input.status)
        .bind(input.priority)
        .fetch_one(&mut **tx)
        .await?;

        Ok(WarehouseArea {
            area_id: row.area_id,
            tenant_id: input.tenant_id,
            warehouse_id: row.warehouse_id,
            code: row.code,
            name: row.name,
            area_type: row.area_type,
            status: row.status,
            priority: row.priority,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn apply_area_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a WarehouseAreaListFilters,
    keyword: Option<&'a str>,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(area_type) = &filters.area_type {
        qb.push(" and area_type = ");
        qb.push_bind(area_type);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
    if let Some(keyword) = keyword {
        qb.push(" and (code ilike ");
        qb.push_bind(keyword);
        qb.push(" or name ilike ");
        qb.push_bind(keyword);
        qb.push(")");
    }
}
