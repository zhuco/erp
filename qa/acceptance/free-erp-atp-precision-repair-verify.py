"""Bind actual ATP repair runs, independent facts and source limitations."""

from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

base = Path(__file__).resolve().parent
root = base.parents[1]
errors = []


def check(ok, label):
    if not ok:
        errors.append(label)


def read(name):
    return (base / name).read_text(encoding="utf-8-sig")


def data(name):
    return json.loads(read(name))


full = read("free-erp-precision-full-atp.log")
focused = read("free-erp-atp-precision-focused.log")
unit = read("free-erp-atp-precision-compatibility-unit.log")
failure = read("free-erp-atp-precision-before.log")
clippy = read("free-erp-atp-precision-clippy.log")
facts = data("free-erp-precision-atp-readback-validation.json")
source = data("free-erp-atp-precision-source-consistency.json")
before = data("free-erp-atp-precision-final-source-before.json")
after = data("free-erp-atp-precision-final-source-after.json")
check("55 passed; 0 failed; 0 ignored;" in full, "complete registered native run")
check("5 passed; 0 failed; 0 ignored;" in focused and "49 filtered out" in focused,
      "earlier focused ATP regression")
check("2 passed; 0 failed; 0 ignored;" in unit and "72 filtered out" in unit,
      "existing whole-unit FEFO unit contracts")
check("0 passed; 1 failed;" in failure and '"available":1' in failure and '"atp_qty":1' in failure,
      "original real HTTP overpromise retained")
check(facts["status"] == "PASS" and facts["checks"] == 444 and facts["precision_cases"] == 21
      and facts["fresh_fixture_count"] == 23 and facts["errors"] == [], "independent SQL facts")
check("[openapi-gate] OpenAPI gate verification passed" in read("free-erp-atp-precision-final-openapi.log"),
      "bundled OpenAPI gate")
check(read("free-erp-atp-precision-format-check.log").strip() == "", "leaf-format check diagnostics")
check(source["before_count"] == source["after_count"] == 890 and source["added"] == [],
      "complete input manifest cardinality")
check(set(source["changed"]) == {"crates/erp-api/tests/shared_contract.rs", "crates/erp-repo/src/shared_contract.rs"}
      and source["status"] == "PARTIAL", "shared source drift is not certified as PASS")
leaves = ["crates/erp-api/src/auth_layers.rs", "crates/erp-api/src/lib.rs",
          "crates/erp-api/src/inventory_views.rs",
          "crates/erp-api/src/inventory_print_report_handlers/inventory_core.rs",
          "crates/erp-api/tests/free_erp_acceptance/auth_scope.rs",
          "crates/erp-api/tests/free_erp_acceptance/inventory_precision.rs",
          "crates/erp-repo/src/atp_inventory.rs", "crates/erp-repo/src/inventory_precision.rs",
          "openapi/src/components.yaml", "openapi/src/paths.yaml", "openapi/erp_api_v1.yaml"]
for name in leaves:
    check(before["files"][name] == after["files"][name], f"changed leaf stable during the full run: {name}")
lint_failures = {crate: int(count) for crate, count in re.findall(
    r"could not compile `([^`]+)` \(lib\) due to (\d+) previous errors", clippy)}
check(lint_failures.get("erp-repo") == 235, "strict dependency lint failure remains visible")
seconds = float(re.search(r"55 passed;.*?finished in ([0-9.]+)s", full)[1])
evidence = ["free-erp-atp-precision-before.log", "free-erp-atp-precision-focused.log",
            "free-erp-atp-precision-source-before.json", "free-erp-atp-precision-focused-source-after.json",
            "free-erp-precision-full-atp.log", "free-erp-atp-precision-final-source-before.json",
            "free-erp-atp-precision-final-source-after.json", "free-erp-atp-precision-source-consistency.json",
            "free-erp-precision-atp-qa-baseline.sql", "free-erp-precision-atp-qa-before.json",
            "free-erp-precision-atp-readback.json", "free-erp-precision-atp-readback-validation.json",
            "free-erp-precision-readback.sql", "free-erp-precision-readback-verify.py",
            "free-erp-atp-precision-compatibility-unit.log", "free-erp-atp-precision-format-check.log",
            "free-erp-atp-precision-final-openapi.log", "free-erp-atp-precision-clippy.log",
            "free-erp-auth-scope-repair.json"]
current_changed = [name for name in before["files"] if (root / name).is_file()
                   and hashlib.sha256((root / name).read_bytes()).hexdigest() != after["files"][name]]
report = {
    "updated_at": datetime.now(timezone.utc).isoformat(), "status": "FAIL" if errors else "PARTIAL",
    "repair": ["Canonical SKU balance, reservation and freeze before projection",
               "Decimal query and authoritative decimal quantities/allocation strings on the existing route",
               "Read-only repeatable-read stock/batch snapshot; no implicit activation or zero-row creation",
               "FEFO expiry exclusion, owned batch metadata and wide sum reconciliation",
               "Bounded 1001-row query, explicit 422 above the 1000-batch interactive budget",
               "Scope storage/policy outages return 503 before handlers run"],
    "actual_native_run": {"passed": 55, "failed": 0, "ignored": 0, "seconds": seconds,
                          "configuration": "debug native; protected QA database; synthetic JWT"},
    "independent_readback": {key: facts[key] for key in ("status", "checks", "fresh_fixture_count", "precision_cases")},
    "direct_whole_contracts": {"passed": 2, "failed": 0, "filtered": 72},
    "leaf_format": "PASS", "openapi_gate": "PASS",
    "strict_clippy": {"status": "FAIL", "dependency_errors": lint_failures,
                      "api_lint_completed": False, "global_suppressions_added": False},
    "source_consistency": source,
    "changed_leaves_sha256": {name: after["files"][name] for name in leaves},
    "current_changes_after_run": current_changed,
    "evidence_integrity_errors": errors,
    "remaining": ["Public inventory lists/filters/summaries/costs/ledgers/reports/exports still need canonical precision",
                  "All legacy WMS/shared/quality/freeze/batch writers need successful canonical integration",
                  "Real password/session/seat acceptance, empty-scope/admin policy and revoked access",
                  "Complete migrations, event versions, audit atomicity and restore/public service acceptance",
                  "Official native/Web page acceptance and Release ATP/load/resource metrics",
                  "Full strict lint cleanup and exact-source acceptance after concurrent work settles"],
    "evidence_sha256": {name: hashlib.sha256((base / name).read_bytes()).hexdigest() for name in evidence},
}
(base / "free-erp-atp-precision-repair.json").write_text(
    json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"status": report["status"], "runtime": report["actual_native_run"],
                  "readback": report["independent_readback"], "errors": errors}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
