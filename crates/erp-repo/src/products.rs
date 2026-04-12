use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub product_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub product_code: Option<String>,
    pub name: String,
    pub alias: Option<String>,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub base_unit: Option<String>,
    pub second_unit: Option<String>,
    pub default_supplier_id: Option<uuid::Uuid>,
    pub default_warehouse_id: Option<uuid::Uuid>,
    pub primary_barcode: Option<String>,
    pub product_tags: Vec<String>,
    pub remark: Option<String>,
    pub warning_stock: Value,
    pub batch_enabled: bool,
    pub shelf_life_days: Option<i32>,
    pub forbid_sale_days: Option<i32>,
    pub serial_enabled: bool,
    pub serial_push_platform: bool,
    pub attrs: Value,
    pub allow_negative_stock: bool,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProductCursor {
    pub created_at: DateTime<Utc>,
    pub product_id: uuid::Uuid,
}

impl ProductCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.product_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let product_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            product_id,
        })
    }
}

pub struct ProductListFilters {
    pub keyword: Option<String>,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub cursor: Option<ProductCursor>,
    pub limit: i64,
}

pub struct ProductListResult {
    pub items: Vec<Product>,
    pub next_cursor: Option<ProductCursor>,
}

pub struct ProductCreate {
    pub tenant_id: TenantId,
    pub product_code: Option<String>,
    pub name: String,
    pub alias: Option<String>,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub base_unit: Option<String>,
    pub second_unit: Option<String>,
    pub default_supplier_id: Option<uuid::Uuid>,
    pub default_warehouse_id: Option<uuid::Uuid>,
    pub primary_barcode: Option<String>,
    pub product_tags: Vec<String>,
    pub remark: Option<String>,
    pub warning_stock: Value,
    pub batch_enabled: bool,
    pub shelf_life_days: Option<i32>,
    pub forbid_sale_days: Option<i32>,
    pub serial_enabled: bool,
    pub serial_push_platform: bool,
    pub attrs: Value,
    pub allow_negative_stock: bool,
}

pub struct ProductUpdate {
    pub product_code: Option<String>,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub base_unit: Option<String>,
    pub second_unit: Option<String>,
    pub default_supplier_id: Option<uuid::Uuid>,
    pub default_warehouse_id: Option<uuid::Uuid>,
    pub primary_barcode: Option<String>,
    pub product_tags: Option<Vec<String>>,
    pub remark: Option<String>,
    pub warning_stock: Option<Value>,
    pub batch_enabled: Option<bool>,
    pub shelf_life_days: Option<i32>,
    pub forbid_sale_days: Option<i32>,
    pub serial_enabled: Option<bool>,
    pub serial_push_platform: Option<bool>,
    pub attrs: Option<Value>,
    pub status: Option<String>,
    pub allow_negative_stock: Option<bool>,
}

pub struct ProductsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_products(
        &self,
        tenant_id: TenantId,
        filters: ProductListFilters,
    ) -> anyhow::Result<ProductListResult> {
        let keyword = filters
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                product_id,
                product_code,
                name,
                alias,
                brand,
                category,
                base_unit,
                second_unit,
                default_supplier_id,
                default_warehouse_id,
                primary_barcode,
                product_tags,
                remark,
                warning_stock,
                batch_enabled,
                shelf_life_days,
                forbid_sale_days,
                serial_enabled,
                serial_push_platform,
                attrs,
                allow_negative_stock,
                status,
                created_at,
                updated_at
            from products
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and status <> 'DELETED'");

        if let Some(keyword) = &keyword {
            qb.push(" and (name ilike ");
            qb.push_bind(keyword);
            qb.push(" or product_code ilike ");
            qb.push_bind(keyword);
            qb.push(" or alias ilike ");
            qb.push_bind(keyword);
            qb.push(" or primary_barcode ilike ");
            qb.push_bind(keyword);
            qb.push(")");
        }
        if let Some(brand) = filters.brand {
            qb.push(" and brand = ");
            qb.push_bind(brand);
        }
        if let Some(category) = filters.category {
            qb.push(" and category = ");
            qb.push_bind(category);
        }
        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, product_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.product_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, product_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            product_id: uuid::Uuid,
            product_code: Option<String>,
            name: String,
            alias: Option<String>,
            brand: Option<String>,
            category: Option<String>,
            base_unit: Option<String>,
            second_unit: Option<String>,
            default_supplier_id: Option<uuid::Uuid>,
            default_warehouse_id: Option<uuid::Uuid>,
            primary_barcode: Option<String>,
            product_tags: Vec<String>,
            remark: Option<String>,
            warning_stock: Value,
            batch_enabled: bool,
            shelf_life_days: Option<i32>,
            forbid_sale_days: Option<i32>,
            serial_enabled: bool,
            serial_push_platform: bool,
            attrs: Value,
            allow_negative_stock: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| ProductCursor {
            created_at: row.created_at,
            product_id: row.product_id,
        });

        let items = rows
            .into_iter()
            .map(|row| Product {
                product_id: row.product_id,
                tenant_id,
                product_code: row.product_code,
                name: row.name,
                alias: row.alias,
                brand: row.brand,
                category: row.category,
                base_unit: row.base_unit,
                second_unit: row.second_unit,
                default_supplier_id: row.default_supplier_id,
                default_warehouse_id: row.default_warehouse_id,
                primary_barcode: row.primary_barcode,
                product_tags: row.product_tags,
                remark: row.remark,
                warning_stock: row.warning_stock,
                batch_enabled: row.batch_enabled,
                shelf_life_days: row.shelf_life_days,
                forbid_sale_days: row.forbid_sale_days,
                serial_enabled: row.serial_enabled,
                serial_push_platform: row.serial_push_platform,
                attrs: row.attrs,
                allow_negative_stock: row.allow_negative_stock,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProductListResult { items, next_cursor })
    }

    pub async fn get_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Product>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            product_id: uuid::Uuid,
            product_code: Option<String>,
            name: String,
            alias: Option<String>,
            brand: Option<String>,
            category: Option<String>,
            base_unit: Option<String>,
            second_unit: Option<String>,
            default_supplier_id: Option<uuid::Uuid>,
            default_warehouse_id: Option<uuid::Uuid>,
            primary_barcode: Option<String>,
            product_tags: Vec<String>,
            remark: Option<String>,
            warning_stock: Value,
            batch_enabled: bool,
            shelf_life_days: Option<i32>,
            forbid_sale_days: Option<i32>,
            serial_enabled: bool,
            serial_push_platform: bool,
            attrs: Value,
            allow_negative_stock: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                product_id,
                product_code,
                name,
                alias,
                brand,
                category,
                base_unit,
                second_unit,
                default_supplier_id,
                default_warehouse_id,
                primary_barcode,
                product_tags,
                remark,
                warning_stock,
                batch_enabled,
                shelf_life_days,
                forbid_sale_days,
                serial_enabled,
                serial_push_platform,
                attrs,
                allow_negative_stock,
                status,
                created_at,
                updated_at
            from products
            where tenant_id = $1 and product_id = $2 and status <> 'DELETED'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Product {
            product_id: row.product_id,
            tenant_id,
            product_code: row.product_code,
            name: row.name,
            alias: row.alias,
            brand: row.brand,
            category: row.category,
            base_unit: row.base_unit,
            second_unit: row.second_unit,
            default_supplier_id: row.default_supplier_id,
            default_warehouse_id: row.default_warehouse_id,
            primary_barcode: row.primary_barcode,
            product_tags: row.product_tags,
            remark: row.remark,
            warning_stock: row.warning_stock,
            batch_enabled: row.batch_enabled,
            shelf_life_days: row.shelf_life_days,
            forbid_sale_days: row.forbid_sale_days,
            serial_enabled: row.serial_enabled,
            serial_push_platform: row.serial_push_platform,
            attrs: row.attrs,
            allow_negative_stock: row.allow_negative_stock,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_product(&self, input: ProductCreate) -> anyhow::Result<Product> {
        #[derive(sqlx::FromRow)]
        struct Row {
            product_id: uuid::Uuid,
            product_code: Option<String>,
            name: String,
            alias: Option<String>,
            brand: Option<String>,
            category: Option<String>,
            base_unit: Option<String>,
            second_unit: Option<String>,
            default_supplier_id: Option<uuid::Uuid>,
            default_warehouse_id: Option<uuid::Uuid>,
            primary_barcode: Option<String>,
            product_tags: Vec<String>,
            remark: Option<String>,
            warning_stock: Value,
            batch_enabled: bool,
            shelf_life_days: Option<i32>,
            forbid_sale_days: Option<i32>,
            serial_enabled: bool,
            serial_push_platform: bool,
            attrs: Value,
            allow_negative_stock: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into products(
                tenant_id,
                product_code,
                name,
                alias,
                brand,
                category,
                base_unit,
                second_unit,
                default_supplier_id,
                default_warehouse_id,
                primary_barcode,
                product_tags,
                remark,
                warning_stock,
                batch_enabled,
                shelf_life_days,
                forbid_sale_days,
                serial_enabled,
                serial_push_platform,
                attrs,
                allow_negative_stock
            )
            values (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21
            )
            returning
                product_id,
                product_code,
                name,
                alias,
                brand,
                category,
                base_unit,
                second_unit,
                default_supplier_id,
                default_warehouse_id,
                primary_barcode,
                product_tags,
                remark,
                warning_stock,
                batch_enabled,
                shelf_life_days,
                forbid_sale_days,
                serial_enabled,
                serial_push_platform,
                attrs,
                allow_negative_stock,
                status,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.product_code)
        .bind(&input.name)
        .bind(&input.alias)
        .bind(&input.brand)
        .bind(&input.category)
        .bind(&input.base_unit)
        .bind(&input.second_unit)
        .bind(input.default_supplier_id)
        .bind(input.default_warehouse_id)
        .bind(&input.primary_barcode)
        .bind(&input.product_tags)
        .bind(&input.remark)
        .bind(&input.warning_stock)
        .bind(input.batch_enabled)
        .bind(input.shelf_life_days)
        .bind(input.forbid_sale_days)
        .bind(input.serial_enabled)
        .bind(input.serial_push_platform)
        .bind(&input.attrs)
        .bind(input.allow_negative_stock)
        .fetch_one(self.pool)
        .await?;

        Ok(Product {
            product_id: row.product_id,
            tenant_id: input.tenant_id,
            product_code: row.product_code,
            name: row.name,
            alias: row.alias,
            brand: row.brand,
            category: row.category,
            base_unit: row.base_unit,
            second_unit: row.second_unit,
            default_supplier_id: row.default_supplier_id,
            default_warehouse_id: row.default_warehouse_id,
            primary_barcode: row.primary_barcode,
            product_tags: row.product_tags,
            remark: row.remark,
            warning_stock: row.warning_stock,
            batch_enabled: row.batch_enabled,
            shelf_life_days: row.shelf_life_days,
            forbid_sale_days: row.forbid_sale_days,
            serial_enabled: row.serial_enabled,
            serial_push_platform: row.serial_push_platform,
            attrs: row.attrs,
            allow_negative_stock: row.allow_negative_stock,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
        input: ProductUpdate,
    ) -> anyhow::Result<Option<Product>> {
        let mut qb = QueryBuilder::<Postgres>::new("update products set ");
        let mut separated = qb.separated(", ");

        if let Some(product_code) = input.product_code {
            separated.push("product_code = ");
            separated.push_bind(product_code);
        }
        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(alias) = input.alias {
            separated.push("alias = ");
            separated.push_bind(alias);
        }
        if let Some(brand) = input.brand {
            separated.push("brand = ");
            separated.push_bind(brand);
        }
        if let Some(category) = input.category {
            separated.push("category = ");
            separated.push_bind(category);
        }
        if let Some(base_unit) = input.base_unit {
            separated.push("base_unit = ");
            separated.push_bind(base_unit);
        }
        if let Some(second_unit) = input.second_unit {
            separated.push("second_unit = ");
            separated.push_bind(second_unit);
        }
        if let Some(default_supplier_id) = input.default_supplier_id {
            separated.push("default_supplier_id = ");
            separated.push_bind(default_supplier_id);
        }
        if let Some(default_warehouse_id) = input.default_warehouse_id {
            separated.push("default_warehouse_id = ");
            separated.push_bind(default_warehouse_id);
        }
        if let Some(primary_barcode) = input.primary_barcode {
            separated.push("primary_barcode = ");
            separated.push_bind(primary_barcode);
        }
        if let Some(product_tags) = input.product_tags {
            separated.push("product_tags = ");
            separated.push_bind(product_tags);
        }
        if let Some(remark) = input.remark {
            separated.push("remark = ");
            separated.push_bind(remark);
        }
        if let Some(warning_stock) = input.warning_stock {
            separated.push("warning_stock = ");
            separated.push_bind(warning_stock);
        }
        if let Some(batch_enabled) = input.batch_enabled {
            separated.push("batch_enabled = ");
            separated.push_bind(batch_enabled);
        }
        if let Some(shelf_life_days) = input.shelf_life_days {
            separated.push("shelf_life_days = ");
            separated.push_bind(shelf_life_days);
        }
        if let Some(forbid_sale_days) = input.forbid_sale_days {
            separated.push("forbid_sale_days = ");
            separated.push_bind(forbid_sale_days);
        }
        if let Some(serial_enabled) = input.serial_enabled {
            separated.push("serial_enabled = ");
            separated.push_bind(serial_enabled);
        }
        if let Some(serial_push_platform) = input.serial_push_platform {
            separated.push("serial_push_platform = ");
            separated.push_bind(serial_push_platform);
        }
        if let Some(attrs) = input.attrs {
            separated.push("attrs = ");
            separated.push_bind(attrs);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(allow_negative_stock) = input.allow_negative_stock {
            separated.push("allow_negative_stock = ");
            separated.push_bind(allow_negative_stock);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and product_id = ");
        qb.push_bind(product_id);
        qb.push(" and status <> 'DELETED'");
        qb.push(
            r#"
            returning
                product_id,
                product_code,
                name,
                alias,
                brand,
                category,
                base_unit,
                second_unit,
                default_supplier_id,
                default_warehouse_id,
                primary_barcode,
                product_tags,
                remark,
                warning_stock,
                batch_enabled,
                shelf_life_days,
                forbid_sale_days,
                serial_enabled,
                serial_push_platform,
                attrs,
                allow_negative_stock,
                status,
                created_at,
                updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            product_id: uuid::Uuid,
            product_code: Option<String>,
            name: String,
            alias: Option<String>,
            brand: Option<String>,
            category: Option<String>,
            base_unit: Option<String>,
            second_unit: Option<String>,
            default_supplier_id: Option<uuid::Uuid>,
            default_warehouse_id: Option<uuid::Uuid>,
            primary_barcode: Option<String>,
            product_tags: Vec<String>,
            remark: Option<String>,
            warning_stock: Value,
            batch_enabled: bool,
            shelf_life_days: Option<i32>,
            forbid_sale_days: Option<i32>,
            serial_enabled: bool,
            serial_push_platform: bool,
            attrs: Value,
            allow_negative_stock: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| Product {
            product_id: row.product_id,
            tenant_id,
            product_code: row.product_code,
            name: row.name,
            alias: row.alias,
            brand: row.brand,
            category: row.category,
            base_unit: row.base_unit,
            second_unit: row.second_unit,
            default_supplier_id: row.default_supplier_id,
            default_warehouse_id: row.default_warehouse_id,
            primary_barcode: row.primary_barcode,
            product_tags: row.product_tags,
            remark: row.remark,
            warning_stock: row.warning_stock,
            batch_enabled: row.batch_enabled,
            shelf_life_days: row.shelf_life_days,
            forbid_sale_days: row.forbid_sale_days,
            serial_enabled: row.serial_enabled,
            serial_push_platform: row.serial_push_platform,
            attrs: row.attrs,
            allow_negative_stock: row.allow_negative_stock,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_product(
        &self,
        tenant_id: TenantId,
        product_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Product>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            product_id: uuid::Uuid,
            product_code: Option<String>,
            name: String,
            alias: Option<String>,
            brand: Option<String>,
            category: Option<String>,
            base_unit: Option<String>,
            second_unit: Option<String>,
            default_supplier_id: Option<uuid::Uuid>,
            default_warehouse_id: Option<uuid::Uuid>,
            primary_barcode: Option<String>,
            product_tags: Vec<String>,
            remark: Option<String>,
            warning_stock: Value,
            batch_enabled: bool,
            shelf_life_days: Option<i32>,
            forbid_sale_days: Option<i32>,
            serial_enabled: bool,
            serial_push_platform: bool,
            attrs: Value,
            allow_negative_stock: bool,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update products
            set status = 'DELETED',
                updated_at = now()
            where tenant_id = $1 and product_id = $2 and status <> 'DELETED'
            returning
                product_id,
                product_code,
                name,
                alias,
                brand,
                category,
                base_unit,
                second_unit,
                default_supplier_id,
                default_warehouse_id,
                primary_barcode,
                product_tags,
                remark,
                warning_stock,
                batch_enabled,
                shelf_life_days,
                forbid_sale_days,
                serial_enabled,
                serial_push_platform,
                attrs,
                allow_negative_stock,
                status,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(product_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| Product {
            product_id: row.product_id,
            tenant_id,
            product_code: row.product_code,
            name: row.name,
            alias: row.alias,
            brand: row.brand,
            category: row.category,
            base_unit: row.base_unit,
            second_unit: row.second_unit,
            default_supplier_id: row.default_supplier_id,
            default_warehouse_id: row.default_warehouse_id,
            primary_barcode: row.primary_barcode,
            product_tags: row.product_tags,
            remark: row.remark,
            warning_stock: row.warning_stock,
            batch_enabled: row.batch_enabled,
            shelf_life_days: row.shelf_life_days,
            forbid_sale_days: row.forbid_sale_days,
            serial_enabled: row.serial_enabled,
            serial_push_platform: row.serial_push_platform,
            attrs: row.attrs,
            allow_negative_stock: row.allow_negative_stock,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
