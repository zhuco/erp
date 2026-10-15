use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};

pub const WAYBILL_STATUS_AVAILABLE: &str = "AVAILABLE";
pub const WAYBILL_STATUS_RESERVED: &str = "RESERVED";
pub const WAYBILL_STATUS_USED: &str = "USED";
pub const WAYBILL_STATUS_CANCELLED: &str = "CANCELLED";

#[derive(Debug, Clone)]
pub struct Waybill {
    pub waybill_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub shop_id: uuid::Uuid,
    pub carrier_code: String,
    pub waybill_no: String,
    pub status: String,
    pub reserved_at: Option<DateTime<Utc>>,
    pub used_at: Option<DateTime<Utc>>,
    pub shipment_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct WaybillListFilters {
    pub shop_id: Option<uuid::Uuid>,
    pub carrier_code: Option<String>,
    pub status: Option<String>,
    pub shipment_id: Option<uuid::Uuid>,
    pub waybill_no: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct WaybillListResult {
    pub items: Vec<Waybill>,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct WaybillStatusSummary {
    pub available: i64,
    pub reserved: i64,
    pub used: i64,
    pub cancelled: i64,
}

#[derive(Debug, Clone)]
pub struct WaybillImportResult {
    pub inserted: i64,
    pub total: i64,
}

pub struct WaybillsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> WaybillsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn import_waybills(
        &self,
        tenant_id: TenantId,
        shop_id: uuid::Uuid,
        carrier_code: &str,
        waybills: &[String],
    ) -> anyhow::Result<WaybillImportResult> {
        if waybills.is_empty() {
            return Ok(WaybillImportResult {
                inserted: 0,
                total: 0,
            });
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            "insert into waybills(tenant_id, shop_id, carrier_code, waybill_no, status) ",
        );
        qb.push_values(waybills, |mut row, waybill_no| {
            row.push_bind(uuid::Uuid::from(tenant_id));
            row.push_bind(shop_id);
            row.push_bind(carrier_code);
            row.push_bind(waybill_no);
            row.push_bind(WAYBILL_STATUS_AVAILABLE);
        });
        qb.push(" on conflict (tenant_id, carrier_code, waybill_no) do nothing");

        let result = qb.build().execute(self.pool).await?;
        Ok(WaybillImportResult {
            inserted: result.rows_affected() as i64,
            total: waybills.len() as i64,
        })
    }

    pub async fn reserve_waybill(
        &self,
        tenant_id: TenantId,
        shop_id: uuid::Uuid,
        carrier_code: &str,
        shipment_id: uuid::Uuid,
        preferred_waybill_no: Option<&str>,
    ) -> anyhow::Result<Waybill> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let existing = sqlx::query_as::<_, WaybillRow>(
            r#"
            select waybill_id, shop_id, carrier_code, waybill_no, status,
                   reserved_at, used_at, shipment_id
            from waybills
            where tenant_id = $1
              and shipment_id = $2
              and status in ($3, $4)
            limit 1
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(WAYBILL_STATUS_RESERVED)
        .bind(WAYBILL_STATUS_USED)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = existing {
            if let Some(preferred) = preferred_waybill_no
                && row.waybill_no != preferred
            {
                return Err(anyhow!("shipment already reserved with another waybill"));
            }
            tx.commit().await?;
            return Ok(row.into_waybill(tenant_id));
        }

        let candidate = if let Some(waybill_no) = preferred_waybill_no {
            sqlx::query_as::<_, WaybillRow>(
                r#"
                select waybill_id, shop_id, carrier_code, waybill_no, status,
                       reserved_at, used_at, shipment_id
                from waybills
                where tenant_id = $1
                  and shop_id = $2
                  and carrier_code = $3
                  and waybill_no = $4
                  and status = $5
                limit 1
                for update
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(shop_id)
            .bind(carrier_code)
            .bind(waybill_no)
            .bind(WAYBILL_STATUS_AVAILABLE)
            .fetch_optional(&mut *tx)
            .await?
        } else {
            sqlx::query_as::<_, WaybillRow>(
                r#"
                select waybill_id, shop_id, carrier_code, waybill_no, status,
                       reserved_at, used_at, shipment_id
                from waybills
                where tenant_id = $1
                  and shop_id = $2
                  and carrier_code = $3
                  and status = $4
                order by waybill_no asc
                limit 1
                for update skip locked
                "#,
            )
            .bind(uuid::Uuid::from(tenant_id))
            .bind(shop_id)
            .bind(carrier_code)
            .bind(WAYBILL_STATUS_AVAILABLE)
            .fetch_optional(&mut *tx)
            .await?
        };

        let Some(candidate) = candidate else {
            return Err(anyhow!("no available waybill"));
        };

        let reserved = sqlx::query_as::<_, WaybillRow>(
            r#"
            update waybills
            set status = $2,
                reserved_at = now(),
                shipment_id = $3
            where waybill_id = $1
            returning waybill_id, shop_id, carrier_code, waybill_no, status,
                      reserved_at, used_at, shipment_id
            "#,
        )
        .bind(candidate.waybill_id)
        .bind(WAYBILL_STATUS_RESERVED)
        .bind(shipment_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(reserved.into_waybill(tenant_id))
    }

    pub async fn mark_used(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Waybill>> {
        let mut tx: Transaction<'_, Postgres> = self.pool.begin().await?;

        let existing = sqlx::query_as::<_, WaybillRow>(
            r#"
            select waybill_id, shop_id, carrier_code, waybill_no, status,
                   reserved_at, used_at, shipment_id
            from waybills
            where tenant_id = $1
              and shipment_id = $2
              and status = $3
            limit 1
            for update
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(WAYBILL_STATUS_USED)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = existing {
            tx.commit().await?;
            return Ok(Some(row.into_waybill(tenant_id)));
        }

        let updated = sqlx::query_as::<_, WaybillRow>(
            r#"
            update waybills
            set status = $3,
                used_at = now()
            where tenant_id = $1
              and shipment_id = $2
              and status = $4
            returning waybill_id, shop_id, carrier_code, waybill_no, status,
                      reserved_at, used_at, shipment_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(WAYBILL_STATUS_USED)
        .bind(WAYBILL_STATUS_RESERVED)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated.map(|row| row.into_waybill(tenant_id)))
    }

    pub async fn release_by_shipment(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Waybill>> {
        let row = sqlx::query_as::<_, WaybillRow>(
            r#"
            update waybills
            set status = $3,
                reserved_at = null,
                shipment_id = null
            where tenant_id = $1
              and shipment_id = $2
              and status = $4
            returning waybill_id, shop_id, carrier_code, waybill_no, status,
                      reserved_at, used_at, shipment_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(WAYBILL_STATUS_AVAILABLE)
        .bind(WAYBILL_STATUS_RESERVED)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_waybill(tenant_id)))
    }

    pub async fn cancel_by_shipment(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Waybill>> {
        let row = sqlx::query_as::<_, WaybillRow>(
            r#"
            update waybills
            set status = $3
            where tenant_id = $1
              and shipment_id = $2
              and status = $4
            returning waybill_id, shop_id, carrier_code, waybill_no, status,
                      reserved_at, used_at, shipment_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .bind(WAYBILL_STATUS_CANCELLED)
        .bind(WAYBILL_STATUS_RESERVED)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_waybill(tenant_id)))
    }

    pub async fn get_by_shipment_id(
        &self,
        tenant_id: TenantId,
        shipment_id: uuid::Uuid,
    ) -> anyhow::Result<Option<Waybill>> {
        let row = sqlx::query_as::<_, WaybillRow>(
            r#"
            select waybill_id, shop_id, carrier_code, waybill_no, status,
                   reserved_at, used_at, shipment_id
            from waybills
            where tenant_id = $1
              and shipment_id = $2
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(shipment_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| row.into_waybill(tenant_id)))
    }

    pub async fn list_waybills(
        &self,
        tenant_id: TenantId,
        filters: WaybillListFilters,
    ) -> anyhow::Result<WaybillListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select waybill_id, shop_id, carrier_code, waybill_no, status,
                   reserved_at, used_at, shipment_id
            from waybills
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = filters.shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(carrier_code) = filters.carrier_code.as_deref() {
            qb.push(" and carrier_code = ");
            qb.push_bind(carrier_code);
        }
        if let Some(status) = filters.status.as_deref() {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(shipment_id) = filters.shipment_id {
            qb.push(" and shipment_id = ");
            qb.push_bind(shipment_id);
        }
        if let Some(waybill_no) = filters.waybill_no.as_deref() {
            qb.push(" and waybill_no = ");
            qb.push_bind(waybill_no);
        }

        qb.push(" order by waybill_no asc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);
        qb.push(" offset ");
        qb.push_bind(filters.offset);

        let rows: Vec<WaybillRow> = qb.build_query_as().fetch_all(self.pool).await?;

        let mut count_qb = QueryBuilder::<Postgres>::new(
            r#"
            select count(*)::bigint
            from waybills
            where tenant_id =
            "#,
        );
        count_qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = filters.shop_id {
            count_qb.push(" and shop_id = ");
            count_qb.push_bind(shop_id);
        }
        if let Some(carrier_code) = filters.carrier_code.as_deref() {
            count_qb.push(" and carrier_code = ");
            count_qb.push_bind(carrier_code);
        }
        if let Some(status) = filters.status.as_deref() {
            count_qb.push(" and status = ");
            count_qb.push_bind(status);
        }
        if let Some(shipment_id) = filters.shipment_id {
            count_qb.push(" and shipment_id = ");
            count_qb.push_bind(shipment_id);
        }
        if let Some(waybill_no) = filters.waybill_no.as_deref() {
            count_qb.push(" and waybill_no = ");
            count_qb.push_bind(waybill_no);
        }

        let total: i64 = count_qb.build_query_scalar().fetch_one(self.pool).await?;

        let items = rows
            .into_iter()
            .map(|row| row.into_waybill(tenant_id))
            .collect();

        Ok(WaybillListResult { items, total })
    }

    pub async fn summarize_status(
        &self,
        tenant_id: TenantId,
        shop_id: Option<uuid::Uuid>,
        carrier_code: Option<String>,
    ) -> anyhow::Result<WaybillStatusSummary> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select status, count(*)::bigint
            from waybills
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(shop_id) = shop_id {
            qb.push(" and shop_id = ");
            qb.push_bind(shop_id);
        }
        if let Some(carrier_code) = carrier_code {
            qb.push(" and carrier_code = ");
            qb.push_bind(carrier_code);
        }

        qb.push(" group by status");

        let rows: Vec<(String, i64)> = qb.build_query_as().fetch_all(self.pool).await?;
        let mut summary = WaybillStatusSummary {
            available: 0,
            reserved: 0,
            used: 0,
            cancelled: 0,
        };
        for (status, count) in rows {
            match status.as_str() {
                WAYBILL_STATUS_AVAILABLE => summary.available = count,
                WAYBILL_STATUS_RESERVED => summary.reserved = count,
                WAYBILL_STATUS_USED => summary.used = count,
                WAYBILL_STATUS_CANCELLED => summary.cancelled = count,
                _ => {}
            }
        }
        Ok(summary)
    }
}

#[derive(sqlx::FromRow)]
struct WaybillRow {
    waybill_id: uuid::Uuid,
    shop_id: uuid::Uuid,
    carrier_code: String,
    waybill_no: String,
    status: String,
    reserved_at: Option<DateTime<Utc>>,
    used_at: Option<DateTime<Utc>>,
    shipment_id: Option<uuid::Uuid>,
}

impl WaybillRow {
    fn into_waybill(self, tenant_id: TenantId) -> Waybill {
        Waybill {
            waybill_id: self.waybill_id,
            tenant_id,
            shop_id: self.shop_id,
            carrier_code: self.carrier_code,
            waybill_no: self.waybill_no,
            status: self.status,
            reserved_at: self.reserved_at,
            used_at: self.used_at,
            shipment_id: self.shipment_id,
        }
    }
}
