"""Independent guarded psql facts and actual original structured HTTP results."""
from collections import Counter
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa" / "acceptance"


def decoded(path):
    data = path.read_bytes()
    return data.decode("utf-16" if data.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")


def instant(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00")).astimezone(timezone.utc).isoformat()


checks = 0
failures = []
counts = Counter()


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        failures.append(message)


before_name = "free-erp-inventory-location-commands-qa-before-full.json"
after_name = "free-erp-inventory-location-commands-qa-after-full.json"
log_name = "free-erp-inventory-location-commands-full-native.log"
before = json.loads(decoded(base / before_name))
after = json.loads(decoded(base / after_name))
for value in (before, after):
    check((value["database"], value["role"], value["host"], value["port"]) ==
          ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), "protected QA identity")
old = {t["tenant_id"]: t for t in before["tenants"]}
now = {t["tenant_id"]: t for t in after["tenants"]}
for identity, data in old.items():
    check(now.get(identity) == data, f"pre-existing tenant changed: {identity}")

log = decoded(base / log_name)
result = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s", log)
check(bool(result) and tuple(map(int, result[-1][:3])) == (125, 0, 0), "125 registered native tests")
tests = re.findall(r"test inventory_location_commands::(\S+) \.\.\. ok", log)
check(len(tests) == len(set(tests)) == 9, "all nine real structured scenarios")
fixture_re = r"QA location command fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+) location=([a-f0-9-]+)"
fixtures = {v[0]: dict(zip(("actor", "warehouse", "sku", "location"), v[1:])) for v in re.findall(fixture_re, log)}
check(len(fixtures) == 17 and all(identity not in old for identity in fixtures), "17 new isolated fixtures")
originals = {}
for identity, fixture in fixtures.items():
    tenant = now[identity]
    ledgers = {l["ledger_id"]: l for l in tenant["ledgers"]}
    shared = {l["ledger_id"]: l for l in tenant["shared_ledgers"]}
    operations = {l["log_id"]: l for l in tenant["operations"]}
    audits = {l["audit_id"]: l for l in tenant["audits"]}
    for receipt in tenant["command_receipts"]:
        snapshot = receipt["original_result"]
        command = snapshot["input"]
        location = command["location"]
        effect = snapshot["location_effect"]
        request_id = command["request_id"]
        originals[(identity, request_id)] = receipt
        counts["committed_originals"] += 1
        check(receipt["status"] == "SUCCEEDED" and snapshot["version"] == 1 and command["kind"] == "INBOUND", f"completed original: {request_id}")
        for field in ("request_id", "warehouse_id", "sku_id", "actor_user_id"):
            check(receipt[field] == command[field], f"receipt binds {field}: {request_id}")
        check(command["tenant_id"] == identity and command["actor_user_id"] == fixture["actor"], f"authenticated identity: {request_id}")
        ordered = {key: command[key] for key in ("tenant_id", "actor_user_id", "request_id", "kind", "warehouse_id", "sku_id", "quantity_delta", "reason", "reference_id")}
        ordered["location"] = {key: location[key] for key in ("location_id", "batch_code", "quality_status")}
        canonical = json.dumps(["inventory-movement-v1", ordered], ensure_ascii=False, separators=(",", ":")).encode("utf8")
        check(hashlib.sha256(canonical).hexdigest() == receipt["payload_hash"], f"independent full fingerprint: {request_id}")
        primary = ledgers[snapshot["ledger_id"]]
        projection = shared[effect["ledger_id"]]
        micros = command["quantity_delta"] * 1_000_000
        frozen = 0 if location["quality_status"] == "GOOD" else micros
        check(snapshot["ledger_id"] == receipt["effect_id"] and primary["ref_type"] == "shared_stock_ledger" and primary["ref_id"] == effect["ledger_id"], f"primary-projection-original identities: {request_id}")
        check(primary["delta_on_hand_micros"] == micros and primary["delta_frozen_micros"] == frozen and primary["delta_reserved_micros"] == primary["delta_in_transit_micros"] == 0, f"physical/quality quantities: {request_id}")
        check(primary["reason_type"] == "PURCHASE_RECEIPT", f"primary reason: {request_id}")
        for row in (primary, projection):
            check((row["warehouse_id"], row["sku_id"]) == (command["warehouse_id"], command["sku_id"]), f"physical key: {request_id}")
        check(all(projection[key] == location[key] for key in ("location_id", "batch_code", "quality_status")) and projection["bucket"] == "ON_HAND" and projection["delta_micros"] == micros and projection["request_id"] == request_id and projection["source_line"] == "", f"bound projection: {request_id}")
        check(instant(effect["created_at"]) == instant(projection["created_at"]) and Decimal(effect["on_hand"]) * 1_000_000 >= micros, f"original location result: {request_id}")
        balance = snapshot["balance"]
        q = balance["quantities"]
        check(Decimal(q["available"]) == Decimal(q["on_hand"]) - Decimal(q["reserved"]) - Decimal(q["frozen"]), f"original available: {request_id}")
        for bucket in ("on_hand", "reserved", "frozen", "in_transit", "available"):
            check(int(Decimal(q[bucket])) == balance["available_whole" if bucket == "available" else bucket], f"integer projection {bucket}: {request_id}")
        metadata = {"request_id": request_id, "payload_hash": receipt["payload_hash"], "movement_kind": "INBOUND", "warehouse_id": command["warehouse_id"], "sku_id": command["sku_id"], "batch_id": None, "qty_delta": command["quantity_delta"], "reason": command["reason"], "reference_id": command["reference_id"], "reason_type": "PURCHASE_RECEIPT", "quantities": q, "location": location}
        operation = operations[snapshot["operation_log_id"]]
        audit = audits[snapshot["audit_id"]]
        check(operation["metadata"] == metadata and audit["after_json"] == metadata, f"both full original logs: {request_id}")
        check(operation["actor_user_id"] == audit["user_id"] == command["actor_user_id"] and operation["action"] == "INVENTORY_INBOUND" and audit["action"] == "INVENTORY.PURCHASE_RECEIPT", f"original actors/actions: {request_id}")
    check(len(operations) == len(audits) == len(shared) == len(tenant["command_receipts"]), f"no detached effects or failed claims: {identity}")
    for balance in tenant["balances"]:
        relevant = [l for l in ledgers.values() if (l["warehouse_id"], l["sku_id"]) == (balance["warehouse_id"], balance["sku_id"])]
        for bucket in ("on_hand", "reserved", "frozen", "in_transit"):
            check(balance[f"{bucket}_micros"] == sum(l[f"delta_{bucket}_micros"] for l in relevant), f"independent primary {bucket} sum: {identity}")
    active_mapping = any(m["status"] == "ACTIVE" for m in tenant["channel_mappings"])
    expected_events = sum(r["original_result"]["input"]["location"]["quality_status"] == "GOOD" for r in tenant["command_receipts"]) if active_mapping else 0
    check(len(tenant["outbox"]) == expected_events, f"actual producer event count: {identity}")
    for event in tenant["outbox"]:
        counts["actual_outbox_events"] += 1
        check(event["status"] == "PENDING" and event["payload"]["quantity_scale"] == 1_000_000 and event["payload"]["available_micros"] == "1000000", f"exact pending producer payload: {identity}")

http_re = r"QA location command HTTP tenant=([a-f0-9-]+) actor=([a-f0-9-]+) action=(\S+) request=(.*?) headers=(\[[^\n]*?\]) status=(\d+) [^\n]*? body=(\{[^\n]*\})"
http = re.findall(http_re, log)
check(len(http) == 69, "69 actual HTTP outcomes")
groups = Counter()
for identity, actor, action, request_text, headers_text, status, response_text in http:
    counts[f"http_{status}"] += 1
    request = json.loads(request_text)
    headers = json.loads(headers_text)
    response = json.loads(response_text)
    if status != "200":
        if status == "500":
            check("check option" in response["message"] and any(t in response["message"] for t in ("operation_logs", "audit_logs")), "actual required-log failure reached")
        continue
    key = (identity, response["request_id"])
    check(key in originals, f"HTTP has original receipt: {key}")
    if key not in originals:
        continue
    receipt = originals[key]
    snapshot = receipt["original_result"]
    balance = snapshot["balance"]
    expected = {"warehouse_id": balance["warehouse_id"], "sku_id": balance["sku_id"], "sku_code": None, "barcode": None, "cost_cent": None, **{k: balance[k] for k in ("on_hand", "reserved", "frozen", "in_transit")}, "available": balance["available_whole"], "quantities": balance["quantities"], "quantity_state": "precise", "updated_at": instant(balance["updated_at"])}
    record = dict(response["record"])
    record["updated_at"] = instant(record["updated_at"])
    check(record == expected and response["location_record"] == snapshot["location_effect"] and response["location"] == snapshot["input"]["location"], f"HTTP returns both original snapshots: {key}")
    check(actor == snapshot["input"]["actor_user_id"] and response["effect_id"] == snapshot["ledger_id"] and response["ledger_id"] == snapshot["location_effect"]["ledger_id"] and response["audit_id"] == snapshot["audit_id"] and response["operation_log_id"] == snapshot["operation_log_id"] and response["payload_hash"] == receipt["payload_hash"], f"HTTP full original binding: {key}")
    if action.startswith("commands/"):
        check(response["replay"] is True, f"readback never writes: {key}")
        counts["successful_readbacks"] += 1
    else:
        check(request.get("request_id", headers[0] if headers else None) == response["request_id"], f"body/header original identity: {key}")
        counts["successful_retries" if response["replay"] else "successful_first_posts"] += 1
        groups[key] += 1
check(sum(amount == 20 for amount in groups.values()) == 1, "one real 20-way mixed-header/body original")
failure_re = r"QA location command failure tenant=([a-f0-9-]+) schema=(\S+) table=(\S+) quality=(\S+)"
faults = re.findall(failure_re, log)
check(len(faults) == 6 and {(v[2], v[3]) for v in faults} == {(table, quality) for table in ("operation_logs", "audit_logs") for quality in ("GOOD", "PENDING", "HOLD")}, "six genuine log failure scopes")
counts["actual_log_failures"] = len(faults)
check(counts["committed_originals"] == 18 and counts["actual_outbox_events"] == 2, "18 durable originals and two actual producer events")

report = {"status": "FAIL" if failures else "PASS", "updated_at": datetime.now(timezone.utc).isoformat(), "checks": checks, "failures": failures, "existing_tenants_unchanged": len(old), "new_location_fixtures": len(fixtures), "counts": dict(counts), "registered_native": {"passed": 125, "failed": 0, "ignored": 0, "debug_seconds": float(result[-1][3]) if result else None}, "evidence": [{"path": (base / name).relative_to(root).as_posix(), "sha256": hashlib.sha256((base / name).read_bytes()).hexdigest()} for name in (before_name, after_name, log_name)], "limits": ["Synthetic bearer authentication and synthetic mapping exercise local service facts; not real login, seats, platform synchronization or native/Web workflows.", "Debug functional duration and one concurrency round do not establish Release capacity.", "Existing unlinked shared history is retained; missing physical linkage is not guessed or automatically backfilled."]}
destination = base / "free-erp-inventory-location-commands-independent.json"
if destination.exists():
    raise SystemExit("refusing to replace independent location evidence")
with destination.open("x", encoding="utf8", newline="\n") as output:
    json.dump(report, output, ensure_ascii=False, indent=2)
    output.write("\n")
print(json.dumps({key: report[key] for key in ("status", "checks", "existing_tenants_unchanged", "new_location_fixtures", "counts", "failures")}, ensure_ascii=False))
raise SystemExit(1 if failures else 0)
