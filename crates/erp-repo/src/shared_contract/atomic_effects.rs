//! The original atomic route posts an owned, precise material ISSUE with server value.
use super::{
    AtomicEffect, ProductionReleaseActor, SharedContractRepo, SharedWrite, insert_receipt,
    inspection_blocks, load_receipt, material_moves::append_material_move_with_tx,
};
use crate::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    inventory::InventoryService,
    inventory_precision::InventoryQuantityKey,
    operation_logs::{OperationLogCreate, OperationLogsRepo},
    outbox::OutboxRepo,
};
use erp_domain::{
    manufacturing::{MaterialMove, MaterialMoveKind, admit_atomic_effect},
    quantity::QTY_SCALE,
    receipt::{ReceiptError, admit_retry, succeeded},
};
use erp_types::TenantId;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AtomicEffectError {
    #[error("atomic material source is outside this enterprise")]
    NotFound,
    #[error("atomic material original proof requires reconciliation")]
    ReconciliationRequired,
}

pub struct AtomicEffectInput<'a> {
    pub work_order_id: Uuid,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub effect: &'a AtomicEffect,
}

pub struct AtomicEffectWrite {
    pub effect: SharedWrite,
    pub document_id: Uuid,
    pub snapshot: Value,
}

#[derive(sqlx::FromRow)]
struct Original {
    work_order_id: Option<Uuid>,
    warehouse_id: Option<Uuid>,
    sku_id: Option<Uuid>,
    actor_user_id: Option<Uuid>,
    source_action: String,
    action_no: String,
    qty_micros: i64,
    amount_cent: i64,
    request_id: Option<String>,
    snapshot: Option<Value>,
}

impl SharedContractRepo<'_> {
    pub async fn commit_atomic_effect_with_actor(
        &self,
        tenant: TenantId,
        request_id: &str,
        payload_hash: &str,
        input: AtomicEffectInput<'_>,
        actor: &ProductionReleaseActor,
    ) -> anyhow::Result<Result<AtomicEffectWrite, ReceiptError>> {
        let request_id = request_id.trim();
        if request_id.is_empty() {
            return Ok(Err(ReceiptError::EmptyRequest));
        }
        admit_atomic_effect(input.effect)
            .map_err(|error| anyhow::anyhow!("atomic effect rejected: {error:?}"))?;
        let action = input.effect.action_no.trim();
        if action.len() > 128 || action.chars().any(char::is_control) {
            anyhow::bail!("atomic action number is outside its byte budget");
        }
        let mut tx = self.pool.begin().await?;
        actor
            .require_warehouse(&mut tx, tenant, input.warehouse_id)
            .await?;
        for lock in [
            format!("material-order:{}", input.work_order_id),
            format!("material-command:{request_id}"),
        ] {
            sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
                .bind(tenant.0.to_string())
                .bind(lock)
                .execute(&mut *tx)
                .await?;
        }
        let owned: bool = sqlx::query_scalar("select exists(select 1 from warehouses where tenant_id=$1 and warehouse_id=$2) and exists(select 1 from skus where tenant_id=$1 and sku_id=$3)")
            .bind(tenant.0).bind(input.warehouse_id).bind(input.sku_id).fetch_one(&mut *tx).await?;
        if !owned {
            return Err(AtomicEffectError::NotFound.into());
        }
        let actor_id = actor.user.as_ref().map(|user| user.user_id.0);
        match admit_retry(
            load_receipt(&mut tx, tenant, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(receipt)) => {
                let document_id = Uuid::parse_str(&receipt.effect_id)
                    .map_err(|_| AtomicEffectError::ReconciliationRequired)?;
                let original: Option<Original> = sqlx::query_as("select work_order_id,warehouse_id,sku_id,actor_user_id,source_action,action_no,qty_micros,amount_cent,request_id,snapshot from atomic_source_documents where tenant_id=$1 and document_id=$2")
                    .bind(tenant.0).bind(document_id).fetch_optional(&mut *tx).await?;
                let original = original.ok_or(AtomicEffectError::ReconciliationRequired)?;
                if original.work_order_id != Some(input.work_order_id)
                    || original.warehouse_id != Some(input.warehouse_id)
                    || original.sku_id != Some(input.sku_id)
                    || original.actor_user_id != actor_id
                    || original.source_action != input.effect.source_action.trim()
                    || original.action_no != action
                    || original.qty_micros != input.effect.qty.as_micros()
                    || original.amount_cent != input.effect.amount_cent
                    || original.request_id.as_deref() != Some(request_id)
                {
                    return Ok(Err(ReceiptError::PayloadConflict));
                }
                let snapshot = original
                    .snapshot
                    .ok_or(AtomicEffectError::ReconciliationRequired)?;
                verify_original(&mut tx, tenant, document_id, &snapshot).await?;
                tx.rollback().await?;
                return Ok(Ok(AtomicEffectWrite {
                    effect: SharedWrite::Replay(receipt),
                    document_id,
                    snapshot,
                }));
            }
            Err(error) => return Ok(Err(error)),
            Ok(None) => {}
        }
        let order: Option<(String, Option<Uuid>, i64, String)> = sqlx::query_as("select status,bom_id,order_qty_micros,product_code from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
            .bind(tenant.0).bind(input.work_order_id).fetch_optional(&mut *tx).await?;
        let (status, bom, planned, product) = order.ok_or(AtomicEffectError::NotFound)?;
        if status != "RELEASED" {
            anyhow::bail!("work order is not open for material issue");
        }
        if inspection_blocks(&mut tx, tenant, "WORK_ORDER", input.work_order_id, None).await? {
            anyhow::bail!("required inspection is incomplete or failed");
        }
        let (sku_code, cost): (String, Option<i64>) = sqlx::query_as(
            "select sku_code,cost_cent from skus where tenant_id=$1 and sku_id=$2 for share",
        )
        .bind(tenant.0)
        .bind(input.sku_id)
        .fetch_one(&mut *tx)
        .await?;
        let cost = cost
            .filter(|cost| *cost >= 0)
            .ok_or(AtomicEffectError::ReconciliationRequired)?;
        let exact_value = i128::from(input.effect.qty.as_micros()) * i128::from(cost);
        let cents =
            i64::try_from((exact_value + i128::from(QTY_SCALE / 2)) / i128::from(QTY_SCALE))
                .map_err(|_| anyhow::anyhow!("atomic material value projection overflow"))?;
        if cents != input.effect.amount_cent {
            anyhow::bail!("declared material amount differs from its server cost source");
        }
        let (required, source) =
            material_source(&mut tx, tenant, bom, planned, &product, &sku_code).await?;
        let issued: i64 = sqlx::query_scalar("select coalesce(sum(case when kind in ('ISSUE','SUPPLEMENT') then qty_micros when kind='RETURN' then -qty_micros else 0 end),0)::bigint from mfg_material_moves where tenant_id=$1 and work_order_id=$2 and sku_id=$3")
            .bind(tenant.0).bind(input.work_order_id).bind(input.sku_id).fetch_one(&mut *tx).await?;
        if issued < 0
            || i128::from(issued) + i128::from(input.effect.qty.as_micros()) > i128::from(required)
        {
            anyhow::bail!("material issue exceeds the original work-order requirement");
        }
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(tenant.0.to_string())
            .bind(format!("atomic:ISSUE:{action}"))
            .execute(&mut *tx)
            .await?;
        let already: bool = sqlx::query_scalar("select exists(select 1 from atomic_source_documents where tenant_id=$1 and source_action='ISSUE' and action_no=$2)")
            .bind(tenant.0).bind(action).fetch_one(&mut *tx).await?;
        if already {
            return Ok(Err(ReceiptError::PayloadConflict));
        }
        let document_id = Uuid::new_v4();
        let (move_id, material) = append_material_move_with_tx(
            &mut tx,
            self,
            tenant,
            super::MaterialMoveInput {
                work_order_id: input.work_order_id,
                warehouse_id: input.warehouse_id,
                sku_id: input.sku_id,
                batch_id: None,
                original_issue_id: None,
                movement: MaterialMove {
                    kind: MaterialMoveKind::Issue,
                    qty: input.effect.qty,
                    supplement_reason_required: false,
                },
                reason: Some(action),
            },
            &status,
        )
        .await?;
        let key = InventoryQuantityKey {
            tenant_id: tenant,
            warehouse_id: input.warehouse_id,
            sku_id: input.sku_id,
        };
        let stock = InventoryService::new(self.pool)
            .get_precise_balance_with_tx(&mut tx, key)
            .await?
            .ok_or(AtomicEffectError::ReconciliationRequired)?;
        let ledger: Uuid = sqlx::query_scalar("select ledger_id from inventory_ledgers where tenant_id=$1 and ref_type='mfg_material_move' and ref_id=$2 and delta_on_hand_micros=$3")
            .bind(tenant.0).bind(move_id.to_string()).bind(-input.effect.qty.as_micros()).fetch_one(&mut *tx).await?;
        let mut snapshot = json!({"schema":1,"document_id":document_id,"request_id":request_id,"work_order_id":input.work_order_id,
            "warehouse_id":input.warehouse_id,"sku_id":input.sku_id,"actor_user_id":actor_id,"action":"ISSUE","action_no":action,
            "move_id":move_id,"inventory_ledger_id":ledger,"qty_micros":input.effect.qty.as_micros().to_string(),
            "unit_cost_cent":cost,"cost_source":"SKU.cost_cent","value_cent_micros":exact_value.to_string(),"amount_cent":cents,
            "material":{"issued_micros":material.issued.as_micros().to_string(),"returned_micros":material.returned.as_micros().to_string(),
                "consumed_micros":material.consumed.as_micros().to_string(),"shop_micros":material.shop_qty.as_micros().to_string()},
            "stock":{"on_hand_micros":stock.on_hand.as_micros().to_string(),"reserved_micros":stock.reserved.as_micros().to_string(),
                "frozen_micros":stock.frozen.as_micros().to_string(),"in_transit_micros":stock.in_transit.as_micros().to_string(),
                "available_micros":stock.available()?.as_micros().to_string()},"source":source,"required_micros":required.to_string()});
        let user = actor.user.as_ref();
        let operation = OperationLogsRepo::insert_with_tx(
            &mut *tx,
            OperationLogCreate {
                tenant_id: tenant,
                actor_type: if user.is_some() { "USER" } else { "SYSTEM" }.into(),
                actor_user_id: user.map(|user| user.user_id),
                action: "MFG_ATOMIC_ISSUE".into(),
                ref_type: Some("atomic_material_document".into()),
                ref_id: Some(document_id.to_string()),
                metadata: Some(snapshot.clone()),
            },
        )
        .await?;
        let audit = AuditLogsRepo::insert_with_tx(
            &mut *tx,
            AuditLogCreate {
                tenant_id: tenant,
                user_id: user.map(|user| user.user_id),
                action: "mfg.atomic.issue".into(),
                entity_type: "atomic_material_document".into(),
                entity_id: Some(document_id.to_string()),
                before_json: None,
                after_json: Some(snapshot.clone()),
                ip: user.and_then(|user| user.ip.clone()),
                ua: user.and_then(|user| user.user_agent.clone()),
            },
        )
        .await?;
        snapshot["operation_log_id"] = json!(operation.log_id);
        snapshot["audit_id"] = json!(audit.audit_id);
        let outbox_id = OutboxRepo::enqueue_with_tx(
            &mut *tx,
            tenant,
            "manufacturing.material.issued",
            &snapshot,
            chrono::Utc::now(),
        )
        .await?;
        snapshot["outbox_id"] = json!(outbox_id);
        let posting_id = Uuid::new_v4();
        snapshot["posting_id"] = json!(posting_id);
        sqlx::query("insert into atomic_source_documents(tenant_id,document_id,source_action,action_no,qty_micros,amount_cent,work_order_id,warehouse_id,sku_id,actor_user_id,move_id,request_id,snapshot) values($1,$2,'ISSUE',$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
            .bind(tenant.0).bind(document_id).bind(action).bind(input.effect.qty.as_micros()).bind(cents).bind(input.work_order_id)
            .bind(input.warehouse_id).bind(input.sku_id).bind(actor_id).bind(move_id).bind(request_id).bind(&snapshot).execute(&mut *tx).await?;
        sqlx::query("insert into atomic_material_values(tenant_id,document_id,move_id,qty_micros,unit_cost_cent,value_cent_micros) values($1,$2,$3,$4,$5,$6::text::numeric)")
            .bind(tenant.0).bind(document_id).bind(move_id).bind(input.effect.qty.as_micros()).bind(cost).bind(exact_value.to_string()).execute(&mut *tx).await?;
        sqlx::query("insert into atomic_posting_events(tenant_id,event_id,document_id,source_action,action_no,amount_cent,posting_status) values($1,$2,$3,'ISSUE',$4,$5,'PENDING_POSTING')")
            .bind(tenant.0).bind(posting_id).bind(document_id).bind(action).bind(cents).execute(&mut *tx).await?;
        let receipt = succeeded(request_id, payload_hash, &document_id.to_string());
        insert_receipt(&mut tx, tenant, &receipt).await?;
        tx.commit().await?;
        Ok(Ok(AtomicEffectWrite {
            effect: SharedWrite::Stored(receipt),
            document_id,
            snapshot,
        }))
    }
}

pub(super) async fn material_source(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    version: Option<Uuid>,
    planned: i64,
    product: &str,
    sku: &str,
) -> anyhow::Result<(i64, Value)> {
    let version = version.ok_or(AtomicEffectError::ReconciliationRequired)?;
    let rows: Vec<(i32,String,i64,String,i64,String)> = sqlx::query_as("select v.version_no,v.status,l.qty_per_micros,l.unit_code,l.scrap_ppm,l.parent_code from shared_mfg_bom_versions v join shared_mfg_bom_version_lines l on l.tenant_id=v.tenant_id and l.version_id=v.version_id where v.tenant_id=$1 and v.version_id=$2 and l.component_code=$3 order by l.line_no limit 1001 for share of v,l")
        .bind(tenant.0).bind(version).bind(sku).fetch_all(&mut **tx).await?;
    if rows.is_empty() || rows.len() > 1000 || planned <= 0 {
        return Err(AtomicEffectError::ReconciliationRequired.into());
    }
    let unit = &rows[0].3;
    let mut required = 0_i128;
    for (version_no, status, per, row_unit, scrap, parent) in &rows {
        if *version_no <= 0
            || !matches!(status.as_str(), "PUBLISHED" | "RETIRED")
            || *per <= 0
            || unit.is_empty()
            || row_unit != unit
            || !(0..=1_000_000).contains(scrap)
            || parent != product
        {
            return Err(AtomicEffectError::ReconciliationRequired.into());
        }
        let numerator = i128::from(planned)
            .checked_mul(i128::from(*per))
            .and_then(|value| value.checked_mul(i128::from(QTY_SCALE + scrap)))
            .ok_or_else(|| anyhow::anyhow!("frozen material requirement overflow"))?;
        let denominator = i128::from(QTY_SCALE) * i128::from(QTY_SCALE);
        let rounded = numerator
            .checked_add(denominator - 1)
            .ok_or_else(|| anyhow::anyhow!("frozen material requirement overflow"))?
            / denominator;
        required = required
            .checked_add(rounded)
            .ok_or_else(|| anyhow::anyhow!("frozen material requirement overflow"))?;
    }
    let required = i64::try_from(required)
        .map_err(|_| anyhow::anyhow!("frozen material requirement overflow"))?;
    Ok((
        required,
        json!({"version_id":version,"version_no":rows[0].0,"unit_code":unit,"sku_code":sku,
        "order_qty_micros":planned.to_string(),"lines":rows}),
    ))
}

pub(super) async fn verify_original(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    document: Uuid,
    snapshot: &Value,
) -> anyhow::Result<()> {
    let parse = |name: &str| -> anyhow::Result<Uuid> {
        snapshot[name]
            .as_str()
            .and_then(|value| Uuid::parse_str(value).ok())
            .ok_or_else(|| AtomicEffectError::ReconciliationRequired.into())
    };
    let qty = snapshot["qty_micros"]
        .as_str()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or(AtomicEffectError::ReconciliationRequired)?;
    let valid: bool = sqlx::query_scalar(r#"
        select exists(select 1 from atomic_source_documents d
          where d.tenant_id=$1 and d.document_id=$7 and d.snapshot=$14
            and d.work_order_id=$3 and d.warehouse_id=$4 and d.sku_id=$5 and d.move_id=$2
            and d.source_action='ISSUE' and d.action_no=$14->>'action_no'
            and d.request_id=$14->>'request_id' and d.qty_micros=$6
            and d.amount_cent=($14->>'amount_cent')::bigint
            and d.actor_user_id is not distinct from ($14->>'actor_user_id')::uuid
            and $14->>'document_id'=$7::text and $14->>'action'='ISSUE' and $14->>'schema'='1')
        and exists(select 1 from mfg_material_moves m
          where m.tenant_id=$1 and m.move_id=$2 and m.work_order_id=$3 and m.warehouse_id=$4
            and m.sku_id=$5 and m.kind='ISSUE' and m.qty_micros=$6
            and m.batch_id is null and m.original_issue_id is null and m.reason=$14->>'action_no'
            and m.material_issued_micros=($14->'material'->>'issued_micros')::bigint
            and m.material_returned_micros=($14->'material'->>'returned_micros')::bigint
            and m.material_consumed_micros=($14->'material'->>'consumed_micros')::bigint
            and m.material_shop_micros=($14->'material'->>'shop_micros')::bigint)
        and exists(select 1 from atomic_material_values v
          where v.tenant_id=$1 and v.document_id=$7 and v.move_id=$2 and v.qty_micros=$6
            and v.value_cent_micros::text=$8 and v.unit_cost_cent=($14->>'unit_cost_cent')::bigint)
        and exists(select 1 from inventory_ledgers l
          where l.tenant_id=$1 and l.ledger_id=$9 and l.warehouse_id=$4 and l.sku_id=$5
            and l.ref_type='mfg_material_move' and l.ref_id=$2::text and l.delta_on_hand_micros=-$6)
        and exists(select 1 from operation_logs o
          where o.tenant_id=$1 and o.log_id=$10 and o.ref_id=$7::text and o.action='MFG_ATOMIC_ISSUE'
            and o.ref_type='atomic_material_document'
            and o.actor_user_id is not distinct from ($14->>'actor_user_id')::uuid
            and o.metadata=($14-'operation_log_id'-'audit_id'-'outbox_id'-'posting_id'))
        and exists(select 1 from audit_logs a
          where a.tenant_id=$1 and a.audit_id=$11 and a.entity_id=$7::text and a.action='mfg.atomic.issue'
            and a.entity_type='atomic_material_document'
            and a.user_id is not distinct from ($14->>'actor_user_id')::uuid
            and a.after_json=($14-'operation_log_id'-'audit_id'-'outbox_id'-'posting_id'))
        and exists(select 1 from outbox_events e
          where e.tenant_id=$1 and e.outbox_id=$12 and e.event_type='manufacturing.material.issued'
            and e.payload=($14-'outbox_id'-'posting_id'))
        and exists(select 1 from atomic_posting_events p
          where p.tenant_id=$1 and p.event_id=$13 and p.document_id=$7 and p.source_action='ISSUE'
            and p.action_no=$14->>'action_no' and p.amount_cent=($14->>'amount_cent')::bigint)
        "#)
        .bind(tenant.0).bind(parse("move_id")?).bind(parse("work_order_id")?).bind(parse("warehouse_id")?).bind(parse("sku_id")?).bind(qty)
        .bind(document).bind(snapshot["value_cent_micros"].as_str().ok_or(AtomicEffectError::ReconciliationRequired)?)
        .bind(parse("inventory_ledger_id")?).bind(parse("operation_log_id")?).bind(parse("audit_id")?).bind(parse("outbox_id")?).bind(parse("posting_id")?).bind(snapshot)
        .fetch_one(&mut **tx).await?;
    if !valid {
        return Err(AtomicEffectError::ReconciliationRequired.into());
    }
    Ok(())
}
