"""Bind observed original-command acceptance to immutable inputs and quality gates."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"


def decoded(path):
    data = path.read_bytes()
    return data.decode("utf-16" if data.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def evidence(name):
    path = base / name
    if not path.is_file():
        raise SystemExit(f"missing evidence {name}")
    return {"path": path.relative_to(root).as_posix(), "sha256": sha(path)}


def passed(name, expected):
    matches = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s", decoded(base / name))
    if not matches or tuple(map(int, matches[-1][:3])) != (expected, 0, 0):
        raise SystemExit(f"actual {expected}/0/0 acceptance required: {name}")
    return {"status": "PASS", "passed": expected, "failed": 0, "ignored": 0,
            "debug_seconds": float(matches[-1][3]), "log": evidence(name)}


counter_name = "free-erp-inventory-commands-before-repair.log"
if "0 passed; 2 failed; 0 ignored" not in decoded(base / counter_name):
    raise SystemExit("original two HTTP/stock counterexamples required")
edge_failure_name = "free-erp-inventory-commands-edges-focused.log"
if "11 passed; 1 failed; 0 ignored" not in decoded(base / edge_failure_name):
    raise SystemExit("original response-cache counterexample required")
independent_name = "free-erp-inventory-commands-independent.json"
independent = json.loads(decoded(base / independent_name))
if independent["status"] != "PASS" or independent["failures"]:
    raise SystemExit("independent original effects required")

owned = [
    "crates/erp-repo/src/inventory_movement_commands.rs", "crates/erp-repo/src/lib.rs",
    "crates/erp-repo/Cargo.toml", "crates/erp-api/src/lib.rs", "crates/erp-api/src/router.rs",
    "crates/erp-api/src/inventory_print_report_handlers.rs",
    "crates/erp-api/src/inventory_print_report_handlers/inventory_commands.rs",
    "crates/erp-api/src/inventory_print_report_handlers/inventory_core.rs",
    "crates/erp-api/src/inventory_print_report_handlers/stock_movement.rs",
    "crates/erp-api/src/middleware/idempotency.rs",
    "crates/erp-api/tests/free_erp_acceptance.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_commands.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_command_edges.rs",
    "migrations/0284_inventory_movement_commands.sql", "Cargo.lock",
]
comparisons = {}
for stage in ("commands-focused", "commands-edges", "commands-refined-focused", "commands-full"):
    before_name = f"free-erp-inventory-consumers-{stage}-before-source.json"
    after_name = f"free-erp-inventory-consumers-{stage}-after-source.json"
    before = json.loads(decoded(base / before_name))["files"]
    after = json.loads(decoded(base / after_name))["files"]
    changed = [p for p in sorted(before.keys() | after.keys()) if before.get(p) != after.get(p)]
    drift = [p for p, digest in after.items() if not (root / p).is_file() or sha(root / p) != digest]
    owned_changed = [p for p in owned if p in changed]
    owned_drift = [p for p in owned if p in drift]
    comparisons[stage] = {"status": "PARTIAL" if changed or drift else "PASS",
        "before_inputs": len(before), "after_inputs": len(after), "changed_in_run": changed,
        "changed_since_after": drift, "owned_changed_in_run": owned_changed,
        "owned_changed_since_after": owned_drift, "before": evidence(before_name), "after": evidence(after_name),
        "limits": "Every listed root dependency is retained. Concurrent source changes disqualify whole-current-target certification even when the observed test binary passes."}

fmt_name = "free-erp-inventory-commands-final-fmt.log"
if "PASS: fixed-entry check-only fmt, seven affected leaves" not in decoded(base / fmt_name):
    raise SystemExit("actual fixed-entry leaf format check required")
clippy_name = "free-erp-inventory-commands-strict-clippy.log"
clippy = decoded(base / clippy_name)
failures = {package + " " + target: int(count) for package, target, count in re.findall(
    r"could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors", clippy)}
if failures and "Cargo clippy failed" in clippy:
    clippy_status = "FAIL"
    clippy_reason = "Strict warnings-as-errors package checks failed; no suppression."
elif "ERP cache exceeds 80 GiB" in clippy or "Build lock wait exceeded 10 minutes" in clippy:
    clippy_status = "NOT_RUN"
    clippy_reason = "Fixed-entry admission or existing build lock prevented compiler execution; no bypass or cache cleanup."
else:
    raise SystemExit("Clippy attempt must have a complete actual result")
leaf_names = ["inventory_movement_commands.rs", "inventory_commands.rs", "inventory_core.rs",
              "stock_movement.rs", "idempotency.rs", "inventory_command_edges.rs"]
diagnostics = []
for block in re.split(r"(?m)(?=^error: )", clippy):
    for name in leaf_names:
        if re.search(r"(?:-->|:::)\s+[^\n]*" + re.escape(name) + r":\d+", block):
            diagnostics.append({"file": name, "message": block.splitlines()[0]})

library_initial_name = "free-erp-inventory-commands-middleware-library.log"
library_blocked_name = "free-erp-inventory-commands-middleware-library-refined.log"
library_final_name = "free-erp-inventory-commands-middleware-library-final.log"
if (base / library_final_name).is_file():
    library = passed(library_final_name, 3)
else:
    library = {"status": "NOT_RUN", "reason": "Original filter matched zero tests; corrected filter was refused by fixed cache admission.",
               "blocked_log": evidence(library_blocked_name)}
library["initial_zero_test_attempt"] = evidence(library_initial_name)
library["initial_admission_refusal"] = evidence(library_blocked_name)

bundle_name = "free-erp-inventory-commands-final-openapi-bundle.log"
gate_name = "free-erp-inventory-commands-final-openapi-gate.log"
if "[openapi-bundle] Bundled" not in decoded(base / bundle_name) or "verification passed" not in decoded(base / gate_name):
    raise SystemExit("actual final OpenAPI bundle and gate required")
openapi_inputs_name = "free-erp-inventory-commands-final-openapi-inputs.json"
openapi_inputs = json.loads(decoded(base / openapi_inputs_name))
openapi_drift = [p for p, digest in openapi_inputs["after"].items() if sha(root / p) != digest]
catalog_name = "free-erp-inventory-commands-migration-readback.json"
catalog = json.loads(decoded(base / catalog_name))
if (catalog["database"], catalog["role"], catalog["host"], catalog["port"]) != (
        "erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438):
    raise SystemExit("guarded migration catalog required")
column_names = {column["name"] for column in catalog["columns"]}
if not {"movement_kind", "actor_user_id", "warehouse_id", "sku_id", "original_result"}.issubset(column_names) or not all(
        catalog[k] for k in ("constraint", "trigger", "guard")):
    raise SystemExit("actual movement migration columns/check/guard required")

report = {"status": "PARTIAL", "updated_at": datetime.now(timezone.utc).isoformat(),
    "scope": "Original adjustment/inbound/outbound commands without batch/location/quality dimensions: shared identity, full payload, exact effect, required logs, immutable original snapshot and durable readback.",
    "actual_routes": [f"POST /v1/tenants/{{tenant_id}}/inventory/{a}" for a in ("adjustments", "inbounds", "outbounds")]
        + ["GET /v1/tenants/{tenant_id}/inventory/commands/{request_id}"],
    "counterexamples": [
        {"passed": 0, "failed": 2, "status": "REPRODUCED", "log": evidence(counter_name),
         "finding": "Same original outbound ID physically deducted 4.9 to 3.9 to 2.9; inbound replay replaced original 5.9 with later 6.9. Failed QA fixtures remain unchanged."},
        {"passed": 11, "failed": 1, "status": "REPRODUCED", "log": evidence(edge_failure_name),
         "finding": "Legacy header response-cache lookup ran before the command handler. Duplicate header produced 500 rather than command validation; three original movement routes now own transactional idempotency."}],
    "initial_fixed": passed("free-erp-inventory-commands-focused.log", 2),
    "refined_focused": passed("free-erp-inventory-commands-refined-focused.log", 12),
    "registered_native": passed("free-erp-inventory-commands-full-native.log", 116),
    "independent": {"status": "PASS", "checks": independent["checks"],
        "existing_tenants_unchanged": independent["existing_tenants_unchanged"],
        "new_command_fixtures": independent["new_command_fixtures"], "counts": independent["counts"],
        "edge_http": independent["counts"]["edge_http"], "original_regression_http": 5,
        "log": evidence(independent_name), "verifier": evidence("free-erp-inventory-commands-verify.py")},
    "implementation": {"atomic": "Claim shared command identity before stock locks; stock/ledger/service Outbox/two logs/original snapshot commit once; failure rolls back the claim.",
        "binding": "Versioned SHA256 binds normalized tenant, original actor/ID, kind, warehouse/SKU, whole quantity, reason and reference.",
        "snapshot": "Committed original receipt is immutable. Replay verifies original ledger/log IDs and exact quantities; later stock never substitutes the original balance or timestamp.",
        "readback": "Fresh read-only lookup requires inventory.read, original actor and current warehouse authorization; missing/invisible=404, revoked=403, incomplete/inconsistent=409.",
        "migration": "Nullable metadata extends existing shared receipts without inventing legacy actor/result; successful new movement rows reject update/delete.",
        "cache": "Three canonical movement routes bypass legacy response caching; invalid IDs never silently fall back to no-identity movement.",
        "source_sha256": {p: sha(root / p) for p in owned}},
    "source_certification": comparisons,
    "quality_gates": {"affected_leaf_fmt": {"status": "PASS", "files": 7, "write": False, "log": evidence(fmt_name)},
        "legacy_cache_family_library": library,
        "strict_backend_clippy": {"status": clippy_status, "reason": clippy_reason,
            "package_error_counts": failures, "affected_leaf_diagnostics_observed": diagnostics,
            "log": evidence(clippy_name), "limits": "No observed leaf diagnostic does not establish package PASS; blocked compiler execution is NOT_RUN."},
        "openapi": {"status": "PARTIAL" if openapi_drift or not openapi_inputs["source_stable_in_window"] else "PASS",
            "bundle": evidence(bundle_name), "gate": evidence(gate_name), "inputs": evidence(openapi_inputs_name), "changed_since_after": openapi_drift}},
    "protected_qa": {"database": "erp_shared_contract_test", "role": "erp_grid_test", "host": "127.0.0.1", "port": 15438,
        "migration": evidence("free-erp-inventory-commands-migration.sql"),
        "migration_catalog": evidence(catalog_name), "migration_catalog_sql": evidence("free-erp-inventory-commands-migration-readback.sql"),
        "migration_limits": "The guarded quiet migration returned tool exit 0 without text output; no text log was created. Read-only catalog and real snapshot immutability tests prove applied objects; this is not a whole upgrade certificate.",
        "readback_sql": evidence("free-erp-inventory-commands-readback.sql"),
        "before": evidence("free-erp-inventory-commands-qa-before-full.json"), "after": evidence("free-erp-inventory-commands-qa-after-full.json"),
        "faults": "Six real INSERT failures use owned random schema/views and request-local search_path; no public log table/trigger rewrite."},
    "cache_admission": {"earlier_refusal": evidence(library_blocked_name), "samples": [evidence("free-erp-inventory-commands-cache-admission.json"), evidence("free-erp-inventory-commands-cache-admission-corrected.json")],
        "scope": "Historical entry refusal and later read-only measurements are separate times. This task did not clean, move, create another cache or kill another build."},
    "remaining": ["Real login, one-seat/multi-device, session revocation and formal native/Web checkpoint recovery.",
        "Actual network failure at commit boundaries; discarded HTTP response is only a durable server lookup test.",
        "Exact decimal input and actual batch/location/quality commands, full WMS/manufacturing/reservation/worker audit migration.",
        "Real mapped-channel Outbox delivery and whole schema upgrade/restore/reconciliation.",
        "ERP-P04/P05 100-round Release load, separate first/replay/read distributions, lock/pool/commit timing and resource budgets.",
        "Strict package gates, stable full-target input certification and public hosting."],
    "completion": "Original commands in the stated scope have observed functional and independent PASS. I01/I02/O01/O02, full free product and launch remain PARTIAL.",
}
destination = base / "free-erp-inventory-commands-repair.json"
if destination.exists():
    raise SystemExit("refusing to overwrite command repair evidence")
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "registered": 116, "independent": independent["checks"],
    "old_tenants_unchanged": independent["existing_tenants_unchanged"], "whole_source": comparisons["commands-full"]["status"],
    "changed_full_inputs": comparisons["commands-full"]["changed_in_run"], "clippy": clippy_status,
    "package_failures": failures, "owned_diagnostics": diagnostics, "library": library["status"],
    "openapi": report["quality_gates"]["openapi"]["status"]}, ensure_ascii=False))
