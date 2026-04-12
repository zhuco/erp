use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, QueryBuilder};

pub const SHIPMENT_PUSH_STATUS_SUCCESS: &str = "SUCCESS";
pub const SHIPMENT_PUSH_STATUS_FAILED: &str = "FAILED";
pub const SHIPMENT_PUSH_STATUS_RETRYING: &str = "RETRYING";
pub const SHIPMENT_PUSH_STATUS_DEAD: &str = "DEAD";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentPushAttempt {
    pub attempt_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub outbox_id: uuid::Uuid,
    pub shipment_id: uuid::Uuid,
    pub shop_id: uuid::Uuid,
    pub platform: String,
    pub platform_order_id: String,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub status: String,
    pub attempt: i32,
    pub duration_ms: i32,
    pub error_code: String,
    pub error_message: String,
    pub request_json: Value,
    pub response_json: Value,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub dead_letter_id: Option<uuid::Uuid>,
    pub trigger: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ShipmentPushAttemptCreate {
    pub tenant_id: TenantId,
    pub outbox_id: uuid::Uuid,
    pub shipment_id: uuid::Uuid,
    pub shop_id: uuid::Uuid,
    pub platform: String,
    pub platform_order_id: String,
    pub carrier_code: Option<String>,
    pub tracking_no: Option<String>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub status: String,
    pub attempt: i32,
    pub duration_ms: i32,
    pub error_code: String,
    pub error_message: String,
    pub request_json: Value,
    pub response_json: Value,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub dead_letter_id: Option<uuid::Uuid>,
    pub trigger: String,
}

#[derive(Debug, Clone)]
pub struct ShipmentPushAttemptCursor {
    pub created_at: DateTime<Utc>,
    pub attempt_id: uuid::Uuid,
}

impl ShipmentPushAttemptCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.attempt_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let attempt_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            attempt_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ShipmentPushAttemptListFilters {
    pub status: Option<String>,
    pub platform_order_id: Option<String>,
    pub cursor: Option<ShipmentPushAttemptCursor>,
    pub limit: i64,
}

pub struct ShipmentPushAttemptListResult {
    pub items: Vec<ShipmentPushAttempt>,
    pub next_cursor: Option<ShipmentPushAttemptCursor>,
}

pub struct ShipmentPushAttemptsRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ShipmentPushAttemptsRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_attempt(
        &self,
        input: ShipmentPushAttemptCreate,
    ) -> anyhow::Result<uuid::Uuid> {
        let ShipmentPushAttemptCreate {
            tenant_id,
            outbox_id,
            shipment_id,
            shop_id,
            platform,
            platform_order_id,
            carrier_code,
            tracking_no,
            shipped_at,
            status,
            attempt,
            duration_ms,
            error_code,
            error_message,
            request_json,
            response_json,
            next_retry_at,
            dead_letter_id,
            trigger,
        } = input;

        let attempt_id: uuid::Uuid = sqlx::query_scalar(
            r#"
            insert into shipment_push_attempts(
              tenant_id,
              outbox_id,
              shipment_id,
              shop_id,
              platform,
              platform_order_id,
              carrier_code,
              tracking_no,
              shipped_at,
              status,
              attempt,
              duration_ms,
              error_code,
              error_message,
              request_json,
              response_json,
              next_retry_at,
              dead_letter_id,
              trigger
            ) values (
              $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19
            )
            returning attempt_id
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .bind(shipment_id)
        .bind(shop_id)
        .bind(platform)
        .bind(platform_order_id)
        .bind(carrier_code)
        .bind(tracking_no)
        .bind(shipped_at)
        .bind(status)
        .bind(attempt)
        .bind(duration_ms)
        .bind(error_code)
        .bind(error_message)
        .bind(request_json)
        .bind(response_json)
        .bind(next_retry_at)
        .bind(dead_letter_id)
        .bind(trigger)
        .fetch_one(self.pool)
        .await?;

        Ok(attempt_id)
    }

    pub async fn update_attempt_outcome(
        &self,
        tenant_id: TenantId,
        outbox_id: uuid::Uuid,
        attempt: i32,
        status: &str,
        next_retry_at: Option<DateTime<Utc>>,
        dead_letter_id: Option<uuid::Uuid>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            update shipment_push_attempts
            set status = $4,
                next_retry_at = $5,
                dead_letter_id = $6
            where tenant_id = $1 and outbox_id = $2 and attempt = $3
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(outbox_id)
        .bind(attempt)
        .bind(status)
        .bind(next_retry_at)
        .bind(dead_letter_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_attempts(
        &self,
        tenant_id: TenantId,
        filters: ShipmentPushAttemptListFilters,
    ) -> anyhow::Result<ShipmentPushAttemptListResult> {
        #[derive(sqlx::FromRow)]
        struct Row {
            attempt_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            outbox_id: uuid::Uuid,
            shipment_id: uuid::Uuid,
            shop_id: uuid::Uuid,
            platform: String,
            platform_order_id: String,
            carrier_code: Option<String>,
            tracking_no: Option<String>,
            shipped_at: Option<DateTime<Utc>>,
            status: String,
            attempt: i32,
            duration_ms: i32,
            error_code: String,
            error_message: String,
            request_json: Value,
            response_json: Value,
            next_retry_at: Option<DateTime<Utc>>,
            dead_letter_id: Option<uuid::Uuid>,
            trigger: String,
            created_at: DateTime<Utc>,
        }

        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select
              attempt_id,
              tenant_id,
              outbox_id,
              shipment_id,
              shop_id,
              platform,
              platform_order_id,
              carrier_code,
              tracking_no,
              shipped_at,
              status,
              attempt,
              duration_ms,
              error_code,
              error_message,
              request_json,
              response_json,
              next_retry_at,
              dead_letter_id,
              trigger,
              created_at
            from shipment_push_attempts
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(platform_order_id) = filters.platform_order_id {
            qb.push(" and platform_order_id = ");
            qb.push_bind(platform_order_id);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, attempt_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.attempt_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, attempt_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| ShipmentPushAttemptCursor {
            created_at: row.created_at,
            attempt_id: row.attempt_id,
        });
        let items = rows
            .into_iter()
            .map(|row| ShipmentPushAttempt {
                attempt_id: row.attempt_id,
                tenant_id: TenantId(row.tenant_id),
                outbox_id: row.outbox_id,
                shipment_id: row.shipment_id,
                shop_id: row.shop_id,
                platform: row.platform,
                platform_order_id: row.platform_order_id,
                carrier_code: row.carrier_code,
                tracking_no: row.tracking_no,
                shipped_at: row.shipped_at,
                status: row.status,
                attempt: row.attempt,
                duration_ms: row.duration_ms,
                error_code: row.error_code,
                error_message: row.error_message,
                request_json: row.request_json,
                response_json: row.response_json,
                next_retry_at: row.next_retry_at,
                dead_letter_id: row.dead_letter_id,
                trigger: row.trigger,
                created_at: row.created_at,
            })
            .collect();

        Ok(ShipmentPushAttemptListResult { items, next_cursor })
    }
}
