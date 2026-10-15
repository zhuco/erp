"""Join immutable regression, independent facts and actual quality-gate evidence."""
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


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def evidence(name):
    path = base / name
    if not path.is_file():
        raise SystemExit(f"missing evidence: {name}")
    return {"path": path.relative_to(root).as_posix(), "sha256": digest(path)}


full_name = "free-erp-inventory-consumers-refined-native-full.log"
full = decoded(base / full_name)
matched = re.findall(
    r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s", full
)
if not matched or tuple(map(int, matched[-1][:3])) != (101, 0, 0):
    raise SystemExit("101/0/0 registered acceptance required")
consumer_tests = re.findall(r"test inventory_consumers::([^\s]+) \.\.\. ok", full)
if len(consumer_tests) != 15:
    raise SystemExit("all 15 new consumer regressions required")
independent_name = "free-erp-inventory-consumers-independent.json"
independent = json.loads(decoded(base / independent_name))
if independent["status"] != "PASS" or independent["expected_registered_total"] != 101:
    raise SystemExit("final independent validation required")

before_name = "free-erp-inventory-consumers-refined-native-before-source.json"
after_name = "free-erp-inventory-consumers-refined-native-after-source.json"
before = json.loads(decoded(base / before_name))["files"]
after = json.loads(decoded(base / after_name))["files"]
changed = [name for name in sorted(set(before) | set(after)) if before.get(name) != after.get(name)]
now_changed = [name for name, sha in after.items()
               if not (root / name).is_file() or digest(root / name) != sha]
source_comparison = {
    "status": "PARTIAL" if changed or now_changed else "PASS",
    "scope": "All listed Rust, schema, lock, build-entry and OpenAPI inputs; shared root is an actual dependency",
    "inputs_before": len(before), "inputs_after": len(after),
    "same_in_test_window": len(set(before) & set(after)) - len([n for n in before.keys() & after.keys() if before[n] != after[n]]),
    "changed_in_test_window": changed,
    "changed_since_after_manifest": now_changed,
    "before_manifest": evidence(before_name), "after_manifest": evidence(after_name),
    "limits": "Observed test success is retained. Concurrent changes disqualify certification of the whole current target; no root dependency is excluded to manufacture PASS.",
}
comparison_name = "free-erp-inventory-consumers-source-consistency.json"
comparison_path = base / comparison_name
if comparison_path.exists():
    raise SystemExit("refusing to replace source comparison")

fmt_name = "free-erp-inventory-consumers-final-leaf-fmt.log"
if "PASS: fixed-entry check-only fmt" not in decoded(base / fmt_name):
    raise SystemExit("actual successful format receipt required")
clippy_name = "free-erp-inventory-consumers-strict-clippy.log"
clippy = decoded(base / clippy_name)
package_failures = {package + " " + target: int(count) for package, target, count in re.findall(
    r"could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors", clippy
)}
if not package_failures or "Cargo clippy failed" not in clippy:
    raise SystemExit("strict Clippy failure evidence missing")
leaf_names = ["inventory.rs", "inventory_precision.rs", "inventory_reads.rs", "inventory_turnover.rs",
              "reports.rs", "inventory_views.rs", "inventory_consumers.rs"]
leaf_diagnostics = []
for block in re.split(r"(?m)(?=^error: )", clippy):
    for name in leaf_names:
        if re.search(r"(?:-->|:::)\s+[^\n]*" + re.escape(name) + r":\d+", block):
            leaf_diagnostics.append({"file": name, "message": block.splitlines()[0]})
bundle_name = "free-erp-inventory-consumers-final-openapi-bundle.log"
gate_name = "free-erp-inventory-consumers-final-openapi-gate.log"
if "[openapi-bundle] Bundled" not in decoded(base / bundle_name) or "verification passed" not in decoded(base / gate_name):
    raise SystemExit("actual bundle and gate required")

owned_sources = [
    "crates/erp-repo/src/inventory.rs", "crates/erp-repo/src/inventory_precision.rs",
    "crates/erp-repo/src/inventory_reads.rs", "crates/erp-repo/src/inventory_turnover.rs",
    "crates/erp-repo/src/reports.rs", "crates/erp-repo/src/lib.rs",
    "crates/erp-repo/src/shared_contract.rs", "crates/erp-api/src/inventory_views.rs",
    "crates/erp-api/src/inventory_print_report_handlers/reports_exports.rs",
    "crates/erp-api/src/lib.rs", "crates/erp-api/tests/free_erp_acceptance.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_consumers.rs",
]
artifact_paths = re.findall(r"Running tests\\free_erp_acceptance\.rs \(([^)]+\.exe)\)", full)
artifact = None
if artifact_paths:
    artifact_path = root / artifact_paths[-1].replace("\\", "/")
    if artifact_path.is_file():
        artifact = {"path": artifact_path.relative_to(root).as_posix(), "sha256": digest(artifact_path),
                    "evidence_kind": "Post-run observation only; no exclusive executable/current-source certification"}

report = {
    "updated_at": datetime.now(timezone.utc).isoformat(), "status": "PARTIAL",
    "scope": "Exact single-balance readers, original whole-unit write/reservation receipts and authorized posted-shipment stock-cover report",
    "product": "Official-hosted free manufacturing and trade; one business login, concurrent multi-device sessions; paid advanced/seat capabilities later",
    "changes": [
        "InventoryRepo and shared single-balance readers delegate to the exact read path; reads do not activate unknown history",
        "Compatibility available is projected after exact reserved/frozen subtraction; original adjustment receipts include committed precise quantities",
        "Original reservation repeats return precise current balances; terminal replay checks original warehouse/SKU/quantity before returning",
        "GET /reports/inventory requires both permissions and applies warehouse grants to count, stock and posted shipment sources in one read snapshot",
        "Decimal warning/out-of-stock compare exact quantities; unknown stock refuses before paging; unclassified sales remain null",
        "Gross SALES_SHIP amounts replace order demand; schema/basis/snapshot disclose current-stock cover rather than average-stock turnover",
    ],
    "registered_acceptance": {"status": "PASS", "passed": 101, "failed": 0, "ignored": 0,
        "seconds": float(matched[-1][3]), "kind": "Debug functional regression; not a performance result",
        "new_consumer_tests": consumer_tests, "log": evidence(full_name),
        "authentication": "Synthetic JWT with stored QA user/warehouse grants; no real password/seat/session qualification"},
    "independent_facts": {k: independent[k] for k in ["status", "checks", "old_tenants_unchanged",
        "new_consumer_tenants", "canonical_keys", "unknown_keys_not_activated", "http_results"]},
    "independent_report": evidence(independent_name),
    "protected_database": {"database": "erp_shared_contract_test", "role": "erp_grid_test", "host": "127.0.0.1", "port": 15438,
        "before": evidence("free-erp-inventory-consumers-full-before-qa.json"),
        "after": evidence("free-erp-inventory-consumers-full-after-qa.json"),
        "operations": "Additive random QA fixtures and missing original baseline tables only; no reset, no business-data correction or production migration"},
    "source_certification": source_comparison,
    "current_source_sha256": {name: digest(root / name) for name in owned_sources},
    "post_run_executable_observation": artifact,
    "quality_gates": {
        "affected_leaf_format": {"status": "PASS", "files": 7, "write": False, "log": evidence(fmt_name)},
        "strict_backend_clippy": {"status": "FAIL", "package_error_counts": package_failures,
            "affected_leaf_diagnostics_observed": leaf_diagnostics, "log": evidence(clippy_name),
            "limits": "No global suppression. Dependency failures prevented completion/certification of API lint; absence of a leaf diagnostic is not package PASS."},
        "openapi": {"status": "PASS", "bundle": evidence(bundle_name), "gate": evidence(gate_name)},
    },
    "preserved_counterexamples": {
        "before_balance": {"log": evidence("free-erp-inventory-consumers-before-repair.log"),
            "finding": "2 actual available projection failures; 2 early report errors also exposed missing original orders/order_items QA tables"},
        "before_report": {"log": evidence("free-erp-inventory-consumers-before-report.log"),
            "finding": "After additive original-table foundation, actual HTTP leaked the second warehouse and reported fractional availability as 1"},
        "before_terminal_identity": {"log": evidence("free-erp-inventory-consumers-before-terminal-identity.log"),
            "finding": "Released quantity 1 replayed as 2 was accepted; reproduced 0/1 before moving identity checks"},
        "interrupted_full": {"log": evidence("free-erp-inventory-consumers-full-native-interrupted.log"),
            "finding": "QA connection refusal invalidated before readback; this task stopped its own unfinished test session. This run has no passing result. QA subsequently recovered without this task restarting PostgreSQL."},
    },
    "remaining": [
        "Classify shared trade shipments/returns by authoritative posted source; implement net sales and average-inventory turnover separately",
        "Qualify all shared/WMS/batch/location/quality write entries and whole legacy-request idempotency/audit atomicity",
        "Full migration/upgrade, all-null historical ledgers, mapped-channel outbox and durable queue/lease/crash recovery",
        "Real password, free one-seat enforcement and concurrent multi-device sessions",
        "Formal native/Web inventory/report interaction, actual file saving and independent reopened file readback",
        "ERP-P02/P03/P14/P16 Release loads, P95/P99, memory/CPU/pool/SQL plans and timeout/correctness gates",
        "Strict package quality gates, whole current target source certification and official public hosting/recovery",
    ],
    "feature_completion": "I01/I05/R01/R03 and free release remain unchecked; this evidence certifies only the stated observed behavior",
}

destination = base / "free-erp-inventory-consumers-repair.json"
archive = base / "free-erp-inventory-consumers-provisional.json"
if archive.exists():
    raise SystemExit("refusing to replace provisional history")
archive.write_bytes(destination.read_bytes())
comparison_path.write_text(json.dumps(source_comparison, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "passed": 101, "new_tests": len(consumer_tests),
    "independent_checks": independent["checks"], "source_certification": source_comparison["status"],
    "changed_in_test_window": changed, "clippy": package_failures,
    "affected_leaf_diagnostics": leaf_diagnostics}, ensure_ascii=False))
