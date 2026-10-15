"""Bind mutation regression facts, source manifests and actual quality gates."""
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
        raise SystemExit(f"missing evidence: {name}")
    return {"path": path.relative_to(root).as_posix(), "sha256": sha(path)}


full_name = "free-erp-inventory-mutations-full-native.log"
focused_name = "free-erp-inventory-mutations-focused.log"
full = decoded(base / full_name)
matches = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s", full)
if not matches or tuple(map(int, matches[-1][:3])) != (104, 0, 0):
    raise SystemExit("104/0/0 registered acceptance required")
if "test result: ok. 3 passed; 0 failed; 0 ignored;" not in decoded(base / focused_name):
    raise SystemExit("focused 3/0/0 required")
independent_name = "free-erp-inventory-mutations-independent-normalized.json"
independent = json.loads(decoded(base / independent_name))
if independent["status"] != "PASS" or independent["expected_registered_total"] != 104:
    raise SystemExit("independent public SQL facts must pass")
counter_name = "free-erp-inventory-mutations-before-repair.log"
counter = decoded(base / counter_name)
if "test result: FAILED. 0 passed; 2 failed;" not in counter or '"on_hand_micros":3900000' not in counter:
    raise SystemExit("actual original partial-commit counterexamples required")

comparisons = {}
for stage in ["fixed", "full"]:
    before_name = f"free-erp-inventory-consumers-mutations-{stage}-before-source.json"
    after_name = f"free-erp-inventory-consumers-mutations-{stage}-after-source.json"
    before = json.loads(decoded(base / before_name))["files"]
    after = json.loads(decoded(base / after_name))["files"]
    changed = [n for n in sorted(set(before) | set(after)) if before.get(n) != after.get(n)]
    current_changed = [n for n, digest in after.items() if not (root / n).is_file() or sha(root / n) != digest]
    comparisons[stage] = {"status": "PARTIAL" if changed or current_changed else "PASS",
        "scope": "All listed Rust/schema/lock/build/OpenAPI inputs, including actual shared root dependencies",
        "inputs_before": len(before), "inputs_after": len(after),
        "unchanged_in_run": len(set(before) & set(after)) - len([n for n in before.keys() & after.keys() if before[n] != after[n]]),
        "changed_in_run": changed, "changed_since_after_manifest": current_changed,
        "before": evidence(before_name), "after": evidence(after_name)}

fmt_name = "free-erp-inventory-mutations-final-fmt.log"
if "PASS: fixed-entry check-only fmt" not in decoded(base / fmt_name):
    raise SystemExit("actual fixed-entry format receipt required")
clippy_name = "free-erp-inventory-mutations-strict-clippy.log"
clippy = decoded(base / clippy_name)
failures = {p + " " + target: int(count) for p, target, count in re.findall(
    r"could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors", clippy)}
if not failures or "Cargo clippy failed" not in clippy:
    raise SystemExit("strict package failure evidence missing")
owned = ["crates/erp-repo/src/inventory.rs", "crates/erp-repo/src/audit_logs.rs",
    "crates/erp-api/src/inventory_print_report_handlers/inventory_core.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_mutation_atomicity.rs"]
diagnostics = []
for block in re.split(r"(?m)(?=^error: )", clippy):
    for name in owned:
        if re.search(r"(?:-->|:::)\s+" + re.escape(name.replace("/", "\\")) + r":\d+", block):
            diagnostics.append({"file": name, "message": block.splitlines()[0]})
bundle_name = "free-erp-inventory-mutations-openapi-bundle.log"
gate_name = "free-erp-inventory-mutations-openapi-gate.log"
if "[openapi-bundle] Bundled" not in decoded(base / bundle_name) or "verification passed" not in decoded(base / gate_name):
    raise SystemExit("actual OpenAPI gate required")

report = {
    "updated_at": datetime.now(timezone.utc).isoformat(), "status": "PARTIAL",
    "scope": "Original integer adjustment/inbound/outbound API branches: stock and required operation/audit records commit together",
    "actual_routes": [f"POST /v1/tenants/{{tenant_id}}/inventory/{action}" for action in ["adjustments", "inbounds", "outbounds"]],
    "counterexample": {"status": "REPRODUCED", "passed": 0, "failed": 2, "log": evidence(counter_name),
        "finding": "Actual operation/audit INSERT failures returned HTTP 500 after physical stock changed 2.9 to 3.9; audit failure also left an operation log. Failed QA samples are preserved."},
    "implementation": {
        "stock": "InventoryRepo::apply_adjustment_with_tx leaves commit to the API caller and shares the given transaction with stock, ledger and batch write",
        "operation": "OperationLogsRepo::insert_with_tx uses the same connection",
        "audit": "AuditLogsRepo::insert_with_tx shares normalization with the original Pool wrapper and uses the same connection",
        "api": "apply_inventory_movement commits once after all required writes; before-commit errors roll back; both logs contain authenticated actor, physical identity, batch_id and precise transaction quantities",
        "source_sha256": {name: sha(root / name) for name in owned},
    },
    "registered_acceptance": {"status": "PASS", "passed": 104, "failed": 0, "ignored": 0,
        "seconds": float(matches[-1][3]), "log": evidence(full_name), "kind": "Debug functional regression, not Release load/performance"},
    "focused_acceptance": {"status": "PASS", "passed": 3, "failed": 0, "ignored": 0,
        "seconds": 2.07, "log": evidence(focused_name), "actual_http": 9,
        "failures": "3 routes x operation/audit INSERT refusal = 6; independent connection sees no changed business fact",
        "successes": "3 routes; 2 stock ledgers including opening, 1 operation and 1 audit; exact committed quantities and actor identities agree"},
    "independent_facts": {k: independent[k] for k in ["status", "checks", "old_tenants_unchanged", "new_mutation_tenants", "http_results", "actual_log_refusals", "successes"]},
    "independent_report": evidence(independent_name),
    "initial_verifier_report": {"report": evidence("free-erp-inventory-mutations-independent.json"),
        "finding": "First verifier FAIL was six exact-text timestamp comparisons. SQLx UTC and psql +08:00 represented the same instants. Corrected verifier normalizes only known timestamp fields; raw facts and all business fields are retained."},
    "protected_qa": {"database": "erp_shared_contract_test", "role": "erp_grid_test", "host": "127.0.0.1", "port": 15438,
        "before": evidence("free-erp-inventory-mutations-before-qa.json"), "after": evidence("free-erp-inventory-mutations-after-qa.json"),
        "fault_injection": "Additive owned random schema/view + request-local search_path; no public table/trigger changes or other-tenant interruption",
        "limits": "Synthetic JWT with stored actor/grant; no real password/seat/session, batch or mapped-channel qualification"},
    "source_certification": comparisons,
    "quality_gates": {
        "affected_leaf_format": {"status": "PASS", "files": 4, "write": False, "log": evidence(fmt_name)},
        "strict_backend_clippy": {"status": "FAIL", "package_error_counts": failures, "affected_leaf_diagnostics_observed": diagnostics,
            "log": evidence(clippy_name), "limits": "No suppression. Dependency failure prevents API lint completion; no leaf diagnostic is not package PASS."},
        "openapi": {"status": "PASS", "bundle": evidence(bundle_name), "gate": evidence(gate_name)},
    },
    "original_sources": {name: evidence("free-erp-inventory-mutations-original-" + Path(name).name + ".txt") for name in owned[:3]},
    "remaining": [
        "Original command identity/full-payload idempotency, original result snapshots, commit-result uncertainty and durable result lookup",
        "Atomic audit/operation facts for every shared, WMS, manufacturing and other stock-write service",
        "Actual batch identity/precision/late-failure paths and actual mapped-channel event enqueue/delivery",
        "Real login/one-seat/multi-device, formal native/Web and saved/reopened file acceptance",
        "Whole migration/upgrade/recovery, strict package gates and whole-current-source certification",
        "ERP-P04/P05 Release load, lock/pool/log/commit segments, P95/P99 and resource budgets; public hosting",
    ],
    "completion": "I01/I02/O01 and free release remain unchecked; tests certify only stated observed legacy movement branches",
}
destination = base / "free-erp-inventory-mutations-repair.json"
if destination.exists():
    raise SystemExit("refusing to replace repair evidence")
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "functional": 104, "independent": independent["checks"],
    "old_tenants_unchanged": independent["old_tenants_unchanged"], "full_source": comparisons["full"]["status"],
    "changed_inputs": comparisons["full"]["changed_in_run"], "clippy": failures, "owned_diagnostics": diagnostics}, ensure_ascii=False))
