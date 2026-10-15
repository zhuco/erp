"""Verify the preserved unsafe HTTP effects against independent PostgreSQL captures."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-atomic-effects-"
destination = BASE / (PREFIX + "original-independent.json")
if destination.exists():
    raise SystemExit("refusing to replace historical evidence")
log_path = BASE / (PREFIX + "original-test.log")
log = log_path.read_text(encoding="utf-8-sig")
markers = re.findall(r"QA atomic_effect_result tenant=([a-f0-9-]+) case=([^ ]+) request=([^ ]+) status=(\d+) unchanged=(\w+) input=(.+?) response=(.+?) after=(.+)", log)
owned = {marker[0] for marker in markers}
errors, checks = [], 0
catalog = BASE / "free-erp-command-readback-timestamp-catalog.json"
columns = json.loads(catalog.read_text(encoding="utf-8-sig"))

def check(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)

def canonical(table, rows):
    result = []
    for source in rows:
        row = dict(source)
        for column in columns.get(table, []):
            if row.get(column) is not None:
                row[column] = datetime.fromisoformat(row[column]).astimezone(timezone.utc).isoformat(timespec="microseconds")
        result.append(row)
    return sorted(result, key=lambda row: json.dumps(row, sort_keys=True, ensure_ascii=False))

def read(stage):
    path = BASE / (PREFIX + stage + ".jsonl")
    fingerprints, retained, digest = {}, {}, hashlib.sha256()
    metadata = None
    with path.open("rb") as stream:
        for raw in stream:
            digest.update(raw)
            if not raw.strip():
                continue
            row = json.loads(raw)
            if "database" in row:
                metadata = row
                continue
            tenant = row["tenant_id"]
            check(tenant not in fingerprints, "unique enterprise " + tenant)
            check(len(row["facts"]) == 60, "complete selected catalog " + tenant)
            fingerprints[tenant] = hashlib.sha256(raw.rstrip(b"\r\n")).hexdigest()
            if tenant in owned:
                retained[tenant] = row
    check(metadata is not None and metadata.get("database") == "erp_shared_contract_test" and metadata.get("role") == "erp_grid_test"
          and metadata.get("host") == "127.0.0.1" and metadata.get("port") == 15438 and metadata.get("selected_tables") == 60,
          "protected QA identity " + stage)
    return fingerprints, retained, digest.hexdigest()

before, _, before_sha = read("original-before")
after, current, after_sha = read("original-after")
for tenant, fingerprint in before.items():
    check(after.get(tenant) == fingerprint, "all existing selected facts unchanged " + tenant)
check(len(owned) == 4 and len(markers) == 4, "all four original counterexamples executed")
check({marker[1] for marker in markers} == {"terminal-order", "cost-claim", "warehouse-scope", "quality-source"}, "complete original scope")
outcomes = []
for tenant, case, request, status, unchanged, input_json, response_json, logged_json in markers:
    input, response, logged = map(json.loads, [input_json, response_json, logged_json])
    captured = current.get(tenant, {})
    facts = captured.get("facts", {})
    check(tenant not in before and captured.get("name") == "QA-ERP-ATOMIC-EFFECTS", case + " isolated owned enterprise")
    check(int(status) == 200 and unchanged == "false" and response.get("status") == "SUCCEEDED", case + " unsafe HTTP and actual mutation")
    for table, rows in logged.items():
        check(table in facts and canonical(table, facts.get(table, [])) == canonical(table, rows), case + " independent SQL readback " + table)
    documents = [row for row in facts.get("atomic_source_documents", []) if row["document_id"] == response.get("effect_id")]
    events = [row for row in facts.get("atomic_posting_events", []) if row["document_id"] == response.get("effect_id")]
    receipts = [row for row in facts.get("command_receipts", []) if row["request_id"] == request]
    moves = facts.get("mfg_material_moves", [])
    check(len(documents) == len(events) == len(receipts) == len(moves) == 1, case + " one actual source event receipt and material effect")
    if documents and events and receipts and moves:
        check(documents[0]["amount_cent"] == events[0]["amount_cent"] == input["amount_cent"], case + " caller amount stored")
        check(receipts[0]["effect_id"] == documents[0]["document_id"] and receipts[0]["status"] == "SUCCEEDED", case + " actual receipt points to effect")
        check(moves[0]["qty_micros"] == 1_000_000 and moves[0]["kind"] == "ISSUE"
              and moves[0]["warehouse_id"] == input["warehouse_id"] and moves[0]["sku_id"] == input["sku_id"], case + " actual material physical identity")
        ledgers = [row for row in facts.get("inventory_ledgers", []) if row.get("ref_type") == "mfg_material_move" and row.get("ref_id") == moves[0]["move_id"]]
        check(len(ledgers) == 1 and ledgers[0].get("delta_on_hand_micros") == -1_000_000, case + " actual precise canonical stock effect")
    if case == "terminal-order":
        check(any(row["status"] == "CLOSED" for row in facts.get("mfg_work_orders", [])), "closed order was actually posted")
    if case == "cost-claim":
        check(any(row.get("cost_cent") == 1200 for row in facts.get("skus", [])) and input["amount_cent"] == 999999, "real unit cost differs from forged posted amount")
    if case == "warehouse-scope":
        check(not any(row["warehouse_id"] == input["warehouse_id"] for row in facts.get("user_warehouse_access", [])), "posted warehouse had no actor grant")
    if case == "quality-source":
        check(bool(facts.get("shared_inspection_freezes")) and bool(facts.get("shared_inspection_freeze_lines"))
              and not facts.get("shared_inspection_reading_events"), "real frozen inspection was unmeasured")
    outcomes.append({"tenant_id": tenant, "case": case, "actual_http": int(status), "request_id": request, "effect_id": response.get("effect_id")})
check("test result: FAILED. 0 passed; 4 failed; 0 ignored;" in log, "four actual failing assertions retained")
report = {"utc": datetime.now(timezone.utc).isoformat(), "status": "PASS" if not errors else "FAIL", "scope": "Independent confirmation of original unsafe outcomes; this is not repaired test success",
          "checks": checks, "existing_enterprises": len(before), "protected_tables": 60, "cases": outcomes, "errors": errors,
          "before_sha256": before_sha, "after_sha256": after_sha, "log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest(),
          "timestamp_catalog_sha256": hashlib.sha256(catalog.read_bytes()).hexdigest()}
with destination.open("x", encoding="utf8") as output:
    output.write(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({key: report[key] for key in ["status", "checks", "existing_enterprises", "errors"]}, ensure_ascii=False))
raise SystemExit(0 if not errors else 1)
