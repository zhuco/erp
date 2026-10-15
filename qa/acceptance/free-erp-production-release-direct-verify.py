"""Read-only complete-fact verification of the existing registered HTTP release target."""
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
destination = base / "free-erp-production-release-direct-independent.json"
if destination.exists():
    raise SystemExit("refusing to replace historical independent evidence")
raw = (base / "free-erp-production-release-direct-test.log").read_bytes()
log = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
tenants = re.findall(r"production-release-http-tenant ([0-9a-f-]{36})", log)
summaries = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out; finished in (\S+)", log)
errors = []
checks = 0

def require(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)

require(len(tenants) == 1, "one actual HTTP test enterprise")
require(len(summaries) == 1, "one actual test terminal summary")
terminal = None
if summaries:
    label, passed, failed, ignored, filtered, elapsed = summaries[0]
    terminal = dict(label=label, passed=int(passed), failed=int(failed), ignored=int(ignored),
                    filtered=int(filtered), elapsed=elapsed)
    require((label, int(passed), int(failed), int(ignored)) == ("ok", 1, 0, 0), "existing registered target must actually pass")
before = {}
seen = set()
changed = []
facts = None
counts = {}
headers = {}
for stage in ["before", "after"]:
    tally = Counter()
    with (base / f"free-erp-production-release-direct-{stage}.jsonl").open(encoding="utf8") as stream:
        header = json.loads(next(stream))
        headers[stage] = header
        require(tuple(header[key] for key in ["database", "role", "host", "port"]) ==
                ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), f"{stage}: protected QA identity")
        require(header["release_commands_present"], f"{stage}: actual release provenance tables")
        for line in stream:
            row = json.loads(line)
            tenant = row["tenant_id"]
            tally["tenants"] += 1
            tally["rows"] += sum(len(items) for items in row["facts"].values())
            digest = hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False,
                                               separators=(",", ":")).encode()).hexdigest()
            if stage == "before":
                before[tenant] = digest
            else:
                seen.add(tenant)
                if tenant in before and before[tenant] != digest:
                    changed.append(tenant)
                if tenants and tenant == tenants[0]:
                    facts = row["facts"]
    counts[stage] = dict(tally)
require(not changed, f"complete preexisting enterprise facts changed: {changed}")
require(not(set(before) - seen), "all preexisting enterprises remain present")
require(bool(tenants) and tenants[0] not in before, "the scenario uses a new QA enterprise")
require(facts is not None and len(facts) == 45, "independently capture all 45 selected tables")
if facts is not None:
    require(len(facts["mfg_receipts"]) == 1, "one actual held receipt")
    require(len(facts["mfg_receipt_release_commands"]) == 1, "two concurrent commands admit one actual release")
    require(len(facts["mfg_receipt_release_effects"]) == 2, "one HOLD and one GOOD effect")
    commands = facts["mfg_receipt_release_commands"]
    inventory = {row["ledger_id"]: row for row in facts["inventory_ledgers"]}
    stock = {row["ledger_id"]: row for row in facts["shared_stock_ledgers"]}
    quantity = {row["ledger_id"]: row for row in facts["mfg_qty_ledgers"]}
    for command in commands:
        snapshot = command["snapshot"]
        require(command["qty_micros"] == command["available_total_micros"] == 6_000_000,
                "original committed receipt and available quantities are exactly six units")
        require(snapshot["qty_micros"] == snapshot["available_total_micros"] == "6000000",
                "original snapshot retains exact decimal strings")
        receipt = next((row for row in facts["mfg_receipts"] if row["receipt_id"] == command["receipt_id"]), None)
        require(receipt is not None and receipt["status"] == "RELEASED" and receipt["available_micros"] == 6_000_000,
                "actual receipt available state")
        require(command["source_qty_ledger_id"] in quantity and command["source_stock_ledger_id"] in stock
                and command["source_inventory_ledger_id"] in inventory and command["source_projection_ledger_id"] in inventory,
                "original quantity, stock and physical/projection ledger IDs exist")
        effects = [row for row in facts["mfg_receipt_release_effects"] if row["request_id"] == command["request_id"]]
        require({row["quality_status"] for row in effects} == {"HOLD", "GOOD"}, "two real quality buckets")
        require(sum(row["delta_on_hand_micros"] for row in effects) == 0
                and sum(row["delta_frozen_micros"] for row in effects) == -6_000_000,
                "release conserves physical quantity and unfreezes only the original hold")
        for effect in effects:
            physical = inventory.get(effect["inventory_ledger_id"])
            projection = inventory.get(effect["projection_ledger_id"])
            shared = stock.get(effect["stock_ledger_id"])
            manufacturing = quantity.get(effect["quantity_ledger_id"])
            require(all(row is not None for row in [physical, projection, shared, manufacturing]), "all effect ledgers exist")
            if any(row is None for row in [physical, projection, shared, manufacturing]):
                continue
            require(all(row["tenant_id"] == command["tenant_id"] and row["warehouse_id"] == command["warehouse_id"]
                        and row["sku_id"] == command["sku_id"] for row in [physical, projection, shared, manufacturing]),
                    "complete owned effect key")
            require(physical["ref_type"] == "shared_stock_ledger" and physical["ref_id"] == shared["ledger_id"]
                    and projection["ref_type"] == "mfg_material_move" and projection["ref_id"] == shared["ledger_id"]
                    and manufacturing["move_id"] == shared["ledger_id"] and manufacturing["source_receipt_id"] == command["receipt_id"],
                    "canonical and manufacturing projection refer to actual RETURNING IDs")
            require(physical["delta_on_hand_micros"] == projection["delta_on_hand_micros"]
                    == shared["delta_micros"] == manufacturing["delta_micros"] == effect["delta_on_hand_micros"],
                    "all exact effect arithmetic agrees")
        original = [row for row in facts["command_receipts"] if row["request_id"] in ["m11b-a", "m11b-b"]]
        require(len(original) == 1 and original[0]["status"] == "SUCCEEDED" and original[0]["effect_id"] == command["receipt_id"],
                "one globally committed original command")
    for balance in facts["inventory_balances"]:
        physical = [row for row in inventory.values() if row["ref_type"] != "mfg_material_move"
                    and row["warehouse_id"] == balance["warehouse_id"] and row["sku_id"] == balance["sku_id"]]
        for bucket, expected in [("on_hand", 6_000_000), ("frozen", 0), ("reserved", 0), ("in_transit", 0)]:
            require(balance[f"{bucket}_micros"] == expected == sum(row[f"delta_{bucket}_micros"] for row in physical),
                    f"physical {bucket} matches canonical ledgers, excluding manufacturing shadows")
    users = {row["user_id"]: row for row in facts["users"]}
    operation = [row for row in facts["operation_logs"] if row["action"] == "MFG_RECEIPT_RELEASE"]
    audit = [row for row in facts["audit_logs"] if row["action"] == "mfg.receipt.release"]
    require(len(operation) == len(audit) == 1 and operation[0]["actor_type"] == "USER"
            and operation[0]["actor_user_id"] in users and audit[0]["user_id"] == operation[0]["actor_user_id"],
            "mandatory logs bind the real current QA user")
report = dict(created_at=datetime.now(timezone.utc).isoformat(), status="FAIL" if errors else "PASS", checks=checks,
              scope="existing registered production release HTTP target and complete protected facts; not performance, real login or deployment",
              terminal_tests=terminal, protected_existing_enterprises=len(before), complete_selected_tables=45,
              preexisting_fact_changes=changed, missing_preexisting_enterprises=sorted(set(before)-seen),
              fixture_tenants=tenants, counts=counts, errors=errors)
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False)+"\n", encoding="utf8")
print(json.dumps({key: report[key] for key in ["status", "checks", "terminal_tests", "protected_existing_enterprises", "errors"]}))
raise SystemExit(bool(errors))
