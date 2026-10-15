"""Independent guarded PostgreSQL facts; original snapshots are never today's stock."""
from collections import Counter
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"


def decoded(path):
    data = path.read_bytes()
    return data.decode("utf-16" if data.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def evidence(name):
    path = base / name
    return {"path": path.relative_to(root).as_posix(), "sha256": digest(path)}


checks = 0
failures = []


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        failures.append(message)


def instant(value):
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc).isoformat()


def record(snapshot):
    balance = snapshot["balance"]
    return {"warehouse_id": balance["warehouse_id"], "sku_id": balance["sku_id"],
            "sku_code": None, "barcode": None, "cost_cent": None,
            **{k: balance[k] for k in ("on_hand", "reserved", "frozen", "in_transit")},
            "available": balance["available_whole"], "updated_at": instant(balance["updated_at"]),
            "quantities": balance["quantities"], "quantity_state": "precise"}


before_name = "free-erp-inventory-commands-qa-before-full.json"
after_name = "free-erp-inventory-commands-qa-after-full.json"
full_name = "free-erp-inventory-commands-full-native.log"
before = json.loads(decoded(base / before_name))
after = json.loads(decoded(base / after_name))
for data in (before, after):
    check((data["database"], data["role"], data["host"], data["port"]) ==
          ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), "protected identity")
old = {t["tenant_id"]: t for t in before["tenants"]}
now = {t["tenant_id"]: t for t in after["tenants"]}
for tenant_id, data in old.items():
    check(now.get(tenant_id) == data, f"existing QA tenant changed: {tenant_id}")

full = decoded(base / full_name)
matched = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s", full)
check(bool(matched) and tuple(map(int, matched[-1][:3])) == (116, 0, 0), "116 registered acceptance")
tests = re.findall(r"test inventory_command(?:s|_edges)::([^\s]+) \.\.\. ok", full)
check(len(tests) == 12 and len(set(tests)) == 12, "all 12 inventory command tests")
fixture_pattern = r"QA inventory command fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+)"
fixtures = {m[0]: {"actor": m[1], "warehouse": m[2], "sku": m[3]}
            for m in re.findall(fixture_pattern, full)}
check(bool(fixtures), "actual command fixtures")
check(all(t not in old for t in fixtures), "new isolated command fixtures")
counts = Counter()
valid_receipts = {}
for tenant_id, fixture in fixtures.items():
    tenant = now[tenant_id]
    check(tenant["name"] == "QA-INVENTORY-COMMANDS", f"owned fixture: {tenant_id}")
    ledgers = {r["ledger_id"]: r for r in tenant["ledgers"]}
    operations = {r["log_id"]: r for r in tenant["operations"]}
    audits = {r["audit_id"]: r for r in tenant["audits"]}
    commands = [r for r in tenant["command_receipts"] if r["movement_kind"] is not None]
    for receipt in tenant["command_receipts"]:
        if receipt["request_id"].startswith("legacy-"):
            check(receipt["movement_kind"] is None and receipt["actor_user_id"] is None
                  and receipt["original_result"] is None, f"legacy identity not fabricated: {receipt['request_id']}")
            counts["intentional_legacy"] += 1
    for receipt in commands:
        counts["movement_receipts"] += 1
        request_id = receipt["request_id"]
        if request_id.startswith("incomplete-"):
            check(receipt["status"] == "RUNNING" and receipt["original_result"] is None,
                  f"intentional incomplete receipt retained: {request_id}")
            counts["intentional_incomplete"] += 1
            continue
        if request_id.startswith("corrupt-"):
            check(receipt["original_result"] == {"version": 1}, f"intentional corrupt receipt retained: {request_id}")
            counts["intentional_corrupt"] += 1
            continue
        snapshot = receipt["original_result"]
        command = snapshot["input"]
        counts["committed_originals"] += 1
        valid_receipts[(tenant_id, request_id)] = receipt
        check(receipt["status"] == "SUCCEEDED" and snapshot["version"] == 1, f"committed version: {request_id}")
        for field in ("request_id", "warehouse_id", "sku_id", "actor_user_id"):
            check(receipt[field] == command[field], f"receipt binds {field}: {request_id}")
        check(command["tenant_id"] == tenant_id and command["kind"] == receipt["movement_kind"], f"tenant/kind: {request_id}")
        # Rust serializes the struct in declaration order, not a sorted JSON map.
        ordered = {k: command[k] for k in ("tenant_id", "actor_user_id", "request_id", "kind",
                   "warehouse_id", "sku_id", "quantity_delta", "reason", "reference_id")}
        canonical = json.dumps(["inventory-movement-v1", ordered], separators=(",", ":"), ensure_ascii=False).encode("utf8")
        check(hashlib.sha256(canonical).hexdigest() == receipt["payload_hash"], f"independent full fingerprint: {request_id}")
        check(snapshot["ledger_id"] == receipt["effect_id"], f"original effect: {request_id}")
        ledger = ledgers[snapshot["ledger_id"]]
        check(ledger["ref_type"] == "inventory_movement_command" and ledger["ref_id"] == request_id, f"source identity: {request_id}")
        check(ledger["warehouse_id"] == command["warehouse_id"] and ledger["sku_id"] == command["sku_id"], f"physical effect: {request_id}")
        check(ledger["delta_on_hand_micros"] == command["quantity_delta"] * 1_000_000, f"exact effect: {request_id}")
        check(all(ledger[k] == 0 for k in ("delta_reserved_micros", "delta_frozen_micros", "delta_in_transit_micros")), f"other buckets: {request_id}")
        q = snapshot["balance"]["quantities"]
        check(Decimal(q["available"]) == Decimal(q["on_hand"]) - Decimal(q["reserved"]) - Decimal(q["frozen"]), f"original availability: {request_id}")
        for bucket in ("on_hand", "reserved", "frozen", "in_transit", "available"):
            expected = snapshot["balance"]["available_whole" if bucket == "available" else bucket]
            check(int(Decimal(q[bucket])) == expected, f"original integer projection {bucket}: {request_id}")
        check(snapshot["balance"]["tenant_id"] == tenant_id and snapshot["balance"]["warehouse_id"] == command["warehouse_id"]
              and snapshot["balance"]["sku_id"] == command["sku_id"], f"original balance identity: {request_id}")
        metadata = {"request_id": request_id, "payload_hash": receipt["payload_hash"], "movement_kind": command["kind"],
                    "warehouse_id": command["warehouse_id"], "sku_id": command["sku_id"], "batch_id": None,
                    "qty_delta": command["quantity_delta"], "reason": command["reason"], "reference_id": command["reference_id"],
                    "reason_type": ledger["reason_type"], "quantities": q}
        operation = operations[snapshot["operation_log_id"]]
        audit = audits[snapshot["audit_id"]]
        check(operation["metadata"] == metadata and audit["after_json"] == metadata, f"both logs original quantities: {request_id}")
        check(operation["actor_user_id"] == command["actor_user_id"] and audit["user_id"] == command["actor_user_id"], f"both logs actor: {request_id}")
        check(len([r for r in ledgers.values() if r["ref_type"] == "inventory_movement_command" and r["ref_id"] == request_id]) == 1,
              f"one original movement: {request_id}")
    key_rows = [b for b in tenant["balances"] if b["warehouse_id"] == fixture["warehouse"] and b["sku_id"] == fixture["sku"]]
    check(len(key_rows) == 1, f"one physical balance: {tenant_id}")
    for balance in key_rows:
        for bucket in ("on_hand", "reserved", "frozen", "in_transit"):
            check(balance[f"{bucket}_micros"] == sum(l[f"delta_{bucket}_micros"] for l in ledgers.values()), f"independent ledger sum {bucket}: {tenant_id}")
    check(len(operations) == len(audits) == len([r for r in commands if (tenant_id, r["request_id"]) in valid_receipts]),
          f"no detached stock/log effects: {tenant_id}")

http_pattern = r"QA inventory command edge HTTP tenant=([a-f0-9-]+) action=(\S+) request=(.*?) status=(\d+) [^\n]*? body=(\{[^\n]*\})"
for tenant_id, action, request_text, status, response_text in re.findall(http_pattern, full):
    counts["edge_http"] += 1
    response = json.loads(response_text)
    request = json.loads(request_text)
    if status != "200":
        counts[f"http_{status}"] += 1
        continue
    key = (tenant_id, response["request_id"])
    check(key in valid_receipts, f"HTTP original receipt: {key}")
    if key not in valid_receipts:
        continue
    receipt = valid_receipts[key]
    expected = record(receipt["original_result"])
    actual = dict(response["record"])
    actual["updated_at"] = instant(actual["updated_at"])
    check(actual == expected, f"HTTP record is original snapshot: {key}")
    check(response["effect_id"] == receipt["effect_id"] and response["payload_hash"] == receipt["payload_hash"], f"HTTP identity/hash: {key}")
    check(response["audit_id"] == receipt["original_result"]["audit_id"]
          and response["operation_log_id"] == receipt["original_result"]["operation_log_id"], f"HTTP original log IDs: {key}")
    if action.startswith("commands/"):
        counts["successful_readbacks"] += 1
        check(response["replay"] is True, f"readback never executes: {key}")
    elif response["replay"] is True:
        counts["successful_retries"] += 1
    else:
        counts["successful_first_posts"] += 1

concurrent = re.findall(r"QA inventory command concurrent tenant=([a-f0-9-]+) action=(\w+) responses=20 original_effects=1", full)
rollback = re.findall(r"QA inventory command rollback tenant=([a-f0-9-]+) action=(\w+) table=(\w+) request_id=(\S+) schema=(\S+)", full)
check(len(concurrent) == 3 and {r[1] for r in concurrent} == {"inbounds", "outbounds", "adjustments"}, "three real 20-way retries")
check(len(rollback) == 6 and {(r[1], r[2]) for r in rollback} ==
      {(a, t) for a in ("inbounds", "outbounds", "adjustments") for t in ("operation_logs", "audit_logs")}, "six real required-log failures")
distinct = re.findall(r"QA inventory command distinct contention tenant=([a-f0-9-]+) requests=20 succeeded=3 conflicts=17", full)
check(len(distinct) == 1, "20 distinct outbound commands have three successes and 17 explicit conflicts")
if distinct:
    tenant = now[distinct[0]]
    check(len(tenant["command_receipts"]) == len(tenant["operations"]) == len(tenant["audits"]) == 3,
          "only three original effects and required log pairs after distinct contention")
    check(tenant["balances"][0]["on_hand_micros"] == 1_900_000,
          "distinct commands cannot exceed the available 3.1 base units")
counts["concurrent_groups"] = len(concurrent)
counts["distinct_contention_groups"] = len(distinct)
counts["actual_log_failures"] = len(rollback)
result = {"status": "FAIL" if failures else "PASS", "checks": checks, "failures": failures,
          "existing_tenants_unchanged": len(old), "new_command_fixtures": len(fixtures),
          "counts": dict(counts), "registered_tests": {"passed": 116, "failed": 0, "ignored": 0,
              "debug_seconds": float(matched[-1][3]) if matched else None},
          "evidence": [evidence(n) for n in (before_name, after_name, full_name)],
          "scope": "Guarded separate psql readback, all pre-existing tenant facts unchanged, full fingerprint, original ledger/log/snapshot bindings and HTTP results.",
          "limits": ["Synthetic bearer JWT; not real password, seats or revoked-session acceptance.",
                     "20 concurrent retries for one round per kind, not ERP-P05 100-round Release capacity.",
                     "HTTP response was deliberately discarded; no actual network commit disconnect or real UI recovery.",
                     "No batch/location/quality movement, real channel mapping or worker convergence certification."],
          "updated_at": datetime.now(timezone.utc).isoformat()}
destination = base / "free-erp-inventory-commands-independent.json"
if destination.exists():
    raise SystemExit("refusing to overwrite independent command evidence")
destination.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({k: result[k] for k in ("status", "checks", "existing_tenants_unchanged", "new_command_fixtures", "counts", "failures")}, ensure_ascii=False))
raise SystemExit(1 if failures else 0)
