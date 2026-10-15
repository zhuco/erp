//! Receipt-local actual release lots; returns and issues share immutable consumption.
use super::{
    ReceiptValueActor, ReceiptValueAuthorizationError, SharedWrite, insert_receipt, load_receipt,
    succeeded,
};
use erp_domain::receipt::{CommandReceipt, CommandStatus, ReceiptError, admit_retry};
use erp_domain::trade::{
    CostReturnInput, ReceiptOffsetFacts, cumulative_return_cost, receipt_offset_holds,
    supplier_return,
};
use erp_types::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Release,
    Return,
    Issue,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Release => "RELEASE",
            Self::Return => "RETURN",
            Self::Issue => "ISSUE",
        }
    }
}

pub(super) struct Input<'a> {
    pub tenant: TenantId,
    pub request: &'a str,
    pub payload: &'a str,
    pub quality: Uuid,
    pub qty: i64,
    pub action: Action,
    pub actor: ReceiptValueActor,
}
impl Input<'_> {
    fn identity(&self) -> anyhow::Result<String> {
        let bytes = serde_json::to_vec(&(
            "receipt-value-v1",
            self.tenant.0,
            self.quality,
            self.action.name(),
            self.qty,
            self.payload,
        ))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

#[derive(sqlx::FromRow)]
struct Source {
    receipt_id: Uuid,
    warehouse_id: Uuid,
    location_id: Uuid,
    sku_id: Uuid,
    supplier_code: String,
    batch_code: String,
    arrived: i64,
    source_arrived: i64,
    released: i64,
    issued: i64,
    unit_cost: i64,
    payable: i64,
}
#[derive(sqlx::FromRow)]
struct Facts {
    returned_qty: i64,
    returned_cost: i64,
    return_rows: i64,
    release_cost: i64,
    release_rows: i64,
    return_posted: i64,
    return_posting_rows: i64,
    invalid_rows: i64,
    held: i64,
    on_hand: i64,
    negative_rows: i64,
}
#[derive(Debug, Serialize, Deserialize)]
struct Legacy {
    released: i64,
    returned_qty: i64,
    release_cost: i64,
    returned_cost: i64,
    fingerprint: Value,
}
#[derive(sqlx::FromRow)]
struct Rollup {
    released: i64,
    returned: i64,
    issued: i64,
    release_cost: i64,
    return_cost: i64,
    issue_cost: i64,
}
#[derive(sqlx::FromRow)]
struct Lot {
    lot_id: Uuid,
    qty: i64,
    cost: i64,
    used_qty: i64,
    used_cost: i64,
}
struct Allocation {
    lot_id: Uuid,
    qty: i64,
    cost: i64,
}

fn reconcile() -> anyhow::Error {
    anyhow::anyhow!("CONFLICT: receipt original quantity or cost requires reconciliation")
}
fn add(a: i64, b: i64) -> anyhow::Result<i64> {
    a.checked_add(b).ok_or_else(reconcile)
}
fn sub(a: i64, b: i64) -> anyhow::Result<i64> {
    a.checked_sub(b).filter(|v| *v >= 0).ok_or_else(reconcile)
}

pub(super) async fn post(
    pool: &PgPool,
    input: Input<'_>,
) -> anyhow::Result<Result<(SharedWrite, i64), ReceiptError>> {
    let request = input.request.trim();
    input.actor.validate(input.tenant)?;
    if let Err(error) = admit_retry(None, request, "") {
        return Ok(Err(error));
    }
    if input.qty <= 0 || input.quality.is_nil() {
        anyhow::bail!("CONFLICT: receipt action needs a positive quantity and source");
    }
    let hash = input.identity()?;
    let mut tx = pool.begin().await?;
    // Request identity is shared with command_receipts. Lock before the source,
    // so cross-quality retries cannot produce two effects or deadlock each other.
    for key in [
        format!("receipt-value-command:{request}"),
        format!("purchase-release:{}", input.quality),
    ] {
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(input.tenant.0.to_string())
            .bind(key)
            .execute(&mut *tx)
            .await?;
    }
    let permit = super::receipt_authority::lock(&mut tx, input.tenant, input.actor).await?;
    let accessible:Option<Uuid>=sqlx::query_scalar("select warehouse_id from shared_receipt_quality where tenant_id=$1 and quality_id=$2 and warehouse_id=$3 for share")
        .bind(input.tenant.0).bind(input.quality).bind(input.actor.warehouse_id).fetch_optional(&mut *tx).await?;
    if accessible.is_none() {
        return Err(ReceiptValueAuthorizationError::Unavailable.into());
    }
    let existing = load_receipt(&mut tx, input.tenant, request).await?;
    match admit_retry(existing.as_ref(), request, &hash) {
        Ok(Some(existing)) => {
            let saved: Option<(Uuid, String, i64, i64)> = sqlx::query_as(
                "select quality_id,action,qty_micros,result from shared_receipt_value_commands where tenant_id=$1 and request_id=$2",
            )
            .bind(input.tenant.0)
            .bind(request)
            .fetch_optional(&mut *tx)
            .await?;
            let Some((quality, action, qty, result)) = saved else {
                permit.recheck(&mut tx).await?;
                return legacy_replay(&mut tx, &input, existing, &permit).await;
            };
            if quality != input.quality || action != input.action.name() || qty != input.qty {
                tx.rollback().await?;
                return Ok(Err(ReceiptError::PayloadConflict));
            }
            let valid: bool = sqlx::query_scalar("select receipt_value_integrity($1,$2)")
                .bind(input.tenant.0)
                .bind(quality)
                .fetch_one(&mut *tx)
                .await?;
            if !valid || existing.effect_id != result.to_string() {
                return Err(reconcile());
            }
            permit.recheck(&mut tx).await?;
            tx.rollback().await?;
            return Ok(Ok((SharedWrite::Replay(existing), result)));
        }
        Err(error) => {
            if existing
                .as_ref()
                .is_some_and(|v| v.payload_hash == input.payload)
            {
                let tracked: bool = sqlx::query_scalar(
                    "select exists(select 1 from shared_receipt_value_commands where tenant_id=$1 and request_id=$2)",
                ).bind(input.tenant.0).bind(request).fetch_one(&mut *tx).await?;
                if !tracked {
                    let Some(existing) = existing else {
                        return Err(reconcile());
                    };
                    permit.recheck(&mut tx).await?;
                    return legacy_replay(&mut tx, &input, existing, &permit).await;
                }
            }
            tx.rollback().await?;
            return Ok(Err(error));
        }
        Ok(None) => {}
    }
    let source: Option<Source> = sqlx::query_as(
        "select q.receipt_id,q.warehouse_id,q.location_id,q.sku_id,q.supplier_code,q.batch_code,
        q.arrived_micros as arrived,p.qty_micros as source_arrived,q.released_micros as released,
        q.issued_micros as issued,q.unit_cost_cent as unit_cost,q.payable_cent as payable
        from shared_receipt_quality q join shared_purchase_receipts p
          on p.tenant_id=q.tenant_id and p.receipt_id=q.receipt_id and p.po_id=q.po_id and p.line_no=q.line_no
        where q.tenant_id=$1 and q.quality_id=$2 and q.warehouse_id=$3 for update of q",
    )
    .bind(input.tenant.0)
    .bind(input.quality)
    .bind(input.actor.warehouse_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(source) = source else {
        return Err(reconcile());
    };
    let facts = source_facts(&mut tx, &input, &source).await?;
    if source.arrived != source.source_arrived
        || source.unit_cost < 0
        || facts.invalid_rows != 0
        || facts.returned_cost != facts.return_posted
    {
        return Err(reconcile());
    }
    receipt_offset_holds(ReceiptOffsetFacts {
        arrived: source.arrived,
        released: source.released,
        returned_qty: facts.returned_qty,
        issued_qty: source.issued,
        payable_cent: source.payable,
        returned_cost: facts.returned_cost,
        released_cost_cent: facts.release_cost,
        release_posting_rows: facts.release_rows,
        return_rows: facts.return_rows,
        return_posting_rows: facts.return_posting_rows,
        held_micros: facts.held,
        on_hand_micros: facts.on_hand,
        issue_rows: sub(facts.negative_rows, facts.return_rows)?,
        over_return_rows: 0,
    })
    .map_err(|_| reconcile())?;
    let remaining = sub(sub(source.released, facts.returned_qty)?, source.issued)?;
    match input.action {
        Action::Release => {
            if add(source.released, input.qty)? > source.arrived {
                anyhow::bail!("CONFLICT: release exceeds the arrival");
            }
        }
        Action::Return | Action::Issue if input.qty > remaining => {
            anyhow::bail!("CONFLICT: receipt action exceeds still released stock");
        }
        Action::Return | Action::Issue => {}
    }
    retain_legacy(&mut tx, &input, &source, &facts).await?;
    let allocations = match input.action {
        Action::Release => Vec::new(),
        Action::Return | Action::Issue => allocate(&mut tx, &input).await?,
    };
    let amount = match input.action {
        Action::Release => {
            supplier_return(input.qty, 0, input.qty, source.unit_cost).map_err(|_| reconcile())?
        }
        Action::Return | Action::Issue => allocations
            .iter()
            .try_fold(0_i64, |sum, x| add(sum, x.cost))?,
    };
    let released = if matches!(input.action, Action::Release) {
        add(source.released, input.qty)?
    } else {
        source.released
    };
    let issued = if matches!(input.action, Action::Issue) {
        add(source.issued, input.qty)?
    } else {
        source.issued
    };
    let payable = match input.action {
        Action::Release => add(source.payable, amount)?,
        Action::Return => sub(source.payable, amount)?,
        Action::Issue => source.payable,
    };
    let result = match input.action {
        Action::Release => sub(source.arrived, released)?,
        Action::Return => amount,
        Action::Issue => sub(remaining, input.qty)?,
    };
    sqlx::query("update shared_receipt_quality set released_micros=$3,issued_micros=$4,payable_cent=$5 where tenant_id=$1 and quality_id=$2")
        .bind(input.tenant.0).bind(input.quality).bind(released).bind(issued).bind(payable)
        .execute(&mut *tx).await?;
    let stock_id = Uuid::new_v4();
    stock(&mut tx, &input, &source, stock_id, false).await?;
    let held_id = if matches!(input.action, Action::Release) {
        let id = Uuid::new_v4();
        stock(&mut tx, &input, &source, id, true).await?;
        Some(id)
    } else {
        None
    };
    let balance_id = if !matches!(input.action, Action::Issue) {
        let id = Uuid::new_v4();
        sqlx::query("insert into partner_balances(tenant_id,balance_id,kind,partner_code,currency,source_type,source_id,amount_cent) values($1,$2,'AP',$3,'CNY',$4,$5,$6)")
            .bind(input.tenant.0).bind(id).bind(&source.supplier_code)
            .bind(if matches!(input.action,Action::Release) {"PURCHASE_RELEASE"} else {"PURCHASE_RETURN"})
            .bind(source.receipt_id.to_string())
            .bind(if matches!(input.action,Action::Release) {amount} else {-amount})
            .execute(&mut *tx).await?;
        Some(id)
    } else {
        None
    };
    let return_id = if matches!(input.action, Action::Return) {
        let id = Uuid::new_v4();
        sqlx::query("insert into shared_supplier_returns(tenant_id,return_id,receipt_id,qty_micros,cost_cent) values($1,$2,$3,$4,$5)")
            .bind(input.tenant.0).bind(id).bind(source.receipt_id).bind(input.qty).bind(amount)
            .execute(&mut *tx).await?;
        Some(id)
    } else {
        None
    };
    if matches!(input.action, Action::Release) {
        sqlx::query("insert into shared_receipt_cost_lots(tenant_id,quality_id,lot_id,kind,request_id,qty_micros,cost_cent) values($1,$2,$3,'RELEASE',$4,$5,$6)")
            .bind(input.tenant.0).bind(input.quality).bind(Uuid::new_v4())
            .bind(request).bind(input.qty).bind(amount).execute(&mut *tx).await?;
    }
    for allocation in allocations {
        sqlx::query("insert into shared_receipt_cost_allocations(tenant_id,quality_id,lot_id,request_id,action,qty_micros,cost_cent) values($1,$2,$3,$4,$5,$6,$7)")
            .bind(input.tenant.0).bind(input.quality).bind(allocation.lot_id)
            .bind(request).bind(input.action.name()).bind(allocation.qty).bind(allocation.cost)
            .execute(&mut *tx).await?;
    }
    let receipt = succeeded(request, &hash, &result.to_string());
    insert_receipt(&mut tx, input.tenant, &receipt).await?;
    sqlx::query("insert into shared_receipt_value_commands(tenant_id,request_id,quality_id,action,qty_micros,amount_cent,result,payload_hash,stock_id,held_id,balance_id,return_id) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)")
        .bind(input.tenant.0).bind(request).bind(input.quality).bind(input.action.name())
        .bind(input.qty).bind(amount).bind(result).bind(&hash).bind(stock_id)
        .bind(held_id).bind(balance_id).bind(return_id).execute(&mut *tx).await?;
    permit.retain(&mut tx, request).await?;
    permit.recheck(&mut tx).await?;
    if let Err(error) = tx.commit().await {
        if error
            .as_database_error()
            .and_then(|db| db.code())
            .as_deref()
            == Some("PR401")
        {
            return Err(ReceiptValueAuthorizationError::Unauthorized.into());
        }
        return Err(error.into());
    }
    Ok(Ok((SharedWrite::Stored(receipt), result)))
}

async fn legacy_replay(
    tx: &mut Transaction<'_, Postgres>,
    input: &Input<'_>,
    existing: CommandReceipt,
    permit: &super::receipt_authority::Permit,
) -> anyhow::Result<Result<(SharedWrite, i64), ReceiptError>> {
    // An old request-id hash proves no intent. Require one actual source action;
    // timestamp pairing is admitted only when both original rows are unique.
    if existing.status != CommandStatus::Succeeded {
        return Err(reconcile());
    }
    let result = existing.effect_id.parse::<i64>().map_err(|_| reconcile())?;
    if result < 0 {
        return Err(reconcile());
    }
    let valid: bool = sqlx::query_scalar(
        "select exists(select 1 from shared_receipt_quality q
        join shared_stock_ledgers s on s.tenant_id=q.tenant_id and s.source_line=q.quality_id::text
          and s.request_id=$3 and s.bucket='ON_HAND' and s.quality_status='GOOD'
          and s.warehouse_id=q.warehouse_id and s.location_id=q.location_id and s.sku_id=q.sku_id and s.batch_code=q.batch_code
        where q.tenant_id=$1 and q.quality_id=$2
          and s.delta_micros=case when $4='RELEASE' then $5::bigint else -$5::bigint end
          and (select count(*) from shared_stock_ledgers l where l.tenant_id=$1 and l.request_id=$3)=case when $4='RELEASE' then 2 else 1 end
          and (select count(*) from shared_stock_ledgers l where l.tenant_id=$1 and l.request_id=$3 and l.bucket='ON_HAND')=1
          and ($4<>'RELEASE' or (select count(*) from shared_stock_ledgers h
            where h.tenant_id=$1 and h.request_id=$3 and h.source_line=q.quality_id::text
              and h.warehouse_id=q.warehouse_id and h.location_id=q.location_id and h.sku_id=q.sku_id and h.batch_code=q.batch_code
              and h.bucket='HELD' and h.quality_status='HELD' and h.delta_micros=-$5::bigint and h.created_at=s.created_at)=1)
          and (select count(*) from shared_supplier_returns r where r.tenant_id=$1 and r.receipt_id=q.receipt_id
            and r.created_at=s.created_at and r.qty_micros=$5 and r.cost_cent=$6)=case when $4='RETURN' then 1 else 0 end
          and (select count(*) from partner_balances p where p.tenant_id=$1 and p.source_id=q.receipt_id::text
            and p.partner_code=q.supplier_code and p.kind='AP' and p.currency='CNY' and p.created_at=s.created_at
            and (($4='RELEASE' and p.source_type='PURCHASE_RELEASE' and p.amount_cent>=0)
              or ($4 in ('RETURN','ISSUE') and p.source_type='PURCHASE_RETURN'
                and ($4='ISSUE' or p.amount_cent=-$6::bigint))))=case when $4='ISSUE' then 0 else 1 end)",
    )
    .bind(input.tenant.0).bind(input.quality).bind(input.request.trim())
    .bind(input.action.name()).bind(input.qty).bind(result).fetch_one(&mut **tx).await?;
    if !valid {
        return Ok(Err(ReceiptError::PayloadConflict));
    }
    permit.recheck(tx).await?;
    Ok(Ok((SharedWrite::Replay(existing), result)))
}

async fn source_facts(
    tx: &mut Transaction<'_, Postgres>,
    input: &Input<'_>,
    source: &Source,
) -> anyhow::Result<Facts> {
    Ok(sqlx::query_as(
        "select r.returned_qty,r.returned_cost,r.return_rows,a.release_cost,a.release_rows,
        a.return_posted,a.return_posting_rows,r.invalid+a.invalid as invalid_rows,
        s.held,s.on_hand,s.negative_rows from
        (select coalesce(sum(qty_micros),0)::bigint returned_qty,coalesce(sum(cost_cent),0)::bigint returned_cost,
          count(*)::bigint return_rows,count(*) filter(where qty_micros<=0 or cost_cent<0
            or (not exists(select 1 from shared_receipt_value_commands c where c.tenant_id=x.tenant_id
              and c.return_id=x.return_id and c.quality_id=$4 and c.action='RETURN' and c.qty_micros=x.qty_micros and c.amount_cent=x.cost_cent)
              and cost_cent<>floor((qty_micros::numeric*$9::numeric+500000)/1000000)))::bigint invalid
          from shared_supplier_returns x where tenant_id=$1 and receipt_id=$2) r cross join
        (select coalesce(sum(amount_cent) filter(where source_type='PURCHASE_RELEASE'),0)::bigint release_cost,
          count(*) filter(where source_type='PURCHASE_RELEASE')::bigint release_rows,
          coalesce(sum(-amount_cent::numeric) filter(where source_type='PURCHASE_RETURN'),0)::bigint return_posted,
          count(*) filter(where source_type='PURCHASE_RETURN')::bigint return_posting_rows,
          count(*) filter(where (source_type='PURCHASE_RELEASE' and amount_cent<0) or (source_type='PURCHASE_RETURN' and amount_cent>0))::bigint invalid
          from partner_balances where tenant_id=$1 and source_id=$2::text and kind='AP' and currency='CNY'
          and partner_code=$3 and source_type in ('PURCHASE_RELEASE','PURCHASE_RETURN')) a cross join
        (select coalesce(sum(delta_micros) filter(where bucket='HELD'),0)::bigint held,
          coalesce(sum(delta_micros) filter(where bucket='ON_HAND' and quality_status='GOOD'),0)::bigint on_hand,
          count(*) filter(where bucket='ON_HAND' and quality_status='GOOD' and delta_micros<0)::bigint negative_rows
          from shared_stock_ledgers where tenant_id=$1 and source_line=$4::text and warehouse_id=$5
          and location_id=$6 and sku_id=$7 and batch_code=$8) s",
    )
    .bind(input.tenant.0).bind(source.receipt_id).bind(&source.supplier_code).bind(input.quality)
    .bind(source.warehouse_id).bind(source.location_id).bind(source.sku_id).bind(&source.batch_code)
    .bind(source.unit_cost)
    .fetch_one(&mut **tx).await?)
}

async fn retain_legacy(
    tx: &mut Transaction<'_, Postgres>,
    input: &Input<'_>,
    source: &Source,
    facts: &Facts,
) -> anyhow::Result<()> {
    let saved: Option<Value> = sqlx::query_scalar("select legacy_facts from shared_receipt_cost_lots where tenant_id=$1 and quality_id=$2 and kind='LEGACY_BALANCE'")
        .bind(input.tenant.0).bind(input.quality).fetch_optional(&mut **tx).await?;
    let roll: Rollup = sqlx::query_as(
        "select coalesce(sum(qty_micros) filter(where action='RELEASE'),0)::bigint released,
        coalesce(sum(qty_micros) filter(where action='RETURN'),0)::bigint returned,
        coalesce(sum(qty_micros) filter(where action='ISSUE'),0)::bigint issued,
        coalesce(sum(amount_cent) filter(where action='RELEASE'),0)::bigint release_cost,
        coalesce(sum(amount_cent) filter(where action='RETURN'),0)::bigint return_cost,
        coalesce(sum(amount_cent) filter(where action='ISSUE'),0)::bigint issue_cost
        from shared_receipt_value_commands where tenant_id=$1 and quality_id=$2",
    )
    .bind(input.tenant.0)
    .bind(input.quality)
    .fetch_one(&mut **tx)
    .await?;
    let fingerprint: Value = sqlx::query_scalar("select receipt_value_legacy_facts($1,$2)")
        .bind(input.tenant.0)
        .bind(input.quality)
        .fetch_one(&mut **tx)
        .await?;
    let legacy = if let Some(saved) = saved {
        let legacy: Legacy = serde_json::from_value(saved)?;
        let valid: bool = sqlx::query_scalar("select receipt_value_integrity($1,$2)")
            .bind(input.tenant.0)
            .bind(input.quality)
            .fetch_one(&mut **tx)
            .await?;
        if !valid || legacy.fingerprint != fingerprint {
            return Err(reconcile());
        }
        legacy
    } else {
        // No historical issue amount exists in the old path. Refuse to invent it.
        if source.issued != 0 || roll.released != 0 || roll.returned != 0 || roll.issued != 0 {
            return Err(reconcile());
        }
        let qty = sub(source.released, facts.returned_qty)?;
        let cost = sub(facts.release_cost, facts.returned_cost)?;
        if qty == 0 && cost != 0 {
            return Err(reconcile());
        }
        let legacy = Legacy {
            released: source.released,
            returned_qty: facts.returned_qty,
            release_cost: facts.release_cost,
            returned_cost: facts.returned_cost,
            fingerprint,
        };
        sqlx::query("insert into shared_receipt_cost_lots(tenant_id,quality_id,lot_id,kind,qty_micros,cost_cent,legacy_facts) values($1,$2,$3,'LEGACY_BALANCE',$4,$5,$6)")
            .bind(input.tenant.0).bind(input.quality).bind(Uuid::new_v4()).bind(qty).bind(cost)
            .bind(serde_json::to_value(&legacy)?).execute(&mut **tx).await?;
        legacy
    };
    if add(legacy.released, roll.released)? != source.released
        || add(legacy.returned_qty, roll.returned)? != facts.returned_qty
        || roll.issued != source.issued
        || add(legacy.release_cost, roll.release_cost)? != facts.release_cost
        || add(legacy.returned_cost, roll.return_cost)? != facts.returned_cost
    {
        return Err(reconcile());
    }
    let _remaining_value = sub(
        facts.release_cost,
        add(facts.returned_cost, roll.issue_cost)?,
    )?;
    Ok(())
}

async fn allocate(
    tx: &mut Transaction<'_, Postgres>,
    input: &Input<'_>,
) -> anyhow::Result<Vec<Allocation>> {
    let lots: Vec<Lot> = sqlx::query_as(
        "select l.lot_id,l.qty_micros qty,l.cost_cent cost,a.used_qty,a.used_cost
        from shared_receipt_cost_lots l cross join lateral
        (select coalesce(sum(qty_micros),0)::bigint used_qty,coalesce(sum(cost_cent),0)::bigint used_cost
        from shared_receipt_cost_allocations where tenant_id=l.tenant_id and quality_id=l.quality_id and lot_id=l.lot_id) a
        where l.tenant_id=$1 and l.quality_id=$2 and l.qty_micros>a.used_qty order by l.ordinal",
    ).bind(input.tenant.0).bind(input.quality).fetch_all(&mut **tx).await?;
    let mut left = input.qty;
    let mut result = Vec::new();
    for lot in lots {
        let qty = left.min(sub(lot.qty, lot.used_qty)?);
        let cost = cumulative_return_cost(CostReturnInput {
            original_qty: lot.qty,
            original_cent: lot.cost,
            returned_qty: lot.used_qty,
            returned_cent: lot.used_cost,
            new_qty: qty,
        })
        .map_err(|_| reconcile())?;
        result.push(Allocation {
            lot_id: lot.lot_id,
            qty,
            cost,
        });
        left = sub(left, qty)?;
        if left == 0 {
            break;
        }
    }
    if left != 0 {
        return Err(reconcile());
    }
    Ok(result)
}

async fn stock(
    tx: &mut Transaction<'_, Postgres>,
    input: &Input<'_>,
    source: &Source,
    id: Uuid,
    held: bool,
) -> anyhow::Result<()> {
    let delta = if !held && matches!(input.action, Action::Release) {
        input.qty
    } else {
        -input.qty
    };
    sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
        .bind(input.tenant.0).bind(id).bind(source.warehouse_id).bind(source.location_id)
        .bind(source.sku_id).bind(&source.batch_code).bind(if held {"HELD"} else {"GOOD"})
        .bind(if held {"HELD"} else {"ON_HAND"}).bind(delta).bind(input.quality.to_string()).bind(input.request.trim())
        .execute(&mut **tx).await?;
    Ok(())
}
