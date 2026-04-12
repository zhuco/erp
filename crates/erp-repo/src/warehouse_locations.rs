use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseLocation {
    pub location_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub area_id: Option<uuid::Uuid>,
    pub code: String,
    pub name: Option<String>,
    pub barcode: Option<String>,
    pub volume: Option<f64>,
    pub max_weight: Option<f64>,
    pub is_picking: bool,
    pub is_disabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct WarehouseLocationListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub area_id: Option<uuid::Uuid>,
    pub is_picking: Option<bool>,
    pub is_disabled: Option<bool>,
    pub keyword: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct WarehouseLocationListResult {
    pub items: Vec<WarehouseLocation>,
    pub total: i64,
}

pub struct WarehouseLocationCreate {
    pub tenant_id: TenantId,
    pub warehouse_id: uuid::Uuid,
    pub area_id: Option<uuid::Uuid>,
    pub code: String,
    pub name: Option<String>,
    pub barcode: Option<String>,
    pub volume: Option<f64>,
    pub max_weight: Option<f64>,
    pub is_picking: bool,
    pub is_disabled: bool,
}

pub struct WarehouseLocationUpdate {
    pub area_id: Option<Option<uuid::Uuid>>,
    pub name: Option<Option<String>>,
    pub barcode: Option<Option<String>>,
    pub volume: Option<Option<f64>>,
    pub max_weight: Option<Option<f64>>,
    pub is_picking: Option<bool>,
    pub is_disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationBalanceRecord {
    pub warehouse_id: uuid::Uuid,
    pub location_id: uuid::Uuid,
    pub location_code: Option<String>,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub on_hand: i64,
    pub updated_at: DateTime<Utc>,
}

pub struct LocationBalanceListFilters {
    pub warehouse_id: Option<uuid::Uuid>,
    pub location_id: Option<uuid::Uuid>,
    pub sku_id: Option<uuid::Uuid>,
    pub offset: i64,
    pub limit: i64,
}

pub struct LocationBalanceListResult {
    pub items: Vec<LocationBalanceRecord>,
    pub total: i64,
}

pub struct WarehouseLocationsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WarehouseLocationsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_locations(
        &self,
        tenant_id: TenantId,
        filters: WarehouseLocationListFilters,
    ) -> anyhow::Result<WarehouseLocationListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from warehouse_locations
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_location_filters(&mut count_qb, &filters, keyword.as_deref());
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select location_id, warehouse_id, area_id, code, name, barcode, volume,
                   max_weight, is_picking, is_disabled, created_at, updated_at
            from warehouse_locations
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_location_filters(&mut qb, &filters, keyword.as_deref());
        qb.push(" order by updated_at desc, location_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            location_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            area_id: Option<uuid::Uuid>,
            code: String,
            name: Option<String>,
            barcode: Option<String>,
            volume: Option<f64>,
            max_weight: Option<f64>,
            is_picking: bool,
            is_disabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| WarehouseLocation {
                location_id: row.location_id,
                tenant_id,
                warehouse_id: row.warehouse_id,
                area_id: row.area_id,
                code: row.code,
                name: row.name,
                barcode: row.barcode,
                volume: row.volume,
                max_weight: row.max_weight,
                is_picking: row.is_picking,
                is_disabled: row.is_disabled,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(WarehouseLocationListResult { items, total })
    }

    pub async fn create_location(
        &self,
        input: WarehouseLocationCreate,
    ) -> anyhow::Result<WarehouseLocation> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let location = self.create_location_with_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(location)
    }

    pub async fn create_locations(
        &self,
        tenant_id: TenantId,
        locations: Vec<WarehouseLocationCreate>,
    ) -> anyhow::Result<Vec<WarehouseLocation>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let mut created = Vec::with_capacity(locations.len());
        for location in locations {
            if location.tenant_id != tenant_id {
                continue;
            }
            created.push(self.create_location_with_tx(&mut tx, location).await?);
        }
        tx.commit().await?;
        Ok(created)
    }

    pub async fn update_location(
        &self,
        tenant_id: TenantId,
        location_id: uuid::Uuid,
        input: WarehouseLocationUpdate,
    ) -> anyhow::Result<Option<WarehouseLocation>> {
        let mut qb = QueryBuilder::<Postgres>::new("update warehouse_locations set ");
        let mut separated = qb.separated(", ");

        if let Some(area_id) = input.area_id {
            separated.push("area_id = ");
            separated.push_bind(area_id);
        }
        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(barcode) = input.barcode {
            separated.push("barcode = ");
            separated.push_bind(barcode);
        }
        if let Some(volume) = input.volume {
            separated.push("volume = ");
            separated.push_bind(volume);
        }
        if let Some(max_weight) = input.max_weight {
            separated.push("max_weight = ");
            separated.push_bind(max_weight);
        }
        if let Some(is_picking) = input.is_picking {
            separated.push("is_picking = ");
            separated.push_bind(is_picking);
        }
        if let Some(is_disabled) = input.is_disabled {
            separated.push("is_disabled = ");
            separated.push_bind(is_disabled);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and location_id = ");
        qb.push_bind(location_id);
        qb.push(
            " returning location_id, warehouse_id, area_id, code, name, barcode, volume, \
             max_weight, is_picking, is_disabled, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            location_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            area_id: Option<uuid::Uuid>,
            code: String,
            name: Option<String>,
            barcode: Option<String>,
            volume: Option<f64>,
            max_weight: Option<f64>,
            is_picking: bool,
            is_disabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| WarehouseLocation {
            location_id: row.location_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            area_id: row.area_id,
            code: row.code,
            name: row.name,
            barcode: row.barcode,
            volume: row.volume,
            max_weight: row.max_weight,
            is_picking: row.is_picking,
            is_disabled: row.is_disabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_location(
        &self,
        tenant_id: TenantId,
        location_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WarehouseLocation>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            location_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            area_id: Option<uuid::Uuid>,
            code: String,
            name: Option<String>,
            barcode: Option<String>,
            volume: Option<f64>,
            max_weight: Option<f64>,
            is_picking: bool,
            is_disabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from warehouse_locations
            where tenant_id = $1 and location_id = $2
            returning location_id, warehouse_id, area_id, code, name, barcode, volume,
                      max_weight, is_picking, is_disabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(location_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| WarehouseLocation {
            location_id: row.location_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            area_id: row.area_id,
            code: row.code,
            name: row.name,
            barcode: row.barcode,
            volume: row.volume,
            max_weight: row.max_weight,
            is_picking: row.is_picking,
            is_disabled: row.is_disabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn list_location_balances(
        &self,
        tenant_id: TenantId,
        filters: LocationBalanceListFilters,
    ) -> anyhow::Result<LocationBalanceListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from inventory_location_balances ilb
            where ilb.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_balance_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                ilb.warehouse_id,
                ilb.location_id,
                wl.code as location_code,
                ilb.sku_id,
                s.sku_code,
                ilb.on_hand,
                ilb.updated_at
            from inventory_location_balances ilb
            left join warehouse_locations wl on wl.location_id = ilb.location_id
            left join skus s on s.sku_id = ilb.sku_id
            where ilb.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_balance_filters(&mut qb, &filters);
        qb.push(" order by ilb.updated_at desc, ilb.location_id desc, ilb.sku_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            warehouse_id: uuid::Uuid,
            location_id: uuid::Uuid,
            location_code: Option<String>,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            on_hand: i64,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| LocationBalanceRecord {
                warehouse_id: row.warehouse_id,
                location_id: row.location_id,
                location_code: row.location_code,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                on_hand: row.on_hand,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(LocationBalanceListResult { items, total })
    }

    async fn create_location_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        input: WarehouseLocationCreate,
    ) -> anyhow::Result<WarehouseLocation> {
        #[derive(sqlx::FromRow)]
        struct Row {
            location_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            area_id: Option<uuid::Uuid>,
            code: String,
            name: Option<String>,
            barcode: Option<String>,
            volume: Option<f64>,
            max_weight: Option<f64>,
            is_picking: bool,
            is_disabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into warehouse_locations(
                tenant_id, warehouse_id, area_id, code, name, barcode, volume,
                max_weight, is_picking, is_disabled
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            on conflict (tenant_id, warehouse_id, code)
            do update set
                area_id = excluded.area_id,
                name = excluded.name,
                barcode = excluded.barcode,
                volume = excluded.volume,
                max_weight = excluded.max_weight,
                is_picking = excluded.is_picking,
                is_disabled = excluded.is_disabled,
                updated_at = now()
            returning location_id, warehouse_id, area_id, code, name, barcode, volume,
                      max_weight, is_picking, is_disabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(input.area_id)
        .bind(&input.code)
        .bind(&input.name)
        .bind(&input.barcode)
        .bind(input.volume)
        .bind(input.max_weight)
        .bind(input.is_picking)
        .bind(input.is_disabled)
        .fetch_one(&mut **tx)
        .await?;

        Ok(WarehouseLocation {
            location_id: row.location_id,
            tenant_id: input.tenant_id,
            warehouse_id: row.warehouse_id,
            area_id: row.area_id,
            code: row.code,
            name: row.name,
            barcode: row.barcode,
            volume: row.volume,
            max_weight: row.max_weight,
            is_picking: row.is_picking,
            is_disabled: row.is_disabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn apply_location_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a WarehouseLocationListFilters,
    keyword: Option<&'a str>,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(area_id) = filters.area_id {
        qb.push(" and area_id = ");
        qb.push_bind(area_id);
    }
    if let Some(is_picking) = filters.is_picking {
        qb.push(" and is_picking = ");
        qb.push_bind(is_picking);
    }
    if let Some(is_disabled) = filters.is_disabled {
        qb.push(" and is_disabled = ");
        qb.push_bind(is_disabled);
    }
    if let Some(keyword) = keyword {
        qb.push(" and (code ilike ");
        qb.push_bind(keyword);
        qb.push(" or name ilike ");
        qb.push_bind(keyword);
        qb.push(" or barcode ilike ");
        qb.push_bind(keyword);
        qb.push(")");
    }
}

fn apply_balance_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a LocationBalanceListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and ilb.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(location_id) = filters.location_id {
        qb.push(" and ilb.location_id = ");
        qb.push_bind(location_id);
    }
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and ilb.sku_id = ");
        qb.push_bind(sku_id);
    }
}
