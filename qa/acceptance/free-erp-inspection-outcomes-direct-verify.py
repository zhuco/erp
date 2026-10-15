"""Independently verify the original zero-reading HTTP competition and all older QA facts."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-inspection-outcomes-"
output = QA / f"{PREFIX}direct-reading-independent.json"
if output.exists():
    raise SystemExit("refusing historical evidence overwrite")
paths = {"before": QA / f"{PREFIX}fixed-full-after.jsonl",
         "after": QA / f"{PREFIX}direct-reading-after.jsonl",
         "test": QA / f"{PREFIX}direct-reading-retry-test.log"}
raw = paths["test"].read_bytes()
log = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
errors = []
checks = 0


def require(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


results = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out; finished in (\S+)", log)
require(len(results) == 1 and results[0][:4] == ("ok", "1", "0", "0") and "QA controlled_test_exit=0" in log,
        "one actual registered passing case and process terminal marker")
pairs = re.findall(r"QA quality_source_direct case=reading tenant=(\S+) first_request=(\S+) first_status=(\d+) second_request=(\S+) second_status=(\d+)", log)
require(len(pairs) == 1, "both original actual HTTP responses required")
tid = pairs[0][0] if pairs else None
old = {}
seen = set()
owned = {}
tables = 0
for when in ["before", "after"]:
    with paths[when].open(encoding="utf8") as stream:
        header = json.loads(next(stream))
        require(tuple(header[k] for k in ["database", "role", "host", "port"]) ==
                ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), f"{when}: protected QA identity")
        for line in stream:
            row = json.loads(line)
            fingerprint = hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
            tables = len(row["facts"])
            if when == "before":
                old[row["tenant_id"]] = fingerprint
            else:
                seen.add(row["tenant_id"])
                if row["tenant_id"] in old:
                    require(old[row["tenant_id"]] == fingerprint, f"older complete facts changed: {row['tenant_id']}")
                if row["tenant_id"] == tid:
                    owned = row["facts"]
require(tables == 54 and not set(old) - seen, "all 54 selected tables and no older enterprises removed")
require(bool(owned) and tid not in old, "actual fresh direct enterprise independently read back")
if pairs and owned:
    _, first, first_status, second, second_status = pairs[0]
    statuses = {first:int(first_status), second:int(second_status)}
    require(sorted(statuses.values()) == [200, 422], "one original measurement and one duplicate-line refusal")
    commands = {r["request_id"]:r for r in owned["command_receipts"]}
    require(all(request in commands if status == 200 else request not in commands for request,status in statuses.items()), "only the successful original request has a receipt")
    readings = owned["mfg_quality_items"]
    events = owned["shared_inspection_reading_events"]
    require(len(readings) == len(events) == 1 and not owned["shared_inspection_reading_reviews"], "one immutable original fact and event, no fabricated review")
    if len(readings) == len(events) == 1:
        q,e = readings[0],events[0]
        require(q["reading_micros"] == 0 and q["reading_present"] and q["passed"] and e["verdict"] == "PASSED", "original zero remains a measured passing value")
        actor = next((u for u in owned["users"] if u["user_id"] == e["actor_user_id"]), {})
        require(actor.get("status") == "ACTIVE" and e["actor_source"] == "USER", "actual owned active submitting account")
        command = commands.get(e["request_id"], {})
        require(command.get("effect_id") == q["item_id"] == e["item_id"] and command.get("payload_hash") == e["payload_hash"], "original full command and event identity")
        bound = next((r for r in owned["shared_inspection_source_bindings"] if r["freeze_id"] == q["freeze_id"]), {})
        order = next((r for r in owned["mfg_work_orders"] if r["work_order_id"] == bound.get("source_id")), {})
        require(bound.get("source_kind") == "WORK_ORDER" and bound.get("work_order_id") == q["work_order_id"]
                and order.get("product_code") == bound.get("item_code"), "original actual work order and frozen product")

report = dict(created_at=datetime.now(timezone.utc).isoformat(), status="FAIL" if errors else "PASS",
              scope="Original registered zero-reading HTTP competition; no real password login or performance claim",
              tests=results, protected_existing_enterprises=len(old), selected_complete_tables=tables,
              independent_assertions=checks, tenant_id=tid, actual_http_pairs=pairs, errors=errors,
              inputs={name:{"path":p.relative_to(ROOT).as_posix(),"sha256":sha(p)} for name,p in paths.items()})
output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({k:report[k] for k in ["status", "tests", "protected_existing_enterprises", "independent_assertions", "errors"]}))
raise SystemExit(bool(errors))
