//! Actor-bound inventory exports. Job identity and audit are one transaction.
use crate::export_jobs::{ExportJob, ExportJobsRepo};
use chrono::{DateTime, Utc};
use erp_domain::quantity::parse_qty;
use erp_grid_core::decimal_text;
use erp_types::{TenantId, UserId};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub const MAX_EXPORT_ROWS: u64 = 1_000_000;
pub const MAX_EXPORT_BYTES: u64 = 1_073_741_824;
pub const MAX_EXPORT_WAREHOUSES: usize = 10_000;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryExportQuery {
    pub warehouse_id: Option<Uuid>,
    pub sku_id: Option<Uuid>,
    pub sku_code: Option<String>,
    pub barcode: Option<String>,
    pub tag: Option<String>,
    pub available_min: Option<String>,
    pub available_max: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryExportRequest {
    pub request_id: Uuid,
    #[serde(default)]
    pub query: InventoryExportQuery,
}

impl InventoryExportRequest {
    pub fn normalize(mut self) -> Result<Self, InventoryExportError> {
        if self.request_id.is_nil()
            || self.query.warehouse_id.is_some_and(|id| id.is_nil())
            || self.query.sku_id.is_some_and(|id| id.is_nil())
        {
            return Err(InventoryExportError::Invalid("nil identity".into()));
        }
        for field in [
            &mut self.query.sku_code,
            &mut self.query.barcode,
            &mut self.query.tag,
        ] {
            if let Some(text) = field {
                let value = text.trim();
                if value.len() > 256 || value.chars().any(char::is_control) {
                    return Err(InventoryExportError::Invalid("invalid text filter".into()));
                }
                *field = (!value.is_empty()).then(|| value.to_owned());
            }
        }
        for text in [&mut self.query.available_min, &mut self.query.available_max]
            .into_iter()
            .flatten()
        {
            let amount = parse_qty(text)
                .map_err(|_| InventoryExportError::Invalid("invalid quantity filter".into()))?;
            if amount.is_negative() {
                return Err(InventoryExportError::Invalid(
                    "negative available filter".into(),
                ));
            }
            *text = decimal_text(i128::from(amount.as_micros()), 6);
        }
        if let (Some(min), Some(max)) = (&self.query.available_min, &self.query.available_max)
            && parse_qty(min)
                .map_err(|_| InventoryExportError::Invalid("invalid minimum".into()))?
                > parse_qty(max)
                    .map_err(|_| InventoryExportError::Invalid("invalid maximum".into()))?
        {
            return Err(InventoryExportError::Invalid(
                "inverted available range".into(),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryExportArtifact {
    pub schema: u8,
    pub quantity_scale: u8,
    pub snapshot_at: DateTime<Utc>,
    pub rows: u64,
    pub bytes: u64,
    pub sha256: String,
}

impl InventoryExportArtifact {
    pub fn validate(&self) -> Result<(), InventoryExportError> {
        if self.schema != 2
            || self.quantity_scale != 6
            || self.rows > MAX_EXPORT_ROWS
            || self.bytes == 0
            || self.bytes > MAX_EXPORT_BYTES
            || self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(InventoryExportError::Invalid(
                "invalid inventory artifact receipt".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryExportSpec {
    pub schema: u8,
    pub request_id: Uuid,
    pub actor_user_id: Uuid,
    pub query: InventoryExportQuery,
    pub warehouse_ids: Vec<Uuid>,
    pub restricted_at_admission: bool,
    pub artifact: Option<InventoryExportArtifact>,
}

#[derive(Debug, thiserror::Error)]
pub enum InventoryExportError {
    #[error("inventory export request identity conflict")]
    Conflict,
    #[error("inventory export access denied")]
    Forbidden,
    #[error("invalid inventory export: {0}")]
    Invalid(String),
    #[error(transparent)]
    Storage(#[from] anyhow::Error),
}

pub struct InventoryExportVisibility {
    pub actor: Uuid,
    pub warehouse_ids: Vec<Uuid>,
    pub restricted: bool,
}

#[derive(sqlx::FromRow)]
struct Authority {
    active: bool,
    permissions: Vec<String>,
    grants: Vec<Uuid>,
    warehouses: Vec<Uuid>,
}

async fn authority(
    conn: &mut PgConnection,
    tenant: TenantId,
    actor: UserId,
) -> Result<Authority, InventoryExportError> {
    sqlx::query_as("select exists(select 1 from users where tenant_id=$1 and user_id=$2 and status='ACTIVE') as active,
      array(select distinct rp.perm_code from user_roles ur join role_permissions rp on rp.tenant_id=ur.tenant_id and rp.role_id=ur.role_id where ur.tenant_id=$1 and ur.user_id=$2) as permissions,
      array(select warehouse_id from user_warehouse_access where tenant_id=$1 and user_id=$2 order by warehouse_id limit 10001) as grants,
      array(select w.warehouse_id from warehouses w where w.tenant_id=$1 and
        (not exists(select 1 from user_warehouse_access where tenant_id=$1 and user_id=$2) or exists(select 1 from user_warehouse_access a where a.tenant_id=$1 and a.user_id=$2 and a.warehouse_id=w.warehouse_id)) order by w.warehouse_id limit 10001) as warehouses")
        .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).fetch_one(conn).await
        .map_err(|error| InventoryExportError::Storage(error.into()))
}

fn allowed(auth: &Authority, write: bool) -> bool {
    let has = |alias: &str, canonical: &str| {
        auth.permissions
            .iter()
            .any(|p| p == alias || p == canonical)
    };
    auth.active
        && has("inventory.read", "WMS.INVENTORY.READ")
        && if write {
            has("exports.write", "OPS.EXPORT.WRITE")
        } else {
            has("exports.read", "OPS.EXPORT.READ")
        }
}

pub async fn visibility(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
) -> Result<Option<InventoryExportVisibility>, InventoryExportError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| InventoryExportError::Storage(e.into()))?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await
        .map_err(|e| InventoryExportError::Storage(e.into()))?;
    let auth = authority(&mut tx, tenant, actor).await?;
    if auth.warehouses.len() > MAX_EXPORT_WAREHOUSES || auth.grants.len() > MAX_EXPORT_WAREHOUSES {
        return Err(InventoryExportError::Invalid(
            "warehouse scope exceeds 10000".into(),
        ));
    }
    Ok(allowed(&auth, false).then(|| InventoryExportVisibility {
        actor: Uuid::from(actor),
        warehouse_ids: auth.warehouses,
        restricted: !auth.grants.is_empty(),
    }))
}

pub fn parse_spec(job: &ExportJob) -> Result<InventoryExportSpec, InventoryExportError> {
    let spec: InventoryExportSpec =
        serde_json::from_value(job.params_json.clone()).map_err(|_| {
            InventoryExportError::Invalid(
                "legacy or malformed inventory job; create a new request".into(),
            )
        })?;
    let normalized = InventoryExportRequest {
        request_id: spec.request_id,
        query: spec.query.clone(),
    }
    .normalize()?;
    if job.job_type != "INVENTORY_EXPORT"
        || spec.schema != 2
        || spec.request_id != job.job_id
        || spec.actor_user_id.is_nil()
        || normalized.query != spec.query
        || spec.warehouse_ids.len() > MAX_EXPORT_WAREHOUSES
        || spec.warehouse_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || spec.warehouse_ids.iter().any(Uuid::is_nil)
        || spec
            .query
            .warehouse_id
            .is_some_and(|id| spec.warehouse_ids != [id])
    {
        return Err(InventoryExportError::Invalid(
            "invalid server inventory scope".into(),
        ));
    }
    if let Some(artifact) = &spec.artifact {
        artifact.validate()?;
    }
    Ok(spec)
}

pub async fn authorize_job(
    pool: &PgPool,
    job: &ExportJob,
    actor: UserId,
    write: bool,
) -> Result<InventoryExportSpec, InventoryExportError> {
    let spec = parse_spec(job)?;
    if spec.actor_user_id != Uuid::from(actor) {
        return Err(InventoryExportError::Forbidden);
    }
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| InventoryExportError::Storage(e.into()))?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await
        .map_err(|e| InventoryExportError::Storage(e.into()))?;
    let auth = authority(&mut tx, job.tenant_id, actor).await?;
    if !allowed(&auth, write)
        || auth.warehouses.len() > MAX_EXPORT_WAREHOUSES
        || auth.grants.len() > MAX_EXPORT_WAREHOUSES
        || (spec.restricted_at_admission && auth.grants.is_empty())
        || spec
            .warehouse_ids
            .iter()
            .any(|id| auth.warehouses.binary_search(id).is_err())
    {
        return Err(InventoryExportError::Forbidden);
    }
    Ok(spec)
}

pub async fn create_job(
    pool: &PgPool,
    tenant: TenantId,
    actor: UserId,
    request: InventoryExportRequest,
) -> Result<ExportJob, InventoryExportError> {
    let request = request.normalize()?;
    // A fresh authority query is mandatory even when a bearer token still has old permissions.
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| InventoryExportError::Storage(e.into()))?;
    sqlx::query("set local statement_timeout = '10s'")
        .execute(&mut *tx)
        .await
        .map_err(|e| InventoryExportError::Storage(e.into()))?;
    let auth = authority(&mut tx, tenant, actor).await?;
    if !allowed(&auth, true) {
        return Err(InventoryExportError::Forbidden);
    }
    if auth.warehouses.len() > MAX_EXPORT_WAREHOUSES || auth.grants.len() > MAX_EXPORT_WAREHOUSES {
        return Err(InventoryExportError::Invalid(
            "warehouse scope exceeds 10000".into(),
        ));
    }
    let warehouses = if let Some(id) = request.query.warehouse_id {
        if auth.warehouses.binary_search(&id).is_err() {
            return Err(InventoryExportError::Forbidden);
        }
        vec![id]
    } else {
        auth.warehouses
    };
    let spec = InventoryExportSpec {
        schema: 2,
        request_id: request.request_id,
        actor_user_id: Uuid::from(actor),
        query: request.query.clone(),
        warehouse_ids: warehouses,
        restricted_at_admission: !auth.grants.is_empty(),
        artifact: None,
    };
    let payload =
        serde_json::to_value(&spec).map_err(|e| InventoryExportError::Storage(e.into()))?;
    let inserted = sqlx::query("insert into export_jobs(job_id,tenant_id,job_type,params_json) values($1,$2,'INVENTORY_EXPORT',$3) on conflict(job_id) do nothing")
        .bind(request.request_id).bind(Uuid::from(tenant)).bind(&payload).execute(&mut *tx).await.map_err(|e| InventoryExportError::Storage(e.into()))?.rows_affected();
    let existing: (Uuid, String, serde_json::Value) = sqlx::query_as(
        "select tenant_id,job_type,params_json from export_jobs where job_id=$1 for share",
    )
    .bind(request.request_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| InventoryExportError::Storage(e.into()))?;
    let same_request = existing.0 == Uuid::from(tenant)
        && existing.1 == "INVENTORY_EXPORT"
        && serde_json::from_value::<InventoryExportSpec>(existing.2.clone())
            .ok()
            .is_some_and(|old| {
                old.schema == 2
                    && old.actor_user_id == spec.actor_user_id
                    && old.request_id == spec.request_id
                    && old.query == spec.query
            });
    if !same_request {
        return Err(InventoryExportError::Conflict);
    }
    if inserted == 1 {
        sqlx::query("insert into audit_logs(tenant_id,user_id,action,entity_type,entity_id,after_json) values($1,$2,'EXPORT.JOB.CREATE','EXPORT_JOB',$3,$4)")
            .bind(Uuid::from(tenant)).bind(Uuid::from(actor)).bind(request.request_id.to_string()).bind(&payload)
            .execute(&mut *tx).await.map_err(|e| InventoryExportError::Storage(e.into()))?;
    }
    tx.commit()
        .await
        .map_err(|e| InventoryExportError::Storage(e.into()))?;
    let job = ExportJobsRepo::new(pool)
        .get_job(tenant, request.request_id)
        .await?
        .ok_or_else(|| {
            InventoryExportError::Storage(anyhow::anyhow!("inventory job missing after admission"))
        })?;
    authorize_job(pool, &job, actor, true).await?;
    Ok(job)
}
