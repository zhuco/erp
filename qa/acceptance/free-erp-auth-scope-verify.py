"""Verify the original authorization regression and independent SQL facts."""

from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

base = Path(__file__).resolve().parent
root = base.parents[1]
errors = []
checks = 0


def check(ok, label):
    global checks
    checks += 1
    if not ok:
        errors.append(label)


def read_json(name):
    return json.loads((base / name).read_text(encoding="utf-8-sig"))


facts = read_json("free-erp-auth-scope-readback.json")
consistency = read_json("free-erp-auth-scope-source-consistency.json")
before = read_json("free-erp-auth-scope-source-before.json")
after = read_json("free-erp-auth-scope-source-after.json")
log = (base / "free-erp-auth-scope-full.log").read_text(encoding="utf-8-sig")
failure = (base / "free-erp-auth-scope-before.log").read_text(encoding="utf-8-sig")
check(tuple(facts[k] for k in ("database", "role", "host", "port")) ==
      ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), "protected QA identity")
check(facts["grants"] == ["33b47eb1-2ef8-44af-ac07-59039f102883"], "one original warehouse grant")
check(facts["warehouses"] == 2, "a real ungranted alternative warehouse exists")
check(len(facts["stock"]) == 1 and facts["stock"][0]["on_hand"] == 1
      and facts["stock"][0]["reserved"] == 0 and facts["stock"][0]["frozen"] == 0,
      "original fixture stock preserved")
check(facts["stock"][0]["warehouse_id"] == facts["grants"][0], "stock belongs to granted warehouse")
check(facts["ledgers"] == 1 and facts["ledger_sum"] == 1, "GET did not create stock effects")
check(facts["policy_views"] == 5 and facts["policy_scope_views"] == 4,
      "all four policy-failure scope views retained")
check(facts["policy_tenant_columns"] == ["tenant_id"], "policy-failure fixture omits only policy columns")
check(facts["public_scope_tables"] == 4 and facts["public_policy_columns"] == 2,
      "shared public scope foundation remains present")
check("49 passed; 0 failed; 0 ignored;" in log and "finished in 6.55s" in log,
      "original registered 49-case run")
for name in (
    "authorization_scope_storage_failure_does_not_admit_the_handler",
    "authorization_scope_policy_failure_does_not_disable_scope_checks",
    "authorization_scope_allows_only_the_granted_warehouse",
):
    check(f"test auth_scope::{name} ... ok" in log, name)
check("422 Unprocessable Entity" in failure and "0 passed; 1 failed;" in failure,
      "original fail-open HTTP reproduction is retained")
check("status=503 Service Unavailable body=data scope check unavailable" in log,
      "scope-storage outage rejects before the handler")
check("status=503 Service Unavailable body=tenant scope policy unavailable" in log,
      "tenant-policy outage does not disable scope enforcement")
check(consistency["before_count"] == consistency["after_count"] == 885,
      "full original input snapshots")
check(set(consistency["changed"]) == {
    "crates/erp-api/tests/shared_contract.rs", "crates/erp-repo/src/shared_contract.rs"
} and consistency["added"] == [] and consistency["status"] == "PARTIAL",
      "concurrent shared-source changes remain explicit")
for name in ("crates/erp-api/src/auth_layers.rs", "crates/erp-api/tests/free_erp_acceptance/auth_scope.rs"):
    check(before["files"][name] == after["files"][name], f"original authorization leaf stable: {name}")

names = ["free-erp-auth-scope-before.log", "free-erp-auth-scope-full.log",
         "free-erp-auth-scope-foundation.sql", "free-erp-auth-scope-foundation.log",
         "free-erp-auth-scope-readback.sql", "free-erp-auth-scope-readback.json",
         "free-erp-auth-scope-source-before.json", "free-erp-auth-scope-source-after.json",
         "free-erp-auth-scope-source-consistency.json", "free-erp-auth-scope-clippy.log"]
report = {
    "updated_at": datetime.now(timezone.utc).isoformat(),
    "status": "FAIL" if errors else "PARTIAL",
    "independent_facts": {"status": "FAIL" if errors else "PASS", "checks": checks, "errors": errors},
    "registered_run": {"passed": 49, "failed": 0, "ignored": 0, "seconds": 6.55,
                       "configuration": "debug native; synthetic JWT; protected QA database"},
    "original_artifact": {"path": consistency["artifact_path"], "sha256": consistency["artifact_sha256"],
                          "note": "Captured immediately after the 49-case run; later builds reuse this path."},
    "source_consistency": consistency,
    "strict_clippy": {"status": "FAIL", "dependency": "erp-repo", "errors": 235,
                      "note": "Compilation stops in the dependency; API lint completion is not established."},
    "boundaries": ["No real password/session acceptance", "No empty-scope default or admin policy change",
                   "No production migration, deployment, load, backup or dual-client acceptance",
                   "Later precise-ATP work changes the healthy fixture to use the authoritative inventory writer"],
    "evidence_sha256": {name: hashlib.sha256((base / name).read_bytes()).hexdigest() for name in names},
}
(base / "free-erp-auth-scope-repair.json").write_text(
    json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"status": report["status"], "facts": report["independent_facts"]}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
