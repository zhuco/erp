//! Current cost authority and recorded production quantities belong to the commit.
use super::{MaterialCostError, ReceiptValueActor, receipt_authority};
use erp_domain::quantity::ScaledQty;
use erp_types::{TenantId, UserId};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct CostValueActor {
    pub authentication_tenant: TenantId,
    pub user_id: UserId,
    pub session_id: Uuid,
    pub credential_epoch: i32,
}
#[derive(Clone, Copy)]
pub struct WorkOrderCostInput<'a> {
    pub work_order_id: Uuid,
    pub item_code: &'a str,
    pub center_id: Uuid,
    pub as_of: &'a str,
    pub overhead_cent: Option<i64>,
    pub completed_qty: ScaledQty,
    pub rejected_qty: ScaledQty,
    pub unfinished_qty: ScaledQty,
}
pub(super) struct Permit {
    guards: Vec<receipt_authority::Permit>,
    snapshot: Value,
    actor: CostValueActor,
}

pub(super) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    actor: CostValueActor,
) -> anyhow::Result<Permit> {
    authorize_permissions(
        tx,
        tenant,
        order,
        actor,
        &["manufacturing.write", "MFG.PRODUCTION.WRITE"],
    )
    .await
}

pub(super) async fn authorize_read(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    actor: CostValueActor,
) -> anyhow::Result<Permit> {
    authorize_permissions(
        tx,
        tenant,
        order,
        actor,
        &["manufacturing.read", "MFG.PRODUCTION.READ"],
    )
    .await
}

async fn authorize_permissions(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    order: Uuid,
    actor: CostValueActor,
    permissions: &[&str],
) -> anyhow::Result<Permit> {
    let mut identity = ReceiptValueActor {
        authentication_tenant: actor.authentication_tenant,
        user_id: actor.user_id,
        session_id: actor.session_id,
        credential_epoch: actor.credential_epoch,
        warehouse_id: Uuid::nil(),
    };
    let mut guards =
        vec![receipt_authority::lock_identity(tx, tenant, identity, permissions).await?];
    let warehouses:Vec<Uuid>=sqlx::query_scalar("select distinct warehouse_id from (select warehouse_id from mfg_material_moves where tenant_id=$1 and work_order_id=$2 union all select warehouse_id from shared_material_issues where tenant_id=$1 and work_order_id=$2 union all select q.warehouse_id from mfg_receipts r join mfg_qty_ledgers q on q.tenant_id=r.tenant_id and q.ledger_id=r.ledger_id where r.tenant_id=$1 and r.work_order_id=$2) scopes where warehouse_id is not null order by warehouse_id limit 1001")
        .bind(tenant.0).bind(order).fetch_all(&mut **tx).await?;
    if warehouses.len() > 1000 {
        return Err(MaterialCostError::Reconcile("cost warehouse scope budget exceeded").into());
    }
    for warehouse in &warehouses {
        identity.warehouse_id = *warehouse;
        guards.push(receipt_authority::lock_permissions(tx, tenant, identity, permissions).await?);
    }
    let snapshot = json!({"user_id":actor.user_id,"session_id":actor.session_id,"credential_epoch":actor.credential_epoch,"warehouses":warehouses,"grants":guards.iter().map(receipt_authority::Permit::snapshot).collect::<Vec<_>>()});
    Ok(Permit {
        guards,
        snapshot,
        actor,
    })
}

async fn recorded_quantities(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    input: WorkOrderCostInput<'_>,
) -> anyhow::Result<Value> {
    let order = input.work_order_id;
    let final_steps:Vec<String>=sqlx::query_scalar("select operation_code from shared_work_order_routes where tenant_id=$1 and work_order_id=$2 and sequence_no=(select max(sequence_no) from shared_work_order_routes where tenant_id=$1 and work_order_id=$2) order by operation_code limit 2")
        .bind(tenant.0).bind(order).fetch_all(&mut **tx).await?;
    let operations = if final_steps.is_empty() {
        sqlx::query_scalar::<_,String>("select distinct operation_code from mfg_reports where tenant_id=$1 and work_order_id=$2 order by operation_code limit 2")
        .bind(tenant.0).bind(order).fetch_all(&mut **tx).await?
    } else {
        final_steps
    };
    let [operation] = operations.as_slice() else {
        return Err(MaterialCostError::Reconcile(
            "cost output needs an unambiguous recorded final operation",
        )
        .into());
    };
    let (good,rejected):(i64,i64)=sqlx::query_as("select coalesce(sum(good_micros),0)::bigint,coalesce(sum(rejected_micros),0)::bigint from mfg_reports where tenant_id=$1 and work_order_id=$2 and operation_code=$3")
        .bind(tenant.0).bind(order).bind(operation).fetch_one(&mut **tx).await?;
    let wip:i64=sqlx::query_scalar("select coalesce(sum(qty_micros),0)::bigint from operation_wip where tenant_id=$1 and work_order_id=$2")
        .bind(tenant.0).bind(order).fetch_one(&mut **tx).await?;
    let legacy: Option<i64> = sqlx::query_scalar(
        "select wip_micros from mfg_wip_balances where tenant_id=$1 and work_order_id=$2",
    )
    .bind(tenant.0)
    .bind(order)
    .fetch_optional(&mut **tx)
    .await?;
    if legacy.is_some_and(|value| value != wip) {
        return Err(MaterialCostError::Reconcile(
            "legacy completion WIP counter is not the recorded operation WIP",
        )
        .into());
    }
    let planned: i64 = sqlx::query_scalar(
        "select order_qty_micros from mfg_work_orders where tenant_id=$1 and work_order_id=$2",
    )
    .bind(tenant.0)
    .bind(order)
    .fetch_one(&mut **tx)
    .await?;
    if good < 0
        || rejected < 0
        || wip < 0
        || i128::from(good) + i128::from(rejected) + i128::from(wip) > i128::from(planned)
    {
        return Err(MaterialCostError::Reconcile(
            "cost quantities do not match recorded final output, rejected output and WIP",
        )
        .into());
    }
    Ok(
        json!({"final_operation":operation,"good_micros":good,"rejected_micros":rejected,"wip_micros":wip,"planned_micros":planned}),
    )
}

pub(super) async fn quantities(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    input: WorkOrderCostInput<'_>,
) -> anyhow::Result<Value> {
    let mut facts = recorded_quantities(tx, tenant, input).await?;
    facts["manual_overhead_confirmation"] = json!({"amount_cent":input.overhead_cent,"source":"explicit cost confirmation input; unknown remains null","actor_evidence":"mfg_cost_commit_authorities for human calls"});
    if input.completed_qty.as_micros()
        != facts["good_micros"]
            .as_i64()
            .ok_or(MaterialCostError::Cost)?
        || input.rejected_qty.as_micros()
            != facts["rejected_micros"]
                .as_i64()
                .ok_or(MaterialCostError::Cost)?
        || input.unfinished_qty.as_micros()
            != facts["wip_micros"]
                .as_i64()
                .ok_or(MaterialCostError::Cost)?
    {
        return Err(MaterialCostError::Reconcile(
            "cost quantities do not match recorded final output, rejected output and WIP",
        )
        .into());
    }
    Ok(facts)
}

pub(super) async fn estimate_quantities<'a>(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    mut input: WorkOrderCostInput<'a>,
) -> anyhow::Result<(WorkOrderCostInput<'a>, Value)> {
    let facts = recorded_quantities(tx, tenant, input).await?;
    if input.completed_qty.as_micros()
        != facts["good_micros"]
            .as_i64()
            .ok_or(MaterialCostError::Cost)?
    {
        return Err(MaterialCostError::Reconcile(
            "estimated finished quantity does not match recorded output",
        )
        .into());
    }
    input.rejected_qty = ScaledQty::from_micros(
        facts["rejected_micros"]
            .as_i64()
            .ok_or(MaterialCostError::Cost)?,
    );
    input.unfinished_qty = ScaledQty::from_micros(
        facts["wip_micros"]
            .as_i64()
            .ok_or(MaterialCostError::Cost)?,
    );
    Ok((input, facts))
}

impl Permit {
    pub(super) async fn recheck(&self, tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<()> {
        for guard in &self.guards {
            guard.recheck(tx).await?;
        }
        Ok(())
    }
    pub(super) async fn retain(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
        request: &str,
        run: Uuid,
        facts: Value,
    ) -> anyhow::Result<()> {
        self.recheck(tx).await?;
        let snapshot = json!({"authority":self.snapshot,"quantities":facts,"cost_run_id":run,"request_id":request});
        sqlx::query("insert into mfg_cost_commit_authorities(tenant_id,request_id,cost_run_id,user_id,session_id,credential_epoch,snapshot) values($1,$2,$3,$4,$5,$6,$7)")
            .bind(tenant.0).bind(request).bind(run).bind(self.actor.user_id.0).bind(self.actor.session_id).bind(self.actor.credential_epoch).bind(&snapshot).execute(&mut **tx).await?;
        crate::operation_logs::OperationLogsRepo::insert_with_tx(
            &mut **tx,
            crate::operation_logs::OperationLogCreate {
                tenant_id: tenant,
                actor_type: "USER".into(),
                actor_user_id: Some(self.actor.user_id),
                action: "MFG_COST_COMMIT".into(),
                ref_type: Some("mfg_cost_run".into()),
                ref_id: Some(run.to_string()),
                metadata: Some(snapshot.clone()),
            },
        )
        .await?;
        crate::audit_logs::AuditLogsRepo::insert_with_tx(
            &mut **tx,
            crate::audit_logs::AuditLogCreate {
                tenant_id: tenant,
                user_id: Some(self.actor.user_id),
                action: "manufacturing.cost.commit".into(),
                entity_type: "mfg_cost_run".into(),
                entity_id: Some(run.to_string()),
                before_json: None,
                after_json: Some(snapshot.clone()),
                ip: None,
                ua: None,
            },
        )
        .await?;
        crate::outbox::OutboxRepo::enqueue_with_tx(
            &mut **tx,
            tenant,
            "manufacturing.cost.committed",
            &snapshot,
            chrono::Utc::now(),
        )
        .await?;
        self.recheck(tx).await?;
        Ok(())
    }
}
