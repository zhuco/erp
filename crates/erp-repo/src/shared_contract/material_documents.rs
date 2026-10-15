//! Real material drafts, optimistic versions and immutable original save results.
use super::{
    ReceiptValueActor, SharedContractRepo, SharedWrite, insert_receipt, load_receipt,
    receipt_authority,
};
use erp_domain::receipt::{ReceiptError, admit_retry, succeeded};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDocumentLine {
    pub line_id: Uuid,
    pub kind: String,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub original_issue_id: Option<Uuid>,
    pub unit_code: String,
    pub qty_micros: String,
    pub reason: Option<String>,
}

pub struct MaterialDocumentSave<'a> {
    pub document_id: Uuid,
    pub work_order_id: Uuid,
    pub expected_version: i32,
    pub lines: &'a [MaterialDocumentLine],
}

pub struct MaterialDocumentWrite {
    pub effect: SharedWrite,
    pub snapshot: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum MaterialDocumentError {
    #[error("material document or source not found")]
    NotFound,
    #[error("material document version conflict")]
    VersionConflict,
    #[error("material document source changed; explicit source review required")]
    SourceChanged,
    #[error("material document original result requires reconciliation")]
    ReconciliationRequired,
}

impl SharedContractRepo<'_> {
    pub async fn save_material_document(
        &self,
        tenant: TenantId,
        request_id: &str,
        input: MaterialDocumentSave<'_>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Result<MaterialDocumentWrite, ReceiptError>> {
        self.save_material_document_inner(tenant, request_id, input, actor, None)
            .await
    }

    pub async fn adopt_material_document_source(
        &self,
        tenant: TenantId,
        request_id: &str,
        input: super::MaterialDocumentSourceAdoption<'_>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Result<MaterialDocumentWrite, ReceiptError>> {
        self.save_material_document_inner(
            tenant,
            request_id,
            MaterialDocumentSave {
                document_id: input.document_id,
                work_order_id: input.work_order_id,
                expected_version: input.expected_version,
                lines: &[],
            },
            actor,
            Some(input.review_token),
        )
        .await
    }

    async fn save_material_document_inner(
        &self,
        tenant: TenantId,
        request_id: &str,
        input: MaterialDocumentSave<'_>,
        actor: ReceiptValueActor,
        review: Option<&str>,
    ) -> anyhow::Result<Result<MaterialDocumentWrite, ReceiptError>> {
        let request_id = request_id.trim();
        if request_id.trim().is_empty()
            || request_id.len() > 128
            || request_id.chars().any(char::is_control)
        {
            return Ok(Err(ReceiptError::EmptyRequest));
        }
        if input.document_id.is_nil() || input.work_order_id.is_nil() || input.expected_version < 0
        {
            anyhow::bail!("invalid material document identity or version");
        }
        if review
            .is_some_and(|token| token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            anyhow::bail!("invalid material source review token");
        }
        let mut tx = self.pool.begin().await?;
        let identity = if review.is_some() {
            Some(
                receipt_authority::lock_identity(
                    &mut tx,
                    tenant,
                    actor,
                    &["manufacturing.write", "MFG.PRODUCTION.WRITE"],
                )
                .await?,
            )
        } else {
            None
        };
        let lines = if review.is_some() {
            lock_document(&mut tx, tenant, input.document_id).await?;
            let saved:Option<Value>=sqlx::query_scalar("select coalesce(original.snapshot,current.snapshot) from mfg_material_documents d join mfg_material_document_revisions current using(tenant_id,document_id,version_no) left join mfg_material_document_revisions original on original.tenant_id=d.tenant_id and original.document_id=d.document_id and original.request_id=$4 where d.tenant_id=$1 and d.document_id=$2 and d.work_order_id=$3")
                .bind(tenant.0).bind(input.document_id).bind(input.work_order_id).bind(request_id).fetch_optional(&mut *tx).await?;
            let saved = saved.ok_or(MaterialDocumentError::NotFound)?;
            let stored: Vec<MaterialDocumentLine> = serde_json::from_value(saved["lines"].clone())
                .map_err(|_| MaterialDocumentError::ReconciliationRequired)?;
            normalized_lines(&stored)?
        } else {
            normalized_lines(input.lines)?
        };
        let payload = if let Some(token) = review {
            serde_json::to_string(&(
                "MFG_MATERIAL_SOURCE_ADOPT_V1",
                tenant,
                actor.user_id,
                input.document_id,
                input.work_order_id,
                input.expected_version,
                &lines,
                token,
            ))?
        } else {
            serde_json::to_string(&(
                1,
                tenant,
                actor.user_id,
                input.document_id,
                input.work_order_id,
                input.expected_version,
                &lines,
            ))?
        };
        let permits = authorize(&mut tx, tenant, actor, &lines, false).await?;
        lock_document(&mut tx, tenant, input.document_id).await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(tenant.0.to_string())
            .bind(format!("material-command:{request_id}"))
            .execute(&mut *tx)
            .await?;
        let retry = match admit_retry(
            load_receipt(&mut tx, tenant, request_id).await?.as_ref(),
            request_id,
            &payload,
        ) {
            Ok(retry) => retry,
            Err(error) => return Ok(Err(error)),
        };
        if let Some(receipt) = retry {
            if receipt.status != erp_domain::receipt::CommandStatus::Succeeded {
                return Err(MaterialDocumentError::ReconciliationRequired.into());
            }
            let id = Uuid::parse_str(&receipt.effect_id)
                .map_err(|_| MaterialDocumentError::ReconciliationRequired)?;
            let original:Option<(Uuid,Uuid,Value)>=sqlx::query_as("select document_id,actor_user_id,snapshot from mfg_material_document_revisions where tenant_id=$1 and revision_id=$2 and request_id=$3")
                .bind(tenant.0).bind(id).bind(request_id).fetch_optional(&mut *tx).await?;
            let (document, user, snapshot) =
                original.ok_or(MaterialDocumentError::ReconciliationRequired)?;
            if document != input.document_id || user != actor.user_id.0 {
                return Ok(Err(ReceiptError::PayloadConflict));
            }
            for permit in &permits {
                permit.recheck(&mut tx).await?;
            }
            if let Some(identity) = &identity {
                identity.recheck(&mut tx).await?;
            }
            tx.rollback().await?;
            return Ok(Ok(MaterialDocumentWrite {
                effect: SharedWrite::Replay(receipt),
                snapshot,
            }));
        }
        let new_command:anyhow::Result<_>=async {
        let source:Option<Value>=sqlx::query_scalar("select jsonb_build_object('work_order_id',work_order_id,'work_order_no',work_order_no,'product_code',product_code,'bom_id',bom_id,'order_qty_micros',order_qty_micros::text) from mfg_work_orders where tenant_id=$1 and work_order_id=$2 and status in('PLANNED','RELEASED','PAUSED') and order_qty_micros>0 for share")
            .bind(tenant.0).bind(input.work_order_id).fetch_optional(&mut *tx).await?;
        let source = source.ok_or(MaterialDocumentError::NotFound)?;
        let current:Option<(Uuid,i32,String,Value)>=sqlx::query_as("select work_order_id,version_no,status,source_snapshot from mfg_material_documents where tenant_id=$1 and document_id=$2 for update")
            .bind(tenant.0).bind(input.document_id).fetch_optional(&mut *tx).await?;
        let previous_source = current.as_ref().map(|row| row.3.clone());
        let version = match current {
            Some((order, version, status, prior)) => {
                if order != input.work_order_id {
                    return Err(MaterialDocumentError::NotFound.into());
                }
                if version != input.expected_version || status != "DRAFT" {
                    return Err(MaterialDocumentError::VersionConflict.into());
                }
                if let Some(token) = review {
                    let expected = super::material_document_sources::review_token(
                        tenant,
                        actor.user_id,
                        input.document_id,
                        input.work_order_id,
                        version,
                        &prior,
                        &source,
                    )?;
                    if token != expected {
                        return Err(MaterialDocumentError::SourceChanged.into());
                    }
                    super::material_document_commit::verify_current_lines(
                        &mut tx,
                        tenant,
                        input.document_id,
                        &lines,
                    )
                    .await?;
                } else if prior != source {
                    return Err(MaterialDocumentError::SourceChanged.into());
                }
                version
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("material version exhausted"))?
            }
            None => {
                if review.is_some() || input.expected_version != 0 {
                    return Err(MaterialDocumentError::VersionConflict.into());
                }
                1
            }
        };
        validate_sources(&mut tx, tenant, input.work_order_id, &lines).await?;
        sqlx::query("insert into mfg_material_documents(tenant_id,document_id,work_order_id,version_no,source_snapshot) values($1,$2,$3,$4,$5) on conflict(tenant_id,document_id) do update set version_no=excluded.version_no,source_snapshot=excluded.source_snapshot,updated_at=now()")
            .bind(tenant.0).bind(input.document_id).bind(input.work_order_id).bind(version).bind(&source).execute(&mut *tx).await?;
        sqlx::query(
            "delete from mfg_material_document_lines where tenant_id=$1 and document_id=$2",
        )
        .bind(tenant.0)
        .bind(input.document_id)
        .execute(&mut *tx)
        .await?;
        for (index, line) in lines.iter().enumerate() {
            sqlx::query("insert into mfg_material_document_lines(tenant_id,document_id,line_id,ordinal,kind,warehouse_id,sku_id,batch_id,original_issue_id,unit_code,qty_micros,reason) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
                .bind(tenant.0).bind(input.document_id).bind(line.line_id).bind(i32::try_from(index+1)?).bind(&line.kind)
                .bind(line.warehouse_id).bind(line.sku_id).bind(line.batch_id).bind(line.original_issue_id).bind(&line.unit_code)
                .bind(line.qty_micros.parse::<i64>()?).bind(&line.reason).execute(&mut *tx).await?;
        }
        let revision = Uuid::new_v4();
        let saved_at: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("select clock_timestamp()")
                .fetch_one(&mut *tx)
                .await?;
        let mut snapshot = json!({"schema":1,"document_id":input.document_id,"work_order_id":input.work_order_id,"version_no":version,
            "status":"DRAFT","posted":false,"revision_id":revision,"request_id":request_id,"lines":lines,"source":source,
            "actor_user_id":actor.user_id,"authority":permits.iter().map(|p|p.snapshot()).collect::<Vec<_>>(),"saved_at":saved_at.to_rfc3339()});
        if let Some(token) = review {
            snapshot["source_adoption"] =
                json!({"review_token":token,"previous_source":previous_source});
        }
        sqlx::query("insert into mfg_material_document_revisions(tenant_id,document_id,version_no,revision_id,request_id,actor_user_id,snapshot) values($1,$2,$3,$4,$5,$6,$7)")
            .bind(tenant.0).bind(input.document_id).bind(version).bind(revision).bind(request_id).bind(actor.user_id.0).bind(&snapshot).execute(&mut *tx).await?;
        let receipt = succeeded(request_id, &payload, &revision.to_string());
        insert_receipt(&mut tx, tenant, &receipt).await?;
        draft_logs(&mut tx, tenant, actor, input.document_id, &snapshot).await?;
        for permit in &permits {
            permit.recheck(&mut tx).await?;
        }
        if let Some(identity) = &identity {
            identity.recheck(&mut tx).await?;
        }
        Ok((receipt,snapshot))
        }.await;
        let (receipt, snapshot) = new_command.map_err(super::material_rejections::before_commit)?;
        tx.commit().await?;
        Ok(Ok(MaterialDocumentWrite {
            effect: SharedWrite::Stored(receipt),
            snapshot,
        }))
    }

    pub async fn read_material_document(
        &self,
        tenant: TenantId,
        work_order: Uuid,
        document: Uuid,
        request_id: Option<&str>,
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
        let snapshot: Option<Value> = if let Some(request) = request_id {
            sqlx::query_scalar("select case when d.posted_request_id=$4 then d.posted_snapshot else r.snapshot end from mfg_material_documents d left join mfg_material_document_revisions r on r.tenant_id=d.tenant_id and r.document_id=d.document_id and r.request_id=$4 where d.tenant_id=$1 and d.document_id=$2 and d.work_order_id=$3 and (d.posted_request_id=$4 or r.request_id is not null)")
                .bind(tenant.0).bind(document).bind(work_order).bind(request).fetch_optional(&mut *tx).await?
        } else {
            sqlx::query_scalar("select case when d.status='POSTED' then d.posted_snapshot else r.snapshot end from mfg_material_documents d join mfg_material_document_revisions r using(tenant_id,document_id,version_no) where d.tenant_id=$1 and d.document_id=$2 and d.work_order_id=$3")
                .bind(tenant.0).bind(document).bind(work_order).fetch_optional(&mut *tx).await?
        };
        let snapshot = snapshot.ok_or(MaterialDocumentError::NotFound)?;
        let lines: Vec<MaterialDocumentLine> = serde_json::from_value(snapshot["lines"].clone())
            .map_err(|_| MaterialDocumentError::ReconciliationRequired)?;
        if snapshot["schema"] == 2 {
            let permits = authorize(&mut tx, tenant, actor, &lines, true).await?;
            if snapshot["document_id"] != json!(document)
                || snapshot["work_order_id"] != json!(work_order)
            {
                return Err(MaterialDocumentError::ReconciliationRequired.into());
            }
            let original_user = erp_types::UserId(Uuid::parse_str(
                snapshot["actor_user_id"]
                    .as_str()
                    .ok_or(MaterialDocumentError::ReconciliationRequired)?,
            )?);
            let version = snapshot["version_no"]
                .as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .ok_or(MaterialDocumentError::ReconciliationRequired)?;
            let request = snapshot["request_id"]
                .as_str()
                .ok_or(MaterialDocumentError::ReconciliationRequired)?;
            let payload = serde_json::to_string(&(
                "MFG_MATERIAL_CONFIRM_V1",
                tenant,
                original_user,
                document,
                work_order,
                version,
            ))?;
            let receipt = load_receipt(&mut tx, tenant, request)
                .await?
                .ok_or(MaterialDocumentError::ReconciliationRequired)?;
            if receipt.status != erp_domain::receipt::CommandStatus::Succeeded
                || receipt.effect_id != document.to_string()
                || receipt.payload_hash != payload
            {
                return Err(MaterialDocumentError::ReconciliationRequired.into());
            }
            super::material_document_commit::verify_committed(&mut tx, tenant, document, &snapshot)
                .await?;
            identity.recheck(&mut tx).await?;
            for permit in &permits {
                permit.recheck(&mut tx).await?;
            }
            tx.rollback().await?;
            return Ok(snapshot);
        }
        let version = snapshot["version_no"]
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .filter(|v| *v > 0)
            .ok_or(MaterialDocumentError::ReconciliationRequired)?;
        let original_user = erp_types::UserId(Uuid::parse_str(
            snapshot["actor_user_id"]
                .as_str()
                .ok_or(MaterialDocumentError::ReconciliationRequired)?,
        )?);
        if snapshot["document_id"] != json!(document)
            || snapshot["work_order_id"] != json!(work_order)
            || snapshot["source"]["work_order_id"] != json!(work_order)
            || snapshot["schema"] != 1
            || snapshot["status"] != "DRAFT"
            || snapshot["posted"] != false
        {
            return Err(MaterialDocumentError::ReconciliationRequired.into());
        }
        let original_payload =
            if let Some(token) = snapshot["source_adoption"]["review_token"].as_str() {
                serde_json::to_string(&(
                    "MFG_MATERIAL_SOURCE_ADOPT_V1",
                    tenant,
                    original_user,
                    document,
                    work_order,
                    version - 1,
                    &lines,
                    token,
                ))?
            } else {
                serde_json::to_string(&(
                    1,
                    tenant,
                    original_user,
                    document,
                    work_order,
                    version - 1,
                    &lines,
                ))?
            };
        let permits = authorize(&mut tx, tenant, actor, &lines, true).await?;
        let id = snapshot["revision_id"]
            .as_str()
            .ok_or(MaterialDocumentError::ReconciliationRequired)?;
        let valid:bool=sqlx::query_scalar("select exists(select 1 from command_receipts c join mfg_material_document_revisions r on r.tenant_id=c.tenant_id and r.request_id=c.request_id where r.tenant_id=$1 and r.revision_id=$2 and c.effect_id=$2::text and c.status='SUCCEEDED' and c.payload_hash=$3)")
            .bind(tenant.0).bind(Uuid::parse_str(id)?).bind(original_payload).fetch_one(&mut *tx).await?;
        if !valid {
            return Err(MaterialDocumentError::ReconciliationRequired.into());
        }
        identity.recheck(&mut tx).await?;
        for permit in &permits {
            permit.recheck(&mut tx).await?;
        }
        tx.rollback().await?;
        Ok(snapshot)
    }
}

pub(super) fn normalized_lines(
    lines: &[MaterialDocumentLine],
) -> anyhow::Result<Vec<MaterialDocumentLine>> {
    if lines.is_empty() || lines.len() > 100 {
        anyhow::bail!("material document requires 1 to 100 actual lines");
    }
    let mut seen = BTreeSet::new();
    let mut result = lines.to_vec();
    for line in &mut result {
        line.kind = line.kind.trim().to_owned();
        line.unit_code = line.unit_code.trim().to_owned();
        line.reason = line
            .reason
            .as_ref()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        let qty = line.qty_micros.parse::<i64>()?;
        line.qty_micros = qty.to_string();
        if line.line_id.is_nil()
            || !seen.insert(line.line_id)
            || line.warehouse_id.is_nil()
            || line.sku_id.is_nil()
            || line.batch_id.is_some_and(|id| id.is_nil())
            || line.original_issue_id.is_some_and(|id| id.is_nil())
            || qty <= 0
            || line.unit_code.is_empty()
            || line.unit_code.len() > 32
            || line.unit_code.chars().any(char::is_control)
            || line.reason.as_ref().is_some_and(|s| s.len() > 4096)
        {
            anyhow::bail!("invalid material document line");
        }
        if !matches!(
            line.kind.as_str(),
            "ISSUE" | "SUPPLEMENT" | "RETURN" | "CONSUME"
        ) || matches!(line.kind.as_str(), "RETURN" | "CONSUME")
            != line.original_issue_id.is_some()
            || (line.kind == "SUPPLEMENT" && line.reason.is_none())
        {
            anyhow::bail!("material action source or reason is missing");
        }
    }
    if serde_json::to_vec(&result)?.len() > 8 * 1024 * 1024 {
        anyhow::bail!("material document byte budget exceeded");
    }
    Ok(result)
}

pub(super) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: ReceiptValueActor,
    lines: &[MaterialDocumentLine],
    read: bool,
) -> anyhow::Result<Vec<receipt_authority::Permit>> {
    if lines.is_empty() || lines.len() > 100 {
        return Err(MaterialDocumentError::ReconciliationRequired.into());
    }
    let warehouses: BTreeSet<_> = lines.iter().map(|l| l.warehouse_id).collect();
    let mut permits = Vec::new();
    for warehouse in warehouses {
        permits.push(
            receipt_authority::lock_permissions(
                tx,
                tenant,
                ReceiptValueActor {
                    warehouse_id: warehouse,
                    ..actor
                },
                if read {
                    &["manufacturing.read", "MFG.PRODUCTION.READ"]
                } else {
                    &["manufacturing.write", "MFG.PRODUCTION.WRITE"]
                },
            )
            .await?,
        );
    }
    Ok(permits)
}

pub(super) async fn lock_document(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    document: Uuid,
) -> anyhow::Result<()> {
    sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
        .bind(tenant.0.to_string())
        .bind(format!("material-document:{document}"))
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(super) async fn validate_sources(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    lines: &[MaterialDocumentLine],
) -> anyhow::Result<()> {
    for line in lines {
        let owned: bool = sqlx::query_scalar(
            "select exists(select 1 from skus where tenant_id=$1 and sku_id=$2)",
        )
        .bind(tenant.0)
        .bind(line.sku_id)
        .fetch_one(&mut **tx)
        .await?;
        if !owned {
            return Err(MaterialDocumentError::NotFound.into());
        }
        if let Some(batch) = line.batch_id {
            let owned:bool=sqlx::query_scalar("select exists(select 1 from sku_batches where tenant_id=$1 and sku_id=$2 and batch_id=$3)")
                .bind(tenant.0).bind(line.sku_id).bind(batch).fetch_one(&mut **tx).await?;
            if !owned {
                return Err(MaterialDocumentError::NotFound.into());
            }
        }
        if let Some(issue) = line.original_issue_id {
            let owned:bool=sqlx::query_scalar("select exists(select 1 from mfg_material_moves where tenant_id=$1 and move_id=$2 and work_order_id=$3 and warehouse_id=$4 and sku_id=$5 and batch_id is not distinct from $6 and kind in('ISSUE','SUPPLEMENT'))")
                .bind(tenant.0).bind(issue).bind(order).bind(line.warehouse_id).bind(line.sku_id).bind(line.batch_id).fetch_one(&mut **tx).await?;
            if !owned {
                return Err(MaterialDocumentError::NotFound.into());
            }
        }
    }
    Ok(())
}

async fn draft_logs(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    actor: ReceiptValueActor,
    document: Uuid,
    snapshot: &Value,
) -> anyhow::Result<()> {
    crate::operation_logs::OperationLogsRepo::insert_with_tx(
        &mut **tx,
        crate::operation_logs::OperationLogCreate {
            tenant_id: tenant,
            actor_type: "USER".into(),
            actor_user_id: Some(actor.user_id),
            action: if snapshot.get("source_adoption").is_some() {
                "MFG_MATERIAL_SOURCE_ADOPTED"
            } else {
                "MFG_MATERIAL_DRAFT_SAVED"
            }
            .into(),
            ref_type: Some("mfg_material_document".into()),
            ref_id: Some(document.to_string()),
            metadata: Some(snapshot.clone()),
        },
    )
    .await?;
    crate::audit_logs::AuditLogsRepo::insert_with_tx(
        &mut **tx,
        crate::audit_logs::AuditLogCreate {
            tenant_id: tenant,
            user_id: Some(actor.user_id),
            action: if snapshot.get("source_adoption").is_some() {
                "manufacturing.material.source.adopt"
            } else {
                "manufacturing.material.document.save"
            }
            .into(),
            entity_type: "mfg_material_document".into(),
            entity_id: Some(document.to_string()),
            before_json: None,
            after_json: Some(snapshot.clone()),
            ip: None,
            ua: None,
        },
    )
    .await?;
    Ok(())
}
