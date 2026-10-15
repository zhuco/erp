"""Immutable observed repair evidence, including every failed and unpassed gate."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa" / "acceptance"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def decoded(name):
    data = (base / name).read_bytes()
    return data.decode("utf-16" if data.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")


def read(name):
    return json.loads(decoded(name))


def evidence(name):
    return {"path": (base / name).relative_to(root).as_posix(), "sha256": sha(base / name)}


def passed(name, expected):
    match = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s", decoded(name))
    if not match or tuple(map(int, match[-1][:3])) != (expected, 0, 0):
        raise SystemExit(f"actual {expected}-test result required: {name}")
    return {"status": "PASS", "passed": expected, "failed": 0, "ignored": 0,
            "debug_seconds": float(match[-1][3]), "log": evidence(name)}


destination = base / "free-erp-inventory-location-commands-repair.json"
if destination.exists():
    raise SystemExit("refusing to overwrite location repair evidence")
independent = read("free-erp-inventory-location-commands-independent.json")
if independent["status"] != "PASS":
    raise SystemExit("independent facts must pass")
original_log = "free-erp-inventory-location-commands-before-repair.log"
first_log = "free-erp-inventory-location-commands-focused.log"
if "0 passed; 2 failed; 0 ignored" not in decoded(original_log) or "8 passed; 1 failed; 0 ignored" not in decoded(first_log):
    raise SystemExit("both actual failure records required")
fmt_name = "free-erp-inventory-location-commands-final-fmt-refined.log"
if "six-leaf fixed-entry fmt check PASS" not in decoded(fmt_name):
    raise SystemExit("actual final format gate required")

owned = ["crates/erp-repo/src/inventory_location_commands.rs", "crates/erp-repo/src/inventory_movement_commands.rs",
         "crates/erp-repo/src/lib.rs", "crates/erp-api/src/inventory_print_report_handlers/stock_movement.rs",
         "crates/erp-api/src/inventory_print_report_handlers/inventory_core.rs", "crates/erp-api/src/inventory_print_report_handlers/inventory_commands.rs",
         "crates/erp-api/tests/free_erp_acceptance.rs", "crates/erp-api/tests/free_erp_acceptance/inventory_location_commands.rs",
         "migrations/0285_inventory_location_projection_guard.sql"]
comparisons = {}
for stage in ("location-refined-focused", "location-full"):
    before_name = f"free-erp-inventory-consumers-{stage}-before-source.json"
    after_name = f"free-erp-inventory-consumers-{stage}-after-source.json"
    before = read(before_name)["files"]
    after = read(after_name)["files"]
    changed = [p for p in sorted(set(before) | set(after)) if before.get(p) != after.get(p)]
    drift = [p for p, digest in after.items() if not (root / p).exists() or sha(root / p) != digest]
    comparisons[stage] = {"status": "PARTIAL" if changed or drift else "PASS",
        "before_inputs": len(before), "after_inputs": len(after), "changed_in_run": changed,
        "changed_since_after": drift, "owned_changed_in_run": [p for p in changed if p in owned],
        "owned_changed_since_after": [p for p in drift if p in owned],
        "before": evidence(before_name), "after": evidence(after_name),
        "limits": "All actual shared roots remain in the certification scope; passing binaries do not certify later source changes."}

test_name = "crates/erp-api/tests/free_erp_acceptance/inventory_location_commands.rs"
tested = decoded("free-erp-inventory-location-commands-tested-test-source.txt").replace("\r\n", "\n")
current = (root / test_name).read_text(encoding="utf8")
replacement = "        f.tenant, actor.unwrap_or(f.actor)\n"
if tested.count(replacement) != 1 or current != tested.replace(replacement, "        f.tenant,\n        actor.unwrap_or(f.actor)\n"):
    raise SystemExit("post-test change must be exactly the recorded formatting correction")
if sha(base / "free-erp-inventory-location-commands-tested-test-source.txt") != read("free-erp-inventory-consumers-location-full-after-source.json")["files"][test_name]:
    raise SystemExit("formatting comparison must use the actual full-test source")

clippy_name = "free-erp-inventory-location-commands-strict-clippy.log"
clippy = decoded(clippy_name)
failures = {f"{package} {target}": int(count) for package, target, count in
            re.findall(r"error: could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors", clippy)}
if not failures:
    raise SystemExit("actual strict Clippy execution required")
diagnostics = []
for path, line, column in re.findall(r"(?m)^\s*-->\s*(.+?):(\d+):(\d+)", clippy):
    normalized = path.replace("\\", "/")
    if normalized in owned:
        diagnostics.append({"path": normalized, "line": int(line), "column": int(column)})

openapi_name = "free-erp-inventory-location-commands-final-openapi-inputs.json"
openapi = read(openapi_name)
api_drift = [p for p, digest in openapi["after"].items() if sha(root / p) != digest]
bundle_name = "free-erp-inventory-location-commands-final-openapi-bundle.log"
gate_name = "free-erp-inventory-location-commands-final-openapi-gate.log"
if "[openapi-bundle] Bundled" not in decoded(bundle_name) or "verification passed" not in decoded(gate_name):
    raise SystemExit("actual bundle and gate required")
schema_name = "free-erp-inventory-location-commands-schema-readback.json"
schema = read(schema_name)
if (schema["database"], schema["role"], schema["host"], schema["port"]) != ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438):
    raise SystemExit("protected schema identity required")
if not (schema["location_table"] and schema["location_disabled"] and schema["outbox_trace"]
        and schema["projection_guard"]["enabled"] == "O" and "INCLUDE (delta_micros)" in schema["location_index"]):
    raise SystemExit("actual location, trace, guard and index required")

report = {"status": "PARTIAL", "updated_at": datetime.now(timezone.utc).isoformat(),
    "scope": "Original whole-unit inbound with warehouse-owned location, batch_code and GOOD/PENDING/FAILED/HOLD quality; canonical stock, shared projection, required logs and both immutable original results share one commit.",
    "routes": ["POST /v1/tenants/{tenant_id}/inventory/inbounds", "GET /v1/tenants/{tenant_id}/inventory/commands/{request_id}"],
    "counterexamples": [{"status": "REPRODUCED", "passed": 0, "failed": 2, "log": evidence(original_log),
        "finding": "Changed quantity 1 to 2 replayed the original ledger with a fabricated 2000000 quantity; structured success had no primary balance or required logs. Failed owned fixtures remain unchanged."},
        {"status": "FAIL", "passed": 8, "failed": 1, "log": evidence(first_log),
         "finding": "Protected QA lacked the real 0030 trace_id column. Failure occurred before the intended log fault. Add only that nullable column; refined tests require the named view's actual check-option error."}],
    "focused": passed("free-erp-inventory-location-commands-refined-focused.log", 9),
    "registered_native": passed("free-erp-inventory-location-commands-full-native.log", 125),
    "independent": {"status": "PASS", "checks": independent["checks"], "existing_tenants_unchanged": independent["existing_tenants_unchanged"],
        "new_location_fixtures": independent["new_location_fixtures"], "counts": independent["counts"],
        "evidence": evidence("free-erp-inventory-location-commands-independent.json"),
        "verifier": evidence("free-erp-inventory-location-commands-verify.py")},
    "implementation": {"current_source_sha256": {p: sha(root / p) for p in owned},
        "identity": "Existing shared command namespace and v1 fingerprints; optional location fields skip serialization when absent, preserving historical ordinary command snapshots.",
        "atomicity": "Claim original ID, lock enabled owned location and existing location key, apply precise primary inventory, append its linked ON_HAND projection, write both logs and immutable original warehouse/location results, commit once.",
        "quality": "GOOD increases available; PENDING/FAILED/HOLD increase physical and frozen equally. Quality release is not implemented by this command.",
        "readback": "Same original authenticated actor and current warehouse grant; verify primary/shared effects, exact quantities, timestamp and both log identities. Replay never substitutes later stock.",
        "index": "Full owned location/batch/quality/bucket key with delta_micros included; this is index/schema evidence, not measured Release latency."},
    "source_certification": comparisons,
    "quality_gates": {"fmt": {"status": "PASS", "leaves": 6, "log": evidence(fmt_name), "original_failure": evidence("free-erp-inventory-location-commands-fmt-failure.json")},
        "post_test_formatting": {"status": "PASS", "exact_whitespace_only_correction": True, "tested_source": evidence("free-erp-inventory-location-commands-tested-test-source.txt"), "business_tests_rerun_after_formatting": 0},
        "strict_backend_clippy": {"status": "FAIL", "package_error_counts": failures, "owned_diagnostics_observed": diagnostics,
            "log": evidence(clippy_name), "limits": "No observed owned diagnostic does not certify any blocked package or API lint."},
        "openapi": {"status": "PARTIAL" if api_drift or not openapi["source_stable_in_window"] else "PASS", "changed_since_gate": api_drift,
            "inputs": evidence(openapi_name), "bundle": evidence(bundle_name), "gate": evidence(gate_name)}},
    "schema": {"status": "PASS", "readback": evidence(schema_name), "sql": evidence("free-erp-inventory-location-commands-schema-readback.sql"),
        "limits": "Only selected real 0031/0058 location schema, the 0030 nullable trace field and new 0285 guard/index were applied in protected QA; no full migration/restore acceptance."},
    "remaining": ["Fractional input and UUID batch_id, location-specific outbound/reserve/release, complete WMS/quality transitions and all other inventory writers.",
        "Unlinked historical shared effects require reconciliation; legacy actor/snapshots are not fabricated or reposted.",
        "Synthetic mapping exercises real Outbox production, not external platform sync or Worker dispatch.",
        "Real password/one-seat/multi-session admission, native/Web checkpoints and commit-boundary network failure recovery.",
        "Release 100-round loads, separate first/replay/read distributions and resource budgets, strict package gates, stable complete source, hosted launch and full restore."],
    "completion": "Selected structured inbound has observed functional and independent PASS; I01/I02/O01/O02 and the complete free ERP remain PARTIAL."}
with destination.open("x", encoding="utf8", newline="\n") as output:
    json.dump(report, output, ensure_ascii=False, indent=2)
    output.write("\n")
print(json.dumps({"status": report["status"], "registered": 125, "independent": independent["checks"],
    "whole_source": comparisons["location-full"]["status"], "full_run_changes": comparisons["location-full"]["changed_in_run"],
    "clippy": "FAIL", "owned_diagnostics": diagnostics, "openapi": report["quality_gates"]["openapi"]["status"]}, ensure_ascii=False))
