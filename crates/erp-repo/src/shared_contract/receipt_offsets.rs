//! One-statement receipt-specific quantity and cost reconciliation.
use super::{PgPool, TenantId, Uuid};
use erp_domain::trade::{ReceiptOffsetFacts, receipt_offset_holds};
use erp_types::UserId;

#[derive(Debug, serde::Serialize)]
pub struct PurchaseReceiptOffset {
    pub quality_id: Uuid,
    pub status: String,
    pub arrived: i64,
    pub released: i64,
    pub returned_qty: i64,
    pub issued_qty: i64,
    pub payable_cent: i64,
    pub returned_cost: i64,
    pub released_cost_cent: i64,
    pub held_micros: i64,
    pub on_hand_micros: i64,
    pub issue_rows: i64,
    pub over_return_rows: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct ReceiptOffsetActor {
    pub user_id: UserId,
    pub session_id: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiptOffsetError {
    #[error("receipt offset needs a valid identity")]
    InvalidIdentity,
    #[error("receipt offset is unavailable in the current scope")]
    NotFound,
    #[error("receipt offset needs reconciliation of its stored source facts")]
    ReconciliationRequired,
    #[error("receipt offset storage is unavailable")]
    Storage(#[from] sqlx::Error),
}

#[derive(sqlx::FromRow)]
struct OffsetRow {
    values_valid: bool,
    quality_id: Uuid,
    arrived: i64,
    source_arrived: i64,
    released: i64,
    issued_qty: i64,
    unit_cost_cent: i64,
    payable_cent: i64,
    returned_qty: i64,
    returned_cost: i64,
    return_rows: i64,
    invalid_return_rows: i64,
    released_cost_cent: i64,
    release_posting_rows: i64,
    return_posting_cost_cent: i64,
    return_posting_rows: i64,
    invalid_postings: i64,
    held_micros: i64,
    on_hand_micros: i64,
    negative_stock_rows: i64,
    over_return_rows: i64,
}

pub(super) async fn read(
    pool: &PgPool,
    tenant: TenantId,
    quality: Uuid,
    actor: ReceiptOffsetActor,
) -> Result<PurchaseReceiptOffset, ReceiptOffsetError> {
    if quality.is_nil() || actor.session_id.is_nil() || actor.user_id.0.is_nil() {
        return Err(ReceiptOffsetError::InvalidIdentity);
    }
    // READ COMMITTED gives one snapshot to this whole statement, so a release
    // or return cannot advance between separate row/ledger/amount reads.
    let row: Option<OffsetRow> = sqlx::query_as(
        r#"
        select receipt_value_integrity(q.tenant_id,q.quality_id) as values_valid,
               q.quality_id, q.arrived_micros as arrived, p.qty_micros as source_arrived,
               q.released_micros as released, q.issued_micros as issued_qty,
               q.unit_cost_cent, q.payable_cent,
               r.returned_qty, r.returned_cost, r.return_rows, r.invalid_return_rows,
               a.released_cost_cent, a.release_posting_rows,
               a.return_posting_cost_cent, a.return_posting_rows, a.invalid_postings,
               s.held_micros, s.on_hand_micros, s.negative_stock_rows,
               excess.over_return_rows
        from shared_receipt_quality q
        join shared_purchase_receipts p
          on p.tenant_id=q.tenant_id and p.receipt_id=q.receipt_id
         and p.po_id=q.po_id and p.line_no=q.line_no
        cross join lateral (
            select coalesce(sum(x.qty_micros),0)::bigint as returned_qty,
                   coalesce(sum(x.cost_cent),0)::bigint as returned_cost,
                   count(*)::bigint as return_rows,
                   count(*) filter(where x.qty_micros<=0 or x.cost_cent<0
                     or (not exists(select 1 from shared_receipt_value_commands v
                           where v.tenant_id=x.tenant_id and v.return_id=x.return_id
                             and v.quality_id=q.quality_id and v.action='RETURN'
                             and v.qty_micros=x.qty_micros and v.amount_cent=x.cost_cent)
                       and x.cost_cent<>floor((x.qty_micros::numeric*q.unit_cost_cent::numeric+500000)/1000000)))::bigint
                       as invalid_return_rows
            from shared_supplier_returns x
            where x.tenant_id=q.tenant_id and x.receipt_id=q.receipt_id
        ) r
        cross join lateral (
            select coalesce(sum(x.amount_cent) filter(where x.source_type='PURCHASE_RELEASE'),0)::bigint
                       as released_cost_cent,
                   count(*) filter(where x.source_type='PURCHASE_RELEASE')::bigint as release_posting_rows,
                   coalesce(sum(-x.amount_cent::numeric) filter(where x.source_type='PURCHASE_RETURN'),0)::bigint
                       as return_posting_cost_cent,
                   count(*) filter(where x.source_type='PURCHASE_RETURN')::bigint as return_posting_rows,
                   count(*) filter(where (x.source_type='PURCHASE_RELEASE' and x.amount_cent<0)
                     or (x.source_type='PURCHASE_RETURN' and x.amount_cent>0))::bigint as invalid_postings
            from partner_balances x
            where x.tenant_id=q.tenant_id and x.source_id=q.receipt_id::text
              and x.kind='AP' and x.currency='CNY' and x.partner_code=q.supplier_code
              and x.source_type in ('PURCHASE_RELEASE','PURCHASE_RETURN')
        ) a
        cross join lateral (
            select coalesce(sum(x.delta_micros) filter(where x.bucket='HELD'),0)::bigint as held_micros,
                   coalesce(sum(x.delta_micros) filter(where x.bucket='ON_HAND' and x.quality_status='GOOD'),0)::bigint
                       as on_hand_micros,
                   count(*) filter(where x.bucket='ON_HAND' and x.quality_status='GOOD' and x.delta_micros<0)::bigint
                       as negative_stock_rows
            from shared_stock_ledgers x
            where x.tenant_id=q.tenant_id and x.source_line=q.quality_id::text
              and x.warehouse_id=q.warehouse_id and x.location_id=q.location_id
              and x.sku_id=q.sku_id and x.batch_code=q.batch_code
        ) s
        cross join lateral (
            select count(*) filter(where z.running_qty>q.released_micros)::bigint as over_return_rows
            from (
                select sum(x.qty_micros) over(order by x.created_at,x.return_id) as running_qty
                from shared_supplier_returns x
                where x.tenant_id=q.tenant_id and x.receipt_id=q.receipt_id
            ) z
        ) excess
        where q.tenant_id=$1 and q.quality_id=$2
          and exists (
            select 1 from users u
            join tenants t on t.tenant_id=u.tenant_id
            join auth_sessions s on s.tenant_id=u.tenant_id and s.user_id=u.user_id
            where u.tenant_id=q.tenant_id and u.user_id=$3 and u.status='ACTIVE'
              and s.session_id=$4 and s.status='ACTIVE' and s.expires_at>clock_timestamp()
              and s.credentials_epoch=t.credentials_epoch
          )
          and exists (
            select 1 from user_roles ur
            join role_permissions rp on rp.tenant_id=ur.tenant_id and rp.role_id=ur.role_id
            where ur.tenant_id=q.tenant_id and ur.user_id=$3
              and rp.perm_code in ('orders.read','OMS.ORDER.READ')
          )
          and exists (
            select 1 from user_warehouse_access w
            where w.tenant_id=q.tenant_id and w.user_id=$3 and w.warehouse_id=q.warehouse_id
          )
        "#,
    )
    .bind(Uuid::from(tenant))
    .bind(quality)
    .bind(Uuid::from(actor.user_id))
    .bind(actor.session_id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Err(ReceiptOffsetError::NotFound);
    };
    if !row.values_valid
        || row.arrived != row.source_arrived
        || row.unit_cost_cent < 0
        || row.invalid_return_rows != 0
        || row.invalid_postings != 0
        || row.return_posting_cost_cent != row.returned_cost
    {
        return Err(ReceiptOffsetError::ReconciliationRequired);
    }
    let issue_rows = row
        .negative_stock_rows
        .checked_sub(row.return_rows)
        .ok_or(ReceiptOffsetError::ReconciliationRequired)?;
    let status = receipt_offset_holds(ReceiptOffsetFacts {
        arrived: row.arrived,
        released: row.released,
        returned_qty: row.returned_qty,
        issued_qty: row.issued_qty,
        payable_cent: row.payable_cent,
        returned_cost: row.returned_cost,
        released_cost_cent: row.released_cost_cent,
        release_posting_rows: row.release_posting_rows,
        return_rows: row.return_rows,
        return_posting_rows: row.return_posting_rows,
        held_micros: row.held_micros,
        on_hand_micros: row.on_hand_micros,
        issue_rows,
        over_return_rows: row.over_return_rows,
    })
    .map_err(|_| ReceiptOffsetError::ReconciliationRequired)?;
    Ok(PurchaseReceiptOffset {
        quality_id: row.quality_id,
        status: status.into(),
        arrived: row.arrived,
        released: row.released,
        returned_qty: row.returned_qty,
        issued_qty: row.issued_qty,
        payable_cent: row.payable_cent,
        returned_cost: row.returned_cost,
        released_cost_cent: row.released_cost_cent,
        held_micros: row.held_micros,
        on_hand_micros: row.on_hand_micros,
        issue_rows,
        over_return_rows: row.over_return_rows,
    })
}
