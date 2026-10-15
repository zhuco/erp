use super::WorkOrderCostInput;
use erp_domain::manufacturing::cost_allocation::{Allocation, POLICY};
use erp_types::TenantId;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn store(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: Uuid,
    input: WorkOrderCostInput<'_>,
    allocation: Allocation,
) -> anyhow::Result<()> {
    let snapshot = json!({"policy":POLICY,"finished_qty_micros":input.completed_qty.as_micros(),"wip_qty_micros":input.unfinished_qty.as_micros(),"rejected_qty_micros":input.rejected_qty.as_micros(),"total_cent":allocation.total_cent,"finished_cent":allocation.finished_cent,"wip_cent":allocation.wip_cent,"rejected_cent":allocation.rejected_cent,"unit_cent":allocation.unit_cent,"unit_cent_micros":allocation.unit_cent_micros.map(|v|v.to_string()),"estimated":allocation.estimated,"overhead_cent":input.overhead_cent});
    sqlx::query("insert into mfg_cost_allocations(tenant_id,cost_run_id,policy,total_cent,finished_cent,wip_cent,rejected_cent,finished_qty_micros,wip_qty_micros,rejected_qty_micros,unit_cent,unit_cent_micros,snapshot) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12::text::numeric,$13)")
        .bind(tenant.0).bind(run).bind(POLICY).bind(allocation.total_cent).bind(allocation.finished_cent).bind(allocation.wip_cent).bind(allocation.rejected_cent).bind(input.completed_qty.as_micros()).bind(input.unfinished_qty.as_micros()).bind(input.rejected_qty.as_micros()).bind(allocation.unit_cent).bind(allocation.unit_cent_micros.map(|v|v.to_string())).bind(snapshot).execute(&mut **tx).await?;
    Ok(())
}
