use anyhow::{anyhow, bail};
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrder {
    pub sales_order_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: Option<uuid::Uuid>,
    pub sales_order_no: String,
    pub source_type: String,
    pub source_order_ids: Vec<String>,
    pub customer_id: Option<uuid::Uuid>,
    pub currency_code: String,
    pub tax_mode: Option<String>,
    pub sales_order_status: String,
    pub delivery_status: String,
    pub invoice_status: String,
    pub payment_status: String,
    pub refund_status: String,
    pub return_status: Option<String>,
    pub stock_status: Option<String>,
    pub total_amount_cent: i64,
    pub discount_amount_cent: i64,
    pub shipping_amount_cent: i64,
    pub receivable_amount_cent: i64,
    pub paid_amount_cent: i64,
    pub refunded_amount_cent: i64,
    pub invoice_amount_cent: i64,
    pub status_flags: Vec<String>,
    pub remark: Option<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<uuid::Uuid>,
    pub updated_by: Option<uuid::Uuid>,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_address: Option<String>,
    pub receiver_province: Option<String>,
    pub receiver_city: Option<String>,
    pub receiver_district: Option<String>,
    pub receiver_postcode: Option<String>,
    pub carrier_code: Option<String>,
    pub logistics_service: Option<String>,
    pub expected_ship_at: Option<DateTime<Utc>>,
    pub wms_outbound_id: Option<uuid::Uuid>,
    pub wms_outbound_status: Option<String>,
    pub wms_wave_id: Option<uuid::Uuid>,
    pub wms_wave_status: Option<String>,
    pub wms_ship_status: Option<String>,
    pub package_count: i32,
    pub waybill_nos: Vec<String>,
    pub review_status: Option<String>,
    pub fulfillment_status: Option<String>,
    pub exception_status: Option<String>,
    pub freeze_reason: Option<String>,
    pub exception_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderItem {
    pub line_id: uuid::Uuid,
    pub sales_order_id: uuid::Uuid,
    pub sku_id: Option<uuid::Uuid>,
    pub sku_code: Option<String>,
    pub sku_name: Option<String>,
    pub qty: i32,
    pub price_cent: i64,
    pub tax_rate: Option<f64>,
    pub discount_cent: i64,
    pub amount_cent: i64,
    pub warehouse_id: Option<uuid::Uuid>,
    pub delivery_deadline: Option<DateTime<Utc>>,
    pub extra: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderDetail {
    pub order: SalesOrder,
    pub items: Vec<SalesOrderItem>,
}

#[derive(Debug, Clone)]
pub struct SalesOrderItemInput {
    pub line_id: Option<uuid::Uuid>,
    pub sku_id: Option<uuid::Uuid>,
    pub sku_code: Option<String>,
    pub sku_name: Option<String>,
    pub qty: i32,
    pub price_cent: i64,
    pub tax_rate: Option<f64>,
    pub discount_cent: i64,
    pub amount_cent: i64,
    pub warehouse_id: Option<uuid::Uuid>,
    pub delivery_deadline: Option<DateTime<Utc>>,
    pub extra: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct SalesOrderCreate {
    pub sales_order_id: Option<uuid::Uuid>,
    pub tenant_id: TenantId,
    pub shop_id: Option<uuid::Uuid>,
    pub sales_order_no: String,
    pub source_type: String,
    pub source_order_ids: Vec<String>,
    pub customer_id: Option<uuid::Uuid>,
    pub currency_code: String,
    pub tax_mode: Option<String>,
    pub sales_order_status: String,
    pub delivery_status: String,
    pub invoice_status: String,
    pub payment_status: String,
    pub refund_status: String,
    pub return_status: Option<String>,
    pub stock_status: Option<String>,
    pub total_amount_cent: i64,
    pub discount_amount_cent: i64,
    pub shipping_amount_cent: i64,
    pub receivable_amount_cent: i64,
    pub paid_amount_cent: i64,
    pub refunded_amount_cent: i64,
    pub invoice_amount_cent: i64,
    pub status_flags: Vec<String>,
    pub remark: Option<String>,
    pub version: Option<i32>,
    pub created_by: Option<uuid::Uuid>,
    pub updated_by: Option<uuid::Uuid>,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_address: Option<String>,
    pub receiver_province: Option<String>,
    pub receiver_city: Option<String>,
    pub receiver_district: Option<String>,
    pub receiver_postcode: Option<String>,
    pub carrier_code: Option<String>,
    pub logistics_service: Option<String>,
    pub expected_ship_at: Option<DateTime<Utc>>,
    pub wms_outbound_id: Option<uuid::Uuid>,
    pub wms_outbound_status: Option<String>,
    pub wms_wave_id: Option<uuid::Uuid>,
    pub wms_wave_status: Option<String>,
    pub wms_ship_status: Option<String>,
    pub package_count: Option<i32>,
    pub waybill_nos: Vec<String>,
    pub review_status: Option<String>,
    pub fulfillment_status: Option<String>,
    pub exception_status: Option<String>,
    pub freeze_reason: Option<String>,
    pub exception_flags: Vec<String>,
    pub items: Vec<SalesOrderItemInput>,
}

#[derive(Debug, Clone)]
pub struct SalesOrderUpdate {
    pub shop_id: Option<uuid::Uuid>,
    pub sales_order_no: Option<String>,
    pub source_type: Option<String>,
    pub source_order_ids: Option<Vec<String>>,
    pub customer_id: Option<uuid::Uuid>,
    pub currency_code: Option<String>,
    pub tax_mode: Option<String>,
    pub sales_order_status: Option<String>,
    pub delivery_status: Option<String>,
    pub invoice_status: Option<String>,
    pub payment_status: Option<String>,
    pub refund_status: Option<String>,
    pub return_status: Option<String>,
    pub stock_status: Option<String>,
    pub total_amount_cent: Option<i64>,
    pub discount_amount_cent: Option<i64>,
    pub shipping_amount_cent: Option<i64>,
    pub receivable_amount_cent: Option<i64>,
    pub paid_amount_cent: Option<i64>,
    pub refunded_amount_cent: Option<i64>,
    pub invoice_amount_cent: Option<i64>,
    pub status_flags: Option<Vec<String>>,
    pub remark: Option<String>,
    pub created_by: Option<uuid::Uuid>,
    pub updated_by: Option<uuid::Uuid>,
    pub receiver_name: Option<String>,
    pub receiver_phone: Option<String>,
    pub receiver_address: Option<String>,
    pub receiver_province: Option<String>,
    pub receiver_city: Option<String>,
    pub receiver_district: Option<String>,
    pub receiver_postcode: Option<String>,
    pub carrier_code: Option<String>,
    pub logistics_service: Option<String>,
    pub expected_ship_at: Option<DateTime<Utc>>,
    pub wms_outbound_id: Option<uuid::Uuid>,
    pub wms_outbound_status: Option<String>,
    pub wms_wave_id: Option<uuid::Uuid>,
    pub wms_wave_status: Option<String>,
    pub wms_ship_status: Option<String>,
    pub package_count: Option<i32>,
    pub waybill_nos: Option<Vec<String>>,
    pub review_status: Option<String>,
    pub fulfillment_status: Option<String>,
    pub exception_status: Option<String>,
    pub freeze_reason: Option<String>,
    pub exception_flags: Option<Vec<String>>,
    pub items: Option<Vec<SalesOrderItemInput>>,
}

#[derive(Debug, Clone)]
pub struct SalesOrderStatusUpdate {
    pub sales_order_status: Option<String>,
    pub delivery_status: Option<String>,
    pub invoice_status: Option<String>,
    pub payment_status: Option<String>,
    pub refund_status: Option<String>,
    pub return_status: Option<String>,
    pub stock_status: Option<String>,
    pub status_flags: Option<Vec<String>>,
    pub review_status: Option<String>,
    pub fulfillment_status: Option<String>,
    pub exception_status: Option<String>,
    pub freeze_reason: Option<String>,
    pub exception_flags: Option<Vec<String>>,
    pub updated_by: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct SalesOrderWmsUpdate {
    pub wms_outbound_status: Option<String>,
    pub wms_wave_id: Option<uuid::Uuid>,
    pub wms_wave_status: Option<String>,
    pub wms_ship_status: Option<String>,
    pub delivery_status: Option<String>,
    pub package_count: Option<i32>,
    pub waybill_nos: Option<Vec<String>>,
    pub carrier_code: Option<String>,
    pub logistics_service: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SalesOrderAfterSalesUpdate {
    pub refund_status: Option<String>,
    pub return_status: Option<String>,
    pub status_flags: Option<Vec<String>>,
}

pub struct SalesOrderListFilters {
    pub shop_id: Option<uuid::Uuid>,
    pub sales_order_status: Option<String>,
    pub delivery_status: Option<String>,
    pub payment_status: Option<String>,
    pub refund_status: Option<String>,
    pub customer_id: Option<uuid::Uuid>,
    pub sales_order_no: Option<String>,
    pub keyword: Option<String>,
    pub source_order_id: Option<String>,
    pub wms_outbound_id: Option<uuid::Uuid>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub updated_from: Option<DateTime<Utc>>,
    pub updated_to: Option<DateTime<Utc>>,
    pub offset: i64,
    pub limit: i64,
}

pub struct SalesOrderListResult {
    pub items: Vec<SalesOrder>,
    pub total: i64,
}

pub struct SalesOrdersRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> SalesOrdersRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_sales_orders(
        &self,
        tenant_id: TenantId,
        filters: SalesOrderListFilters,
    ) -> anyhow::Result<SalesOrderListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from sales_orders
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_sales_order_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                sales_order_id,
                shop_id,
                sales_order_no,
                source_type,
                source_order_ids,
                customer_id,
                currency_code,
                tax_mode,
                sales_order_status,
                delivery_status,
                invoice_status,
                payment_status,
                refund_status,
                return_status,
                stock_status,
                total_amount_cent,
                discount_amount_cent,
                shipping_amount_cent,
                receivable_amount_cent,
                paid_amount_cent,
                refunded_amount_cent,
                invoice_amount_cent,
                status_flags,
                remark,
                version,
                created_at,
                updated_at,
                created_by,
                updated_by,
                receiver_name,
                receiver_phone,
                receiver_address,
                receiver_province,
                receiver_city,
                receiver_district,
                receiver_postcode,
                carrier_code,
                logistics_service,
                expected_ship_at,
                wms_outbound_id,
                wms_outbound_status,
                wms_wave_id,
                wms_wave_status,
                wms_ship_status,
                package_count,
                waybill_nos,
                review_status,
                fulfillment_status,
                exception_status,
                freeze_reason,
                exception_flags
            from sales_orders
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_sales_order_filters(&mut qb, &filters);
        qb.push(" order by updated_at desc, sales_order_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset.max(0));
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<SalesOrderRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_order(tenant_id))
            .collect();

        Ok(SalesOrderListResult { items, total })
    }

    pub async fn get_sales_order(
        &self,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SalesOrder>> {
        let row = self
            .fetch_sales_order_row(tenant_id, sales_order_id)
            .await?;
        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    pub async fn get_sales_order_detail(
        &self,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SalesOrderDetail>> {
        let row = self
            .fetch_sales_order_row(tenant_id, sales_order_id)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let items = self
            .list_sales_order_items(tenant_id, sales_order_id)
            .await?;
        Ok(Some(SalesOrderDetail {
            order: row.into_order(tenant_id),
            items,
        }))
    }

    pub async fn list_sales_order_items(
        &self,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<SalesOrderItem>> {
        let rows: Vec<SalesOrderItemRow> = sqlx::query_as(
            r#"
            select
                line_id,
                sales_order_id,
                sku_id,
                sku_code,
                sku_name,
                qty,
                price_cent,
                tax_rate,
                discount_cent,
                amount_cent,
                warehouse_id,
                delivery_deadline,
                extra,
                created_at,
                updated_at
            from sales_order_items
            where tenant_id = $1 and sales_order_id = $2
            order by line_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sales_order_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(SalesOrderItemRow::into_item).collect())
    }

    pub async fn create_sales_order(
        &self,
        input: SalesOrderCreate,
    ) -> anyhow::Result<SalesOrderDetail> {
        if input.items.is_empty() {
            bail!("销售单明细不能为空");
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let sales_order_id = input.sales_order_id.unwrap_or_else(uuid::Uuid::new_v4);
        let version = input.version.unwrap_or(1);
        let package_count = input.package_count.unwrap_or(0);
        let source_order_ids = serde_json::to_value(&input.source_order_ids)
            .map_err(|_| anyhow!("来源订单字段无效"))?;
        let status_flags =
            serde_json::to_value(&input.status_flags).map_err(|_| anyhow!("状态标记无效"))?;
        let waybill_nos =
            serde_json::to_value(&input.waybill_nos).map_err(|_| anyhow!("运单号无效"))?;
        let exception_flags =
            serde_json::to_value(&input.exception_flags).map_err(|_| anyhow!("异常标记无效"))?;

        let row: SalesOrderRow = sqlx::query_as(
            r#"
            insert into sales_orders(
                sales_order_id,
                tenant_id,
                shop_id,
                sales_order_no,
                source_type,
                source_order_ids,
                customer_id,
                currency_code,
                tax_mode,
                sales_order_status,
                delivery_status,
                invoice_status,
                payment_status,
                refund_status,
                return_status,
                stock_status,
                total_amount_cent,
                discount_amount_cent,
                shipping_amount_cent,
                receivable_amount_cent,
                paid_amount_cent,
                refunded_amount_cent,
                invoice_amount_cent,
                status_flags,
                remark,
                version,
                created_by,
                updated_by,
                receiver_name,
                receiver_phone,
                receiver_address,
                receiver_province,
                receiver_city,
                receiver_district,
                receiver_postcode,
                carrier_code,
                logistics_service,
                expected_ship_at,
                wms_outbound_id,
                wms_outbound_status,
                wms_wave_id,
                wms_wave_status,
                wms_ship_status,
                package_count,
                waybill_nos,
                review_status,
                fulfillment_status,
                exception_status,
                freeze_reason,
                exception_flags
            ) values (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,
                $23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35,$36,$37,$38,$39,$40,$41,$42,
                $43,$44,$45,$46,$47
            )
            returning
                sales_order_id,
                shop_id,
                sales_order_no,
                source_type,
                source_order_ids,
                customer_id,
                currency_code,
                tax_mode,
                sales_order_status,
                delivery_status,
                invoice_status,
                payment_status,
                refund_status,
                return_status,
                stock_status,
                total_amount_cent,
                discount_amount_cent,
                shipping_amount_cent,
                receivable_amount_cent,
                paid_amount_cent,
                refunded_amount_cent,
                invoice_amount_cent,
                status_flags,
                remark,
                version,
                created_at,
                updated_at,
                created_by,
                updated_by,
                receiver_name,
                receiver_phone,
                receiver_address,
                receiver_province,
                receiver_city,
                receiver_district,
                receiver_postcode,
                carrier_code,
                logistics_service,
                expected_ship_at,
                wms_outbound_id,
                wms_outbound_status,
                wms_wave_id,
                wms_wave_status,
                wms_ship_status,
                package_count,
                waybill_nos,
                review_status,
                fulfillment_status,
                exception_status,
                freeze_reason,
                exception_flags
            "#,
        )
        .bind(sales_order_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.shop_id)
        .bind(&input.sales_order_no)
        .bind(&input.source_type)
        .bind(source_order_ids)
        .bind(input.customer_id)
        .bind(&input.currency_code)
        .bind(&input.tax_mode)
        .bind(&input.sales_order_status)
        .bind(&input.delivery_status)
        .bind(&input.invoice_status)
        .bind(&input.payment_status)
        .bind(&input.refund_status)
        .bind(&input.return_status)
        .bind(&input.stock_status)
        .bind(input.total_amount_cent)
        .bind(input.discount_amount_cent)
        .bind(input.shipping_amount_cent)
        .bind(input.receivable_amount_cent)
        .bind(input.paid_amount_cent)
        .bind(input.refunded_amount_cent)
        .bind(input.invoice_amount_cent)
        .bind(status_flags)
        .bind(&input.remark)
        .bind(version)
        .bind(input.created_by)
        .bind(input.updated_by)
        .bind(&input.receiver_name)
        .bind(&input.receiver_phone)
        .bind(&input.receiver_address)
        .bind(&input.receiver_province)
        .bind(&input.receiver_city)
        .bind(&input.receiver_district)
        .bind(&input.receiver_postcode)
        .bind(&input.carrier_code)
        .bind(&input.logistics_service)
        .bind(input.expected_ship_at)
        .bind(input.wms_outbound_id)
        .bind(&input.wms_outbound_status)
        .bind(input.wms_wave_id)
        .bind(&input.wms_wave_status)
        .bind(&input.wms_ship_status)
        .bind(package_count)
        .bind(waybill_nos)
        .bind(&input.review_status)
        .bind(&input.fulfillment_status)
        .bind(&input.exception_status)
        .bind(&input.freeze_reason)
        .bind(exception_flags)
        .fetch_one(&mut *tx)
        .await?;

        let mut items = Vec::with_capacity(input.items.len());
        for item in input.items {
            let line_id = item.line_id.unwrap_or_else(uuid::Uuid::new_v4);
            let extra = item.extra.clone().unwrap_or_else(|| serde_json::json!({}));
            items.push((line_id, extra, item));
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into sales_order_items(
                tenant_id,
                sales_order_id,
                line_id,
                sku_id,
                sku_code,
                sku_name,
                qty,
                price_cent,
                tax_rate,
                discount_cent,
                amount_cent,
                warehouse_id,
                delivery_deadline,
                extra
            )
            "#,
        );
        qb.push_values(items.iter(), |mut b, (line_id, extra, item)| {
            b.push_bind(uuid::Uuid::from(input.tenant_id))
                .push_bind(sales_order_id)
                .push_bind(line_id)
                .push_bind(item.sku_id)
                .push_bind(&item.sku_code)
                .push_bind(&item.sku_name)
                .push_bind(item.qty)
                .push_bind(item.price_cent)
                .push_bind(item.tax_rate)
                .push_bind(item.discount_cent)
                .push_bind(item.amount_cent)
                .push_bind(item.warehouse_id)
                .push_bind(item.delivery_deadline)
                .push_bind(extra);
        });
        qb.build().execute(&mut *tx).await?;

        tx.commit().await?;

        let order = row.into_order(input.tenant_id);
        let items = self
            .list_sales_order_items(input.tenant_id, order.sales_order_id)
            .await?;
        Ok(SalesOrderDetail { order, items })
    }

    pub async fn update_sales_order(
        &self,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
        input: SalesOrderUpdate,
    ) -> anyhow::Result<Option<SalesOrderDetail>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let mut qb = QueryBuilder::<Postgres>::new("update sales_orders set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(shop_id) = input.shop_id {
            separated.push("shop_id = ");
            separated.push_bind(shop_id);
            has_updates = true;
        }
        if let Some(sales_order_no) = input.sales_order_no {
            separated.push("sales_order_no = ");
            separated.push_bind(sales_order_no);
            has_updates = true;
        }
        if let Some(source_type) = input.source_type {
            separated.push("source_type = ");
            separated.push_bind(source_type);
            has_updates = true;
        }
        if let Some(source_order_ids) = input.source_order_ids {
            let value =
                serde_json::to_value(&source_order_ids).map_err(|_| anyhow!("来源订单无效"))?;
            separated.push("source_order_ids = ");
            separated.push_bind(value);
            has_updates = true;
        }
        if let Some(customer_id) = input.customer_id {
            separated.push("customer_id = ");
            separated.push_bind(customer_id);
            has_updates = true;
        }
        if let Some(currency_code) = input.currency_code {
            separated.push("currency_code = ");
            separated.push_bind(currency_code);
            has_updates = true;
        }
        if let Some(tax_mode) = input.tax_mode {
            separated.push("tax_mode = ");
            separated.push_bind(tax_mode);
            has_updates = true;
        }
        if let Some(sales_order_status) = input.sales_order_status {
            separated.push("sales_order_status = ");
            separated.push_bind(sales_order_status);
            has_updates = true;
        }
        if let Some(delivery_status) = input.delivery_status {
            separated.push("delivery_status = ");
            separated.push_bind(delivery_status);
            has_updates = true;
        }
        if let Some(invoice_status) = input.invoice_status {
            separated.push("invoice_status = ");
            separated.push_bind(invoice_status);
            has_updates = true;
        }
        if let Some(payment_status) = input.payment_status {
            separated.push("payment_status = ");
            separated.push_bind(payment_status);
            has_updates = true;
        }
        if let Some(refund_status) = input.refund_status {
            separated.push("refund_status = ");
            separated.push_bind(refund_status);
            has_updates = true;
        }
        if let Some(return_status) = input.return_status {
            separated.push("return_status = ");
            separated.push_bind(return_status);
            has_updates = true;
        }
        if let Some(stock_status) = input.stock_status {
            separated.push("stock_status = ");
            separated.push_bind(stock_status);
            has_updates = true;
        }
        if let Some(total_amount_cent) = input.total_amount_cent {
            separated.push("total_amount_cent = ");
            separated.push_bind(total_amount_cent);
            has_updates = true;
        }
        if let Some(discount_amount_cent) = input.discount_amount_cent {
            separated.push("discount_amount_cent = ");
            separated.push_bind(discount_amount_cent);
            has_updates = true;
        }
        if let Some(shipping_amount_cent) = input.shipping_amount_cent {
            separated.push("shipping_amount_cent = ");
            separated.push_bind(shipping_amount_cent);
            has_updates = true;
        }
        if let Some(receivable_amount_cent) = input.receivable_amount_cent {
            separated.push("receivable_amount_cent = ");
            separated.push_bind(receivable_amount_cent);
            has_updates = true;
        }
        if let Some(paid_amount_cent) = input.paid_amount_cent {
            separated.push("paid_amount_cent = ");
            separated.push_bind(paid_amount_cent);
            has_updates = true;
        }
        if let Some(refunded_amount_cent) = input.refunded_amount_cent {
            separated.push("refunded_amount_cent = ");
            separated.push_bind(refunded_amount_cent);
            has_updates = true;
        }
        if let Some(invoice_amount_cent) = input.invoice_amount_cent {
            separated.push("invoice_amount_cent = ");
            separated.push_bind(invoice_amount_cent);
            has_updates = true;
        }
        if let Some(status_flags) = input.status_flags {
            let value = serde_json::to_value(&status_flags).map_err(|_| anyhow!("状态标记无效"))?;
            separated.push("status_flags = ");
            separated.push_bind(value);
            has_updates = true;
        }
        if let Some(remark) = input.remark {
            separated.push("remark = ");
            separated.push_bind(remark);
            has_updates = true;
        }
        if let Some(created_by) = input.created_by {
            separated.push("created_by = ");
            separated.push_bind(created_by);
            has_updates = true;
        }
        if let Some(updated_by) = input.updated_by {
            separated.push("updated_by = ");
            separated.push_bind(updated_by);
            has_updates = true;
        }
        if let Some(receiver_name) = input.receiver_name {
            separated.push("receiver_name = ");
            separated.push_bind(receiver_name);
            has_updates = true;
        }
        if let Some(receiver_phone) = input.receiver_phone {
            separated.push("receiver_phone = ");
            separated.push_bind(receiver_phone);
            has_updates = true;
        }
        if let Some(receiver_address) = input.receiver_address {
            separated.push("receiver_address = ");
            separated.push_bind(receiver_address);
            has_updates = true;
        }
        if let Some(receiver_province) = input.receiver_province {
            separated.push("receiver_province = ");
            separated.push_bind(receiver_province);
            has_updates = true;
        }
        if let Some(receiver_city) = input.receiver_city {
            separated.push("receiver_city = ");
            separated.push_bind(receiver_city);
            has_updates = true;
        }
        if let Some(receiver_district) = input.receiver_district {
            separated.push("receiver_district = ");
            separated.push_bind(receiver_district);
            has_updates = true;
        }
        if let Some(receiver_postcode) = input.receiver_postcode {
            separated.push("receiver_postcode = ");
            separated.push_bind(receiver_postcode);
            has_updates = true;
        }
        if let Some(carrier_code) = input.carrier_code {
            separated.push("carrier_code = ");
            separated.push_bind(carrier_code);
            has_updates = true;
        }
        if let Some(logistics_service) = input.logistics_service {
            separated.push("logistics_service = ");
            separated.push_bind(logistics_service);
            has_updates = true;
        }
        if let Some(expected_ship_at) = input.expected_ship_at {
            separated.push("expected_ship_at = ");
            separated.push_bind(expected_ship_at);
            has_updates = true;
        }
        if let Some(wms_outbound_id) = input.wms_outbound_id {
            separated.push("wms_outbound_id = ");
            separated.push_bind(wms_outbound_id);
            has_updates = true;
        }
        if let Some(wms_outbound_status) = input.wms_outbound_status {
            separated.push("wms_outbound_status = ");
            separated.push_bind(wms_outbound_status);
            has_updates = true;
        }
        if let Some(wms_wave_id) = input.wms_wave_id {
            separated.push("wms_wave_id = ");
            separated.push_bind(wms_wave_id);
            has_updates = true;
        }
        if let Some(wms_wave_status) = input.wms_wave_status {
            separated.push("wms_wave_status = ");
            separated.push_bind(wms_wave_status);
            has_updates = true;
        }
        if let Some(wms_ship_status) = input.wms_ship_status {
            separated.push("wms_ship_status = ");
            separated.push_bind(wms_ship_status);
            has_updates = true;
        }
        if let Some(package_count) = input.package_count {
            separated.push("package_count = ");
            separated.push_bind(package_count);
            has_updates = true;
        }
        if let Some(waybill_nos) = input.waybill_nos {
            let value = serde_json::to_value(&waybill_nos).map_err(|_| anyhow!("运单号无效"))?;
            separated.push("waybill_nos = ");
            separated.push_bind(value);
            has_updates = true;
        }
        if let Some(review_status) = input.review_status {
            separated.push("review_status = ");
            separated.push_bind(review_status);
            has_updates = true;
        }
        if let Some(fulfillment_status) = input.fulfillment_status {
            separated.push("fulfillment_status = ");
            separated.push_bind(fulfillment_status);
            has_updates = true;
        }
        if let Some(exception_status) = input.exception_status {
            separated.push("exception_status = ");
            separated.push_bind(exception_status);
            has_updates = true;
        }
        if let Some(freeze_reason) = input.freeze_reason {
            separated.push("freeze_reason = ");
            separated.push_bind(freeze_reason);
            has_updates = true;
        }
        if let Some(exception_flags) = input.exception_flags {
            let value =
                serde_json::to_value(&exception_flags).map_err(|_| anyhow!("异常标记无效"))?;
            separated.push("exception_flags = ");
            separated.push_bind(value);
            has_updates = true;
        }

        let order_row = if has_updates {
            separated.push("version = version + 1");
            separated.push("updated_at = now()");
            qb.push(" where tenant_id = ");
            qb.push_bind(uuid::Uuid::from(tenant_id));
            qb.push(" and sales_order_id = ");
            qb.push_bind(sales_order_id);
            qb.push(
                " returning sales_order_id, shop_id, sales_order_no, source_type, source_order_ids, customer_id, currency_code, tax_mode, sales_order_status, delivery_status, invoice_status, payment_status, refund_status, return_status, stock_status, total_amount_cent, discount_amount_cent, shipping_amount_cent, receivable_amount_cent, paid_amount_cent, refunded_amount_cent, invoice_amount_cent, status_flags, remark, version, created_at, updated_at, created_by, updated_by, receiver_name, receiver_phone, receiver_address, receiver_province, receiver_city, receiver_district, receiver_postcode, carrier_code, logistics_service, expected_ship_at, wms_outbound_id, wms_outbound_status, wms_wave_id, wms_wave_status, wms_ship_status, package_count, waybill_nos, review_status, fulfillment_status, exception_status, freeze_reason, exception_flags",
            );
            qb.build_query_as().fetch_optional(&mut *tx).await?
        } else {
            self.fetch_sales_order_row_with_tx(&mut tx, tenant_id, sales_order_id)
                .await?
        };

        let Some(order_row) = order_row else {
            return Ok(None);
        };

        if let Some(items) = input.items {
            sqlx::query(
                r#"
                delete from sales_order_items
                where tenant_id = $1 and sales_order_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(sales_order_id)
            .execute(&mut *tx)
            .await?;

            if !items.is_empty() {
                let mut qb = QueryBuilder::<Postgres>::new(
                    r#"
                    insert into sales_order_items(
                        tenant_id,
                        sales_order_id,
                        line_id,
                        sku_id,
                        sku_code,
                        sku_name,
                        qty,
                        price_cent,
                        tax_rate,
                        discount_cent,
                        amount_cent,
                        warehouse_id,
                        delivery_deadline,
                        extra
                    )
                    "#,
                );
                qb.push_values(items.iter(), |mut b, item| {
                    let line_id = item.line_id.unwrap_or_else(uuid::Uuid::new_v4);
                    let extra = item.extra.clone().unwrap_or_else(|| serde_json::json!({}));
                    b.push_bind(uuid::Uuid::from(tenant_id))
                        .push_bind(sales_order_id)
                        .push_bind(line_id)
                        .push_bind(item.sku_id)
                        .push_bind(&item.sku_code)
                        .push_bind(&item.sku_name)
                        .push_bind(item.qty)
                        .push_bind(item.price_cent)
                        .push_bind(item.tax_rate)
                        .push_bind(item.discount_cent)
                        .push_bind(item.amount_cent)
                        .push_bind(item.warehouse_id)
                        .push_bind(item.delivery_deadline)
                        .push_bind(extra);
                });
                qb.build().execute(&mut *tx).await?;
            } else if !has_updates {
                sqlx::query(
                    r#"
                    update sales_orders
                    set version = version + 1,
                        updated_at = now()
                    where tenant_id = $1 and sales_order_id = $2
                    "#,
                )
                .bind(uuid::Uuid::from(tenant_id))
                .bind(sales_order_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        let items = self
            .list_sales_order_items(tenant_id, sales_order_id)
            .await?;
        Ok(Some(SalesOrderDetail {
            order: order_row.into_order(tenant_id),
            items,
        }))
    }

    pub async fn update_sales_order_status(
        &self,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
        input: SalesOrderStatusUpdate,
    ) -> anyhow::Result<Option<SalesOrder>> {
        let mut qb = QueryBuilder::<Postgres>::new("update sales_orders set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(status) = input.sales_order_status {
            separated.push("sales_order_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.delivery_status {
            separated.push("delivery_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.invoice_status {
            separated.push("invoice_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.payment_status {
            separated.push("payment_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.refund_status {
            separated.push("refund_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.return_status {
            separated.push("return_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.stock_status {
            separated.push("stock_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(flags) = input.status_flags {
            let value = serde_json::to_value(&flags).map_err(|_| anyhow!("状态标记无效"))?;
            separated.push("status_flags = ");
            separated.push_bind(value);
            has_updates = true;
        }
        if let Some(status) = input.review_status {
            separated.push("review_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.fulfillment_status {
            separated.push("fulfillment_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.exception_status {
            separated.push("exception_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(reason) = input.freeze_reason {
            separated.push("freeze_reason = ");
            separated.push_bind(reason);
            has_updates = true;
        }
        if let Some(flags) = input.exception_flags {
            let value = serde_json::to_value(&flags).map_err(|_| anyhow!("异常标记无效"))?;
            separated.push("exception_flags = ");
            separated.push_bind(value);
            has_updates = true;
        }
        if let Some(updated_by) = input.updated_by {
            separated.push("updated_by = ");
            separated.push_bind(updated_by);
            has_updates = true;
        }

        if !has_updates {
            return self.get_sales_order(tenant_id, sales_order_id).await;
        }

        separated.push("version = version + 1");
        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and sales_order_id = ");
        qb.push_bind(sales_order_id);
        qb.push(
            " returning sales_order_id, shop_id, sales_order_no, source_type, source_order_ids, customer_id, currency_code, tax_mode, sales_order_status, delivery_status, invoice_status, payment_status, refund_status, return_status, stock_status, total_amount_cent, discount_amount_cent, shipping_amount_cent, receivable_amount_cent, paid_amount_cent, refunded_amount_cent, invoice_amount_cent, status_flags, remark, version, created_at, updated_at, created_by, updated_by, receiver_name, receiver_phone, receiver_address, receiver_province, receiver_city, receiver_district, receiver_postcode, carrier_code, logistics_service, expected_ship_at, wms_outbound_id, wms_outbound_status, wms_wave_id, wms_wave_status, wms_ship_status, package_count, waybill_nos, review_status, fulfillment_status, exception_status, freeze_reason, exception_flags",
        );

        let row: Option<SalesOrderRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    pub async fn bind_wms_outbound(
        &self,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
        outbound_doc_id: uuid::Uuid,
        outbound_status: Option<String>,
        ship_status: Option<String>,
        delivery_status: Option<String>,
    ) -> anyhow::Result<Option<SalesOrder>> {
        let mut qb = QueryBuilder::<Postgres>::new("update sales_orders set wms_outbound_id = ");
        qb.push_bind(outbound_doc_id);
        if let Some(status) = outbound_status {
            qb.push(", wms_outbound_status = ");
            qb.push_bind(status);
        }
        if let Some(status) = ship_status {
            qb.push(", wms_ship_status = ");
            qb.push_bind(status);
        }
        if let Some(status) = delivery_status {
            qb.push(", delivery_status = ");
            qb.push_bind(status);
        }
        qb.push(", version = version + 1, updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and sales_order_id = ");
        qb.push_bind(sales_order_id);
        qb.push(
            " returning sales_order_id, shop_id, sales_order_no, source_type, source_order_ids, customer_id, currency_code, tax_mode, sales_order_status, delivery_status, invoice_status, payment_status, refund_status, return_status, stock_status, total_amount_cent, discount_amount_cent, shipping_amount_cent, receivable_amount_cent, paid_amount_cent, refunded_amount_cent, invoice_amount_cent, status_flags, remark, version, created_at, updated_at, created_by, updated_by, receiver_name, receiver_phone, receiver_address, receiver_province, receiver_city, receiver_district, receiver_postcode, carrier_code, logistics_service, expected_ship_at, wms_outbound_id, wms_outbound_status, wms_wave_id, wms_wave_status, wms_ship_status, package_count, waybill_nos, review_status, fulfillment_status, exception_status, freeze_reason, exception_flags",
        );

        let row: Option<SalesOrderRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    pub async fn update_wms_status_by_outbound(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
        input: SalesOrderWmsUpdate,
    ) -> anyhow::Result<Option<SalesOrder>> {
        let mut qb = QueryBuilder::<Postgres>::new("update sales_orders set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(status) = input.wms_outbound_status {
            separated.push("wms_outbound_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(wave_id) = input.wms_wave_id {
            separated.push("wms_wave_id = ");
            separated.push_bind(wave_id);
            has_updates = true;
        }
        if let Some(status) = input.wms_wave_status {
            separated.push("wms_wave_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.wms_ship_status {
            separated.push("wms_ship_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.delivery_status {
            separated.push("delivery_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(package_count) = input.package_count {
            separated.push("package_count = ");
            separated.push_bind(package_count);
            has_updates = true;
        }
        if let Some(waybill_nos) = input.waybill_nos {
            let value = serde_json::to_value(&waybill_nos).map_err(|_| anyhow!("运单号无效"))?;
            separated.push("waybill_nos = ");
            separated.push_bind(value);
            has_updates = true;
        }
        if let Some(carrier_code) = input.carrier_code {
            separated.push("carrier_code = ");
            separated.push_bind(carrier_code);
            has_updates = true;
        }
        if let Some(logistics_service) = input.logistics_service {
            separated.push("logistics_service = ");
            separated.push_bind(logistics_service);
            has_updates = true;
        }

        if !has_updates {
            return self
                .get_sales_order_by_outbound(tenant_id, outbound_doc_id)
                .await;
        }

        separated.push("version = version + 1");
        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and wms_outbound_id = ");
        qb.push_bind(outbound_doc_id);
        qb.push(
            " returning sales_order_id, shop_id, sales_order_no, source_type, source_order_ids, customer_id, currency_code, tax_mode, sales_order_status, delivery_status, invoice_status, payment_status, refund_status, return_status, stock_status, total_amount_cent, discount_amount_cent, shipping_amount_cent, receivable_amount_cent, paid_amount_cent, refunded_amount_cent, invoice_amount_cent, status_flags, remark, version, created_at, updated_at, created_by, updated_by, receiver_name, receiver_phone, receiver_address, receiver_province, receiver_city, receiver_district, receiver_postcode, carrier_code, logistics_service, expected_ship_at, wms_outbound_id, wms_outbound_status, wms_wave_id, wms_wave_status, wms_ship_status, package_count, waybill_nos, review_status, fulfillment_status, exception_status, freeze_reason, exception_flags",
        );

        let row: Option<SalesOrderRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    pub async fn update_after_sales_status(
        &self,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
        input: SalesOrderAfterSalesUpdate,
    ) -> anyhow::Result<Option<SalesOrder>> {
        let mut qb = QueryBuilder::<Postgres>::new("update sales_orders set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(status) = input.refund_status {
            separated.push("refund_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(status) = input.return_status {
            separated.push("return_status = ");
            separated.push_bind(status);
            has_updates = true;
        }
        if let Some(flags) = input.status_flags {
            let value = serde_json::to_value(&flags).map_err(|_| anyhow!("状态标记无效"))?;
            separated.push("status_flags = ");
            separated.push_bind(value);
            has_updates = true;
        }

        if !has_updates {
            return self.get_sales_order(tenant_id, sales_order_id).await;
        }

        separated.push("version = version + 1");
        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and sales_order_id = ");
        qb.push_bind(sales_order_id);
        qb.push(
            " returning sales_order_id, shop_id, sales_order_no, source_type, source_order_ids, customer_id, currency_code, tax_mode, sales_order_status, delivery_status, invoice_status, payment_status, refund_status, return_status, stock_status, total_amount_cent, discount_amount_cent, shipping_amount_cent, receivable_amount_cent, paid_amount_cent, refunded_amount_cent, invoice_amount_cent, status_flags, remark, version, created_at, updated_at, created_by, updated_by, receiver_name, receiver_phone, receiver_address, receiver_province, receiver_city, receiver_district, receiver_postcode, carrier_code, logistics_service, expected_ship_at, wms_outbound_id, wms_outbound_status, wms_wave_id, wms_wave_status, wms_ship_status, package_count, waybill_nos, review_status, fulfillment_status, exception_status, freeze_reason, exception_flags",
        );

        let row: Option<SalesOrderRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    pub async fn get_sales_order_by_outbound(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SalesOrder>> {
        let row: Option<SalesOrderRow> = sqlx::query_as(
            r#"
            select
                sales_order_id,
                shop_id,
                sales_order_no,
                source_type,
                source_order_ids,
                customer_id,
                currency_code,
                tax_mode,
                sales_order_status,
                delivery_status,
                invoice_status,
                payment_status,
                refund_status,
                return_status,
                stock_status,
                total_amount_cent,
                discount_amount_cent,
                shipping_amount_cent,
                receivable_amount_cent,
                paid_amount_cent,
                refunded_amount_cent,
                invoice_amount_cent,
                status_flags,
                remark,
                version,
                created_at,
                updated_at,
                created_by,
                updated_by,
                receiver_name,
                receiver_phone,
                receiver_address,
                receiver_province,
                receiver_city,
                receiver_district,
                receiver_postcode,
                carrier_code,
                logistics_service,
                expected_ship_at,
                wms_outbound_id,
                wms_outbound_status,
                wms_wave_id,
                wms_wave_status,
                wms_ship_status,
                package_count,
                waybill_nos,
                review_status,
                fulfillment_status,
                exception_status,
                freeze_reason,
                exception_flags
            from sales_orders
            where tenant_id = $1 and wms_outbound_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    pub async fn find_by_source_order(
        &self,
        tenant_id: TenantId,
        source_order_id: &str,
    ) -> anyhow::Result<Option<SalesOrder>> {
        let row: Option<SalesOrderRow> = sqlx::query_as(
            r#"
            select
                sales_order_id,
                shop_id,
                sales_order_no,
                source_type,
                source_order_ids,
                customer_id,
                currency_code,
                tax_mode,
                sales_order_status,
                delivery_status,
                invoice_status,
                payment_status,
                refund_status,
                return_status,
                stock_status,
                total_amount_cent,
                discount_amount_cent,
                shipping_amount_cent,
                receivable_amount_cent,
                paid_amount_cent,
                refunded_amount_cent,
                invoice_amount_cent,
                status_flags,
                remark,
                version,
                created_at,
                updated_at,
                created_by,
                updated_by,
                receiver_name,
                receiver_phone,
                receiver_address,
                receiver_province,
                receiver_city,
                receiver_district,
                receiver_postcode,
                carrier_code,
                logistics_service,
                expected_ship_at,
                wms_outbound_id,
                wms_outbound_status,
                wms_wave_id,
                wms_wave_status,
                wms_ship_status,
                package_count,
                waybill_nos,
                review_status,
                fulfillment_status,
                exception_status,
                freeze_reason,
                exception_flags
            from sales_orders
            where tenant_id = $1 and source_order_ids ? $2
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(source_order_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_order(tenant_id)))
    }

    async fn fetch_sales_order_row(
        &self,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SalesOrderRow>> {
        let row: Option<SalesOrderRow> = sqlx::query_as(
            r#"
            select
                sales_order_id,
                shop_id,
                sales_order_no,
                source_type,
                source_order_ids,
                customer_id,
                currency_code,
                tax_mode,
                sales_order_status,
                delivery_status,
                invoice_status,
                payment_status,
                refund_status,
                return_status,
                stock_status,
                total_amount_cent,
                discount_amount_cent,
                shipping_amount_cent,
                receivable_amount_cent,
                paid_amount_cent,
                refunded_amount_cent,
                invoice_amount_cent,
                status_flags,
                remark,
                version,
                created_at,
                updated_at,
                created_by,
                updated_by,
                receiver_name,
                receiver_phone,
                receiver_address,
                receiver_province,
                receiver_city,
                receiver_district,
                receiver_postcode,
                carrier_code,
                logistics_service,
                expected_ship_at,
                wms_outbound_id,
                wms_outbound_status,
                wms_wave_id,
                wms_wave_status,
                wms_ship_status,
                package_count,
                waybill_nos,
                review_status,
                fulfillment_status,
                exception_status,
                freeze_reason,
                exception_flags
            from sales_orders
            where tenant_id = $1 and sales_order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sales_order_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row)
    }

    async fn fetch_sales_order_row_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        sales_order_id: uuid::Uuid,
    ) -> anyhow::Result<Option<SalesOrderRow>> {
        let row: Option<SalesOrderRow> = sqlx::query_as(
            r#"
            select
                sales_order_id,
                shop_id,
                sales_order_no,
                source_type,
                source_order_ids,
                customer_id,
                currency_code,
                tax_mode,
                sales_order_status,
                delivery_status,
                invoice_status,
                payment_status,
                refund_status,
                return_status,
                stock_status,
                total_amount_cent,
                discount_amount_cent,
                shipping_amount_cent,
                receivable_amount_cent,
                paid_amount_cent,
                refunded_amount_cent,
                invoice_amount_cent,
                status_flags,
                remark,
                version,
                created_at,
                updated_at,
                created_by,
                updated_by,
                receiver_name,
                receiver_phone,
                receiver_address,
                receiver_province,
                receiver_city,
                receiver_district,
                receiver_postcode,
                carrier_code,
                logistics_service,
                expected_ship_at,
                wms_outbound_id,
                wms_outbound_status,
                wms_wave_id,
                wms_wave_status,
                wms_ship_status,
                package_count,
                waybill_nos,
                review_status,
                fulfillment_status,
                exception_status,
                freeze_reason,
                exception_flags
            from sales_orders
            where tenant_id = $1 and sales_order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(sales_order_id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(row)
    }
}

fn apply_sales_order_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a SalesOrderListFilters,
) {
    if let Some(shop_id) = filters.shop_id {
        qb.push(" and shop_id = ");
        qb.push_bind(shop_id);
    }
    if let Some(status) = &filters.sales_order_status {
        qb.push(" and sales_order_status = ");
        qb.push_bind(status);
    }
    if let Some(status) = &filters.delivery_status {
        qb.push(" and delivery_status = ");
        qb.push_bind(status);
    }
    if let Some(status) = &filters.payment_status {
        qb.push(" and payment_status = ");
        qb.push_bind(status);
    }
    if let Some(status) = &filters.refund_status {
        qb.push(" and refund_status = ");
        qb.push_bind(status);
    }
    if let Some(customer_id) = filters.customer_id {
        qb.push(" and customer_id = ");
        qb.push_bind(customer_id);
    }
    if let Some(sales_order_no) = &filters.sales_order_no {
        qb.push(" and sales_order_no = ");
        qb.push_bind(sales_order_no);
    }
    if let Some(keyword) = &filters.keyword {
        qb.push(" and (sales_order_no ilike ");
        qb.push_bind(format!("%{keyword}%"));
        qb.push(" or receiver_name ilike ");
        qb.push_bind(format!("%{keyword}%"));
        qb.push(" or receiver_phone ilike ");
        qb.push_bind(format!("%{keyword}%"));
        qb.push(")");
    }
    if let Some(source_order_id) = &filters.source_order_id {
        qb.push(" and source_order_ids ? ");
        qb.push_bind(source_order_id);
    }
    if let Some(outbound_id) = filters.wms_outbound_id {
        qb.push(" and wms_outbound_id = ");
        qb.push_bind(outbound_id);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and created_at <= ");
        qb.push_bind(created_to);
    }
    if let Some(updated_from) = filters.updated_from {
        qb.push(" and updated_at >= ");
        qb.push_bind(updated_from);
    }
    if let Some(updated_to) = filters.updated_to {
        qb.push(" and updated_at <= ");
        qb.push_bind(updated_to);
    }
}

#[derive(sqlx::FromRow)]
struct SalesOrderRow {
    sales_order_id: uuid::Uuid,
    shop_id: Option<uuid::Uuid>,
    sales_order_no: String,
    source_type: String,
    source_order_ids: Value,
    customer_id: Option<uuid::Uuid>,
    currency_code: String,
    tax_mode: Option<String>,
    sales_order_status: String,
    delivery_status: String,
    invoice_status: String,
    payment_status: String,
    refund_status: String,
    return_status: Option<String>,
    stock_status: Option<String>,
    total_amount_cent: i64,
    discount_amount_cent: i64,
    shipping_amount_cent: i64,
    receivable_amount_cent: i64,
    paid_amount_cent: i64,
    refunded_amount_cent: i64,
    invoice_amount_cent: i64,
    status_flags: Value,
    remark: Option<String>,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    created_by: Option<uuid::Uuid>,
    updated_by: Option<uuid::Uuid>,
    receiver_name: Option<String>,
    receiver_phone: Option<String>,
    receiver_address: Option<String>,
    receiver_province: Option<String>,
    receiver_city: Option<String>,
    receiver_district: Option<String>,
    receiver_postcode: Option<String>,
    carrier_code: Option<String>,
    logistics_service: Option<String>,
    expected_ship_at: Option<DateTime<Utc>>,
    wms_outbound_id: Option<uuid::Uuid>,
    wms_outbound_status: Option<String>,
    wms_wave_id: Option<uuid::Uuid>,
    wms_wave_status: Option<String>,
    wms_ship_status: Option<String>,
    package_count: i32,
    waybill_nos: Value,
    review_status: Option<String>,
    fulfillment_status: Option<String>,
    exception_status: Option<String>,
    freeze_reason: Option<String>,
    exception_flags: Value,
}

impl SalesOrderRow {
    fn into_order(self, tenant_id: TenantId) -> SalesOrder {
        SalesOrder {
            sales_order_id: self.sales_order_id,
            tenant_id,
            shop_id: self.shop_id,
            sales_order_no: self.sales_order_no,
            source_type: self.source_type,
            source_order_ids: json_array_to_vec(&self.source_order_ids),
            customer_id: self.customer_id,
            currency_code: self.currency_code,
            tax_mode: self.tax_mode,
            sales_order_status: self.sales_order_status,
            delivery_status: self.delivery_status,
            invoice_status: self.invoice_status,
            payment_status: self.payment_status,
            refund_status: self.refund_status,
            return_status: self.return_status,
            stock_status: self.stock_status,
            total_amount_cent: self.total_amount_cent,
            discount_amount_cent: self.discount_amount_cent,
            shipping_amount_cent: self.shipping_amount_cent,
            receivable_amount_cent: self.receivable_amount_cent,
            paid_amount_cent: self.paid_amount_cent,
            refunded_amount_cent: self.refunded_amount_cent,
            invoice_amount_cent: self.invoice_amount_cent,
            status_flags: json_array_to_vec(&self.status_flags),
            remark: self.remark,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
            created_by: self.created_by,
            updated_by: self.updated_by,
            receiver_name: self.receiver_name,
            receiver_phone: self.receiver_phone,
            receiver_address: self.receiver_address,
            receiver_province: self.receiver_province,
            receiver_city: self.receiver_city,
            receiver_district: self.receiver_district,
            receiver_postcode: self.receiver_postcode,
            carrier_code: self.carrier_code,
            logistics_service: self.logistics_service,
            expected_ship_at: self.expected_ship_at,
            wms_outbound_id: self.wms_outbound_id,
            wms_outbound_status: self.wms_outbound_status,
            wms_wave_id: self.wms_wave_id,
            wms_wave_status: self.wms_wave_status,
            wms_ship_status: self.wms_ship_status,
            package_count: self.package_count,
            waybill_nos: json_array_to_vec(&self.waybill_nos),
            review_status: self.review_status,
            fulfillment_status: self.fulfillment_status,
            exception_status: self.exception_status,
            freeze_reason: self.freeze_reason,
            exception_flags: json_array_to_vec(&self.exception_flags),
        }
    }
}

#[derive(sqlx::FromRow)]
struct SalesOrderItemRow {
    line_id: uuid::Uuid,
    sales_order_id: uuid::Uuid,
    sku_id: Option<uuid::Uuid>,
    sku_code: Option<String>,
    sku_name: Option<String>,
    qty: i32,
    price_cent: i64,
    tax_rate: Option<f64>,
    discount_cent: i64,
    amount_cent: i64,
    warehouse_id: Option<uuid::Uuid>,
    delivery_deadline: Option<DateTime<Utc>>,
    extra: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SalesOrderItemRow {
    fn into_item(self) -> SalesOrderItem {
        SalesOrderItem {
            line_id: self.line_id,
            sales_order_id: self.sales_order_id,
            sku_id: self.sku_id,
            sku_code: self.sku_code,
            sku_name: self.sku_name,
            qty: self.qty,
            price_cent: self.price_cent,
            tax_rate: self.tax_rate,
            discount_cent: self.discount_cent,
            amount_cent: self.amount_cent,
            warehouse_id: self.warehouse_id,
            delivery_deadline: self.delivery_deadline,
            extra: self.extra,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn json_array_to_vec(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|value| value.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}
