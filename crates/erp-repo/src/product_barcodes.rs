use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductBarcode {
    pub barcode_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub sku_code: Option<String>,
    pub sku_name: Option<String>,
    pub barcode: String,
    pub level: String,
    pub unit: String,
    pub ratio: i32,
    pub is_default: bool,
    pub status: String,
    pub print_template: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProductBarcodeListFilters {
    pub sku_id: Option<uuid::Uuid>,
    pub barcode: Option<String>,
    pub status: Option<String>,
    pub level: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

pub struct ProductBarcodeListResult {
    pub items: Vec<ProductBarcode>,
    pub total: i64,
}

pub struct ProductBarcodeCreate {
    pub tenant_id: TenantId,
    pub sku_id: uuid::Uuid,
    pub barcode: String,
    pub level: String,
    pub unit: String,
    pub ratio: i32,
    pub is_default: bool,
    pub status: String,
    pub print_template: Option<String>,
}

pub struct ProductBarcodeUpdate {
    pub barcode: Option<String>,
    pub level: Option<String>,
    pub unit: Option<String>,
    pub ratio: Option<i32>,
    pub is_default: Option<bool>,
    pub status: Option<String>,
    pub print_template: Option<Option<String>>,
}

pub struct ProductBarcodesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ProductBarcodesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_barcodes(
        &self,
        tenant_id: TenantId,
        filters: ProductBarcodeListFilters,
    ) -> anyhow::Result<ProductBarcodeListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            "select count(*) from product_barcodes pb where pb.tenant_id = ",
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                pb.barcode_id,
                pb.sku_id,
                s.sku_code,
                s.name as sku_name,
                pb.barcode,
                pb.level,
                pb.unit,
                pb.ratio,
                pb.is_default,
                pb.status,
                pb.print_template,
                pb.created_at,
                pb.updated_at
            from product_barcodes pb
            join skus s
              on s.sku_id = pb.sku_id
             and s.tenant_id = pb.tenant_id
            where pb.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_filters(&mut qb, &filters);
        qb.push(" order by pb.updated_at desc, pb.barcode_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit.max(1));

        #[derive(sqlx::FromRow)]
        struct Row {
            barcode_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            barcode: String,
            level: String,
            unit: String,
            ratio: i32,
            is_default: bool,
            status: String,
            print_template: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| ProductBarcode {
                barcode_id: row.barcode_id,
                tenant_id,
                sku_id: row.sku_id,
                sku_code: row.sku_code,
                sku_name: row.sku_name,
                barcode: row.barcode,
                level: row.level,
                unit: row.unit,
                ratio: row.ratio,
                is_default: row.is_default,
                status: row.status,
                print_template: row.print_template,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(ProductBarcodeListResult { items, total })
    }

    pub async fn get_barcode(
        &self,
        tenant_id: TenantId,
        barcode_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductBarcode>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            barcode_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            barcode: String,
            level: String,
            unit: String,
            ratio: i32,
            is_default: bool,
            status: String,
            print_template: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                pb.barcode_id,
                pb.sku_id,
                s.sku_code,
                s.name as sku_name,
                pb.barcode,
                pb.level,
                pb.unit,
                pb.ratio,
                pb.is_default,
                pb.status,
                pb.print_template,
                pb.created_at,
                pb.updated_at
            from product_barcodes pb
            join skus s
              on s.sku_id = pb.sku_id
             and s.tenant_id = pb.tenant_id
            where pb.tenant_id = $1 and pb.barcode_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(barcode_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductBarcode {
            barcode_id: row.barcode_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            barcode: row.barcode,
            level: row.level,
            unit: row.unit,
            ratio: row.ratio,
            is_default: row.is_default,
            status: row.status,
            print_template: row.print_template,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_barcode(
        &self,
        input: ProductBarcodeCreate,
    ) -> anyhow::Result<ProductBarcode> {
        #[derive(sqlx::FromRow)]
        struct Row {
            barcode_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            barcode: String,
            level: String,
            unit: String,
            ratio: i32,
            is_default: bool,
            status: String,
            print_template: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into product_barcodes(
                tenant_id, sku_id, barcode, level, unit, ratio, is_default, status, print_template
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            returning
                barcode_id,
                sku_id,
                (select sku_code from skus where sku_id = $2) as sku_code,
                (select name from skus where sku_id = $2) as sku_name,
                barcode,
                level,
                unit,
                ratio,
                is_default,
                status,
                print_template,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.sku_id)
        .bind(&input.barcode)
        .bind(&input.level)
        .bind(&input.unit)
        .bind(input.ratio)
        .bind(input.is_default)
        .bind(&input.status)
        .bind(&input.print_template)
        .fetch_one(self.pool)
        .await?;

        if row.is_default {
            self.clear_default(input.tenant_id, input.sku_id, row.barcode_id)
                .await?;
        }

        Ok(ProductBarcode {
            barcode_id: row.barcode_id,
            tenant_id: input.tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            barcode: row.barcode,
            level: row.level,
            unit: row.unit,
            ratio: row.ratio,
            is_default: row.is_default,
            status: row.status,
            print_template: row.print_template,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_barcode(
        &self,
        tenant_id: TenantId,
        barcode_id: uuid::Uuid,
        input: ProductBarcodeUpdate,
    ) -> anyhow::Result<Option<ProductBarcode>> {
        let mut qb = QueryBuilder::<Postgres>::new("update product_barcodes set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(barcode) = input.barcode {
            separated.push("barcode = ");
            separated.push_bind(barcode);
            has_updates = true;
        }
        if let Some(level) = input.level {
            separated.push("level = ");
            separated.push_bind(level);
            has_updates = true;
        }
        if let Some(unit) = input.unit {
            separated.push("unit = ");
            separated.push_bind(unit);
            has_updates = true;
        }
        if let Some(ratio) = input.ratio {
            separated.push("ratio = ");
            separated.push_bind(ratio);
            has_updates = true;
        }
        if let Some(is_default) = input.is_default {
            separated.push("is_default = ");
            separated.push_bind(is_default);
            has_updates = true;
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(print_template) = input.print_template {
            separated.push("print_template = ");
            separated.push_bind(print_template);
            has_updates = true;
        }

        if !has_updates {
            return self.get_barcode(tenant_id, barcode_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and barcode_id = ");
        qb.push_bind(barcode_id);
        qb.push(
            r#"
            returning
                barcode_id,
                sku_id,
                (select sku_code from skus where sku_id = product_barcodes.sku_id) as sku_code,
                (select name from skus where sku_id = product_barcodes.sku_id) as sku_name,
                barcode,
                level,
                unit,
                ratio,
                is_default,
                status,
                print_template,
                created_at,
                updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            barcode_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            barcode: String,
            level: String,
            unit: String,
            ratio: i32,
            is_default: bool,
            status: String,
            print_template: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        if let Some(row) = row.as_ref() {
            if row.is_default {
                self.clear_default(tenant_id, row.sku_id, row.barcode_id)
                    .await?;
            }
        }

        Ok(row.map(|row| ProductBarcode {
            barcode_id: row.barcode_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            barcode: row.barcode,
            level: row.level,
            unit: row.unit,
            ratio: row.ratio,
            is_default: row.is_default,
            status: row.status,
            print_template: row.print_template,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_barcode(
        &self,
        tenant_id: TenantId,
        barcode_id: uuid::Uuid,
    ) -> anyhow::Result<Option<ProductBarcode>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            barcode_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            sku_code: Option<String>,
            sku_name: Option<String>,
            barcode: String,
            level: String,
            unit: String,
            ratio: i32,
            is_default: bool,
            status: String,
            print_template: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            delete from product_barcodes
            where tenant_id = $1 and barcode_id = $2
            returning
                barcode_id,
                sku_id,
                (select sku_code from skus where sku_id = product_barcodes.sku_id) as sku_code,
                (select name from skus where sku_id = product_barcodes.sku_id) as sku_name,
                barcode,
                level,
                unit,
                ratio,
                is_default,
                status,
                print_template,
                created_at,
                updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(barcode_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| ProductBarcode {
            barcode_id: row.barcode_id,
            tenant_id,
            sku_id: row.sku_id,
            sku_code: row.sku_code,
            sku_name: row.sku_name,
            barcode: row.barcode,
            level: row.level,
            unit: row.unit,
            ratio: row.ratio,
            is_default: row.is_default,
            status: row.status,
            print_template: row.print_template,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    async fn clear_default(
        &self,
        tenant_id: TenantId,
        sku_id: uuid::Uuid,
        barcode_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update product_barcodes
            set is_default = false, updated_at = now()
            where tenant_id = $1 and sku_id = $2 and barcode_id <> $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sku_id)
        .bind(barcode_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }
}

fn apply_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a ProductBarcodeListFilters) {
    if let Some(sku_id) = filters.sku_id {
        qb.push(" and pb.sku_id = ");
        qb.push_bind(sku_id);
    }
    if let Some(barcode) = filters.barcode.as_ref() {
        qb.push(" and pb.barcode = ");
        qb.push_bind(barcode);
    }
    if let Some(status) = filters.status.as_ref() {
        qb.push(" and pb.status = ");
        qb.push_bind(status);
    }
    if let Some(level) = filters.level.as_ref() {
        qb.push(" and pb.level = ");
        qb.push_bind(level);
    }
}
