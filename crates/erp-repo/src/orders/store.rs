use super::models::*;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use erp_domain::{OrderItemProjection, OrderProjection};
use erp_types::{
    AfterSaleState, OmsOrderStatus, OrderExceptionStatus, OrderFulfillmentStatus, OrderId,
    OrderMainStatus, OrderPaymentStatus, OrderReviewStatus, OrderStatus, Platform, RefundState,
    ShopId, TenantId, UserId,
};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::{collections::HashMap, str::FromStr};

fn month_start(dt: DateTime<Utc>) -> NaiveDate {
    let d = dt.date_naive();
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
}

pub struct OrdersStore<'a> {
    pool: &'a PgPool,
}
impl<'a> OrdersStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Upsert an order projection. Returns stable internal order_id.
    pub async fn upsert_order_projection(
        &self,
        order: &OrderProjection,
    ) -> anyhow::Result<OrderId> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        // 1) Ensure mapping exists (platform_order_id -> order_id + order_month)
        let initial_month = month_start(order.order_created_at);
        let maybe_new_order_id = uuid::Uuid::new_v4();

        sqlx::query(
            r#"
            insert into order_key_index(
                tenant_id, platform, platform_order_id, order_id, order_month, shop_id, order_created_at
            ) values ($1, $2, $3, $4, $5, $6, $7)
            on conflict (tenant_id, platform, platform_order_id) do nothing
            "#,
        )
        .bind(uuid::Uuid::from(order.tenant_id))
        .bind(order.platform.to_string())
        .bind(&order.platform_order_id)
        .bind(maybe_new_order_id)
        .bind(initial_month)
        .bind(uuid::Uuid::from(order.shop_id))
        .bind(order.order_created_at)
        .execute(&mut *tx)
        .await?;

        #[derive(sqlx::FromRow)]
        struct MappingRow {
            order_id: uuid::Uuid,
            order_month: NaiveDate,
        }

        let mapping: MappingRow = sqlx::query_as::<_, MappingRow>(
            r#"
            select order_id, order_month
            from order_key_index
            where tenant_id = $1 and platform = $2 and platform_order_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(order.tenant_id))
        .bind(order.platform.to_string())
        .bind(&order.platform_order_id)
        .fetch_one(&mut *tx)
        .await?;

        let order_id = mapping.order_id;
        let order_month = mapping.order_month;

        let customer_id = resolve_customer_id(&mut tx, order).await?;

        // 2) Upsert into orders with ordering guard
        let order_applied = sqlx::query(
            r#"
            insert into orders(
                tenant_id, order_month, order_id, platform, shop_id, platform_order_id,
                status, total_amount_cent,
                goods_amount_cent, postage_amount_cent, discount_amount_cent,
                platform_discount_cent, seller_discount_cent, coupon_discount_cent,
                currency,
                buyer_name, receiver_name, receiver_phone, receiver_address,
                customer_id,
                pay_at, ship_at, finish_at, cancel_at,
                buyer_message, seller_remark,
                logistics_company, tracking_no,
                order_created_at, platform_updated_at, raw
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31)
            on conflict (tenant_id, order_month, platform, platform_order_id)
            do update set
                status = excluded.status,
                total_amount_cent = excluded.total_amount_cent,
                goods_amount_cent = excluded.goods_amount_cent,
                postage_amount_cent = excluded.postage_amount_cent,
                discount_amount_cent = excluded.discount_amount_cent,
                platform_discount_cent = excluded.platform_discount_cent,
                seller_discount_cent = excluded.seller_discount_cent,
                coupon_discount_cent = excluded.coupon_discount_cent,
                currency = excluded.currency,
                buyer_name = excluded.buyer_name,
                receiver_name = excluded.receiver_name,
                receiver_phone = excluded.receiver_phone,
                receiver_address = excluded.receiver_address,
                customer_id = coalesce(excluded.customer_id, orders.customer_id),
                pay_at = excluded.pay_at,
                ship_at = excluded.ship_at,
                finish_at = excluded.finish_at,
                cancel_at = excluded.cancel_at,
                buyer_message = excluded.buyer_message,
                seller_remark = excluded.seller_remark,
                logistics_company = excluded.logistics_company,
                tracking_no = excluded.tracking_no,
                platform_updated_at = coalesce(excluded.platform_updated_at, orders.platform_updated_at),
                raw = excluded.raw,
                updated_at = now()
            where (
                excluded.platform_updated_at is not null
                and excluded.platform_updated_at >= coalesce(orders.platform_updated_at, '-infinity'::timestamptz)
            ) or (
                excluded.platform_updated_at is null
                and (
                    case excluded.status
                        when 'PENDING_PAYMENT' then 10
                        when 'ON_HOLD' then 15
                        when 'PENDING_SHIPMENT' then 20
                        when 'ALLOCATED' then 25
                        when 'PARTIALLY_SHIPPED' then 30
                        when 'SHIPPED' then 40
                        when 'COMPLETED' then 50
                        when 'REFUNDING' then 60
                        when 'REFUNDED' then 70
                        when 'CANCELLED' then 80
                        when 'CLOSED' then 80
                        else 0
                    end
                ) >= (
                    case orders.status
                        when 'PENDING_PAYMENT' then 10
                        when 'ON_HOLD' then 15
                        when 'PENDING_SHIPMENT' then 20
                        when 'ALLOCATED' then 25
                        when 'PARTIALLY_SHIPPED' then 30
                        when 'SHIPPED' then 40
                        when 'COMPLETED' then 50
                        when 'REFUNDING' then 60
                        when 'REFUNDED' then 70
                        when 'CANCELLED' then 80
                        when 'CLOSED' then 80
                        else 0
                    end
                )
            )
            "#,
        )
        .bind(uuid::Uuid::from(order.tenant_id))
        .bind(order_month)
        .bind(order_id)
        .bind(order.platform.to_string())
        .bind(uuid::Uuid::from(order.shop_id))
        .bind(&order.platform_order_id)
        .bind(order.status.to_string())
        .bind(order.total_amount_cent)
        .bind(order.goods_amount_cent)
        .bind(order.postage_amount_cent)
        .bind(order.discount_amount_cent)
        .bind(order.platform_discount_cent)
        .bind(order.seller_discount_cent)
        .bind(order.coupon_discount_cent)
        .bind(&order.currency)
        .bind(&order.buyer_name)
        .bind(&order.receiver_name)
        .bind(&order.receiver_phone)
        .bind(&order.receiver_address)
        .bind(customer_id)
        .bind(order.pay_at)
        .bind(order.ship_at)
        .bind(order.finish_at)
        .bind(order.cancel_at)
        .bind(&order.buyer_message)
        .bind(&order.seller_remark)
        .bind(&order.logistics_company)
        .bind(&order.tracking_no)
        .bind(order.order_created_at)
        .bind(order.platform_updated_at)
        .bind(&order.raw)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;

        if order_applied {
            // 3) Update order_index (order_id -> partition locator)
            sqlx::query(
                r#"
                insert into order_index(
                    tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
                ) values ($1,$2,$3,$4,$5,$6,$7)
                on conflict (tenant_id, order_id)
                do update set
                    order_month = excluded.order_month,
                    platform = excluded.platform,
                    platform_order_id = excluded.platform_order_id,
                    shop_id = excluded.shop_id,
                    order_created_at = excluded.order_created_at,
                    updated_at = now()
                "#,
            )
            .bind(uuid::Uuid::from(order.tenant_id))
            .bind(order_id)
            .bind(order_month)
            .bind(order.platform.to_string())
            .bind(&order.platform_order_id)
            .bind(uuid::Uuid::from(order.shop_id))
            .bind(order.order_created_at)
            .execute(&mut *tx)
            .await?;

            // 4) Replace items (simple & safe baseline; can optimize with upsert/bulk later)
            sqlx::query(
                r#"
                delete from order_items
                where tenant_id = $1 and order_month = $2 and order_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(order.tenant_id))
            .bind(order_month)
            .bind(order_id)
            .execute(&mut *tx)
            .await?;

            for it in &order.items {
                self.insert_item(&mut tx, order.tenant_id, order_month, OrderId(order_id), it)
                    .await?;
            }
        }

        tx.commit().await?;
        Ok(OrderId(order_id))
    }

    async fn insert_item(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        order_month: NaiveDate,
        order_id: OrderId,
        it: &OrderItemProjection,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            insert into order_items(
                tenant_id, order_month, order_id, line_id,
                sku_id, title, qty,
                price_cent, amount_cent,
                pay_amount_cent, discount_cent,
                refund_state, after_sale_state,
                refund_amount_cent, after_sale_type, after_sale_id,
                raw
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_month)
        .bind(uuid::Uuid::from(order_id))
        .bind(&it.line_id)
        .bind(&it.sku_id)
        .bind(&it.title)
        .bind(it.qty)
        .bind(it.price_cent)
        .bind(it.amount_cent)
        .bind(it.pay_amount_cent)
        .bind(it.discount_cent)
        .bind(it.refund_state.to_string())
        .bind(it.after_sale_state.to_string())
        .bind(it.refund_amount_cent)
        .bind(&it.after_sale_type)
        .bind(&it.after_sale_id)
        .bind(&it.raw)
        .execute(&mut **tx)
        .await?;

        self.insert_bundle_components(tx, tenant_id, order_id, it)
            .await?;
        Ok(())
    }

    async fn insert_bundle_components(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        order_id: OrderId,
        it: &OrderItemProjection,
    ) -> anyhow::Result<()> {
        let Some(sku_raw) = it.sku_id.as_ref() else {
            return Ok(());
        };
        let Ok(bundle_sku_id) = uuid::Uuid::parse_str(sku_raw) else {
            return Ok(());
        };

        #[derive(sqlx::FromRow)]
        struct BomRow {
            component_sku_id: uuid::Uuid,
            qty: i64,
            bom_version: i32,
            expire_at: Option<DateTime<Utc>>,
        }

        let rows: Vec<BomRow> = sqlx::query_as(
            r#"
            select component_sku_id, qty, bom_version, expire_at
            from sku_boms
            where tenant_id = $1 and bundle_sku_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(bundle_sku_id)
        .fetch_all(&mut **tx)
        .await?;

        if rows.is_empty() {
            return Ok(());
        }

        let order_item_id = order_item_uuid(uuid::Uuid::from(order_id), &it.line_id);
        let bom_version = rows.iter().map(|row| row.bom_version).max().unwrap_or(1);

        let snapshot = Value::Array(
            rows.iter()
                .map(|row| {
                    serde_json::json!({
                        "component_sku_id": row.component_sku_id,
                        "qty": row.qty,
                        "expire_at": row.expire_at,
                    })
                })
                .collect(),
        );

        let mut qb = QueryBuilder::<Postgres>::new(
            "insert into order_item_components(tenant_id, order_id, order_item_id, bundle_sku_id, component_sku_id, component_qty, bom_version, bom_snapshot) ",
        );
        qb.push_values(rows.iter(), |mut row, component| {
            let qty = i64::from(it.qty);
            let component_qty = qty.checked_mul(component.qty).unwrap_or(component.qty);
            row.push_bind(uuid::Uuid::from(tenant_id));
            row.push_bind(uuid::Uuid::from(order_id));
            row.push_bind(order_item_id);
            row.push_bind(bundle_sku_id);
            row.push_bind(component.component_sku_id);
            row.push_bind(component_qty);
            row.push_bind(bom_version);
            row.push_bind(snapshot.clone());
        });
        qb.push(" on conflict do nothing");
        qb.build().execute(&mut **tx).await?;

        Ok(())
    }

    pub async fn locate_by_platform_order_id(
        &self,
        tenant_id: TenantId,
        platform: Platform,
        platform_order_id: &str,
    ) -> anyhow::Result<Option<(OrderId, NaiveDate)>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            order_id: uuid::Uuid,
            order_month: NaiveDate,
        }

        let row = sqlx::query_as::<_, Row>(
            r#"
            select order_id, order_month
            from order_key_index
            where tenant_id = $1 and platform = $2 and platform_order_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(platform.to_string())
        .bind(platform_order_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|r| (OrderId(r.order_id), r.order_month)))
    }

    pub async fn get_order_shipping_info(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Option<OrderShippingInfo>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            status: String,
            platform: String,
            platform_order_id: String,
            shop_id: uuid::Uuid,
            order_month: NaiveDate,
        }

        let row = sqlx::query_as::<_, Row>(
            r#"
            select o.status, o.platform, o.platform_order_id, o.shop_id, o.order_month
            from orders o
            join order_index i
              on i.tenant_id = o.tenant_id
             and i.order_id = o.order_id
             and i.order_month = o.order_month
            where i.tenant_id = $1 and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let platform = Platform::from_str(&row.platform)
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
        let status = parse_order_status(&row.status);

        Ok(Some(OrderShippingInfo {
            status,
            platform,
            platform_order_id: row.platform_order_id,
            shop_id: ShopId(row.shop_id),
            order_month: row.order_month,
        }))
    }

    pub async fn list_order_items(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Vec<OrderItemLine>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            line_id: String,
            sku_id: Option<String>,
            qty: i32,
        }

        let rows: Vec<Row> = sqlx::query_as::<_, Row>(
            r#"
            select oi.line_id, oi.sku_id, oi.qty
            from order_items oi
            join order_index i
              on i.tenant_id = oi.tenant_id
             and i.order_id = oi.order_id
             and i.order_month = oi.order_month
            where i.tenant_id = $1 and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OrderItemLine {
                line_id: row.line_id,
                sku_id: row.sku_id,
                qty: row.qty as i64,
            })
            .collect())
    }

    pub async fn list_order_item_stock_policies(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Vec<OrderItemStockPolicy>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            line_id: String,
            sku_id: Option<String>,
            sku_exists: bool,
            allow_negative_stock: bool,
        }

        let rows: Vec<Row> = sqlx::query_as::<_, Row>(
            r#"
            select
                oi.line_id,
                oi.sku_id,
                (s.sku_id is not null) as sku_exists,
                coalesce(pw.allow_negative_stock, false) as allow_negative_stock
            from order_items oi
            join order_index i
              on i.tenant_id = oi.tenant_id
             and i.order_id = oi.order_id
             and i.order_month = oi.order_month
            join orders o
              on o.tenant_id = i.tenant_id
             and o.order_id = i.order_id
             and o.order_month = i.order_month
            left join skus s
              on s.tenant_id = oi.tenant_id
             and s.sku_id::text = oi.sku_id
            left join products p
              on p.tenant_id = s.tenant_id
             and p.product_id = s.product_id
            left join product_warehouses pw
              on pw.tenant_id = p.tenant_id
             and pw.product_id = p.product_id
             and pw.warehouse_id = o.ship_warehouse_id
            where i.tenant_id = $1 and i.order_id = $2
            order by oi.line_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OrderItemStockPolicy {
                line_id: row.line_id,
                sku_id: row.sku_id,
                sku_exists: row.sku_exists,
                allow_negative_stock: row.allow_negative_stock,
            })
            .collect())
    }

    pub async fn get_order_review_check(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Option<OrderReviewCheck>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            logistics_company: Option<String>,
            tracking_no: Option<String>,
            inventory_shortage: bool,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select o.logistics_company, o.tracking_no, o.inventory_shortage
            from orders o
            join order_index i
              on i.tenant_id = o.tenant_id
             and i.order_id = o.order_id
             and i.order_month = o.order_month
            where i.tenant_id = $1 and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| OrderReviewCheck {
            logistics_company: row.logistics_company,
            tracking_no: row.tracking_no,
            inventory_shortage: row.inventory_shortage,
        }))
    }

    pub async fn list_order_edit_items(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Vec<OrderEditItemInput>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            line_id: String,
            sku_id: Option<String>,
            title: Option<String>,
            qty: i32,
            price_cent: i64,
        }

        let rows: Vec<Row> = sqlx::query_as::<_, Row>(
            r#"
            select oi.line_id, oi.sku_id, oi.title, oi.qty, oi.price_cent
            from order_items oi
            join order_index i
              on i.tenant_id = oi.tenant_id
             and i.order_id = oi.order_id
             and i.order_month = oi.order_month
            where i.tenant_id = $1 and i.order_id = $2
            order by oi.line_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OrderEditItemInput {
                line_id: Some(row.line_id),
                sku_id: row.sku_id,
                title: row.title,
                qty: row.qty,
                price_cent: row.price_cent,
            })
            .collect())
    }

    pub async fn rematch_order_items(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<u64> {
        let updated_primary = sqlx::query(
            r#"
            update order_items oi
            set sku_id = m.sku_id::text,
                updated_at = now()
            from order_index i
            join orders o
              on o.tenant_id = i.tenant_id
             and o.order_id = i.order_id
             and o.order_month = i.order_month
            join channel_sku_mappings m
              on m.tenant_id = o.tenant_id
             and m.shop_id = o.shop_id
             and m.platform = o.platform
             and m.platform_sku_id = (oi.raw->>'platform_sku_id')
             and m.status = 'ACTIVE'
            where oi.tenant_id = i.tenant_id
              and oi.order_id = i.order_id
              and oi.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
              and oi.sku_id is null
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .execute(self.pool)
        .await?
        .rows_affected();

        let updated_outer = sqlx::query(
            r#"
            update order_items oi
            set sku_id = m.sku_id::text,
                updated_at = now()
            from order_index i
            join orders o
              on o.tenant_id = i.tenant_id
             and o.order_id = i.order_id
             and o.order_month = i.order_month
            join channel_sku_mappings m
              on m.tenant_id = o.tenant_id
             and m.shop_id = o.shop_id
             and m.platform = o.platform
             and m.platform_outer_sku_id = (oi.raw->>'outer_sku_id')
             and m.status = 'ACTIVE'
            where oi.tenant_id = i.tenant_id
              and oi.order_id = i.order_id
              and oi.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
              and oi.sku_id is null
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .execute(self.pool)
        .await?
        .rows_affected();

        Ok(updated_primary + updated_outer)
    }

    pub async fn has_mapping_missing(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            select exists (
                select 1
                from order_items oi
                join order_index i
                  on i.tenant_id = oi.tenant_id
                 and i.order_id = oi.order_id
                 and i.order_month = oi.order_month
                where i.tenant_id = $1
                  and i.order_id = $2
                  and oi.sku_id is null
            )
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_one(self.pool)
        .await?;

        Ok(exists)
    }

    pub async fn mark_order_shipped(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        shipped_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update orders o
            set order_status = 'SHIPPED',
                fulfillment_status = 'SHIPPED',
                ship_at = $3,
                updated_at = now()
            from order_index i
            where o.tenant_id = i.tenant_id
              and o.order_id = i.order_id
              and o.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(shipped_at)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_order_receiver(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        input: OrderReceiverUpdate,
    ) -> anyhow::Result<bool> {
        let updated = sqlx::query(
            r#"
            update orders o
            set receiver_name = $3,
                receiver_phone = $4,
                receiver_address = $5,
                updated_at = now()
            from order_index i
            where o.tenant_id = i.tenant_id
              and o.order_id = i.order_id
              and o.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(input.receiver_name)
        .bind(input.receiver_phone)
        .bind(input.receiver_address)
        .execute(self.pool)
        .await?
        .rows_affected()
            > 0;

        Ok(updated)
    }

    pub async fn get_order_raw(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Option<Value>> {
        // Locate partition
        #[derive(sqlx::FromRow)]
        struct LocRow {
            order_month: NaiveDate,
        }

        let loc = sqlx::query_as::<_, LocRow>(
            r#"
            select order_month
            from order_index
            where tenant_id = $1 and order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;

        let Some(loc) = loc else {
            return Ok(None);
        };

        let raw: Option<Value> = sqlx::query_scalar(
            r#"
            select raw
            from orders
            where tenant_id = $1 and order_month = $2 and order_id = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(loc.order_month)
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(raw)
    }

    pub async fn list_orders(
        &self,
        tenant_id: TenantId,
        filters: OmsOrderListFilters,
    ) -> anyhow::Result<OmsOrderListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new("select count(*) from orders o");
        apply_order_filters(&mut count_qb, tenant_id, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                o.order_id,
                o.platform,
                o.shop_id,
                o.platform_order_id,
                o.status,
                o.oms_status,
                o.order_status,
                o.review_status,
                o.fulfillment_status,
                o.exception_status,
                o.payment_status,
                o.total_amount_cent,
                o.receiver_name,
                o.receiver_phone,
                o.customer_id,
                c.name as customer_name,
                o.order_created_at,
                o.updated_at,
                o.exception_flags,
                o.tracking_no,
                exists (
                    select 1
                    from order_items oi
                    where oi.tenant_id = o.tenant_id
                      and oi.order_id = o.order_id
                      and oi.order_month = o.order_month
                      and oi.sku_id is null
                ) as mapping_missing,
                (o.inventory_shortage or (o.exception_flags ? 'INVENTORY_SHORTAGE')) as inventory_shortage,
                exists (
                    select 1
                    from shipments s
                    where s.tenant_id = o.tenant_id
                      and s.order_id = o.order_id
                      and s.status = 'FAILED'
                ) as shipping_failed
            from orders o
            left join customers c
              on c.tenant_id = o.tenant_id and c.customer_id = o.customer_id
            "#,
        );
        apply_order_filters(&mut qb, tenant_id, &filters);
        qb.push(" order by o.order_created_at desc, o.order_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            order_id: uuid::Uuid,
            platform: String,
            shop_id: uuid::Uuid,
            platform_order_id: String,
            status: String,
            oms_status: String,
            order_status: String,
            review_status: String,
            fulfillment_status: String,
            exception_status: String,
            payment_status: Option<String>,
            total_amount_cent: i64,
            receiver_name: Option<String>,
            receiver_phone: Option<String>,
            customer_id: Option<uuid::Uuid>,
            customer_name: Option<String>,
            order_created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            exception_flags: Value,
            tracking_no: Option<String>,
            mapping_missing: bool,
            inventory_shortage: bool,
            shipping_failed: bool,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| {
                let platform = Platform::from_str(&row.platform)
                    .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
                let platform_status = parse_order_status(&row.status);
                let oms_status = parse_oms_status(&row.oms_status);
                let order_status = parse_order_main_status(&row.order_status);
                let review_status = parse_review_status(&row.review_status);
                let fulfillment_status = parse_fulfillment_status(&row.fulfillment_status);
                let exception_status = parse_exception_status(&row.exception_status);
                let payment_status = row
                    .payment_status
                    .as_deref()
                    .and_then(|raw| parse_payment_status(raw).ok());
                let receiver_phone_last4 = row.receiver_phone.as_ref().and_then(|phone| {
                    let last4: String = phone
                        .chars()
                        .rev()
                        .take(4)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    if last4.is_empty() {
                        None
                    } else {
                        Some(last4)
                    }
                });
                Ok(OmsOrderSummary {
                    id: OrderId(row.order_id),
                    platform,
                    shop_id: ShopId(row.shop_id),
                    platform_order_id: row.platform_order_id,
                    platform_status,
                    oms_status,
                    order_status,
                    review_status,
                    fulfillment_status,
                    exception_status,
                    payment_status,
                    total_amount_cent: row.total_amount_cent,
                    receiver_name: row.receiver_name,
                    receiver_phone: row.receiver_phone,
                    receiver_phone_last4,
                    customer_id: row.customer_id,
                    customer_name: row.customer_name,
                    created_at: row.order_created_at,
                    updated_at: row.updated_at,
                    exception_flags: json_array_to_vec(&row.exception_flags),
                    mapping_missing: row.mapping_missing,
                    inventory_shortage: row.inventory_shortage,
                    shipping_failed: row.shipping_failed,
                    tracking_no: row.tracking_no,
                })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;

        Ok(OmsOrderListResult { items, total })
    }

    pub async fn get_order_detail(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Option<OmsOrderDetail>> {
        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct OrderRow {
            order_id: uuid::Uuid,
            platform: String,
            shop_id: uuid::Uuid,
            platform_order_id: String,
            order_type: Option<String>,
            sales_channel: Option<String>,
            order_source: Option<String>,
            salesperson: Option<String>,
            status: String,
            oms_status: String,
            order_status: String,
            review_status: String,
            fulfillment_status: String,
            exception_status: String,
            exception_flags: Value,
            total_amount_cent: i64,
            goods_amount_cent: Option<i64>,
            postage_amount_cent: Option<i64>,
            discount_amount_cent: Option<i64>,
            receiver_name: Option<String>,
            receiver_phone: Option<String>,
            receiver_tel: Option<String>,
            receiver_address: Option<String>,
            customer_id: Option<uuid::Uuid>,
            customer_account: Option<String>,
            customer_name: Option<String>,
            customer_status: Option<String>,
            logistics_company: Option<String>,
            tracking_no: Option<String>,
            shipment_no: Option<String>,
            ship_warehouse_id: Option<uuid::Uuid>,
            ship_warehouse_name: Option<String>,
            estimated_weight_gram: Option<i32>,
            package_weight_gram: Option<i32>,
            paid_amount_cent: Option<i64>,
            subsidy_amount_cent: Option<i64>,
            receivable_amount_cent: Option<i64>,
            other_fee_cent: Option<i64>,
            estimated_postage_cent: Option<i64>,
            invoice_type: Option<String>,
            invoice_title: Option<String>,
            payment_status: Option<String>,
            pay_at: Option<DateTime<Utc>>,
            ship_at: Option<DateTime<Utc>>,
            buyer_message: Option<String>,
            seller_remark: Option<String>,
            freeze_reason: Option<String>,
            cancel_reason: Option<String>,
            exception_reason: Option<String>,
            extra_remark: Option<String>,
            order_created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            version: i32,
        }

        let order_row: Option<OrderRow> = sqlx::query_as(
            r#"
            select
                o.order_id,
                o.platform,
                o.shop_id,
                o.platform_order_id,
                o.order_type,
                o.sales_channel,
                o.order_source,
                o.salesperson,
                o.status,
                o.oms_status,
                o.order_status,
                o.review_status,
                o.fulfillment_status,
                o.exception_status,
                o.exception_flags,
                o.total_amount_cent,
                o.goods_amount_cent,
                o.postage_amount_cent,
                o.discount_amount_cent,
                o.receiver_name,
                o.receiver_phone,
                o.receiver_tel,
                o.receiver_address,
                o.customer_id,
                o.customer_account,
                c.name as customer_name,
                c.status as customer_status,
                o.logistics_company,
                o.tracking_no,
                o.shipment_no,
                o.ship_warehouse_id,
                w.name as ship_warehouse_name,
                o.estimated_weight_gram,
                o.package_weight_gram,
                o.paid_amount_cent,
                o.subsidy_amount_cent,
                o.receivable_amount_cent,
                o.other_fee_cent,
                o.estimated_postage_cent,
                o.invoice_type,
                o.invoice_title,
                o.payment_status,
                o.pay_at,
                o.ship_at,
                o.buyer_message,
                o.seller_remark,
                o.freeze_reason,
                o.cancel_reason,
                o.exception_reason,
                o.extra_remark,
                o.order_created_at,
                o.updated_at,
                o.version
            from orders o
            join order_index i
              on i.tenant_id = o.tenant_id
             and i.order_id = o.order_id
             and i.order_month = o.order_month
            left join customers c
              on c.tenant_id = o.tenant_id and c.customer_id = o.customer_id
            left join warehouses w
              on w.tenant_id = o.tenant_id and w.warehouse_id = o.ship_warehouse_id
            where i.tenant_id = $1 and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;

        let Some(order_row) = order_row else {
            return Ok(None);
        };

        let platform = Platform::from_str(&order_row.platform)
            .map_err(|e| anyhow::anyhow!("bad platform in db: {e}"))?;
        let order = OmsOrder {
            id: OrderId(order_row.order_id),
            platform,
            shop_id: ShopId(order_row.shop_id),
            platform_order_id: order_row.platform_order_id,
            order_type: order_row.order_type,
            sales_channel: order_row.sales_channel,
            order_source: order_row.order_source,
            salesperson: order_row.salesperson,
            platform_status: parse_order_status(&order_row.status),
            oms_status: parse_oms_status(&order_row.oms_status),
            order_status: parse_order_main_status(&order_row.order_status),
            review_status: parse_review_status(&order_row.review_status),
            fulfillment_status: parse_fulfillment_status(&order_row.fulfillment_status),
            exception_status: parse_exception_status(&order_row.exception_status),
            exception_flags: json_array_to_vec(&order_row.exception_flags),
            total_amount_cent: order_row.total_amount_cent,
            goods_amount_cent: order_row.goods_amount_cent,
            postage_amount_cent: order_row.postage_amount_cent,
            discount_amount_cent: order_row.discount_amount_cent,
            receiver_name: order_row.receiver_name,
            receiver_phone: order_row.receiver_phone,
            receiver_tel: order_row.receiver_tel,
            receiver_address: order_row.receiver_address,
            customer_id: order_row.customer_id,
            customer_account: order_row.customer_account,
            customer_name: order_row.customer_name,
            customer_status: order_row.customer_status,
            logistics_company: order_row.logistics_company,
            tracking_no: order_row.tracking_no,
            shipment_no: order_row.shipment_no,
            ship_warehouse_id: order_row.ship_warehouse_id,
            ship_warehouse_name: order_row.ship_warehouse_name,
            estimated_weight_gram: order_row.estimated_weight_gram,
            package_weight_gram: order_row.package_weight_gram,
            paid_amount_cent: order_row.paid_amount_cent,
            subsidy_amount_cent: order_row.subsidy_amount_cent,
            receivable_amount_cent: order_row.receivable_amount_cent,
            other_fee_cent: order_row.other_fee_cent,
            estimated_postage_cent: order_row.estimated_postage_cent,
            invoice_type: order_row.invoice_type,
            invoice_title: order_row.invoice_title,
            payment_status: order_row
                .payment_status
                .as_deref()
                .and_then(|raw| parse_payment_status(raw).ok()),
            pay_at: order_row.pay_at,
            ship_at: order_row.ship_at,
            buyer_message: order_row.buyer_message,
            seller_remark: order_row.seller_remark,
            freeze_reason: order_row.freeze_reason,
            cancel_reason: order_row.cancel_reason,
            exception_reason: order_row.exception_reason,
            extra_remark: order_row.extra_remark,
            order_created_at: order_row.order_created_at,
            updated_at: order_row.updated_at,
            version: order_row.version,
        };

        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct ItemRow {
            line_id: String,
            sku_id: Option<String>,
            title: Option<String>,
            qty: i32,
            price_cent: i64,
            raw: Value,
        }

        let item_rows: Vec<ItemRow> = sqlx::query_as(
            r#"
            select oi.line_id, oi.sku_id, oi.title, oi.qty, oi.price_cent, oi.raw
            from order_items oi
            join order_index i
              on i.tenant_id = oi.tenant_id
             and i.order_id = oi.order_id
             and i.order_month = oi.order_month
            where i.tenant_id = $1 and i.order_id = $2
            order by oi.line_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(self.pool)
        .await?;

        let items = item_rows
            .into_iter()
            .map(|row| {
                let platform_sku_id = row
                    .raw
                    .get("platform_sku_id")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string())
                    .or_else(|| row.sku_id.clone());
                let outer_sku_id = row
                    .raw
                    .get("outer_sku_id")
                    .and_then(|value| value.as_str())
                    .map(|value| value.to_string());
                OmsOrderItem {
                    id: row.line_id,
                    platform_sku_id,
                    outer_sku_id,
                    title: row.title,
                    quantity: row.qty,
                    price_cent: row.price_cent,
                    mapping_missing: row.sku_id.is_none(),
                }
            })
            .collect();

        #[derive(sqlx::FromRow)]
        struct HitRow {
            rule_id: uuid::Uuid,
            rule_name: String,
            action: String,
            hit: bool,
            message: String,
            created_at: DateTime<Utc>,
        }

        let hit_rows: Vec<HitRow> = sqlx::query_as(
            r#"
            select h.rule_id, r.name as rule_name, r.action, h.hit, h.message, h.created_at
            from order_review_hits h
            join order_review_rules r on r.rule_id = h.rule_id
            where h.tenant_id = $1 and h.order_id = $2
            order by r.priority desc, h.created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(self.pool)
        .await?;

        let review_hits = hit_rows
            .into_iter()
            .map(|row| OrderReviewHit {
                rule_id: row.rule_id,
                rule_name: row.rule_name,
                action: row.action,
                hit: row.hit,
                message: row.message,
                created_at: row.created_at,
            })
            .collect();

        #[derive(sqlx::FromRow)]
        struct TagRow {
            tag: String,
            created_by: Option<uuid::Uuid>,
            created_at: DateTime<Utc>,
        }

        let tag_rows: Vec<TagRow> = sqlx::query_as(
            r#"
            select tag, created_by, created_at
            from order_tags
            where tenant_id = $1 and order_id = $2
            order by created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(self.pool)
        .await?;

        let tags = tag_rows
            .into_iter()
            .map(|row| OrderTag {
                tag: row.tag,
                created_by: row.created_by.map(UserId),
                created_at: row.created_at,
            })
            .collect();

        #[derive(sqlx::FromRow)]
        struct NoteRow {
            note_id: uuid::Uuid,
            note: String,
            created_by: Option<uuid::Uuid>,
            created_at: DateTime<Utc>,
        }

        let note_rows: Vec<NoteRow> = sqlx::query_as(
            r#"
            select note_id, note, created_by, created_at
            from order_notes
            where tenant_id = $1 and order_id = $2
            order by created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(self.pool)
        .await?;

        let notes = note_rows
            .into_iter()
            .map(|row| OrderNote {
                note_id: row.note_id,
                note: row.note,
                created_by: row.created_by.map(UserId),
                created_at: row.created_at,
            })
            .collect();

        Ok(Some(OmsOrderDetail {
            order,
            items,
            review_hits,
            tags,
            notes,
        }))
    }

    pub async fn get_order_review_state(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Option<OrderReviewState>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            oms_status: String,
            order_status: String,
            review_status: String,
            fulfillment_status: String,
            exception_status: String,
            exception_flags: Value,
            version: i32,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select o.oms_status,
                   o.order_status,
                   o.review_status,
                   o.fulfillment_status,
                   o.exception_status,
                   o.exception_flags,
                   o.version
            from orders o
            join order_index i
              on i.tenant_id = o.tenant_id
             and i.order_id = o.order_id
             and i.order_month = o.order_month
            where i.tenant_id = $1 and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| OrderReviewState {
            oms_status: parse_oms_status(&row.oms_status),
            order_status: parse_order_main_status(&row.order_status),
            review_status: parse_review_status(&row.review_status),
            fulfillment_status: parse_fulfillment_status(&row.fulfillment_status),
            exception_status: parse_exception_status(&row.exception_status),
            exception_flags: json_array_to_vec(&row.exception_flags),
            version: row.version,
        }))
    }

    pub async fn get_order_edit_state(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Option<OrderEditState>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            order_month: NaiveDate,
            shop_id: uuid::Uuid,
            status: String,
            oms_status: String,
            order_status: String,
            review_status: String,
            version: i32,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select o.order_month,
                   o.shop_id,
                   o.status,
                   o.oms_status,
                   o.order_status,
                   o.review_status,
                   o.version
            from orders o
            join order_index i
              on i.tenant_id = o.tenant_id
             and i.order_id = o.order_id
             and i.order_month = o.order_month
            where i.tenant_id = $1 and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| OrderEditState {
            order_month: row.order_month,
            shop_id: ShopId(row.shop_id),
            status: parse_order_status(&row.status),
            oms_status: parse_oms_status(&row.oms_status),
            order_status: parse_order_main_status(&row.order_status),
            review_status: parse_review_status(&row.review_status),
            version: row.version,
        }))
    }

    pub async fn update_order_edit(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        input: OrderEditInput,
    ) -> anyhow::Result<Option<()>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            order_month: NaiveDate,
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let row: Option<Row> = sqlx::query_as(
            r#"
            select o.order_month
            from orders o
            join order_index i
              on i.tenant_id = o.tenant_id
             and i.order_id = o.order_id
             and i.order_month = o.order_month
            where i.tenant_id = $1 and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let updated: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            update orders o
            set receiver_name = coalesce($3, receiver_name),
                receiver_phone = coalesce($4, receiver_phone),
                receiver_tel = coalesce($5, receiver_tel),
                receiver_address = coalesce($6, receiver_address),
                logistics_company = coalesce($7, logistics_company),
                tracking_no = coalesce($8, tracking_no),
                order_type = coalesce($9, order_type),
                sales_channel = coalesce($10, sales_channel),
                order_source = coalesce($11, order_source),
                salesperson = coalesce($12, salesperson),
                ship_warehouse_id = coalesce($13, ship_warehouse_id),
                shipment_no = coalesce($14, shipment_no),
                estimated_weight_gram = coalesce($15, estimated_weight_gram),
                package_weight_gram = coalesce($16, package_weight_gram),
                customer_account = coalesce($17, customer_account),
                paid_amount_cent = coalesce($18, paid_amount_cent),
                subsidy_amount_cent = coalesce($19, subsidy_amount_cent),
                receivable_amount_cent = coalesce($20, receivable_amount_cent),
                other_fee_cent = coalesce($21, other_fee_cent),
                estimated_postage_cent = coalesce($22, estimated_postage_cent),
                invoice_type = coalesce($23, invoice_type),
                invoice_title = coalesce($24, invoice_title),
                payment_status = coalesce($25, payment_status),
                buyer_message = coalesce($26, buyer_message),
                seller_remark = coalesce($27, seller_remark),
                freeze_reason = coalesce($28, freeze_reason),
                cancel_reason = coalesce($29, cancel_reason),
                exception_reason = coalesce($30, exception_reason),
                extra_remark = coalesce($31, extra_remark),
                version = o.version + 1,
                updated_at = now()
            from order_index i
            where o.tenant_id = i.tenant_id
              and o.order_id = i.order_id
              and o.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
            returning o.order_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(input.receiver_name)
        .bind(input.receiver_phone)
        .bind(input.receiver_tel)
        .bind(input.receiver_address)
        .bind(input.logistics_company)
        .bind(input.tracking_no)
        .bind(input.order_type)
        .bind(input.sales_channel)
        .bind(input.order_source)
        .bind(input.salesperson)
        .bind(input.ship_warehouse_id)
        .bind(input.shipment_no)
        .bind(input.estimated_weight_gram)
        .bind(input.package_weight_gram)
        .bind(input.customer_account)
        .bind(input.paid_amount_cent)
        .bind(input.subsidy_amount_cent)
        .bind(input.receivable_amount_cent)
        .bind(input.other_fee_cent)
        .bind(input.estimated_postage_cent)
        .bind(input.invoice_type)
        .bind(input.invoice_title)
        .bind(input.payment_status)
        .bind(input.buyer_message)
        .bind(input.seller_remark)
        .bind(input.freeze_reason)
        .bind(input.cancel_reason)
        .bind(input.exception_reason)
        .bind(input.extra_remark)
        .fetch_optional(&mut *tx)
        .await?;

        if updated.is_none() {
            return Ok(None);
        }

        if let Some(items) = input.items {
            sqlx::query(
                r#"
                delete from order_item_components
                where tenant_id = $1 and order_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(order_id))
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                delete from order_items
                where tenant_id = $1 and order_month = $2 and order_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(row.order_month)
            .bind(uuid::Uuid::from(order_id))
            .execute(&mut *tx)
            .await?;

            for item in items {
                let line_id = non_empty_trimmed(item.line_id.as_deref())
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let amount_cent = item.price_cent.saturating_mul(i64::from(item.qty));
                let projection = OrderItemProjection {
                    line_id,
                    sku_id: item.sku_id.clone(),
                    title: item.title.clone(),
                    qty: item.qty,
                    price_cent: item.price_cent,
                    amount_cent,
                    pay_amount_cent: Some(amount_cent),
                    discount_cent: Some(0),
                    refund_state: RefundState::None,
                    after_sale_state: AfterSaleState::None,
                    refund_amount_cent: None,
                    after_sale_type: None,
                    after_sale_id: None,
                    raw: serde_json::json!({
                        "manual_edit": true,
                        "sku_id": item.sku_id,
                        "title": item.title,
                        "qty": item.qty,
                        "price_cent": item.price_cent,
                    }),
                };
                self.insert_item(&mut tx, tenant_id, row.order_month, order_id, &projection)
                    .await?;
            }
        }

        tx.commit().await?;
        Ok(Some(()))
    }

    pub async fn update_order_amounts(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        total_amount_cent: i64,
        goods_amount_cent: Option<i64>,
        postage_amount_cent: Option<i64>,
        discount_amount_cent: Option<i64>,
    ) -> anyhow::Result<Option<()>> {
        let updated: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            update orders o
            set total_amount_cent = $3,
                goods_amount_cent = $4,
                postage_amount_cent = $5,
                discount_amount_cent = $6,
                updated_at = now()
            from order_index i
            where o.tenant_id = i.tenant_id
              and o.order_id = i.order_id
              and o.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
            returning o.order_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(total_amount_cent)
        .bind(goods_amount_cent)
        .bind(postage_amount_cent)
        .bind(discount_amount_cent)
        .fetch_optional(self.pool)
        .await?;

        Ok(updated.map(|_| ()))
    }

    pub async fn update_order_review_state(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        input: OrderReviewUpdate,
    ) -> anyhow::Result<Option<OrderReviewState>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            oms_status: String,
            order_status: String,
            review_status: String,
            fulfillment_status: String,
            exception_status: String,
            exception_flags: Value,
            version: i32,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            update orders o
            set oms_status = $3,
                order_status = $4,
                review_status = $5,
                fulfillment_status = $6,
                exception_status = $7,
                exception_flags = $8,
                version = o.version + 1,
                updated_at = now()
            from order_index i
            where o.tenant_id = i.tenant_id
              and o.order_id = i.order_id
              and o.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
              and ($9::int is null or o.version = $9)
            returning o.oms_status,
                      o.order_status,
                      o.review_status,
                      o.fulfillment_status,
                      o.exception_status,
                      o.exception_flags,
                      o.version
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(input.oms_status.to_string())
        .bind(input.order_status.to_string())
        .bind(input.review_status.to_string())
        .bind(input.fulfillment_status.to_string())
        .bind(input.exception_status.to_string())
        .bind(Value::Array(
            input
                .exception_flags
                .into_iter()
                .map(Value::String)
                .collect(),
        ))
        .bind(input.expected_version)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| OrderReviewState {
            oms_status: parse_oms_status(&row.oms_status),
            order_status: parse_order_main_status(&row.order_status),
            review_status: parse_review_status(&row.review_status),
            fulfillment_status: parse_fulfillment_status(&row.fulfillment_status),
            exception_status: parse_exception_status(&row.exception_status),
            exception_flags: json_array_to_vec(&row.exception_flags),
            version: row.version,
        }))
    }

    pub async fn update_order_cancel_info(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        cancel_reason: Option<String>,
    ) -> anyhow::Result<Option<()>> {
        let updated: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            update orders o
            set cancel_reason = coalesce($3, cancel_reason),
                order_status = 'CANCELLED',
                fulfillment_status = 'NOT_ALLOCATED',
                exception_status = 'NONE',
                cancel_at = now(),
                updated_at = now()
            from order_index i
            where o.tenant_id = i.tenant_id
              and o.order_id = i.order_id
              and o.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
            returning o.order_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(cancel_reason)
        .fetch_optional(self.pool)
        .await?;

        Ok(updated.map(|_| ()))
    }

    pub async fn mark_order_refunded(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        cancel_reason: Option<String>,
    ) -> anyhow::Result<Option<()>> {
        let updated: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            update orders o
            set status = 'REFUNDED',
                payment_status = 'REFUNDED',
                cancel_reason = coalesce($3, cancel_reason),
                updated_at = now()
            from order_index i
            where o.tenant_id = i.tenant_id
              and o.order_id = i.order_id
              and o.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
            returning o.order_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(cancel_reason)
        .fetch_optional(self.pool)
        .await?;

        Ok(updated.map(|_| ()))
    }

    pub async fn upsert_exception_tickets(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        reason_codes: &[String],
    ) -> anyhow::Result<Vec<OrderExceptionTicket>> {
        if reason_codes.is_empty() {
            return Ok(Vec::new());
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let mut tickets = Vec::with_capacity(reason_codes.len());

        #[derive(sqlx::FromRow)]
        struct Row {
            ticket_id: uuid::Uuid,
            order_id: uuid::Uuid,
            reason_code: String,
            status: String,
            handler_id: Option<uuid::Uuid>,
            handled_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        for code in reason_codes {
            let row: Row = sqlx::query_as(
                r#"
                insert into order_exception_tickets(
                    tenant_id, order_id, reason_code, status, created_at, updated_at
                ) values ($1,$2,$3,'OPEN',now(),now())
                on conflict (tenant_id, order_id, reason_code)
                do update set updated_at = now()
                returning ticket_id, order_id, reason_code, status, handler_id, handled_at, created_at, updated_at
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(order_id))
            .bind(code)
            .fetch_one(&mut *tx)
            .await?;
            tickets.push(OrderExceptionTicket {
                ticket_id: row.ticket_id,
                order_id: OrderId(row.order_id),
                reason_code: row.reason_code,
                status: row.status,
                handler_id: row.handler_id.map(UserId),
                handled_at: row.handled_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        tx.commit().await?;
        Ok(tickets)
    }

    pub async fn update_exception_ticket_status(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        updates: &[OrderExceptionTicketUpdate],
    ) -> anyhow::Result<Vec<OrderExceptionTicket>> {
        if updates.is_empty() {
            return Ok(Vec::new());
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let mut tickets = Vec::with_capacity(updates.len());

        #[derive(sqlx::FromRow)]
        struct Row {
            ticket_id: uuid::Uuid,
            order_id: uuid::Uuid,
            reason_code: String,
            status: String,
            handler_id: Option<uuid::Uuid>,
            handled_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        for update in updates {
            let row: Row = sqlx::query_as(
                r#"
                insert into order_exception_tickets(
                    tenant_id, order_id, reason_code, status, handler_id, handled_at, created_at, updated_at
                ) values ($1,$2,$3,$4,$5,$6,now(),now())
                on conflict (tenant_id, order_id, reason_code)
                do update set
                    status = excluded.status,
                    handler_id = excluded.handler_id,
                    handled_at = excluded.handled_at,
                    updated_at = now()
                returning ticket_id, order_id, reason_code, status, handler_id, handled_at, created_at, updated_at
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(order_id))
            .bind(&update.reason_code)
            .bind(&update.status)
            .bind(update.handler_id.map(uuid::Uuid::from))
            .bind(update.handled_at)
            .fetch_one(&mut *tx)
            .await?;

            tickets.push(OrderExceptionTicket {
                ticket_id: row.ticket_id,
                order_id: OrderId(row.order_id),
                reason_code: row.reason_code,
                status: row.status,
                handler_id: row.handler_id.map(UserId),
                handled_at: row.handled_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        tx.commit().await?;
        Ok(tickets)
    }

    pub async fn get_exception_stats(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<OrderExceptionStats> {
        #[derive(sqlx::FromRow)]
        struct TotalRow {
            total: i64,
            open: i64,
            processed: i64,
        }

        let total_row: TotalRow = sqlx::query_as(
            r#"
            select
                count(*) as total,
                coalesce(sum(case when status = 'OPEN' then 1 else 0 end), 0) as open,
                coalesce(sum(case when status <> 'OPEN' then 1 else 0 end), 0) as processed
            from order_exception_tickets
            where tenant_id = $1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_one(self.pool)
        .await?;

        #[derive(sqlx::FromRow)]
        struct ReasonRow {
            reason_code: String,
            total: i64,
            open: i64,
            processed: i64,
        }

        let reason_rows: Vec<ReasonRow> = sqlx::query_as(
            r#"
            select
                reason_code,
                count(*) as total,
                coalesce(sum(case when status = 'OPEN' then 1 else 0 end), 0) as open,
                coalesce(sum(case when status <> 'OPEN' then 1 else 0 end), 0) as processed
            from order_exception_tickets
            where tenant_id = $1
            group by reason_code
            order by total desc, reason_code asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_all(self.pool)
        .await?;

        let reasons = reason_rows
            .into_iter()
            .map(|row| OrderExceptionReasonStat {
                reason_code: row.reason_code,
                total: row.total,
                open: row.open,
                processed: row.processed,
            })
            .collect();

        Ok(OrderExceptionStats {
            total: total_row.total,
            open: total_row.open,
            processed: total_row.processed,
            reasons,
        })
    }

    pub async fn upsert_review_hits(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        hits: Vec<(uuid::Uuid, bool, String)>,
    ) -> anyhow::Result<()> {
        if hits.is_empty() {
            return Ok(());
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        for (rule_id, hit, message) in hits {
            sqlx::query(
                r#"
                insert into order_review_hits(
                    tenant_id, order_id, rule_id, hit, message
                ) values ($1, $2, $3, $4, $5)
                on conflict (tenant_id, order_id, rule_id)
                do update set hit = excluded.hit,
                              message = excluded.message,
                              updated_at = now()
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(order_id))
            .bind(rule_id)
            .bind(hit)
            .bind(message)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn add_order_tags(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        tags: Vec<OrderTagCreate>,
    ) -> anyhow::Result<Vec<OrderTag>> {
        if tags.is_empty() {
            return Ok(vec![]);
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        for tag in &tags {
            sqlx::query(
                r#"
                insert into order_tags(tenant_id, order_id, tag, created_by)
                values ($1, $2, $3, $4)
                on conflict do nothing
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(uuid::Uuid::from(order_id))
            .bind(&tag.tag)
            .bind(tag.created_by.map(uuid::Uuid::from))
            .execute(&mut *tx)
            .await?;
        }
        let rows: Vec<(String, Option<uuid::Uuid>, DateTime<Utc>)> = sqlx::query_as(
            r#"
            select tag, created_by, created_at
            from order_tags
            where tenant_id = $1 and order_id = $2
            order by created_at desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(rows
            .into_iter()
            .map(|row| OrderTag {
                tag: row.0,
                created_by: row.1.map(UserId),
                created_at: row.2,
            })
            .collect())
    }

    pub async fn add_order_note(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        note: OrderNoteCreate,
    ) -> anyhow::Result<OrderNote> {
        #[derive(sqlx::FromRow)]
        struct Row {
            note_id: uuid::Uuid,
            note: String,
            created_by: Option<uuid::Uuid>,
            created_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into order_notes(tenant_id, order_id, note, created_by)
            values ($1, $2, $3, $4)
            returning note_id, note, created_by, created_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(&note.note)
        .bind(note.created_by.map(uuid::Uuid::from))
        .fetch_one(self.pool)
        .await?;

        Ok(OrderNote {
            note_id: row.note_id,
            note: row.note,
            created_by: row.created_by.map(UserId),
            created_at: row.created_at,
        })
    }

    pub async fn split_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
        plan: Vec<OrderSplitPlanChild>,
    ) -> anyhow::Result<OrderSplitResult> {
        const ORDER_SPLIT_FLAG: &str = "ORDER_SPLIT";

        if plan.len() < 2 {
            return Err(anyhow::anyhow!("split plan requires at least two children"));
        }

        fn allocate_amount(total: Option<i64>, weights: &[i64]) -> Vec<Option<i64>> {
            match total {
                None => vec![None; weights.len()],
                Some(value) => {
                    if weights.is_empty() {
                        return Vec::new();
                    }
                    let weight_sum: i64 = weights.iter().sum();
                    let mut remaining = value;
                    let mut allocations = Vec::with_capacity(weights.len());
                    for (idx, weight) in weights.iter().enumerate() {
                        let share = if idx == weights.len() - 1 {
                            remaining
                        } else if weight_sum == 0 {
                            value / weights.len() as i64
                        } else {
                            (value * *weight) / weight_sum
                        };
                        allocations.push(Some(share));
                        remaining -= share;
                    }
                    allocations
                }
            }
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct OrderRow {
            order_id: uuid::Uuid,
            order_month: NaiveDate,
            platform: String,
            shop_id: uuid::Uuid,
            platform_order_id: String,
            status: String,
            oms_status: String,
            review_status: String,
            exception_flags: Value,
            total_amount_cent: i64,
            goods_amount_cent: Option<i64>,
            postage_amount_cent: Option<i64>,
            discount_amount_cent: Option<i64>,
            platform_discount_cent: Option<i64>,
            seller_discount_cent: Option<i64>,
            coupon_discount_cent: Option<i64>,
            currency: String,
            buyer_name: Option<String>,
            receiver_name: Option<String>,
            receiver_phone: Option<String>,
            receiver_tel: Option<String>,
            receiver_address: Option<String>,
            customer_id: Option<uuid::Uuid>,
            pay_at: Option<DateTime<Utc>>,
            ship_at: Option<DateTime<Utc>>,
            finish_at: Option<DateTime<Utc>>,
            cancel_at: Option<DateTime<Utc>>,
            buyer_message: Option<String>,
            seller_remark: Option<String>,
            logistics_company: Option<String>,
            tracking_no: Option<String>,
            order_created_at: DateTime<Utc>,
            platform_updated_at: Option<DateTime<Utc>>,
            raw: Value,
            inventory_shortage: bool,
            order_type: Option<String>,
            sales_channel: Option<String>,
            order_source: Option<String>,
            salesperson: Option<String>,
            ship_warehouse_id: Option<uuid::Uuid>,
            shipment_no: Option<String>,
            estimated_weight_gram: Option<i32>,
            package_weight_gram: Option<i32>,
            customer_account: Option<String>,
            paid_amount_cent: Option<i64>,
            subsidy_amount_cent: Option<i64>,
            receivable_amount_cent: Option<i64>,
            other_fee_cent: Option<i64>,
            estimated_postage_cent: Option<i64>,
            invoice_type: Option<String>,
            invoice_title: Option<String>,
            payment_status: Option<String>,
            freeze_reason: Option<String>,
            cancel_reason: Option<String>,
            exception_reason: Option<String>,
            extra_remark: Option<String>,
            version: i32,
        }

        let order_row: Option<OrderRow> = sqlx::query_as(
            r#"
            select
                o.order_id,
                o.order_month,
                o.platform,
                o.shop_id,
                o.platform_order_id,
                o.status,
                o.oms_status,
                o.review_status,
                o.exception_flags,
                o.total_amount_cent,
                o.goods_amount_cent,
                o.postage_amount_cent,
                o.discount_amount_cent,
                o.platform_discount_cent,
                o.seller_discount_cent,
                o.coupon_discount_cent,
                o.currency,
                o.buyer_name,
                o.receiver_name,
                o.receiver_phone,
                o.receiver_tel,
                o.receiver_address,
                o.customer_id,
                o.pay_at,
                o.ship_at,
                o.finish_at,
                o.cancel_at,
                o.buyer_message,
                o.seller_remark,
                o.logistics_company,
                o.tracking_no,
                o.order_created_at,
                o.platform_updated_at,
                o.raw,
                o.inventory_shortage,
                o.order_type,
                o.sales_channel,
                o.order_source,
                o.salesperson,
                o.ship_warehouse_id,
                o.shipment_no,
                o.estimated_weight_gram,
                o.package_weight_gram,
                o.customer_account,
                o.paid_amount_cent,
                o.subsidy_amount_cent,
                o.receivable_amount_cent,
                o.other_fee_cent,
                o.estimated_postage_cent,
                o.invoice_type,
                o.invoice_title,
                o.payment_status,
                o.freeze_reason,
                o.cancel_reason,
                o.exception_reason,
                o.extra_remark,
                o.version
            from orders o
            join order_index i
              on i.tenant_id = o.tenant_id
             and i.order_id = o.order_id
             and i.order_month = o.order_month
            where i.tenant_id = $1 and i.order_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_optional(&mut *tx)
        .await?;

        let Some(order_row) = order_row else {
            return Err(anyhow::anyhow!("order not found"));
        };

        let review_status = parse_review_status(&order_row.review_status);
        if !review_status.is_pending() {
            return Err(anyhow::anyhow!("order not pending review"));
        }
        let oms_status = parse_oms_status(&order_row.oms_status);
        if matches!(
            oms_status,
            OmsOrderStatus::Shipped | OmsOrderStatus::Done | OmsOrderStatus::Cancelled
        ) {
            return Err(anyhow::anyhow!("order already shipped"));
        }

        let exception_flags = json_array_to_vec(&order_row.exception_flags);
        if exception_flags.iter().any(|flag| flag == ORDER_SPLIT_FLAG) {
            return Err(anyhow::anyhow!("order already split"));
        }

        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct ItemRow {
            line_id: String,
            sku_id: Option<String>,
            title: Option<String>,
            qty: i32,
            price_cent: i64,
            amount_cent: i64,
            pay_amount_cent: Option<i64>,
            discount_cent: Option<i64>,
            refund_state: String,
            after_sale_state: String,
            refund_amount_cent: Option<i64>,
            after_sale_type: Option<String>,
            after_sale_id: Option<String>,
            raw: Value,
        }

        let item_rows: Vec<ItemRow> = sqlx::query_as(
            r#"
            select
                line_id,
                sku_id,
                title,
                qty,
                price_cent,
                amount_cent,
                pay_amount_cent,
                discount_cent,
                refund_state,
                after_sale_state,
                refund_amount_cent,
                after_sale_type,
                after_sale_id,
                raw
            from order_items
            where tenant_id = $1 and order_id = $2 and order_month = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(order_row.order_month)
        .fetch_all(&mut *tx)
        .await?;

        if item_rows.is_empty() {
            return Err(anyhow::anyhow!("order has no items"));
        }

        let mut item_map = HashMap::new();
        for row in item_rows {
            item_map.insert(row.line_id.clone(), row);
        }

        let child_count = plan.len();
        let mut line_totals: HashMap<String, i32> = HashMap::new();
        let mut line_qtys: HashMap<String, Vec<i32>> = HashMap::new();

        for (child_idx, child) in plan.iter().enumerate() {
            if child.items.is_empty() {
                return Err(anyhow::anyhow!("split child items empty"));
            }
            for item in &child.items {
                let entry = line_totals.entry(item.line_id.clone()).or_insert(0);
                *entry += item.qty;
                let qty_entry = line_qtys
                    .entry(item.line_id.clone())
                    .or_insert_with(|| vec![0; child_count]);
                qty_entry[child_idx] += item.qty;
            }
        }

        for (line_id, row) in &item_map {
            let Some(total_qty) = line_totals.get(line_id) else {
                return Err(anyhow::anyhow!("split plan missing item"));
            };
            if *total_qty != row.qty {
                return Err(anyhow::anyhow!("split qty mismatch"));
            }
        }
        if line_totals.len() != item_map.len() {
            return Err(anyhow::anyhow!("split plan missing items"));
        }

        let mut line_pay_allocs: HashMap<String, Vec<Option<i64>>> = HashMap::new();
        let mut line_discount_allocs: HashMap<String, Vec<Option<i64>>> = HashMap::new();

        for (line_id, row) in &item_map {
            let weights = line_qtys
                .get(line_id)
                .cloned()
                .unwrap_or_else(|| vec![0; child_count]);
            let weights_i64 = weights.iter().map(|qty| *qty as i64).collect::<Vec<_>>();
            line_pay_allocs.insert(
                line_id.clone(),
                allocate_amount(row.pay_amount_cent, &weights_i64),
            );
            line_discount_allocs.insert(
                line_id.clone(),
                allocate_amount(row.discount_cent, &weights_i64),
            );
        }

        let mut child_items: Vec<Vec<OrderItemProjection>> =
            (0..child_count).map(|_| Vec::new()).collect();
        let mut child_goods_amounts = vec![0i64; child_count];

        for (child_idx, child) in plan.iter().enumerate() {
            for item in &child.items {
                let row = item_map
                    .get(&item.line_id)
                    .ok_or_else(|| anyhow::anyhow!("split item not found"))?;
                if item.qty <= 0 {
                    return Err(anyhow::anyhow!("split item qty invalid"));
                }
                let amount_cent = row.price_cent.saturating_mul(i64::from(item.qty));
                let mut raw = row.raw.clone();
                if let Some(obj) = raw.as_object_mut() {
                    obj.insert(
                        "split_from_line_id".to_string(),
                        Value::String(row.line_id.clone()),
                    );
                    obj.insert(
                        "split_qty".to_string(),
                        Value::Number(serde_json::Number::from(item.qty as i64)),
                    );
                }
                let pay_amount = line_pay_allocs
                    .get(&item.line_id)
                    .and_then(|allocs| allocs.get(child_idx).cloned())
                    .flatten();
                let discount = line_discount_allocs
                    .get(&item.line_id)
                    .and_then(|allocs| allocs.get(child_idx).cloned())
                    .flatten();
                let projection = OrderItemProjection {
                    line_id: uuid::Uuid::new_v4().to_string(),
                    sku_id: row.sku_id.clone(),
                    title: row.title.clone(),
                    qty: item.qty,
                    price_cent: row.price_cent,
                    amount_cent,
                    pay_amount_cent: pay_amount,
                    discount_cent: discount,
                    refund_state: RefundState::from_str(&row.refund_state)
                        .unwrap_or(RefundState::None),
                    after_sale_state: AfterSaleState::from_str(&row.after_sale_state)
                        .unwrap_or(AfterSaleState::None),
                    refund_amount_cent: row.refund_amount_cent,
                    after_sale_type: row.after_sale_type.clone(),
                    after_sale_id: row.after_sale_id.clone(),
                    raw,
                };
                child_goods_amounts[child_idx] += amount_cent;
                child_items[child_idx].push(projection);
            }
        }

        let postage_allocs = allocate_amount(order_row.postage_amount_cent, &child_goods_amounts);
        let discount_allocs = allocate_amount(order_row.discount_amount_cent, &child_goods_amounts);
        let platform_discount_allocs =
            allocate_amount(order_row.platform_discount_cent, &child_goods_amounts);
        let seller_discount_allocs =
            allocate_amount(order_row.seller_discount_cent, &child_goods_amounts);
        let coupon_discount_allocs =
            allocate_amount(order_row.coupon_discount_cent, &child_goods_amounts);
        let paid_allocs = allocate_amount(order_row.paid_amount_cent, &child_goods_amounts);
        let subsidy_allocs = allocate_amount(order_row.subsidy_amount_cent, &child_goods_amounts);
        let receivable_allocs =
            allocate_amount(order_row.receivable_amount_cent, &child_goods_amounts);
        let other_fee_allocs = allocate_amount(order_row.other_fee_cent, &child_goods_amounts);
        let estimated_postage_allocs =
            allocate_amount(order_row.estimated_postage_cent, &child_goods_amounts);

        let mut child_orders = Vec::new();
        let child_exception_flags = exception_flags
            .iter()
            .filter(|flag| *flag != ORDER_SPLIT_FLAG)
            .cloned()
            .collect::<Vec<_>>();

        for idx in 0..child_count {
            let child_order_id = uuid::Uuid::new_v4();
            let platform_order_id = format!("{}-S{}", order_row.platform_order_id, idx + 1);
            let mut raw = order_row.raw.clone();
            if let Some(obj) = raw.as_object_mut() {
                obj.insert(
                    "split_from_order_id".to_string(),
                    Value::String(order_row.order_id.to_string()),
                );
                obj.insert(
                    "split_child_index".to_string(),
                    Value::Number(serde_json::Number::from((idx + 1) as i64)),
                );
            }

            let goods_amount_cent = child_goods_amounts[idx];
            let postage_amount_cent = postage_allocs[idx].unwrap_or(0);
            let discount_amount_cent = discount_allocs[idx].unwrap_or(0);
            let total_amount_cent = goods_amount_cent + postage_amount_cent - discount_amount_cent;

            let exception_flags_json = Value::Array(
                child_exception_flags
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            );

            sqlx::query(
                r#"
                insert into orders(
                    tenant_id, order_month, order_id, platform, shop_id, platform_order_id,
                    status, total_amount_cent, goods_amount_cent, postage_amount_cent, discount_amount_cent,
                    platform_discount_cent, seller_discount_cent, coupon_discount_cent,
                    currency, buyer_name, receiver_name, receiver_phone, receiver_address,
                    customer_id, pay_at, ship_at, finish_at, cancel_at,
                    buyer_message, seller_remark, logistics_company, tracking_no,
                    order_created_at, platform_updated_at, raw,
                    oms_status, review_status, exception_flags, version,
                    inventory_shortage, order_type, sales_channel, order_source, salesperson,
                    ship_warehouse_id, shipment_no, estimated_weight_gram, package_weight_gram,
                    customer_account, receiver_tel, paid_amount_cent, subsidy_amount_cent,
                    receivable_amount_cent, other_fee_cent, estimated_postage_cent,
                    invoice_type, invoice_title, payment_status, freeze_reason, cancel_reason,
                    exception_reason, extra_remark
                ) values (
                    $1,$2,$3,$4,$5,$6,
                    $7,$8,$9,$10,$11,
                    $12,$13,$14,
                    $15,$16,$17,$18,$19,
                    $20,$21,$22,$23,$24,
                    $25,$26,$27,$28,
                    $29,$30,$31,
                    $32,$33,$34,$35,
                    $36,$37,$38,$39,$40,
                    $41,$42,$43,$44,
                    $45,$46,$47,$48,
                    $49,$50,$51,
                    $52,$53,$54,$55,
                    $56,$57
                )
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(order_row.order_month)
            .bind(child_order_id)
            .bind(&order_row.platform)
            .bind(order_row.shop_id)
            .bind(&platform_order_id)
            .bind(&order_row.status)
            .bind(total_amount_cent)
            .bind(goods_amount_cent)
            .bind(postage_amount_cent)
            .bind(discount_amount_cent)
            .bind(platform_discount_allocs[idx])
            .bind(seller_discount_allocs[idx])
            .bind(coupon_discount_allocs[idx])
            .bind(&order_row.currency)
            .bind(&order_row.buyer_name)
            .bind(&order_row.receiver_name)
            .bind(&order_row.receiver_phone)
            .bind(&order_row.receiver_address)
            .bind(order_row.customer_id)
            .bind(order_row.pay_at)
            .bind(order_row.ship_at)
            .bind(order_row.finish_at)
            .bind(order_row.cancel_at)
            .bind(&order_row.buyer_message)
            .bind(&order_row.seller_remark)
            .bind(&order_row.logistics_company)
            .bind(&order_row.tracking_no)
            .bind(order_row.order_created_at)
            .bind(order_row.platform_updated_at)
            .bind(&raw)
            .bind(&order_row.oms_status)
            .bind(&order_row.review_status)
            .bind(exception_flags_json)
            .bind(0i32)
            .bind(order_row.inventory_shortage)
            .bind(&order_row.order_type)
            .bind(&order_row.sales_channel)
            .bind(&order_row.order_source)
            .bind(&order_row.salesperson)
            .bind(order_row.ship_warehouse_id)
            .bind(&order_row.shipment_no)
            .bind(order_row.estimated_weight_gram)
            .bind(order_row.package_weight_gram)
            .bind(&order_row.customer_account)
            .bind(&order_row.receiver_tel)
            .bind(paid_allocs[idx])
            .bind(subsidy_allocs[idx])
            .bind(receivable_allocs[idx])
            .bind(other_fee_allocs[idx])
            .bind(estimated_postage_allocs[idx])
            .bind(&order_row.invoice_type)
            .bind(&order_row.invoice_title)
            .bind(&order_row.payment_status)
            .bind(&order_row.freeze_reason)
            .bind(&order_row.cancel_reason)
            .bind(&order_row.exception_reason)
            .bind(&order_row.extra_remark)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                insert into order_index(
                    tenant_id, order_id, order_month, platform, platform_order_id, shop_id, order_created_at
                ) values ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(child_order_id)
            .bind(order_row.order_month)
            .bind(&order_row.platform)
            .bind(&platform_order_id)
            .bind(order_row.shop_id)
            .bind(order_row.order_created_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                insert into order_key_index(
                    tenant_id, platform, platform_order_id, order_id, order_month, shop_id, order_created_at
                ) values ($1,$2,$3,$4,$5,$6,$7)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(&order_row.platform)
            .bind(&platform_order_id)
            .bind(child_order_id)
            .bind(order_row.order_month)
            .bind(order_row.shop_id)
            .bind(order_row.order_created_at)
            .execute(&mut *tx)
            .await?;

            let mut result_items = Vec::new();
            for item in &child_items[idx] {
                self.insert_item(
                    &mut tx,
                    tenant_id,
                    order_row.order_month,
                    OrderId(child_order_id),
                    item,
                )
                .await?;
                result_items.push(OrderSplitResultItem {
                    line_id: item.line_id.clone(),
                    sku_id: item.sku_id.clone(),
                    title: item.title.clone(),
                    qty: item.qty,
                    price_cent: item.price_cent,
                });
            }

            child_orders.push(OrderSplitResultChild {
                order_id: OrderId(child_order_id),
                items: result_items,
            });
        }

        let mut next_flags = exception_flags.clone();
        if !next_flags.iter().any(|flag| flag == ORDER_SPLIT_FLAG) {
            next_flags.push(ORDER_SPLIT_FLAG.to_string());
        }

        sqlx::query(
            r#"
            update orders o
            set oms_status = $3,
                order_status = 'CANCELLED',
                review_status = $4,
                fulfillment_status = 'NOT_ALLOCATED',
                exception_status = 'NONE',
                exception_flags = $5,
                cancel_reason = $6,
                cancel_at = now(),
                version = o.version + 1,
                updated_at = now()
            from order_index i
            where o.tenant_id = i.tenant_id
              and o.order_id = i.order_id
              and o.order_month = i.order_month
              and i.tenant_id = $1
              and i.order_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .bind(OmsOrderStatus::Cancelled.to_string())
        .bind(OrderReviewStatus::Rejected.to_string())
        .bind(Value::Array(
            next_flags
                .iter()
                .cloned()
                .map(Value::String)
                .collect::<Vec<_>>(),
        ))
        .bind(Some("订单拆分".to_string()))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(OrderSplitResult {
            source_order_id: OrderId(order_row.order_id),
            child_orders,
        })
    }
}

fn parse_order_status(raw: &str) -> OrderStatus {
    match raw.to_uppercase().as_str() {
        "PENDING_PAYMENT" => OrderStatus::PendingPayment,
        "PENDING_SHIPMENT" => OrderStatus::PendingShipment,
        "ALLOCATED" => OrderStatus::Allocated,
        "PARTIALLY_SHIPPED" => OrderStatus::PartiallyShipped,
        "SHIPPED" => OrderStatus::Shipped,
        "COMPLETED" => OrderStatus::Completed,
        "CANCELLED" => OrderStatus::Cancelled,
        "CLOSED" => OrderStatus::Closed,
        "REFUNDING" => OrderStatus::Refunding,
        "REFUNDED" => OrderStatus::Refunded,
        "ON_HOLD" => OrderStatus::OnHold,
        _ => OrderStatus::Unknown,
    }
}

fn parse_oms_status(raw: &str) -> OmsOrderStatus {
    OmsOrderStatus::from_str(raw).unwrap_or(OmsOrderStatus::New)
}

fn parse_review_status(raw: &str) -> OrderReviewStatus {
    OrderReviewStatus::from_str(raw).unwrap_or(OrderReviewStatus::NotReviewed)
}

fn parse_order_main_status(raw: &str) -> OrderMainStatus {
    OrderMainStatus::from_str(raw).unwrap_or(OrderMainStatus::PendingReview)
}

fn parse_fulfillment_status(raw: &str) -> OrderFulfillmentStatus {
    OrderFulfillmentStatus::from_str(raw).unwrap_or(OrderFulfillmentStatus::NotAllocated)
}

fn parse_exception_status(raw: &str) -> OrderExceptionStatus {
    OrderExceptionStatus::from_str(raw).unwrap_or(OrderExceptionStatus::None)
}

fn parse_payment_status(raw: &str) -> Result<OrderPaymentStatus, String> {
    OrderPaymentStatus::from_str(raw)
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

fn apply_order_filters(
    qb: &mut QueryBuilder<'_, Postgres>,
    tenant_id: TenantId,
    filters: &OmsOrderListFilters,
) {
    qb.push(" where o.tenant_id = ");
    qb.push_bind(uuid::Uuid::from(tenant_id));

    if let Some(platforms) = &filters.platforms {
        if !platforms.is_empty() {
            let values: Vec<String> = platforms.iter().map(|value| value.to_string()).collect();
            qb.push(" and o.platform = any(");
            qb.push_bind(values);
            qb.push(")");
        }
    } else if let Some(platform) = filters.platform {
        qb.push(" and o.platform = ");
        qb.push_bind(platform.to_string());
    }
    if let Some(shop_id) = filters.shop_id {
        qb.push(" and o.shop_id = ");
        qb.push_bind(uuid::Uuid::from(shop_id));
    }
    if let Some(shop_ids_filter) = &filters.shop_ids_filter {
        if !shop_ids_filter.is_empty() {
            let ids: Vec<uuid::Uuid> = shop_ids_filter
                .iter()
                .map(|id| uuid::Uuid::from(*id))
                .collect();
            qb.push(" and o.shop_id = any(");
            qb.push_bind(ids);
            qb.push(")");
        }
    }
    if let Some(shop_ids) = &filters.shop_ids {
        if !shop_ids.is_empty() {
            let ids: Vec<uuid::Uuid> = shop_ids.iter().map(|id| uuid::Uuid::from(*id)).collect();
            qb.push(" and o.shop_id = any(");
            qb.push_bind(ids);
            qb.push(")");
        }
    }
    if let Some(oms_statuses) = &filters.oms_statuses {
        if !oms_statuses.is_empty() {
            let values: Vec<String> = oms_statuses.iter().map(|value| value.to_string()).collect();
            qb.push(" and o.oms_status = any(");
            qb.push_bind(values);
            qb.push(")");
        }
    } else if let Some(oms_status) = filters.oms_status {
        qb.push(" and o.oms_status = ");
        qb.push_bind(oms_status.to_string());
    }
    if let Some(order_status) = filters.order_status {
        qb.push(" and o.order_status = ");
        qb.push_bind(order_status.to_string());
    }
    if let Some(review_status) = filters.review_status {
        qb.push(" and o.review_status = ");
        qb.push_bind(review_status.to_string());
    }
    if let Some(fulfillment_status) = filters.fulfillment_status {
        qb.push(" and o.fulfillment_status = ");
        qb.push_bind(fulfillment_status.to_string());
    }
    if let Some(exception_status) = filters.exception_status {
        qb.push(" and o.exception_status = ");
        qb.push_bind(exception_status.to_string());
    }
    if let Some(payment_status) = filters.payment_status {
        qb.push(" and o.payment_status = ");
        qb.push_bind(payment_status.to_string());
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and o.order_created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and o.order_created_at <= ");
        qb.push_bind(created_to);
    }
    if let Some(updated_from) = filters.updated_from {
        qb.push(" and o.updated_at >= ");
        qb.push_bind(updated_from);
    }
    if let Some(updated_to) = filters.updated_to {
        qb.push(" and o.updated_at <= ");
        qb.push_bind(updated_to);
    }
    if let Some(ship_warehouse_ids) = &filters.ship_warehouse_ids {
        if !ship_warehouse_ids.is_empty() {
            qb.push(" and o.ship_warehouse_id = any(");
            qb.push_bind(ship_warehouse_ids.clone());
            qb.push(")");
        }
    } else if let Some(ship_warehouse_id) = filters.ship_warehouse_id {
        qb.push(" and o.ship_warehouse_id = ");
        qb.push_bind(ship_warehouse_id);
    }
    if let Some(platform_order_id) = filters.platform_order_id.as_deref() {
        let platform_order_id = platform_order_id.trim();
        if !platform_order_id.is_empty() {
            let pattern = format!("%{platform_order_id}%");
            qb.push(" and o.platform_order_id ilike ");
            qb.push_bind(pattern);
        }
    }
    if let Some(item_keyword) = filters.item_keyword.as_deref() {
        let item_keyword = item_keyword.trim();
        if !item_keyword.is_empty() {
            let pattern = format!("%{item_keyword}%");
            qb.push(" and exists (select 1 from order_items oi");
            qb.push(" where oi.tenant_id = o.tenant_id");
            qb.push(" and oi.order_id = o.order_id");
            qb.push(" and oi.order_month = o.order_month");
            qb.push(" and (oi.title ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or oi.sku_id ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or oi.line_id ilike ");
            qb.push_bind(pattern);
            qb.push("))");
        }
    }
    if let Some(total_amount_cent) = filters.total_amount_cent {
        qb.push(" and o.total_amount_cent = ");
        qb.push_bind(total_amount_cent);
    }
    if let Some(goods_qty) = filters.goods_qty {
        qb.push(" and (select coalesce(sum(oi.qty), 0) from order_items oi");
        qb.push(" where oi.tenant_id = o.tenant_id and oi.order_id = o.order_id");
        qb.push(" and oi.order_month = o.order_month) = ");
        qb.push_bind(goods_qty);
    }
    if let Some(customer_account) = filters.customer_account.as_deref() {
        let customer_account = customer_account.trim();
        if !customer_account.is_empty() {
            let pattern = format!("%{customer_account}%");
            qb.push(" and (coalesce(o.customer_account, '') ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or coalesce(o.customer_id::text, '') ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or exists (select 1 from customers c where c.tenant_id = o.tenant_id");
            qb.push(" and c.customer_id = o.customer_id and c.name ilike ");
            qb.push_bind(pattern);
            qb.push("))");
        }
    }
    if let Some(receiver_name) = filters.receiver_name.as_deref() {
        let receiver_name = receiver_name.trim();
        if !receiver_name.is_empty() {
            let pattern = format!("%{receiver_name}%");
            qb.push(" and coalesce(o.receiver_name, '') ilike ");
            qb.push_bind(pattern);
        }
    }
    if let Some(receiver_phone) = filters.receiver_phone.as_deref() {
        let receiver_phone = receiver_phone.trim();
        if !receiver_phone.is_empty() {
            let pattern = format!("%{receiver_phone}%");
            qb.push(" and coalesce(o.receiver_phone, '') ilike ");
            qb.push_bind(pattern);
        }
    }
    if let Some(receiver_address) = filters.receiver_address.as_deref() {
        let receiver_address = receiver_address.trim();
        if !receiver_address.is_empty() {
            let pattern = format!("%{receiver_address}%");
            qb.push(" and coalesce(o.receiver_address, '') ilike ");
            qb.push_bind(pattern);
        }
    }
    if let Some(remark) = filters.remark.as_deref() {
        let remark = remark.trim();
        if !remark.is_empty() {
            let pattern = format!("%{remark}%");
            qb.push(" and (coalesce(o.buyer_message, '') ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or coalesce(o.seller_remark, '') ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or coalesce(o.extra_remark, '') ilike ");
            qb.push_bind(pattern);
            qb.push(")");
        }
    }
    if let Some(order_source) = filters.order_source.as_deref() {
        let order_source = order_source.trim();
        if !order_source.is_empty() {
            let pattern = format!("%{order_source}%");
            qb.push(" and coalesce(o.order_source, '') ilike ");
            qb.push_bind(pattern);
        }
    }
    if let Some(logistics_company) = filters.logistics_company.as_deref() {
        let logistics_company = logistics_company.trim();
        if !logistics_company.is_empty() {
            let pattern = format!("%{logistics_company}%");
            qb.push(" and coalesce(o.logistics_company, '') ilike ");
            qb.push_bind(pattern);
        }
    }
    if let Some(estimated_postage_cent) = filters.estimated_postage_cent {
        qb.push(" and coalesce(o.estimated_postage_cent, o.postage_amount_cent, 0) = ");
        qb.push_bind(estimated_postage_cent);
    }
    if let Some(estimated_profit_cent) = filters.estimated_profit_cent {
        qb.push(
            " and (o.total_amount_cent - coalesce(o.estimated_postage_cent, o.postage_amount_cent, 0)) = ",
        );
        qb.push_bind(estimated_profit_cent);
    }
    if let Some(estimated_profit_rate) = filters.estimated_profit_rate {
        qb.push(
            " and round((o.total_amount_cent - coalesce(o.estimated_postage_cent, o.postage_amount_cent, 0))::numeric * 100 / nullif(o.total_amount_cent, 0), 2) = ",
        );
        qb.push_bind(estimated_profit_rate);
    }
    if !filters.exception_flags.is_empty() {
        qb.push(" and o.exception_flags ?| ");
        qb.push_bind(filters.exception_flags.clone());
    }
    if !filters.tags.is_empty() {
        qb.push(" and exists (select 1 from order_tags ot where ot.tenant_id = o.tenant_id");
        qb.push(" and ot.order_id = o.order_id and ot.tag = any(");
        qb.push_bind(filters.tags.clone());
        qb.push("))");
    }
    if let Some(tagged) = filters.tagged {
        if tagged {
            qb.push(" and exists (select 1 from order_tags ot where ot.tenant_id = o.tenant_id");
            qb.push(" and ot.order_id = o.order_id)");
        } else {
            qb.push(
                " and not exists (select 1 from order_tags ot where ot.tenant_id = o.tenant_id",
            );
            qb.push(" and ot.order_id = o.order_id)");
        }
    }
    if let Some(keyword) = filters.keyword.as_deref() {
        let keyword = keyword.trim();
        if !keyword.is_empty() {
            let pattern = format!("%{keyword}%");
            qb.push(" and (o.platform_order_id ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or o.receiver_name ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or o.receiver_phone ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or o.tracking_no ilike ");
            qb.push_bind(pattern.clone());
            qb.push(" or o.order_id::text ilike ");
            qb.push_bind(pattern);
            qb.push(")");
        }
    }
}

fn non_empty_trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
}

fn normalize_phone(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
}

async fn resolve_customer_id(
    tx: &mut Transaction<'_, Postgres>,
    order: &OrderProjection,
) -> anyhow::Result<Option<uuid::Uuid>> {
    let name = non_empty_trimmed(order.buyer_name.as_deref())
        .or_else(|| non_empty_trimmed(order.receiver_name.as_deref()));
    let phone = order
        .receiver_phone
        .as_deref()
        .map(normalize_phone)
        .and_then(|value| non_empty_trimmed(Some(&value)));
    let address = non_empty_trimmed(order.receiver_address.as_deref());
    let note = non_empty_trimmed(order.buyer_message.as_deref());

    if name.is_none() && phone.is_none() {
        return Ok(None);
    }

    if let Some(phone_value) = phone.as_deref() {
        let existing: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select customer_id
            from customers
            where tenant_id = $1 and phone = $2 and status = 'ACTIVE'
            "#,
        )
        .bind(uuid::Uuid::from(order.tenant_id))
        .bind(phone_value)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some(customer_id) = existing {
            if name.is_some() || address.is_some() || note.is_some() {
                sqlx::query(
                    r#"
                    update customers
                    set name = coalesce($3, name),
                        address = coalesce($4, address),
                        note = coalesce($5, note),
                        updated_at = now()
                    where tenant_id = $1 and customer_id = $2
                    "#,
                )
                .bind(uuid::Uuid::from(order.tenant_id))
                .bind(customer_id)
                .bind(&name)
                .bind(&address)
                .bind(&note)
                .execute(&mut **tx)
                .await?;
            }

            return Ok(Some(customer_id));
        }
    }

    let fallback_name = name
        .clone()
        .or_else(|| phone.clone())
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let inserted: Option<uuid::Uuid> = sqlx::query_scalar(
        r#"
        insert into customers(tenant_id, name, phone, address, note, status)
        values ($1, $2, $3, $4, $5, 'ACTIVE')
        on conflict (tenant_id, phone)
        do nothing
        returning customer_id
        "#,
    )
    .bind(uuid::Uuid::from(order.tenant_id))
    .bind(fallback_name)
    .bind(&phone)
    .bind(&address)
    .bind(&note)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(inserted)
}

fn order_item_uuid(order_id: uuid::Uuid, line_id: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(line_id)
        .unwrap_or_else(|_| uuid::Uuid::new_v5(&order_id, line_id.as_bytes()))
}
