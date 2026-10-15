use super::models::*;
use anyhow::{anyhow, bail};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use erp_types::{OrderId, TenantId};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use std::collections::HashMap;

use crate::inventory::{InventoryDelta, InventoryService};

const ORDER_COGS_REF_OUTBOUND_DOC: &str = "OUTBOUND_DOC";

pub struct WmsStore<'a> {
    pool: &'a PgPool,
}

impl<'a> WmsStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_outbound_docs(
        &self,
        tenant_id: TenantId,
        filters: OutboundDocListFilters,
    ) -> anyhow::Result<OutboundDocListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from outbound_docs d
            left join shipments s
              on s.tenant_id = d.tenant_id
             and s.shipment_id = d.shipment_id
            left join order_index i
              on i.tenant_id = s.tenant_id
             and i.order_id = s.order_id
            left join orders o
              on o.tenant_id = i.tenant_id
             and o.order_id = i.order_id
             and o.order_month = i.order_month
            where d.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_outbound_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                d.outbound_doc_id,
                d.warehouse_id,
                d.doc_type,
                d.shipment_id,
                d.doc_no,
                d.status,
                d.outbound_status,
                d.pick_status,
                d.verify_status,
                d.pack_status,
                d.ship_status,
                d.picked_at,
                d.packed_at,
                d.shipped_at,
                d.created_at,
                d.updated_at,
                s.order_id,
                s.platform,
                s.platform_order_id,
                s.shop_id,
                s.carrier_code,
                s.tracking_no,
                o.receiver_name,
                o.receiver_phone,
                o.receiver_address,
                case
                    when o.inventory_shortage then o.exception_flags || '["INVENTORY_SHORTAGE"]'::jsonb
                    else o.exception_flags
                end as exception_flags,
                tags.tags
            from outbound_docs d
            left join shipments s
              on s.tenant_id = d.tenant_id
             and s.shipment_id = d.shipment_id
            left join order_index i
              on i.tenant_id = s.tenant_id
             and i.order_id = s.order_id
            left join orders o
              on o.tenant_id = i.tenant_id
             and o.order_id = i.order_id
             and o.order_month = i.order_month
            left join lateral (
                select array_agg(tag) as tags
                from order_tags ot
                where ot.tenant_id = s.tenant_id
                  and ot.order_id = s.order_id
            ) tags on true
            where d.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_outbound_filters(&mut qb, &filters);
        qb.push(" order by d.created_at desc, d.outbound_doc_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            outbound_doc_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            doc_type: String,
            shipment_id: Option<uuid::Uuid>,
            doc_no: String,
            status: String,
            outbound_status: String,
            pick_status: String,
            verify_status: String,
            pack_status: String,
            ship_status: String,
            picked_at: Option<DateTime<Utc>>,
            packed_at: Option<DateTime<Utc>>,
            shipped_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            order_id: Option<uuid::Uuid>,
            platform: Option<String>,
            platform_order_id: Option<String>,
            shop_id: Option<uuid::Uuid>,
            carrier_code: Option<String>,
            tracking_no: Option<String>,
            receiver_name: Option<String>,
            receiver_phone: Option<String>,
            receiver_address: Option<String>,
            exception_flags: Option<Value>,
            tags: Option<Vec<String>>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| OutboundDoc {
                outbound_doc_id: row.outbound_doc_id,
                tenant_id,
                warehouse_id: row.warehouse_id,
                doc_type: row.doc_type,
                shipment_id: row.shipment_id,
                order_id: row.order_id,
                platform: row.platform,
                platform_order_id: row.platform_order_id,
                shop_id: row.shop_id,
                receiver_name: row.receiver_name,
                receiver_phone: row.receiver_phone,
                receiver_address: row.receiver_address,
                carrier_code: row.carrier_code,
                tracking_no: row.tracking_no,
                exception_flags: row
                    .exception_flags
                    .as_ref()
                    .map(json_array_to_vec)
                    .unwrap_or_default(),
                tags: row.tags.unwrap_or_default(),
                doc_no: row.doc_no,
                status: row.status,
                outbound_status: row.outbound_status,
                pick_status: row.pick_status,
                verify_status: row.verify_status,
                pack_status: row.pack_status,
                ship_status: row.ship_status,
                picked_at: row.picked_at,
                packed_at: row.packed_at,
                shipped_at: row.shipped_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(OutboundDocListResult { items, total })
    }

    pub async fn get_outbound_doc(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<Option<OutboundDocDetail>> {
        let doc = match self.fetch_outbound_doc(tenant_id, outbound_doc_id).await? {
            Some(doc) => doc,
            None => return Ok(None),
        };

        let items = self
            .fetch_outbound_items(tenant_id, outbound_doc_id)
            .await?;
        let pick_tasks = self.fetch_pick_tasks(tenant_id, outbound_doc_id).await?;
        let pack_records = self.fetch_pack_records(tenant_id, outbound_doc_id).await?;

        Ok(Some(OutboundDocDetail {
            doc,
            items,
            pick_tasks,
            pack_records,
        }))
    }

    pub async fn audit_outbound_doc(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<OutboundDocDetail> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct DocRow {
            status: String,
        }

        let doc: DocRow = sqlx::query_as(
            r#"
            select status
            from outbound_docs
            where tenant_id = $1 and outbound_doc_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("outbound doc not found"))?;

        if doc.status == "CANCELLED" {
            return Err(anyhow!("outbound doc cancelled"));
        }
        if doc.status == "SHIPPED" {
            return Err(anyhow!("outbound doc shipped"));
        }
        if doc.status == "AUDITED" {
            tx.commit().await?;
            return self
                .get_outbound_doc(tenant_id, outbound_doc_id)
                .await?
                .ok_or_else(|| anyhow!("outbound doc not found"));
        }
        if doc.status != "DRAFT" {
            return Err(anyhow!("outbound doc not auditable"));
        }

        sqlx::query(
            r#"
            update outbound_docs
            set status = 'AUDITED',
                outbound_status = 'PENDING_ALLOCATE',
                pick_status = 'NOT_PICKED',
                verify_status = 'NOT_VERIFIED',
                pack_status = 'NOT_PACKED',
                ship_status = 'NOT_SHIPPED',
                updated_at = now()
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_outbound_doc(tenant_id, outbound_doc_id)
            .await?
            .ok_or_else(|| anyhow!("outbound doc not found"))
    }

    pub async fn create_outbound_doc(
        &self,
        input: OutboundDocCreateInput,
    ) -> anyhow::Result<OutboundDocDetail> {
        if input.items.is_empty() {
            return Err(anyhow!("outbound items required"));
        }

        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let inserted: Option<OutboundDocRow> = sqlx::query_as(
            r#"
            insert into outbound_docs(
                tenant_id, warehouse_id, doc_type, shipment_id, doc_no, status,
                outbound_status, pick_status, verify_status, pack_status, ship_status
            ) values ($1,$2,$3,$4,$5,'DRAFT','DRAFT','NOT_PICKED','NOT_VERIFIED','NOT_PACKED','NOT_SHIPPED')
            on conflict do nothing
            returning
                outbound_doc_id, warehouse_id, doc_type
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(&input.doc_type)
        .bind(input.shipment_id)
        .bind(&input.doc_no)
        .fetch_optional(&mut *tx)
        .await?;

        let inserted_new = inserted.is_some();
        let doc_row = if let Some(row) = inserted {
            row
        } else {
            let mut qb = QueryBuilder::<Postgres>::new(
                r#"
                select
                    outbound_doc_id, warehouse_id, doc_type
                from outbound_docs
                where tenant_id =
                "#,
            );
            qb.push_bind(uuid::Uuid::from(input.tenant_id));
            qb.push(" and (doc_no = ");
            qb.push_bind(&input.doc_no);
            if let Some(shipment_id) = input.shipment_id {
                qb.push(" or shipment_id = ");
                qb.push_bind(shipment_id);
            }
            qb.push(")");

            let row: OutboundDocRow = qb
                .build_query_as()
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| anyhow!("outbound doc conflict"))?;

            if row.warehouse_id != input.warehouse_id || row.doc_type != input.doc_type {
                return Err(anyhow!("outbound doc conflict"));
            }
            row
        };

        if inserted_new {
            let mut items_qb = QueryBuilder::<Postgres>::new(
                r#"
                insert into outbound_doc_items(
                    tenant_id, outbound_doc_id, sku_id, planned_qty, picked_qty, packed_qty
                )
                "#,
            );
            items_qb.push_values(input.items.iter(), |mut b, item| {
                b.push_bind(uuid::Uuid::from(input.tenant_id))
                    .push_bind(doc_row.outbound_doc_id)
                    .push_bind(item.sku_id)
                    .push_bind(item.planned_qty)
                    .push_bind(0_i64)
                    .push_bind(0_i64);
            });
            items_qb.build().execute(&mut *tx).await?;
        } else {
            let existing_items = self
                .fetch_outbound_items_with_tx(&mut tx, input.tenant_id, doc_row.outbound_doc_id)
                .await?;
            if existing_items.len() != input.items.len() {
                return Err(anyhow!("outbound items conflict"));
            }
            for item in &input.items {
                let Some(existing) = existing_items.iter().find(|row| row.sku_id == item.sku_id)
                else {
                    return Err(anyhow!("outbound items conflict"));
                };
                if existing.planned_qty != item.planned_qty {
                    return Err(anyhow!("outbound items conflict"));
                }
            }
        }

        tx.commit().await?;
        self.get_outbound_doc(input.tenant_id, doc_row.outbound_doc_id)
            .await?
            .ok_or_else(|| anyhow!("outbound doc not found"))
    }

    pub async fn create_pick_tasks_for_outbound(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
        auto_pick: bool,
    ) -> anyhow::Result<Vec<OutboundPickTask>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let existing: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from outbound_pick_tasks
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_one(&mut *tx)
        .await?;

        if existing > 0 {
            tx.commit().await?;
            return self.fetch_pick_tasks(tenant_id, outbound_doc_id).await;
        }

        let (status, outbound_status, pick_status, picked_at) = if auto_pick {
            ("PICKED", "PENDING_VERIFY", "PICKED", Some(Utc::now()))
        } else {
            ("PICKING", "PICKING", "PICKING", None)
        };
        sqlx::query(
            r#"
            update outbound_docs
            set status = $3,
                outbound_status = $4,
                pick_status = $5,
                verify_status = 'NOT_VERIFIED',
                pack_status = 'NOT_PACKED',
                ship_status = 'NOT_SHIPPED',
                picked_at = coalesce($6, picked_at),
                updated_at = now()
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .bind(status)
        .bind(outbound_status)
        .bind(pick_status)
        .bind(picked_at)
        .execute(&mut *tx)
        .await?;

        #[derive(sqlx::FromRow)]
        struct ItemRow {
            sku_id: uuid::Uuid,
            planned_qty: i64,
        }

        let items: Vec<ItemRow> = sqlx::query_as(
            r#"
            select sku_id, planned_qty
            from outbound_doc_items
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_all(&mut *tx)
        .await?;

        if items.is_empty() {
            return Err(anyhow!("outbound items missing"));
        }

        let mut pick_qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into outbound_pick_tasks(
                tenant_id, outbound_doc_id, sku_id, qty, picked_qty, status
            )
            "#,
        );
        pick_qb.push_values(items.iter(), |mut b, row| {
            let picked_qty = if auto_pick { row.planned_qty } else { 0_i64 };
            let status = if auto_pick { "PICKED" } else { "NOT_PICKED" };
            b.push_bind(uuid::Uuid::from(tenant_id))
                .push_bind(outbound_doc_id)
                .push_bind(row.sku_id)
                .push_bind(row.planned_qty)
                .push_bind(picked_qty)
                .push_bind(status);
        });
        pick_qb.build().execute(&mut *tx).await?;

        if auto_pick {
            sqlx::query(
                r#"
                update outbound_doc_items
                set picked_qty = planned_qty,
                    updated_at = now()
                where tenant_id = $1 and outbound_doc_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(outbound_doc_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.fetch_pick_tasks(tenant_id, outbound_doc_id).await
    }

    pub async fn list_waves(
        &self,
        tenant_id: TenantId,
        filters: WaveListFilters,
    ) -> anyhow::Result<WaveListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from waves
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_wave_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select wave_id, warehouse_id, status, created_at, updated_at
            from waves
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_wave_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, wave_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            wave_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| Wave {
                wave_id: row.wave_id,
                tenant_id,
                warehouse_id: row.warehouse_id,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(WaveListResult { items, total })
    }

    pub async fn get_wave(
        &self,
        tenant_id: TenantId,
        wave_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WaveDetail>> {
        let wave = match self.fetch_wave(tenant_id, wave_id).await? {
            Some(wave) => wave,
            None => return Ok(None),
        };

        let docs = self.fetch_wave_docs(tenant_id, wave_id).await?;
        let tasks = self.fetch_wave_tasks(tenant_id, wave_id).await?;

        Ok(Some(WaveDetail { wave, docs, tasks }))
    }

    pub async fn list_wave_rules(
        &self,
        tenant_id: TenantId,
        filters: WaveRuleListFilters,
    ) -> anyhow::Result<WaveRuleListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from wave_rules
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_wave_rule_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                rule_id, warehouse_id, carrier_code, max_orders, max_qty,
                cutoff_time, strategy, enabled, created_at, updated_at
            from wave_rules
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_wave_rule_filters(&mut qb, &filters);
        qb.push(" order by created_at desc, rule_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            carrier_code: Option<String>,
            max_orders: Option<i32>,
            max_qty: Option<i64>,
            cutoff_time: Option<chrono::NaiveTime>,
            strategy: Option<String>,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| WaveRule {
                rule_id: row.rule_id,
                tenant_id,
                warehouse_id: row.warehouse_id,
                carrier_code: row.carrier_code,
                max_orders: row.max_orders,
                max_qty: row.max_qty,
                cutoff_time: row.cutoff_time,
                strategy: row.strategy,
                enabled: row.enabled,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(WaveRuleListResult { items, total })
    }

    pub async fn get_wave_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<Option<WaveRule>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            carrier_code: Option<String>,
            max_orders: Option<i32>,
            max_qty: Option<i64>,
            cutoff_time: Option<chrono::NaiveTime>,
            strategy: Option<String>,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select
                rule_id, warehouse_id, carrier_code, max_orders, max_qty,
                cutoff_time, strategy, enabled, created_at, updated_at
            from wave_rules
            where tenant_id = $1 and rule_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| WaveRule {
            rule_id: row.rule_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            carrier_code: row.carrier_code,
            max_orders: row.max_orders,
            max_qty: row.max_qty,
            cutoff_time: row.cutoff_time,
            strategy: row.strategy,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn create_wave_rule(&self, input: WaveRuleCreateInput) -> anyhow::Result<WaveRule> {
        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            carrier_code: Option<String>,
            max_orders: Option<i32>,
            max_qty: Option<i64>,
            cutoff_time: Option<chrono::NaiveTime>,
            strategy: Option<String>,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into wave_rules(
                tenant_id, warehouse_id, carrier_code, max_orders, max_qty,
                cutoff_time, strategy, enabled
            ) values ($1,$2,$3,$4,$5,$6,$7,$8)
            returning
                rule_id, warehouse_id, carrier_code, max_orders, max_qty,
                cutoff_time, strategy, enabled, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .bind(input.carrier_code.as_deref())
        .bind(input.max_orders)
        .bind(input.max_qty)
        .bind(input.cutoff_time)
        .bind(input.strategy.as_deref())
        .bind(input.enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(WaveRule {
            rule_id: row.rule_id,
            tenant_id: input.tenant_id,
            warehouse_id: row.warehouse_id,
            carrier_code: row.carrier_code,
            max_orders: row.max_orders,
            max_qty: row.max_qty,
            cutoff_time: row.cutoff_time,
            strategy: row.strategy,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_wave_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
        input: WaveRuleUpdateInput,
    ) -> anyhow::Result<Option<WaveRule>> {
        let mut qb = QueryBuilder::<Postgres>::new("update wave_rules set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(carrier_code) = input.carrier_code {
            separated.push("carrier_code = ");
            separated.push_bind(carrier_code);
            has_updates = true;
        }
        if let Some(max_orders) = input.max_orders {
            separated.push("max_orders = ");
            separated.push_bind(max_orders);
            has_updates = true;
        }
        if let Some(max_qty) = input.max_qty {
            separated.push("max_qty = ");
            separated.push_bind(max_qty);
            has_updates = true;
        }
        if let Some(cutoff_time) = input.cutoff_time {
            separated.push("cutoff_time = ");
            separated.push_bind(cutoff_time);
            has_updates = true;
        }
        if let Some(strategy) = input.strategy {
            separated.push("strategy = ");
            separated.push_bind(strategy);
            has_updates = true;
        }
        if let Some(enabled) = input.enabled {
            separated.push("enabled = ");
            separated.push_bind(enabled);
            has_updates = true;
        }

        if !has_updates {
            return self.get_wave_rule(tenant_id, rule_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and rule_id = ");
        qb.push_bind(rule_id);
        qb.push(
            r#"
            returning
                rule_id, warehouse_id, carrier_code, max_orders, max_qty,
                cutoff_time, strategy, enabled, created_at, updated_at
            "#,
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            rule_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            carrier_code: Option<String>,
            max_orders: Option<i32>,
            max_qty: Option<i64>,
            cutoff_time: Option<chrono::NaiveTime>,
            strategy: Option<String>,
            enabled: bool,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| WaveRule {
            rule_id: row.rule_id,
            tenant_id,
            warehouse_id: row.warehouse_id,
            carrier_code: row.carrier_code,
            max_orders: row.max_orders,
            max_qty: row.max_qty,
            cutoff_time: row.cutoff_time,
            strategy: row.strategy,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn delete_wave_rule(
        &self,
        tenant_id: TenantId,
        rule_id: uuid::Uuid,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            r#"
            delete from wave_rules
            where tenant_id = $1 and rule_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(rule_id)
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn count_available_outbounds(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        status: &str,
        cutoff_at: DateTime<Utc>,
    ) -> anyhow::Result<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from outbound_docs
            where tenant_id = $1
              and warehouse_id = $2
              and status = $3
              and created_at <= $4
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(status)
        .bind(cutoff_at)
        .fetch_one(self.pool)
        .await?;
        Ok(count)
    }

    pub async fn list_wave_candidates(
        &self,
        tenant_id: TenantId,
        warehouse_id: uuid::Uuid,
        status: &str,
        cutoff_at: DateTime<Utc>,
        carrier_code: Option<&str>,
        limit: i64,
    ) -> anyhow::Result<Vec<WaveCandidate>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            outbound_doc_id: uuid::Uuid,
            total_qty: Option<i64>,
            created_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select
                d.outbound_doc_id,
                sum(i.planned_qty)::bigint as total_qty,
                d.created_at
            from outbound_docs d
            join outbound_doc_items i
              on i.tenant_id = d.tenant_id
             and i.outbound_doc_id = d.outbound_doc_id
            left join shipments s
              on s.tenant_id = d.tenant_id
             and s.shipment_id = d.shipment_id
            where d.tenant_id = $1
              and d.warehouse_id = $2
              and d.status = $3
              and d.created_at <= $4
              and ($5::text is null or s.carrier_code = $5)
            group by d.outbound_doc_id, d.created_at
            order by d.created_at asc, d.outbound_doc_id asc
            limit $6
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(warehouse_id)
        .bind(status)
        .bind(cutoff_at)
        .bind(carrier_code)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| WaveCandidate {
                outbound_doc_id: row.outbound_doc_id,
                total_qty: row.total_qty.unwrap_or(0),
                created_at: row.created_at,
            })
            .collect())
    }

    pub async fn create_wave(&self, input: WaveCreateInput) -> anyhow::Result<WaveDetail> {
        self.create_wave_with_status(input, "DRAFT").await
    }

    pub async fn create_wave_with_status(
        &self,
        input: WaveCreateInput,
        status: &str,
    ) -> anyhow::Result<WaveDetail> {
        if input
            .outbound_doc_ids
            .as_ref()
            .is_some_and(|ids| ids.is_empty())
        {
            bail!("outbound_doc_ids required");
        }
        if input.outbound_doc_ids.is_none() && input.max_docs <= 0 {
            bail!("max_docs must be positive");
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct DocRow {
            outbound_doc_id: uuid::Uuid,
        }

        let docs: Vec<DocRow> = if let Some(doc_ids) = &input.outbound_doc_ids {
            sqlx::query_as(
                r#"
                select outbound_doc_id
                from outbound_docs
                where tenant_id = $1
                  and warehouse_id = $2
                  and outbound_doc_id = any($3)
                  and status = $4
                order by created_at asc, outbound_doc_id asc
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.warehouse_id)
            .bind(doc_ids)
            .bind(status)
            .fetch_all(&mut *tx)
            .await?
        } else {
            sqlx::query_as(
                r#"
                select outbound_doc_id
                from outbound_docs
                where tenant_id = $1
                  and warehouse_id = $2
                  and status = $3
                  and created_at <= $4
                order by created_at asc, outbound_doc_id asc
                limit $5
                for update skip locked
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.warehouse_id)
            .bind(status)
            .bind(input.cutoff_at)
            .bind(input.max_docs)
            .fetch_all(&mut *tx)
            .await?
        };

        if docs.is_empty() {
            return Err(anyhow!("no outbound docs available"));
        }
        if let Some(doc_ids) = &input.outbound_doc_ids
            && docs.len() != doc_ids.len()
        {
            return Err(anyhow!("some outbound docs not available"));
        }

        let wave_row: WaveRow = sqlx::query_as(
            r#"
            insert into waves(tenant_id, warehouse_id, status)
            values ($1,$2,'CREATED')
            returning wave_id, warehouse_id, status, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.warehouse_id)
        .fetch_one(&mut *tx)
        .await?;

        let doc_ids: Vec<uuid::Uuid> = docs.iter().map(|row| row.outbound_doc_id).collect();
        let mut wave_docs_qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into wave_docs(tenant_id, wave_id, outbound_doc_id)
            "#,
        );
        wave_docs_qb.push_values(doc_ids.iter(), |mut b, doc_id| {
            b.push_bind(uuid::Uuid::from(input.tenant_id))
                .push_bind(wave_row.wave_id)
                .push_bind(*doc_id);
        });
        wave_docs_qb.build().execute(&mut *tx).await?;

        let (next_status, outbound_status, pick_status, picked_at) = if input.auto_pick {
            ("PICKED", "PENDING_VERIFY", "PICKED", Some(Utc::now()))
        } else {
            ("PICKING", "PICKING", "PICKING", None)
        };
        sqlx::query(
            r#"
            update outbound_docs
            set status = $3,
                outbound_status = $4,
                pick_status = $5,
                verify_status = 'NOT_VERIFIED',
                pack_status = 'NOT_PACKED',
                ship_status = 'NOT_SHIPPED',
                picked_at = coalesce($6, picked_at),
                updated_at = now()
            where tenant_id = $1
              and outbound_doc_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&doc_ids)
        .bind(next_status)
        .bind(outbound_status)
        .bind(pick_status)
        .bind(picked_at)
        .execute(&mut *tx)
        .await?;

        #[derive(sqlx::FromRow)]
        struct ItemRow {
            outbound_doc_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            planned_qty: i64,
        }

        let item_rows: Vec<ItemRow> = sqlx::query_as(
            r#"
            select outbound_doc_id, sku_id, planned_qty
            from outbound_doc_items
            where tenant_id = $1 and outbound_doc_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&doc_ids)
        .fetch_all(&mut *tx)
        .await?;

        let mut pick_qb = QueryBuilder::<Postgres>::new(
            r#"
            insert into outbound_pick_tasks(
                tenant_id, outbound_doc_id, sku_id, qty, picked_qty, status
            )
            "#,
        );
        pick_qb.push_values(item_rows.iter(), |mut b, row| {
            let picked_qty = if input.auto_pick {
                row.planned_qty
            } else {
                0_i64
            };
            let status = if input.auto_pick {
                "PICKED"
            } else {
                "NOT_PICKED"
            };
            b.push_bind(uuid::Uuid::from(input.tenant_id))
                .push_bind(row.outbound_doc_id)
                .push_bind(row.sku_id)
                .push_bind(row.planned_qty)
                .push_bind(picked_qty)
                .push_bind(status);
        });
        pick_qb.build().execute(&mut *tx).await?;

        if input.auto_pick {
            sqlx::query(
                r#"
                update outbound_doc_items
                set picked_qty = planned_qty,
                    updated_at = now()
                where tenant_id = $1 and outbound_doc_id = any($2)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(&doc_ids)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get_wave(input.tenant_id, wave_row.wave_id)
            .await?
            .ok_or_else(|| anyhow!("wave not found"))
    }

    pub async fn list_pick_tasks(
        &self,
        tenant_id: TenantId,
        filters: PickTaskListFilters,
    ) -> anyhow::Result<PickTaskListResult> {
        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)
            from outbound_pick_tasks t
            join outbound_docs d
              on d.tenant_id = t.tenant_id
             and d.outbound_doc_id = t.outbound_doc_id
            where t.tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_pick_task_filters(&mut count_qb, &filters);
        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                t.pick_task_id,
                t.outbound_doc_id,
                t.sku_id,
                t.from_location_id,
                t.container_id,
                t.qty,
                t.picked_qty,
                t.status,
                t.created_at,
                t.updated_at
            from outbound_pick_tasks t
            join outbound_docs d
              on d.tenant_id = t.tenant_id
             and d.outbound_doc_id = t.outbound_doc_id
            where t.tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));
        apply_pick_task_filters(&mut qb, &filters);
        qb.push(" order by t.created_at desc, t.pick_task_id desc");
        qb.push(" offset ");
        qb.push_bind(filters.offset);
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            pick_task_id: uuid::Uuid,
            outbound_doc_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            from_location_id: Option<uuid::Uuid>,
            container_id: Option<uuid::Uuid>,
            qty: i64,
            picked_qty: i64,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| OutboundPickTask {
                pick_task_id: row.pick_task_id,
                outbound_doc_id: row.outbound_doc_id,
                sku_id: row.sku_id,
                from_location_id: row.from_location_id,
                container_id: row.container_id,
                qty: row.qty,
                picked_qty: row.picked_qty,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(PickTaskListResult { items, total })
    }

    pub async fn confirm_pick_task(&self, input: PickTaskConfirmInput) -> anyhow::Result<()> {
        if input.picked_qty <= 0 {
            return Err(anyhow!("picked_qty must be positive"));
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct TaskRow {
            outbound_doc_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            qty: i64,
            picked_qty: i64,
            status: String,
        }

        let task: TaskRow = sqlx::query_as(
            r#"
            select outbound_doc_id, sku_id, qty, picked_qty, status
            from outbound_pick_tasks
            where tenant_id = $1 and pick_task_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.pick_task_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("pick task not found"))?;

        if task.status == "PICKED" {
            tx.commit().await?;
            return Ok(());
        }

        let remaining = task
            .qty
            .checked_sub(task.picked_qty)
            .ok_or_else(|| anyhow!("invalid pick task qty"))?;
        if input.picked_qty > remaining {
            return Err(anyhow!("picked_qty exceeds remaining"));
        }

        let new_picked = task
            .picked_qty
            .checked_add(input.picked_qty)
            .ok_or_else(|| anyhow!("picked_qty overflow"))?;
        let new_status = if new_picked >= task.qty {
            "PICKED"
        } else {
            "PICKING"
        };

        sqlx::query(
            r#"
            update outbound_pick_tasks
            set picked_qty = $3,
                status = $4,
                container_id = coalesce($5, container_id),
                updated_at = now()
            where tenant_id = $1 and pick_task_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.pick_task_id)
        .bind(new_picked)
        .bind(new_status)
        .bind(input.container_id)
        .execute(&mut *tx)
        .await?;

        let updated: Option<i64> = sqlx::query_scalar(
            r#"
            update outbound_doc_items
            set picked_qty = picked_qty + $4,
                updated_at = now()
            where tenant_id = $1
              and outbound_doc_id = $2
              and sku_id = $3
              and picked_qty + $4 <= planned_qty
            returning picked_qty
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(task.outbound_doc_id)
        .bind(task.sku_id)
        .bind(input.picked_qty)
        .fetch_optional(&mut *tx)
        .await?;

        if updated.is_none() {
            return Err(anyhow!("picked_qty exceeds planned qty"));
        }

        let remaining_items: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from outbound_doc_items
            where tenant_id = $1
              and outbound_doc_id = $2
              and picked_qty < planned_qty
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(task.outbound_doc_id)
        .fetch_one(&mut *tx)
        .await?;

        if remaining_items == 0 {
            sqlx::query(
                r#"
                update outbound_docs
                set status = 'PICKED',
                    outbound_status = 'PENDING_VERIFY',
                    pick_status = 'PICKED',
                    picked_at = coalesce(picked_at, now()),
                    updated_at = now()
                where tenant_id = $1 and outbound_doc_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(task.outbound_doc_id)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                r#"
                update outbound_docs
                set outbound_status = 'PICKING',
                    pick_status = 'PICKING',
                    updated_at = now()
                where tenant_id = $1 and outbound_doc_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(task.outbound_doc_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn pack_outbound(&self, input: OutboundPackInput) -> anyhow::Result<()> {
        if input.items.is_empty() {
            return Err(anyhow!("pack items required"));
        }
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct DocRow {
            shipment_id: Option<uuid::Uuid>,
            status: String,
        }

        let doc: DocRow = sqlx::query_as(
            r#"
            select shipment_id, status
            from outbound_docs
            where tenant_id = $1 and outbound_doc_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("outbound doc not found"))?;

        if doc.status == "SHIPPED" || doc.status == "CANCELLED" {
            return Err(anyhow!("outbound doc not packable"));
        }

        let mut packed_map = HashMap::new();
        for item in &input.items {
            let updated: Option<i64> = sqlx::query_scalar(
                r#"
                update outbound_doc_items
                set packed_qty = packed_qty + $4,
                    updated_at = now()
                where tenant_id = $1
                  and outbound_doc_id = $2
                  and sku_id = $3
                  and packed_qty + $4 <= planned_qty
                  and packed_qty + $4 <= picked_qty
                returning packed_qty
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.outbound_doc_id)
            .bind(item.sku_id)
            .bind(item.packed_qty)
            .fetch_optional(&mut *tx)
            .await?;

            let packed_qty =
                updated.ok_or_else(|| anyhow!("packed_qty exceeds picked/planned qty"))?;
            packed_map.insert(item.sku_id, packed_qty);
        }

        if !input.batches.is_empty() {
            let shipment_id = doc
                .shipment_id
                .ok_or_else(|| anyhow!("shipment_id required for batch collection"))?;
            for entry in &input.batches {
                if entry.qty <= 0 {
                    return Err(anyhow!("batch qty must be positive"));
                }
                if !packed_map.contains_key(&entry.sku_id) {
                    return Err(anyhow!("batch sku not in packed items"));
                }

                #[derive(sqlx::FromRow)]
                struct BatchRow {
                    batch_id: uuid::Uuid,
                    batch_no: String,
                }

                let batch_row: BatchRow = if let Some(batch_id) = entry.batch_id {
                    sqlx::query_as(
                        r#"
                        select batch_id, batch_no
                        from sku_batches
                        where tenant_id = $1 and sku_id = $2 and batch_id = $3
                        "#,
                    )
                    .bind(uuid::Uuid::from(input.tenant_id))
                    .bind(entry.sku_id)
                    .bind(batch_id)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or_else(|| anyhow!("batch not found"))?
                } else {
                    let batch_no = entry
                        .batch_no
                        .as_ref()
                        .map(|v| v.trim())
                        .filter(|v| !v.is_empty())
                        .ok_or_else(|| anyhow!("batch_id or batch_no required"))?;
                    sqlx::query_as(
                        r#"
                        select batch_id, batch_no
                        from sku_batches
                        where tenant_id = $1 and sku_id = $2 and batch_no = $3
                        "#,
                    )
                    .bind(uuid::Uuid::from(input.tenant_id))
                    .bind(entry.sku_id)
                    .bind(batch_no)
                    .fetch_optional(&mut *tx)
                    .await?
                    .ok_or_else(|| anyhow!("batch not found"))?
                };

                sqlx::query(
                    r#"
                    insert into shipment_sku_batches(
                        tenant_id, shipment_id, sku_id, batch_id, batch_no, qty
                    ) values ($1,$2,$3,$4,$5,$6)
                    on conflict (tenant_id, shipment_id, sku_id, batch_id)
                    do update set qty = shipment_sku_batches.qty + excluded.qty
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(shipment_id)
                .bind(entry.sku_id)
                .bind(batch_row.batch_id)
                .bind(&batch_row.batch_no)
                .bind(entry.qty)
                .execute(&mut *tx)
                .await?;
            }
        }

        if !packed_map.is_empty() {
            let sku_ids: Vec<uuid::Uuid> = packed_map.keys().cloned().collect();

            #[derive(sqlx::FromRow)]
            struct FlagRow {
                sku_id: uuid::Uuid,
                batch_enabled: bool,
                serial_enabled: bool,
            }

            let flags: Vec<FlagRow> = sqlx::query_as(
                r#"
                select sku_id, batch_enabled, serial_enabled
                from skus
                where tenant_id = $1 and sku_id = any($2)
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(&sku_ids)
            .fetch_all(&mut *tx)
            .await?;

            let serial_required: Vec<uuid::Uuid> = flags
                .iter()
                .filter(|row| row.serial_enabled)
                .map(|row| row.sku_id)
                .collect();
            let batch_required: Vec<uuid::Uuid> = flags
                .iter()
                .filter(|row| row.batch_enabled)
                .map(|row| row.sku_id)
                .collect();

            if !serial_required.is_empty() {
                let shipment_id = doc
                    .shipment_id
                    .ok_or_else(|| anyhow!("shipment_id required for serial check"))?;
                #[derive(sqlx::FromRow)]
                struct SerialCountRow {
                    sku_id: uuid::Uuid,
                    qty: i64,
                }
                let rows: Vec<SerialCountRow> = sqlx::query_as(
                    r#"
                    select sku_id, count(*)::bigint as qty
                    from shipment_item_serials
                    where tenant_id = $1
                      and shipment_id = $2
                      and sku_id = any($3)
                    group by sku_id
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(shipment_id)
                .bind(&serial_required)
                .fetch_all(&mut *tx)
                .await?;
                let serial_map: HashMap<uuid::Uuid, i64> =
                    rows.into_iter().map(|row| (row.sku_id, row.qty)).collect();
                for sku_id in &serial_required {
                    let required_qty = packed_map.get(sku_id).copied().unwrap_or(0);
                    let bound_qty = serial_map.get(sku_id).copied().unwrap_or(0);
                    if bound_qty < required_qty {
                        return Err(anyhow!("serials required before packing"));
                    }
                }
            }

            if !batch_required.is_empty() {
                let shipment_id = doc
                    .shipment_id
                    .ok_or_else(|| anyhow!("shipment_id required for batch check"))?;
                #[derive(sqlx::FromRow)]
                struct BatchCountRow {
                    sku_id: uuid::Uuid,
                    qty: i64,
                }
                let rows: Vec<BatchCountRow> = sqlx::query_as(
                    r#"
                    select sku_id, sum(qty)::bigint as qty
                    from shipment_sku_batches
                    where tenant_id = $1
                      and shipment_id = $2
                      and sku_id = any($3)
                    group by sku_id
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(shipment_id)
                .bind(&batch_required)
                .fetch_all(&mut *tx)
                .await?;
                let batch_map: HashMap<uuid::Uuid, i64> =
                    rows.into_iter().map(|row| (row.sku_id, row.qty)).collect();
                for sku_id in &batch_required {
                    let required_qty = packed_map.get(sku_id).copied().unwrap_or(0);
                    let bound_qty = batch_map.get(sku_id).copied().unwrap_or(0);
                    if bound_qty < required_qty {
                        return Err(anyhow!("batch info required before packing"));
                    }
                }
            }
        }

        sqlx::query(
            r#"
            insert into pack_records(
                tenant_id, outbound_doc_id, shipment_id, package_no,
                weight_gram, operator_id, pack_status
            ) values ($1,$2,$3,$4,$5,$6,$7)
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .bind(doc.shipment_id)
        .bind(&input.package_no)
        .bind(input.weight_gram)
        .bind(input.operator_id)
        .bind("PACKED")
        .execute(&mut *tx)
        .await?;

        if let Some(shipment_id) = doc.shipment_id {
            sqlx::query(
                r#"
                insert into shipment_packages(
                    tenant_id, shipment_id, package_no, carrier_code, tracking_no, weight_gram
                ) values ($1,$2,$3,$4,$5,$6)
                on conflict (tenant_id, shipment_id, package_no)
                do update set
                    weight_gram = excluded.weight_gram,
                    updated_at = now()
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment_id)
            .bind(&input.package_no)
            .bind(Option::<String>::None)
            .bind(Option::<String>::None)
            .bind(input.weight_gram)
            .execute(&mut *tx)
            .await?;
        }

        let remaining_items: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from outbound_doc_items
            where tenant_id = $1
              and outbound_doc_id = $2
              and packed_qty < planned_qty
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_one(&mut *tx)
        .await?;

        let (next_status, outbound_status, pack_status, packed_at) = if remaining_items == 0 {
            ("PACKED", "PENDING_SHIP", "PACKED", Some(Utc::now()))
        } else {
            ("PACKING", "PENDING_PACK", "PACKING", None)
        };

        sqlx::query(
            r#"
            update outbound_docs
            set status = $3,
                outbound_status = $4,
                pack_status = $5,
                packed_at = coalesce($6, packed_at),
                updated_at = now()
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .bind(next_status)
        .bind(outbound_status)
        .bind(pack_status)
        .bind(packed_at)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn record_pack_check_event(&self, input: PackCheckEventInput) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            insert into pack_check_events(
                tenant_id, outbound_doc_id, shipment_id, package_no,
                event_type, result, weight_gram, reason, operator_id
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .bind(input.shipment_id)
        .bind(&input.package_no)
        .bind(&input.event_type)
        .bind(&input.result)
        .bind(input.weight_gram)
        .bind(input.reason.as_deref())
        .bind(input.operator_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_pack_record_weight(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
        package_no: &str,
        weight_gram: i32,
        operator_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<()> {
        let updated: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            update pack_records
            set weight_gram = $4,
                operator_id = $5
            where tenant_id = $1
              and outbound_doc_id = $2
              and package_no = $3
            returning pack_record_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .bind(package_no)
        .bind(weight_gram)
        .bind(operator_id)
        .fetch_optional(self.pool)
        .await?;

        if updated.is_none() {
            return Err(anyhow!("pack record not found"));
        }
        Ok(())
    }

    pub async fn has_unweighed_packs(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from pack_records
            where tenant_id = $1
              and outbound_doc_id = $2
              and weight_gram is null
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_one(self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn has_unchecked_packs(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from pack_records pr
            left join lateral (
                select result
                from pack_check_events e
                where e.tenant_id = pr.tenant_id
                  and e.outbound_doc_id = pr.outbound_doc_id
                  and e.package_no = pr.package_no
                  and e.event_type = 'CHECK'
                order by e.created_at desc, e.check_event_id desc
                limit 1
            ) latest on true
            where pr.tenant_id = $1
              and pr.outbound_doc_id = $2
              and (latest.result is null or latest.result <> 'PASSED')
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_one(self.pool)
        .await?;

        Ok(count > 0)
    }

    pub async fn reset_outbound_inspection(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update outbound_docs
            set status = 'PACKED',
                outbound_status = 'PENDING_VERIFY',
                verify_status = 'VERIFY_EXCEPTION',
                pack_status = 'PACKED',
                updated_at = now()
            where tenant_id = $1
              and outbound_doc_id = $2
              and status = 'INSPECTED'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn inspect_outbound(&self, input: OutboundInspectInput) -> anyhow::Result<()> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct DocRow {
            shipment_id: Option<uuid::Uuid>,
            status: String,
        }

        let doc: DocRow = sqlx::query_as(
            r#"
            select shipment_id, status
            from outbound_docs
            where tenant_id = $1 and outbound_doc_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("outbound doc not found"))?;

        if doc.status == "INSPECTED" {
            tx.commit().await?;
            return Ok(());
        }

        if doc.status != "PACKED" {
            return Err(anyhow!("outbound doc not packed"));
        }

        #[derive(sqlx::FromRow)]
        struct ItemRow {
            sku_id: uuid::Uuid,
            planned_qty: i64,
            packed_qty: i64,
            serial_enabled: bool,
            batch_enabled: bool,
        }

        let items: Vec<ItemRow> = sqlx::query_as(
            r#"
            select i.sku_id, i.planned_qty, i.packed_qty,
                   s.serial_enabled, s.batch_enabled
            from outbound_doc_items i
            join skus s
              on s.tenant_id = i.tenant_id
             and s.sku_id = i.sku_id
            where i.tenant_id = $1 and i.outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_all(&mut *tx)
        .await?;

        if items.is_empty() {
            return Err(anyhow!("outbound items missing"));
        }

        let mut serial_required = Vec::new();
        let mut batch_required = Vec::new();
        for item in &items {
            if item.packed_qty != item.planned_qty {
                return Err(anyhow!("packed_qty mismatch"));
            }
            if item.serial_enabled {
                serial_required.push(item.sku_id);
            }
            if item.batch_enabled {
                batch_required.push(item.sku_id);
            }
        }

        if (!serial_required.is_empty() || !batch_required.is_empty()) && doc.shipment_id.is_none()
        {
            return Err(anyhow!("shipment_id required for inspection"));
        }

        if !serial_required.is_empty() {
            let shipment_id = doc
                .shipment_id
                .ok_or_else(|| anyhow!("shipment_id required for serial check"))?;
            #[derive(sqlx::FromRow)]
            struct SerialCountRow {
                sku_id: uuid::Uuid,
                qty: i64,
            }
            let rows: Vec<SerialCountRow> = sqlx::query_as(
                r#"
                select sku_id, count(*)::bigint as qty
                from shipment_item_serials
                where tenant_id = $1
                  and shipment_id = $2
                  and sku_id = any($3)
                group by sku_id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment_id)
            .bind(&serial_required)
            .fetch_all(&mut *tx)
            .await?;

            let serial_map: HashMap<uuid::Uuid, i64> =
                rows.into_iter().map(|row| (row.sku_id, row.qty)).collect();
            for item in &items {
                if !item.serial_enabled {
                    continue;
                }
                let bound_qty = serial_map.get(&item.sku_id).copied().unwrap_or(0);
                if bound_qty < item.planned_qty {
                    return Err(anyhow!("serials required before inspection"));
                }
            }
        }

        if !batch_required.is_empty() {
            let shipment_id = doc
                .shipment_id
                .ok_or_else(|| anyhow!("shipment_id required for batch check"))?;
            #[derive(sqlx::FromRow)]
            struct BatchCountRow {
                sku_id: uuid::Uuid,
                qty: i64,
            }
            let rows: Vec<BatchCountRow> = sqlx::query_as(
                r#"
                select sku_id, sum(qty)::bigint as qty
                from shipment_sku_batches
                where tenant_id = $1
                  and shipment_id = $2
                  and sku_id = any($3)
                group by sku_id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment_id)
            .bind(&batch_required)
            .fetch_all(&mut *tx)
            .await?;

            let batch_map: HashMap<uuid::Uuid, i64> =
                rows.into_iter().map(|row| (row.sku_id, row.qty)).collect();
            for item in &items {
                if !item.batch_enabled {
                    continue;
                }
                let bound_qty = batch_map.get(&item.sku_id).copied().unwrap_or(0);
                if bound_qty < item.planned_qty {
                    return Err(anyhow!("batch info required before inspection"));
                }
            }
        }

        sqlx::query(
            r#"
            update outbound_docs
            set status = 'INSPECTED',
                outbound_status = 'PENDING_SHIP',
                verify_status = 'VERIFIED',
                pack_status = 'PACKED',
                updated_at = now()
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn start_outbound_verify(
        &self,
        input: OutboundVerifyStartInput,
    ) -> anyhow::Result<OutboundVerifyState> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        let package_no = normalize_package_no(&input.package_no);

        let doc_exists: Option<uuid::Uuid> = sqlx::query_scalar(
            r#"
            select outbound_doc_id
            from outbound_docs
            where tenant_id = $1 and outbound_doc_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_optional(&mut *tx)
        .await?;
        if doc_exists.is_none() {
            return Err(anyhow!("outbound doc not found"));
        }

        #[derive(sqlx::FromRow)]
        struct SessionRow {
            session_id: uuid::Uuid,
        }

        let active_session: Option<SessionRow> = sqlx::query_as(
            r#"
            select session_id
            from wms_outbound_verify_sessions
            where tenant_id = $1
              and outbound_doc_id = $2
              and package_no = $3
              and status = 'VERIFYING'
            order by created_at desc, session_id desc
            limit 1
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .bind(&package_no)
        .fetch_optional(&mut *tx)
        .await?;

        let session_id = if let Some(session) = active_session {
            sqlx::query(
                r#"
                update wms_outbound_verify_sessions
                set shipment_no = $4,
                    tracking_no = $5,
                    updated_at = now()
                where tenant_id = $1 and outbound_doc_id = $2 and session_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.outbound_doc_id)
            .bind(session.session_id)
            .bind(input.shipment_no.trim())
            .bind(input.tracking_no.as_deref().and_then(non_empty_owned))
            .execute(&mut *tx)
            .await?;
            session.session_id
        } else {
            let inserted: uuid::Uuid = sqlx::query_scalar(
                r#"
                insert into wms_outbound_verify_sessions(
                    tenant_id, outbound_doc_id, shipment_no, tracking_no, package_no, status, started_by
                ) values ($1, $2, $3, $4, $5, 'VERIFYING', $6)
                returning session_id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.outbound_doc_id)
            .bind(input.shipment_no.trim())
            .bind(input.tracking_no.as_deref().and_then(non_empty_owned))
            .bind(&package_no)
            .bind(input.started_by)
            .fetch_one(&mut *tx)
            .await?;
            inserted
        };

        sqlx::query(
            r#"
            update outbound_docs
            set verify_status = 'VERIFYING',
                updated_at = now()
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .execute(&mut *tx)
        .await?;

        let state = self
            .build_verify_state_with_tx(
                &mut tx,
                input.tenant_id,
                input.outbound_doc_id,
                session_id,
                "VERIFYING".to_string(),
            )
            .await?;
        tx.commit().await?;
        Ok(state)
    }

    pub async fn scan_outbound_verify_item(
        &self,
        input: OutboundVerifyScanInput,
    ) -> anyhow::Result<OutboundVerifyState> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct SessionRow {
            session_id: uuid::Uuid,
            package_no: String,
        }

        let session: SessionRow = sqlx::query_as(
            r#"
            select session_id, package_no
            from wms_outbound_verify_sessions
            where tenant_id = $1
              and outbound_doc_id = $2
              and status = 'VERIFYING'
            order by created_at desc, session_id desc
            limit 1
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("verify session not started"))?;

        #[derive(sqlx::FromRow)]
        struct MatchedSkuRow {
            sku_id: uuid::Uuid,
        }

        let matched: Vec<MatchedSkuRow> = sqlx::query_as(
            r#"
            select distinct i.sku_id
            from outbound_doc_items i
            join skus s
              on s.tenant_id = i.tenant_id
             and s.sku_id = i.sku_id
            left join product_barcodes pb
              on pb.tenant_id = i.tenant_id
             and pb.sku_id = i.sku_id
             and pb.status = 'ACTIVE'
            where i.tenant_id = $1
              and i.outbound_doc_id = $2
              and (s.barcode = $3 or pb.barcode = $3)
            limit 2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .bind(input.barcode.trim())
        .fetch_all(&mut *tx)
        .await?;
        if matched.len() > 1 {
            return Err(anyhow!("barcode maps to multiple sku"));
        }
        let matched_sku = matched.first().map(|row| row.sku_id);

        sqlx::query(
            r#"
            insert into wms_outbound_verify_scan_lines(
                session_id, tenant_id, outbound_doc_id, package_no, barcode, sku_id, qty, scanned_by, extra_json
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(session.session_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .bind(session.package_no)
        .bind(input.barcode.trim())
        .bind(matched_sku)
        .bind(input.qty)
        .bind(input.scanned_by)
        .bind(serde_json::json!({
            "matched": matched_sku.is_some()
        }))
        .execute(&mut *tx)
        .await?;

        let state = self
            .build_verify_state_with_tx(
                &mut tx,
                input.tenant_id,
                input.outbound_doc_id,
                session.session_id,
                "VERIFYING".to_string(),
            )
            .await?;
        tx.commit().await?;
        Ok(state)
    }

    pub async fn finish_outbound_verify(
        &self,
        input: OutboundVerifyFinishInput,
    ) -> anyhow::Result<OutboundVerifyState> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct SessionRow {
            session_id: uuid::Uuid,
            status: String,
        }

        let active_session: Option<SessionRow> = sqlx::query_as(
            r#"
            select session_id, status
            from wms_outbound_verify_sessions
            where tenant_id = $1
              and outbound_doc_id = $2
              and status = 'VERIFYING'
            order by created_at desc, session_id desc
            limit 1
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_optional(&mut *tx)
        .await?;

        let (session_id, verify_status) = if let Some(session) = active_session {
            let current = self
                .build_verify_state_with_tx(
                    &mut tx,
                    input.tenant_id,
                    input.outbound_doc_id,
                    session.session_id,
                    session.status,
                )
                .await?;
            let final_status = if current.missing_total_qty == 0 && current.extra_total_qty == 0 {
                "VERIFIED".to_string()
            } else {
                "VERIFY_EXCEPTION".to_string()
            };

            sqlx::query(
                r#"
                update wms_outbound_verify_sessions
                set status = $4,
                    finished_by = $5,
                    finished_at = now(),
                    updated_at = now()
                where tenant_id = $1 and outbound_doc_id = $2 and session_id = $3
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.outbound_doc_id)
            .bind(session.session_id)
            .bind(&final_status)
            .bind(input.finished_by)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                update outbound_docs
                set verify_status = $3,
                    outbound_status = case
                      when $3 = 'VERIFIED' and outbound_status <> 'SHIPPED' then 'PENDING_SHIP'
                      else outbound_status
                    end,
                    updated_at = now()
                where tenant_id = $1 and outbound_doc_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.outbound_doc_id)
            .bind(&final_status)
            .execute(&mut *tx)
            .await?;
            (session.session_id, final_status)
        } else {
            let latest: SessionRow = sqlx::query_as(
                r#"
                select session_id, status
                from wms_outbound_verify_sessions
                where tenant_id = $1 and outbound_doc_id = $2
                order by created_at desc, session_id desc
                limit 1
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(input.outbound_doc_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("verify session not started"))?;
            (latest.session_id, latest.status)
        };

        let state = self
            .build_verify_state_with_tx(
                &mut tx,
                input.tenant_id,
                input.outbound_doc_id,
                session_id,
                verify_status,
            )
            .await?;
        tx.commit().await?;
        Ok(state)
    }

    pub async fn ship_outbound(&self, input: OutboundShipInput) -> anyhow::Result<()> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct DocRow {
            shipment_id: Option<uuid::Uuid>,
            status: String,
            warehouse_id: uuid::Uuid,
        }

        let doc: DocRow = sqlx::query_as(
            r#"
            select shipment_id, status, warehouse_id
            from outbound_docs
            where tenant_id = $1 and outbound_doc_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("outbound doc not found"))?;

        if doc.status == "SHIPPED" {
            tx.commit().await?;
            return Ok(());
        }

        if input.inspect_required {
            if doc.status != "INSPECTED" {
                return Err(anyhow!("outbound doc not inspected"));
            }
        } else if input.allow_unpacked {
            if doc.status != "PACKED" && doc.status != "PICKED" && doc.status != "INSPECTED" {
                return Err(anyhow!("outbound doc not ready"));
            }
        } else if doc.status != "PACKED" && doc.status != "INSPECTED" {
            return Err(anyhow!("outbound doc not packed"));
        }

        #[derive(sqlx::FromRow)]
        struct ItemRow {
            sku_id: uuid::Uuid,
            planned_qty: i64,
            batch_enabled: bool,
        }

        let items: Vec<ItemRow> = sqlx::query_as(
            r#"
            select i.sku_id, i.planned_qty, s.batch_enabled
            from outbound_doc_items i
            join skus s
              on s.tenant_id = i.tenant_id
             and s.sku_id = i.sku_id
            where i.tenant_id = $1 and i.outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .fetch_all(&mut *tx)
        .await?;

        if items.is_empty() {
            return Err(anyhow!("outbound items missing"));
        }

        let batch_required: Vec<uuid::Uuid> = items
            .iter()
            .filter(|item| item.batch_enabled)
            .map(|item| item.sku_id)
            .collect();
        if !batch_required.is_empty() {
            let shipment_id = doc
                .shipment_id
                .ok_or_else(|| anyhow!("shipment_id required for batch check"))?;
            #[derive(sqlx::FromRow)]
            struct BatchCountRow {
                sku_id: uuid::Uuid,
                qty: i64,
            }
            let rows: Vec<BatchCountRow> = sqlx::query_as(
                r#"
                select sku_id, sum(qty)::bigint as qty
                from shipment_sku_batches
                where tenant_id = $1
                  and shipment_id = $2
                  and sku_id = any($3)
                group by sku_id
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment_id)
            .bind(&batch_required)
            .fetch_all(&mut *tx)
            .await?;

            let batch_map: HashMap<uuid::Uuid, i64> =
                rows.into_iter().map(|row| (row.sku_id, row.qty)).collect();
            for item in &items {
                if !item.batch_enabled {
                    continue;
                }
                let bound_qty = batch_map.get(&item.sku_id).copied().unwrap_or(0);
                if bound_qty < item.planned_qty {
                    return Err(anyhow!("batch info required before shipping"));
                }
            }

            #[derive(sqlx::FromRow)]
            struct ExpiredBatchRow {
                sku_id: uuid::Uuid,
                batch_no: String,
                expire_at: DateTime<Utc>,
            }
            let expired_batch: Option<ExpiredBatchRow> = sqlx::query_as(
                r#"
                select sb.sku_id, sb.batch_no, b.expire_at
                from shipment_sku_batches sb
                join sku_batches b
                  on b.tenant_id = sb.tenant_id
                 and b.batch_id = sb.batch_id
                where sb.tenant_id = $1
                  and sb.shipment_id = $2
                  and sb.sku_id = any($3)
                  and b.expire_at is not null
                  and b.expire_at <= $4
                order by b.expire_at asc, sb.batch_no asc
                limit 1
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment_id)
            .bind(&batch_required)
            .bind(Utc::now())
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(expired_batch) = expired_batch {
                return Err(anyhow!(
                    "batch expired before shipping: sku_id={}, batch_no={}, expire_at={}",
                    expired_batch.sku_id,
                    expired_batch.batch_no,
                    expired_batch.expire_at.to_rfc3339(),
                ));
            }

            #[derive(sqlx::FromRow)]
            struct FutureProductionBatchRow {
                sku_id: uuid::Uuid,
                batch_no: String,
                production_date: DateTime<Utc>,
            }
            let future_production_batch: Option<FutureProductionBatchRow> = sqlx::query_as(
                r#"
                select sb.sku_id, sb.batch_no, b.production_date
                from shipment_sku_batches sb
                join sku_batches b
                  on b.tenant_id = sb.tenant_id
                 and b.batch_id = sb.batch_id
                where sb.tenant_id = $1
                  and sb.shipment_id = $2
                  and sb.sku_id = any($3)
                  and b.production_date is not null
                  and b.production_date > $4
                order by b.production_date asc, sb.batch_no asc
                limit 1
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment_id)
            .bind(&batch_required)
            .bind(Utc::now())
            .fetch_optional(&mut *tx)
            .await?;

            if let Some(future_production_batch) = future_production_batch {
                return Err(anyhow!(
                    "batch production date in future before shipping: sku_id={}, batch_no={}, production_date={}",
                    future_production_batch.sku_id,
                    future_production_batch.batch_no,
                    future_production_batch.production_date.to_rfc3339(),
                ));
            }
        }

        let service = InventoryService::new(self.pool);
        for item in &items {
            #[derive(sqlx::FromRow)]
            struct BalanceRow {
                on_hand: i64,
                reserved: i64,
                frozen: i64,
            }

            let balance: BalanceRow = sqlx::query_as(
                r#"
                select on_hand, reserved, frozen
                from inventory_balances
                where tenant_id = $1 and warehouse_id = $2 and sku_id = $3
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(doc.warehouse_id)
            .bind(item.sku_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("insufficient inventory"))?;

            let available = balance.on_hand - balance.reserved - balance.frozen;
            if available < item.planned_qty {
                return Err(anyhow!("insufficient inventory"));
            }

            service
                .apply_with_tx(
                    &mut tx,
                    InventoryDelta {
                        tenant_id: input.tenant_id,
                        warehouse_id: doc.warehouse_id,
                        sku_id: item.sku_id,
                        delta_on_hand: -item.planned_qty,
                        delta_reserved: 0,
                        delta_in_transit: 0,
                        delta_frozen: 0,
                        reason_type: "SALES_SHIP".to_string(),
                        ref_type: Some("OUTBOUND_DOC".to_string()),
                        ref_id: Some(input.outbound_doc_id.to_string()),
                        occurred_at: Utc::now(),
                    },
                )
                .await?;
        }

        let shipped_at = input.shipped_at.unwrap_or_else(Utc::now);
        sqlx::query(
            r#"
            update outbound_docs
            set status = 'SHIPPED',
                outbound_status = 'SHIPPED',
                ship_status = 'SHIPPED',
                shipped_at = $3,
                updated_at = now()
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.outbound_doc_id)
        .bind(shipped_at)
        .execute(&mut *tx)
        .await?;

        if let Some(shipment_id) = doc.shipment_id {
            #[derive(sqlx::FromRow)]
            struct ShipmentRow {
                order_id: uuid::Uuid,
                shop_id: uuid::Uuid,
                platform: String,
                platform_order_id: String,
                carrier_code: Option<String>,
                tracking_no: Option<String>,
            }

            let shipment = sqlx::query_as::<_, ShipmentRow>(
                r#"
                select order_id, shop_id, platform, platform_order_id,
                       carrier_code, tracking_no
                from shipments
                where tenant_id = $1 and shipment_id = $2
                for update
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow!("shipment not found"))?;

            sqlx::query(
                r#"
                update shipments
                set status = 'SHIPPED',
                    ship_status = 'SHIPPED',
                    last_push_error = '',
                    shipped_at = $3,
                    updated_at = now()
                where tenant_id = $1 and shipment_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment_id)
            .bind(shipped_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                update orders o
                set status = 'SHIPPED',
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
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(shipment.order_id)
            .bind(shipped_at)
            .execute(&mut *tx)
            .await?;

            let sku_ids: Vec<uuid::Uuid> = items.iter().map(|item| item.sku_id).collect();
            if !sku_ids.is_empty() {
                #[derive(sqlx::FromRow)]
                struct CostRow {
                    sku_id: uuid::Uuid,
                    cost_cent: Option<i64>,
                }

                let cost_rows: Vec<CostRow> = sqlx::query_as(
                    r#"
                    select sku_id, cost_cent
                    from skus
                    where tenant_id = $1 and sku_id = any($2)
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(&sku_ids)
                .fetch_all(&mut *tx)
                .await?;

                let cost_map: HashMap<uuid::Uuid, i64> = cost_rows
                    .into_iter()
                    .map(|row| (row.sku_id, row.cost_cent.unwrap_or(0)))
                    .collect();

                let mut qb = QueryBuilder::<Postgres>::new(
                    r#"
                    insert into order_cogs(
                        tenant_id, order_id, shipment_id, outbound_doc_id, order_item_id,
                        sku_id, qty, unit_cost_cent, total_cost_cent, ref_type, ref_id
                    )
                    "#,
                );
                qb.push_values(items.iter(), |mut b, item| {
                    let unit_cost = *cost_map.get(&item.sku_id).unwrap_or(&0);
                    let total_cost = unit_cost.saturating_mul(item.planned_qty);
                    b.push_bind(uuid::Uuid::from(input.tenant_id))
                        .push_bind(shipment.order_id)
                        .push_bind(shipment_id)
                        .push_bind(input.outbound_doc_id)
                        .push_bind(Option::<uuid::Uuid>::None)
                        .push_bind(item.sku_id)
                        .push_bind(item.planned_qty)
                        .push_bind(unit_cost)
                        .push_bind(total_cost)
                        .push_bind(ORDER_COGS_REF_OUTBOUND_DOC)
                        .push_bind(input.outbound_doc_id.to_string());
                });
                qb.push(" on conflict (tenant_id, ref_type, ref_id, sku_id) do nothing");
                qb.build().execute(&mut *tx).await?;
            }

            let payload = serde_json::json!({
                "tenant_id": input.tenant_id,
                "shipment_id": shipment_id,
                "order_id": shipment.order_id,
                "shop_id": shipment.shop_id,
                "platform": shipment.platform,
                "platform_order_id": shipment.platform_order_id,
                "warehouse_id": doc.warehouse_id,
                "carrier_code": shipment.carrier_code,
                "tracking_no": shipment.tracking_no,
                "shipped_at": shipped_at,
                "outbound_doc_id": input.outbound_doc_id,
            });

            let exists: Option<uuid::Uuid> = sqlx::query_scalar(
                r#"
                select outbox_id
                from outbox_events
                where tenant_id = $1
                  and event_type = $2
                  and payload->>'shipment_id' = $3
                  and status in ('PENDING','FAILED','DISPATCHING')
                limit 1
                "#,
            )
            .bind(uuid::Uuid::from(input.tenant_id))
            .bind(SHIPMENT_SHIPPED_EVENT)
            .bind(shipment_id.to_string())
            .fetch_optional(&mut *tx)
            .await?;

            if exists.is_none() {
                let event_month = month_start(shipped_at);
                sqlx::query(
                    r#"
                    insert into outbox_events(
                        tenant_id, event_month, event_type, payload
                    ) values ($1,$2,$3,$4)
                    "#,
                )
                .bind(uuid::Uuid::from(input.tenant_id))
                .bind(event_month)
                .bind(SHIPMENT_SHIPPED_EVENT)
                .bind(payload)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn cancel_outbounds_for_order(
        &self,
        tenant_id: TenantId,
        order_id: OrderId,
    ) -> anyhow::Result<Vec<uuid::Uuid>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            outbound_doc_id: uuid::Uuid,
            status: String,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select d.outbound_doc_id, d.status
            from outbound_docs d
            join shipments s
              on s.tenant_id = d.tenant_id
             and s.shipment_id = d.shipment_id
            where d.tenant_id = $1 and s.order_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(uuid::Uuid::from(order_id))
        .fetch_all(&mut *tx)
        .await?;

        let outbound_doc_ids: Vec<uuid::Uuid> = rows
            .into_iter()
            .filter(|row| row.status != "SHIPPED")
            .map(|row| row.outbound_doc_id)
            .collect();

        if outbound_doc_ids.is_empty() {
            tx.commit().await?;
            return Ok(Vec::new());
        }

        sqlx::query(
            r#"
            update outbound_docs
            set status = 'CANCELLED',
                outbound_status = 'CLOSED',
                ship_status = 'NOT_SHIPPED',
                updated_at = now()
            where tenant_id = $1
              and outbound_doc_id = any($2)
              and status <> 'SHIPPED'
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&outbound_doc_ids)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            delete from pack_records
            where tenant_id = $1 and outbound_doc_id = any($2)
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(&outbound_doc_ids)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(outbound_doc_ids)
    }

    async fn build_verify_state_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
        session_id: uuid::Uuid,
        verify_status: String,
    ) -> anyhow::Result<OutboundVerifyState> {
        #[derive(sqlx::FromRow)]
        struct PlannedRow {
            sku_id: uuid::Uuid,
            planned_qty: i64,
        }

        #[derive(sqlx::FromRow)]
        struct ScannedRow {
            sku_id: uuid::Uuid,
            qty: i64,
        }

        let planned_rows: Vec<PlannedRow> = sqlx::query_as(
            r#"
            select sku_id, planned_qty
            from outbound_doc_items
            where tenant_id = $1 and outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_all(&mut **tx)
        .await?;

        let scanned_rows: Vec<ScannedRow> = sqlx::query_as(
            r#"
            select sku_id, sum(qty)::bigint as qty
            from wms_outbound_verify_scan_lines
            where tenant_id = $1
              and outbound_doc_id = $2
              and session_id = $3
              and sku_id is not null
            group by sku_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .bind(session_id)
        .fetch_all(&mut **tx)
        .await?;

        let unmatched_scanned_qty: i64 = sqlx::query_scalar(
            r#"
            select coalesce(sum(qty), 0)::bigint
            from wms_outbound_verify_scan_lines
            where tenant_id = $1
              and outbound_doc_id = $2
              and session_id = $3
              and sku_id is null
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .bind(session_id)
        .fetch_one(&mut **tx)
        .await?;

        let mut planned_map: HashMap<uuid::Uuid, i64> = HashMap::new();
        for row in planned_rows {
            planned_map.insert(row.sku_id, row.planned_qty);
        }

        let mut scanned_map: HashMap<uuid::Uuid, i64> = HashMap::new();
        let mut matched_scanned_qty = 0_i64;
        for row in scanned_rows {
            matched_scanned_qty += row.qty;
            scanned_map.insert(row.sku_id, row.qty);
        }

        let mut missing_total_qty = 0_i64;
        let mut extra_total_qty = unmatched_scanned_qty;

        for (sku_id, planned_qty) in &planned_map {
            let scanned_qty = scanned_map.get(sku_id).copied().unwrap_or(0);
            if scanned_qty < *planned_qty {
                missing_total_qty += planned_qty - scanned_qty;
            } else if scanned_qty > *planned_qty {
                extra_total_qty += scanned_qty - planned_qty;
            }
        }

        for (sku_id, scanned_qty) in scanned_map {
            if !planned_map.contains_key(&sku_id) {
                extra_total_qty += scanned_qty;
            }
        }

        Ok(OutboundVerifyState {
            outbound_doc_id,
            session_id,
            verify_status,
            scanned_total_qty: matched_scanned_qty + unmatched_scanned_qty,
            missing_total_qty,
            extra_total_qty,
        })
    }

    async fn fetch_outbound_doc(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<Option<OutboundDoc>> {
        let row: Option<OutboundDocRowWithShipment> = sqlx::query_as(
            r#"
            select
                d.outbound_doc_id,
                d.warehouse_id,
                d.doc_type,
                d.shipment_id,
                d.doc_no,
                d.status,
                d.outbound_status,
                d.pick_status,
                d.verify_status,
                d.pack_status,
                d.ship_status,
                d.picked_at,
                d.packed_at,
                d.shipped_at,
                d.created_at,
                d.updated_at,
                s.order_id,
                s.platform,
                s.platform_order_id,
                s.shop_id,
                s.carrier_code,
                s.tracking_no,
                o.receiver_name,
                o.receiver_phone,
                o.receiver_address,
                case
                    when o.inventory_shortage then o.exception_flags || '["INVENTORY_SHORTAGE"]'::jsonb
                    else o.exception_flags
                end as exception_flags,
                tags.tags
            from outbound_docs d
            left join shipments s
              on s.tenant_id = d.tenant_id
             and s.shipment_id = d.shipment_id
            left join order_index i
              on i.tenant_id = s.tenant_id
             and i.order_id = s.order_id
            left join orders o
              on o.tenant_id = i.tenant_id
             and o.order_id = i.order_id
             and o.order_month = i.order_month
            left join lateral (
                select array_agg(tag) as tags
                from order_tags ot
                where ot.tenant_id = s.tenant_id
                  and ot.order_id = s.order_id
            ) tags on true
            where d.tenant_id = $1 and d.outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_doc(tenant_id)))
    }

    async fn fetch_outbound_items(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<OutboundDocItem>> {
        #[derive(sqlx::FromRow)]
        struct ItemRow {
            outbound_doc_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            planned_qty: i64,
            picked_qty: i64,
            packed_qty: i64,
            batch_enabled: bool,
            serial_enabled: bool,
        }

        let rows: Vec<ItemRow> = sqlx::query_as(
            r#"
            select i.outbound_doc_id, i.sku_id, i.planned_qty, i.picked_qty, i.packed_qty,
                   s.batch_enabled, s.serial_enabled
            from outbound_doc_items i
            join skus s
              on s.tenant_id = i.tenant_id
             and s.sku_id = i.sku_id
            where i.tenant_id = $1 and i.outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OutboundDocItem {
                outbound_doc_id: row.outbound_doc_id,
                sku_id: row.sku_id,
                planned_qty: row.planned_qty,
                picked_qty: row.picked_qty,
                packed_qty: row.packed_qty,
                batch_enabled: row.batch_enabled,
                serial_enabled: row.serial_enabled,
            })
            .collect())
    }

    async fn fetch_outbound_items_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<OutboundDocItem>> {
        #[derive(sqlx::FromRow)]
        struct ItemRow {
            outbound_doc_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            planned_qty: i64,
            picked_qty: i64,
            packed_qty: i64,
            batch_enabled: bool,
            serial_enabled: bool,
        }

        let rows: Vec<ItemRow> = sqlx::query_as(
            r#"
            select i.outbound_doc_id, i.sku_id, i.planned_qty, i.picked_qty, i.packed_qty,
                   s.batch_enabled, s.serial_enabled
            from outbound_doc_items i
            join skus s
              on s.tenant_id = i.tenant_id
             and s.sku_id = i.sku_id
            where i.tenant_id = $1 and i.outbound_doc_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_all(&mut **tx)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OutboundDocItem {
                outbound_doc_id: row.outbound_doc_id,
                sku_id: row.sku_id,
                planned_qty: row.planned_qty,
                picked_qty: row.picked_qty,
                packed_qty: row.packed_qty,
                batch_enabled: row.batch_enabled,
                serial_enabled: row.serial_enabled,
            })
            .collect())
    }

    async fn fetch_pick_tasks(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<OutboundPickTask>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            pick_task_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            from_location_id: Option<uuid::Uuid>,
            container_id: Option<uuid::Uuid>,
            qty: i64,
            picked_qty: i64,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select pick_task_id, sku_id, from_location_id, container_id, qty, picked_qty,
                   status, created_at, updated_at
            from outbound_pick_tasks
            where tenant_id = $1 and outbound_doc_id = $2
            order by created_at asc, pick_task_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OutboundPickTask {
                pick_task_id: row.pick_task_id,
                outbound_doc_id,
                sku_id: row.sku_id,
                from_location_id: row.from_location_id,
                container_id: row.container_id,
                qty: row.qty,
                picked_qty: row.picked_qty,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    async fn fetch_pack_records(
        &self,
        tenant_id: TenantId,
        outbound_doc_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<PackRecord>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            pack_record_id: uuid::Uuid,
            shipment_id: Option<uuid::Uuid>,
            package_no: String,
            weight_gram: Option<i32>,
            operator_id: Option<uuid::Uuid>,
            packed_at: DateTime<Utc>,
            pack_status: String,
            void_reason: Option<String>,
            voided_at: Option<DateTime<Utc>>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select pack_record_id, shipment_id, package_no, weight_gram,
                   operator_id, packed_at, pack_status, void_reason, voided_at
            from pack_records
            where tenant_id = $1 and outbound_doc_id = $2
            order by packed_at desc, pack_record_id desc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbound_doc_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| PackRecord {
                pack_record_id: row.pack_record_id,
                shipment_id: row.shipment_id,
                package_no: row.package_no,
                weight_gram: row.weight_gram,
                operator_id: row.operator_id,
                packed_at: row.packed_at,
                pack_status: row.pack_status,
                void_reason: row.void_reason,
                voided_at: row.voided_at,
            })
            .collect())
    }

    async fn fetch_wave(
        &self,
        tenant_id: TenantId,
        wave_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Wave>> {
        let row: Option<WaveRow> = sqlx::query_as(
            r#"
            select wave_id, warehouse_id, status, created_at, updated_at
            from waves
            where tenant_id = $1 and wave_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(wave_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_wave(tenant_id)))
    }

    async fn fetch_wave_docs(
        &self,
        tenant_id: TenantId,
        wave_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<OutboundDoc>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            outbound_doc_id: uuid::Uuid,
            warehouse_id: uuid::Uuid,
            doc_type: String,
            shipment_id: Option<uuid::Uuid>,
            doc_no: String,
            status: String,
            outbound_status: String,
            pick_status: String,
            verify_status: String,
            pack_status: String,
            ship_status: String,
            picked_at: Option<DateTime<Utc>>,
            packed_at: Option<DateTime<Utc>>,
            shipped_at: Option<DateTime<Utc>>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            order_id: Option<uuid::Uuid>,
            platform: Option<String>,
            platform_order_id: Option<String>,
            shop_id: Option<uuid::Uuid>,
            carrier_code: Option<String>,
            tracking_no: Option<String>,
            receiver_name: Option<String>,
            receiver_phone: Option<String>,
            receiver_address: Option<String>,
            exception_flags: Option<Value>,
            tags: Option<Vec<String>>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select
                d.outbound_doc_id,
                d.warehouse_id,
                d.doc_type,
                d.shipment_id,
                d.doc_no,
                d.status,
                d.outbound_status,
                d.pick_status,
                d.verify_status,
                d.pack_status,
                d.ship_status,
                d.picked_at,
                d.packed_at,
                d.shipped_at,
                d.created_at,
                d.updated_at,
                s.order_id,
                s.platform,
                s.platform_order_id,
                s.shop_id,
                s.carrier_code,
                s.tracking_no,
                o.receiver_name,
                o.receiver_phone,
                o.receiver_address,
                case
                    when o.inventory_shortage then o.exception_flags || '["INVENTORY_SHORTAGE"]'::jsonb
                    else o.exception_flags
                end as exception_flags,
                tags.tags
            from wave_docs wd
            join outbound_docs d
              on d.tenant_id = wd.tenant_id
             and d.outbound_doc_id = wd.outbound_doc_id
            left join shipments s
              on s.tenant_id = d.tenant_id
             and s.shipment_id = d.shipment_id
            left join order_index i
              on i.tenant_id = s.tenant_id
             and i.order_id = s.order_id
            left join orders o
              on o.tenant_id = i.tenant_id
             and o.order_id = i.order_id
             and o.order_month = i.order_month
            left join lateral (
                select array_agg(tag) as tags
                from order_tags ot
                where ot.tenant_id = s.tenant_id
                  and ot.order_id = s.order_id
            ) tags on true
            where wd.tenant_id = $1 and wd.wave_id = $2
            order by d.created_at asc, d.outbound_doc_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(wave_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OutboundDoc {
                outbound_doc_id: row.outbound_doc_id,
                tenant_id,
                warehouse_id: row.warehouse_id,
                doc_type: row.doc_type,
                shipment_id: row.shipment_id,
                order_id: row.order_id,
                platform: row.platform,
                platform_order_id: row.platform_order_id,
                shop_id: row.shop_id,
                receiver_name: row.receiver_name,
                receiver_phone: row.receiver_phone,
                receiver_address: row.receiver_address,
                carrier_code: row.carrier_code,
                tracking_no: row.tracking_no,
                exception_flags: row
                    .exception_flags
                    .as_ref()
                    .map(json_array_to_vec)
                    .unwrap_or_default(),
                tags: row.tags.unwrap_or_default(),
                doc_no: row.doc_no,
                status: row.status,
                outbound_status: row.outbound_status,
                pick_status: row.pick_status,
                verify_status: row.verify_status,
                pack_status: row.pack_status,
                ship_status: row.ship_status,
                picked_at: row.picked_at,
                packed_at: row.packed_at,
                shipped_at: row.shipped_at,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    async fn fetch_wave_tasks(
        &self,
        tenant_id: TenantId,
        wave_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<OutboundPickTask>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            pick_task_id: uuid::Uuid,
            outbound_doc_id: uuid::Uuid,
            sku_id: uuid::Uuid,
            from_location_id: Option<uuid::Uuid>,
            container_id: Option<uuid::Uuid>,
            qty: i64,
            picked_qty: i64,
            status: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            r#"
            select
                t.pick_task_id,
                t.outbound_doc_id,
                t.sku_id,
                t.from_location_id,
                t.container_id,
                t.qty,
                t.picked_qty,
                t.status,
                t.created_at,
                t.updated_at
            from outbound_pick_tasks t
            join wave_docs wd
              on wd.tenant_id = t.tenant_id
             and wd.outbound_doc_id = t.outbound_doc_id
            where t.tenant_id = $1 and wd.wave_id = $2
            order by t.created_at asc, t.pick_task_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(wave_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| OutboundPickTask {
                pick_task_id: row.pick_task_id,
                outbound_doc_id: row.outbound_doc_id,
                sku_id: row.sku_id,
                from_location_id: row.from_location_id,
                container_id: row.container_id,
                qty: row.qty,
                picked_qty: row.picked_qty,
                status: row.status,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }
}

fn month_start(dt: DateTime<Utc>) -> NaiveDate {
    let d = dt.date_naive();
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
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

fn normalize_package_no(raw: &str) -> String {
    let normalized = raw.trim();
    if normalized.is_empty() {
        "DEFAULT".to_string()
    } else {
        normalized.to_string()
    }
}

fn non_empty_owned(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn apply_outbound_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a OutboundDocListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and d.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and d.status = ");
        qb.push_bind(status);
    }
    if let Some(outbound_status) = &filters.outbound_status {
        qb.push(" and d.outbound_status = ");
        qb.push_bind(outbound_status);
    }
    if let Some(pick_status) = &filters.pick_status {
        qb.push(" and d.pick_status = ");
        qb.push_bind(pick_status);
    }
    if let Some(verify_status) = &filters.verify_status {
        qb.push(" and d.verify_status = ");
        qb.push_bind(verify_status);
    }
    if let Some(pack_status) = &filters.pack_status {
        qb.push(" and d.pack_status = ");
        qb.push_bind(pack_status);
    }
    if let Some(ship_status) = &filters.ship_status {
        qb.push(" and d.ship_status = ");
        qb.push_bind(ship_status);
    }
    if let Some(doc_type) = &filters.doc_type {
        qb.push(" and d.doc_type = ");
        qb.push_bind(doc_type);
    }
    if let Some(doc_no) = &filters.doc_no {
        qb.push(" and d.doc_no = ");
        qb.push_bind(doc_no);
    }
    if let Some(doc_no_keyword) = &filters.doc_no_keyword {
        qb.push(" and d.doc_no ilike ");
        qb.push_bind(format!("%{doc_no_keyword}%"));
    }
    if let Some(shipment_id) = filters.shipment_id {
        qb.push(" and d.shipment_id = ");
        qb.push_bind(shipment_id);
    }
    if let Some(shop_id) = filters.shop_id {
        qb.push(" and s.shop_id = ");
        qb.push_bind(shop_id);
    }
    if let Some(carrier_code) = &filters.carrier_code {
        qb.push(" and s.carrier_code = ");
        qb.push_bind(carrier_code);
    }
    if let Some(tracking_no) = &filters.tracking_no {
        qb.push(" and s.tracking_no ilike ");
        qb.push_bind(format!("%{tracking_no}%"));
    }
    if let Some(tag) = &filters.tag {
        qb.push(
            r#"
            and exists (
                select 1
                from order_tags ot
                where ot.tenant_id = d.tenant_id
                  and ot.order_id = s.order_id
                  and ot.tag =
            "#,
        );
        qb.push_bind(tag);
        qb.push(")");
    }
    if let Some(flag) = &filters.exception_flag {
        qb.push(" and o.exception_flags ? ");
        qb.push_bind(flag);
    }
    if let Some(created_from) = filters.created_from {
        qb.push(" and d.created_at >= ");
        qb.push_bind(created_from);
    }
    if let Some(created_to) = filters.created_to {
        qb.push(" and d.created_at <= ");
        qb.push_bind(created_to);
    }
}

fn apply_wave_filters<'a>(qb: &mut QueryBuilder<'a, Postgres>, filters: &'a WaveListFilters) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and status = ");
        qb.push_bind(status);
    }
}

fn apply_pick_task_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a PickTaskListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and d.warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(outbound_doc_id) = filters.outbound_doc_id {
        qb.push(" and t.outbound_doc_id = ");
        qb.push_bind(outbound_doc_id);
    }
    if let Some(status) = &filters.status {
        qb.push(" and t.status = ");
        qb.push_bind(status);
    }
}

fn apply_wave_rule_filters<'a>(
    qb: &mut QueryBuilder<'a, Postgres>,
    filters: &'a WaveRuleListFilters,
) {
    if let Some(warehouse_id) = filters.warehouse_id {
        qb.push(" and warehouse_id = ");
        qb.push_bind(warehouse_id);
    }
    if let Some(carrier_code) = &filters.carrier_code {
        qb.push(" and carrier_code = ");
        qb.push_bind(carrier_code);
    }
    if let Some(enabled) = filters.enabled {
        qb.push(" and enabled = ");
        qb.push_bind(enabled);
    }
}

#[derive(sqlx::FromRow)]
struct OutboundDocRow {
    outbound_doc_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    doc_type: String,
}

#[derive(sqlx::FromRow)]
struct OutboundDocRowWithShipment {
    outbound_doc_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    doc_type: String,
    shipment_id: Option<uuid::Uuid>,
    doc_no: String,
    status: String,
    outbound_status: String,
    pick_status: String,
    verify_status: String,
    pack_status: String,
    ship_status: String,
    picked_at: Option<DateTime<Utc>>,
    packed_at: Option<DateTime<Utc>>,
    shipped_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    order_id: Option<uuid::Uuid>,
    platform: Option<String>,
    platform_order_id: Option<String>,
    shop_id: Option<uuid::Uuid>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    receiver_name: Option<String>,
    receiver_phone: Option<String>,
    receiver_address: Option<String>,
    exception_flags: Option<Value>,
    tags: Option<Vec<String>>,
}

impl OutboundDocRowWithShipment {
    fn into_doc(self, tenant_id: TenantId) -> OutboundDoc {
        OutboundDoc {
            outbound_doc_id: self.outbound_doc_id,
            tenant_id,
            warehouse_id: self.warehouse_id,
            doc_type: self.doc_type,
            shipment_id: self.shipment_id,
            order_id: self.order_id,
            platform: self.platform,
            platform_order_id: self.platform_order_id,
            shop_id: self.shop_id,
            receiver_name: self.receiver_name,
            receiver_phone: self.receiver_phone,
            receiver_address: self.receiver_address,
            carrier_code: self.carrier_code,
            tracking_no: self.tracking_no,
            exception_flags: self
                .exception_flags
                .as_ref()
                .map(json_array_to_vec)
                .unwrap_or_default(),
            tags: self.tags.unwrap_or_default(),
            doc_no: self.doc_no,
            status: self.status,
            outbound_status: self.outbound_status,
            pick_status: self.pick_status,
            verify_status: self.verify_status,
            pack_status: self.pack_status,
            ship_status: self.ship_status,
            picked_at: self.picked_at,
            packed_at: self.packed_at,
            shipped_at: self.shipped_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WaveRow {
    wave_id: uuid::Uuid,
    warehouse_id: uuid::Uuid,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl WaveRow {
    fn into_wave(self, tenant_id: TenantId) -> Wave {
        Wave {
            wave_id: self.wave_id,
            tenant_id,
            warehouse_id: self.warehouse_id,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
