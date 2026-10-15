"""Run and verify the isolated rollback-only inspection migration subset."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-production-quality-source-"
paths = {
    "sql": QA / f"{PREFIX}chain.sql",
    **{f"migration_{number}": ROOT / "migrations" / name for number, name in [
        (291, "0291_inspection_source_bindings.sql"),
        (292, "0292_inspection_command_identity.sql"),
        (293, "0293_inspection_bounded_command_keys.sql"),
    ]},
}
log = QA / f"{PREFIX}chain-current.log"
readback = QA / f"{PREFIX}chain-rollback-readback.log"
destination = QA / f"{PREFIX}chain-independent.json"
if any(path.exists() for path in [log, readback, destination]):
    raise SystemExit("refusing to replace historical chain evidence")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


source_before = {name: sha(path) for name, path in paths.items()}
psql = ["G:/PostgreSQL/17/bin/psql.exe", "-X", "-q", "-t", "-A", "-h", "127.0.0.1",
        "-p", "15438", "-U", "erp_grid_test", "-d", "erp_shared_contract_test",
        "-v", "ON_ERROR_STOP=1"]
with log.open("xb") as stream:
    run = subprocess.run(psql + ["-f", str(paths["sql"])], stdout=stream,
                         stderr=subprocess.STDOUT, cwd=ROOT, check=False)
source_after = {name: sha(path) for name, path in paths.items()}
with readback.open("xb") as stream:
    rollback = subprocess.run(psql + ["-c", """
begin transaction read only;
select jsonb_build_object('database',current_database(),'role',current_user,
 'host',inet_server_addr(),'port',inet_server_port(),
 'fixture_schema_absent',not exists(select 1 from pg_namespace
  where nspname='qa_inspection_source_chain'));
commit;
"""], stdout=stream, stderr=subprocess.STDOUT, cwd=ROOT, check=False)


def objects(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8-sig").splitlines()
            if line.lstrip().startswith("{")]


rows = objects(log)
backs = objects(readback)
errors = []
checks = {}


def require(name, condition):
    checks[name] = bool(condition)
    if not condition:
        errors.append(name)


require("actual_chain_exit_zero", run.returncode == 0)
require("actual_read_only_rollback_readback_exit_zero", rollback.returncode == 0)
require("chain_source_unchanged_during_execution", source_before == source_after)
require("one_actual_chain_result", len(rows) == 1)
require("one_actual_rollback_result", len(backs) == 1)
result = rows[0] if len(rows) == 1 else {}
back = backs[0] if len(backs) == 1 else {}
identity = ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438)
for label, row in [("chain", result), ("rollback", back)]:
    require(f"{label}_protected_database_identity", tuple(row.get(key) for key in
            ["database", "role", "host", "port"]) == identity)
require("rollback_removed_only_own_temporary_schema", back.get("fixture_schema_absent") is True)
require("canonical_payload_preserved_at_32000_bytes", result.get("canonical_payload_bytes") == 32000)
require("canonical_effect_preserved_at_32000_bytes", result.get("canonical_effect_bytes") == 32000)
indexes = result.get("indexes", [])
require("no_global_payload_or_effect_btree", not any("command_receipts" in index and
        ("payload_hash" in index or "effect_id" in index) for index in indexes))
constraints = result.get("constraints", [])
commands = [row for row in constraints if row["name"] in
            ["inspection_binding_original_command", "inspection_review_original_command"]]
require("both_original_commands_use_existing_deferred_primary_key", len(commands) == 2 and all(
        row["definition"] == "FOREIGN KEY (tenant_id, request_id) REFERENCES "
        "command_receipts(tenant_id, request_id) DEFERRABLE INITIALLY DEFERRED"
        and row["validated"] for row in commands))
owners = [row for row in constraints if row["name"] == "inspection_reading_owner_shape"]
require("sale_owner_shape_keeps_historical_validation_pending", len(owners) == 1 and
        not owners[0]["validated"] and "SALES_ORDER" in owners[0]["definition"])
triggers = result.get("triggers", [])
for name in ["inspection_binding_original_payload", "inspection_review_original_payload",
             "inspection_binding_complete", "inspection_bound_reading_complete",
             "inspection_review_parent_complete"]:
    require(f"actual_deferred_trigger_{name}", any(f"TRIGGER {name} " in trigger and
            "DEFERRABLE INITIALLY DEFERRED" in trigger for trigger in triggers))

report = {
    "created_at": datetime.now(timezone.utc).isoformat(),
    "status": "FAIL" if errors else "PASS",
    "scope": "actual rollback-only 0291-0293 subset on nine cloned parents; not the complete ERP chain or production migration",
    "tests": checks,
    "errors": errors,
    "actual_result": result,
    "actual_rollback_readback": back,
    "source_before": source_before,
    "source_after": source_after,
    "artifacts": {name: {"path": path.relative_to(ROOT).as_posix(), "bytes": path.stat().st_size,
                         "sha256": sha(path)} for name, path in
                  {**paths, "actual_log": log, "rollback_readback": readback}.items()},
}
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "checks": len(checks), "errors": errors,
                  "scope": report["scope"]}, ensure_ascii=False))
raise SystemExit(bool(errors))
