//! Readable source changes and scoped, bounded material document pages.
use super::{
    MaterialDocumentError, MaterialDocumentLine, ReceiptValueActor, SharedContractRepo,
    material_documents, receipt_authority,
};
use erp_types::{TenantId, UserId};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub struct MaterialDocumentSourceAdoption<'a> {
    pub document_id: Uuid,
    pub work_order_id: Uuid,
    pub expected_version: i32,
    pub review_token: &'a str,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct MaterialDocumentSummary {
    pub document_id: Uuid,
    pub work_order_id: Uuid,
    pub version_no: i32,
    pub status: String,
    pub line_count: i64,
    pub source_changed: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl SharedContractRepo<'_> {
    pub async fn review_material_document_source(
        &self,
        tenant: TenantId,
        order: Uuid,
        document: Uuid,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        let mut tx = self.pool.begin().await?;
        let identity = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.read", "MFG.PRODUCTION.READ"],
        )
        .await?;
        let head:Option<(i32,String,Value,Value)>=sqlx::query_as("select d.version_no,d.status,d.source_snapshot,r.snapshot from mfg_material_documents d join mfg_material_document_revisions r using(tenant_id,document_id,version_no) where d.tenant_id=$1 and d.document_id=$2 and d.work_order_id=$3 for share of d,r")
            .bind(tenant.0).bind(document).bind(order).fetch_optional(&mut *tx).await?;
        let (version, status, prior, saved) = head.ok_or(MaterialDocumentError::NotFound)?;
        if status != "DRAFT" {
            return Err(MaterialDocumentError::VersionConflict.into());
        }
        let lines: Vec<MaterialDocumentLine> = serde_json::from_value(saved["lines"].clone())
            .map_err(|_| MaterialDocumentError::ReconciliationRequired)?;
        let permits = material_documents::authorize(&mut tx, tenant, actor, &lines, true).await?;
        super::material_document_commit::verify_current_lines(&mut tx, tenant, document, &lines)
            .await?;
        let current:Option<Value>=sqlx::query_scalar("select jsonb_build_object('work_order_id',work_order_id,'work_order_no',work_order_no,'product_code',product_code,'bom_id',bom_id,'order_qty_micros',order_qty_micros::text) from mfg_work_orders where tenant_id=$1 and work_order_id=$2 and status in('PLANNED','RELEASED','PAUSED') and order_qty_micros>0 for share")
            .bind(tenant.0).bind(order).fetch_optional(&mut *tx).await?;
        let current = current.ok_or(MaterialDocumentError::NotFound)?;
        let token = review_token(
            tenant,
            actor.user_id,
            document,
            order,
            version,
            &prior,
            &current,
        )?;
        let result = json!({"document_id":document,"work_order_id":order,"version_no":version,"changed":prior!=current,"saved_source":prior,
            "current_source":current,"review_token":token});
        identity.recheck(&mut tx).await?;
        for permit in &permits {
            permit.recheck(&mut tx).await?;
        }
        tx.rollback().await?;
        Ok(result)
    }

    pub async fn list_material_documents(
        &self,
        tenant: TenantId,
        order: Uuid,
        offset: i64,
        limit: i64,
        status: Option<&str>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<(i64, Vec<MaterialDocumentSummary>)> {
        if !(0..=1000000).contains(&offset)
            || !(1..=2000).contains(&limit)
            || status.is_some_and(|s| !matches!(s, "DRAFT" | "POSTED"))
        {
            anyhow::bail!("invalid material document query window or status");
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read")
            .execute(&mut *tx)
            .await?;
        let permit = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.read", "MFG.PRODUCTION.READ"],
        )
        .await?;
        let warehouses:Vec<Uuid>=sqlx::query_scalar("select a.warehouse_id from user_warehouse_access a join warehouses w using(tenant_id,warehouse_id) where a.tenant_id=$1 and a.user_id=$2 order by a.warehouse_id limit 10001 for share of a,w")
            .bind(tenant.0).bind(actor.user_id.0).fetch_all(&mut *tx).await?;
        if warehouses.len() > 10000 {
            anyhow::bail!("material warehouse scope budget exceeded");
        }
        let owned: bool = sqlx::query_scalar(
            "select exists(select 1 from mfg_work_orders where tenant_id=$1 and work_order_id=$2)",
        )
        .bind(tenant.0)
        .bind(order)
        .fetch_one(&mut *tx)
        .await?;
        if !owned {
            return Err(MaterialDocumentError::NotFound.into());
        }
        let total:i64=sqlx::query_scalar("select count(*) from mfg_material_documents d where d.tenant_id=$1 and d.work_order_id=$2 and ($3::text is null or d.status=$3) and exists(select 1 from mfg_material_document_lines l where l.tenant_id=d.tenant_id and l.document_id=d.document_id) and not exists(select 1 from mfg_material_document_lines l where l.tenant_id=d.tenant_id and l.document_id=d.document_id and not(l.warehouse_id=any($4::uuid[])))")
            .bind(tenant.0).bind(order).bind(status).bind(&warehouses).fetch_one(&mut *tx).await?;
        let data:Vec<MaterialDocumentSummary>=sqlx::query_as("select d.document_id,d.work_order_id,d.version_no,d.status,(select count(*) from mfg_material_document_lines l where l.tenant_id=d.tenant_id and l.document_id=d.document_id) as line_count,d.source_snapshot is distinct from jsonb_build_object('work_order_id',w.work_order_id,'work_order_no',w.work_order_no,'product_code',w.product_code,'bom_id',w.bom_id,'order_qty_micros',w.order_qty_micros::text) as source_changed,d.created_at,d.updated_at from mfg_material_documents d join mfg_work_orders w using(tenant_id,work_order_id) where d.tenant_id=$1 and d.work_order_id=$2 and ($3::text is null or d.status=$3) and exists(select 1 from mfg_material_document_lines l where l.tenant_id=d.tenant_id and l.document_id=d.document_id) and not exists(select 1 from mfg_material_document_lines l where l.tenant_id=d.tenant_id and l.document_id=d.document_id and not(l.warehouse_id=any($4::uuid[]))) order by d.document_id offset $5 limit $6")
            .bind(tenant.0).bind(order).bind(status).bind(&warehouses).bind(offset).bind(limit).fetch_all(&mut *tx).await?;
        permit.recheck(&mut tx).await?;
        tx.rollback().await?;
        Ok((total, data))
    }
}

pub(super) fn review_token(
    tenant: TenantId,
    user: UserId,
    document: Uuid,
    order: Uuid,
    version: i32,
    prior: &Value,
    current: &Value,
) -> anyhow::Result<String> {
    let bytes = serde_json::to_vec(&(
        "MFG_MATERIAL_SOURCE_REVIEW_V1",
        tenant,
        user,
        document,
        order,
        version,
        prior,
        current,
    ))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
