//! Read original cost evidence without repricing or repairing historical facts.
use super::{
    CostValueActor, MaterialCostError, ReceiptValueAuthorizationError, SharedContractRepo,
    cost_commit,
};
use erp_types::TenantId;
use serde_json::{Value, json};
use uuid::Uuid;

const EVIDENCE_BYTE_LIMIT: i64 = 4 * 1024 * 1024;

impl SharedContractRepo<'_> {
    pub async fn work_order_cost_evidence(
        &self,
        tenant: TenantId,
        order: Uuid,
        run: Uuid,
        actor: CostValueActor,
    ) -> anyhow::Result<Value> {
        let mut tx = self.pool.begin().await?;
        let exists: Option<Uuid> = sqlx::query_scalar("select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for share")
            .bind(tenant.0).bind(order).fetch_optional(&mut *tx).await?;
        // Match writers: work order precedes identity and warehouse grants.
        let permit = cost_commit::authorize_read(&mut tx, tenant, order, actor).await?;
        if exists.is_none() {
            return Err(ReceiptValueAuthorizationError::Unavailable.into());
        }
        let original: Option<Value> = sqlx::query_scalar("select jsonb_build_object('cost_run_id',cost_run_id,'work_order_id',work_order_id,'material_cent',material_cent,'labor_cent',labor_cent,'overhead_cent',overhead_cent,'total_cent',total_cent,'unit_cent',unit_cent,'completed_qty_micros',completed_qty_micros::text,'estimated',estimated,'unfinished_cent',unfinished_cent,'defect_cent',defect_cent) from mfg_cost_runs where tenant_id=$1 and work_order_id=$2 and cost_run_id=$3 for share")
            .bind(tenant.0).bind(order).bind(run).fetch_optional(&mut *tx).await?;
        let original = original.ok_or(ReceiptValueAuthorizationError::Unavailable)?;
        let mut evidence = json!({});
        // Table/column names below are fixed by the server, never request input.
        for (key, table, column, id) in [
            ("material", "mfg_material_cost_bases", "cost_run_id", run),
            ("labor", "mfg_cost_labor_bases", "cost_run_id", run),
            ("allocation", "mfg_cost_allocations", "cost_run_id", run),
            (
                "authority",
                "mfg_cost_commit_authorities",
                "cost_run_id",
                run,
            ),
            (
                "center_binding",
                "mfg_cost_center_bindings",
                "work_order_id",
                order,
            ),
        ] {
            let sizes: Vec<i64> = sqlx::query_scalar(&format!("select octet_length(snapshot::text)::bigint from {table} where tenant_id=$1 and {column}=$2 limit 2"))
                .bind(tenant.0).bind(id).fetch_all(&mut *tx).await?;
            if sizes.len() > 1 || sizes.iter().any(|size| *size > EVIDENCE_BYTE_LIMIT) {
                return Err(MaterialCostError::Reconcile(
                    "cost evidence exceeds row or byte budget",
                )
                .into());
            }
            let projection = if key == "allocation" {
                "to_jsonb(e) || jsonb_build_object('unit_cent_micros',e.unit_cent_micros::text)"
            } else {
                "to_jsonb(e)"
            };
            let row: Option<Value> = sqlx::query_scalar(&format!(
                "select {projection} from {table} e where tenant_id=$1 and {column}=$2"
            ))
            .bind(tenant.0)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
            evidence[key] = row.unwrap_or(Value::Null);
        }
        let adjustments: Vec<Value> = sqlx::query_scalar("select to_jsonb(a) from mfg_cost_adjustments a where tenant_id=$1 and work_order_id=$2 order by created_at,adjustment_id limit 1001")
            .bind(tenant.0).bind(order).fetch_all(&mut *tx).await?;
        if adjustments.len() > 1000 {
            return Err(MaterialCostError::Reconcile("cost adjustment row budget exceeded").into());
        }
        let issues = evidence_issues(&original, &evidence);
        permit.recheck(&mut tx).await?;
        tx.commit().await?;
        Ok(
            json!({"schema":1,"original":original,"evidence":evidence,"work_order_adjustments":adjustments,
            "adjustment_scope":"work-order level; not allocated to this original run",
            "reconciliation_required":!issues.is_empty(),"evidence_issues":issues}),
        )
    }
}

fn evidence_issues(original: &Value, evidence: &Value) -> Vec<String> {
    let mut issues = Vec::new();
    for key in [
        "material",
        "labor",
        "allocation",
        "authority",
        "center_binding",
    ] {
        if evidence[key].is_null() {
            issues.push(format!("missing_{key}"));
        }
    }
    for (key, amount) in [("material", "material_cent"), ("labor", "labor_cent")] {
        if !evidence[key].is_null()
            && (evidence[key][amount] != original[amount]
                || evidence[key]["snapshot"][amount] != original[amount])
        {
            issues.push(format!("inconsistent_{key}"));
        }
    }
    if !evidence["material"].is_null() && evidence["material"]["snapshot"]["schema"] != 2 {
        issues.push("legacy_material_price_basis".into());
    }
    let authority = &evidence["authority"];
    if !authority.is_null()
        && (authority["snapshot"]["cost_run_id"] != original["cost_run_id"]
            || ["user_id", "session_id", "credential_epoch"]
                .iter()
                .any(|key| authority[*key] != authority["snapshot"]["authority"][*key]))
    {
        issues.push("inconsistent_authority".into());
    }
    let a = &evidence["allocation"];
    if !a.is_null() {
        let sum = ["finished_cent", "wip_cent", "rejected_cent"]
            .iter()
            .map(|key| a[*key].as_i64().map(i128::from))
            .collect::<Option<Vec<_>>>()
            .map(|values| values.iter().sum::<i128>());
        if sum != original["total_cent"].as_i64().map(i128::from)
            || a["total_cent"] != original["total_cent"]
            || a["unit_cent"] != original["unit_cent"]
            || a["wip_cent"] != original["unfinished_cent"]
            || a["rejected_cent"] != original["defect_cent"]
            || a["finished_qty_micros"]
                .as_i64()
                .map(|q| q.to_string())
                .as_deref()
                != original["completed_qty_micros"].as_str()
        {
            issues.push("inconsistent_allocation".into());
        }
        if [
            "policy",
            "total_cent",
            "finished_cent",
            "wip_cent",
            "rejected_cent",
            "finished_qty_micros",
            "wip_qty_micros",
            "rejected_qty_micros",
            "unit_cent",
            "unit_cent_micros",
        ]
        .iter()
        .any(|key| a[*key] != a["snapshot"][*key])
        {
            issues.push("inconsistent_allocation_snapshot".into());
        }
    }
    if original["overhead_cent"].is_null() {
        issues.push("unconfirmed_overhead".into());
    }
    let total = ["material_cent", "labor_cent", "overhead_cent"]
        .iter()
        .map(|key| {
            let amount = original[*key].as_i64();
            if *key == "overhead_cent" && original[*key].is_null() && original["estimated"] == true
            {
                Some(0)
            } else {
                amount.map(i128::from)
            }
        })
        .collect::<Option<Vec<_>>>()
        .map(|values| values.iter().sum::<i128>());
    if total != original["total_cent"].as_i64().map(i128::from) {
        issues.push("inconsistent_original_total".into());
    }
    issues
}
