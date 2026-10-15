"""Independently compare full protected facts and original release ledger ownership."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--original", action="store_true")
args = parser.parse_args()
if not re.fullmatch(r"[a-z0-9-]+", args.stage):
    raise SystemExit("invalid immutable verification stage")
destination = QA / f"free-erp-production-release-{args.stage}-independent.json"
if destination.exists():
    raise SystemExit("refusing to replace historical verification")
files = {kind: QA / f"free-erp-production-release-{args.stage}-{kind}{suffix}"
         for kind, suffix in [("before", ".jsonl"), ("after", ".jsonl"), ("test", ".log")]}
raw = files["test"].read_bytes()
log = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
fixtures = {}
for tid, case, actor, warehouse, sku, order, receipt, report, inspection in re.findall(
        r"QA production_release_fixture tenant=(\S+) case=(\S+) actor=(\S+) warehouse=(\S+) sku=(\S+) work_order=(\S+) receipt=(\S+) report=(\S+) inspection=(\S+)", log):
    fixtures[tid] = dict(case=case, actor=actor, warehouse=warehouse, sku=sku,
                         order=order, receipt=receipt, report=report, inspection=inspection)
http = []
for line in log.splitlines():
    match = re.search(r"QA production_release_http tenant=(\S+) path=(\S+) input=(.*) status=(\d+) response=(.*)", line)
    if match:
        http.append(dict(tenant=match[1], path=match[2], input=json.loads(match[3]),
                         status=int(match[4]), response=json.loads(match[5])))
errors = []
def require(condition, message):
    if not condition:
        errors.append(message)

before = {}
old_changes = []
owned = {}
headers = {}
counts = {}
for kind in ["before", "after"]:
    tally = Counter()
    with files[kind].open(encoding="utf8") as stream:
        headers[kind] = json.loads(next(stream))
        require(tuple(headers[kind][key] for key in ["database", "role", "host", "port"]) ==
                ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), f"{kind}: protected identity")
        for line in stream:
            row = json.loads(line)
            tid = row["tenant_id"]
            tally["tenants"] += 1
            tally["rows"] += sum(len(items) for items in row["facts"].values())
            digest = hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False,
                                               separators=(",", ":")).encode()).hexdigest()
            if kind == "before":
                before[tid] = digest
            else:
                if tid in before and before[tid] != digest:
                    old_changes.append(tid)
                if tid in fixtures:
                    owned[tid] = row["facts"]
    counts[kind] = dict(tally)
require(not old_changes, f"preexisting complete facts changed: {old_changes}")
require(set(fixtures) <= set(owned), "all new fixtures independently read back")
require(not(set(fixtures) & set(before)), "fixtures must be new enterprises")
summaries = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out; finished in (\S+)", log)
require(len(summaries) == 1, "one actually executed terminal summary")
terminal = None
if summaries:
    label, passed, failed, ignored, filtered, elapsed = summaries[-1]
    terminal = dict(label=label, passed=int(passed), failed=int(failed), ignored=int(ignored),
                    filtered=int(filtered), elapsed=elapsed)
    if args.original:
        require((terminal["passed"],terminal["failed"],terminal["ignored"]) == (0,9,0), "retain original nine actual failures")
    else:
        require(terminal["passed"] >= 9 and terminal["failed"] == 0 and terminal["ignored"] == 0,
                "actual regression must pass with no ignored cases")
if args.original:
    require(not headers["before"]["release_commands_present"] and not headers["after"]["release_commands_present"],
            "original release snapshot table was absent, not proof of correct replay")
else:
    require(headers["before"]["release_commands_present"] and headers["after"]["release_commands_present"],
            "new original result provenance must exist")
case_results = []
for tid, fixture in fixtures.items():
    facts = owned.get(tid, {})
    requests = [entry for entry in http if entry["tenant"] == tid]
    releases = [entry for entry in requests if entry["path"].endswith("/release")]
    inventories = {row["ledger_id"]:row for row in facts.get("inventory_ledgers", [])}
    stocks = {row["ledger_id"]:row for row in facts.get("shared_stock_ledgers", [])}
    quantities = {row["ledger_id"]:row for row in facts.get("mfg_qty_ledgers", [])}
    commands = facts.get("mfg_receipt_release_commands", [])
    children = facts.get("mfg_receipt_release_effects", [])
    if not args.original:
        for command in commands:
            snapshot = command["snapshot"]
            prefix = fixture["case"]
            require(command["tenant_id"] == tid and snapshot["tenant_id"] == tid, f"{prefix}: source enterprise")
            require(all(str(command[field]) == snapshot[field] for field in ["qty_micros","available_total_micros"]),
                    f"{prefix}: exact original quantities must be decimal strings")
            require(command["source_qty_ledger_id"] in quantities and command["source_stock_ledger_id"] in stocks
                    and command["source_inventory_ledger_id"] in inventories and command["source_projection_ledger_id"] in inventories,
                    f"{prefix}: actual original source ledgers")
            effects = [entry for entry in children if entry["request_id"] == command["request_id"]]
            require(len(effects) == 2 and {entry["quality_status"] for entry in effects} == {"HOLD","GOOD"},
                    f"{prefix}: two owned effects")
            require(sum(entry["delta_on_hand_micros"] for entry in effects) == 0
                    and sum(entry["delta_frozen_micros"] for entry in effects) == -command["qty_micros"],
                    f"{prefix}: release conserves physical quantity and only reduces held quantity")
            for effect in effects:
                canonical = inventories.get(effect["inventory_ledger_id"])
                projection = inventories.get(effect["projection_ledger_id"])
                stock = stocks.get(effect["stock_ledger_id"])
                quantity = quantities.get(effect["quantity_ledger_id"])
                require(all(item is not None for item in [canonical,projection,stock,quantity]), f"{prefix}: actual effect ledgers")
                if any(item is None for item in [canonical,projection,stock,quantity]):
                    continue
                require(all(item["tenant_id"] == tid and item["warehouse_id"] == command["warehouse_id"]
                            and item["sku_id"] == command["sku_id"] for item in [canonical,projection,stock,quantity]),
                        f"{prefix}: full owned effect key")
                require(canonical["ref_type"] == "shared_stock_ledger" and canonical["ref_id"] == stock["ledger_id"]
                        and projection["ref_type"] == "mfg_material_move" and projection["ref_id"] == stock["ledger_id"]
                        and quantity["move_id"] == stock["ledger_id"] and quantity["source_receipt_id"] == command["receipt_id"],
                        f"{prefix}: actual canonical and manufacturing projection provenance")
                require(canonical["delta_on_hand_micros"] == projection["delta_on_hand_micros"] == stock["delta_micros"]
                        == quantity["delta_micros"] == effect["delta_on_hand_micros"], f"{prefix}: exact effect arithmetic")
            matching = [entry for entry in releases if entry["input"]["request_id"] == command["request_id"] and entry["status"] == 200]
            if fixture["case"] == "internal-quality-guard":
                require(f"system_actor=true request_id={command['request_id']}" in log
                        and any(row["actor_type"] == "SYSTEM" and row["action"] == "MFG_RECEIPT_RELEASE"
                                for row in facts.get("operation_logs", [])),
                        f"{prefix}: actual legacy internal entry has mandatory SYSTEM provenance")
            else:
                require(matching and all(entry["response"].get("available_micros") == command["available_total_micros"] for entry in matching),
                        f"{prefix}: HTTP uses original result, including later receipt replay")
        for balance in facts.get("inventory_balances", []):
            canonical_rows = [row for row in inventories.values() if row["warehouse_id"] == balance["warehouse_id"]
                              and row["sku_id"] == balance["sku_id"] and row["ref_type"] != "mfg_material_move"]
            for bucket in ["on_hand","reserved","frozen","in_transit"]:
                known = balance[f"{bucket}_micros"] is not None and all(row[f"delta_{bucket}_micros"] is not None for row in canonical_rows)
                if known:
                    require(balance[f"{bucket}_micros"] == sum(row[f"delta_{bucket}_micros"] for row in canonical_rows),
                            f"{fixture['case']}: exact balance uses physical ledgers, excluding source projections")
                else:
                    require(fixture["case"] == "unknown-physical" and not commands
                            and all(entry["status"] == 409 for entry in releases),
                            f"{fixture['case']}: unknown source remains unknown and cannot be released")
        for entry in releases:
            if entry["status"] in [403,404,409]:
                require(not any(row["request_id"] == entry["input"]["request_id"] for row in commands),
                        f"{fixture['case']}: rejected release cannot own a committed result")
    case_results.append(dict(tenant_id=tid, **fixture, release_statuses=[entry["status"] for entry in releases],
                             command_count=len(commands), effect_count=len(children), http=requests))
expected = {"open-required","forged-pass","wrong-work-order","warehouse-scope","source-quantity",
            "missing-inspection","held-available","original-result","valid-fractional"}
require(expected <= {item["case"] for item in case_results}, "all nine business scenarios actually executed")
if not args.original:
    for name in ["operation_logs","audit_logs"]:
        matching = [item for item in case_results if item["case"] == f"fault-{name}"]
        if matching:
            require(matching[0]["release_statuses"] == [500,200,200], f"{name}: fault, original retry and replay")
            require(f"table={name}" in log and "actual_marker=true unchanged=true" in log,
                    f"{name}: prove actual required INSERT and complete rollback")
    for same in ["true","false"]:
        matching = [item for item in case_results if item["case"] == ("twenty-original" if same == "true" else "twenty-competing")]
        if matching:
            require(f"same={same} actual_waiting=20 results=20 commands=1" in log,
                    f"concurrency {same}: actual twenty-way PG blocker chain and one release")
    if any(item["case"] == "immutable-sources" for item in case_results):
        require("actual_rejected=6" in log, "six real parent/child/original receipt mutations must be rejected")
report = dict(created_at=datetime.now(timezone.utc).isoformat(), status="FAIL" if errors else "PASS",
              scope="complete protected facts and actual production release provenance; not performance or deployment",
              original_counterexamples=args.original, terminal_tests=terminal,
              protected_existing_enterprises=len(before), complete_selected_tables=len(next(iter(owned.values()),{})),
              preexisting_fact_changes=old_changes, counts=counts, cases=case_results, errors=errors)
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False)+"\n", encoding="utf8")
print(json.dumps({"status":report["status"],"tests":terminal,"old_enterprises":len(before),
                  "cases":len(case_results),"errors":errors},ensure_ascii=False))
raise SystemExit(bool(errors))
