use anyhow::anyhow;
use chrono::{DateTime, Utc};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDevice {
    pub device_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub name: String,
    pub fingerprint: String,
    pub status: String,
    pub is_default: bool,
    #[serde(skip_serializing)]
    pub device_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PrintDeviceCursor {
    pub created_at: DateTime<Utc>,
    pub device_id: uuid::Uuid,
}

impl PrintDeviceCursor {
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at.to_rfc3339(), self.device_id)
    }

    pub fn decode(raw: &str) -> anyhow::Result<Self> {
        let (ts, id) = raw
            .split_once('|')
            .ok_or_else(|| anyhow!("invalid cursor"))?;
        let created_at = DateTime::parse_from_rfc3339(ts)
            .map_err(|_| anyhow!("invalid cursor"))?
            .with_timezone(&Utc);
        let device_id = uuid::Uuid::parse_str(id).map_err(|_| anyhow!("invalid cursor"))?;
        Ok(Self {
            created_at,
            device_id,
        })
    }
}

pub struct PrintDeviceListFilters {
    pub fingerprint: Option<String>,
    pub status: Option<String>,
    pub cursor: Option<PrintDeviceCursor>,
    pub limit: i64,
}

pub struct PrintDeviceListResult {
    pub items: Vec<PrintDevice>,
    pub next_cursor: Option<PrintDeviceCursor>,
}

pub struct PrintDeviceCreate {
    pub tenant_id: TenantId,
    pub name: String,
    pub fingerprint: String,
    pub status: String,
    pub is_default: bool,
    pub device_token: String,
}

pub struct PrintDeviceUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
    pub is_default: Option<bool>,
}

pub struct PrintDevicesRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> PrintDevicesRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_devices(
        &self,
        tenant_id: TenantId,
        filters: PrintDeviceListFilters,
    ) -> anyhow::Result<PrintDeviceListResult> {
        let mut qb = QueryBuilder::<Postgres>::new(
            r#"
            select device_id, name, fingerprint, status, is_default, device_token, created_at, updated_at
            from print_devices
            where tenant_id =
            "#,
        );
        qb.push_bind(uuid::Uuid::from(tenant_id));

        if let Some(fingerprint) = filters.fingerprint {
            qb.push(" and fingerprint = ");
            qb.push_bind(fingerprint);
        }
        if let Some(status) = filters.status {
            qb.push(" and status = ");
            qb.push_bind(status);
        }
        if let Some(cursor) = filters.cursor {
            qb.push(" and (created_at, device_id) < (");
            qb.push_bind(cursor.created_at);
            qb.push(", ");
            qb.push_bind(cursor.device_id);
            qb.push(")");
        }

        qb.push(" order by created_at desc, device_id desc");
        qb.push(" limit ");
        qb.push_bind(filters.limit);

        #[derive(sqlx::FromRow)]
        struct Row {
            device_id: uuid::Uuid,
            name: String,
            fingerprint: String,
            status: String,
            is_default: bool,
            device_token: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let rows: Vec<Row> = qb.build_query_as().fetch_all(self.pool).await?;
        let next_cursor = rows.last().map(|row| PrintDeviceCursor {
            created_at: row.created_at,
            device_id: row.device_id,
        });

        let items = rows
            .into_iter()
            .map(|row| PrintDevice {
                device_id: row.device_id,
                tenant_id,
                name: row.name,
                fingerprint: row.fingerprint,
                status: row.status,
                is_default: row.is_default,
                device_token: row.device_token,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok(PrintDeviceListResult { items, next_cursor })
    }

    pub async fn create_device(&self, input: PrintDeviceCreate) -> anyhow::Result<PrintDevice> {
        #[derive(sqlx::FromRow)]
        struct Row {
            device_id: uuid::Uuid,
            name: String,
            fingerprint: String,
            status: String,
            is_default: bool,
            device_token: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Row = sqlx::query_as(
            r#"
            insert into print_devices(tenant_id, name, fingerprint, status, is_default, device_token)
            values ($1, $2, $3, $4, $5, $6)
            on conflict (tenant_id, fingerprint)
            do update set
                name = excluded.name,
                status = excluded.status,
                is_default = excluded.is_default,
                device_token = excluded.device_token,
                updated_at = now()
            returning device_id, name, fingerprint, status, is_default, device_token, created_at, updated_at
            "#,
        )
        .bind(uuid::Uuid::from(input.tenant_id))
        .bind(&input.name)
        .bind(&input.fingerprint)
        .bind(&input.status)
        .bind(input.is_default)
        .bind(&input.device_token)
        .fetch_one(self.pool)
        .await?;

        Ok(PrintDevice {
            device_id: row.device_id,
            tenant_id: input.tenant_id,
            name: row.name,
            fingerprint: row.fingerprint,
            status: row.status,
            is_default: row.is_default,
            device_token: row.device_token,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    pub async fn update_device(
        &self,
        tenant_id: TenantId,
        device_id: uuid::Uuid,
        input: PrintDeviceUpdate,
    ) -> anyhow::Result<Option<PrintDevice>> {
        let mut qb = QueryBuilder::<Postgres>::new("update print_devices set ");
        let mut separated = qb.separated(", ");

        if let Some(name) = input.name {
            separated.push("name = ");
            separated.push_bind(name);
        }
        if let Some(status) = input.status {
            separated.push("status = ");
            separated.push_bind(status);
        }
        if let Some(is_default) = input.is_default {
            separated.push("is_default = ");
            separated.push_bind(is_default);
        }

        separated.push("updated_at = now()");

        qb.push(" where tenant_id = ");
        qb.push_bind(uuid::Uuid::from(tenant_id));
        qb.push(" and device_id = ");
        qb.push_bind(device_id);
        qb.push(
            " returning device_id, name, fingerprint, status, is_default, device_token, created_at, updated_at",
        );

        #[derive(sqlx::FromRow)]
        struct Row {
            device_id: uuid::Uuid,
            name: String,
            fingerprint: String,
            status: String,
            is_default: bool,
            device_token: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = qb.build_query_as().fetch_optional(self.pool).await?;
        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(PrintDevice {
            device_id: row.device_id,
            tenant_id,
            name: row.name,
            fingerprint: row.fingerprint,
            status: row.status,
            is_default: row.is_default,
            device_token: row.device_token,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn get_default_device(
        &self,
        tenant_id: TenantId,
    ) -> anyhow::Result<Option<PrintDevice>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            device_id: uuid::Uuid,
            name: String,
            fingerprint: String,
            status: String,
            is_default: bool,
            device_token: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select device_id, name, fingerprint, status, is_default, device_token, created_at, updated_at
            from print_devices
            where tenant_id = $1 and status = 'ACTIVE'
            order by is_default desc, updated_at desc, device_id desc
            limit 1
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PrintDevice {
            device_id: row.device_id,
            tenant_id,
            name: row.name,
            fingerprint: row.fingerprint,
            status: row.status,
            is_default: row.is_default,
            device_token: row.device_token,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn get_device_by_fingerprint(
        &self,
        tenant_id: TenantId,
        fingerprint: &str,
    ) -> anyhow::Result<Option<PrintDevice>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            device_id: uuid::Uuid,
            name: String,
            fingerprint: String,
            status: String,
            is_default: bool,
            device_token: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select device_id, name, fingerprint, status, is_default, device_token, created_at, updated_at
            from print_devices
            where tenant_id = $1 and fingerprint = $2
            "#,
        )
        .bind(uuid::Uuid::from(tenant_id))
        .bind(fingerprint)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PrintDevice {
            device_id: row.device_id,
            tenant_id,
            name: row.name,
            fingerprint: row.fingerprint,
            status: row.status,
            is_default: row.is_default,
            device_token: row.device_token,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }

    pub async fn get_device_by_token(&self, token: &str) -> anyhow::Result<Option<PrintDevice>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            device_id: uuid::Uuid,
            tenant_id: uuid::Uuid,
            name: String,
            fingerprint: String,
            status: String,
            is_default: bool,
            device_token: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
        }

        let row: Option<Row> = sqlx::query_as(
            r#"
            select device_id, tenant_id, name, fingerprint, status, is_default, device_token, created_at, updated_at
            from print_devices
            where device_token = $1
            "#,
        )
        .bind(token)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|row| PrintDevice {
            device_id: row.device_id,
            tenant_id: TenantId(row.tenant_id),
            name: row.name,
            fingerprint: row.fingerprint,
            status: row.status,
            is_default: row.is_default,
            device_token: row.device_token,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }))
    }
}
