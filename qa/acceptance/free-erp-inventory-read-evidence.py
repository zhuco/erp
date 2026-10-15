"""Bind inventory-read repair claims to executed logs and immutable manifests."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
prefix = "free-erp-inventory-read-"


def document(name):
    return json.loads((base / (prefix + name)).read_text("utf-8-sig"))


def log(name):
    return (base / (prefix + name + ".log")).read_text("utf-8-sig")


def test_result(name, count):
    text = log(name)
    results = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", text)
    assert results and sum(int(result[0]) for result in results) == count
    assert all(failed == ignored == "0" for _, failed, ignored in results)
    return {"status": "PASS", "passed": count, "log": "qa/acceptance/" + prefix + name + ".log"}


readback = document("independent-readback.json")
assert readback["result"] == "PASS"
tests = {"registered_native": test_result("final-native-full", 68),
         "report_and_inbound": test_result("report-native-repaired", 9),
         "core_query": test_result("decimal-core", 4),
         "selection_summary": test_result("selection-repaired", 2),
         "xlsx_csv_export": test_result("decimal-export", 8),
         "chart": test_result("decimal-chart", 2),
         "aggregate_api_guards": test_result("api-aggregate-unit", 2),
         "aggregate_repo_temp_pg": test_result("repo-report-unit", 2)}
assert "running 0 tests" in log("api-read-unit")
tests["inventory_core_unit_filter"] = {
    "status": "NOT_EXERCISED", "passed": 0,
    "reason": "This filter matches no unit tests; the read handlers were exercised by the 13 registered integration cases.",
    "log": "qa/acceptance/" + prefix + "api-read-unit.log"}
grid_checks = document("grid-repair-results.json")
assert all(task["result"] == "PASS" for task in grid_checks)
before = document("grid-repair-before-source.json")["files"]
after = document("grid-repair-after-source.json")["files"]
assert before == after, "grid verification interval inputs changed"
assert "OpenAPI gate verification passed" in log("openapi-final")
api_lint = log("api-clippy")
assert "could not compile `erp-repo`" in api_lint
summaries = re.findall(r"error: could not compile `([^`]+)` \(([^)]+)\)[^\n]*due to (\d+) previous errors?", api_lint)
lint = {"api": {"status": "FAIL", "failed_targets": [
                {"package": package, "target": target, "diagnostics": int(count)}
                for package, target, count in summaries],
                "reason": "Dependency lint failures prevent completion of API lint.",
                "log": "qa/acceptance/" + prefix + "api-clippy.log"}}
if (base / (prefix + "repo-clippy-repaired.log")).exists():
    repo_lint = log("repo-clippy-repaired")
    if "Cargo clippy failed" in repo_lint:
        assert not re.search(r"--> crates[\\/]erp-repo[\\/]src[\\/]inventory_(?:reads|grid_report)\.rs:", repo_lint)
        lint["repo_after_leaf_repair"] = {
            "status": "FAIL", "affected_read_leaves_have_diagnostics": False,
            "failed_targets": [
                {"package": package, "target": target, "diagnostics": int(count)}
                for package, target, count in re.findall(
                    r"error: could not compile `([^`]+)` \(([^)]+)\)[^\n]*due to (\d+) previous errors?", repo_lint)],
            "log": "qa/acceptance/" + prefix + "repo-clippy-repaired.log"}
        tests["aggregate_repo_after_lint_repair"] = test_result("repo-report-repaired", 2)

native_after = document("final-native-after-source.json")["files"]
current_drift = sorted(name for name, digest in native_after.items()
                       if hashlib.sha256((root / name).read_bytes()).hexdigest() != digest)
sha_names = [prefix + name for name in ["final-native-full.log", "final-native-executable.json",
            "independent-readback.json", "grid-repair-results.json", "grid-executables.json",
            "api-clippy.log", "repo-report-unit.log", "api-aggregate-unit.log"]]
sha_names += [prefix + name + ".log" for name in ["before", "summary-before", "focused-second",
              "report-native-first", "grid-clippy-native", "grid-clippy-web"]]
sha_names += [prefix + name + ".json" for name in ["final-native-before-source", "final-native-after-source",
              "grid-repair-before-source", "grid-repair-after-source"]]
sha_names += [prefix + name + ".log" for name in ["repo-report-repaired", "repo-clippy-repaired",
              "backend-format", "grid-format", "report-format-repaired"]]
report = {
    "updated_at": datetime.now(timezone.utc).isoformat(), "status": "PARTIAL",
    "scope": "Public inventory four GET reads, aggregate precision and the existing native/Web report host",
    "implemented": ["Canonical six-decimal bucket/available reads and filters before pagination",
                    "Cross-warehouse wide quantities and eight-decimal SKU-master value estimates",
                    "Read-only snapshot and source-key reconciliation before numeric filters/pages",
                    "Nullable ordinary/KIT cost, explicit overflow and safety-stock policy errors",
                    "Aggregate schema2 metadata and exact schema1 compatibility or explicit upgrade",
                    "Explicit unauthorized warehouse refusal on ledger route",
                    "Typed decimal numeric sort, selection sum, CSV/XLSX and chart precision"],
    "tests": tests, "client_unique_native_tests": 25,
    "independent_readback": {"file": "qa/acceptance/" + prefix + "independent-readback.json",
                             "checks": readback["checks"], "fixtures": readback["new_fixtures"],
                             "http_responses": readback["http_responses"],
                             "pre_existing_fixtures_unchanged": readback["pre_existing_fixtures_unchanged"]},
    "source_certification": {
        "native_registered": "PARTIAL", "native_input_count": readback["native_input_count"],
        "native_input_drift_during_run": readback["native_input_drift"],
        "selected_read_inputs_stable_during_native_run": readback["selected_read_inputs_stable"],
        "grid_repaired_interval": "PASS", "grid_repaired_interval_input_count": len(before),
        "current_drift_from_registered_native_snapshot": current_drift,
        "note": "Backend shared root is a real selected dependency. The leaf let-chain cleanup after the full run is verified by dedicated repository tests; do not certify the current whole workspace using an older executable."},
    "checks": {"grid_web_compile": "PASS", "grid_native_strict_clippy": "PASS",
               "grid_web_strict_clippy": "PASS", "openapi": "PASS",
               "selected_leaf_format": "PASS", "lint": lint},
    "native_executable_observation": document("final-native-executable.json"),
    "client_executable_observations": document("grid-executables.json"),
    "performance": {"status": "NOT_RUN", "release_inventory": "NOT_RUN",
                    "debug_registered_test_seconds": 23.81,
                    "note": "Debug harness duration and compilation are not request P95/P99 or scale acceptance."},
    "not_accepted": ["Worker inventory CSV (legacy on_hand/reserved and available omits frozen), turnover and get_balance",
                     "Legacy mutation reply precision and all successful old writer migrations",
                     "Formal inventory pages, decimal editing/formula contracts and real dual-client file delivery",
                     "Actual 500/2000-row capacity, source-UNION query plan and Release P95/P99/memory",
                     "Historic all-null ledger precision branch; current old-shape inserts receive canonical trigger fields",
                     "Mapped-channel outbox and current full worker schema/processing",
                     "Real password, one-seat/multi-device sessions and current default unrestricted/admin scope policy",
                     "Full migration chain, backup restore, public hosting and launch"],
    "limitations": readback["limits"],
    "evidence_sha256": {name: hashlib.sha256((base / name).read_bytes()).hexdigest()
                        for name in sha_names if (base / name).is_file()},
}
destination = base / (prefix + "repair.json")
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "native_tests": 68,
                  "client_tests": 25, "independent_checks": readback["checks"],
                  "native_certification": "PARTIAL", "grid_interval": "PASS",
                  "backend_lint": lint}, ensure_ascii=False))
