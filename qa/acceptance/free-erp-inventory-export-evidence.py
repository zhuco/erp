"""Bind export repair evidence to actual logs, facts and source observations."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"

def read(name):
    return (base / name).read_text(encoding="utf-8-sig")

def load(name):
    return json.loads(read(name))

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def tests(name):
    text = read(name)
    match = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored.*?finished in ([0-9.]+)s", text)
    if not match:
        raise SystemExit(f"No completed test result: {name}")
    return {"status":"PASS" if match[1]=="ok" and match[3]==match[4]=="0" else "FAIL",
        "passed":int(match[2]),"failed":int(match[3]),"ignored":int(match[4]),"seconds":float(match[5]),
        "log":f"qa/acceptance/{name}","log_sha256":digest(base/name)}

owned = [
    "crates/erp-repo/src/inventory_export_jobs.rs", "crates/erp-repo/src/inventory_export_cursor.rs",
    "crates/erp-repo/src/inventory_reads.rs", "crates/erp-repo/src/export_jobs.rs", "crates/erp-repo/src/lib.rs",
    "crates/erp-worker/src/jobs/inventory_export.rs", "crates/erp-worker/src/jobs/export_jobs.rs", "crates/erp-worker/src/jobs/mod.rs",
    "crates/erp-api/src/inventory_export_access.rs", "crates/erp-api/src/export_file_stream.rs",
    "crates/erp-api/src/inventory_print_report_handlers/reports_exports.rs", "crates/erp-api/Cargo.toml",
    "crates/erp-api/tests/free_erp_acceptance.rs", "crates/erp-api/tests/free_erp_acceptance/inventory_exports.rs", "Cargo.lock",
]
intervals = []
for stage in ["initial-native", "refined-native", "lint-repair"]:
    before = load(f"free-erp-inventory-export-{stage}-before-source.json")["files"]
    after = load(f"free-erp-inventory-export-{stage}-after-source.json")["files"]
    changed = [key for key in sorted(before.keys()|after.keys()) if before.get(key)!=after.get(key)]
    intervals.append({"stage":stage,"input_count":len(before),"status":"PARTIAL" if changed else "PASS",
        "changed":changed,"owned_files_changed":[key for key in changed if key in owned],
        "before_manifest":f"qa/acceptance/free-erp-inventory-export-{stage}-before-source.json",
        "after_manifest":f"qa/acceptance/free-erp-inventory-export-{stage}-after-source.json"})

clippy_name = "free-erp-inventory-export-lint-repaired-worker-clippy.log"
clippy = read(clippy_name)
diagnostics = {crate:int(count) for crate,count in re.findall(r"could not compile `([^`]+)` \(lib\) due to (\d+) previous errors",clippy)}
if diagnostics.get("erp-repo") != 234:
    raise SystemExit("Unexpected latest repository Clippy result; inspect before reporting")
before_permission = read("free-erp-inventory-export-before-permission-route.log")
assert "status=200 OK" in before_permission and "inventory source permission required" in before_permission

observed = load("free-erp-inventory-export-refined-executable-observed.json")
last_log = read("free-erp-inventory-export-lint-repair-affected.log")
exe_match = re.search(r"Running .*?\((build\\cargo\\debug\\deps\\[^)]+\.exe)\)",last_log)
if not exe_match:
    raise SystemExit("Latest completed target executable was not identified")
exe_path = root / exe_match[1]
latest_executable = {"path":exe_path.relative_to(root).as_posix(),"sha256":digest(exe_path),"bytes":exe_path.stat().st_size,
    "observation":"after run; source interval changed; not an exclusive executable attestation"}

independent = load("free-erp-inventory-export-independent.json")
latest_independent = load("free-erp-inventory-export-lint-repair-independent.json")
docs = load("free-erp-specification-docs.json")
assert independent["status"]==latest_independent["status"]==docs["status"]=="PASS"
checks = []
for name,scope in [
    ("free-erp-inventory-export-worker-check.log","initial native Worker and dependencies"),
    ("free-erp-inventory-export-final-backend-check.log","configured native backend packages and dependencies"),
    ("free-erp-inventory-export-final-worker-check.log","configured native Worker and dependencies; observed before lint-only normalization repair"),
]:
    text = read(name)
    assert re.search(r"Finished `dev` profile",text) and "could not compile" not in text
    checks.append({"status":"PASS","scope":scope,"log":f"qa/acceptance/{name}",
        "cache_note":"incremental session finalization access denied" if "did not finalize incremental compilation" in text else None})

for name in ["free-erp-inventory-export-final-openapi.log", "free-erp-inventory-export-final-leaf-fmt.log", "free-erp-inventory-export-final-worker-leaf-fmt.log"]:
    text = read(name)
    assert "Cargo fmt failed" not in text and "Exception:" not in text
assert "OpenAPI gate verification passed" in read("free-erp-inventory-export-final-openapi.log")

report = {
    "updated_at":datetime.now(timezone.utc).isoformat(), "status":"PARTIAL",
    "scope":"Existing inventory export job admission, Worker snapshot CSV, durable artifact receipt and authorized streaming download; no new business UI or production deployment",
    "original_counterexample":{"status":"REPRODUCED_AND_REPAIRED","http_before":200,"http_after":403,
        "permission":"exports.write without inventory.read","log":"qa/acceptance/free-erp-inventory-export-before-permission-route.log",
        "instrumentation_note":"First test used an unregistered tenant-prefixed URL and produced an empty 404 body; it was corrected before preserving this real-route counterexample"},
    "registered_acceptance":tests("free-erp-inventory-export-refined-native-full.log"),
    "initial_focused_acceptance":tests("free-erp-inventory-export-initial-native.log"),
    "latest_after_lint_repair":tests("free-erp-inventory-export-lint-repair-affected.log"),
    "stream_and_existing_product_guards":tests("free-erp-inventory-export-stream-unit.log"),
    "independent_initial":{"status":independent["status"],"checks":independent["checks"],"old_tenants_unchanged":independent["old_tenants_unchanged"],"report":"qa/acceptance/free-erp-inventory-export-independent.json"},
    "independent_latest":{"status":latest_independent["status"],"checks":latest_independent["checks"],"old_tenants_unchanged":latest_independent["old_tenants_unchanged"],"new_tenants":latest_independent["new_export_tenants"],"canonical_keys":latest_independent["canonical_keys"],"unknown_keys_not_activated":latest_independent["unknown_keys_not_activated"],"report":"qa/acceptance/free-erp-inventory-export-lint-repair-independent.json"},
    "source_certification":{"status":"PARTIAL","intervals":intervals,"current_owned_hashes":{name:digest(root/name) for name in owned},"limits":"Concurrent edits to dependency/contract inputs remain; stable owned leaves alone do not certify the entire target"},
    "executable_observations":{"full_registered":observed,"latest_focused":latest_executable},
    "configured_compile_checks":checks,
    "strict_clippy":{"status":"FAIL","log":f"qa/acceptance/{clippy_name}","crate_errors":diagnostics,
        "owned_new_repo_leaf_diagnostics":re.findall(r"inventory_export_\w+\.rs:\d+",clippy),"worker_lint":"NOT_REACHED",
        "repair":"One newly introduced manual_flatten diagnostic was repaired; latest 18-case target reverified"},
    "selected_leaf_format":{"status":"PASS","files":9,"module_roots_and_external_module_parents":"not formatted by the leaf entry; small registration edits inspected separately"},
    "openapi":{"status":"PASS","log":"qa/acceptance/free-erp-inventory-export-final-openapi.log","source":"openapi/src/components.yaml and paths.yaml","bundle_sha256":digest(root/"openapi/erp_api_v1.yaml")},
    "documentation":{"status":docs["status"],"report":"qa/acceptance/free-erp-specification-docs.json","checks":docs["checks"],"rust_builds_triggered_by_doc_check":0},
    "implemented_limits":{"matching_rows":1000000,"csv_bytes":1073741824,"warehouse_scope":10000,"fetch_rows":1000,"sku_code_utf8_bytes":1024,"encode_chunk_bytes":8388608,"download_chunk_bytes":65536,"sql_statement_seconds":10,"cursor_idle_seconds":30,"generation_or_retry_verification_seconds":300},
    "large_functional_sample":{"rows":1002,"bytes":150386,"download_chunks":3,"performance_gate":"NOT_RUN: this sample and debug durations do not prove ERP-P16"},
    "not_passed":["whole target exclusive source/executable certification","strict backend/API/Worker Clippy","real password, free-seat and concurrent-session acceptance","real native/Web inventory task creation, digest-aware download and local file readback","registered Release inventory export workload, per-stage timing and peak memory","global queue/tenant fairness, storage/retention quota and real RUNNING crash reclamation","unpublished prepared-candidate retention/recovery; current temp cleanup permits a new generation snapshot on retry","authorization revocation after bytes start transferring; previous bytes cannot be recalled","actual crash/IO/DB failure injection at publication and audit admission","object-store publication, public hosting, backup/restore and complete production migration chain","remaining turnover/get_balance/legacy write-response precision and full free ERP feature closure"],
    "qa_protection":{"database":"127.0.0.1:15438/erp_shared_contract_test","role":"erp_grid_test","foundation":"additive original RBAC/flag/audit/export DDL only; no fixture reset/backfill or global queue execution","files":"only own UUID tenant/job output and owned temporary files; deliberately corrupt fixture retained"},
}
destination=base/"free-erp-inventory-export-repair.json"
provisional=base/"free-erp-inventory-export-provisional.json"
if not provisional.exists():
    provisional.write_bytes(destination.read_bytes())
destination.write_text(json.dumps(report,indent=2,ensure_ascii=False)+"\n",encoding="utf8")
print(json.dumps({"status":report["status"],"full_tests":report["registered_acceptance"]["passed"],"latest_tests":report["latest_after_lint_repair"]["passed"],"independent_checks":latest_independent["checks"],"clippy":diagnostics,"docs":docs["status"]},ensure_ascii=False))
