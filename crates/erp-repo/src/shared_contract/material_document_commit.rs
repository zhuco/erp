//! Confirms stored material lines once, in the same stock/value/receipt transaction.
use super::{
    MaterialDocumentError, MaterialDocumentLine, MaterialDocumentWrite, MaterialMoveInput,
    ReceiptValueActor, SharedContractRepo, SharedWrite, atomic_effects, insert_receipt, item_cost,
    load_receipt, material_documents, material_moves,
};
use erp_domain::{
    manufacturing::{MaterialMove, MaterialMoveKind},
    quantity::ScaledQty,
    receipt::{CommandStatus, ReceiptError, admit_retry, succeeded},
};
use erp_types::TenantId;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use std::collections::BTreeSet;
use uuid::Uuid;

pub struct MaterialDocumentConfirm {
    pub document_id: Uuid,
    pub work_order_id: Uuid,
    pub expected_version: i32,
}

#[derive(sqlx::FromRow)]
struct Header {
    work_order_id: Uuid,
    version_no: i32,
    status: String,
    source_snapshot: Value,
    posted_request_id: Option<String>,
    posted_snapshot: Option<Value>,
}

impl SharedContractRepo<'_> {
    pub async fn confirm_material_document(
        &self,
        tenant: TenantId,
        request_id: &str,
        input: MaterialDocumentConfirm,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Result<MaterialDocumentWrite, ReceiptError>> {
        let request_id = request_id.trim();
        if request_id.is_empty()
            || request_id.len() > 128
            || request_id.chars().any(char::is_control)
        {
            return Ok(Err(ReceiptError::EmptyRequest));
        }
        if input.expected_version < 1 || input.document_id.is_nil() || input.work_order_id.is_nil()
        {
            anyhow::bail!("invalid material confirmation identity or version");
        }
        let payload = serde_json::to_string(&(
            "MFG_MATERIAL_CONFIRM_V1",
            tenant,
            actor.user_id,
            input.document_id,
            input.work_order_id,
            input.expected_version,
        ))?;
        let mut tx = self.pool.begin().await?;
        let identity = super::receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.write", "MFG.PRODUCTION.WRITE"],
        )
        .await?;
        material_documents::lock_document(&mut tx, tenant, input.document_id).await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(tenant.0.to_string())
            .bind(format!("material-command:{request_id}"))
            .execute(&mut *tx)
            .await?;
        let header:Option<Header>=sqlx::query_as("select work_order_id,version_no,status,source_snapshot,posted_request_id,posted_snapshot from mfg_material_documents where tenant_id=$1 and document_id=$2 for update")
            .bind(tenant.0).bind(input.document_id).fetch_optional(&mut *tx).await?;
        let header = header.ok_or(MaterialDocumentError::NotFound)?;
        if header.work_order_id != input.work_order_id {
            return Err(MaterialDocumentError::NotFound.into());
        }
        let draft:Value=sqlx::query_scalar("select snapshot from mfg_material_document_revisions where tenant_id=$1 and document_id=$2 and version_no=$3")
            .bind(tenant.0).bind(input.document_id).bind(header.version_no).fetch_one(&mut *tx).await?;
        let lines: Vec<MaterialDocumentLine> = serde_json::from_value(draft["lines"].clone())
            .map_err(|_| MaterialDocumentError::ReconciliationRequired)?;
        let lines = material_documents::normalized_lines(&lines)?;
        let permits = material_documents::authorize(&mut tx, tenant, actor, &lines, false).await?;
        verify_current_lines(&mut tx, tenant, input.document_id, &lines).await?;
        let retry = match admit_retry(
            load_receipt(&mut tx, tenant, request_id).await?.as_ref(),
            request_id,
            &payload,
        ) {
            Ok(retry) => retry,
            Err(error) => return Ok(Err(error)),
        };
        if let Some(receipt) = retry {
            if receipt.status != CommandStatus::Succeeded
                || receipt.effect_id != input.document_id.to_string()
                || header.status != "POSTED"
                || header.posted_request_id.as_deref() != Some(request_id)
            {
                return Err(MaterialDocumentError::ReconciliationRequired.into());
            }
            let snapshot = header
                .posted_snapshot
                .ok_or(MaterialDocumentError::ReconciliationRequired)?;
            verify_committed(&mut tx, tenant, input.document_id, &snapshot).await?;
            for permit in &permits {
                permit.recheck(&mut tx).await?;
            }
            identity.recheck(&mut tx).await?;
            tx.rollback().await?;
            return Ok(Ok(MaterialDocumentWrite {
                effect: SharedWrite::Replay(receipt),
                snapshot,
            }));
        }
        if header.status != "DRAFT" {
            return Ok(Err(ReceiptError::PayloadConflict));
        }
        let new_command:anyhow::Result<_>=async {
        if header.version_no != input.expected_version {return Err(MaterialDocumentError::VersionConflict.into());}
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(tenant.0.to_string())
            .bind(format!("material-order:{}", input.work_order_id))
            .execute(&mut *tx)
            .await?;
        let inputs: Vec<_> = lines
            .iter()
            .map(|line| movement(input.work_order_id, line))
            .collect::<anyhow::Result<_>>()?;
        super::rework_material::lock_many_owners(&mut tx, tenant, &inputs).await?;
        let current:Option<(String,Option<Uuid>,i64,String,Value)>=sqlx::query_as("select status,bom_id,order_qty_micros,product_code,jsonb_build_object('work_order_id',work_order_id,'work_order_no',work_order_no,'product_code',product_code,'bom_id',bom_id,'order_qty_micros',order_qty_micros::text) from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
            .bind(tenant.0).bind(input.work_order_id).fetch_optional(&mut *tx).await?;
        let (status, bom, planned, product, source) =
            current.ok_or(MaterialDocumentError::NotFound)?;
        if source != header.source_snapshot || draft["source"] != source {
            return Err(MaterialDocumentError::SourceChanged.into());
        }
        material_documents::validate_sources(&mut tx, tenant, input.work_order_id, &lines).await?;
        let unknown:bool=sqlx::query_scalar("select exists(select 1 from mfg_material_moves where tenant_id=$1 and work_order_id=$2 and (warehouse_id is null or sku_id is null))")
            .bind(tenant.0).bind(input.work_order_id).fetch_one(&mut *tx).await?;
        if unknown {
            return Err(MaterialDocumentError::ReconciliationRequired.into());
        }
        let originals: BTreeSet<_> = lines.iter().filter_map(|l| l.original_issue_id).collect();
        for issue in originals {
            sqlx::query("select move_id from mfg_material_moves where tenant_id=$1 and move_id=$2 for update")
            .bind(tenant.0).bind(issue).fetch_one(&mut *tx).await?;
        }
        // Existing single-line order is original source -> cost -> reservations -> stock.
        let mut cost_codes = BTreeSet::new();
        for line in &lines {
            if line.kind != "CONSUME" {
                let code: String = sqlx::query_scalar(
                    "select sku_code from skus where tenant_id=$1 and sku_id=$2 for share",
                )
                .bind(tenant.0)
                .bind(line.sku_id)
                .fetch_one(&mut *tx)
                .await?;
                cost_codes.insert(code);
                if line.kind == "RETURN" {
                    let code:Option<String>=sqlx::query_scalar("select snapshot->'price'->>'sku_code' from mfg_material_issue_values where tenant_id=$1 and issue_id=$2")
                        .bind(tenant.0).bind(line.original_issue_id).fetch_optional(&mut *tx).await?.flatten();
                    if let Some(code) = code {
                        cost_codes.insert(code);
                    }
                }
            }
        }
        for code in cost_codes {
            item_cost::material_return_lock(&mut tx, tenant, &json!({"sku_code":code})).await?;
        }
        let physical: BTreeSet<_> = lines
            .iter()
            .filter(|l| l.kind != "CONSUME")
            .map(|l| (l.warehouse_id, l.sku_id))
            .collect();
        for (warehouse, sku) in &physical {
            let reservations:Vec<Uuid>=sqlx::query_scalar("select reservation_id from inventory_reservations where tenant_id=$1 and source_type='MFG_WORK_ORDER' and source_id=$2 and warehouse_id=$3 and sku_id=$4 and status='RESERVED' order by reservation_id limit 10001 for update")
                .bind(tenant.0).bind(input.work_order_id.to_string()).bind(warehouse).bind(sku).fetch_all(&mut *tx).await?;
            if reservations.len() > 10000 {
                anyhow::bail!("material reservation budget exceeded");
            }
        }
        for (warehouse, sku) in &physical {
            sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
                .bind(tenant.0.to_string())
                .bind(format!("inventory:{warehouse}:{sku}"))
                .execute(&mut *tx)
                .await?;
        }
        let mut effects = Vec::with_capacity(lines.len());
        for (line, movement) in lines.iter().zip(inputs) {
            let proof = if matches!(line.kind.as_str(), "ISSUE" | "SUPPLEMENT") {
                let code: String = sqlx::query_scalar(
                    "select sku_code from skus where tenant_id=$1 and sku_id=$2",
                )
                .bind(tenant.0)
                .bind(line.sku_id)
                .fetch_one(&mut *tx)
                .await?;
                let (required, proof) =
                    atomic_effects::material_source(&mut tx, tenant, bom, planned, &product, &code)
                        .await?;
                if proof["unit_code"] != line.unit_code {
                    return Err(MaterialDocumentError::ReconciliationRequired.into());
                }
                let net:i64=sqlx::query_scalar("select coalesce(sum(case when kind in('ISSUE','SUPPLEMENT') then qty_micros when kind='RETURN' then -qty_micros else 0 end),0)::bigint from mfg_material_moves where tenant_id=$1 and work_order_id=$2 and sku_id=$3")
                    .bind(tenant.0).bind(input.work_order_id).bind(line.sku_id).fetch_one(&mut *tx).await?;
                if net < 0
                    || i128::from(net) + i128::from(movement.movement.qty.as_micros())
                        > i128::from(required)
                {
                    anyhow::bail!("material document exceeds frozen requirement");
                }
                proof
            } else {
                let issue = line
                    .original_issue_id
                    .ok_or(MaterialDocumentError::ReconciliationRequired)?;
                let unit:Option<String>=sqlx::query_scalar("select snapshot->>'unit_code' from mfg_material_document_effects where tenant_id=$1 and move_id=$2")
                    .bind(tenant.0).bind(issue).fetch_optional(&mut *tx).await?.flatten();
                let unit = if unit.is_some() {
                    unit
                } else {
                    sqlx::query_scalar("select snapshot->'source'->>'unit_code' from atomic_source_documents where tenant_id=$1 and move_id=$2")
                    .bind(tenant.0).bind(issue).fetch_optional(&mut *tx).await?.flatten()
                };
                if unit.as_deref() != Some(line.unit_code.as_str()) {
                    return Err(MaterialDocumentError::ReconciliationRequired.into());
                }
                json!({"original_issue_id":issue,"unit_code":unit})
            };
            let (id, balance) = material_moves::append_material_move_as_actor_with_tx(
                &mut tx,
                self,
                tenant,
                movement,
                &status,
                ReceiptValueActor {
                    warehouse_id: line.warehouse_id,
                    ..actor
                },
            )
            .await?;
            let value: Value = if matches!(line.kind.as_str(), "ISSUE" | "SUPPLEMENT") {
                sqlx::query_scalar("select jsonb_build_object('state',state,'amount_cent',total_cent::text,'original',snapshot) from mfg_material_issue_values where tenant_id=$1 and issue_id=$2")
                    .bind(tenant.0).bind(id).fetch_one(&mut *tx).await?
            } else {
                sqlx::query_scalar("select jsonb_build_object('state',case when cost_cent is null then 'UNVALUED' else 'VALUED' end,'amount_cent',cost_cent::text,'original',snapshot) from mfg_material_value_allocations where tenant_id=$1 and allocation_id=$2")
                    .bind(tenant.0).bind(id).fetch_optional(&mut *tx).await?.ok_or(MaterialDocumentError::ReconciliationRequired)?
            };
            if value["state"] != "VALUED" || value["amount_cent"].as_str().is_none() {
                return Err(MaterialDocumentError::ReconciliationRequired.into());
            }
            let stock:Option<(i64,i64,i64,i64)>=sqlx::query_as("select on_hand_micros,reserved_micros,frozen_micros,in_transit_micros from inventory_balances where tenant_id=$1 and warehouse_id=$2 and sku_id=$3")
                .bind(tenant.0).bind(line.warehouse_id).bind(line.sku_id).fetch_optional(&mut *tx).await?;
            let stock = stock.ok_or(MaterialDocumentError::ReconciliationRequired)?;
            let snapshot = json!({"line_id":line.line_id,"move_id":id,"kind":line.kind,"warehouse_id":line.warehouse_id,"sku_id":line.sku_id,
                "batch_id":line.batch_id,"original_issue_id":line.original_issue_id,"unit_code":line.unit_code,"qty_micros":line.qty_micros,
                "source":proof,"value":value,"material":{"issued_micros":balance.issued.as_micros().to_string(),"returned_micros":balance.returned.as_micros().to_string(),
                    "consumed_micros":balance.consumed.as_micros().to_string(),"shop_micros":balance.shop_qty.as_micros().to_string()},
                "stock":{"on_hand_micros":stock.0.to_string(),"reserved_micros":stock.1.to_string(),"frozen_micros":stock.2.to_string(),"in_transit_micros":stock.3.to_string()}});
            sqlx::query("insert into mfg_material_document_effects(tenant_id,document_id,line_id,move_id,snapshot) values($1,$2,$3,$4,$5)")
                .bind(tenant.0).bind(input.document_id).bind(line.line_id).bind(id).bind(&snapshot).execute(&mut *tx).await?;
            effects.push(snapshot);
        }
        let posted_at: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("select clock_timestamp()")
                .fetch_one(&mut *tx)
                .await?;
        let snapshot = json!({"schema":2,"document_id":input.document_id,"work_order_id":input.work_order_id,"version_no":header.version_no,"request_id":request_id,
            "status":"POSTED","posted":true,"lines":lines,"source":source,"effects":effects,"actor_user_id":actor.user_id,
            "authority":permits.iter().map(|p|p.snapshot()).collect::<Vec<_>>(),"posted_at":posted_at.to_rfc3339()});
        let operation = crate::operation_logs::OperationLogsRepo::insert_with_tx(
            &mut *tx,
            crate::operation_logs::OperationLogCreate {
                tenant_id: tenant,
                actor_type: "USER".into(),
                actor_user_id: Some(actor.user_id),
                action: "MFG_MATERIAL_DOCUMENT_CONFIRMED".into(),
                ref_type: Some("mfg_material_document".into()),
                ref_id: Some(input.document_id.to_string()),
                metadata: Some(snapshot.clone()),
            },
        )
        .await?;
        let audit = crate::audit_logs::AuditLogsRepo::insert_with_tx(
            &mut *tx,
            crate::audit_logs::AuditLogCreate {
                tenant_id: tenant,
                user_id: Some(actor.user_id),
                action: "manufacturing.material.document.confirm".into(),
                entity_type: "mfg_material_document".into(),
                entity_id: Some(input.document_id.to_string()),
                before_json: Some(draft),
                after_json: Some(snapshot.clone()),
                ip: None,
                ua: None,
            },
        )
        .await?;
        let event = crate::outbox::OutboxRepo::enqueue_with_tx(
            &mut *tx,
            tenant,
            "manufacturing.material.document.confirmed",
            &snapshot,
            chrono::Utc::now(),
        )
        .await?;
        let mut snapshot = snapshot;
        snapshot["operation_log_id"] = json!(operation.log_id);
        snapshot["audit_id"] = json!(audit.audit_id);
        snapshot["event_id"] = json!(event);
        let receipt = succeeded(request_id, &payload, &input.document_id.to_string());
        insert_receipt(&mut tx, tenant, &receipt).await?;
        sqlx::query("update mfg_material_documents set status='POSTED',posted_request_id=$3,posted_snapshot=$4,updated_at=now() where tenant_id=$1 and document_id=$2")
            .bind(tenant.0).bind(input.document_id).bind(request_id).bind(&snapshot).execute(&mut *tx).await?;
        for permit in &permits {
            permit.recheck(&mut tx).await?;
        }
        identity.recheck(&mut tx).await?;
        Ok((receipt,snapshot))
        }.await;
        let (receipt, snapshot) = new_command.map_err(super::material_rejections::before_commit)?;
        tx.commit().await?;
        Ok(Ok(MaterialDocumentWrite {
            effect: SharedWrite::Stored(receipt),
            snapshot,
        }))
    }
}

fn movement(order: Uuid, line: &MaterialDocumentLine) -> anyhow::Result<MaterialMoveInput<'_>> {
    let kind = match line.kind.as_str() {
        "ISSUE" => MaterialMoveKind::Issue,
        "SUPPLEMENT" => MaterialMoveKind::Supplement,
        "RETURN" => MaterialMoveKind::Return,
        "CONSUME" => MaterialMoveKind::Consume,
        _ => return Err(MaterialDocumentError::ReconciliationRequired.into()),
    };
    Ok(MaterialMoveInput {
        work_order_id: order,
        warehouse_id: line.warehouse_id,
        sku_id: line.sku_id,
        batch_id: line.batch_id,
        original_issue_id: line.original_issue_id,
        movement: MaterialMove {
            kind,
            qty: ScaledQty::from_micros(line.qty_micros.parse()?),
            supplement_reason_required: line.reason.is_some(),
        },
        reason: line.reason.as_deref(),
    })
}

pub(super) async fn verify_current_lines(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    document: Uuid,
    lines: &[MaterialDocumentLine],
) -> anyhow::Result<()> {
    let current:Value=sqlx::query_scalar("select coalesce(jsonb_agg(jsonb_build_object('line_id',line_id,'kind',kind,'warehouse_id',warehouse_id,'sku_id',sku_id,'batch_id',batch_id,'original_issue_id',original_issue_id,'unit_code',unit_code,'qty_micros',qty_micros::text,'reason',reason) order by ordinal),'[]'::jsonb) from mfg_material_document_lines where tenant_id=$1 and document_id=$2")
        .bind(tenant.0).bind(document).fetch_one(&mut **tx).await?;
    if current != serde_json::to_value(lines)? {
        return Err(MaterialDocumentError::ReconciliationRequired.into());
    }
    Ok(())
}

pub(super) async fn verify_committed(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    document: Uuid,
    snapshot: &Value,
) -> anyhow::Result<()> {
    let effects = snapshot["effects"]
        .as_array()
        .filter(|e| !e.is_empty() && e.len() <= 100)
        .ok_or(MaterialDocumentError::ReconciliationRequired)?;
    if snapshot["document_id"] != json!(document)
        || snapshot["status"] != "POSTED"
        || snapshot["posted"] != true
    {
        return Err(MaterialDocumentError::ReconciliationRequired.into());
    }
    let count: i64 = sqlx::query_scalar(
        "select count(*) from mfg_material_document_effects where tenant_id=$1 and document_id=$2",
    )
    .bind(tenant.0)
    .bind(document)
    .fetch_one(&mut **tx)
    .await?;
    if usize::try_from(count)? != effects.len() {
        return Err(MaterialDocumentError::ReconciliationRequired.into());
    }
    for effect in effects {
        let id = Uuid::parse_str(
            effect["move_id"]
                .as_str()
                .ok_or(MaterialDocumentError::ReconciliationRequired)?,
        )?;
        let stored:Option<Value>=sqlx::query_scalar("select e.snapshot from mfg_material_document_effects e join mfg_material_moves m using(tenant_id,move_id) where e.tenant_id=$1 and e.document_id=$2 and e.move_id=$3 and m.kind=e.snapshot->>'kind' and m.qty_micros::text=e.snapshot->>'qty_micros' and m.warehouse_id::text=e.snapshot->>'warehouse_id' and m.sku_id::text=e.snapshot->>'sku_id' and m.work_order_id::text=$4 and m.batch_id is not distinct from (e.snapshot->>'batch_id')::uuid and m.original_issue_id is not distinct from (e.snapshot->>'original_issue_id')::uuid")
            .bind(tenant.0).bind(document).bind(id).bind(snapshot["work_order_id"].as_str()).fetch_optional(&mut **tx).await?;
        if stored.as_ref() != Some(effect) {
            return Err(MaterialDocumentError::ReconciliationRequired.into());
        }
        let value:bool=sqlx::query_scalar("select exists(select 1 from mfg_material_issue_values where tenant_id=$1 and issue_id=$2 and state='VALUED' and total_cent::text=$3) or exists(select 1 from mfg_material_value_allocations where tenant_id=$1 and allocation_id=$2 and cost_cent::text=$3)")
            .bind(tenant.0).bind(id).bind(effect["value"]["amount_cent"].as_str()).fetch_one(&mut **tx).await?;
        if !value {
            return Err(MaterialDocumentError::ReconciliationRequired.into());
        }
        if effect["kind"] != "CONSUME" {
            let qty = effect["qty_micros"]
                .as_str()
                .ok_or(MaterialDocumentError::ReconciliationRequired)?
                .parse::<i64>()?;
            let delta = if effect["kind"] == "RETURN" {
                qty
            } else {
                qty.checked_neg()
                    .ok_or(MaterialDocumentError::ReconciliationRequired)?
            };
            let ledger:bool=sqlx::query_scalar("select exists(select 1 from inventory_ledgers where tenant_id=$1 and ref_type='mfg_material_move' and ref_id=$2 and delta_on_hand_micros=$3)")
                .bind(tenant.0).bind(id.to_string()).bind(delta).fetch_one(&mut **tx).await?;
            if !ledger {
                return Err(MaterialDocumentError::ReconciliationRequired.into());
            }
        }
    }
    let operation = Uuid::parse_str(
        snapshot["operation_log_id"]
            .as_str()
            .ok_or(MaterialDocumentError::ReconciliationRequired)?,
    )?;
    let audit = Uuid::parse_str(
        snapshot["audit_id"]
            .as_str()
            .ok_or(MaterialDocumentError::ReconciliationRequired)?,
    )?;
    let event = Uuid::parse_str(
        snapshot["event_id"]
            .as_str()
            .ok_or(MaterialDocumentError::ReconciliationRequired)?,
    )?;
    let mut body = snapshot.clone();
    let fields = body
        .as_object_mut()
        .ok_or(MaterialDocumentError::ReconciliationRequired)?;
    for name in ["operation_log_id", "audit_id", "event_id"] {
        fields.remove(name);
    }
    let proved:bool=sqlx::query_scalar("select exists(select 1 from operation_logs where tenant_id=$1 and log_id=$2 and action='MFG_MATERIAL_DOCUMENT_CONFIRMED' and ref_id=$5 and metadata=$6) and exists(select 1 from audit_logs where tenant_id=$1 and audit_id=$3 and action='manufacturing.material.document.confirm' and entity_id=$5 and after_json=$6) and exists(select 1 from outbox_events where tenant_id=$1 and outbox_id=$4 and event_type='manufacturing.material.document.confirmed' and payload=$6)")
        .bind(tenant.0).bind(operation).bind(audit).bind(event).bind(document.to_string()).bind(body).fetch_one(&mut **tx).await?;
    if !proved {
        return Err(MaterialDocumentError::ReconciliationRequired.into());
    }
    Ok(())
}
