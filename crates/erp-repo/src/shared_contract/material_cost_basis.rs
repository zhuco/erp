//! The confirmed rework value is consumed once; a return is not an expense.
use erp_types::TenantId;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum MaterialCostError {
    #[error("production material cost requires reconciliation: {0}")]
    Reconcile(&'static str),
    #[error("a confirmed material price is missing or the value is unrepresentable")]
    Cost,
}

pub(super) struct Basis {
    pub cent: i64,
    rework_cent: i64,
    legacy_qty: i64,
    snapshot: Value,
}

pub(super) async fn calculate(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    _item: &str,
) -> anyhow::Result<Basis> {
    // The caller holds the work-order lock, as do material actions and rework allocations.
    let rows:Vec<Value>=sqlx::query_scalar("select jsonb_build_object('move_id',m.move_id,'qty_micros',m.qty_micros,'original_issue_id',m.original_issue_id,'source_disposition_id',c.disposition_id,'value_cent',v.cost_cent,'valid',v.move_id=m.move_id and v.original_issue_id=m.original_issue_id and v.disposition_id=c.disposition_id and v.receipt_id=c.receipt_id and v.kind='CONSUME' and v.qty_micros=m.qty_micros and c.rework_work_order_id=m.work_order_id and c.kind='REWORK' and c.qty_micros>0 and c.cost_cent>=0 and v.cost_cent<=c.cost_cent) from mfg_material_moves m join mfg_receipt_disposition_commands c on c.tenant_id=m.tenant_id and c.material_move_id=m.original_issue_id left join mfg_rework_material_values v on v.tenant_id=m.tenant_id and v.move_id=m.move_id where m.tenant_id=$1 and m.work_order_id=$2 and m.kind='CONSUME' order by m.move_id limit 10001")
        .bind(tenant.0).bind(order).fetch_all(&mut **tx).await?;
    if rows.len() > 10000 {
        return Err(MaterialCostError::Reconcile("cost source budget exceeded").into());
    }
    let mut rework_cent = 0i64;
    for row in &rows {
        if row["valid"] != true {
            return Err(MaterialCostError::Reconcile(
                "rework consumption has no matching original value allocation",
            )
            .into());
        }
        let value = row["value_cent"].as_i64().ok_or(MaterialCostError::Cost)?;
        if value < 0 {
            return Err(MaterialCostError::Cost.into());
        }
        rework_cent = rework_cent
            .checked_add(value)
            .ok_or(MaterialCostError::Cost)?;
    }
    let ordinary:Vec<Value>=sqlx::query_scalar("select to_jsonb(v) from mfg_material_value_allocations v join mfg_material_moves i on i.tenant_id=v.tenant_id and i.move_id=v.issue_id where v.tenant_id=$1 and i.work_order_id=$2 and v.kind='CONSUME' order by v.allocation_id limit 10001")
        .bind(tenant.0).bind(order).fetch_all(&mut **tx).await?;
    if ordinary.len() > 10000 {
        return Err(
            MaterialCostError::Reconcile("ordinary material source budget exceeded").into(),
        );
    }
    let incomplete:bool=sqlx::query_scalar("select exists(select 1 from mfg_material_moves m where m.tenant_id=$1 and m.work_order_id=$2 and m.kind='CONSUME' and not exists(select 1 from mfg_receipt_disposition_commands c where c.tenant_id=m.tenant_id and c.material_move_id=m.original_issue_id) and not exists(select 1 from mfg_material_value_allocations v where v.tenant_id=m.tenant_id and v.allocation_id=m.move_id and v.issue_id=m.original_issue_id and v.kind='CONSUME' and v.qty_micros=m.qty_micros and v.cost_cent is not null)) or exists(select 1 from shared_stock_ledgers l join shared_material_issues i on i.tenant_id=l.tenant_id and i.issue_id::text=l.source_line where i.tenant_id=$1 and i.work_order_id=$2 and l.bucket='WIP' and l.delta_micros<0 and not exists(select 1 from shared_stock_ledgers h where h.tenant_id=l.tenant_id and h.source_line=l.source_line and h.request_id=l.request_id and h.bucket='ON_HAND' and h.delta_micros>0) and not exists(select 1 from mfg_material_value_allocations v where v.tenant_id=i.tenant_id and v.issue_id=i.issue_id and v.request_id=l.request_id and v.kind='CONSUME' and v.qty_micros=-l.delta_micros and v.cost_cent is not null))")
        .bind(tenant.0).bind(order).fetch_one(&mut **tx).await?;
    if incomplete {
        return Err(MaterialCostError::Reconcile(
            "ordinary consumption has no proven original value allocation",
        )
        .into());
    }
    let mut ordinary_cent = 0i64;
    for entry in &ordinary {
        let value = entry["cost_cent"]
            .as_i64()
            .ok_or(MaterialCostError::Reconcile(
                "ordinary consumption remains unvalued",
            ))?;
        ordinary_cent = ordinary_cent
            .checked_add(value)
            .ok_or(MaterialCostError::Cost)?;
    }
    let legacy_qty = 0i64;
    let cent = ordinary_cent
        .checked_add(rework_cent)
        .ok_or(MaterialCostError::Cost)?;
    Ok(Basis {
        cent,
        rework_cent,
        legacy_qty,
        snapshot: json!({"schema":2,"work_order_id":order,"rework_consumptions":rows,"rework_cent":rework_cent,"legacy_quantity_micros":legacy_qty,"ordinary_consumptions":ordinary,"ordinary_cent":ordinary_cent,"legacy_single_item_price":null,"legacy_cent":0,"material_cent":cent}),
    })
}

pub(super) async fn store(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: Uuid,
    basis: &Basis,
) -> anyhow::Result<()> {
    sqlx::query("insert into mfg_material_cost_bases(tenant_id,cost_run_id,material_cent,rework_cent,legacy_qty_micros,snapshot) values($1,$2,$3,$4,$5,$6)")
        .bind(tenant.0).bind(run).bind(basis.cent).bind(basis.rework_cent).bind(basis.legacy_qty).bind(&basis.snapshot).execute(&mut **tx).await?;
    Ok(())
}
