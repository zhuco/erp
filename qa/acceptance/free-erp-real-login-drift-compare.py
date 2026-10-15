"""Compare one protected enterprise at all three immutable snapshot boundaries."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
tenant = "51868bb1-3fa6-4edc-a84f-87c6e95c18a4"
catalog = json.loads((BASE / "free-erp-real-login-timestamp-catalog.json").read_text(encoding="utf8"))
target = BASE / "free-erp-real-login-original-protected-drift.json"
if target.exists():
    raise SystemExit("refusing to replace original protected drift evidence")


def rows(path):
    with path.open("rb") as stream:
        for raw in stream:
            value = json.loads(raw)
            if value.get("tenant_id") == tenant:
                return value
    raise RuntimeError("protected enterprise absent " + str(path))


def canonical(table, values):
    normalized = []
    for old in values:
        row = dict(old)
        for column in catalog.get(table, []):
            if row.get(column) is not None:
                row[column] = datetime.fromisoformat(row[column]).astimezone(timezone.utc).isoformat(timespec="microseconds")
        normalized.append(row)
    return sorted(normalized, key=lambda value: json.dumps(value, sort_keys=True))


paths = [BASE / ("free-erp-real-login-" + stage + ".jsonl") for stage in ["before", "pre-cases", "original-third-after"]]
values = [rows(path) for path in paths]
changes = []
for index in [0, 1]:
    for table in sorted(values[index]["facts"]):
        old, new = [canonical(table, value["facts"][table]) for value in values[index:index + 2]]
        if old == new:
            continue
        changed_fields = sorted({key for a, b in zip(old, new) for key in set(a) | set(b) if a.get(key) != b.get(key)})
        changes.append({"from": paths[index].name, "to": paths[index + 1].name, "table": table,
                        "before_rows": len(old), "after_rows": len(new), "changed_fields": changed_fields,
                        "before_table_sha256": hashlib.sha256(json.dumps(old, sort_keys=True).encode()).hexdigest(),
                        "after_table_sha256": hashlib.sha256(json.dumps(new, sort_keys=True).encode()).hexdigest()})
report = {"utc": datetime.now(timezone.utc).isoformat(), "status": "CHANGED" if changes else "UNCHANGED",
          "scope": "One independently identified older enterprise; fields reported without credential values; no whitelist or data mutation",
          "tenant_id": tenant, "name": values[0]["name"], "changes": changes}
with target.open("x", encoding="utf8") as output:
    output.write(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(report, ensure_ascii=False))
