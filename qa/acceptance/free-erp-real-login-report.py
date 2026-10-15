"""Collect immutable real-login evidence without replacing a prior run."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-real-login-"
TARGET = BASE / (PREFIX + "repair.json")
if TARGET.exists():
    raise SystemExit("refusing to replace real-login repair evidence")


def load(name):
    return json.loads((BASE / name).read_text(encoding="utf-8-sig"))


def digest(path):
    sha = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            sha.update(chunk)
    return sha.hexdigest()


def result(name):
    log = (BASE / name).read_text(encoding="utf-8-sig")
    values = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
                        r"(\d+) measured; (\d+) filtered out; finished in ([\d.]+)s", log)
    if len(values) != 1:
        raise SystemExit("one actual terminal test result required: " + name)
    status, passed, failed, ignored, measured, filtered, seconds = values[0]
    return {"status": "PASS" if status == "ok" else "FAIL", "passed": int(passed),
            "failed": int(failed), "ignored": int(ignored), "measured": int(measured),
            "filtered_out": int(filtered), "seconds": float(seconds), "log": name}


def clippy(name):
    raw = (BASE / name).read_text(encoding="utf-8-sig")
    errors = re.findall(r"^error(?:\[[^\]]+\])?: (.+)$", raw, re.M)
    summaries = re.findall(r"^error: could not compile .+$", raw, re.M)
    successful = re.search(r"ERP strict (?:repo|api) Clippy exit_code=0", raw) is not None
    status = "PASS" if successful and not errors else ("FAIL" if errors and summaries else "NOT_RUN")
    return {"status": status, "diagnostics": len(errors), "compiler_summaries": summaries,
            "log": name, "warnings_as_errors": True}


original = load(PREFIX + "original-pre-cases-second-independent.json")
repaired = load(PREFIX + "repaired-independent.json")
full = load(PREFIX + "full-independent.json")
current = load(PREFIX + "full-recheck-independent.json")
docs = load(PREFIX + "docs-quality-final-verify.json")
strict_repo = clippy(PREFIX + "repo-clippy-current.log")
strict_api = clippy(PREFIX + "api-clippy-current.log")
phase_results = {
    "original_real_password_cases": result(PREFIX + "original-third-test.log"),
    "repaired_real_password_and_genuine_v1": result(PREFIX + "repaired-test.log"),
    "original_provenance_test_blocked_by_database_guard": result(PREFIX + "provenance-test.log"),
    "first_registered_full": result(PREFIX + "full-test.log"),
    "current_registered_full": result(PREFIX + "full-recheck-test.log"),
}
if any(report["errors"] for report in [original, repaired, full, current]):
    raise SystemExit("independent fact errors must be resolved or separately explained")
if docs["status"] != "PASS":
    raise SystemExit("current detailed documentation not validated")
current_result = phase_results["current_registered_full"]
if current_result["status"] != "PASS" or current_result["ignored"] or current_result["filtered_out"]:
    raise SystemExit("complete registered regression must actually pass")

sources = [
    "crates/erp-repo/src/inventory_movement_commands.rs",
    "crates/erp-repo/src/access_plan.rs",
    "crates/erp-api/tests/free_erp_acceptance.rs",
    "crates/erp-api/tests/free_erp_acceptance/account_login.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_commands.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_location_commands.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_command_edges.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_batch_writes.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_location_outbound.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_location_reservation_edges.rs",
    "crates/erp-api/tests/free_erp_acceptance/inventory_kit_source_edges.rs",
    "crates/erp-api/tests/free_erp_acceptance/fulfillment_cost_batches.rs",
    "crates/erp-api/tests/free_erp_acceptance/login_sessions.rs",
]
artifacts = [
    PREFIX + name for name in [
        "original-test.log", "original-second-test.log", "original-third-test.log",
        "original-independent.json", "original-pre-cases-independent.json",
        "original-pre-cases-second-independent.json", "original-protected-drift.json",
        "original-legacy-fixture.json", "repaired-test.log", "repaired-independent.json",
        "provenance-test.log", "full-test.log", "full-independent.json",
        "full-recheck-test.log", "full-recheck-independent.json", "docs-final-verify.json",
        "docs-quality-final-verify.json",
        "repo-clippy-current.log", "api-clippy-current.log", "format-current.log",
        "format-repair.log", "format-recheck.log",
        "timestamp-catalog.json", "readback.sql", "capture.py", "verify.py",
        "drift-compare.py", "report.py",
    ]
]
artifacts += ["free-erp-inventory-consumers-real-login-" + stage + "-source.json" for stage in [
    "original-third-before", "original-third-after", "repaired-before", "repaired-after",
    "full-before", "full-after", "full-recheck-before", "full-recheck-after",
]]
report = {
    "utc": datetime.now(timezone.utc).isoformat(),
    "status": "PARTIAL",
    "scope": "Real-password stock fixtures, credential-generation sessions, cross-device original movement results and historical v1; ongoing goal",
    "authorization": "Existing repair goal; no deploy, production data mutation, Grok messaging or cache cleanup",
    "protected_database": {"database": "erp_shared_contract_test", "role": "erp_grid_test",
                           "host": "127.0.0.1", "port": 15438, "selected_tables": 69},
    "functional_test_results": phase_results,
    "independent_results": {"original": original, "repaired": repaired,
                            "first_full": full, "current_full": current},
    "repairs": [
        "Genuine salted Argon2 password, actual database roles and server-issued sid/cep tokens for normal stock, batch and location fixtures",
        "Movement fingerprint v2 excludes transport session; immutable original snapshot and mandatory operation/audit metadata preserve original submitter session",
        "Version-aware exact historical v1 verification and semantic cross-device replay without rewriting any old receipt, hash or inventory effect",
        "Current credential generation required for named-session reuse; stale active sessions preserved as revoked and excluded from the quota",
        "Real named login extends expiry without shrinking it; original eight-session reuse remains valid",
        "Remove duplicated unused session count before the existing commit protection; retain final authoritative count",
        "Actual paid QA second-actor preparation bound to exact owned fixture names; free production seats unchanged",
        "Current authentication denial assertions preserve all business facts; expiry concurrency fixture starts at the actual eight-session ceiling",
        "v2 original provenance alteration is rejected by the existing immutable receipt trigger; both real sessions, logs and one effect independently read back",
    ],
    "preserved_failures": [
        "Compile failure during concurrent auth helper edits; original evidence retained",
        "Missing cloned real-login field in the duplicated kit fixture; corrected without replacing its compile-failure log",
        "Original actual password-generation 401 and cross-device 409; both entire cases executed",
        "Earliest baseline detects one pre-case older tenant auth-session change; original FAIL and independently localized timestamp window retained",
        "First provenance test attempted an update blocked by the existing immutable guard; test corrected without disabling database protection",
        "First full regression 375 passed and 5 failed; five fixture/assertion repairs verified in a separate complete run",
    ],
    "earliest_protected_data_drift": load(PREFIX + "original-protected-drift.json"),
    "documentation": {"status": docs["status"], "checks": docs["checks"],
                      "report": "qa/acceptance/" + PREFIX + "docs-quality-final-verify.json"},
    "format": {"first_check": "FAIL", "affected_leaf_repair": "PASS",
               "selected_leaf_recheck": "PASS", "selected_leaf_files": 12,
               "log": "qa/acceptance/" + PREFIX + "format-recheck.log"},
    "strict_clippy": {"repo": strict_repo, "api": strict_api},
    "remaining_gates": {
        "all_business_fixtures_real_login": "PARTIAL",
        "all_business_domains_current_role_session_warehouse_authority": "PARTIAL",
        "authority_revocation_during_inventory_lock_wait": "NOT_COMPLETE",
        "formal_password_recovery_and_session_renewal": "NOT_COMPLETE",
        "real_native_and_wasm_login_edit_save_unknown_recovery": "NOT_RUN",
        "full_clean_migration_chain": "NOT_RUN",
        "strict_clippy": "PASS" if all(value["status"] == "PASS" for value in [strict_repo, strict_api]) else "NOT_PASSED",
        "source_input_stability": "PARTIAL" if current["source_input_drift"] else "PASS_FOR_RECHECK_WINDOW_ONLY",
        "release_login_stock_and_multi_device_performance": "NOT_RUN",
        "official_hosting_operational_release": "NOT_RUN",
    },
    "performance_boundary": "Debug functional durations are not P95/P99. The concurrency-boundary fixture uses low-cost QA hashing and cannot qualify production password or Release performance.",
    "current_related_source_sha256": {name: digest(ROOT / name) for name in sources},
    "artifact_sha256": {"qa/acceptance/" + name: digest(BASE / name) for name in artifacts},
}
with TARGET.open("x", encoding="utf-8") as output:
    output.write(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({"status": report["status"], "current_registered_full": current_result,
                  "protected_older_enterprises": current["older_enterprises"],
                  "source_input_drift": current["source_input_drift"],
                  "strict_clippy": {key: value["diagnostics"] for key, value in report["strict_clippy"].items()}},
                 ensure_ascii=False))
