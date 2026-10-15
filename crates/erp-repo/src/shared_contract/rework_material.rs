//! Rework follows its original quality allocation, including unqualified returns.
use super::{DispositionError, MaterialMoveInput, ReceiptValueActor, receipt_authority};
use crate::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    operation_logs::{OperationLogCreate, OperationLogsRepo},
};
use erp_types::TenantId;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn source(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    issue: Option<Uuid>,
) -> anyhow::Result<Option<Value>> {
    if let Some(issue) = issue {
        Ok(sqlx::query_scalar("select to_jsonb(c) from mfg_receipt_disposition_commands c where tenant_id=$1 and material_move_id=$2 and kind='REWORK'")
   .bind(tenant.0).bind(issue).fetch_optional(&mut **tx).await?)
    } else {
        Ok(None)
    }
}

pub(super) async fn lock_owners(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    input: MaterialMoveInput<'_>,
) -> anyhow::Result<()> {
    lock_many_owners(tx, tenant, &[input]).await
}

pub(super) async fn lock_many_owners(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    inputs: &[MaterialMoveInput<'_>],
) -> anyhow::Result<()> {
    let mut owners = Vec::new();
    for input in inputs {
        owners.push(input.work_order_id);
        if let Some(source) = source(tx, tenant, input.original_issue_id).await? {
            let owner: Uuid = sqlx::query_scalar(
                "select work_order_id from mfg_receipts where tenant_id=$1 and receipt_id=$2",
            )
            .bind(tenant.0)
            .bind(Uuid::parse_str(
                source["receipt_id"]
                    .as_str()
                    .ok_or(DispositionError::Scope)?,
            )?)
            .fetch_one(&mut **tx)
            .await?;
            owners.push(owner);
        }
    }
    owners.sort();
    owners.dedup();
    for owner in owners {
        sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
   .bind(tenant.0).bind(owner).execute(&mut **tx).await?;
    }
    Ok(())
}

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    input: MaterialMoveInput<'_>,
    movement: Uuid,
    source: &Value,
    actor: ReceiptValueActor,
) -> anyhow::Result<()> {
    let original = Uuid::parse_str(
        source["material_move_id"]
            .as_str()
            .ok_or(DispositionError::Scope)?,
    )?;
    let disposition = Uuid::parse_str(
        source["disposition_id"]
            .as_str()
            .ok_or(DispositionError::Scope)?,
    )?;
    let receipt = Uuid::parse_str(
        source["receipt_id"]
            .as_str()
            .ok_or(DispositionError::Scope)?,
    )?;
    let original_qty = source["qty_micros"]
        .as_i64()
        .ok_or(DispositionError::Quantity)?;
    let original_value = source["cost_cent"].as_i64().ok_or(DispositionError::Cost)?;
    let prior:i64=sqlx::query_scalar("select coalesce(sum(qty_micros),0)::bigint from mfg_rework_material_values where tenant_id=$1 and original_issue_id=$2")
  .bind(tenant.0).bind(original).fetch_one(&mut **tx).await?;
    let next = prior
        .checked_add(input.movement.qty.as_micros())
        .ok_or(DispositionError::Quantity)?;
    if original_qty <= 0 || next > original_qty || original_value < 0 {
        return Err(DispositionError::Quantity.into());
    }
    let value = i64::try_from(
        i128::from(original_value) * i128::from(next) / i128::from(original_qty)
            - i128::from(original_value) * i128::from(prior) / i128::from(original_qty),
    )?;
    let kind = input.movement.kind.as_str();
    let permit = receipt_authority::lock_permissions(
        tx,
        tenant,
        actor,
        &["manufacturing.write", "MFG.PRODUCTION.WRITE"],
    )
    .await?;
    let snapshot = json!({"source":source,"move_id":movement,"original_issue_id":original,"kind":kind,
   "qty_micros":input.movement.qty.as_micros(),"cost_cent":value,"actor":permit.snapshot(),"quality_status":if kind=="RETURN"{"HOLD"}else{"WIP_CONSUMED"}});
    sqlx::query("insert into mfg_rework_material_values(tenant_id,move_id,disposition_id,original_issue_id,receipt_id,kind,qty_micros,cost_cent,snapshot) values($1,$2,$3,$4,$5,$6,$7,$8,$9)")
  .bind(tenant.0).bind(movement).bind(disposition).bind(original).bind(receipt).bind(kind).bind(input.movement.qty.as_micros()).bind(value).bind(&snapshot).execute(&mut **tx).await?;
    if kind == "RETURN" {
        let source_input = &source["snapshot"]["input"];
        let location = Uuid::parse_str(
            source_input["location_id"]
                .as_str()
                .ok_or(DispositionError::Scope)?,
        )?;
        let batch = source["snapshot"]["source"]["stock"]["batch_code"]
            .as_str()
            .ok_or(DispositionError::Scope)?;
        sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,$6,'HOLD','ON_HAND',$7,$8,$9)")
   .bind(tenant.0).bind(movement).bind(input.warehouse_id).bind(location).bind(input.sku_id).bind(batch)
   .bind(input.movement.qty.as_micros()).bind(movement.to_string()).bind(format!("rework-return:{movement}")).execute(&mut **tx).await?;
    }
    OperationLogsRepo::insert_with_tx(
        &mut **tx,
        OperationLogCreate {
            tenant_id: tenant,
            actor_type: "USER".into(),
            actor_user_id: Some(actor.user_id),
            action: "REWORK_MATERIAL".into(),
            ref_type: Some("mfg_material_move".into()),
            ref_id: Some(movement.to_string()),
            metadata: Some(snapshot.clone()),
        },
    )
    .await?;
    AuditLogsRepo::insert_with_tx(
        &mut **tx,
        AuditLogCreate {
            tenant_id: tenant,
            user_id: Some(actor.user_id),
            action: "manufacturing.rework.material".into(),
            entity_type: "mfg_material_move".into(),
            entity_id: Some(movement.to_string()),
            before_json: None,
            after_json: Some(snapshot.clone()),
            ip: None,
            ua: None,
        },
    )
    .await?;
    crate::outbox::OutboxRepo::enqueue_with_tx(
        &mut **tx,
        tenant,
        "manufacturing.rework.material",
        &snapshot,
        chrono::Utc::now(),
    )
    .await?;
    permit.recheck(tx).await?;
    Ok(())
}
