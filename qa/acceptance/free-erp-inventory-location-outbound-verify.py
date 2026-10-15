"""Independent guarded QA facts, linked quantities and actual structured HTTP."""
import argparse
from collections import Counter
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
parser = argparse.ArgumentParser()
parser.add_argument("--stage", choices=("focused", "full"), default="full")
args = parser.parse_args()
stage = args.stage
before_name = f"free-erp-inventory-location-outbound-qa-before-{stage}.json"
after_name = f"free-erp-inventory-location-outbound-qa-after-{stage}.json"
log_name = f"free-erp-inventory-location-outbound-{stage}.log"
destination = base / f"free-erp-inventory-location-outbound-{stage}-independent.json"
if destination.exists():
    raise SystemExit("refusing to overwrite historical independent readback")


def decoded(path):
    value = path.read_bytes()
    return value.decode("utf-16" if value.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")


def instant(value):
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc).isoformat()


def micros(value):
    number = Decimal(value) * 1_000_000
    if number != number.to_integral_value():
        raise ValueError("quantity exceeds six-digit precision")
    return int(number)


def formatted(value):
    sign = "-" if value < 0 else ""
    whole, fraction = divmod(abs(value), 1_000_000)
    return f"{sign}{whole}" + (f".{fraction:06d}".rstrip("0") if fraction else "")


checks = 0
failures = []
counts = Counter()


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        failures.append(message)


before = json.loads(decoded(base / before_name))
after = json.loads(decoded(base / after_name))
for value in (before, after):
    check((value["database"], value["role"], value["host"], value["port"]) ==
          ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), "protected QA identity")
old = {t["tenant_id"]: t for t in before["tenants"]}
now = {t["tenant_id"]: t for t in after["tenants"]}
for identity, original in old.items():
    check(now.get(identity) == original, f"pre-existing tenant changed: {identity}")

log = decoded(base / log_name)
results = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s", log)
expected_tests = 11 if stage == "focused" else 136
check(bool(results) and tuple(map(int, results[-1][:3])) == (expected_tests, 0, 0), "registered native result")
tests = re.findall(r"test inventory_location_outbound::(\S+) \.\.\. ok", log)
check(len(tests) == len(set(tests)) == 11, "all eleven structured outbound scenarios")
fixture_re = r"QA location outbound fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+) location=([a-f0-9-]+)"
fixtures = {values[0]: dict(zip(("actor", "warehouse", "sku", "location"), values[1:]))
            for values in re.findall(fixture_re, log)}
check(len(fixtures) == 12 and all(identity not in old for identity in fixtures), "twelve new owned fixtures")
originals = {}
linked_seed_ids = {}
for identity, source, location, bucket, quantity in re.findall(
        r"QA location outbound linked seed tenant=([a-f0-9-]+) ledger_id=([a-f0-9-]+) location=([a-f0-9-]+) bucket=(\S+) micros=(-?\d+)", log):
    linked_seed_ids[(identity, source)] = (location, bucket, int(quantity))
check(len(linked_seed_ids) == 3, "three actually linked synthetic opening/reservation effects")

for identity, fixture in fixtures.items():
    tenant = now[identity]
    check(tenant["name"] == "QA-INVENTORY-LOCATION-OUTBOUND", f"owned fixture: {identity}")
    ledgers = {row["ledger_id"]: row for row in tenant["ledgers"]}
    shared = {row["ledger_id"]: row for row in tenant["shared_ledgers"]}
    operations = {row["log_id"]: row for row in tenant["operations"]}
    audits = {row["audit_id"]: row for row in tenant["audits"]}
    used_primary = set()
    used_shared = set()
    for receipt in tenant["command_receipts"]:
        snapshot = receipt["original_result"]
        command = snapshot["input"]
        location = command["location"]
        effect = snapshot["location_effect"]
        request_id = command["request_id"]
        originals[(identity, request_id)] = receipt
        kind = command["kind"]
        counts[f"committed_{kind.lower()}"] += 1
        check(receipt["status"] == "SUCCEEDED" and snapshot["version"] == 1 and kind in ("INBOUND", "OUTBOUND"), f"completed original: {request_id}")
        for field in ("request_id", "warehouse_id", "sku_id", "actor_user_id"):
            check(receipt[field] == command[field], f"receipt binds {field}: {request_id}")
        check(command["tenant_id"] == identity and command["actor_user_id"] == fixture["actor"], f"authenticated identity: {request_id}")
        ordered = {key: command[key] for key in ("tenant_id", "actor_user_id", "request_id", "kind", "warehouse_id", "sku_id", "quantity_delta", "reason", "reference_id")}
        ordered["location"] = {key: location[key] for key in ("location_id", "batch_code", "quality_status")}
        canonical = json.dumps(["inventory-movement-v1", ordered], ensure_ascii=False, separators=(",", ":")).encode("utf8")
        check(hashlib.sha256(canonical).hexdigest() == receipt["payload_hash"], f"independent full fingerprint: {request_id}")
        primary = ledgers[snapshot["ledger_id"]]
        projection = shared[effect["ledger_id"]]
        used_primary.add(primary["ledger_id"])
        used_shared.add(projection["ledger_id"])
        delta = command["quantity_delta"] * 1_000_000
        reason = "PURCHASE_RECEIPT" if kind == "INBOUND" else "TRANSFER_OUT"
        frozen = delta if kind == "INBOUND" and location["quality_status"] != "GOOD" else 0
        check((delta > 0 if kind == "INBOUND" else delta < 0) and (kind != "OUTBOUND" or location["quality_status"] == "GOOD"), f"admitted direction/quality: {request_id}")
        check(snapshot["ledger_id"] == receipt["effect_id"] and primary["ref_type"] == "shared_stock_ledger" and primary["ref_id"] == effect["ledger_id"], f"bound primary and projection identities: {request_id}")
        check(primary["delta_on_hand_micros"] == delta and primary["delta_frozen_micros"] == frozen and primary["delta_reserved_micros"] == primary["delta_in_transit_micros"] == 0, f"exact authoritative quantities: {request_id}")
        check(primary["reason_type"] == reason, f"original source reason: {request_id}")
        for row in (primary, projection):
            check((row["warehouse_id"], row["sku_id"]) == (command["warehouse_id"], command["sku_id"]), f"physical key: {request_id}")
        check(all(projection[key] == location[key] for key in ("location_id", "batch_code", "quality_status")) and projection["bucket"] == "ON_HAND" and projection["delta_micros"] == delta and projection["request_id"] == request_id and projection["source_line"] == "", f"full projection key and signed effect: {request_id}")
        location_hand = micros(effect["on_hand"])
        check(location_hand >= max(0, delta) and formatted(location_hand) == effect["on_hand"] and instant(effect["created_at"]) == instant(projection["created_at"]), f"immutable original location result: {request_id}")
        balance = snapshot["balance"]
        q = balance["quantities"]
        check(micros(q["available"]) == micros(q["on_hand"]) - micros(q["reserved"]) - micros(q["frozen"]), f"original precise available: {request_id}")
        for bucket in ("on_hand", "reserved", "frozen", "in_transit", "available"):
            number = micros(q[bucket])
            check(number >= 0 and formatted(number) == q[bucket] and number // 1_000_000 == balance["available_whole" if bucket == "available" else bucket], f"exact/integer original {bucket}: {request_id}")
        metadata = {"request_id":request_id,"payload_hash":receipt["payload_hash"],"movement_kind":kind,
                    "warehouse_id":command["warehouse_id"],"sku_id":command["sku_id"],"batch_id":None,
                    "qty_delta":command["quantity_delta"],"reason":command["reason"],"reference_id":command["reference_id"],
                    "reason_type":reason,"quantities":q,"location":location}
        operation = operations[snapshot["operation_log_id"]]
        audit = audits[snapshot["audit_id"]]
        check(operation["metadata"] == audit["after_json"] == metadata, f"original required log facts: {request_id}")
        check(operation["actor_user_id"] == audit["user_id"] == command["actor_user_id"] and operation["action"] == f"INVENTORY_{kind}" and audit["action"] == f"INVENTORY.{reason}", f"authenticated log identities: {request_id}")
    for (tenant_id, source), (location, bucket, quantity) in linked_seed_ids.items():
        if tenant_id != identity:
            continue
        projection = shared[source]
        primary = [row for row in ledgers.values() if row["ref_type"] == "shared_stock_ledger" and row["ref_id"] == source]
        check(len(primary) == 1 and primary[0][f"delta_{bucket.lower()}_micros"] == quantity, f"synthetic seed is a unique primary projection: {source}")
        check(projection["location_id"] == location and projection["bucket"] == bucket and projection["delta_micros"] == quantity and projection["source_line"] == projection["request_id"] == "QA-SEED", f"actual seed dimensions: {source}")
        used_primary.add(primary[0]["ledger_id"])
        used_shared.add(source)
    unlinked = [row for source, row in shared.items() if source not in used_shared]
    check(all(row["source_line"] == row["request_id"] == "QA-UNLINKED" for row in unlinked), f"no detached command projections: {identity}")
    counts["unlinked_historical_fixtures"] += len(unlinked)
    check(used_primary == set(ledgers) and len(operations) == len(audits) == len(tenant["command_receipts"]), f"no duplicate/partial effects or failed command claims: {identity}")
    for balance in tenant["balances"]:
        rows = [row for row in ledgers.values() if (row["warehouse_id"],row["sku_id"]) == (balance["warehouse_id"],balance["sku_id"])]
        for bucket in ("on_hand","reserved","frozen","in_transit"):
            check(balance[f"{bucket}_micros"] == sum(row[f"delta_{bucket}_micros"] for row in rows), f"independent physical {bucket} sum: {identity}")
    for event in tenant["outbox"]:
        counts["actual_outbox_events"] += 1
        check(event["status"] == "PENDING" and event["payload"]["quantity_scale"] == 1_000_000 and event["payload"]["available_micros"] == "2000000", f"real exact outbound Outbox: {identity}")

http_re = r"QA location command HTTP tenant=([a-f0-9-]+) actor=([a-f0-9-]+) action=(\S+) request=(.*?) headers=(\[[^\n]*?\]) status=(\d+) [^\n]*? body=(\{[^\n]*\})"
http = [row for row in re.findall(http_re, log) if row[0] in fixtures]
check(len(http) == 98, "98 actual owned structured HTTP outcomes")
for identity, actor, action, request_text, headers_text, status, result_text in http:
    counts[f"http_{status}"] += 1
    request = json.loads(request_text)
    headers = json.loads(headers_text)
    response = json.loads(result_text)
    if status != "200":
        if status == "500":
            check("check option" in response["message"] and any(table in response["message"] for table in ("operation_logs","audit_logs")), "actual named log fault reached")
        continue
    receipt = originals[(identity, response["request_id"])]
    snapshot = receipt["original_result"]
    command = snapshot["input"]
    check(actor == command["actor_user_id"] and response["payload_hash"] == receipt["payload_hash"], "HTTP binds authenticated original fingerprint")
    for response_field, snapshot_field in (("effect_id","ledger_id"),("operation_log_id","operation_log_id"),("audit_id","audit_id")):
        check(response[response_field] == snapshot[snapshot_field], f"HTTP binds original {response_field}")
    check(response["location"] == command["location"] and response["ledger_id"] == snapshot["location_effect"]["ledger_id"] and response["location_record"]["on_hand"] == snapshot["location_effect"]["on_hand"], "HTTP keeps original location result")
    check(instant(response["location_record"]["created_at"]) == instant(snapshot["location_effect"]["created_at"]) and instant(response["record"]["updated_at"]) == instant(snapshot["balance"]["updated_at"]), "HTTP keeps both original timestamps")
    check(response["qty_micros"] == command["quantity_delta"] * 1_000_000 and response["record"]["quantities"] == snapshot["balance"]["quantities"], "HTTP original signed quantity and warehouse result")
    if action.startswith("commands/"):
        counts["successful_gets"] += 1
        check(response["replay"] is True and action == f"commands/{command['request_id']}", "authenticated original GET identity")
    else:
        counts["successful_post_retries" if response["replay"] else "successful_first_posts"] += 1
        check(request.get("request_id", headers[0] if headers else None) == command["request_id"], "body/header original identity")
        check(request["quantity"] == abs(command["quantity_delta"]) and all(request[key] == command["location"][key] for key in ("location_id","batch_code","quality_status")), "HTTP full original intent")

check((counts["committed_inbound"],counts["committed_outbound"],counts["actual_outbox_events"],counts["unlinked_historical_fixtures"]) == (18,11,2,1), "independent actual command/effect/event totals")
check(counts["http_500"] == 2 and counts["successful_first_posts"] == 29 and counts["successful_gets"] == 2, "actual failures, unique first submissions and readbacks")
report = {"updated_at":datetime.now(timezone.utc).isoformat(),"status":"FAIL" if failures else "PASS",
          "stage":stage,"checks":checks,"failures":failures,"existing_tenants_unchanged":len(old),
          "new_owned_fixtures":len(fixtures),"counts":dict(counts),
          "inputs":{name:hashlib.sha256((base/name).read_bytes()).hexdigest() for name in (before_name,after_name,log_name)},
          "limits":"Synthetic linked opening/reservation and mapping fixtures prove local transactional behavior; not full reserve/release services, external dispatch, real password sessions, Release performance or stable complete-source certification."}
destination.write_text(json.dumps(report,indent=2,ensure_ascii=False)+"\n",encoding="utf8")
print(json.dumps({key:report[key] for key in ("status","checks","existing_tenants_unchanged","new_owned_fixtures","counts","failures")},ensure_ascii=False))
if failures:
    raise SystemExit(1)
