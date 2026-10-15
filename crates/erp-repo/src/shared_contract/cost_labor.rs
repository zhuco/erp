//! A report's recorded date and bound center select its captured labor source.
use super::{CostValueActor, MaterialCostError, SharedContractRepo, cost_commit};
use erp_types::TenantId;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) struct Labor {
    pub cent: i64,
    snapshot: Value,
}
impl SharedContractRepo<'_> {
    pub async fn bind_cost_center(
        &self,
        tenant: TenantId,
        order: Uuid,
        center: Uuid,
        reason: &str,
        actor: CostValueActor,
    ) -> anyhow::Result<Value> {
        if reason.trim().is_empty() || reason.len() > 4096 {
            return Err(MaterialCostError::Reconcile(
                "cost center binding needs an explicit reason",
            )
            .into());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
            .bind(tenant.0).bind(order).fetch_one(&mut *tx).await?;
        let permit = cost_commit::authorize(&mut tx, tenant, order, actor).await?;
        let owned: bool = sqlx::query_scalar(
            "select exists(select 1 from shared_work_centers where tenant_id=$1 and center_id=$2)",
        )
        .bind(tenant.0)
        .bind(center)
        .fetch_one(&mut *tx)
        .await?;
        if !owned {
            return Err(MaterialCostError::Reconcile(
                "bound work center is outside the enterprise",
            )
            .into());
        }
        let old:Option<(Uuid,Value)>=sqlx::query_as("select center_id,snapshot from mfg_cost_center_bindings where tenant_id=$1 and work_order_id=$2")
            .bind(tenant.0).bind(order).fetch_optional(&mut *tx).await?;
        if let Some((stored, snapshot)) = old {
            if stored != center {
                return Err(MaterialCostError::Reconcile(
                    "cost center is already bound; historical assignment cannot be replaced",
                )
                .into());
            }
            permit.recheck(&mut tx).await?;
            tx.rollback().await?;
            return Ok(snapshot);
        }
        let snapshot = json!({"work_order_id":order,"center_id":center,"reason":reason.trim(),"user_id":actor.user_id,"session_id":actor.session_id,"credential_epoch":actor.credential_epoch,"scope":"explicit work-order default center; frozen route centers override per operation"});
        sqlx::query("insert into mfg_cost_center_bindings(tenant_id,work_order_id,center_id,snapshot) values($1,$2,$3,$4)")
            .bind(tenant.0).bind(order).bind(center).bind(&snapshot).execute(&mut *tx).await?;
        crate::operation_logs::OperationLogsRepo::insert_with_tx(
            &mut *tx,
            crate::operation_logs::OperationLogCreate {
                tenant_id: tenant,
                actor_type: "USER".into(),
                actor_user_id: Some(actor.user_id),
                action: "MFG_COST_CENTER_BIND".into(),
                ref_type: Some("mfg_work_order".into()),
                ref_id: Some(order.to_string()),
                metadata: Some(snapshot.clone()),
            },
        )
        .await?;
        crate::audit_logs::AuditLogsRepo::insert_with_tx(
            &mut *tx,
            crate::audit_logs::AuditLogCreate {
                tenant_id: tenant,
                user_id: Some(actor.user_id),
                action: "manufacturing.cost.center.bind".into(),
                entity_type: "mfg_work_order".into(),
                entity_id: Some(order.to_string()),
                before_json: None,
                after_json: Some(snapshot.clone()),
                ip: None,
                ua: None,
            },
        )
        .await?;
        crate::outbox::OutboxRepo::enqueue_with_tx(
            &mut *tx,
            tenant,
            "manufacturing.cost.center.bound",
            &snapshot,
            chrono::Utc::now(),
        )
        .await?;
        permit.recheck(&mut tx).await?;
        tx.commit().await?;
        Ok(snapshot)
    }
}

pub(super) async fn calculate(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    requested_center: Uuid,
    requested_date: &str,
) -> anyhow::Result<Labor> {
    let binding:Option<(Uuid,chrono::DateTime<chrono::Utc>)>=sqlx::query_as("select center_id,created_at from mfg_cost_center_bindings where tenant_id=$1 and work_order_id=$2")
        .bind(tenant.0).bind(order).fetch_optional(&mut **tx).await?;
    let (default_center, bound_at) = binding.ok_or(MaterialCostError::Reconcile(
        "cost center has no recorded work-order binding",
    ))?;
    if requested_center != default_center {
        return Err(MaterialCostError::Reconcile(
            "cost request center differs from its recorded assignment",
        )
        .into());
    }
    let cutoff = chrono::NaiveDate::parse_from_str(requested_date, "%Y-%m-%d")
        .map_err(|_| MaterialCostError::Reconcile("cost cutoff date is invalid"))?;
    let reports:Vec<Value>=sqlx::query_scalar("select to_jsonb(r) from mfg_reports r where tenant_id=$1 and work_order_id=$2 order by created_at,report_id limit 10001")
        .bind(tenant.0).bind(order).fetch_all(&mut **tx).await?;
    if reports.len() > 10000 {
        return Err(MaterialCostError::Reconcile("labor source budget exceeded").into());
    }
    let mut total = 0i64;
    let mut evidence = Vec::with_capacity(reports.len());
    for report in reports {
        let priced_report = if let Some(original) = report["reverses_report_id"].as_str() {
            let original = Uuid::parse_str(original).map_err(|_| {
                MaterialCostError::Reconcile("labor reversal source identity is invalid")
            })?;
            let parent:Value=sqlx::query_scalar("select to_jsonb(r) from mfg_reports r where tenant_id=$1 and work_order_id=$2 and report_id=$3")
                .bind(tenant.0).bind(order).bind(original).fetch_optional(&mut **tx).await?.ok_or(MaterialCostError::Reconcile("labor reversal has no original report"))?;
            let minutes = parent["minutes"].as_i64().ok_or(MaterialCostError::Cost)?;
            if parent["reverses_report_id"].is_null()
                && minutes >= 0
                && minutes.checked_neg() == report["minutes"].as_i64()
            {
                parent
            } else {
                return Err(MaterialCostError::Reconcile(
                    "labor reversal differs from original reported minutes",
                )
                .into());
            }
        } else {
            if report["minutes"].as_i64().is_none_or(|m| m < 0) {
                return Err(MaterialCostError::Reconcile(
                    "negative labor has no original reversal source",
                )
                .into());
            }
            report.clone()
        };
        let created = chrono::DateTime::parse_from_rfc3339(
            priced_report["created_at"]
                .as_str()
                .ok_or(MaterialCostError::Cost)?,
        )
        .map_err(|_| MaterialCostError::Cost)?
        .with_timezone(&chrono::Utc);
        let started = priced_report["started_at"].as_str().unwrap_or("");
        let day = if started.is_empty() {
            created.date_naive()
        } else {
            chrono::DateTime::parse_from_rfc3339(started)
                .map_err(|_| {
                    MaterialCostError::Reconcile("reported start time cannot select a labor date")
                })?
                .date_naive()
        };
        if day > cutoff {
            return Err(
                MaterialCostError::Reconcile("cost cutoff precedes a recorded work date").into(),
            );
        }
        let operation = priced_report["operation_code"]
            .as_str()
            .ok_or(MaterialCostError::Cost)?;
        let routed:Option<String>=sqlx::query_scalar("select work_center from shared_work_order_routes where tenant_id=$1 and work_order_id=$2 and operation_code=$3")
            .bind(tenant.0).bind(order).bind(operation).fetch_optional(&mut **tx).await?;
        let center = if let Some(code) = routed.filter(|c| !c.is_empty()) {
            sqlx::query_scalar::<_,Uuid>("select center_id from shared_work_centers where tenant_id=$1 and (center_code=$2 or center_id::text=$2)")
                .bind(tenant.0).bind(code).fetch_optional(&mut **tx).await?.ok_or(MaterialCostError::Reconcile("frozen operation work center has no enterprise identity"))?
        } else {
            default_center
        };
        let capture = created.max(bound_at);
        let rate:Option<Value>=sqlx::query_scalar("select to_jsonb(r) from shared_work_center_rates r where tenant_id=$1 and center_id=$2 and start_date<=$3 and (end_date='' or end_date>=$3) and created_at<=$4 order by version_no desc,rate_id desc limit 1 for share")
            .bind(tenant.0).bind(center).bind(day.to_string()).bind(capture).fetch_optional(&mut **tx).await?;
        let rate = rate.ok_or(MaterialCostError::Reconcile(
            "recorded work date has no captured rate version",
        ))?;
        let minutes = report["minutes"].as_i64().ok_or(MaterialCostError::Cost)?;
        let unit = rate["rate_cent"]
            .as_i64()
            .filter(|v| *v >= 0)
            .ok_or(MaterialCostError::Cost)?;
        let amount = minutes.checked_mul(unit).ok_or(MaterialCostError::Cost)?;
        total = total.checked_add(amount).ok_or(MaterialCostError::Cost)?;
        evidence.push(json!({"report":report,"price_report":priced_report,"work_date":day.to_string(),"center_id":center,"rate":rate,"labor_cent":amount,"capture_boundary":capture}));
    }
    if total < 0 {
        return Err(
            MaterialCostError::Reconcile("reported labor reversals exceed their source").into(),
        );
    }
    Ok(Labor {
        cent: total,
        snapshot: json!({"work_order_id":order,"default_center_id":default_center,"cost_cutoff":cutoff.to_string(),"reports":evidence,"labor_cent":total,"empty_start_policy":"recorded server creation date","binding_policy":"rates captured no later than the later of report creation and explicit center binding"}),
    })
}

pub(super) async fn store(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    run: Uuid,
    labor: &Labor,
) -> anyhow::Result<()> {
    sqlx::query("insert into mfg_cost_labor_bases(tenant_id,cost_run_id,labor_cent,snapshot) values($1,$2,$3,$4)")
        .bind(tenant.0).bind(run).bind(labor.cent).bind(&labor.snapshot).execute(&mut **tx).await?;
    Ok(())
}
