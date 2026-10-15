"""Immutable handoff: findings, actual results, original failures and unpassed gates."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
output = BASE / "free-erp-inspection-gates-repair.json"
if output.exists():
    raise SystemExit("historical handoff exists")

def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda:stream.read(4*1024*1024),b""):
            result.update(block)
    return result.hexdigest()

def read(name):
    return json.loads((BASE/name).read_text(encoding="utf-8-sig"))

focused = read("free-erp-inspection-gates-fixed-final-independent.json")
full = read("free-erp-inspection-gates-full-independent.json")
unsafe = read("free-erp-inspection-gates-quality-unfixed-readback-independent.json")
docs = read("free-erp-inspection-gates-docs-delivery.json")
openapi = read("free-erp-inspection-gates-openapi-full.json")
sources = read("free-erp-inspection-gates-source-audit.json")
assert all(item["status"] == "PASS" for item in [focused,full,unsafe,docs,openapi])
assert [int(value) for value in full["test_summary"][0][1:5]] == [332,0,0,0]
assert len(full["extended_cases"]) == 14 and full["protected_existing_tenants"] == 6374
original = (BASE/"free-erp-inspection-gates-original-test.log").read_text(encoding="utf-8-sig")
expanded = (BASE/"free-erp-inspection-gates-fixed-expanded-test.log").read_text(encoding="utf-8-sig")
assert "0 passed; 4 failed; 0 ignored" in original
assert "11 passed; 1 failed; 0 ignored" in expanded
clippy = {}
for scope in ["repo","api"]:
    name = f"free-erp-inspection-gates-clippy-{scope}.log"
    value = (BASE/name).read_text(encoding="utf-8-sig")
    assert "Cargo clippy failed with exit code 101" in value
    diagnostics = re.findall(r"could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors",value)
    clippy[scope] = {"status":"FAIL","log":f"qa/acceptance/{name}","terminal_diagnostics":diagnostics,
                     "tracked_leaf_diagnostics":re.findall(r".*(?:production_receipt_release.rs|trade_stock.rs|inspection_gates.rs).*",value),
                     "api_validation":"BLOCKED_BY_DEPENDENCY" if scope=="api" else "PACKAGE_FAIL"}

current_leaf_hashes = {name:digest(ROOT/name) for name in sources["tracked_implementation_leaves"]}
for name,value in current_leaf_hashes.items():
    assert value == sources["current_input_sha256"][name],f"reviewed implementation leaf changed: {name}"
current_document_hashes = {name:digest(ROOT/name) for name in docs["document_sha256"]}
document_drift = [name for name,value in current_document_hashes.items() if value!=docs["document_sha256"][name]]
current_referenced_drift = [name for name,value in docs["referenced_source_sha256"].items()
                            if not (ROOT/name).is_file() or digest(ROOT/name)!=value]
artifact_files = sorted(set(BASE.glob("free-erp-inspection-gates-*"))
                       | set(BASE.glob("free-erp-inventory-consumers-inspection-gates-*-source.json")))
artifacts = [{"path":file.relative_to(ROOT).as_posix(),"bytes":file.stat().st_size,"sha256":digest(file)}
             for file in artifact_files if file.is_file() and file != output]
result = {"created_at":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL",
  "objective":"Repair complete quality admission for actual trade SHIP, manufacturing ISSUE/SUPPLEMENT and direct RELEASED receipts; join the exact stock projection before testing actual trade",
  "product_policy":"Manufacturing and trade, official free hosted service, one business login with concurrent multi-device sessions; advanced paid capabilities later",
  "findings":[
    "The old required-line join accepted an empty/partial frozen copy. All four unsafe actual consumers were reproduced after the trade precision guard was correctly integrated.",
    "Three actual consumers now share complete owned source/template/reading/review proof and lock the real source before reading an absent freeze.",
    "The exact stock posting bridge preserves the database precision guard; REVIEW releases the original reservation before matching physical consumption in the same transaction.",
    "A real twenty-blocked PG SHIP race observes the legacy empty freeze committed by the source-lock owner; every SHIP returns 409 with no successful command.",
    "Zero-available REVIEW and a single precise 0.5 to 0.3 reservation release with original replay pass actual database checks.",
    "The missing SalesDelivery OpenAPI path, its current thirteen request fields and four existing success shapes are now schema-described without changing the Rust input contract."],
  "actual_results":{
    "original_first_four":{"status":"FAIL","passed":0,"failed":4,"ignored":0,
      "scope":"Two actual unsafe manufacturing successes; trade preparation was initially blocked by the real precision guard, not an actual SHIP quality result."},
    "quality_unfixed_retry":{"status":"REPRODUCED_UNSAFE_ORIGINAL","passed":0,"failed":4,"ignored":0,"seconds":3.51,
      "independent_status":unsafe["status"],"assertions":unsafe["assertions"],"protected_tenants":unsafe["protected_existing_tenants"]},
    "first_expanded":{"passed":11,"failed":1,"ignored":0,"seconds":16.34,
      "failure":"Own test readback omitted mfg_material_moves; actual ISSUE/SUPPLEMENT and their replays returned correct 200. Original log retained."},
    "focused":{"passed":14,"failed":0,"ignored":0,"seconds":17.85,"independent_assertions":focused["assertions"],
      "protected_tenants":focused["protected_existing_tenants"],"tables":56},
    "full":{"passed":332,"failed":0,"ignored":0,"filtered":0,"seconds":250.70,"independent_assertions":full["assertions"],
      "protected_tenants":full["protected_existing_tenants"],"tables":56,"independent_fixture_groups":14,
      "scope":"Fourteen new fixture groups independently checked; other registered groups use their actual Rust assertions."},
    "format":{"status":"PASS","entry":"scripts/Invoke-ErpBuild.ps1","leaf_files":sources["tracked_implementation_leaves"],
      "repo_check_log":"qa/acceptance/free-erp-inspection-gates-leaf-format-check.log",
      "api_check_log":"qa/acceptance/free-erp-inspection-gates-api-leaf-format-check.log",
      "exit_code_observed_by_executor":0,"parent_roots":"Not globally formatted; concurrent changes preserved"},
    "openapi":{"status":openapi["status"],"contracts":6,"actual_success_bodies":openapi["successful_bodies"],
      "actual_rejected_responses":openapi["rejected_responses"],"project_v2_gate":"PASS",
      "planning_lookup_cancel_variants":"Defined in schema but not exercised by these fourteen quality-gate fixtures"},
    "documentation":{"status":docs["status"],"checks":docs["checks"],"current_document_drift":document_drift,
      "current_referenced_source_drift":current_referenced_drift,"scope":"Structure/current DTO inventory; no runtime or performance inferred"},
    "clippy":clippy},
  "source_qualification":{"status":"PARTIAL","window_manifest":"qa/acceptance/free-erp-inspection-gates-source-audit.json",
    "full_window_changed_inputs":len(sources["test_windows"][-1]["changed_inputs"]),
    "reviewed_leaf_sha256":current_leaf_hashes,"boundary":"Root production continuations, schemas and contracts also have concurrent edits. The actual debug result is bound to its input window, not all later current code."},
  "unpassed_gates":["Release P95/P99 and throughput","Peak memory and 4/8 MiB source-proof boundaries",
    "Formal shared connection-pool/queue backpressure","Normal password login, roles, session revocation and warehouse scope across all consumers",
    "Formal desktop/Web pages with actual edit/save/relogin","Whole migration chain, production deployment and officially hosted operation",
    "More than one real sample per project, units/calibration/attachments","All legacy UNAVAILABLE/HELD/WIP/cancel/return stock writers",
    "Repeated partial release and cancel/source competition for legacy release_stock_line",
    "Remaining complete trade costing/returns, manufacturing, MRP and free ERP launch features",
    "Concurrent new tasks/reports/commands and atomic-effects/board-claim business truth and authorization gates"],
  "preserved_failures":["Original trade preparation guard rejection","Parallel domain-symbol compilation failure before business execution",
    "Raw duplicate compatibility receipt sum in the earlier verifier","Own expanded-test missing readback table",
    "Unclassified concurrently added routes and current manufacturing documentation drift","Strict repo/API Clippy failures"],
  "data_boundary":"Protected PostgreSQL QA 127.0.0.1:15438/erp_shared_contract_test, erp_grid_test; fifty-six selected tables/all columns per old tenant unchanged. No production data migration or deployment in this tranche.",
  "artifacts":artifacts}
with output.open("x",encoding="utf-8") as stream:
    json.dump(result,stream,ensure_ascii=False,indent=2)
    stream.write("\n")
print(json.dumps({"status":result["status"],"artifacts":len(artifacts),"focused":14,"full":332,
  "independent_assertions":full["assertions"],"protected":full["protected_existing_tenants"],
  "document_drift":document_drift,"current_referenced_source_drift":current_referenced_drift},ensure_ascii=False))
