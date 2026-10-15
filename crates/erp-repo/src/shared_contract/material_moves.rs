//! Material identity and original-source remaining quantity in the original transaction.

use super::{
    ReceiptValueActor, SharedContractRepo, SharedWrite, insert_receipt, inspection_blocks,
    load_receipt, material_balance, receipt_authority, rework_material, succeeded,
};
use crate::{
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
};
use erp_domain::manufacturing::{
    MaterialBalance, MaterialMove, MaterialMoveKind, apply_material_move, issue_from_reservation,
};
use erp_domain::quantity::ScaledQty;
use erp_domain::receipt::{ReceiptError, admit_retry};
use erp_types::TenantId;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct MaterialMoveInput<'a> {
    pub work_order_id: Uuid,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub original_issue_id: Option<Uuid>,
    pub movement: MaterialMove,
    pub reason: Option<&'a str>,
}

pub struct MaterialMoveResult {
    pub effect: SharedWrite,
    pub move_id: Uuid,
    pub balance: MaterialBalance,
}

/// One stored material action. Display order is not the business identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialHistoryLine {
    pub move_id: Uuid,
    pub kind: String,
    pub qty_micros: i64,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub original_issue_id: Option<Uuid>,
    pub reason: Option<String>,
    pub issued_micros: i64,
    pub returned_micros: i64,
    pub consumed_micros: i64,
    pub shop_micros: i64,
}

#[derive(Clone, Copy)]
pub struct ManufacturingReservationInput {
    pub work_order_id: Uuid,
    pub warehouse_id: Uuid,
    pub sku_id: Uuid,
    pub qty: ScaledQty,
}

#[derive(sqlx::FromRow)]
struct SavedManufacturingReservation {
    warehouse_id: Uuid,
    sku_id: Uuid,
    source_type: Option<String>,
    source_id: Option<String>,
    original_qty_micros: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct OriginalIssue {
    kind: String,
    qty_micros: i64,
    work_order_id: Uuid,
    warehouse_id: Option<Uuid>,
    sku_id: Option<Uuid>,
    batch_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct SavedMovement {
    work_order_id: Uuid,
    warehouse_id: Option<Uuid>,
    sku_id: Option<Uuid>,
    batch_id: Option<Uuid>,
    original_issue_id: Option<Uuid>,
    kind: String,
    qty_micros: i64,
    reason: Option<String>,
    material_issued_micros: Option<i64>,
    material_returned_micros: Option<i64>,
    material_consumed_micros: Option<i64>,
    material_shop_micros: Option<i64>,
}

impl SharedContractRepo<'_> {
    pub async fn post_manufacturing_reservation(
        &self,
        tenant: TenantId,
        request_id: &str,
        payload_hash: &str,
        input: ManufacturingReservationInput,
    ) -> anyhow::Result<Result<SharedWrite, ReceiptError>> {
        let request_id = request_id.trim();
        if request_id.is_empty() {
            return Ok(Err(ReceiptError::EmptyRequest));
        }
        if input.qty.as_micros() <= 0 {
            anyhow::bail!("reservation quantity must be positive");
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(Uuid::from(tenant).to_string())
            .bind(format!("material-command:{request_id}"))
            .execute(&mut *tx)
            .await?;
        let state: String = sqlx::query_scalar(
            "select status from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update",
        )
        .bind(Uuid::from(tenant))
        .bind(input.work_order_id)
        .fetch_one(&mut *tx)
        .await?;
        let owned:bool=sqlx::query_scalar("select exists(select 1 from warehouses where tenant_id=$1 and warehouse_id=$2) and exists(select 1 from skus where tenant_id=$1 and sku_id=$3)")
            .bind(Uuid::from(tenant)).bind(input.warehouse_id).bind(input.sku_id).fetch_one(&mut *tx).await?;
        if !owned {
            anyhow::bail!("reservation warehouse or SKU is outside the enterprise");
        }
        match admit_retry(
            load_receipt(&mut tx, tenant, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(receipt)) => {
                let row:Option<SavedManufacturingReservation>=sqlx::query_as("select warehouse_id,sku_id,source_type,source_id,original_qty_micros from inventory_reservations where tenant_id=$1 and reservation_id=$2")
                    .bind(Uuid::from(tenant)).bind(Uuid::parse_str(&receipt.effect_id)?).fetch_optional(&mut *tx).await?;
                let Some(row) = row else {
                    return Ok(Err(ReceiptError::PayloadConflict));
                };
                if row.warehouse_id != input.warehouse_id
                    || row.sku_id != input.sku_id
                    || row.source_type.as_deref() != Some("MFG_WORK_ORDER")
                    || row.source_id.as_deref() != Some(input.work_order_id.to_string().as_str())
                    || row.original_qty_micros != Some(input.qty.as_micros())
                {
                    return Ok(Err(ReceiptError::PayloadConflict));
                }
                tx.rollback().await?;
                return Ok(Ok(SharedWrite::Replay(receipt)));
            }
            Err(error) => return Ok(Err(error)),
            Ok(None) => {}
        }
        if state != "PLANNED" && state != "RELEASED" {
            anyhow::bail!("work order is not open for reservation");
        }
        let key = InventoryQuantityKey {
            tenant_id: tenant,
            warehouse_id: input.warehouse_id,
            sku_id: input.sku_id,
        };
        InventoryService::new(self.pool)
            .apply_precise_with_tx(
                &mut tx,
                InventoryPreciseDelta {
                    key,
                    on_hand: ScaledQty::ZERO,
                    reserved: input.qty,
                    frozen: ScaledQty::ZERO,
                    in_transit: ScaledQty::ZERO,
                    reason_type: "MFG_RESERVATION".into(),
                    ref_type: Some("MFG_WORK_ORDER".into()),
                    ref_id: Some(input.work_order_id.to_string()),
                    occurred_at: chrono::Utc::now(),
                },
            )
            .await?;
        let id = Uuid::new_v4();
        sqlx::query("insert into inventory_reservations(tenant_id,reservation_id,warehouse_id,sku_id,qty,qty_micros,status,source_type,source_id) values($1,$2,$3,$4,$5,$6,'RESERVED','MFG_WORK_ORDER',$7)")
            .bind(Uuid::from(tenant)).bind(id).bind(input.warehouse_id).bind(input.sku_id).bind(input.qty.as_micros()/erp_domain::quantity::QTY_SCALE).bind(input.qty.as_micros())
            .bind(input.work_order_id.to_string()).execute(&mut *tx).await?;
        let receipt = succeeded(request_id, payload_hash, &id.to_string());
        insert_receipt(&mut tx, tenant, &receipt).await?;
        tx.commit().await?;
        Ok(Ok(SharedWrite::Stored(receipt)))
    }

    pub async fn post_material_move(
        &self,
        tenant: TenantId,
        request_id: &str,
        payload_hash: &str,
        input: MaterialMoveInput<'_>,
    ) -> anyhow::Result<Result<MaterialMoveResult, ReceiptError>> {
        self.post_material_move_inner(tenant, request_id, payload_hash, input, None)
            .await
    }

    pub async fn post_material_move_as_actor(
        &self,
        tenant: TenantId,
        request_id: &str,
        payload_hash: &str,
        input: MaterialMoveInput<'_>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Result<MaterialMoveResult, ReceiptError>> {
        self.post_material_move_inner(tenant, request_id, payload_hash, input, Some(actor))
            .await
    }

    async fn post_material_move_inner(
        &self,
        tenant: TenantId,
        request_id: &str,
        payload_hash: &str,
        mut input: MaterialMoveInput<'_>,
        actor: Option<ReceiptValueActor>,
    ) -> anyhow::Result<Result<MaterialMoveResult, ReceiptError>> {
        let request_id = request_id.trim();
        if request_id.is_empty() {
            return Ok(Err(ReceiptError::EmptyRequest));
        }
        input.reason = input.reason.map(str::trim).filter(|v| !v.is_empty());
        input.movement.supplement_reason_required = input.reason.is_some();
        if input.reason.is_some_and(|v| v.len() > 4096) {
            anyhow::bail!("material reason is too long");
        }
        let needs_source = matches!(
            input.movement.kind,
            MaterialMoveKind::Return | MaterialMoveKind::Consume
        );
        if needs_source != input.original_issue_id.is_some() {
            anyhow::bail!(
                "RETURN/CONSUME require original_issue_id; ISSUE/SUPPLEMENT start a new source"
            );
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(Uuid::from(tenant).to_string())
            .bind(format!("material-order:{}", input.work_order_id))
            .execute(&mut *tx)
            .await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(Uuid::from(tenant).to_string())
            .bind(format!("material-command:{request_id}"))
            .execute(&mut *tx)
            .await?;
        rework_material::lock_owners(&mut tx, tenant, input).await?;
        let status: String = sqlx::query_scalar(
            "select status from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update",
        )
        .bind(Uuid::from(tenant))
        .bind(input.work_order_id)
        .fetch_one(&mut *tx)
        .await?;
        let rework = rework_material::source(&mut tx, tenant, input.original_issue_id).await?;
        if rework.is_some() && actor.is_none() {
            return Err(super::ReceiptValueAuthorizationError::Unauthorized.into());
        }
        let permit = if let Some(actor) = actor {
            if actor.warehouse_id != input.warehouse_id {
                return Err(super::DispositionError::Scope.into());
            }
            Some(
                receipt_authority::lock_permissions(
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
        // Recheck physical ownership before returning even a previous receipt.
        let owned:bool=sqlx::query_scalar("select exists(select 1 from warehouses where tenant_id=$1 and warehouse_id=$2) and exists(select 1 from skus where tenant_id=$1 and sku_id=$3)")
            .bind(Uuid::from(tenant)).bind(input.warehouse_id).bind(input.sku_id).fetch_one(&mut *tx).await?;
        if !owned {
            anyhow::bail!("material warehouse or SKU is outside the enterprise");
        }
        if let Some(batch) = input.batch_id {
            let owned:bool=sqlx::query_scalar("select exists(select 1 from sku_batches where tenant_id=$1 and sku_id=$2 and batch_id=$3)")
                .bind(Uuid::from(tenant)).bind(input.sku_id).bind(batch).fetch_one(&mut *tx).await?;
            if !owned {
                anyhow::bail!("material batch is outside the selected SKU");
            }
        }
        match admit_retry(
            load_receipt(&mut tx, tenant, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(Some(receipt)) => {
                let id = Uuid::parse_str(&receipt.effect_id)?;
                let row:SavedMovement=sqlx::query_as("select work_order_id,warehouse_id,sku_id,batch_id,original_issue_id,kind,qty_micros,reason,material_issued_micros,material_returned_micros,material_consumed_micros,material_shop_micros from mfg_material_moves where tenant_id=$1 and move_id=$2")
                    .bind(Uuid::from(tenant)).bind(id).fetch_one(&mut *tx).await?;
                if row.work_order_id != input.work_order_id
                    || row.warehouse_id != Some(input.warehouse_id)
                    || row.sku_id != Some(input.sku_id)
                    || row.batch_id != input.batch_id
                    || row.original_issue_id != input.original_issue_id
                    || row.kind != input.movement.kind.as_str()
                    || row.qty_micros != input.movement.qty.as_micros()
                    || row.reason.as_deref() != input.reason
                {
                    return Ok(Err(ReceiptError::PayloadConflict));
                }
                let balance =
                    MaterialBalance {
                        issued: ScaledQty::from_micros(row.material_issued_micros.ok_or_else(
                            || anyhow::anyhow!("original material snapshot missing"),
                        )?),
                        returned: ScaledQty::from_micros(row.material_returned_micros.ok_or_else(
                            || anyhow::anyhow!("original material snapshot missing"),
                        )?),
                        consumed: ScaledQty::from_micros(row.material_consumed_micros.ok_or_else(
                            || anyhow::anyhow!("original material snapshot missing"),
                        )?),
                        shop_qty: ScaledQty::from_micros(row.material_shop_micros.ok_or_else(
                            || anyhow::anyhow!("original material snapshot missing"),
                        )?),
                    };
                if rework.is_some() {
                    let proved:bool=sqlx::query_scalar("select exists(select 1 from mfg_rework_material_values where tenant_id=$1 and move_id=$2 and original_issue_id=$3 and kind=$4 and qty_micros=$5)")
                        .bind(tenant.0).bind(id).bind(input.original_issue_id).bind(input.movement.kind.as_str()).bind(input.movement.qty.as_micros()).fetch_one(&mut *tx).await?;
                    if !proved {
                        return Err(super::DispositionError::Reconcile(
                            "rework movement has no original value evidence",
                        )
                        .into());
                    }
                }
                if let Some(permit) = &permit {
                    permit.recheck(&mut tx).await?;
                }
                tx.rollback().await?;
                return Ok(Ok(MaterialMoveResult {
                    effect: SharedWrite::Replay(receipt),
                    move_id: id,
                    balance,
                }));
            }
            Err(error) => return Ok(Err(error)),
            Ok(None) => {}
        }
        let (id, next) =
            append_material_move_core(&mut tx, self, tenant, input, &status, actor).await?;
        let receipt = succeeded(request_id, payload_hash, &id.to_string());
        insert_receipt(&mut tx, tenant, &receipt).await?;
        if let Some(permit) = &permit {
            permit.recheck(&mut tx).await?;
        }
        tx.commit().await?;
        Ok(Ok(MaterialMoveResult {
            effect: SharedWrite::Stored(receipt),
            move_id: id,
            balance: next,
        }))
    }

    /// Reads one page of stored material actions. A read writes no receipt.
    pub async fn read_material_history(
        &self,
        tenant: TenantId,
        work_order_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> anyhow::Result<(i64, Vec<MaterialHistoryLine>)> {
        if !(0..=10_000).contains(&offset) || !(1..=50).contains(&limit) {
            anyhow::bail!("material history page is outside the allowed window");
        }
        let total: i64 = sqlx::query_scalar(
            "select count(*)::bigint from mfg_material_moves where tenant_id = $1 and work_order_id = $2",
        )
        .bind(Uuid::from(tenant))
        .bind(work_order_id)
        .fetch_one(self.pool)
        .await?;
        let rows: Vec<(
            Uuid,
            String,
            i64,
            Uuid,
            Uuid,
            Option<Uuid>,
            Option<Uuid>,
            Option<String>,
            i64,
            i64,
            i64,
            i64,
        )> = sqlx::query_as(
            r#"
            select move_id, kind, qty_micros, warehouse_id, sku_id, batch_id, original_issue_id,
                   reason, material_issued_micros, material_returned_micros,
                   material_consumed_micros, material_shop_micros
            from mfg_material_moves
            where tenant_id = $1 and work_order_id = $2
            order by created_at, move_id
            offset $3 limit $4
            "#,
        )
        .bind(Uuid::from(tenant))
        .bind(work_order_id)
        .bind(offset)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        let lines = rows
            .into_iter()
            .map(|row| MaterialHistoryLine {
                move_id: row.0,
                kind: row.1,
                qty_micros: row.2,
                warehouse_id: row.3,
                sku_id: row.4,
                batch_id: row.5,
                original_issue_id: row.6,
                reason: row.7,
                issued_micros: row.8,
                returned_micros: row.9,
                consumed_micros: row.10,
                shop_micros: row.11,
            })
            .collect();
        Ok((total, lines))
    }
}

/// The caller holds the work-order and command locks and validates physical ownership.
/// This kernel does not commit or write a command receipt; the caller commits all effects.
pub(super) async fn append_material_move_with_tx(
    tx: &mut Transaction<'_, Postgres>,
    repo: &SharedContractRepo<'_>,
    tenant: TenantId,
    input: MaterialMoveInput<'_>,
    status: &str,
) -> anyhow::Result<(Uuid, MaterialBalance)> {
    append_material_move_core(tx, repo, tenant, input, status, None).await
}

pub(super) async fn append_material_move_as_actor_with_tx(
    tx: &mut Transaction<'_, Postgres>,
    repo: &SharedContractRepo<'_>,
    tenant: TenantId,
    input: MaterialMoveInput<'_>,
    status: &str,
    actor: ReceiptValueActor,
) -> anyhow::Result<(Uuid, MaterialBalance)> {
    append_material_move_core(tx, repo, tenant, input, status, Some(actor)).await
}

async fn append_material_move_core(
    tx: &mut Transaction<'_, Postgres>,
    repo: &SharedContractRepo<'_>,
    tenant: TenantId,
    input: MaterialMoveInput<'_>,
    status: &str,
    actor: Option<ReceiptValueActor>,
) -> anyhow::Result<(Uuid, MaterialBalance)> {
    let rework = rework_material::source(tx, tenant, input.original_issue_id).await?;
    if rework.is_some() && actor.is_none() {
        return Err(super::ReceiptValueAuthorizationError::Unauthorized.into());
    }
    if status != "RELEASED"
        && !(status == "PAUSED" && input.movement.kind == MaterialMoveKind::Return)
    {
        anyhow::bail!("work order is not open for this material action");
    }
    if matches!(
        input.movement.kind,
        MaterialMoveKind::Issue | MaterialMoveKind::Supplement
    ) && inspection_blocks(tx, tenant, "WORK_ORDER", input.work_order_id, None).await?
    {
        anyhow::bail!("required inspection is incomplete or failed");
    }
    if let Some(source) = input.original_issue_id {
        let row:Option<OriginalIssue>=sqlx::query_as("select kind,qty_micros,work_order_id,warehouse_id,sku_id,batch_id from mfg_material_moves where tenant_id=$1 and move_id=$2 for update")
            .bind(Uuid::from(tenant)).bind(source).fetch_optional(&mut **tx).await?;
        let Some(original) = row else {
            anyhow::bail!("original material issue is missing");
        };
        if !matches!(original.kind.as_str(), "ISSUE" | "SUPPLEMENT")
            || original.work_order_id != input.work_order_id
            || original.warehouse_id != Some(input.warehouse_id)
            || original.sku_id != Some(input.sku_id)
            || original.batch_id != input.batch_id
        {
            anyhow::bail!("original material issue does not match the selected material");
        }
        let (returned,consumed):(i64,i64)=sqlx::query_as("select coalesce(sum(qty_micros) filter(where kind='RETURN'),0)::bigint,coalesce(sum(qty_micros) filter(where kind='CONSUME'),0)::bigint from mfg_material_moves where tenant_id=$1 and original_issue_id=$2")
            .bind(Uuid::from(tenant)).bind(source).fetch_one(&mut **tx).await?;
        apply_material_move(
            MaterialBalance {
                issued: ScaledQty::from_micros(original.qty_micros),
                returned: ScaledQty::from_micros(returned),
                consumed: ScaledQty::from_micros(consumed),
                shop_qty: ScaledQty::ZERO,
            },
            input.movement,
        )
        .map_err(|e| anyhow::anyhow!("material move rejected: {e:?}"))?;
    }
    let (issued,returned,consumed):(i64,i64,i64)=sqlx::query_as("select coalesce(sum(qty_micros) filter(where kind in ('ISSUE','SUPPLEMENT')),0)::bigint,coalesce(sum(qty_micros) filter(where kind='RETURN'),0)::bigint,coalesce(sum(qty_micros) filter(where kind='CONSUME'),0)::bigint from mfg_material_moves where tenant_id=$1 and work_order_id=$2 and warehouse_id=$3 and sku_id=$4 and batch_id is not distinct from $5")
        .bind(Uuid::from(tenant)).bind(input.work_order_id).bind(input.warehouse_id).bind(input.sku_id).bind(input.batch_id).fetch_one(&mut **tx).await?;
    let next = apply_material_move(
        MaterialBalance {
            issued: ScaledQty::from_micros(issued),
            returned: ScaledQty::from_micros(returned),
            consumed: ScaledQty::from_micros(consumed),
            shop_qty: ScaledQty::ZERO,
        },
        input.movement,
    )
    .map_err(|e| anyhow::anyhow!("material move rejected: {e:?}"))?;
    let id = Uuid::new_v4();
    let price = if matches!(
        input.movement.kind,
        MaterialMoveKind::Issue | MaterialMoveKind::Supplement
    ) {
        Some(super::item_cost::material_price(tx, tenant, input.sku_id).await?)
    } else {
        None
    };
    if input.movement.kind == MaterialMoveKind::Return
        && rework.is_none()
        && let Some(issue) = input.original_issue_id
    {
        super::material_values::prepare_return(tx, tenant, issue).await?;
    }
    apply_stock(tx, repo, tenant, input, id).await?;
    sqlx::query("insert into mfg_material_moves(tenant_id,move_id,work_order_id,kind,qty_micros,reason,warehouse_id,sku_id,batch_id,original_issue_id,material_issued_micros,material_returned_micros,material_consumed_micros,material_shop_micros) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
        .bind(Uuid::from(tenant)).bind(id).bind(input.work_order_id).bind(input.movement.kind.as_str()).bind(input.movement.qty.as_micros()).bind(input.reason)
        .bind(input.warehouse_id).bind(input.sku_id).bind(input.batch_id).bind(input.original_issue_id)
        .bind(next.issued.as_micros()).bind(next.returned.as_micros()).bind(next.consumed.as_micros()).bind(next.shop_qty.as_micros()).execute(&mut **tx).await?;
    if let Some(price) = price {
        super::material_values::freeze(tx, tenant, id, price).await?;
    }
    let is_rework = rework.is_some();
    if let Some(source) = rework {
        rework_material::record(
            tx,
            tenant,
            input,
            id,
            &source,
            actor.ok_or(super::ReceiptValueAuthorizationError::Unauthorized)?,
        )
        .await?;
    } else if let Some(issue) = input.original_issue_id {
        super::material_values::allocate(
            tx,
            tenant,
            issue,
            id,
            input.movement.kind.as_str(),
            input.movement.qty.as_micros(),
            None,
        )
        .await?;
    }
    if !is_rework {
        super::material_values::audit(tx, tenant, id, actor).await?;
    }
    let total = material_balance(tx, tenant, input.work_order_id).await?;
    sqlx::query("update mfg_work_orders set issued_micros=$3,consumed_micros=$4 where tenant_id=$1 and work_order_id=$2")
        .bind(Uuid::from(tenant)).bind(input.work_order_id)
        .bind(total.issued.try_sub(total.returned)?.as_micros()).bind(total.consumed.as_micros()).execute(&mut **tx).await?;
    Ok((id, next))
}

async fn apply_stock(
    tx: &mut Transaction<'_, Postgres>,
    repo: &SharedContractRepo<'_>,
    tenant: TenantId,
    input: MaterialMoveInput<'_>,
    move_id: Uuid,
) -> anyhow::Result<()> {
    let kind = input.movement.kind;
    if kind == MaterialMoveKind::Consume {
        return Ok(());
    }
    let returned_rework = kind == MaterialMoveKind::Return
        && rework_material::source(tx, tenant, input.original_issue_id)
            .await?
            .is_some();
    let key = InventoryQuantityKey {
        tenant_id: tenant,
        warehouse_id: input.warehouse_id,
        sku_id: input.sku_id,
    };
    let rows:Vec<(Uuid,i64)>=sqlx::query_as("select reservation_id,qty_micros from inventory_reservations where tenant_id=$1 and source_type='MFG_WORK_ORDER' and source_id=$2 and warehouse_id=$3 and sku_id=$4 and status='RESERVED' order by reservation_id limit 10001 for update")
        .bind(Uuid::from(tenant)).bind(input.work_order_id.to_string()).bind(input.warehouse_id).bind(input.sku_id).fetch_all(&mut **tx).await?;
    if rows.len() > 10000 {
        anyhow::bail!("source reservation budget exceeded");
    }
    let stock = InventoryService::new(repo.pool)
        .lock_precise_balance(tx, key)
        .await?;
    let quantity = input.movement.qty.as_micros();
    let (delta_hand, delta_reserved) = if kind == MaterialMoveKind::Return {
        (input.movement.qty, ScaledQty::ZERO)
    } else {
        let own = rows.iter().try_fold(0_i64, |sum, (_, qty)| {
            if *qty < 0 {
                anyhow::bail!("negative source reservation");
            }
            sum.checked_add(*qty)
                .ok_or_else(|| anyhow::anyhow!("source reservation quantity overflow"))
        })?;
        let (_, next_reserved) = issue_from_reservation(
            stock.on_hand,
            stock.reserved,
            ScaledQty::from_micros(own),
            input.movement.qty,
        )
        .map_err(|error| anyhow::anyhow!("material reservation rejected: {error:?}"))?;
        let mut spend = quantity.min(own);
        for (id, qty) in rows {
            if spend == 0 {
                break;
            }
            let used = spend.min(qty);
            sqlx::query("update inventory_reservations set qty_micros=qty_micros-$3,status=case when qty_micros=$3 then 'CONSUMED' else status end,updated_at=now() where tenant_id=$1 and reservation_id=$2")
                .bind(Uuid::from(tenant)).bind(id).bind(used).execute(&mut **tx).await?;
            spend -= used;
        }
        (
            ScaledQty::ZERO.try_sub(input.movement.qty)?,
            next_reserved.try_sub(stock.reserved)?,
        )
    };
    let stock_effect = InventoryService::new(repo.pool)
        .apply_precise_effect_with_tx(
            tx,
            InventoryPreciseDelta {
                key,
                on_hand: delta_hand,
                reserved: delta_reserved,
                frozen: if returned_rework {
                    input.movement.qty
                } else {
                    ScaledQty::ZERO
                },
                in_transit: ScaledQty::ZERO,
                reason_type: format!("MFG_{}", kind.as_str()),
                ref_type: Some(
                    if returned_rework {
                        "shared_stock_ledger"
                    } else {
                        "mfg_material_move"
                    }
                    .into(),
                ),
                ref_id: Some(move_id.to_string()),
                occurred_at: chrono::Utc::now(),
            },
        )
        .await?;
    crate::inventory_batch_effects::apply(tx, &stock_effect, input.batch_id).await?;
    if returned_rework {
        sqlx::query("insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,ref_type,ref_id,occurred_at) select tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,'mfg_material_move',ref_id,occurred_at from inventory_ledgers where tenant_id=$1 and ledger_id=$2")
            .bind(tenant.0).bind(stock_effect.ledger_id).execute(&mut **tx).await?;
    }
    // Source detail for original material history; admission and physical stock
    // use inventory_balances/inventory_ledgers only, never mfg_qty_balances.
    sqlx::query("insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type) values($1,$2,$3,$4,$5,$6)")
        .bind(Uuid::from(tenant)).bind(input.warehouse_id).bind(input.sku_id).bind(move_id).bind(delta_hand.as_micros())
        .bind(format!("MFG_{}",kind.as_str())).execute(&mut **tx).await?;
    Ok(())
}
