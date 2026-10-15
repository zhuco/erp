"""Wire the owned atomic material service with bounded, freshly archived edits."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"

def replace_function(source, name, replacement):
    marker = "fn " + name + "("
    assert source.count(marker) == 1, name
    middle = source.index(marker)
    start = source.rfind("\n", 0, middle) + 1
    brace = source.index("{", middle)
    depth, end = 1, brace + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[:start] + replacement.strip("\n") + source[end:]

names = ["crates/erp-repo/src/shared_contract.rs", "crates/erp-api/src/manufacturing.rs",
         "crates/erp-domain/src/manufacturing.rs", "crates/erp-repo/src/shared_contract/production_receipt_release.rs"]
changes = {}
for index, name in enumerate(names):
    path = ROOT / name
    raw = path.read_bytes()
    source = raw.decode("utf8").replace("\r\n", "\n")
    if index == 0:
        assert "mod atomic_effects;" not in source
        source = source.replace("mod material_moves;", "mod atomic_effects;\nmod material_moves;", 1)
        marker = "pub use command_readback::{"
        assert source.count(marker) == 1
        source = source.replace(marker, "pub use atomic_effects::{AtomicEffectError, AtomicEffectInput, AtomicEffectWrite};\n" + marker, 1)
        source = source.replace("admit_atomic_effect, ", "").replace("atomic_effect_already_posted, ", "")
        source = replace_function(source, "commit_atomic_effect", '''
    pub async fn commit_atomic_effect(
        &self,
        tenant_id: TenantId,
        request_id: &str,
        payload_hash: &str,
        work_order_id: Uuid,
        warehouse_id: Uuid,
        sku_id: Uuid,
        effect: &AtomicEffect,
    ) -> anyhow::Result<Result<(SharedWrite, Uuid), ReceiptError>> {
        let outcome = self.commit_atomic_effect_with_actor(
            tenant_id, request_id, payload_hash,
            AtomicEffectInput { work_order_id, warehouse_id, sku_id, effect },
            &ProductionReleaseActor::default(),
        ).await?;
        Ok(outcome.map(|write| (write.effect, write.document_id)))
    }
''')
    elif index == 1:
        source = replace_function(source, "commit_atomic_effect_route", '''
async fn commit_atomic_effect_route(
    State(state): State<AppState>,
    Path((tenant_id, work_order_id)): Path<(String, uuid::Uuid)>,
    headers: HeaderMap,
    Extension(ctx): Extension<AuthContext>,
    Extension(rctx): Extension<RequestContext>,
    Json(req): Json<AtomicEffectRequest>,
) -> impl IntoResponse {
    let tenant_id = match require_tenant(&tenant_id, &ctx) {
        Ok(id) => id, Err(resp) => return resp,
    };
    let request_id = match shared_request_id(&headers, Some(&req.request_id)) {
        Ok(Some(id)) => id,
        Ok(None) => return bad_request("request_id required").into_response(),
        Err(resp) => return resp,
    };
    if req.failing_line || req.prepared_lines < 1 {
        return bad_request("bad atomic compatibility preflight").into_response();
    }
    let qty = match erp_domain::quantity::parse_qty(&req.qty) {
        Ok(qty) => qty, Err(_) => return bad_request("bad atomic qty").into_response(),
    };
    let effect = erp_domain::manufacturing::AtomicEffect {
        source_action: req.source_action.trim().to_string(),
        action_no: req.action_no.trim().to_string(), qty, amount_cent: req.amount_cent,
    };
    let payload = match serde_json::to_string(&(2,tenant_id,ctx.tenant_id,ctx.user_id,work_order_id,
        req.warehouse_id,req.sku_id,effect.source_action.as_str(),effect.action_no.as_str(),
        effect.qty.as_micros(),effect.amount_cent)) {
        Ok(payload) => payload,
        Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR,format!("{error}")).into_response(),
    };
    let actor = ctx.production_release_actor(&rctx);
    match SharedContractRepo::new(state.db.pool()).commit_atomic_effect_with_actor(
        tenant_id,&request_id,&payload,
        erp_repo::shared_contract::AtomicEffectInput {work_order_id,warehouse_id:req.warehouse_id,sku_id:req.sku_id,effect:&effect},
        &actor).await {
        Ok(Ok(write)) => (StatusCode::OK,Json(serde_json::json!({
            "request_id":request_id,"status":"SUCCEEDED","replay":matches!(write.effect,SharedWrite::Replay(_)),
            "effect_id":write.document_id,"partial_success":false,"applied_lines":1,"snapshot":write.snapshot,
        }))).into_response(),
        Ok(Err(_)) => conflict("payload conflict").into_response(),
        Err(error) => {
            if matches!(error.downcast_ref::<erp_repo::shared_contract::ProductionReleaseError>(),
                Some(erp_repo::shared_contract::ProductionReleaseError::Forbidden)) {
                return forbidden("atomic material warehouse access denied").into_response();
            }
            match error.downcast_ref::<erp_repo::shared_contract::AtomicEffectError>() {
                Some(erp_repo::shared_contract::AtomicEffectError::NotFound) => not_found("atomic material source not found").into_response(),
                Some(erp_repo::shared_contract::AtomicEffectError::ReconciliationRequired) =>
                    (StatusCode::CONFLICT,Json(serde_json::json!({"code":"atomic_material_reconciliation_required"}))).into_response(),
                None => (StatusCode::UNPROCESSABLE_ENTITY,format!("{error}")).into_response(),
            }
        }
    }
}
''')
    elif index == 2:
        source = replace_function(source, "admit_atomic_effect", '''
pub fn admit_atomic_effect(effect: &AtomicEffect) -> Result<(), ManufacturingError> {
    if effect.source_action.trim() != "ISSUE" || effect.action_no.trim().is_empty() {
        return Err(ManufacturingError::IllegalTransition(
            "atomic material requires an ISSUE and a stable action number".into(),
        ));
    }
    if effect.qty.as_micros() <= 0 || effect.amount_cent < 0 {
        return Err(ManufacturingError::Quantity(QuantityError::InvalidText));
    }
    Ok(())
}
''')
    else:
        assert source.count("    user: Option<ProductionReleaseUser>,") == 1
        source = source.replace("    user: Option<ProductionReleaseUser>,", "    pub(super) user: Option<ProductionReleaseUser>,", 1)
        assert source.count("    async fn require_warehouse(") == 1
        source = source.replace("    async fn require_warehouse(", "    pub(super) async fn require_warehouse(", 1)
    archive = BASE / f"free-erp-atomic-effects-before-wire-{index}.txt"
    with archive.open("xb") as output:
        output.write(raw)
    assert path.read_bytes() == raw, "source changed during bounded wiring: " + name
    newline = "\r\n" if b"\r\n" in raw else "\n"
    updated = source.replace("\n", newline).encode("utf8")
    path.write_bytes(updated)
    changes[name] = {"before_sha256":hashlib.sha256(raw).hexdigest(),"after_sha256":hashlib.sha256(updated).hexdigest(),
                     "archive":archive.relative_to(ROOT).as_posix()}
with (BASE / "free-erp-atomic-effects-wire.json").open("x", encoding="utf8") as output:
    json.dump(changes, output, ensure_ascii=False, indent=2)
print(json.dumps({"changed":len(changes)}))
