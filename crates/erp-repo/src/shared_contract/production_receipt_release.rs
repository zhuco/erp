//! Production release uses owned receipt facts, server inspection rules and one transaction.
use super::{SharedContractRepo, SharedWrite, insert_receipt, load_receipt, receipt_totals};
use crate::{
    audit_logs::{AuditLogCreate, AuditLogsRepo},
    inventory::InventoryService,
    inventory_precision::{InventoryPreciseDelta, InventoryQuantityKey},
    operation_logs::{OperationLogCreate, OperationLogsRepo},
};
use erp_domain::{
    quantity::ScaledQty,
    receipt::{ReceiptError, admit_retry, succeeded},
};
use erp_types::{TenantId, UserId};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ProductionReleaseError {
    #[error("production release warehouse access denied")]
    Forbidden,
    #[error("production receipt is outside this enterprise")]
    NotFound,
    #[error("production release reconciliation required: {0}")]
    ReconciliationRequired(&'static str),
    #[error("production release inspection is incomplete or failed")]
    InspectionBlocked,
    #[error("production release exceeds the 1000 inspection item budget")]
    InspectionLimit,
    #[error("production release warehouse scope exceeds 10000 grants")]
    ScopeLimit,
}

/// Application-authenticated identity. This type is never decoded from request JSON.
#[derive(Clone, Debug)]
pub struct ProductionReleaseUser {
    pub authentication_tenant: TenantId,
    pub user_id: UserId,
    pub is_admin: bool,
    pub request_id: String,
    pub trace_id: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

/// Explicit internal callers use SYSTEM; HTTP must supply its authenticated user.
#[derive(Clone, Debug, Default)]
pub struct ProductionReleaseActor {
    pub(super) user: Option<ProductionReleaseUser>,
}

impl ProductionReleaseActor {
    pub fn authenticated(user: ProductionReleaseUser) -> Self {
        Self { user: Some(user) }
    }

    pub(super) async fn require_warehouse(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
        warehouse: Uuid,
    ) -> anyhow::Result<()> {
        let Some(user) = &self.user else {
            return Ok(());
        };
        if user.user_id.0.is_nil()
            || user.authentication_tenant.0.is_nil()
            || (!user.is_admin && user.authentication_tenant != tenant)
        {
            return Err(ProductionReleaseError::Forbidden.into());
        }
        // Warehouse replacement locks the same user before inserting/removing grants.
        // Keep this shared lock until commit, including the established empty = all policy.
        let status: Option<String> = sqlx::query_scalar(
            "select status from users where tenant_id=$1 and user_id=$2 for share",
        )
        .bind(user.authentication_tenant.0)
        .bind(user.user_id.0)
        .fetch_optional(&mut **tx)
        .await?;
        if status.as_deref() != Some("ACTIVE") {
            return Err(ProductionReleaseError::Forbidden.into());
        }
        if user.is_admin {
            return Ok(());
        }
        let grants: Vec<Uuid> = sqlx::query_scalar(
            "select warehouse_id from user_warehouse_access where tenant_id=$1 and user_id=$2 order by warehouse_id limit 10001 for share")
            .bind(tenant.0).bind(user.user_id.0).fetch_all(&mut **tx).await?;
        if grants.len() > 10000 {
            return Err(ProductionReleaseError::ScopeLimit.into());
        }
        if !grants.is_empty() && !grants.contains(&warehouse) {
            return Err(ProductionReleaseError::Forbidden.into());
        }
        Ok(())
    }

    async fn record(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: TenantId,
        receipt: Uuid,
        mut metadata: Value,
    ) -> anyhow::Result<()> {
        let user_id = self.user.as_ref().map(|user| user.user_id);
        if let Some(user) = &self.user {
            metadata["transport_request_id"] = json!(user.request_id);
            metadata["trace_id"] = json!(user.trace_id);
            metadata["authentication_tenant_id"] = json!(user.authentication_tenant);
        } else {
            metadata["actor_source"] = json!("internal.manufacturing");
        }
        OperationLogsRepo::insert_with_tx(
            &mut **tx,
            OperationLogCreate {
                tenant_id: tenant,
                actor_type: if self.user.is_some() {
                    "USER"
                } else {
                    "SYSTEM"
                }
                .into(),
                actor_user_id: user_id,
                action: "MFG_RECEIPT_RELEASE".into(),
                ref_type: Some("mfg_receipt".into()),
                ref_id: Some(receipt.to_string()),
                metadata: Some(metadata.clone()),
            },
        )
        .await?;
        AuditLogsRepo::insert_with_tx(
            &mut **tx,
            AuditLogCreate {
                tenant_id: tenant,
                user_id,
                action: "mfg.receipt.release".into(),
                entity_type: "mfg_receipt".into(),
                entity_id: Some(receipt.to_string()),
                before_json: None,
                after_json: Some(metadata),
                ip: self.user.as_ref().and_then(|user| user.ip.clone()),
                ua: self.user.as_ref().and_then(|user| user.user_agent.clone()),
            },
        )
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ReceiptSource {
    work_order_id: Uuid,
    qty_micros: i64,
    available_micros: i64,
    status: String,
    report_id: Option<Uuid>,
    batch_code: Option<String>,
    inspection_id: Option<Uuid>,
    ledger_id: Option<Uuid>,
}

pub(super) struct Proof {
    pub(super) warehouse: Uuid,
    pub(super) sku: Uuid,
    pub(super) location: Uuid,
    pub(super) batch: String,
    qty_ledger: Uuid,
    stock_ledger: Uuid,
    inventory_ledger: Uuid,
    projection_ledger: Uuid,
    inspection: Uuid,
    pub(super) snapshot: Value,
}

pub(super) async fn held_source(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    receipt: Uuid,
) -> anyhow::Result<(Uuid, i64, Proof)> {
    let owner: Option<Uuid> = sqlx::query_scalar(
        "select work_order_id from mfg_receipts where tenant_id=$1 and receipt_id=$2",
    )
    .bind(tenant.0)
    .bind(receipt)
    .fetch_optional(&mut **tx)
    .await?;
    let owner = owner.ok_or(ProductionReleaseError::NotFound)?;
    lock_quality_source(tx, tenant, "WORK_ORDER", owner).await?;
    let source:ReceiptSource=sqlx::query_as("select work_order_id,qty_micros,available_micros,status,report_id,batch_code,inspection_id,ledger_id from mfg_receipts where tenant_id=$1 and receipt_id=$2 for update")
        .bind(tenant.0).bind(receipt).fetch_one(&mut **tx).await?;
    let proof = source_proof(tx, tenant, receipt, &source).await?;
    Ok((owner, source.qty_micros, proof))
}

fn reconcile(reason: &'static str) -> anyhow::Error {
    ProductionReleaseError::ReconciliationRequired(reason).into()
}

/// Every WORK_ORDER inspection writer locks this row before inserting a new item.
/// This protects the absence of a required item as well as already stored rows.
pub(super) async fn lock_quality_source(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    kind: &str,
    source: Uuid,
) -> anyhow::Result<()> {
    if kind == "WORK_ORDER" {
        let found: Option<Uuid> = sqlx::query_scalar(
            "select work_order_id from mfg_work_orders where tenant_id=$1 and work_order_id=$2 for update")
            .bind(tenant.0).bind(source).fetch_optional(&mut **tx).await?;
        if found.is_none() {
            return Err(reconcile("inspection work order is missing"));
        }
    }
    Ok(())
}

async fn source_proof(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    receipt: Uuid,
    source: &ReceiptSource,
) -> anyhow::Result<Proof> {
    if source.status != "HOLD" || source.qty_micros <= 0 || source.available_micros != 0 {
        return Err(reconcile("receipt is not an intact positive held quantity"));
    }
    let report = source
        .report_id
        .filter(|id| !id.is_nil())
        .ok_or_else(|| reconcile("original report identity is missing"))?;
    let inspection = source
        .inspection_id
        .filter(|id| !id.is_nil())
        .ok_or_else(|| reconcile("original inspection identity is missing"))?;
    let qty_ledger = source
        .ledger_id
        .filter(|id| !id.is_nil())
        .ok_or_else(|| reconcile("original quantity ledger identity is missing"))?;
    let batch = source
        .batch_code
        .clone()
        .ok_or_else(|| reconcile("original batch is unknown"))?;
    let report_row: Option<(i64, Value)> = sqlx::query_as(
        "select good_micros,to_jsonb(r) from mfg_reports r where tenant_id=$1 and report_id=$2 and work_order_id=$3 for share")
        .bind(tenant.0).bind(report).bind(source.work_order_id).fetch_optional(&mut **tx).await?;
    let Some((reported, report_snapshot)) = report_row else {
        return Err(reconcile("report does not belong to receipt work order"));
    };
    let reversed: i64 = sqlx::query_scalar(
        "select coalesce(sum(good_micros::numeric),0)::bigint from mfg_reports where tenant_id=$1 and work_order_id=$2 and reverses_report_id=$3")
        .bind(tenant.0).bind(source.work_order_id).bind(report).fetch_one(&mut **tx).await?;
    let received: i64 = sqlx::query_scalar(
        "select coalesce(sum(qty_micros::numeric),0)::bigint from mfg_receipts where tenant_id=$1 and work_order_id=$2 and report_id=$3")
        .bind(tenant.0).bind(source.work_order_id).bind(report).fetch_one(&mut **tx).await?;
    if i128::from(reported) + i128::from(reversed) < i128::from(received) {
        return Err(reconcile(
            "receipts exceed the valid original report quantity",
        ));
    }
    let inspection_row: Option<(bool, Value)> = sqlx::query_as(
        "select (execution_id=$3 and result='HOLD' and good_micros=$4 and rejected_micros=0),to_jsonb(i) from mfg_inspections i where tenant_id=$1 and inspection_id=$2 for update")
        .bind(tenant.0).bind(inspection).bind(source.work_order_id).bind(source.qty_micros)
        .fetch_optional(&mut **tx).await?;
    let Some((true, inspection_snapshot)) = inspection_row else {
        return Err(reconcile(
            "original held inspection is missing or inconsistent",
        ));
    };
    let quantities: Vec<(Uuid, Uuid, Uuid, i64, Value)> = sqlx::query_as(
        "select ledger_id,warehouse_id,sku_id,delta_micros,to_jsonb(l) from mfg_qty_ledgers l where tenant_id=$1 and move_id=$2 and reason_type='MFG_RECEIPT' order by ledger_id limit 2 for share")
        .bind(tenant.0).bind(receipt).fetch_all(&mut **tx).await?;
    let [(stored_id, warehouse, sku, delta, quantity_snapshot)] = quantities.as_slice() else {
        return Err(reconcile(
            "receipt requires exactly one original quantity ledger",
        ));
    };
    if *stored_id != qty_ledger || *delta != source.qty_micros || warehouse.is_nil() || sku.is_nil()
    {
        return Err(reconcile(
            "quantity ledger identity or quantity differs from receipt",
        ));
    }
    let batch_enabled: Option<bool> = sqlx::query_scalar(
        "select batch_enabled from skus where tenant_id=$1 and sku_id=$2 for share",
    )
    .bind(tenant.0)
    .bind(sku)
    .fetch_optional(&mut **tx)
    .await?;
    let warehouse_owned: Option<Uuid> = sqlx::query_scalar(
        "select warehouse_id from warehouses where tenant_id=$1 and warehouse_id=$2 for share",
    )
    .bind(tenant.0)
    .bind(warehouse)
    .fetch_optional(&mut **tx)
    .await?;
    if batch_enabled.is_none() || warehouse_owned.is_none() {
        return Err(reconcile(
            "original warehouse or SKU is outside this enterprise",
        ));
    }
    // Legacy text batches have no UUID balance ownership. Do not borrow another batch's holds.
    if batch_enabled == Some(true) {
        return Err(reconcile(
            "receipt needs a verified exact UUID batch quantity mapping",
        ));
    }
    let stocks: Vec<(Uuid, Uuid, bool, Value)> = sqlx::query_as(
        "select ledger_id,location_id,(warehouse_id=$3 and sku_id=$4 and batch_code=$5 and quality_status='HOLD' and bucket='ON_HAND' and delta_micros=$6),to_jsonb(s) from shared_stock_ledgers s where tenant_id=$1 and source_line=$2 order by ledger_id limit 2 for share")
        .bind(tenant.0).bind(receipt.to_string()).bind(warehouse).bind(sku).bind(&batch)
        .bind(source.qty_micros).fetch_all(&mut **tx).await?;
    let [(stock_ledger, location, true, stock_snapshot)] = stocks.as_slice() else {
        return Err(reconcile(
            "original held stock ledger is missing or inconsistent",
        ));
    };
    let inventories: Vec<(Uuid, bool, Value)> = sqlx::query_as(
        "select ledger_id,coalesce((delta_on_hand_micros=$5 and delta_frozen_micros=$5 and delta_reserved_micros=0 and delta_in_transit_micros=0 and reason_type='MFG_RECEIPT'),false),to_jsonb(i) from inventory_ledgers i where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 and ref_type='shared_stock_ledger' and ref_id=$4 order by ledger_id limit 2 for share")
        .bind(tenant.0).bind(warehouse).bind(sku).bind(stock_ledger.to_string())
        .bind(source.qty_micros).fetch_all(&mut **tx).await?;
    let [(inventory_ledger, true, inventory_snapshot)] = inventories.as_slice() else {
        return Err(reconcile(
            "original exact physical stock ledger is missing or inconsistent",
        ));
    };
    let projections: Vec<(Uuid, bool, Value)> = sqlx::query_as(
        "select ledger_id,coalesce((delta_on_hand_micros=$5 and delta_frozen_micros=$5 and delta_reserved_micros=0 and delta_in_transit_micros=0 and reason_type='MFG_RECEIPT'),false),to_jsonb(i) from inventory_ledgers i where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 and ref_type='mfg_material_move' and ref_id=$4 order by ledger_id limit 2 for share")
        .bind(tenant.0).bind(warehouse).bind(sku).bind(receipt.to_string())
        .bind(source.qty_micros).fetch_all(&mut **tx).await?;
    let [(projection_ledger, true, projection_snapshot)] = projections.as_slice() else {
        return Err(reconcile(
            "original manufacturing quantity projection is missing or inconsistent",
        ));
    };
    Ok(Proof {
        warehouse: *warehouse,
        sku: *sku,
        location: *location,
        batch,
        qty_ledger,
        stock_ledger: *stock_ledger,
        inventory_ledger: *inventory_ledger,
        projection_ledger: *projection_ledger,
        inspection,
        snapshot: json!({"report":report_snapshot,"valid_report_good_micros":
            (i128::from(reported)+i128::from(reversed)).to_string(),"received_from_report_micros":received.to_string(),
            "inspection":inspection_snapshot,"quantity":quantity_snapshot,"stock":stock_snapshot,
            "inventory":inventory_snapshot,"projection":projection_snapshot}),
    })
}

pub(super) async fn inspection_proof(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    work_order: Uuid,
    batch: &str,
    receipt: Option<Uuid>,
) -> anyhow::Result<Value> {
    scoped_inspection_proof(
        tx,
        tenant,
        super::inspection_sources::SourceKind::WorkOrder,
        work_order,
        Some(batch),
        receipt,
        false,
    )
    .await
}

/// Complete evidence for material issue, direct receipt and trade shipment.
/// None retains the material caller's existing all-batch scope.
pub(super) async fn source_inspection_proof(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    source_kind: &str,
    source_id: Uuid,
    batch: Option<&str>,
) -> anyhow::Result<Value> {
    let kind = super::inspection_sources::SourceKind::parse(source_kind)?;
    if !matches!(
        kind,
        super::inspection_sources::SourceKind::WorkOrder
            | super::inspection_sources::SourceKind::SalesOrder
    ) {
        return Err(reconcile("physical source inspection kind is unsupported"));
    }
    scoped_inspection_proof(tx, tenant, kind, source_id, batch, None, false).await
}

pub(super) async fn concession_inspection_proof(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    owner: Uuid,
    batch: &str,
    receipt: Uuid,
) -> anyhow::Result<Value> {
    scoped_inspection_proof(
        tx,
        tenant,
        super::inspection_sources::SourceKind::WorkOrder,
        owner,
        Some(batch),
        Some(receipt),
        true,
    )
    .await
}

async fn scoped_inspection_proof(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    scope_kind: super::inspection_sources::SourceKind,
    source_id: Uuid,
    batch: Option<&str>,
    receipt: Option<Uuid>,
    concession: bool,
) -> anyhow::Result<Value> {
    use super::inspection_sources::{SourceKind, lock_source};
    // Acquire the source lock before even an empty header read. A competing
    // freeze must not commit between an absence check and physical movement.
    let owner = lock_source(tx, tenant, scope_kind.as_str(), source_id)
        .await
        .map_err(|_| reconcile("inspection source cannot be reconciled"))?;
    let scope_ids: Vec<Uuid> = std::iter::once(source_id).chain(receipt).collect();
    let headers:Vec<Value>=sqlx::query_scalar(
        "select jsonb_build_object('freeze',case when octet_length(f.source_kind)<=32 and octet_length(f.item_code)<=1024 and octet_length(f.batch_code)<=512 then to_jsonb(f) else jsonb_build_object('budget_exceeded',true) end,'template_valid',exists(select 1 from shared_inspection_templates t where t.tenant_id=f.tenant_id and t.template_id=f.template_id and t.version_no=f.version_no)) from shared_inspection_freezes f where f.tenant_id=$1 and f.source_id=any($2::uuid[]) and ($3::text is null or f.batch_code in('',$3)) order by f.freeze_id limit 1001 for share")
        .bind(tenant.0).bind(&scope_ids).bind(batch).fetch_all(&mut **tx).await?;
    if headers.len() > 1000 {
        return Err(ProductionReleaseError::InspectionLimit.into());
    }
    let mut owned_receipt = None;
    let mut selected = Vec::new();
    for header in headers {
        let f = &header["freeze"];
        if f["budget_exceeded"] == true {
            return Err(ProductionReleaseError::InspectionLimit.into());
        }
        let kind = SourceKind::parse(f["source_kind"].as_str().unwrap_or_default())
            .map_err(|_| reconcile("frozen inspection has an unknown source kind"))?;
        let source = f["source_id"]
            .as_str()
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| reconcile("frozen source identity is unknown"))?;
        let target = match kind {
            kind if kind == scope_kind && source == source_id => &owner,
            SourceKind::ProductionReceipt if Some(source) == receipt => {
                if owned_receipt.is_none() {
                    owned_receipt =
                        Some(lock_source(tx, tenant, "PRODUCTION_RECEIPT", source).await?);
                }
                owned_receipt
                    .as_ref()
                    .ok_or_else(|| reconcile("receipt inspection owner is missing"))?
            }
            _ => continue,
        };
        if header["template_valid"] != true {
            return Err(reconcile("frozen template version is not owned"));
        }
        target
            .require_item(
                tx,
                tenant,
                f["item_code"].as_str().unwrap_or_default(),
                f["batch_code"].as_str().unwrap_or_default(),
            )
            .await
            .map_err(|_| reconcile("frozen item or batch does not own this source"))?;
        selected.push(header);
    }
    let ids: Vec<Uuid> = selected
        .iter()
        .map(|h| {
            h["freeze"]["freeze_id"]
                .as_str()
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or_else(|| reconcile("freeze identity is unknown"))
        })
        .collect::<anyhow::Result<_>>()?;
    let lines:Vec<Value>=sqlx::query_scalar(
        "select case when octet_length(l.item_code)<=1024 then to_jsonb(l) else jsonb_build_object('budget_exceeded',true) end from shared_inspection_freeze_lines l where l.tenant_id=$1 and l.freeze_id=any($2::uuid[]) order by l.freeze_id,l.item_code limit 1001 for share")
        .bind(tenant.0).bind(&ids).fetch_all(&mut **tx).await?;
    if lines.len() > 1000 || lines.iter().any(|line| line["budget_exceeded"] == true) {
        return Err(ProductionReleaseError::InspectionLimit.into());
    }
    let binding_bytes:i64=sqlx::query_scalar(
        "select coalesce(sum(octet_length(to_jsonb(b)::text)::bigint),0)::bigint from shared_inspection_source_bindings b where b.tenant_id=$1 and b.freeze_id=any($2::uuid[])")
        .bind(tenant.0).bind(&ids).fetch_one(&mut **tx).await?;
    if binding_bytes > 8 * 1024 * 1024 {
        return Err(ProductionReleaseError::InspectionLimit.into());
    }
    let bindings:Vec<Value>=sqlx::query_scalar(
        "select case when octet_length(to_jsonb(b)::text)<=4194304 then to_jsonb(b) else jsonb_build_object('budget_exceeded',true) end from shared_inspection_source_bindings b where b.tenant_id=$1 and b.freeze_id=any($2::uuid[]) order by b.freeze_id limit 1001 for share")
        .bind(tenant.0).bind(&ids).fetch_all(&mut **tx).await?;
    if bindings.len() > 1000
        || bindings
            .iter()
            .any(|binding| binding["budget_exceeded"] == true)
    {
        return Err(ProductionReleaseError::InspectionLimit.into());
    }
    let legacy_templates: Vec<Uuid> = selected
        .iter()
        .filter(|h| {
            !bindings
                .iter()
                .any(|b| b["freeze_id"] == h["freeze"]["freeze_id"])
        })
        .map(|h| {
            h["freeze"]["template_id"]
                .as_str()
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or_else(|| reconcile("legacy template identity is unknown"))
        })
        .collect::<anyhow::Result<_>>()?;
    let template_lines:Vec<Value>=sqlx::query_scalar(
        "select case when octet_length(l.item_code)<=1024 then to_jsonb(l) else jsonb_build_object('budget_exceeded',true) end from shared_inspection_template_lines l where l.tenant_id=$1 and l.template_id=any($2::uuid[]) order by l.template_id,l.item_code limit 1001 for share")
        .bind(tenant.0).bind(&legacy_templates).fetch_all(&mut **tx).await?;
    if template_lines.len() > 1000
        || template_lines
            .iter()
            .any(|line| line["budget_exceeded"] == true)
    {
        return Err(ProductionReleaseError::InspectionLimit.into());
    }
    let copy = |line: &Value| {
        json!({"item_code":line["item_code"],"required":line["required"],
        "min_micros":line["min_micros"],"max_micros":line["max_micros"],"sample_count":line["sample_count"]})
    };
    let mut frozen = Vec::new();
    for header in selected {
        let f = &header["freeze"];
        let actual: Vec<&Value> = lines
            .iter()
            .filter(|line| line["freeze_id"] == f["freeze_id"])
            .collect();
        if actual.is_empty() {
            return Err(reconcile("frozen inspection has no complete item copy"));
        }
        let actual_copy = json!(actual.iter().map(|line| copy(line)).collect::<Vec<_>>());
        if let Some(binding) = bindings.iter().find(|b| b["freeze_id"] == f["freeze_id"]) {
            if binding["frozen_lines"] != actual_copy
                || binding["line_count"].as_u64() != Some(actual.len() as u64)
                || [
                    "source_kind",
                    "source_id",
                    "template_id",
                    "version_no",
                    "item_code",
                    "batch_code",
                ]
                .iter()
                .any(|field| binding[*field] != f[*field])
            {
                return Err(reconcile(
                    "frozen items differ from their original source binding",
                ));
            }
        } else {
            let expected: Vec<Value> = template_lines
                .iter()
                .filter(|line| line["template_id"] == f["template_id"])
                .map(copy)
                .collect();
            if actual_copy != json!(expected) {
                return Err(reconcile(
                    "legacy frozen copy cannot be reconciled with its owned template version",
                ));
            }
        }
        frozen.extend(
            actual
                .into_iter()
                .map(|line| json!({"freeze":f,"line":line})),
        );
    }
    let readings: Vec<Value> = sqlx::query_scalar(
        "select case when octet_length(q.item_code)<=1024 and octet_length(q.inspector)<=512 and octet_length(q.source_kind)<=32 then to_jsonb(q) else jsonb_build_object('budget_exceeded',true) end from mfg_quality_items q where q.tenant_id=$1 and ($3::text is null or q.batch_code in('',$3)) and ((q.work_order_id=$2 and q.source_kind='') or q.freeze_id=any($4::uuid[]) or (upper(btrim(q.source_kind))=$6 and q.source_id=$7) or (upper(btrim(q.source_kind)) in('PRODUCTION_RECEIPT','RECEIPT') and q.source_id=$5)) order by q.item_id limit 1001 for share")
        .bind(tenant.0).bind(owner.work_order).bind(batch).bind(&ids).bind(receipt)
        .bind(scope_kind.as_str()).bind(source_id).fetch_all(&mut **tx).await?;
    if frozen.len() > 1000
        || readings.len() > 1000
        || readings.iter().any(|q| q["budget_exceeded"] == true)
    {
        return Err(ProductionReleaseError::InspectionLimit.into());
    }
    let effective = super::inspection_outcomes::effective_readings(tx, tenant, &readings)
        .await
        .map_err(
            |error| match error.downcast_ref::<super::InspectionOutcomeError>() {
                Some(super::InspectionOutcomeError::Limit) => {
                    ProductionReleaseError::InspectionLimit.into()
                }
                Some(super::InspectionOutcomeError::HistoryRequired) => {
                    reconcile("inspection reading history cannot be reconciled")
                }
                _ => error,
            },
        )?;
    if concession && effective.readings.is_empty() {
        return Err(ProductionReleaseError::InspectionBlocked.into());
    }
    let valid = |q: &Value, min: i64, max: i64| {
        q["reading_present"] == true
            && (concession || q["passed"] == true)
            && (concession || q["review_allows_release"] != false)
            && q["inspector"]
                .as_str()
                .is_some_and(|name| !name.trim().is_empty())
            && min <= max
            && q["reading_micros"]
                .as_i64()
                .is_some_and(|v| concession || (v >= min && v <= max))
    };
    for item in &frozen {
        let line = &item["line"];
        if line["required"] != true {
            continue;
        }
        let min = line["min_micros"]
            .as_i64()
            .ok_or_else(|| reconcile("frozen minimum is unknown"))?;
        let max = line["max_micros"]
            .as_i64()
            .ok_or_else(|| reconcile("frozen maximum is unknown"))?;
        let matches: Vec<_> = effective
            .readings
            .iter()
            .filter(|q| {
                q["freeze_id"] == item["freeze"]["freeze_id"]
                    && q["item_code"] == line["item_code"]
                    && SourceKind::parse(q["source_kind"].as_str().unwrap_or_default()).ok()
                        == SourceKind::parse(
                            item["freeze"]["source_kind"].as_str().unwrap_or_default(),
                        )
                        .ok()
                    && q["source_id"] == item["freeze"]["source_id"]
                    && q["batch_code"] == item["freeze"]["batch_code"]
            })
            .collect();
        if matches.len() != 1 || !valid(matches[0], min, max) {
            return Err(ProductionReleaseError::InspectionBlocked.into());
        }
    }
    for q in &readings {
        if q["budget_exceeded"] == true {
            return Err(ProductionReleaseError::InspectionLimit.into());
        }
        if !q["freeze_id"].is_null() {
            let Some(item) = frozen.iter().find(|f| {
                f["freeze"]["freeze_id"] == q["freeze_id"]
                    && f["line"]["item_code"] == q["item_code"]
            }) else {
                return Err(reconcile("reading refers to an unknown frozen requirement"));
            };
            if SourceKind::parse(q["source_kind"].as_str().unwrap_or_default()).ok()
                != SourceKind::parse(item["freeze"]["source_kind"].as_str().unwrap_or_default())
                    .ok()
                || ["source_id", "batch_code"]
                    .iter()
                    .any(|key| q[*key] != item["freeze"][*key])
                || ["required", "min_micros", "max_micros"]
                    .iter()
                    .any(|key| q[*key] != item["line"][*key])
            {
                return Err(reconcile(
                    "reading differs from its frozen source and standard",
                ));
            }
        }
        if let Some(current) = effective
            .readings
            .iter()
            .find(|current| current["item_id"] == q["item_id"])
            && (q["required"] == true || q["reading_present"] == true)
        {
            let min = q["min_micros"]
                .as_i64()
                .ok_or_else(|| reconcile("reading minimum is unknown"))?;
            let max = q["max_micros"]
                .as_i64()
                .ok_or_else(|| reconcile("reading maximum is unknown"))?;
            if !valid(current, min, max) {
                return Err(ProductionReleaseError::InspectionBlocked.into());
            }
        }
    }
    let required = frozen
        .iter()
        .filter(|f| f["line"]["required"] == true)
        .count()
        + readings
            .iter()
            .filter(|q| q["freeze_id"].is_null() && q["required"] == true)
            .count();
    Ok(
        json!({"policy":if required == 0 {"no_required_items"} else {"all_required_items_passed"},
        "required_item_count":required,"frozen":frozen,"readings":readings,"source_bindings":bindings,
        "effective_readings":effective.readings,"reading_events":effective.events,"reading_reviews":effective.reviews}),
    )
}

impl SharedContractRepo<'_> {
    pub async fn release_production_receipt_with_actor(
        &self,
        tenant: TenantId,
        request_id: &str,
        payload_hash: &str,
        receipt: Uuid,
        expected_work_order: Option<Uuid>,
        actor: &ProductionReleaseActor,
    ) -> anyhow::Result<Result<(SharedWrite, i64), ReceiptError>> {
        if receipt.is_nil()
            || tenant.0.is_nil()
            || expected_work_order.is_some_and(|id| id.is_nil())
        {
            return Err(ProductionReleaseError::NotFound.into());
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("select pg_advisory_xact_lock(hashtext($1::text),hashtext($2))")
            .bind(tenant.0.to_string())
            .bind(format!("production-release-command:{}", request_id.trim()))
            .execute(&mut *tx)
            .await?;
        let work_order: Option<Uuid> = sqlx::query_scalar(
            "select work_order_id from mfg_receipts where tenant_id=$1 and receipt_id=$2",
        )
        .bind(tenant.0)
        .bind(receipt)
        .fetch_optional(&mut *tx)
        .await?;
        let work_order = work_order.ok_or(ProductionReleaseError::NotFound)?;
        if expected_work_order.is_some_and(|id| id != work_order) {
            return Err(reconcile(
                "receipt does not belong to the requested work order",
            ));
        }
        lock_quality_source(&mut tx, tenant, "WORK_ORDER", work_order).await?;
        let source: ReceiptSource = sqlx::query_as(
            "select work_order_id,qty_micros,available_micros,status,report_id,batch_code,inspection_id,ledger_id from mfg_receipts where tenant_id=$1 and receipt_id=$2 for update")
            .bind(tenant.0).bind(receipt).fetch_one(&mut *tx).await?;
        if source.work_order_id != work_order {
            return Err(reconcile("receipt work order changed during admission"));
        }
        let retry = match admit_retry(
            load_receipt(&mut tx, tenant, request_id).await?.as_ref(),
            request_id,
            payload_hash,
        ) {
            Ok(value) => value,
            Err(error) => {
                tx.rollback().await?;
                return Ok(Err(error));
            }
        };
        if let Some(existing) = retry {
            let cached: Option<(Uuid, Uuid, Uuid, String, i64)> = sqlx::query_as(
                "select receipt_id,work_order_id,warehouse_id,payload_hash,available_total_micros from mfg_receipt_release_commands where tenant_id=$1 and request_id=$2")
                .bind(tenant.0).bind(request_id.trim()).fetch_optional(&mut *tx).await?;
            let Some((original_receipt, original_order, warehouse, original_payload, available)) =
                cached
            else {
                return Err(reconcile(
                    "original release result is unknown; current totals cannot replace it",
                ));
            };
            if existing.status != erp_domain::receipt::CommandStatus::Succeeded
                || original_receipt != receipt
                || original_order != work_order
                || original_payload != payload_hash
                || existing.effect_id != receipt.to_string()
            {
                return Err(reconcile("original command does not own this release"));
            }
            actor.require_warehouse(&mut tx, tenant, warehouse).await?;
            tx.rollback().await?;
            return Ok(Ok((SharedWrite::Replay(existing), available)));
        }
        let split:bool=sqlx::query_scalar("select exists(select 1 from mfg_receipt_disposition_commands where tenant_id=$1 and receipt_id=$2)")
            .bind(tenant.0).bind(receipt).fetch_one(&mut *tx).await?;
        if split {
            return Err(reconcile(
                "receipt has source-linked dispositions; release its actual remainder through that protocol",
            ));
        }
        let proof = source_proof(&mut tx, tenant, receipt, &source).await?;
        actor
            .require_warehouse(&mut tx, tenant, proof.warehouse)
            .await?;
        let quality =
            inspection_proof(&mut tx, tenant, work_order, &proof.batch, Some(receipt)).await?;
        let qty = ScaledQty::from_micros(source.qty_micros);
        let hold_stock = Uuid::new_v4();
        let good_stock = Uuid::new_v4();
        let key = InventoryQuantityKey {
            tenant_id: tenant,
            warehouse_id: proof.warehouse,
            sku_id: proof.sku,
        };
        let mut effects = Vec::with_capacity(2);
        for (stock, delta, frozen, quality_status) in [
            (
                hold_stock,
                ScaledQty::ZERO.try_sub(qty)?,
                ScaledQty::ZERO.try_sub(qty)?,
                "HOLD",
            ),
            (good_stock, qty, ScaledQty::ZERO, "GOOD"),
        ] {
            let effect = InventoryService::new(self.pool)
                .apply_precise_effect_with_tx(
                    &mut tx,
                    InventoryPreciseDelta {
                        key,
                        on_hand: delta,
                        reserved: ScaledQty::ZERO,
                        frozen,
                        in_transit: ScaledQty::ZERO,
                        reason_type: "MFG_RECEIPT_RELEASE".into(),
                        ref_type: Some("shared_stock_ledger".into()),
                        ref_id: Some(stock.to_string()),
                        occurred_at: chrono::Utc::now(),
                    },
                )
                .await?;
            // This shadow is the existing manufacturing source projection, not a second physical change.
            let projection: Uuid = sqlx::query_scalar(
                "insert into inventory_ledgers(tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,ref_type,ref_id,occurred_at) select tenant_id,warehouse_id,sku_id,delta_on_hand,delta_reserved,delta_frozen,delta_in_transit,delta_on_hand_micros,delta_reserved_micros,delta_frozen_micros,delta_in_transit_micros,reason_type,'mfg_material_move',$5,occurred_at from inventory_ledgers where tenant_id=$1 and warehouse_id=$2 and sku_id=$3 and ledger_id=$4 returning ledger_id")
                .bind(tenant.0).bind(proof.warehouse).bind(proof.sku).bind(effect.ledger_id)
                .bind(stock.to_string()).fetch_one(&mut *tx).await?;
            let quantity_ledger: Uuid = sqlx::query_scalar(
                "insert into mfg_qty_ledgers(tenant_id,warehouse_id,sku_id,move_id,delta_micros,reason_type,source_receipt_id) values($1,$2,$3,$4,$5,'MFG_RECEIPT_RELEASE',$6) returning ledger_id")
                .bind(tenant.0).bind(proof.warehouse).bind(proof.sku).bind(stock)
                .bind(delta.as_micros()).bind(receipt).fetch_one(&mut *tx).await?;
            sqlx::query("insert into shared_stock_ledgers(tenant_id,ledger_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket,delta_micros,source_line,request_id) values($1,$2,$3,$4,$5,$6,$7,'ON_HAND',$8,$9,$10)")
                .bind(tenant.0).bind(stock).bind(proof.warehouse).bind(proof.location).bind(proof.sku)
                .bind(&proof.batch).bind(quality_status).bind(delta.as_micros()).bind(receipt.to_string())
                .bind(request_id.trim()).execute(&mut *tx).await?;
            effects.push(json!({"quality_status":quality_status,"stock_ledger_id":stock,
                "inventory_ledger_id":effect.ledger_id,"projection_ledger_id":projection,
                "quantity_ledger_id":quantity_ledger,"delta_on_hand_micros":delta.as_micros().to_string(),
                "delta_frozen_micros":frozen.as_micros().to_string()}));
        }
        let inspection_rows = sqlx::query("update mfg_inspections set result='RELEASED' where tenant_id=$1 and inspection_id=$2 and result='HOLD'")
            .bind(tenant.0).bind(proof.inspection).execute(&mut *tx).await?.rows_affected();
        let receipt_rows = sqlx::query("update mfg_receipts set status='RELEASED',available_micros=$3 where tenant_id=$1 and receipt_id=$2 and status='HOLD' and available_micros=0")
            .bind(tenant.0).bind(receipt).bind(source.qty_micros).execute(&mut *tx).await?.rows_affected();
        if inspection_rows != 1 || receipt_rows != 1 {
            return Err(reconcile(
                "release source transition did not affect exactly one row",
            ));
        }
        // Original result is read and durably stored before commit. No fallible read follows commit.
        let (_, available) = receipt_totals(&mut tx, tenant, work_order).await?;
        let snapshot = json!({"schema":1,"tenant_id":tenant,"request_id":request_id.trim(),
            "receipt_id":receipt,"work_order_id":work_order,"warehouse_id":proof.warehouse,
            "sku_id":proof.sku,"location_id":proof.location,"batch_code":proof.batch,
            "qty_micros":source.qty_micros.to_string(),"available_total_micros":available.to_string(),
            "source":proof.snapshot,"quality":quality,"effects":effects});
        actor
            .record(&mut tx, tenant, receipt, snapshot.clone())
            .await?;
        let command = succeeded(request_id.trim(), payload_hash, &receipt.to_string());
        insert_receipt(&mut tx, tenant, &command).await?;
        sqlx::query("insert into mfg_receipt_release_commands(tenant_id,request_id,payload_hash,receipt_id,work_order_id,warehouse_id,sku_id,qty_micros,available_total_micros,source_qty_ledger_id,source_stock_ledger_id,source_inventory_ledger_id,source_projection_ledger_id,snapshot) values($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)")
            .bind(tenant.0).bind(request_id.trim()).bind(payload_hash).bind(receipt).bind(work_order)
            .bind(proof.warehouse).bind(proof.sku).bind(source.qty_micros).bind(available)
            .bind(proof.qty_ledger).bind(proof.stock_ledger).bind(proof.inventory_ledger)
            .bind(proof.projection_ledger).bind(&snapshot).execute(&mut *tx).await?;
        let children = sqlx::query("insert into mfg_receipt_release_effects(tenant_id,request_id,warehouse_id,sku_id,quality_status,stock_ledger_id,inventory_ledger_id,projection_ledger_id,quantity_ledger_id,delta_on_hand_micros,delta_frozen_micros) select $1,$2,$3,$4,e->>'quality_status',(e->>'stock_ledger_id')::uuid,(e->>'inventory_ledger_id')::uuid,(e->>'projection_ledger_id')::uuid,(e->>'quantity_ledger_id')::uuid,(e->>'delta_on_hand_micros')::bigint,(e->>'delta_frozen_micros')::bigint from jsonb_array_elements($5::jsonb->'effects') e")
            .bind(tenant.0).bind(request_id.trim()).bind(proof.warehouse).bind(proof.sku)
            .bind(&snapshot).execute(&mut *tx).await?.rows_affected();
        if children != 2 {
            return Err(reconcile("release must persist both actual effects"));
        }
        tx.commit().await?;
        Ok(Ok((SharedWrite::Stored(command), available)))
    }
}
