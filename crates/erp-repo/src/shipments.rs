use anyhow::anyhow;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use erp_types::TenantId;
use serde_json::json;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

use crate::fulfillment::ShipmentCancelRequest;

pub const SHIPMENT_REQUESTED_EVENT: &str = "SHIPMENT_REQUESTED";
pub const SHIPMENT_FAILED_EVENT: &str = "SHIPMENT_FAILED";
pub const SHIPMENT_SHIPPED_EVENT: &str = "SHIPMENT_SHIPPED";
pub const SHIPMENT_CONFIRMED_EVENT: &str = "SHIPMENT_CONFIRMED";
pub const SHIPMENT_TYPE_PLATFORM: &str = "PLATFORM";
const SHIPMENT_STATUS_VERSION_REQUESTED: i32 = 10;
const SHIPMENT_STATUS_VERSION_FAILED: i32 = 20;
const SHIPMENT_STATUS_VERSION_SUCCESS: i32 = 30;

#[derive(Debug, Clone)]
pub struct Shipment {
    pub shipment_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shipment_type: String,
    pub shop_id: uuid::Uuid,
    pub order_id: uuid::Uuid,
    pub platform: String,
    pub platform_order_id: String,
    pub warehouse_id: Option<uuid::Uuid>,
    pub shipment_no: Option<String>,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub status: String,
    pub ship_status: String,
    pub requested_at: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub weight_gram: Option<i32>,
    pub sender_address_id: Option<uuid::Uuid>,
    pub last_error: String,
    pub last_push_error: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ShipmentCreateInput {
    pub shipment_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shipment_type: String,
    pub shop_id: uuid::Uuid,
    pub order_id: uuid::Uuid,
    pub platform: String,
    pub platform_order_id: String,
    pub warehouse_id: uuid::Uuid,
    pub shipment_no: Option<String>,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub weight_gram: Option<i32>,
    pub sender_address_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct ShipmentPreprocessUpdate {
    pub warehouse_id: Option<uuid::Uuid>,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub weight_gram: Option<i32>,
    pub sender_address_id: Option<uuid::Uuid>,
}

pub struct ShipmentListFilters {
    pub shop_id: Option<uuid::Uuid>,
    pub shop_ids: Option<Vec<uuid::Uuid>>,
    pub status: Option<String>,
    pub platform: Option<String>,
    pub shipment_type: Option<String>,
    pub tracking_no: Option<String>,
    pub updated_from: Option<DateTime<Utc>>,
    pub updated_to: Option<DateTime<Utc>>,
    pub limit: i64,
    pub offset: i64,
}

pub struct ShipmentListResult {
    pub items: Vec<Shipment>,
}

#[derive(Debug, Clone)]
pub struct ShipmentPackage {
    pub package_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shipment_id: uuid::Uuid,
    pub package_no: String,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub weight_gram: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ShipmentPackageUpsertInput {
    pub package_no: String,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub weight_gram: Option<i32>,
}

pub struct ShipmentsRepo<'a> {
    pool: &'a PgPool,
}

fn month_start(dt: DateTime<Utc>) -> NaiveDate {
    let d = dt.date_naive();
    NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid date")
}

impl<'a> ShipmentsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_requested(
        &self,
        input: ShipmentCreateInput,
    ) -> anyhow::Result<(Shipment, bool)> {
        let shipment_no = input
            .shipment_no
            .clone()
            .or_else(|| Some(input.shipment_id.to_string()));
        let inserted = sqlx::query(
            r#"
            insert into shipments(
                shipment_id, tenant_id, shop_id, order_id, platform, platform_order_id,
                warehouse_id, shipment_no, carrier_code, tracking_no, status, status_version, requested_at,
                updated_at, shipment_type, weight_gram, sender_address_id, ship_status, last_push_error
            ) values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'REQUESTED',$11,$12,now(),$13,$14,$15,'NOT_SHIPPED','')
            on conflict do nothing
            "#,
        )
        .bind(input.shipment_id)
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(input.shop_id)
        .bind(input.order_id)
        .bind(&input.platform)
        .bind(&input.platform_order_id)
        .bind(input.warehouse_id)
        .bind(&shipment_no)
        .bind(&input.carrier_code)
        .bind(&input.tracking_no)
        .bind(SHIPMENT_STATUS_VERSION_REQUESTED)
        .bind(input.requested_at)
        .bind(&input.shipment_type)
        .bind(input.weight_gram)
        .bind(input.sender_address_id)
        .execute(self.pool)
        .await?
        .rows_affected()
            > 0;

        if inserted {
            let shipment = self
                .get_by_id(input.tenant_id, input.shipment_id)
                .await?
                .ok_or_else(|| anyhow!("shipment not found after insert"))?;
            return Ok((shipment, true));
        }

        if let Some(existing) = self
            .get_by_order_id_with_type(input.tenant_id, input.order_id, &input.shipment_type)
            .await?
        {
            return Ok((existing, false));
        }

        if let Some(existing) = self.get_by_id(input.tenant_id, input.shipment_id).await? {
            if existing.order_id != input.order_id {
                return Err(anyhow!("idempotency key conflict"));
            }
            return Ok((existing, false));
        }

        Err(anyhow!("shipment insert failed"))
    }

    pub async fn get_by_order_id(
        &self,
        tenant_id: TenantId,
        order_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Shipment>> {
        self.get_by_order_id_with_type(tenant_id, order_id, SHIPMENT_TYPE_PLATFORM)
            .await
    }

    pub async fn get_by_order_id_with_type(
        &self,
        tenant_id: TenantId,
        order_id: uuid::Uuid,
        shipment_type: &str,
    ) -> anyhow::Result<Option<Shipment>> {
        let row = sqlx::query_as::<_, ShipmentRow>(
            r#"
            select
                shipment_id, shop_id, order_id, platform, platform_order_id,
                warehouse_id, shipment_no, carrier_code, tracking_no, status, ship_status,
                requested_at, shipped_at, weight_gram, sender_address_id, last_error, last_push_error,
                created_at, updated_at,
                shipment_type
            from shipments
            where tenant_id = $1 and order_id = $2
              and shipment_type = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(order_id)
        .bind(shipment_type)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_shipment(tenant_id)))
    }

    pub async fn get_by_id(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Shipment>> {
        let row = sqlx::query_as::<_, ShipmentRow>(
            r#"
            select
                shipment_id, shop_id, order_id, platform, platform_order_id,
                warehouse_id, shipment_no, carrier_code, tracking_no, status, ship_status,
                requested_at, shipped_at, weight_gram, sender_address_id, last_error, last_push_error,
                created_at, updated_at,
                shipment_type
            from shipments
            where tenant_id = $1 and shipment_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_shipment(tenant_id)))
    }

    pub async fn update_logistics(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        carrier_code: Option<String>,
        tracking_no: Option<String>,
    ) -> anyhow::Result<Shipment> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let current: ShipmentRow = sqlx::query_as(
            r#"
            select
                shipment_id, shop_id, order_id, platform, platform_order_id,
                warehouse_id, shipment_no, carrier_code, tracking_no, status, ship_status,
                requested_at, shipped_at, weight_gram, last_error, last_push_error,
                created_at, updated_at,
                shipment_type
            from shipments
            where tenant_id = $1 and shipment_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("shipment not found"))?;

        let next_carrier = carrier_code.or_else(|| current.carrier_code.clone());
        let next_tracking = tracking_no.or_else(|| current.tracking_no.clone());

        let updated: ShipmentRow = sqlx::query_as(
            r#"
            update shipments
            set carrier_code = $3,
                tracking_no = $4,
                updated_at = now()
            where tenant_id = $1 and shipment_id = $2
            returning
                shipment_id, shop_id, order_id, platform, platform_order_id,
                warehouse_id, shipment_no, carrier_code, tracking_no, status, ship_status,
                requested_at, shipped_at, weight_gram, last_error, last_push_error,
                created_at, updated_at,
                shipment_type
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(next_carrier.clone())
        .bind(next_tracking.clone())
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated.into_shipment(tenant_id))
    }

    pub async fn request_cancel(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        reason: Option<String>,
        requested_by: Option<uuid::Uuid>,
    ) -> anyhow::Result<ShipmentCancelRequest> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct ShipmentRowLite {
            order_id: uuid::Uuid,
        }

        let shipment: ShipmentRowLite = sqlx::query_as(
            r#"
            select order_id
            from shipments
            where tenant_id = $1
              and shipment_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow!("shipment not found"))?;

        #[derive(sqlx::FromRow)]
        struct Row {
            cancel_request_id: uuid::Uuid,
            shipment_id: uuid::Uuid,
            order_id: uuid::Uuid,
            status: String,
            reason: Option<String>,
            requested_by: Option<uuid::Uuid>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into shipment_cancel_requests(
                tenant_id, shipment_id, order_id, reason, requested_by, status, created_at, updated_at
            ) values ($1,$2,$3,$4,$5,'PENDING',now(),now())
            on conflict (tenant_id, shipment_id)
            do update set reason = excluded.reason, updated_at = now()
            returning cancel_request_id, shipment_id, order_id, status, reason, requested_by, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(shipment.order_id)
        .bind(reason.clone())
        .bind(requested_by)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(ShipmentCancelRequest {
            cancel_request_id: row.cancel_request_id,
            tenant_id,
            shipment_id: row.shipment_id,
            order_id: row.order_id,
            status: row.status,
            reason: row.reason,
            requested_by: row.requested_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn mark_success(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        shipped_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update shipments
            set status = 'SUCCESS',
                ship_status = 'SHIPPED',
                last_push_error = '',
                shipped_at = $3,
                status_version = $4,
                updated_at = now()
            where tenant_id = $1 and shipment_id = $2
              and status_version <= $4
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(shipped_at)
        .bind(SHIPMENT_STATUS_VERSION_SUCCESS)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_failed(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update shipments
            set status = 'FAILED',
                ship_status = 'SHIP_FAILED',
                status_version = $3,
                updated_at = now()
            where tenant_id = $1 and shipment_id = $2
              and status_version <= $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(SHIPMENT_STATUS_VERSION_FAILED)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_push_status(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        ship_status: &str,
        last_push_error: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update shipments
            set ship_status = $3,
                last_push_error = $4,
                updated_at = now()
            where tenant_id = $1 and shipment_id = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(ship_status)
        .bind(last_push_error)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_shipments(
        &self,
        tenant_id: TenantId,
        filters: ShipmentListFilters,
    ) -> anyhow::Result<ShipmentListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
                shipment_id, shop_id, order_id, platform, platform_order_id,
                warehouse_id, shipment_no, carrier_code, tracking_no, status, ship_status,
                requested_at, shipped_at, weight_gram, sender_address_id, last_error, last_push_error,
                created_at, updated_at,
                shipment_type
            from shipments
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = filters.shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(shop_ids) = filters.shop_ids
            && !shop_ids.is_empty()
        {
            qb.push(" and shop_id = any(");
            qb.push_bind(shop_ids);
            qb.push(")");
        }
        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(platform) = filters.platform {
            qb.push(" and platform = ");
            qb.push_bind(platform);
        }
        if let Some(shipment_type) = filters.shipment_type {
            qb.push(" and shipment_type = ");
            qb.push_bind(shipment_type);
        }
        if let Some(tracking_no) = filters.tracking_no {
            qb.push(" and tracking_no = ");
            qb.push_bind(tracking_no);
        }
        if let Some(updated_from) = filters.updated_from {
            qb.push(" and updated_at >= ");
            qb.push_bind(updated_from);
        }
        if let Some(updated_to) = filters.updated_to {
            qb.push(" and updated_at <= ");
            qb.push_bind(updated_to);
        }

        qb.push(" order by updated_at desc, shipment_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<ShipmentRow> = qb.build_query_as().fetch_all(self.pool).await?;
        let items = rows
            .into_iter()
            .map(|row| row.into_shipment(tenant_id))
            .collect();

        Ok(ShipmentListResult { items })
    }

    pub async fn retry_failed_shipment(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        #[derive(sqlx::FromRow)]
        struct Row {
            status: String,
            order_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_order_id: String,
            warehouse_id: Option<uuid::Uuid>,
            carrier_code: Option<String>,
            tracking_no: Option<String>,
        }

        let row = sqlx::query_as::<_, Row>(
            r#"
            select status, order_id, shop_id, platform, platform_order_id,
                   warehouse_id, carrier_code, tracking_no
            from shipments
            where tenant_id = $1 and shipment_id = $2
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Err(anyhow!("shipment not found"));
        };

        if row.status != "FAILED" {
            return Err(anyhow!("shipment status not retryable"));
        }

        let payload = json!({
            "tenant_id": tenant_id,
            "shipment_id": shipment_id,
            "order_id": row.order_id,
            "shop_id": row.shop_id,
            "platform": row.platform,
            "platform_order_id": row.platform_order_id,
            "warehouse_id": row.warehouse_id,
            "carrier_code": row.carrier_code,
            "tracking_no": row.tracking_no,
        });

        let outbox_exists: Option<uuid::Uuid> = sqlx::query_scalar(
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
        .bind(uuid::Uuid::from(tenant_id))
        .bind(SHIPMENT_REQUESTED_EVENT)
        .bind(shipment_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;

        if outbox_exists.is_none() {
            let event_month = month_start(Utc::now());
            let trace_id: Option<String> = None;
            sqlx::query(
                r#"
                insert into outbox_events(tenant_id, event_month, event_type, payload, trace_id)
                values ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(event_month)
            .bind(SHIPMENT_REQUESTED_EVENT)
            .bind(payload)
            .bind(trace_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn update_preprocess_fields(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        input: ShipmentPreprocessUpdate,
    ) -> anyhow::Result<Option<Shipment>> {
        let mut qb = QueryBuilder::<Postgres>::new("update shipments set ");
        let mut separated = qb.separated(", ");
        let mut has_updates = false;

        if let Some(warehouse_id) = input.warehouse_id {
            separated.push("warehouse_id = ");
            separated.push_bind(warehouse_id);
            has_updates = true;
        }
        if let Some(carrier_code) = input.carrier_code {
            separated.push("carrier_code = ");
            separated.push_bind(carrier_code);
            has_updates = true;
        }
        if let Some(tracking_no) = input.tracking_no {
            separated.push("tracking_no = ");
            separated.push_bind(tracking_no);
            has_updates = true;
        }
        if let Some(weight_gram) = input.weight_gram {
            separated.push("weight_gram = ");
            separated.push_bind(weight_gram);
            has_updates = true;
        }
        if let Some(sender_address_id) = input.sender_address_id {
            separated.push("sender_address_id = ");
            separated.push_bind(sender_address_id);
            has_updates = true;
        }

        if !has_updates {
            return self.get_by_id(tenant_id, shipment_id).await;
        }

        separated.push("updated_at = now()");
        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and shipment_id = ");
        qb.push_bind(shipment_id);
        qb.push(
            " returning shipment_id, shop_id, order_id, platform, platform_order_id, warehouse_id, shipment_no, carrier_code, tracking_no, status, ship_status, requested_at, shipped_at, weight_gram, sender_address_id, last_error, last_push_error, created_at, updated_at, shipment_type",
        );

        let row: Option<ShipmentRow> = qb.build_query_as().fetch_optional(self.pool).await?;
        Ok(row.map(|row| row.into_shipment(tenant_id)))
    }

    pub async fn list_packages(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Vec<ShipmentPackage>> {
        let rows: Vec<ShipmentPackageRow> = sqlx::query_as(
            r#"
            select
                package_id, shipment_id, package_no,
                carrier_code, tracking_no, weight_gram,
                created_at, updated_at
            from shipment_packages
            where tenant_id = $1 and shipment_id = $2
            order by created_at asc, package_id asc
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| row.into_package(tenant_id))
            .collect())
    }

    pub async fn upsert_packages(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
        packages: Vec<ShipmentPackageUpsertInput>,
    ) -> anyhow::Result<Vec<ShipmentPackage>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;
        if !packages.is_empty() {
            let mut qb = QueryBuilder::<Postgres>::new(
                r#"
                insert into shipment_packages(
                    tenant_id, shipment_id, package_no, carrier_code, tracking_no, weight_gram
                )
                "#,
            );
            qb.push_values(packages.iter(), |mut b, item| {
                b.push_bind(uuid::Uuid::from(tenant_id))
                    .push_bind(shipment_id)
                    .push_bind(&item.package_no)
                    .push_bind(&item.carrier_code)
                    .push_bind(&item.tracking_no)
                    .push_bind(item.weight_gram);
            });
            qb.push(
                r#"
                on conflict (tenant_id, shipment_id, package_no)
                do update set
                    carrier_code = excluded.carrier_code,
                    tracking_no = excluded.tracking_no,
                    weight_gram = excluded.weight_gram,
                    updated_at = now()
                "#,
            );
            qb.build().execute(&mut *tx).await?;
        }

        if packages.is_empty() {
            sqlx::query(
                r#"
                delete from shipment_packages
                where tenant_id = $1 and shipment_id = $2
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(shipment_id)
            .execute(&mut *tx)
            .await?;
        } else {
            let package_nos = packages
                .iter()
                .map(|item| item.package_no.clone())
                .collect::<Vec<_>>();
            sqlx::query(
                r#"
                delete from shipment_packages
                where tenant_id = $1
                  and shipment_id = $2
                  and not (package_no = any($3))
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(shipment_id)
            .bind(&package_nos)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.list_packages(tenant_id, shipment_id).await
    }
}

#[derive(sqlx::FromRow)]
struct ShipmentRow {
    shipment_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    order_id: uuid::Uuid,
    platform: String,
    platform_order_id: String,
    warehouse_id: Option<uuid::Uuid>,
    shipment_no: Option<String>,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    status: String,
    ship_status: String,
    requested_at: Option<DateTime<Utc>>,
    shipped_at: Option<DateTime<Utc>>,
    weight_gram: Option<i32>,
    sender_address_id: Option<uuid::Uuid>,
    last_error: String,
    last_push_error: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    shipment_type: String,
}

#[derive(sqlx::FromRow)]
struct ShipmentPackageRow {
    package_id: uuid::Uuid,
    shipment_id: uuid::Uuid,
    package_no: String,
    carrier_code: Option<String>,
    tracking_no: Option<String>,
    weight_gram: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ShipmentPackageRow {
    fn into_package(self, tenant_id: TenantId) -> ShipmentPackage {
        ShipmentPackage {
            package_id: self.package_id,
            tenant_id,
            shipment_id: self.shipment_id,
            package_no: self.package_no,
            carrier_code: self.carrier_code,
            tracking_no: self.tracking_no,
            weight_gram: self.weight_gram,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

impl ShipmentRow {
    fn into_shipment(self, tenant_id: TenantId) -> Shipment {
        Shipment {
            shipment_id: self.shipment_id,
            tenant_id,
            shipment_type: self.shipment_type,
            shop_id: self.shop_id,
            order_id: self.order_id,
            platform: self.platform,
            platform_order_id: self.platform_order_id,
            warehouse_id: self.warehouse_id,
            shipment_no: self.shipment_no,
            carrier_code: self.carrier_code,
            tracking_no: self.tracking_no,
            status: self.status,
            ship_status: self.ship_status,
            requested_at: self.requested_at,
            shipped_at: self.shipped_at,
            weight_gram: self.weight_gram,
            sender_address_id: self.sender_address_id,
            last_error: self.last_error,
            last_push_error: self.last_push_error,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
