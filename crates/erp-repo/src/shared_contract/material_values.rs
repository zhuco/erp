//! Normal issue values remain attached to their SKU and original issue.
use super::{MaterialCostError, ReceiptValueActor, atomic_effects, item_cost};
use erp_types::TenantId;
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    movement: Uuid,
    actor: Option<ReceiptValueActor>,
) -> anyhow::Result<()> {
    let snapshot:Value=sqlx::query_scalar("select jsonb_build_object('move_id',m.move_id,'work_order_id',m.work_order_id,'warehouse_id',m.warehouse_id,'sku_id',m.sku_id,'kind',m.kind,'qty_micros',m.qty_micros,'issue_value',(select to_jsonb(v) from mfg_material_issue_values v where v.tenant_id=m.tenant_id and v.issue_id=m.move_id),'allocation',(select to_jsonb(v) from mfg_material_value_allocations v where v.tenant_id=m.tenant_id and v.allocation_id=m.move_id)) from mfg_material_moves m where m.tenant_id=$1 and m.move_id=$2")
        .bind(tenant.0).bind(movement).fetch_one(&mut **tx).await?;
    crate::operation_logs::OperationLogsRepo::insert_with_tx(
        &mut **tx,
        crate::operation_logs::OperationLogCreate {
            tenant_id: tenant,
            actor_type: if actor.is_some() { "USER" } else { "INTERNAL" }.into(),
            actor_user_id: actor.map(|a| a.user_id),
            action: "MATERIAL_SOURCE_VALUE".into(),
            ref_type: Some("mfg_material_move".into()),
            ref_id: Some(movement.to_string()),
            metadata: Some(snapshot.clone()),
        },
    )
    .await?;
    crate::audit_logs::AuditLogsRepo::insert_with_tx(
        &mut **tx,
        crate::audit_logs::AuditLogCreate {
            tenant_id: tenant,
            user_id: actor.map(|a| a.user_id),
            action: "manufacturing.material.source_value".into(),
            entity_type: "mfg_material_move".into(),
            entity_id: Some(movement.to_string()),
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
        "manufacturing.material.source_value",
        &snapshot,
        chrono::Utc::now(),
    )
    .await?;
    Ok(())
}

pub(super) async fn prepare_return(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    issue: Uuid,
) -> anyhow::Result<()> {
    let price:Option<Value>=sqlx::query_scalar("select snapshot->'price' from mfg_material_issue_values where tenant_id=$1 and issue_id=$2 and state='VALUED'")
        .bind(tenant.0).bind(issue).fetch_optional(&mut **tx).await?;
    if let Some(price) = price {
        item_cost::material_return_lock(tx, tenant, &price).await?;
    }
    Ok(())
}

pub(super) async fn freeze(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    issue: Uuid,
    price: Value,
) -> anyhow::Result<()> {
    let (order,warehouse,sku,qty):(Uuid,Uuid,Uuid,i64)=sqlx::query_as("select work_order_id,warehouse_id,sku_id,qty_micros from mfg_material_moves where tenant_id=$1 and move_id=$2 and kind in('ISSUE','SUPPLEMENT')")
        .bind(tenant.0).bind(issue).fetch_one(&mut **tx).await?;
    let cent = item_cost::material_book_issue(tx, tenant, issue, qty, &price).await?;
    let snapshot = json!({"issue_id":issue,"work_order_id":order,"warehouse_id":warehouse,"sku_id":sku,"qty_micros":qty,"price":price,"total_cent":cent});
    sqlx::query("insert into mfg_material_issue_values(tenant_id,issue_id,work_order_id,warehouse_id,sku_id,qty_micros,state,total_cent,snapshot) values($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(tenant.0).bind(issue).bind(order).bind(warehouse).bind(sku).bind(qty).bind(if cent.is_some(){"VALUED"}else{"UNVALUED"}).bind(cent).bind(snapshot).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn allocate(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    issue: Uuid,
    allocation: Uuid,
    kind: &str,
    qty: i64,
    request: Option<&str>,
) -> anyhow::Result<()> {
    let original: Value = sqlx::query_scalar(
        "select to_jsonb(m) from mfg_material_moves m where tenant_id=$1 and move_id=$2 for update",
    )
    .bind(tenant.0)
    .bind(issue)
    .fetch_one(&mut **tx)
    .await?;
    let total_qty = original["qty_micros"]
        .as_i64()
        .ok_or(MaterialCostError::Cost)?;
    let frozen: Option<Value> = sqlx::query_scalar(
        "select to_jsonb(v) from mfg_material_issue_values v where tenant_id=$1 and issue_id=$2",
    )
    .bind(tenant.0)
    .bind(issue)
    .fetch_optional(&mut **tx)
    .await?;
    let atomic:Option<(Uuid,Value)>=sqlx::query_as("select document_id,snapshot from atomic_source_documents where tenant_id=$1 and move_id=$2 and source_action='ISSUE' and snapshot is not null")
        .bind(tenant.0).bind(issue).fetch_optional(&mut **tx).await?;
    let (total, source) = if let Some((document, snapshot)) = atomic {
        atomic_effects::verify_original(tx, tenant, document, &snapshot).await?;
        (
            snapshot["amount_cent"].as_i64(),
            json!({"kind":"ATOMIC_ORIGINAL","document_id":document,"amount_cent":snapshot["amount_cent"]}),
        )
    } else if let Some(frozen) = frozen {
        if frozen["work_order_id"] != original["work_order_id"]
            || frozen["warehouse_id"] != original["warehouse_id"]
            || frozen["sku_id"] != original["sku_id"]
            || frozen["qty_micros"] != original["qty_micros"]
        {
            return Err(
                MaterialCostError::Reconcile("original material value identity changed").into(),
            );
        }
        (frozen["total_cent"].as_i64(), frozen)
    } else {
        (None, json!({"kind":"LEGACY_UNVALUED"}))
    };
    let prior:i64=sqlx::query_scalar("select coalesce(sum(qty_micros),0)::bigint from mfg_material_value_allocations where tenant_id=$1 and issue_id=$2")
        .bind(tenant.0).bind(issue).fetch_one(&mut **tx).await?;
    let next = prior.checked_add(qty).ok_or(MaterialCostError::Cost)?;
    if total_qty <= 0 || qty <= 0 || next > total_qty {
        return Err(MaterialCostError::Reconcile(
            "material value allocation exceeds original quantity",
        )
        .into());
    }
    let actual:i64=sqlx::query_scalar("select ((select coalesce(sum(qty_micros),0) from mfg_material_moves where tenant_id=$1 and original_issue_id=$2 and move_id<>$3 and kind in('CONSUME','RETURN'))+(select coalesce(sum(-delta_micros),0) from shared_stock_ledgers where tenant_id=$1 and source_line=$2::text and bucket='WIP' and delta_micros<0 and request_id is distinct from $4))::bigint")
        .bind(tenant.0).bind(issue).bind(allocation).bind(request).fetch_one(&mut **tx).await?;
    let cent = if actual == prior {
        total
            .map(|value| {
                i64::try_from(
                    i128::from(value) * i128::from(next) / i128::from(total_qty)
                        - i128::from(value) * i128::from(prior) / i128::from(total_qty),
                )
            })
            .transpose()?
    } else {
        None
    };
    if kind == "RETURN"
        && source["state"] == "VALUED"
        && let Some(cent) = cent
    {
        item_cost::material_book_return(tx, tenant, issue, qty, cent, &source["snapshot"]["price"])
            .await?;
    }
    sqlx::query("insert into mfg_material_value_allocations(tenant_id,allocation_id,issue_id,request_id,kind,qty_micros,cost_cent,snapshot) values($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(tenant.0).bind(allocation).bind(issue).bind(request).bind(kind).bind(qty).bind(cent).bind(json!({"original":original,"source":source,"prior_micros":prior,"actual_prior_micros":actual,"cost_cent":cent})).execute(&mut **tx).await?;
    Ok(())
}
