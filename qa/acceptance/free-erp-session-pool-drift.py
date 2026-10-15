"""Describe prior protected-data drift by field names, without credential values."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-session-pool-"
TARGET = BASE / (PREFIX + "repaired-first-protected-drift.json")
if TARGET.exists():
    raise SystemExit("refusing to replace drift evidence")
report = json.loads((BASE / (PREFIX + "repaired-first-independent.json")).read_text(encoding="utf-8-sig"))
tenants = set(report["older_enterprises_changed"])
catalog = json.loads((BASE / "free-erp-real-login-timestamp-catalog.json").read_text(encoding="utf-8-sig"))


def capture(name):
    retained,sha = {},hashlib.sha256()
    with (BASE / name).open("rb") as stream:
        for raw in stream:
            sha.update(raw)
            if raw.strip():
                row = json.loads(raw)
                if row.get("tenant_id") in tenants:
                    retained[row["tenant_id"]] = row["facts"]
    return retained,sha.hexdigest()


def canonical(table,values):
    rows = []
    for original in values:
        row = dict(original)
        for key in catalog.get(table,[]):
            if row.get(key) is not None:
                row[key] = datetime.fromisoformat(row[key]).astimezone(timezone.utc).isoformat(timespec="microseconds")
        rows.append(row)
    return sorted(rows,key=lambda row:json.dumps(row,sort_keys=True))


before,before_sha = capture(PREFIX + "original-after.jsonl")
after,after_sha = capture(PREFIX + "repaired-first-after.jsonl")
if before_sha != report["before_sha256"] or after_sha != report["after_sha256"]:
    raise SystemExit("snapshot identity mismatch")
tables = {}
for tenant in sorted(tenants):
    for table in sorted(set(before.get(tenant,{})) | set(after.get(tenant,{}))):
        a = canonical(table,before.get(tenant,{}).get(table,[]))
        b = canonical(table,after.get(tenant,{}).get(table,[]))
        if a==b:
            continue
        summary = tables.setdefault(table,{"enterprises":0,"before_rows":0,"after_rows":0,
                                            "added_columns":set(),"removed_columns":set(),"changed_fields":set()})
        summary["enterprises"] += 1
        summary["before_rows"] += len(a)
        summary["after_rows"] += len(b)
        a_keys = set().union(*(set(row) for row in a)) if a else set()
        b_keys = set().union(*(set(row) for row in b)) if b else set()
        summary["added_columns"].update(b_keys-a_keys)
        summary["removed_columns"].update(a_keys-b_keys)
        for key in a_keys & b_keys:
            if sorted((json.dumps(row.get(key),sort_keys=True) for row in a)) != sorted((json.dumps(row.get(key),sort_keys=True) for row in b)):
                summary["changed_fields"].add(key)
for summary in tables.values():
    for name in ["added_columns","removed_columns","changed_fields"]:
        summary[name] = sorted(summary[name])
output = {"utc":datetime.now(timezone.utc).isoformat(),"status":"CHANGED","older_enterprises_changed":len(tenants),
          "scope":"Historical snapshot field/count comparison only; no whitelist, data rollback or claim of unchanged old facts",
          "tables":tables,"before_sha256":before_sha,"after_sha256":after_sha}
with TARGET.open("x",encoding="utf8") as stream:
    stream.write(json.dumps(output,ensure_ascii=False,indent=2)+"\n")
print(json.dumps(output,ensure_ascii=False))
