"""Preserve observed outbound gates and complete-source qualification separately."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
destination = base / "free-erp-inventory-location-outbound-repair.json"
if destination.exists():
    raise SystemExit("refusing to replace historical repair evidence")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(name):
    return json.loads((base / name).read_text(encoding="utf8"))


def decoded(name):
    value = (base / name).read_bytes()
    return value.decode("utf-16" if value.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")


def evidence(name):
    path = base / name
    return {"path": path.relative_to(root).as_posix(), "sha256": sha(path)}


original_name = "free-erp-inventory-location-outbound-before-repair.log"
if "0 passed; 2 failed; 0 ignored" not in decoded(original_name):
    raise SystemExit("the actual original counterexample is required")
owned = [
    "crates/erp-repo/src/inventory_location_commands.rs",
    "crates/erp-repo/src/inventory_movement_commands.rs",
    "crates/erp-repo/src/inventory_precision.rs",
    "crates/erp-api/src/inventory_print_report_handlers/stock_movement.rs",
    "crates/erp-api/src/inventory_print_report_handlers/inventory_commands.rs",
    "crates/erp-api/tests/free_erp_acceptance.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_location_commands.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_location_outbound.rs",
]
observed = {}
source = {}
for stage, expected in (("focused", 11), ("full", 136)):
    log_name = f"free-erp-inventory-location-outbound-{stage}.log"
    results = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s", decoded(log_name))
    if not results or tuple(map(int, results[-1][:3])) != (expected, 0, 0):
        raise SystemExit(f"observed {stage} tests did not pass")
    independent_name = f"free-erp-inventory-location-outbound-{stage}-independent.json"
    independent = read(independent_name)
    if independent["status"] != "PASS":
        raise SystemExit(f"independent {stage} facts did not pass")
    for name, digest in independent["inputs"].items():
        if sha(base / name) != digest:
            raise SystemExit(f"independent input changed: {name}")
    observed[stage] = {
        "status": "PASS", "passed": expected, "failed": 0, "ignored": 0,
        "debug_seconds": float(results[-1][3]), "log": evidence(log_name),
        "independent_checks": independent["checks"],
        "existing_tenants_unchanged": independent["existing_tenants_unchanged"],
        "new_owned_fixtures": independent["new_owned_fixtures"], "counts": independent["counts"],
        "independent": evidence(independent_name),
    }
    before_name = f"free-erp-inventory-consumers-location-outbound-{stage}-before-source.json"
    after_name = f"free-erp-inventory-consumers-location-outbound-{stage}-after-source.json"
    before = read(before_name)["files"]
    after = read(after_name)["files"]
    changed = [p for p in sorted(set(before) | set(after)) if before.get(p) != after.get(p)]
    drift = [p for p, digest in after.items() if not (root / p).is_file() or sha(root / p) != digest]
    source[stage] = {
        "status": "PARTIAL" if changed or drift else "PASS",
        "before_inputs": len(before), "after_inputs": len(after),
        "changed_in_run": changed, "changed_since_after": drift,
        "owned_changed_in_run": [p for p in changed if p in owned],
        "owned_changed_since_after": [p for p in drift if p in owned],
        "before": evidence(before_name), "after": evidence(after_name),
    }
if source["full"]["owned_changed_in_run"] or source["full"]["owned_changed_since_after"]:
    raise SystemExit("owned inputs must match the complete observed test window")

clippy_name = "free-erp-inventory-location-outbound-strict-clippy.log"
clippy = decoded(clippy_name)
clippy_failures = {f"{package} {target}": int(count) for package, target, count in
                   re.findall(r"error: could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors", clippy)}
if not clippy_failures:
    raise SystemExit("actual strict Clippy outcome required")
owned_diagnostics = [line for line in clippy.splitlines() if "-->" in line and
                     any(Path(name).name in line for name in owned)]
fmt_name = "free-erp-inventory-location-outbound-fmt.log"
if "seven-leaf fixed-entry fmt check PASS" not in decoded(fmt_name):
    raise SystemExit("actual seven-leaf format gate required")
openapi_name = "free-erp-inventory-location-outbound-openapi-inputs.json"
openapi = read(openapi_name)
openapi_drift = [p for p, digest in openapi["after"].items() if sha(root / p) != digest]
if sha(root / "openapi/erp_api_v1.yaml") != openapi["bundle_sha256"]:
    openapi_drift.append("openapi/erp_api_v1.yaml")
bundle_name = "free-erp-inventory-location-outbound-openapi-bundle.log"
gate_name = "free-erp-inventory-location-outbound-openapi-gate.log"
if "[openapi-bundle] Bundled" not in decoded(bundle_name) or "verification passed" not in decoded(gate_name):
    raise SystemExit("actual OpenAPI bundle/gate outcomes required")

report = {
    "updated_at": datetime.now(timezone.utc).isoformat(), "status": "PARTIAL",
    "scope": "Original whole-unit GOOD free-stock outbound by warehouse/location/SKU/batch; canonical and location quantities, required logs, Outbox and both immutable original results share one commit.",
    "routes": ["POST /v1/tenants/{tenant_id}/inventory/outbounds", "GET /v1/tenants/{tenant_id}/inventory/commands/{request_id}"],
    "counterexample": {"status":"REPRODUCED","passed":0,"failed":2,"log":evidence(original_name),
                       "finding":"Successful canonical structured inbound could not be followed by location outbound; both adequate-stock cases returned 400 unsupported. Original failed fixtures are preserved.",
                       "before_source":evidence("free-erp-inventory-consumers-location-outbound-before-repair-source.json"),
                       "after_source":evidence("free-erp-inventory-consumers-location-outbound-original-after-source.json")},
    "observed_acceptance": observed, "source_certification": source,
    "current_owned_source_sha256": {p:sha(root / p) for p in owned},
    "implementation": {
        "identity": "Existing shared original command namespace and v1 fingerprint, including actor, direction and complete location dimensions; absent optional location retains old ordinary snapshots.",
        "admission": "Lock owned enabled location row, existing location ON_HAND key, then canonical physical key before reading all four location buckets. GOOD free-stock admission subtracts RESERVED/FROZEN/UNAVAILABLE with wide arithmetic, followed by exact aggregate physical admission.",
        "atomicity": "Claim original ID, primary exact stock effect, linked signed ON_HAND projection, required logs, producer events and original warehouse/location snapshots commit once; rejected commands and actual log faults leave no partial effects.",
        "readback": "Preserve original 3.9 warehouse/location results after later stock reaches 5.9; originally zero location balance remains valid. Current warehouse grant applies to replay and GET; disabling a location rejects new writes while original committed results remain recoverable.",
        "historical": "An unlinked legacy five-unit location projection cannot be sold or silently activated; no historical actor, source or original snapshot is fabricated.",
    },
    "quality_gates": {
        "format": {"status":"PASS","log":evidence(fmt_name)},
        "clippy": {"status":"FAIL","failures":clippy_failures,"observed_owned_diagnostics":owned_diagnostics,
                   "log":evidence(clippy_name),"limits":"No observed owned diagnostic does not certify the blocked packages or API lint."},
        "openapi": {"status":"PARTIAL" if openapi_drift or not openapi["source_stable_in_window"] else "PASS",
                    "changed_since_gate":openapi_drift,"inputs":evidence(openapi_name),"bundle":evidence(bundle_name),"gate":evidence(gate_name)},
    },
    "verification_tool": evidence("free-erp-inventory-location-outbound-verify.py"),
    "remaining": [
        "Fractional command input and UUID batch_id; source-bound reserve/release/consume, sales delivery, transfer transit, full WMS and quality transition services.",
        "Synthetic linked opening/reservation and channel mappings prove local behavior, not complete reservation service or external platform/Worker dispatch.",
        "Real password/one-seat/multi-session admission, native/Web checkpoints and actual commit-boundary network failure recovery.",
        "Release 100-round contention and production-size load, strict package gates, stable complete source, full migration/restore and hosted launch.",
    ],
    "completion": "Observed structured free-stock outbound and independent readbacks pass. I01/I02/O01/O02 and the complete free ERP remain PARTIAL.",
}
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status":report["status"],"focused":observed["focused"]["passed"],"full":observed["full"]["passed"],
                  "independent":observed["full"]["independent_checks"],"source":source["full"]["status"],
                  "owned_changed_in_full":source["full"]["owned_changed_in_run"],"clippy":"FAIL",
                  "openapi":report["quality_gates"]["openapi"]["status"]}, ensure_ascii=False))
