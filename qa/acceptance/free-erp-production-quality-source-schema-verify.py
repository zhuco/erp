"""Verify protected QA-only schema application and every preexisting selected fact."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-production-quality-source-"
destination = QA / f"{PREFIX}migration-independent.json"
if destination.exists():
    raise SystemExit("refusing to replace historical schema evidence")
errors = []
old = {}
seen = set()
headers = {}
new_bindings = new_proofs = 0
for when, stage in [("before", "migration-retry-before"), ("after", "migration-after")]:
    with (QA / f"{PREFIX}{stage}.jsonl").open(encoding="utf8") as stream:
        headers[when] = json.loads(next(stream))
        if tuple(headers[when][key] for key in ["database", "role", "host", "port"]) != (
                "erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438):
            errors.append(f"{when}: protected QA identity not verified")
        for line in stream:
            row = json.loads(line)
            tid = row["tenant_id"]
            sha = hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False,
                                           separators=(",", ":")).encode()).hexdigest()
            if when == "before":
                old[tid] = sha
            else:
                seen.add(tid)
                if tid in old and sha != old[tid]:
                    errors.append(f"preexisting complete facts changed: {tid}")
                new_bindings += len(row["facts"]["shared_inspection_source_bindings"])
                new_proofs += len(row["facts"]["mfg_quality_release_proofs"])
if set(old) - seen:
    errors.append(f"preexisting enterprises removed: {sorted(set(old) - seen)}")
if headers["before"]["inspection_bindings_present"] or not headers["after"]["inspection_bindings_present"]:
    errors.append("typed source table must be absent before and present after this actual application")
expected = {"shared_inspection_source_bindings", "mfg_quality_release_proofs"}
before_names = {column["table"] for column in headers["before"]["catalog"]}
after_names = {column["table"] for column in headers["after"]["catalog"]}
if expected & before_names or not expected <= after_names:
    errors.append("both new typed schema tables must be independently read back")
if new_bindings or new_proofs:
    errors.append("migration must not invent source bindings or approvals for historical rows")
report = dict(created_at=datetime.now(timezone.utc).isoformat(), status="FAIL" if errors else "PASS",
              scope="guarded QA-only migration and complete old fact preservation, not whole migration chain or production deployment",
              protected_existing_enterprises=len(old), complete_selected_tables=52, added_schema_tables=sorted(expected),
              historical_bindings_inserted=new_bindings, historical_proofs_inserted=new_proofs, errors=errors,
              inputs={name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in [
                  "migrations/0291_inspection_source_bindings.sql", "qa/acceptance/free-erp-production-quality-source-apply.sql",
                  "qa/acceptance/free-erp-production-quality-source-readback.sql"]})
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({key: report[key] for key in ["status", "protected_existing_enterprises", "errors"]}))
raise SystemExit(bool(errors))
