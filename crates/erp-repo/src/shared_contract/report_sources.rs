//! Complete bounded report sources must be sound before totals or new output.
use super::{
    OperationReportError, ReceiptValueActor, ReportTotals, ScaledQty, SharedContractRepo,
    receipt_authority,
};
use erp_types::TenantId;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct ReportProof {
    report_id: Uuid,
    valid: bool,
    operation_code: String,
    good_micros: i64,
    rejected_micros: i64,
    rework_micros: i64,
    minutes: i64,
}

pub(super) async fn verify_reports(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
) -> anyhow::Result<BTreeMap<String, ReportTotals>> {
    let rows:Vec<ReportProof>=sqlx::query_as("select r.report_id,r.operation_code,r.good_micros,r.rejected_micros,r.rework_micros,r.minutes,coalesce(o.work_order_id=r.work_order_id and c.status='SUCCEEDED' and c.payload_hash=o.payload_hash and c.effect_id=r.report_id::text and o.actor_user_id::text=r.session_user_id and o.snapshot->>'effect_id'=r.report_id::text and o.snapshot->>'work_order_id'=r.work_order_id::text and (to_jsonb(r)-'created_at')=((o.snapshot->'report')-'created_at') and r.created_at=(o.snapshot->'report'->>'created_at')::timestamptz,false) as valid from mfg_reports r left join mfg_operation_report_outcomes o on o.tenant_id=r.tenant_id and o.report_id=r.report_id left join command_receipts c on c.tenant_id=o.tenant_id and c.request_id=o.request_id where r.tenant_id=$1 and r.work_order_id=$2 order by r.report_id limit 1001 for share of r")
        .bind(tenant.0).bind(order).fetch_all(&mut **tx).await?;
    if rows.len() > 1000 {
        return Err(OperationReportError::ContextLimit.into());
    }
    if rows.iter().any(|r| r.report_id.is_nil() || !r.valid) {
        return Err(OperationReportError::ReconciliationRequired.into());
    }
    aggregate_reports(rows)
}

#[derive(Default)]
struct WideTotals {
    good: i128,
    rejected: i128,
    rework: i128,
    minutes: i128,
}
fn aggregate_reports(rows: Vec<ReportProof>) -> anyhow::Result<BTreeMap<String, ReportTotals>> {
    let mut wide: BTreeMap<String, WideTotals> = BTreeMap::new();
    for row in rows {
        let sum = wide.entry(row.operation_code).or_default();
        // At most 1000 i64 rows are admitted: these sums fit i128 independently
        // of UUID ordering. Convert only the final net totals back to i64.
        sum.good += i128::from(row.good_micros);
        sum.rejected += i128::from(row.rejected_micros);
        sum.rework += i128::from(row.rework_micros);
        sum.minutes += i128::from(row.minutes);
    }
    let mut totals = BTreeMap::new();
    for (operation, sum) in wide {
        let checked = |value| {
            i64::try_from(value)
                .ok()
                .filter(|v| *v >= 0)
                .ok_or(OperationReportError::ReconciliationRequired)
        };
        totals.insert(
            operation,
            ReportTotals {
                good: ScaledQty::from_micros(checked(sum.good)?),
                rejected: ScaledQty::from_micros(checked(sum.rejected)?),
                rework: ScaledQty::from_micros(checked(sum.rework)?),
                minutes: checked(sum.minutes)?,
            },
        );
    }
    Ok(totals)
}

pub(super) fn empty_totals() -> ReportTotals {
    ReportTotals {
        good: ScaledQty::from_micros(0),
        rejected: ScaledQty::from_micros(0),
        rework: ScaledQty::from_micros(0),
        minutes: 0,
    }
}

#[derive(sqlx::FromRow)]
struct FrozenOperation {
    operation_code: String,
    predecessor_code: String,
    sequence_no: i32,
    route_id: Uuid,
    version_no: i32,
    work_center: String,
    setup_minutes: i64,
    minutes_per: i64,
    inspection_required: bool,
    report_field: String,
    parent_valid: bool,
}
pub(super) async fn verify_routes(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
) -> anyhow::Result<()> {
    routes(tx, tenant, order).await?;
    Ok(())
}
async fn routes(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
) -> anyhow::Result<Vec<FrozenOperation>> {
    let rows:Vec<FrozenOperation>=sqlx::query_as("select r.operation_code,r.predecessor_code,r.sequence_no,r.route_id,r.version_no,r.work_center,r.setup_minutes,r.minutes_per,r.inspection_required,r.report_field,coalesce(d.version_no=r.version_no and d.product_code=w.product_code and s.operation_code=r.operation_code,false) as parent_valid from shared_work_order_routes r join mfg_work_orders w on w.tenant_id=r.tenant_id and w.work_order_id=r.work_order_id left join shared_operation_routes d on d.tenant_id=r.tenant_id and d.route_id=r.route_id left join shared_operation_steps s on s.tenant_id=r.tenant_id and s.route_id=r.route_id and s.operation_code=r.operation_code where r.tenant_id=$1 and r.work_order_id=$2 order by r.sequence_no,r.operation_code limit 1001 for share of r")
        .bind(tenant.0).bind(order).fetch_all(&mut **tx).await?;
    if rows.len() > 1000 {
        return Err(OperationReportError::ContextLimit.into());
    }
    if rows.is_empty() {
        return Err(OperationReportError::ReconciliationRequired.into());
    }
    let sequences: BTreeMap<_, _> = rows
        .iter()
        .map(|r| (r.operation_code.as_str(), r.sequence_no))
        .collect();
    let mut used = BTreeSet::new();
    for row in &rows {
        if !row.parent_valid
            || row.route_id.is_nil()
            || row.version_no <= 0
            || row.sequence_no <= 0
            || row.operation_code.trim().is_empty()
            || row.work_center.trim().is_empty()
            || row.setup_minutes < 0
            || row.minutes_per < 0
            || !used.insert(row.sequence_no)
            || (!row.predecessor_code.is_empty()
                && !sequences
                    .get(row.predecessor_code.as_str())
                    .is_some_and(|p| *p < row.sequence_no))
        {
            return Err(OperationReportError::ReconciliationRequired.into());
        }
    }
    Ok(rows)
}

impl SharedContractRepo<'_> {
    pub async fn read_reporting_context(
        &self,
        tenant: TenantId,
        order: Uuid,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read")
            .execute(&mut *tx)
            .await?;
        let work:Option<Value>=sqlx::query_scalar("select to_jsonb(w) from mfg_work_orders w where tenant_id=$1 and work_order_id=$2 for share")
            .bind(tenant.0).bind(order).fetch_optional(&mut *tx).await?;
        let permit = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.read", "MFG.PRODUCTION.READ"],
        )
        .await?;
        let work = work.ok_or(OperationReportError::NotFound)?;
        let qty = work["order_qty_micros"]
            .as_i64()
            .filter(|v| *v > 0)
            .ok_or(OperationReportError::ReconciliationRequired)?;
        let operations = routes(&mut tx, tenant, order).await?;
        let verified = verify_reports(&mut tx, tenant, order).await?;
        let mut data = Vec::with_capacity(operations.len());
        for row in operations {
            let totals = verified
                .get(&row.operation_code)
                .copied()
                .unwrap_or_else(empty_totals);
            let ceiling = if row.predecessor_code.is_empty() {
                qty
            } else {
                verified
                    .get(&row.predecessor_code)
                    .copied()
                    .unwrap_or_else(empty_totals)
                    .good
                    .as_micros()
            };
            let output = totals.good.try_add(totals.rejected)?.as_micros();
            if ceiling < 0
                || output < 0
                || output > ceiling
                || totals.good.is_negative()
                || totals.rejected.is_negative()
                || totals.rework.is_negative()
                || totals.minutes < 0
            {
                return Err(OperationReportError::ReconciliationRequired.into());
            }
            data.push(json!({"operation_code":row.operation_code,"predecessor_code":row.predecessor_code,
                "sequence_no":row.sequence_no,"route_id":row.route_id,"version_no":row.version_no,
                "work_center":row.work_center,"inspection_required":row.inspection_required,"report_field":row.report_field,
                "good_micros":totals.good.as_micros().to_string(),"rejected_micros":totals.rejected.as_micros().to_string(),
                "rework_micros":totals.rework.as_micros().to_string(),"minutes":totals.minutes.to_string(),
                "ceiling_micros":ceiling.to_string(),"remaining_micros":(ceiling-output).to_string()}));
        }
        let observed: chrono::DateTime<chrono::Utc> =
            sqlx::query_scalar("select clock_timestamp()")
                .fetch_one(&mut *tx)
                .await?;
        permit.recheck(&mut tx).await?;
        tx.rollback().await?;
        Ok(
            json!({"work_order_id":order,"work_order_no":work["work_order_no"],"product_code":work["product_code"],
            "status":work["status"],"order_qty_micros":qty.to_string(),"actor_user_id":actor.user_id,
            "session_id":actor.session_id,"observed_at":observed,"operations":data}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn row(good: i64, minutes: i64) -> ReportProof {
        ReportProof {
            report_id: Uuid::new_v4(),
            valid: true,
            operation_code: "CUT".into(),
            good_micros: good,
            rejected_micros: 0,
            rework_micros: 0,
            minutes,
        }
    }
    #[test]
    fn final_net_is_independent_of_large_report_reversal_order() -> anyhow::Result<()> {
        let a = aggregate_reports(vec![row(i64::MAX, i64::MAX), row(1, 1), row(-1, -1)])?;
        let b = aggregate_reports(vec![row(-1, -1), row(1, 1), row(i64::MAX, i64::MAX)])?;
        let value = a
            .get("CUT")
            .ok_or_else(|| anyhow::anyhow!("missing actual total"))?;
        assert_eq!(value.good.as_micros(), i64::MAX);
        assert_eq!(value.minutes, i64::MAX);
        assert_eq!(a, b);
        Ok(())
    }
    #[test]
    fn unknown_overflow_or_negative_net_is_never_clamped_to_zero() {
        assert!(aggregate_reports(vec![row(i64::MAX, 0), row(1, 0)]).is_err());
        assert!(aggregate_reports(vec![row(-1, 0)]).is_err());
        assert!(aggregate_reports(vec![row(0, i64::MAX), row(0, 1)]).is_err());
        assert!(aggregate_reports(vec![row(0, -1)]).is_err());
    }
}
