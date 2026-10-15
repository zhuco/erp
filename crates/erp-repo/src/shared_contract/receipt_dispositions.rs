//! Quality decisions consume a proven original receipt or an explicitly isolated allocation.
use super::{
    ReceiptValueActor, SharedContractRepo, SharedWrite, insert_receipt, load_receipt,
    production_receipt_release, receipt_authority, succeeded,
};
use crate::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
    operation_logs::{OperationLogCreate, OperationLogsRepo},
};
use erp_domain::{
    quantity::ScaledQty,
    receipt::{ReceiptError, admit_retry},
};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DispositionError {
    #[error("original quality source requires reconciliation: {0}")]
    Reconcile(&'static str),
    #[error("quality source is outside the selected stock scope")]
    Scope,
    #[error("quality disposition exceeds its source remainder")]
    Quantity,
    #[error("quality disposition requires a confirmed original production cost")]
    Cost,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptDispositionInput {
    pub receipt_id: Uuid,
    pub parent_disposition_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returned_source_move_id: Option<Uuid>,
    pub warehouse_id: Uuid,
    pub location_id: Uuid,
    pub sku_id: Uuid,
    pub kind: String,
    pub qty_micros: i64,
    pub reason: String,
    pub rework_work_order_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptDispositionResult {
    pub disposition_id: Uuid,
    pub receipt_id: Uuid,
    pub source_remaining_micros: i64,
    pub version_no: i32,
    pub cost_cent: Option<i64>,
    pub material_move_id: Option<Uuid>,
}
impl SharedContractRepo<'_> {
    pub async fn receipt_disposition_state(
        &self,
        tenant: TenantId,
        receipt: Uuid,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        let mut tx = self.pool.begin().await?;
        let (_, qty, proof) =
            production_receipt_release::held_source(&mut tx, tenant, receipt).await?;
        if actor.warehouse_id != proof.warehouse {
            return Err(DispositionError::Scope.into());
        }
        let permit =
            receipt_authority::lock_permissions(&mut tx, tenant, actor, &["quality.read"]).await?;
        let state:Value=sqlx::query_scalar("select jsonb_build_object(
            'receipt_id',$2::uuid,'original_micros',$3::bigint,
            'held_micros',($3::numeric-coalesce(sum(qty_micros) filter(where parent_disposition_id is null and returned_source_move_id is null),0))::bigint,
            'isolated_micros',(coalesce(sum(qty_micros) filter(where kind='ISOLATE'),0)-coalesce(sum(qty_micros) filter(where parent_disposition_id is not null or returned_source_move_id is not null),0)+(select coalesce(sum(qty_micros),0) from mfg_rework_material_values where tenant_id=$1 and receipt_id=$2 and kind='RETURN'))::bigint,
            'available_micros',coalesce(sum(qty_micros) filter(where kind in('ACCEPT','CONCESSION')),0)::bigint,
            'scrapped_micros',coalesce(sum(qty_micros) filter(where kind='SCRAP'),0)::bigint,
            'rework_micros',(coalesce(sum(qty_micros) filter(where kind='REWORK'),0)-(select coalesce(sum(qty_micros),0) from mfg_rework_material_values where tenant_id=$1 and receipt_id=$2 and kind='RETURN'))::bigint,
            'rework_returned_micros',(select coalesce(sum(qty_micros),0)::bigint from mfg_rework_material_values where tenant_id=$1 and receipt_id=$2 and kind='RETURN'),
            'returned_micros',coalesce(sum(qty_micros) filter(where kind='RETURN_SUPPLIER'),0)::bigint,
            'version_no',coalesce(max(version_no),0)) from mfg_receipt_disposition_commands where tenant_id=$1 and receipt_id=$2")
            .bind(tenant.0).bind(receipt).bind(qty).fetch_one(&mut *tx).await?;
        let fields = [
            "held_micros",
            "isolated_micros",
            "available_micros",
            "scrapped_micros",
            "rework_micros",
            "returned_micros",
        ];
        let mut total = 0i128;
        for field in fields {
            let amount = state[field].as_i64().ok_or(DispositionError::Reconcile(
                "source summary quantity is unrepresentable",
            ))?;
            if amount < 0 {
                return Err(
                    DispositionError::Reconcile("source summary quantity is negative").into(),
                );
            }
            total += i128::from(amount);
        }
        if total != i128::from(qty) {
            return Err(DispositionError::Reconcile(
                "source summary does not conserve original quantity",
            )
            .into());
        }
        permit.recheck(&mut tx).await?;
        tx.rollback().await?;
        Ok(state)
    }
    pub async fn dispose_receipt(
        &self,
        tenant: TenantId,
        request: &str,
        input: &ReceiptDispositionInput,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Result<(SharedWrite, ReceiptDispositionResult), ReceiptError>> {
        actor.validate(tenant)?;
        if input.receipt_id.is_nil()
            || input.qty_micros <= 0
            || input.reason.len() > 4096
            || input.kind.len() > 32
            || input.parent_disposition_id.is_some_and(|id| id.is_nil())
            || input.returned_source_move_id.is_some_and(|id| id.is_nil())
            || (input.parent_disposition_id.is_some() && input.returned_source_move_id.is_some())
        {
            return Err(DispositionError::Quantity.into());
        }
        let payload = format!(
            "receipt-disposition-v1:{:x}",
            Sha256::digest(serde_json::to_vec(&(tenant.0, input))?)
        );
        let mut tx = self.pool.begin().await?;
        // Work-order locks always precede receipt, user and stock locks.
        let owner: Uuid = sqlx::query_scalar(
            "select work_order_id from mfg_receipts where tenant_id=$1 and receipt_id=$2",
        )
        .bind(tenant.0)
        .bind(input.receipt_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DispositionError::Scope)?;
        let mut owners = vec![owner];
        if let Some(rework) = input.rework_work_order_id {
            owners.push(rework);
        }
        owners.sort();
        owners.dedup();
        for order in owners {
            production_receipt_release::lock_quality_source(&mut tx, tenant, "WORK_ORDER", order)
                .await?;
        }
        let (_, original_qty, proof) =
            production_receipt_release::held_source(&mut tx, tenant, input.receipt_id).await?;
        if proof.warehouse != input.warehouse_id
            || proof.location != input.location_id
            || proof.sku != input.sku_id
            || actor.warehouse_id != input.warehouse_id
        {
            return Err(DispositionError::Scope.into());
        }
        let permit =
            receipt_authority::lock_permissions(&mut tx, tenant, actor, &["quality.write"]).await?;
        if input.rework_work_order_id.is_some() {
            receipt_authority::lock_permissions(
                &mut tx,
                tenant,
                actor,
                &["MFG.PRODUCTION.WRITE", "manufacturing.write"],
            )
            .await?;
        }
        let override_grant = if input.kind == "CONCESSION" {
            Some(
                receipt_authority::lock_permissions(
                    &mut tx,
                    tenant,
                    actor,
                    &["quality.concession", "QUALITY.CONCESSION"],
                )
                .await?
                .snapshot(),
            )
        } else {
            None
        };
        if let Some(old) = load_receipt(&mut tx, tenant, request).await? {
            match admit_retry(Some(&old), request, &payload) {
                Err(e) => {
                    tx.rollback().await?;
                    return Ok(Err(e));
                }
                Ok(_) => {
                    let snapshot:Option<Value>=sqlx::query_scalar("select snapshot from mfg_receipt_disposition_commands where tenant_id=$1 and request_id=$2 and disposition_id::text=$3 and payload_hash=$4")
                        .bind(tenant.0).bind(request).bind(&old.effect_id).bind(&payload).fetch_optional(&mut *tx).await?;
                    let snapshot = snapshot.ok_or(DispositionError::Reconcile(
                        "original disposition result is missing",
                    ))?;
                    let result: ReceiptDispositionResult =
                        serde_json::from_value(snapshot["result"].clone())?;
                    permit.recheck(&mut tx).await?;
                    tx.rollback().await?;
                    return Ok(Ok((SharedWrite::Replay(old), result)));
                }
            }
        }
        if let Err(e) = admit_retry(None, request, &payload) {
            return Ok(Err(e));
        }
        let (source_qty, source_quality, depth) = if let Some(returned) =
            input.returned_source_move_id
        {
            let row:Option<(i64,i32)>=sqlx::query_as("select v.qty_micros,c.depth from mfg_rework_material_values v join mfg_receipt_disposition_commands c on c.tenant_id=v.tenant_id and c.disposition_id=v.disposition_id where v.tenant_id=$1 and v.receipt_id=$2 and v.move_id=$3 and v.kind='RETURN'")
                .bind(tenant.0).bind(input.receipt_id).bind(returned).fetch_optional(&mut *tx).await?;
            let (qty, depth) = row.ok_or(DispositionError::Scope)?;
            if depth >= 32 {
                return Err(DispositionError::Scope.into());
            }
            (qty, "HOLD", depth + 1)
        } else if let Some(parent) = input.parent_disposition_id {
            let row:Option<(i64,String,i32)>=sqlx::query_as("select qty_micros,kind,depth from mfg_receipt_disposition_commands where tenant_id=$1 and receipt_id=$2 and disposition_id=$3 for share")
                .bind(tenant.0).bind(input.receipt_id).bind(parent).fetch_optional(&mut *tx).await?;
            let (qty, kind, depth) = row.ok_or(DispositionError::Scope)?;
            if kind != "ISOLATE" || depth >= 32 {
                return Err(DispositionError::Scope.into());
            }
            (qty, "ISOLATED", depth + 1)
        } else {
            (original_qty, "HOLD", 0)
        };
        let used:i64=sqlx::query_scalar("select coalesce(sum(qty_micros::numeric),0)::bigint from mfg_receipt_disposition_commands where tenant_id=$1 and receipt_id=$2 and parent_disposition_id is not distinct from $3 and returned_source_move_id is not distinct from $4")
            .bind(tenant.0).bind(input.receipt_id).bind(input.parent_disposition_id).bind(input.returned_source_move_id).fetch_one(&mut *tx).await?;
        let remaining = source_qty
            .checked_sub(used)
            .ok_or(DispositionError::Quantity)?;
        if input.qty_micros > remaining {
            return Err(DispositionError::Quantity.into());
        }
        let version:i32=sqlx::query_scalar("select coalesce(max(version_no),0)+1 from mfg_receipt_disposition_commands where tenant_id=$1 and receipt_id=$2")
            .bind(tenant.0).bind(input.receipt_id).fetch_one(&mut *tx).await?;
        let qty = ScaledQty::from_micros(input.qty_micros);
        erp_domain::manufacturing::admit_cited_disposition(
            erp_domain::manufacturing::CitedDispositionInput {
                held: ScaledQty::from_micros(remaining),
                kind: &input.kind,
                qty,
                reason: &input.reason,
                authorizer: &actor.user_id.to_string(),
                inspection_id: &input.receipt_id.to_string(),
                batch_code: &proof.batch,
                location_id: &proof.location.to_string(),
                prior_version: version - 1,
                unit_cost_cent: 0,
            },
        )
        .map_err(|_| DispositionError::Quantity)?;
        if input.kind == "ACCEPT" {
            production_receipt_release::inspection_proof(
                &mut tx,
                tenant,
                owner,
                &proof.batch,
                Some(input.receipt_id),
            )
            .await?;
        }
        let decision_evidence = if input.kind == "CONCESSION" {
            if input.reason.trim().is_empty() {
                return Err(DispositionError::Quantity.into());
            }
            Some(
                production_receipt_release::concession_inspection_proof(
                    &mut tx,
                    tenant,
                    owner,
                    &proof.batch,
                    input.receipt_id,
                )
                .await?,
            )
        } else {
            None
        };
        let leaves = matches!(input.kind.as_str(), "SCRAP" | "RETURN_SUPPLIER" | "REWORK");
        // A manufactured arrival has no supplier-return source. Procurement
        // returns must cite their actual purchase receipt and original payable.
        if input.kind == "RETURN_SUPPLIER" {
            return Err(DispositionError::Reconcile(
                "supplier return needs its original purchase receipt, not manufactured goods",
            )
            .into());
        }
        if (input.kind == "REWORK") != input.rework_work_order_id.is_some() {
            return Err(DispositionError::Scope.into());
        }
        if let Some(rework) = input.rework_work_order_id {
            if rework == owner {
                return Err(DispositionError::Scope.into());
            }
            let active:bool=sqlx::query_scalar("select exists(select 1 from mfg_work_orders where tenant_id=$1 and work_order_id=$2 and status='RELEASED')")
                .bind(tenant.0).bind(rework).fetch_one(&mut *tx).await?;
            if !active {
                return Err(DispositionError::Scope.into());
            }
            let capacity:bool=sqlx::query_scalar("select order_qty_micros-issued_micros >= $3 from mfg_work_orders where tenant_id=$1 and work_order_id=$2")
                .bind(tenant.0).bind(rework).bind(input.qty_micros).fetch_one(&mut *tx).await?;
            if !capacity {
                return Err(DispositionError::Quantity.into());
            }
            let cited:bool=sqlx::query_scalar("select exists(select 1 from mfg_exceptions where tenant_id=$1 and work_order_id=$2 and kind='REWORK' and source_work_order_id=$3)")
                .bind(tenant.0).bind(rework).bind(owner).fetch_one(&mut *tx).await?;
            if !cited {
                return Err(DispositionError::Scope.into());
            }
        }
        let value = if leaves {
            Some(cost_allocation(&mut tx, tenant, input, owner, original_qty).await?)
        } else {
            None
        };
        let disposition = Uuid::new_v4();
        let material = if input.kind == "REWORK" {
            Some(Uuid::new_v4())
        } else {
            None
        };
        let first = material.unwrap_or_else(Uuid::new_v4);
        let key = InventoryQuantityKey {
            tenant_id: tenant,
            warehouse_id: proof.warehouse,
            sku_id: proof.sku,
        };
        let mut legs = vec![(first, -input.qty_micros, -input.qty_micros, source_quality)];
        if !leaves {
            legs.push((
                Uuid::new_v4(),
                input.qty_micros,
                if input.kind == "ISOLATE" {
                    input.qty_micros
                } else {
                    0
                },
                if input.kind == "ISOLATE" {
                    "ISOLATED"
                } else {
                    "GOOD"
                },
            ));
        }
        let mut effects = Vec::with_capacity(2);
        for (stock, delta, frozen, quality) in legs {
            let effect = InventoryService::new(self.pool)
                .apply_precise_effect_with_tx(
                    &mut tx,
                    InventoryPreciseDelta {
                        key,
                        on_hand: ScaledQty::from_micros(delta),
                        reserved: ScaledQty::ZERO,
                        frozen: ScaledQty::from_micros(frozen),
                        in_transit: ScaledQty::ZERO,
                        reason_type: "QUALITY_DISPOSITION".into(),
                        ref_type: Some("shared_stock_ledger".into()),
                        ref_id: Some(stock.to_string()),
                        occurred_at: chrono::Utc::now(),
                    },
                )
                .await?;
            let projection:Uuid=sqlx::query_scalar("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,ref_type,ref_id,occurred_at) select tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,'mfg_material_move',$5,occurred_at from inventory_ledgers where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 and ledger_id=$4 returning ledger_id")
                .bind(tenant.0).bind(proof.warehouse).bind(proof.sku).bind(effect.ledger_id).bind(stock.to_string()).fetch_one(&mut *tx).await?;
            sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type,source_receipt_id) values($1,$2,$3,$4,$5,'QUALITY_DISPOSITION',$6)")
                .bind(tenant.0).bind(proof.warehouse).bind(proof.sku).bind(stock).bind(delta).bind(input.receipt_id).execute(&mut *tx).await?;
            sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,$6,$7,'ON_HAND',$8,$9,$10)")
                .bind(tenant.0).bind(stock).bind(proof.warehouse).bind(proof.location).bind(proof.sku).bind(&proof.batch)
                .bind(quality).bind(delta).bind(disposition.to_string()).bind(request).execute(&mut *tx).await?;
            effects.push(json!({"stock_ledger_id":stock,"inventory_ledger_id":effect.ledger_id,"projection_ledger_id":projection,
                "delta_micros":delta,"delta_frozen_micros":frozen,"quality":quality}));
        }
        if let (Some(move_id), Some(rework)) = (material, input.rework_work_order_id) {
            sqlx::query("insert into mfg_material_moves(tenant_id,move_id,work_order_id,kind,qty_micros,reason,warehouse_id,sku_id,material_issued_micros,material_returned_micros,material_consumed_micros,material_shop_micros) values($1,$2,$3,'ISSUE',$4,$5,$6,$7,$4,0,0,$4)")
                .bind(tenant.0).bind(move_id).bind(rework).bind(input.qty_micros).bind(&input.reason).bind(proof.warehouse).bind(proof.sku).execute(&mut *tx).await?;
            sqlx::query("update mfg_work_orders set issued_micros=issued_micros+$3 where tenant_id=$1 and work_order_id=$2")
                .bind(tenant.0).bind(rework).bind(input.qty_micros).execute(&mut *tx).await?;
        }
        let result = ReceiptDispositionResult {
            disposition_id: disposition,
            receipt_id: input.receipt_id,
            source_remaining_micros: remaining - input.qty_micros,
            version_no: version,
            cost_cent: value,
            material_move_id: material,
        };
        let snapshot = json!({"input":input,"result":result,"source":proof.snapshot,"actor":permit.snapshot(),"effects":effects,"decision_evidence":decision_evidence,"override_grant":override_grant});
        sqlx::query("insert into mfg_quality_dispositions(tenant_id,disposition_id,work_order_id,batch_code,warehouse_id,sku_id,kind,qty_micros,reason,authorizer,held_left_micros,quality_status,version_no) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
            .bind(tenant.0).bind(disposition).bind(owner).bind(&proof.batch).bind(proof.warehouse).bind(proof.sku).bind(&input.kind)
            .bind(input.qty_micros).bind(&input.reason).bind(actor.user_id.to_string()).bind(result.source_remaining_micros)
            .bind(&input.kind).bind(version).execute(&mut *tx).await?;
        let command = succeeded(request, &payload, &disposition.to_string());
        insert_receipt(&mut tx, tenant, &command).await?;
        sqlx::query("insert into mfg_receipt_disposition_commands(tenant_id,request_id,disposition_id,receipt_id,parent_disposition_id,kind,qty_micros,held_left_micros,depth,version_no,cost_cent,rework_work_order_id,material_move_id,payload_hash,snapshot,returned_source_move_id) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
            .bind(tenant.0).bind(request).bind(disposition).bind(input.receipt_id).bind(input.parent_disposition_id).bind(&input.kind)
            .bind(input.qty_micros).bind(result.source_remaining_micros).bind(depth).bind(version).bind(value).bind(input.rework_work_order_id)
            .bind(material).bind(&payload).bind(&snapshot).bind(input.returned_source_move_id).execute(&mut *tx).await?;
        permit.recheck(&mut tx).await?;
        OperationLogsRepo::insert_with_tx(
            &mut *tx,
            OperationLogCreate {
                tenant_id: tenant,
                actor_type: "USER".into(),
                actor_user_id: Some(actor.user_id),
                action: "QUALITY_RECEIPT_DISPOSITION".into(),
                ref_type: Some("mfg_receipt".into()),
                ref_id: Some(input.receipt_id.to_string()),
                metadata: Some(snapshot.clone()),
            },
        )
        .await?;
        AuditLogsRepo::insert_with_tx(
            &mut *tx,
            AuditLogCreate {
                tenant_id: tenant,
                user_id: Some(actor.user_id),
                action: "quality.receipt.disposition".into(),
                entity_type: "mfg_receipt".into(),
                entity_id: Some(input.receipt_id.to_string()),
                before_json: None,
                after_json: Some(snapshot),
                ip: None,
                ua: None,
            },
        )
        .await?;
        permit.recheck(&mut tx).await?;
        tx.commit().await?;
        Ok(Ok((SharedWrite::Stored(command), result)))
    }
}

async fn cost_allocation(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    input: &ReceiptDispositionInput,
    owner: Uuid,
    total_qty: i64,
) -> anyhow::Result<i64> {
    let receipt = input.receipt_id;
    let qty = input.qty_micros;
    let parent = input.parent_disposition_id;
    let returned = input.returned_source_move_id;
    let tree = "with recursive tree as (select disposition_id,parent_disposition_id,returned_source_move_id as origin,kind,qty_micros from mfg_receipt_disposition_commands where tenant_id=$1 and receipt_id=$2 and parent_disposition_id is null union all select c.disposition_id,c.parent_disposition_id,coalesce(c.returned_source_move_id,p.origin),c.kind,c.qty_micros from mfg_receipt_disposition_commands c join tree p on p.disposition_id=c.parent_disposition_id where c.tenant_id=$1 and c.receipt_id=$2)";
    let origin = if let Some(parent) = parent {
        sqlx::query_scalar::<_, Option<Uuid>>(&format!(
            "{tree} select origin from tree where disposition_id=$3"
        ))
        .bind(tenant.0)
        .bind(receipt)
        .bind(parent)
        .fetch_one(&mut **tx)
        .await?
    } else {
        returned
    };
    if let Some(origin) = origin {
        let (available,value):(i64,i64)=sqlx::query_as("select qty_micros,cost_cent from mfg_rework_material_values where tenant_id=$1 and move_id=$2 and receipt_id=$3 and kind='RETURN'")
            .bind(tenant.0).bind(origin).bind(receipt).fetch_one(&mut **tx).await?;
        let prior:i64=sqlx::query_scalar(&format!("{tree} select coalesce(sum(qty_micros),0)::bigint from tree where origin=$3 and kind in('SCRAP','REWORK','RETURN_SUPPLIER')"))
            .bind(tenant.0).bind(receipt).bind(origin).fetch_one(&mut **tx).await?;
        let next = prior.checked_add(qty).ok_or(DispositionError::Quantity)?;
        if available <= 0 || next > available {
            return Err(DispositionError::Quantity.into());
        }
        return Ok(i64::try_from(
            i128::from(value) * i128::from(next) / i128::from(available)
                - i128::from(value) * i128::from(prior) / i128::from(available),
        )?);
    }
    let frozen: Option<i64> = sqlx::query_scalar(
        "select unit_cent from mfg_receipt_disposition_costs where tenant_id=$1 and receipt_id=$2",
    )
    .bind(tenant.0)
    .bind(receipt)
    .fetch_optional(&mut **tx)
    .await?;
    let unit = if let Some(unit) = frozen {
        unit
    } else {
        let source:Option<(Uuid,i64,Value)>=sqlx::query_as("select cost_run_id,unit_cent,to_jsonb(c) from mfg_cost_runs c where tenant_id=$1 and work_order_id=$2 and estimated=false and unit_cent is not null order by created_at desc,cost_run_id desc limit 1 for share")
            .bind(tenant.0).bind(owner).fetch_optional(&mut **tx).await?;
        let (cost, unit, snapshot) = source.ok_or(DispositionError::Cost)?;
        if unit < 0 {
            return Err(DispositionError::Cost.into());
        }
        let total = i64::try_from(i128::from(total_qty) * i128::from(unit) / 1_000_000)?;
        sqlx::query("insert into mfg_receipt_disposition_costs(tenant_id,receipt_id,cost_run_id,unit_cent,total_cent,snapshot) values($1,$2,$3,$4,$5,$6)")
            .bind(tenant.0).bind(receipt).bind(cost).bind(unit).bind(total).bind(snapshot).execute(&mut **tx).await?;
        unit
    };
    let prior:i64=sqlx::query_scalar(&format!("{tree} select coalesce(sum(qty_micros::numeric),0)::bigint from tree where origin is null and kind in('SCRAP','RETURN_SUPPLIER','REWORK')"))
        .bind(tenant.0).bind(receipt).fetch_one(&mut **tx).await?;
    let next = prior.checked_add(qty).ok_or(DispositionError::Quantity)?;
    if next > total_qty {
        return Err(DispositionError::Quantity.into());
    }
    Ok(i64::try_from(
        (i128::from(next) * i128::from(unit) / 1_000_000)
            - (i128::from(prior) * i128::from(unit) / 1_000_000),
    )?)
}
