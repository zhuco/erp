//! Purchase arrivals bind cost and command intent to the current admitted line.
use super::{SharedWrite, insert_receipt, load_receipt, succeeded};
use erp_domain::receipt::{CommandReceipt, CommandStatus, ReceiptError, admit_retry};
use erp_types::TenantId;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum PurchaseArrivalError {
    #[error("original purchase arrival requires reconciliation: {0}")]
    Reconciliation(&'static str),
}

fn reconcile(reason: &'static str) -> anyhow::Error {
    PurchaseArrivalError::Reconciliation(reason).into()
}

pub(super) struct Input<'a> {
    pub tenant: TenantId,
    pub request: &'a str,
    pub order: Uuid,
    pub line: i32,
    pub warehouse: Uuid,
    pub location: Uuid,
    pub sku: Uuid,
    pub batch: &'a str,
    pub qty: i64,
    pub unit_cost: i64,
}

impl Input<'_> {
    fn identity(&self) -> anyhow::Result<String> {
        let bytes = serde_json::to_vec(&(
            "purchase-arrival-v2",
            self.tenant.0,
            self.order,
            self.line,
            self.warehouse,
            self.location,
            self.sku,
            self.batch.trim(),
            self.qty,
            self.unit_cost,
        ))?;
        Ok(format!("purchase-arrival-v2:{:x}", Sha256::digest(bytes)))
    }
}

pub(super) async fn lock_order(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    line: i32,
) -> anyhow::Result<()> {
    // Revision/closure and both receipt writers serialize on the same owner.
    // Always acquire the order before its line, including retries.
    for key in [
        format!("purchase-revision:{order}"),
        format!("purchase-receipt:{order}:{line}"),
    ] {
        sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
            .bind(tenant.0.to_string())
            .bind(key)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct Legacy {
    po_id: Uuid,
    line_no: i32,
    warehouse_id: Uuid,
    location_id: Uuid,
    sku_id: Uuid,
    batch_code: Option<String>,
    arrived_micros: i64,
    unit_cost_cent: i64,
    original_quantity: Option<i64>,
    original_line: Option<i32>,
    original_order: Option<Uuid>,
    original_ledger: bool,
}

async fn verify_original(
    tx: &mut Transaction<'_, Postgres>,
    input: &Input<'_>,
    receipt: &CommandReceipt,
) -> anyhow::Result<Result<(), ReceiptError>> {
    if receipt.status != CommandStatus::Succeeded {
        return Err(reconcile(
            "the original command has no proven successful outcome",
        ));
    }
    let quality = Uuid::parse_str(&receipt.effect_id)
        .map_err(|_| reconcile("the original result identity is unknown"))?;
    let original: Option<Legacy> = sqlx::query_as(
        "select q.po_id,q.line_no,q.warehouse_id,q.location_id,q.sku_id,
         case when octet_length(q.batch_code)<=512 then q.batch_code end batch_code,
         q.arrived_micros,q.unit_cost_cent,r.qty_micros original_quantity,r.line_no original_line,r.po_id original_order,
         exists(select 1 from shared_stock_ledgers s where s.tenant_id=q.tenant_id
          and s.request_id=$3 and s.source_line=q.quality_id::text and s.warehouse_id=q.warehouse_id
          and s.location_id=q.location_id and s.sku_id=q.sku_id and s.batch_code=q.batch_code
          and s.bucket='HELD' and s.quality_status='HELD' and s.delta_micros=q.arrived_micros) original_ledger
         from shared_receipt_quality q left join shared_purchase_receipts r
          on r.tenant_id=q.tenant_id and r.receipt_id=q.receipt_id
         where q.tenant_id=$1 and q.quality_id=$2 for share of q",
    ).bind(input.tenant.0).bind(quality).bind(input.request).fetch_optional(&mut **tx).await?;
    let original = original.ok_or_else(|| reconcile("the original source is missing"))?;
    if !original.original_ledger
        || original.original_quantity != Some(original.arrived_micros)
        || original.original_line != Some(original.line_no)
        || original.original_order != Some(original.po_id)
        || original.batch_code.is_none()
    {
        return Err(reconcile(
            "the original receipt and held-stock evidence disagree",
        ));
    }
    if original.po_id != input.order
        || original.line_no != input.line
        || original.warehouse_id != input.warehouse
        || original.location_id != input.location
        || original.sku_id != input.sku
        || original.batch_code.as_deref() != Some(input.batch.trim())
        || original.arrived_micros != input.qty
        || original.unit_cost_cent != input.unit_cost
    {
        return Ok(Err(ReceiptError::PayloadConflict));
    }
    Ok(Ok(()))
}

pub(super) async fn commit(
    pool: &PgPool,
    input: Input<'_>,
) -> anyhow::Result<Result<(SharedWrite, Uuid), ReceiptError>> {
    if input.batch.trim().is_empty()
        || input.batch.trim().len() > 512
        || input.qty <= 0
        || input.unit_cost < 0
        || input.line <= 0
    {
        anyhow::bail!(
            "arrival needs a bounded batch, positive quantity and a known nonnegative cost"
        );
    }
    let hash = input.identity()?;
    let mut tx = pool.begin().await?;
    lock_order(&mut tx, input.tenant, input.order, input.line).await?;
    let previous = load_receipt(&mut tx, input.tenant, input.request).await?;
    if let Some(original) = previous
        .as_ref()
        .filter(|r| !r.payload_hash.starts_with("purchase-arrival-v2:"))
    {
        if let Err(error) = verify_original(&mut tx, &input, original).await? {
            tx.rollback().await?;
            return Ok(Err(error));
        }
        let id = Uuid::parse_str(&original.effect_id)?;
        tx.rollback().await?;
        return Ok(Ok((SharedWrite::Replay(original.clone()), id)));
    }
    match admit_retry(previous.as_ref(), input.request, &hash) {
        Ok(Some(original)) => {
            // A digest proves intent, not a committed result. Every replay needs
            // its original quantity row and actual held-stock effect.
            if let Err(error) = verify_original(&mut tx, &input, &original).await? {
                tx.rollback().await?;
                return Ok(Err(error));
            }
            let id = Uuid::parse_str(&original.effect_id)?;
            tx.rollback().await?;
            return Ok(Ok((SharedWrite::Replay(original), id)));
        }
        Err(error) => {
            tx.rollback().await?;
            return Ok(Err(error));
        }
        Ok(None) => {}
    }
    // Filtering OPEN before selecting the latest version would resurrect an older order.
    let source: Option<(i64, String, i64)> = sqlx::query_as(
        "select l.qty_micros,r.supplier_code,l.price_cent from shared_purchase_revisions r
         join shared_purchase_lines l on l.tenant_id=r.tenant_id and l.revision_id=r.revision_id
         where r.tenant_id=$1 and r.po_id=$2 and l.line_no=$3 and r.status='OPEN'
          and r.version_no=(select max(current.version_no) from shared_purchase_revisions current where current.tenant_id=$1 and current.po_id=$2)",
    ).bind(input.tenant.0).bind(input.order).bind(input.line).fetch_optional(&mut *tx).await?;
    let (ordered, supplier, unit_cost) =
        source.ok_or_else(|| anyhow::anyhow!("CONFLICT: current purchase line is not open"))?;
    if unit_cost < 0 || unit_cost != input.unit_cost {
        anyhow::bail!("CONFLICT: arrival unit cost differs from the current purchase line");
    }
    let received: i64 = sqlx::query_scalar(
        "select coalesce(sum(qty_micros),0)::bigint from shared_purchase_receipts where tenant_id=$1 and po_id=$2 and line_no=$3",
    ).bind(input.tenant.0).bind(input.order).bind(input.line).fetch_one(&mut *tx).await?;
    let total = received
        .checked_add(input.qty)
        .ok_or_else(|| anyhow::anyhow!("CONFLICT: arrival quantity exceeds its exact range"))?;
    erp_domain::trade::purchase_remaining(ordered, total)
        .map_err(|_| anyhow::anyhow!("CONFLICT: purchase receipt exceeds the source line"))?;
    let quality = Uuid::new_v4();
    let receipt = Uuid::new_v4();
    sqlx::query("insert into shared_purchase_receipts(tenant_id,receipt_id,po_id,line_no,qty_micros) values($1,$2,$3,$4,$5)")
        .bind(input.tenant.0).bind(receipt).bind(input.order).bind(input.line).bind(input.qty).execute(&mut *tx).await?;
    sqlx::query("insert into shared_receipt_quality(tenant_id,quality_id,receipt_id,po_id,line_no,warehouse_id,location_id,sku_id,supplier_code,batch_code,arrived_micros,unit_cost_cent) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
        .bind(input.tenant.0).bind(quality).bind(receipt).bind(input.order).bind(input.line)
        .bind(input.warehouse).bind(input.location).bind(input.sku).bind(supplier).bind(input.batch.trim())
        .bind(input.qty).bind(unit_cost).execute(&mut *tx).await?;
    sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,$6,'HELD','HELD',$7,$8,$9)")
        .bind(input.tenant.0).bind(Uuid::new_v4()).bind(input.warehouse).bind(input.location).bind(input.sku)
        .bind(input.batch.trim()).bind(input.qty).bind(quality.to_string()).bind(input.request).execute(&mut *tx).await?;
    let committed = succeeded(input.request, &hash, &quality.to_string());
    insert_receipt(&mut tx, input.tenant, &committed).await?;
    tx.commit().await?;
    Ok(Ok((SharedWrite::Stored(committed), quality)))
}
