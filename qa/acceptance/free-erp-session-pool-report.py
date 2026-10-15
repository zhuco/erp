"""Collect the preserved failures and current login qualification artifacts."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-session-pool-"
TARGET = BASE / (PREFIX + "repair.json")
if TARGET.exists():
    raise SystemExit("refusing to replace repair evidence")


def load(name):
    return json.loads((BASE/name).read_text(encoding="utf-8-sig"))


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for raw in iter(lambda:stream.read(1048576),b""):
            h.update(raw)
    return h.hexdigest()


def result(name):
    log = (BASE/name).read_text(encoding="utf-8-sig")
    matches = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in ([\d.]+)s",log)
    if len(matches)!=1:
        raise SystemExit("one actual terminal test result required: "+name)
    status,passed,failed,ignored,measured,filtered,seconds = matches[0]
    return {"status":"PASS" if status=="ok" else "FAIL","passed":int(passed),"failed":int(failed),
            "ignored":int(ignored),"measured":int(measured),"filtered_out":int(filtered),"seconds":float(seconds),"log":name}


def lint(name):
    log = (BASE/name).read_text(encoding="utf-8-sig")
    diagnostics = re.findall(r"^error(?:\[[^\]]+\])?: (.+)$",log,re.M)
    summaries = re.findall(r"^error: could not compile .+$",log,re.M)
    passed = re.search(r"ERP strict (?:repo|api) Clippy exit_code=0",log) is not None
    return {"status":"PASS" if passed and not diagnostics else ("FAIL" if summaries else "NOT_RUN"),
            "diagnostics":len(diagnostics),"compiler_summaries":summaries,"log":name,"warnings_as_errors":True}


original = load(PREFIX+"original-independent.json")
run_status = load(PREFIX+"oct7-detached-status.json")
reconciled = load(PREFIX+"oct7-detached-reconciled.json")
quality_status = load(PREFIX+"oct7-quality-status.json")
if run_status["functional_exit"]!=0 or reconciled["status"]!="PASS" or reconciled["independent_status"]!="PASS":
    raise SystemExit("durable registered result has not been independently reconciled")
if quality_status["state"]!="COMPLETED":
    raise SystemExit("quality actions are not terminal")
first = load(PREFIX+"repaired-first-independent.json")
five = load(PREFIX+"oct7-five-independent.json")
first_full = load(PREFIX+"oct7-full-independent.json")
full = load(PREFIX+"oct7-detached-independent.json")
docs = load(PREFIX+"oct7-docs-final.json")
contract = load(PREFIX+"oct7-openapi.json")
repo = lint(PREFIX+"oct7-repo-clippy.log")
api = lint(PREFIX+"oct7-api-clippy.log")
phases = {key:result(PREFIX+name) for key,name in {
    "original_five":"original-test.log",
    "first_repair_five":"repaired-test.log",
    "single_concurrent_diagnostic":"concurrent-diagnostic.log",
    "current_five":"oct7-five-test.log",
    "password_budget_library":"oct7-budget-test.log",
    "complete_registered_first":"oct7-full-test.log",
    "complete_registered_current":"oct7-detached-test.log",
}.items()}
if original["errors"] or five["errors"] or full["errors"]:
    raise SystemExit("unexplained independent fact errors in a currently claimed phase")
if docs["status"]!="PASS" or contract["status"]!="PASS":
    raise SystemExit("current documentation or public login contract not verified")
current = phases["complete_registered_current"]
if current["status"]!="PASS" or current["ignored"] or current["filtered_out"]:
    raise SystemExit("complete registered regression not actually passed")
for key in ["current_five","password_budget_library"]:
    if phases[key]["status"]!="PASS" or phases[key]["ignored"]:
        raise SystemExit("required current login/resource contract missing: "+key)

artifacts = [PREFIX+name for name in [
    "original-test.log","original-independent.json","repaired-test.log","repaired-first-independent.json",
    "repaired-first-protected-drift.json","concurrent-diagnostic.log","oct7-five-test.log","oct7-five-independent.json",
    "oct7-budget-test.log","oct7-full-test.log","oct7-full-independent.json",
    "oct7-full-recheck-test.log","oct7-interrupted.json","oct7-detached-test.log",
    "oct7-detached-independent.json","oct7-detached-status.json","oct7-detached-events.jsonl","oct7-openapi-first.json",
    "oct7-detached-reconciled.json","oct7-api-package-format.log","oct7-quality-status.json",
    "oct7-openapi-gate.log","oct7-openapi.json","oct7-docs-final.json","oct7-repo-clippy.log","oct7-api-clippy.log",
    "oct7-format.log","oct7-format-repair.log","oct7-budget-format.log","oct7-format-final.log",
    "oct7-qa-start.log","capture.py","verify.py","drift.py","openapi-verify.mjs","report.py",
]]
artifacts += ["free-erp-inventory-consumers-session-pool-"+stage+"-source.json" for stage in [
    "original-before","original-after","repaired-before","repaired-after",
    "oct7-five-before","oct7-five-after","oct7-full-before","oct7-full-after",
    "oct7-four-before","oct7-four-after","oct7-full-recheck-before","oct7-detached-before","oct7-detached-after",
]]
source_names = [
    "crates/erp-repo/src/access_plan.rs","crates/erp-api/src/auth_handlers.rs",
    "crates/erp-api/tests/free_erp_acceptance.rs","crates/erp-api/tests/free_erp_acceptance/account_login.rs",
    "crates/erp-api/tests/free_erp_acceptance/session_pool.rs","crates/erp-api/tests/free_erp_acceptance/login_sessions.rs",
    "crates/erp-api/src/export_job_views.rs","crates/erp-api/src/inventory_print_report_handlers/reports_exports.rs",
    "openapi/src/paths.yaml","openapi/erp_api_v1.yaml",
]
report = {
    "utc":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL",
    "scope":"Single-connection real-password session admission, current credential lookup and bounded password verification; ongoing ERP goal",
    "database":{"name":"erp_shared_contract_test","role":"erp_grid_test","host":"127.0.0.1","port":15438,"selected_tables":69,"observed_isolation":"read committed"},
    "functional_results":phases,"independent_original":original,"independent_current_five":five,
    "independent_first_registered_full":first_full,"independent_current_full":full,
    "durable_run":run_status,"durable_result_reconciliation":reconciled,"quality_run":quality_status,
    "preserved_interrupted_run":load(PREFIX+"oct7-interrupted.json"),
    "preserved_first_repair_failure":{"status":first["status"],"checks":first["checks"],"older_enterprises":first["older_enterprises"],
        "older_enterprises_changed":len(first["older_enterprises_changed"]),"error_count":len(first["errors"]),
        "source_input_drift":first["source_input_drift"],"drift":load(PREFIX+"repaired-first-protected-drift.json")},
    "repairs":[
        "Session opening uses exactly one pooled transaction connection; remove the separate session-level guard and retain tenant-row/quota serialization",
        "After the tenant credential commit boundary reread current active user and the server-verified password hash; refuse stale validation before session effects",
        "Keep historical sessions, current-generation named reuse, the free one-seat/eight-session policy and transaction-only commit locks",
        "Run real password verification off the async executor, with at most 32 admitted tasks and two concurrent workers",
        "Owned permits follow an already-running password job until it actually finishes; queued cancellation returns its admission slot",
        "Publish login_busy 503 in both source and bundled OpenAPI; do not mislabel resource overload as bad credentials",
        "Ordinary completed export metadata retains its link; only stored exit-manifest files require exit delivery, and metadata database errors remain explicit",
        "Actual count fixture cutoff is sampled from the database after its stock commit; preserved sheet and sale facts no longer depend on a calendar literal",
        "Correct-password test retries respect the server-reported bounded authentication backoff; no security throttle bypass",
    ],
    "historical_diagnostic_boundary":"The single concurrent diagnostic passed before the background-hashing format record. It does not prove hashing caused every earlier 500 or qualify all five scenarios.",
    "validation_limits":[
        "HTTP handlers are called in process with real server-signed identity and PostgreSQL, not actual desktop/Web or public network",
        "Credential races use the shipped repository user/epoch transaction; complete password-recovery/current-user HTTP races are separate",
        "The queue unit runs genuine salted correct/incorrect password verification and cancellation of queued work; already-running cancellation peak load/RSS remains separate",
        "All durations are debug functional test durations, not P95/P99 or Release qualification",
        "Selected 69-table protection is exact within each before/after window; the first historical column-change failure remains FAIL",
    ],
    "public_contract":contract,"documentation":{"status":docs["status"],"checks":docs["checks"]},
    "strict_clippy":{"repo":repo,"api":api},
    "remaining_gates":{
        "all_business_normal_accounts":"PARTIAL","authorization_revoke_during_inventory_lock_wait":"NOT_COMPLETE",
        "complete_renewal_recovery_and_credentials_http":"PARTIAL","real_native_wasm_login_edit_save_recovery":"NOT_RUN",
        "complete_migration_chain":"NOT_RUN","strict_clippy":"PASS" if repo["status"]==api["status"]=="PASS" else "NOT_PASSED",
        "api_package_format":"FAIL",
        "source_stability":"PARTIAL" if full["source_input_drift"] else "PASS_FOR_THIS_WINDOW_ONLY",
        "release_password_stock_and_resource_performance":"NOT_RUN","official_hosting_release":"NOT_RUN",
    },
    "current_related_source_sha256":{name:sha(ROOT/name) for name in source_names},
    "artifact_sha256":{"qa/acceptance/"+name:sha(BASE/name) for name in artifacts},
}
with TARGET.open("x",encoding="utf8") as stream:
    stream.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"current_full":current,
                  "protected_older_enterprises":full["older_enterprises"],"source_input_drift":full["source_input_drift"],
                  "strict_clippy":{key:value["status"] for key,value in report["strict_clippy"].items()}},ensure_ascii=False))
