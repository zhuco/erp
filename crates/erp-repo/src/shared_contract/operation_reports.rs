//! Authenticated reporting uses the frozen route and retains the original outcome.
use super::{
    OperationReport, ReceiptValueActor, ReportTotals, ScaledQty, SharedContractRepo, admit_retry,
    insert_receipt, load_receipt, receipt_authority, succeeded,
};
use erp_types::TenantId;
use serde_json::{Value, json};
use uuid::Uuid;

pub struct OperationReportInput<'a> {
    pub request_id: &'a str,
    pub payload_hash: &'a str,
    pub work_order_id: Uuid,
    pub report: &'a OperationReport,
    pub fields: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum OperationReportError {
    #[error("complete reporting context exceeds its 1000-row budget")]
    ContextLimit,
    #[error("work order or report is outside the requested context")]
    NotFound,
    #[error("original report command requires reconciliation")]
    ReconciliationRequired,
    #[error("report request payload conflict")]
    PayloadConflict,
    #[error("report is outside the frozen route, status or quantity allowance")]
    Rejected,
    #[error("report session must be the authenticated session")]
    SessionMismatch,
}

impl SharedContractRepo<'_> {
    pub async fn save_authorized_operation_report(
        &self,
        tenant: TenantId,
        input: OperationReportInput<'_>,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        let mut tx = self.pool.begin().await?;
        // All report operations take the work-order lock, including reversals.
        let order: Option<(String, i64)> = sqlx::query_as(
            "select status,order_qty_micros from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update",
        ).bind(tenant.0).bind(input.work_order_id).fetch_optional(&mut *tx).await?;
        let permit = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.write", "MFG.PRODUCTION.WRITE"],
        )
        .await?;
        let report = input.report;
        if report.session_user_id != actor.user_id.0.to_string()
            || report.session_id != actor.session_id.to_string()
        {
            return Err(OperationReportError::SessionMismatch.into());
        }
        let Some((status, order_qty)) = order else {
            return Err(OperationReportError::NotFound.into());
        };
        sqlx::query("select pg_advisory_xact_lock(hashtext($1),hashtext($2))")
            .bind(tenant.0.to_string())
            .bind(input.request_id)
            .execute(&mut *tx)
            .await?;
        let receipt = load_receipt(&mut tx, tenant, input.request_id).await?;
        match admit_retry(receipt.as_ref(), input.request_id, input.payload_hash) {
            Ok(Some(existing)) => {
                let row: Option<(Uuid, Uuid, String, Value)> = sqlx::query_as(
                    "select work_order_id,actor_user_id,payload_hash,snapshot from mfg_operation_report_outcomes where tenant_id=$1 and request_id=$2",
                ).bind(tenant.0).bind(input.request_id).fetch_optional(&mut *tx).await?;
                let Some((order, user, hash, mut snapshot)) = row else {
                    return Err(OperationReportError::ReconciliationRequired.into());
                };
                if order != input.work_order_id
                    || user != actor.user_id.0
                    || hash != input.payload_hash
                    || snapshot["effect_id"].as_str() != Some(existing.effect_id.as_str())
                {
                    return Err(OperationReportError::ReconciliationRequired.into());
                }
                verify_original(&mut tx, tenant, input.request_id, &snapshot).await?;
                permit.recheck(&mut tx).await?;
                snapshot["replay"] = json!(true);
                tx.rollback().await?;
                return Ok(snapshot);
            }
            Err(_) => return Err(OperationReportError::PayloadConflict.into()),
            Ok(None) => {}
        }
        if status != "RELEASED" || order_qty <= 0 {
            return Err(OperationReportError::Rejected.into());
        }
        super::report_sources::verify_routes(&mut tx, tenant, input.work_order_id).await?;
        let verified =
            super::report_sources::verify_reports(&mut tx, tenant, input.work_order_id).await?;
        let started = chrono::DateTime::parse_from_rfc3339(&report.started_at)
            .map_err(|_| OperationReportError::Rejected)?;
        let ended = chrono::DateTime::parse_from_rfc3339(&report.ended_at)
            .map_err(|_| OperationReportError::Rejected)?;
        if ended < started {
            return Err(OperationReportError::Rejected.into());
        }
        let route: Option<Value> = sqlx::query_scalar(
            "select to_jsonb(r) from shared_work_order_routes r where tenant_id=$1 and work_order_id=$2 and operation_code=$3 for share",
        ).bind(tenant.0).bind(input.work_order_id).bind(&report.operation).fetch_optional(&mut *tx).await?;
        let Some(route) = route else {
            return Err(OperationReportError::ReconciliationRequired.into());
        };
        let predecessor = route["predecessor_code"]
            .as_str()
            .ok_or(OperationReportError::ReconciliationRequired)?;
        let ceiling = if predecessor.is_empty() {
            ScaledQty::from_micros(order_qty)
        } else {
            let exists: bool = sqlx::query_scalar(
                "select exists(select 1 from shared_work_order_routes where tenant_id=$1 and work_order_id=$2 and operation_code=$3)",
            ).bind(tenant.0).bind(input.work_order_id).bind(predecessor).fetch_one(&mut *tx).await?;
            if !exists {
                return Err(OperationReportError::ReconciliationRequired.into());
            }
            verified
                .get(predecessor)
                .copied()
                .unwrap_or_else(super::report_sources::empty_totals)
                .good
        };
        let original = if let Some(id) = report.reverses_report_id.as_deref() {
            let id = Uuid::parse_str(id).map_err(|_| OperationReportError::Rejected)?;
            let row: Option<OriginalReport> = sqlx::query_as(
                "select work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,reverses_report_id from mfg_reports where tenant_id=$1 and report_id=$2 for share",
            ).bind(tenant.0).bind(id).fetch_optional(&mut *tx).await?;
            let Some(original) = row else {
                return Err(OperationReportError::NotFound.into());
            };
            if original.work_order_id != input.work_order_id
                || original.operation_code != report.operation
                || original.reverses_report_id.is_some()
            {
                return Err(OperationReportError::Rejected.into());
            }
            let reversed: bool = sqlx::query_scalar(
                "select exists(select 1 from mfg_reports where tenant_id=$1 and reverses_report_id=$2)",
            ).bind(tenant.0).bind(id).fetch_one(&mut *tx).await?;
            if reversed {
                return Err(OperationReportError::Rejected.into());
            }
            let received: bool = sqlx::query_scalar("select exists(select 1 from mfg_receipts where tenant_id=$1 and work_order_id=$2 and report_id=$3 and qty_micros>0)")
                .bind(tenant.0).bind(input.work_order_id).bind(id).fetch_one(&mut *tx).await?;
            if received {
                return Err(OperationReportError::Rejected.into());
            }
            Some(ReportTotals {
                good: ScaledQty::from_micros(original.good_micros),
                rejected: ScaledQty::from_micros(original.rejected_micros),
                rework: ScaledQty::from_micros(original.rework_micros),
                minutes: original.minutes,
            })
        } else {
            None
        };
        let already = verified
            .get(&report.operation)
            .copied()
            .unwrap_or_else(super::report_sources::empty_totals);
        let totals = erp_domain::manufacturing::admit_routed_report(
            ceiling,
            Some(ceiling),
            already,
            original.as_ref(),
            report,
        )
        .map_err(|_| OperationReportError::Rejected)?;
        if totals.good.is_negative()
            || totals.rejected.is_negative()
            || totals.rework.is_negative()
            || totals.minutes < 0
        {
            return Err(OperationReportError::Rejected.into());
        }
        if report.reverses_report_id.is_some() {
            let successors: Vec<String> = sqlx::query_scalar("select operation_code from shared_work_order_routes where tenant_id=$1 and work_order_id=$2 and predecessor_code=$3 order by operation_code limit 1001")
                .bind(tenant.0).bind(input.work_order_id).bind(&report.operation).fetch_all(&mut *tx).await?;
            if successors.len() > 1000 {
                return Err(OperationReportError::ReconciliationRequired.into());
            }
            for next in successors {
                let downstream = verified
                    .get(&next)
                    .copied()
                    .unwrap_or_else(super::report_sources::empty_totals);
                let output = downstream.good.try_add(downstream.rejected)?;
                if output.as_micros() > totals.good.as_micros() {
                    return Err(OperationReportError::Rejected.into());
                }
            }
        }
        let report_id = Uuid::new_v4();
        let reverse = report
            .reverses_report_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()?;
        sqlx::query("insert into mfg_reports(tenant_id,report_id,work_order_id,operation_code,good_micros,rejected_micros,rework_micros,minutes,operator_id,session_user_id,session_id,crew_code,started_at,ended_at,reverses_report_id) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)")
            .bind(tenant.0).bind(report_id).bind(input.work_order_id).bind(&report.operation)
            .bind(report.good.as_micros()).bind(report.rejected.as_micros()).bind(report.rework.as_micros())
            .bind(report.minutes).bind(&report.operator_id).bind(&report.session_user_id).bind(&report.session_id)
            .bind(&report.crew_code).bind(&report.started_at).bind(&report.ended_at).bind(reverse).execute(&mut *tx).await?;
        let stored_report: Value = sqlx::query_scalar(
            "select to_jsonb(r) from mfg_reports r where tenant_id=$1 and report_id=$2",
        )
        .bind(tenant.0)
        .bind(report_id)
        .fetch_one(&mut *tx)
        .await?;
        let snapshot = json!({"schema":2,"report":stored_report,"request_id":input.request_id,"status":"SUCCEEDED","replay":false,
            "effect_id":report_id,"work_order_id":input.work_order_id,"actor_user_id":actor.user_id,
            "good_micros":totals.good.as_micros(),"rejected_micros":totals.rejected.as_micros(),
            "rework_micros":totals.rework.as_micros(),"minutes":totals.minutes,"fields":input.fields,
            "authority":permit.snapshot(),"source":{"route":route,"ceiling_micros":ceiling.as_micros().to_string(),"order_qty_micros":order_qty.to_string()}});
        insert_receipt(
            &mut tx,
            tenant,
            &succeeded(input.request_id, input.payload_hash, &report_id.to_string()),
        )
        .await?;
        sqlx::query("insert into mfg_operation_report_outcomes(tenant_id,request_id,work_order_id,report_id,actor_user_id,payload_hash,snapshot) values($1,$2,$3,$4,$5,$6,$7)")
            .bind(tenant.0).bind(input.request_id).bind(input.work_order_id).bind(report_id).bind(actor.user_id.0)
            .bind(input.payload_hash).bind(&snapshot).execute(&mut *tx).await?;
        permit.recheck(&mut tx).await?;
        tx.commit().await?;
        Ok(snapshot)
    }

    pub async fn read_authorized_operation_report(
        &self,
        tenant: TenantId,
        work_order: Uuid,
        request: &str,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<Value> {
        let mut tx = self.pool.begin().await?;
        let permit = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.read", "MFG.PRODUCTION.READ"],
        )
        .await?;
        let snapshot: Option<Value> = sqlx::query_scalar(
            "select snapshot from mfg_operation_report_outcomes where tenant_id=$1 and work_order_id=$2 and request_id=$3 and actor_user_id=$4",
        ).bind(tenant.0).bind(work_order).bind(request).bind(actor.user_id.0).fetch_optional(&mut *tx).await?;
        let result = snapshot.ok_or(OperationReportError::NotFound)?;
        verify_original(&mut tx, tenant, request, &result).await?;
        permit.recheck(&mut tx).await?;
        tx.rollback().await?;
        Ok(result)
    }

    pub async fn read_authorized_report_totals(
        &self,
        tenant: TenantId,
        work_order: Uuid,
        operation: &str,
        actor: ReceiptValueActor,
    ) -> anyhow::Result<ReportTotals> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("set transaction isolation level repeatable read")
            .execute(&mut *tx)
            .await?;
        let permit = receipt_authority::lock_identity(
            &mut tx,
            tenant,
            actor,
            &["manufacturing.read", "MFG.PRODUCTION.READ"],
        )
        .await?;
        let exists: bool = sqlx::query_scalar("select exists(select 1 from mfg_work_orders w join shared_work_order_routes r on r.tenant_id=w.tenant_id and r.work_order_id=w.work_order_id where w.tenant_id=$1 and w.work_order_id=$2 and r.operation_code=$3)")
            .bind(tenant.0).bind(work_order).bind(operation).fetch_one(&mut *tx).await?;
        if !exists {
            return Err(OperationReportError::NotFound.into());
        }
        super::report_sources::verify_routes(&mut tx, tenant, work_order).await?;
        let verified = super::report_sources::verify_reports(&mut tx, tenant, work_order).await?;
        let result = verified
            .get(operation)
            .copied()
            .unwrap_or_else(super::report_sources::empty_totals);
        permit.recheck(&mut tx).await?;
        tx.rollback().await?;
        Ok(result)
    }
}

#[derive(sqlx::FromRow)]
struct OriginalReport {
    work_order_id: Uuid,
    operation_code: String,
    good_micros: i64,
    rejected_micros: i64,
    rework_micros: i64,
    minutes: i64,
    reverses_report_id: Option<Uuid>,
}
async fn verify_original(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: TenantId,
    request: &str,
    snapshot: &Value,
) -> anyhow::Result<()> {
    let valid:bool=sqlx::query_scalar("select exists(select 1 from mfg_operation_report_outcomes o join command_receipts c on c.tenant_id=o.tenant_id and c.request_id=o.request_id join mfg_reports r on r.tenant_id=o.tenant_id and r.report_id=o.report_id and r.work_order_id=o.work_order_id where o.tenant_id=$1 and o.request_id=$2 and c.status='SUCCEEDED' and c.payload_hash=o.payload_hash and c.effect_id=o.report_id::text and (to_jsonb(r)-'created_at')=(($3->'report')-'created_at') and r.created_at=($3->'report'->>'created_at')::timestamptz)")
        .bind(tenant.0).bind(request).bind(snapshot).fetch_one(&mut **tx).await?;
    if !valid {
        return Err(OperationReportError::ReconciliationRequired.into());
    }
    Ok(())
}
