"""Independent PostgreSQL readback for complete quality admission at physical consumers."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--before", required=True)
parser.add_argument("--after", required=True)
parser.add_argument("--original", action="store_true")
parser.add_argument("--tests", type=int, default=4)
args = parser.parse_args()
if not re.fullmatch(r"[a-z0-9-]+", args.stage):
    raise SystemExit("invalid immutable stage")
destination = BASE / f"free-erp-inspection-gates-{args.stage}-independent.json"
if destination.exists():
    raise SystemExit("refusing to replace historical evidence")
log_path = BASE / f"free-erp-inspection-gates-{args.stage}-test.log"
log = log_path.read_text(encoding="utf-8-sig")
fixtures = re.findall(r"QA inspection_gate_fixture tenant=([a-f0-9-]+) case=([a-z0-9-]+) work_order=([a-f0-9-]+)", log)
markers = re.findall(r"QA inspection_gate_result tenant=([a-f0-9-]+) case=([a-z0-9-]+) request=([a-f0-9-]+) expected=(\d+) actual=(\d+) unchanged=(true|false)", log)
errors = []
assertions = 0

def check(condition, message):
    global assertions
    assertions += 1
    if not condition:
        errors.append(message)

def capture(path, retain):
    fingerprints, kept, metadata = {}, {}, None
    digest = hashlib.sha256()
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
            check(tenant not in fingerprints, f"duplicate snapshot tenant: {tenant}")
            fingerprints[tenant] = hashlib.sha256(raw.rstrip(b"\r\n")).hexdigest()
            if tenant in retain:
                kept[tenant] = row
    check(metadata is not None and metadata.get("database") == "erp_shared_contract_test"
          and metadata.get("role") == "erp_grid_test" and metadata.get("port") == 15438
          and metadata.get("host") == "127.0.0.1", "protected independent database identity")
    return fingerprints, kept, digest.hexdigest()

owned = {tenant for tenant, _, _ in fixtures}
before_path, after_path = ROOT / args.before, ROOT / args.after
before, _, before_sha = capture(before_path, set())
after, current, after_sha = capture(after_path, owned)
for tenant, fingerprint in before.items():
    check(after.get(tenant) == fingerprint, f"existing tenant full selected facts changed: {tenant}")
core_cases = {"gate-sale-empty", "gate-sale-partial", "gate-material-empty", "gate-direct-receipt-empty"}
check(core_cases <= {case for _, case, _ in fixtures}, "all four actual core fixtures executed")
check(core_cases <= {case for _, case, *_ in markers}, "all four actual core HTTP results recorded")
core = []
for tenant, case, request, expected, actual, unchanged in markers:
    if case not in core_cases:
        continue
    check(tenant not in before and tenant in current, f"{case}: isolated fresh fixture")
    check(int(actual) == (200 if args.original else int(expected)), f"{case}: actual HTTP status")
    check(unchanged == ("false" if args.original else "true"), f"{case}: actual before/after marker")
    row = current.get(tenant, {})
    facts = row.get("facts", {})
    check(row.get("name") == "QA-ERP-INSPECTION-GATE", f"{case}: fixture ownership label")
    freezes = facts.get("shared_inspection_freezes", [])
    check(len(freezes) == 1, f"{case}: actual legacy freeze")
    templates = facts.get("shared_inspection_template_lines", [])
    check({line["item_code"] for line in templates} == {"WIDTH", "HEIGHT"}
          and all(line["required"] and line["sample_count"] == 1 for line in templates), f"{case}: owned two-project template")
    lines = facts.get("shared_inspection_freeze_lines", [])
    check(len(lines) == (1 if case == "gate-sale-partial" else 0), f"{case}: actual incomplete copy")
    check(not facts.get("shared_inspection_source_bindings"), f"{case}: no invented historical binding")
    readings = facts.get("mfg_quality_items", [])
    check(len(readings) == (1 if case == "gate-sale-partial" else 0), f"{case}: actual historical readings")
    if case == "gate-sale-partial" and readings:
        check(readings[0]["reading_present"] and readings[0]["passed"]
              and readings[0]["reading_micros"] == 1_500_000 and readings[0]["work_order_id"] is None,
              f"{case}: visible pass cannot replace absent HEIGHT")
    receipts = [command for command in facts.get("command_receipts", []) if command["request_id"] == request]
    check(len(receipts) == (1 if args.original else 0), f"{case}: persisted original command count")
    if receipts:
        check(receipts[0]["status"] == "SUCCEEDED", f"{case}: unsafe original committed success")
    if case.startswith("gate-sale"):
        deliveries = facts.get("shared_sales_deliveries", [])
        check(len(deliveries) == 1 and deliveries[0]["stage"] == ("SHIPPED" if args.original else "ISSUED"), f"{case}: physical delivery stage")
        if freezes and deliveries:
            check(freezes[0]["source_kind"] == "SALES_ORDER" and freezes[0]["source_id"] == deliveries[0]["so_id"], f"{case}: real sales source ownership")
        stock = facts.get("shared_stock_ledgers", [])
        good = [entry for entry in stock if entry["quality_status"] == "GOOD"]
        check(len(good) == 4 and sorted((entry["bucket"],entry["delta_micros"]) for entry in good)
              == [("ON_HAND",-1_000_000),("ON_HAND",1_000_000),("RESERVED",-1_000_000),("RESERVED",1_000_000)],
              f"{case}: actual trade opening, reservation and review precise effects")
        for entry in stock:
            linked = [ledger for ledger in facts.get("inventory_ledgers", [])
                      if ledger.get("ref_type") == "shared_stock_ledger" and ledger.get("ref_id") == entry["ledger_id"]]
            check(len(linked) == 1, f"{case}: one canonical inventory ledger per physical stock projection")
            if linked:
                field = "delta_on_hand_micros" if entry["bucket"] == "ON_HAND" else "delta_reserved_micros"
                check(linked[0].get(field) == entry["delta_micros"], f"{case}: actual canonical exact quantity")
                check(linked[0]["warehouse_id"] == entry["warehouse_id"] and linked[0]["sku_id"] == entry["sku_id"],
                      f"{case}: canonical physical key ownership")
                if entry["bucket"] == "ON_HAND" and entry["quality_status"] != "GOOD":
                    check(linked[0].get("delta_frozen_micros") == entry["delta_micros"], f"{case}: held physical stock stays frozen")
        projection_ids = set()
        for receipt in facts.get("mfg_receipts", []):
            copies = [ledger for ledger in facts.get("inventory_ledgers", [])
                      if ledger.get("ref_type") == "mfg_material_move"
                      and ledger.get("ref_id") == receipt["receipt_id"] and ledger.get("reason_type") == "MFG_RECEIPT"]
            sources = [entry for entry in stock if entry.get("source_line") == receipt["receipt_id"]
                       and entry["bucket"] == "ON_HAND" and entry["delta_micros"] == receipt["qty_micros"]]
            check(len(copies) == len(sources) == 1, f"{case}: receipt compatibility projection has one real source")
            if len(copies) == len(sources) == 1:
                canonical = [ledger for ledger in facts.get("inventory_ledgers", [])
                             if ledger.get("ref_type") == "shared_stock_ledger"
                             and ledger.get("ref_id") == sources[0]["ledger_id"]]
                exact = len(canonical) == 1 and all(copies[0].get(key) == canonical[0].get(key)
                    for key in ["warehouse_id","sku_id","reason_type","occurred_at"]
                    + [f"delta_{field}{suffix}" for field in ["on_hand","reserved","frozen","in_transit"]
                       for suffix in ["","_micros"]])
                check(exact, f"{case}: compatibility copy matches the actual canonical receipt ledger in every quantity field")
                if exact:
                    projection_ids.add(copies[0]["ledger_id"])
        for balance in facts.get("inventory_balances", []):
            ledgers = [ledger for ledger in facts.get("inventory_ledgers", [])
                       if ledger["warehouse_id"] == balance["warehouse_id"] and ledger["sku_id"] == balance["sku_id"]
                       and ledger["ledger_id"] not in projection_ids]
            for field in ["on_hand","reserved","frozen","in_transit"]:
                total = sum(ledger[f"delta_{field}_micros"] if ledger.get(f"delta_{field}_micros") is not None
                            else ledger[f"delta_{field}"] * 1_000_000 for ledger in ledgers)
                check(balance.get(f"{field}_micros") == total, f"{case}: independently reconstructed {field} balance")
    elif case == "gate-material-empty":
        moves = facts.get("mfg_material_moves", [])
        check(len(moves) == (1 if args.original else 0), f"{case}: material move count")
        if moves:
            check(moves[0]["kind"] == "ISSUE" and moves[0]["qty_micros"] == 100_000, f"{case}: unsafe original precise issue")
    elif case == "gate-direct-receipt-empty":
        direct = [receipt for receipt in facts.get("mfg_receipts", []) if receipt["status"] == "RELEASED"]
        check(len(direct) == (1 if args.original else 0), f"{case}: directly available receipt count")
        if direct:
            check(direct[0]["qty_micros"] == direct[0]["available_micros"] == 100_000, f"{case}: unsafe original available quantity")
    check(len(facts.get("mfg_work_orders", [])) == 1 and facts["mfg_work_orders"][0]["product_code"] == "QA-P", f"{case}: real manufacturing owner")
    check(len(facts.get("users", [])) == 1 and facts["users"][0]["status"] == "ACTIVE", f"{case}: real active tenant login")
    core.append({"tenant":tenant,"case":case,"request_id":request,"expected":int(expected),"actual":int(actual),"unsafe_original":args.original})

extended = []
if not args.original and args.tests >= 14:
    required_cases = core_cases | {
        "gate-sale-lowercase-complete", "gate-material-complete", "gate-direct-receipt-complete",
        "gate-no-freeze", "gate-template-version", "gate-other-batch", "gate-sale-review-chain",
        "gate-sale-owner-race", "gate-review-zero-available", "gate-reservation-release",
    }
    check(len(fixtures) == 14 and {case for _, case, _ in fixtures} == required_cases,
          "all fourteen fresh registered fixture groups executed exactly once")
    http_events = []
    for tenant, path, raw_input, status, raw_response in re.findall(
            r"QA (?:inspection_gate|production_release)_http tenant=([a-f0-9-]+) path=(\S+) input=(.*?) status=(\d+) response=(.+)", log):
        try:
            http_events.append((tenant, path, json.loads(raw_input), int(status), json.loads(raw_response)))
        except json.JSONDecodeError:
            errors.append(f"unparseable actual HTTP event: {tenant} {path}")

    def verify_precise(facts, case):
        stock = facts.get("shared_stock_ledgers", [])
        ledgers = facts.get("inventory_ledgers", [])
        projection_ids = set()
        for entry in stock:
            linked = [item for item in ledgers if item.get("ref_type") == "shared_stock_ledger"
                      and item.get("ref_id") == entry["ledger_id"]]
            check(len(linked) == 1, f"{case}: each physical projection has one canonical ledger")
            if len(linked) == 1:
                expected = {name: 0 for name in ["on_hand", "reserved", "frozen", "in_transit"]}
                field = {"ON_HAND":"on_hand", "RESERVED":"reserved", "FROZEN":"frozen", "IN_TRANSIT":"in_transit"}.get(entry["bucket"])
                check(field is not None, f"{case}: this fixture uses a physical inventory bucket")
                if field is not None:
                    expected[field] = entry["delta_micros"]
                    if field == "on_hand" and entry["quality_status"] != "GOOD":
                        expected["frozen"] = entry["delta_micros"]
                    for name, quantity in expected.items():
                        check(linked[0].get(f"delta_{name}_micros") == quantity,
                              f"{case}: exact {name} canonical movement")
                check(all(linked[0][name] == entry[name] for name in ["tenant_id", "warehouse_id", "sku_id"]),
                      f"{case}: canonical enterprise and inventory key")
        for receipt in facts.get("mfg_receipts", []):
            copies = [item for item in ledgers if item.get("ref_type") == "mfg_material_move"
                      and item.get("ref_id") == receipt["receipt_id"] and item.get("reason_type") == "MFG_RECEIPT"]
            sources = [entry for entry in stock if entry.get("source_line") == receipt["receipt_id"]
                       and entry["bucket"] == "ON_HAND" and entry["delta_micros"] == receipt["qty_micros"]]
            check(len(copies) == len(sources) == 1, f"{case}: one receipt compatibility projection with real source")
            if len(copies) == len(sources) == 1:
                canonical = [item for item in ledgers if item.get("ref_type") == "shared_stock_ledger"
                             and item.get("ref_id") == sources[0]["ledger_id"]]
                exact = len(canonical) == 1 and all(copies[0].get(key) == canonical[0].get(key)
                    for key in ["tenant_id", "warehouse_id", "sku_id", "reason_type", "occurred_at"]
                    + [f"delta_{field}{suffix}" for field in ["on_hand", "reserved", "frozen", "in_transit"]
                       for suffix in ["", "_micros"]])
                check(exact, f"{case}: receipt copy is exactly its actual canonical projection")
                if exact:
                    projection_ids.add(copies[0]["ledger_id"])
        for balance in facts.get("inventory_balances", []):
            selected = [entry for entry in ledgers if entry["warehouse_id"] == balance["warehouse_id"]
                        and entry["sku_id"] == balance["sku_id"] and entry["ledger_id"] not in projection_ids]
            for field in ["on_hand", "reserved", "frozen", "in_transit"]:
                total = sum(entry[f"delta_{field}_micros"] if entry.get(f"delta_{field}_micros") is not None
                            else entry[f"delta_{field}"] * 1_000_000 for entry in selected)
                check(balance.get(f"{field}_micros") == total, f"{case}: independent complete {field} balance")
            check(balance["on_hand_micros"] - balance["reserved_micros"] - balance["frozen_micros"] >= 0,
                  f"{case}: nonnegative actual available quantity")

    for tenant, case, work_order in fixtures:
        row = current.get(tenant, {})
        facts = row.get("facts", {})
        check(tenant not in before and row.get("name") == "QA-ERP-INSPECTION-GATE", f"{case}: fresh owned fixture")
        verify_precise(facts, case)
        commands = facts.get("command_receipts", [])
        freezes = facts.get("shared_inspection_freezes", [])
        moves = facts.get("mfg_material_moves", [])
        deliveries = facts.get("shared_sales_deliveries", [])
        actual_http = [event for event in http_events if event[0] == tenant]
        if case in {"gate-sale-lowercase-complete", "gate-material-complete", "gate-direct-receipt-complete"}:
            check(len(freezes) == 1 and not facts.get("shared_inspection_source_bindings"), f"{case}: no invented legacy binding")
            lines = facts.get("shared_inspection_freeze_lines", [])
            templates = facts.get("shared_inspection_template_lines", [])
            fields = ["item_code", "required", "min_micros", "max_micros", "sample_count"]
            normalize = lambda values: sorted(tuple(value.get(field) for field in fields) for value in values)
            check(len(lines) == 2 and normalize(lines) == normalize(templates), f"{case}: independently complete owned template copy")
            check(len(facts.get("mfg_quality_items", [])) == 2 and all(item["passed"] and item["reading_present"]
                  and item["reading_micros"] == 1_500_000 for item in facts["mfg_quality_items"]), f"{case}: two actual complete legacy readings")
        if case == "gate-material-complete":
            check(sorted((move["kind"], move["qty_micros"]) for move in moves) == [("ISSUE",100_000),("SUPPLEMENT",100_000)],
                  f"{case}: precisely one ISSUE and one SUPPLEMENT")
            events = [event for event in actual_http if event[1].endswith("/materials")]
            check(len(events) == 4 and all(event[3] == 200 for event in events), f"{case}: both actual writes and original replays")
            for request in {event[2]["request_id"] for event in events}:
                check(len([item for item in commands if item["request_id"] == request]) == 1, f"{case}: one original command per material intent")
        elif case == "gate-no-freeze":
            check(not freezes and len(moves) == 1 and moves[0]["kind"] == "ISSUE" and moves[0]["qty_micros"] == 100_000,
                  f"{case}: actual no-freeze ISSUE stays valid")
        elif case == "gate-template-version":
            rejected = re.findall(rf"QA inspection_gate_extra_rejected tenant={tenant} case=gate-template-version request=([a-f0-9-]+) status=422 unchanged=true", log)
            check(len(rejected) == 1 and len(freezes) == 1 and freezes[0]["version_no"] == 99 and not moves,
                  f"{case}: actual version mismatch rejects without material effect")
            if rejected:
                check(not any(item["request_id"] == rejected[0] for item in commands), f"{case}: rejected request has no command")
        elif case in {"gate-direct-receipt-complete", "gate-other-batch"}:
            released = [receipt for receipt in facts.get("mfg_receipts", []) if receipt["status"] == "RELEASED"]
            check(len(released) == 1 and released[0]["qty_micros"] == released[0]["available_micros"] == 100_000,
                  f"{case}: actual exact available receipt")
            if case == "gate-other-batch":
                check(len(freezes) == 1 and freezes[0]["batch_code"] == "QA-OTHER-BATCH" and not facts.get("shared_inspection_freeze_lines"),
                      f"{case}: distinct batch incomplete evidence stays scoped")
        elif case in {"gate-sale-lowercase-complete", "gate-sale-review-chain"}:
            ships = [event for event in actual_http if event[2].get("action") == "SHIP"]
            expected_statuses = [200,200] if case.endswith("complete") else [409,409,200,200]
            check([event[3] for event in ships] == expected_statuses, f"{case}: actual sequential SHIP results")
            check(len(deliveries) == 1 and deliveries[0]["stage"] == "SHIPPED", f"{case}: actual final SHIPPED stage")
            check(len({event[2]["request_id"] for event in ships}) == 1, f"{case}: same rejected/original SHIP request identity")
            if ships:
                check(len([item for item in commands if item["request_id"] == ships[0][2]["request_id"]]) == 1,
                      f"{case}: one actual committed SHIP command")
                check(ships[-1][4].get("replay") is True, f"{case}: actual original replay response")
            if case == "gate-sale-review-chain":
                events, reviews = facts.get("shared_inspection_reading_events", []), facts.get("shared_inspection_reading_reviews", [])
                check(len(events) == 3 and len(reviews) == 2 and {item["decision"] for item in reviews} == {"REJECT","APPROVE"},
                      f"{case}: original rejected event and retake approval both retained")
                check(sorted((item["item_code"],item["attempt_no"]) for item in events) == [("HEIGHT",1),("WIDTH",1),("WIDTH",2)],
                      f"{case}: actual ordered retake lineage")
                users = {user["user_id"] for user in facts.get("users", []) if user["status"] == "ACTIVE"}
                check(all(item["actor_user_id"] in users for item in events) and all(item["reviewer_user_id"] in users for item in reviews),
                      f"{case}: actual owned ACTIVE submitter and reviewer")
        elif case == "gate-sale-owner-race":
            race = re.findall(rf"QA inspection_gate_race tenant={tenant} source=([a-f0-9-]+) freeze=([a-f0-9-]+) actual_pg_blocked=20 responses=20 successful_ships=0",log)
            results = re.findall(rf"QA inspection_gate_race_result tenant={tenant} request=([a-f0-9-]+) status=409 command_present=false",log)
            check(len(race) == 1 and len(set(results)) == 20, f"{case}: actual PG twenty-blocked and twenty completed results")
            check(len(deliveries) == 1 and deliveries[0]["stage"] == "ISSUED", f"{case}: race retains original ISSUED stage")
            check(len(freezes) == 1 and not facts.get("shared_inspection_freeze_lines"), f"{case}: actual source-lock committed empty legacy copy")
            check(not any(item["request_id"] in set(results) for item in commands), f"{case}: no successful race command")
            if race and freezes:
                check(freezes[0]["source_id"] == race[0][0] and freezes[0]["freeze_id"] == race[0][1], f"{case}: exact actual race source and freeze")
        elif case == "gate-review-zero-available":
            check(f"QA inspection_gate_zero_available tenant={tenant} before_review=0" in log, f"{case}: actual zero-available precondition measured")
            check(len(deliveries) == 1 and deliveries[0]["stage"] == "ISSUED", f"{case}: actual REVIEW and ISSUE complete")
            balances = facts.get("inventory_balances", [])
            check(len(balances) == 1 and balances[0]["on_hand_micros"]-balances[0]["reserved_micros"]-balances[0]["frozen_micros"] == 0,
                  f"{case}: independent exact available remains zero")
        elif case == "gate-reservation-release":
            released = re.findall(rf"QA inspection_gate_positive tenant={tenant} case=gate-reservation-release request=([a-f0-9-]+) source=([a-f0-9-]+) reserved=300000 physical_unchanged=true replay_unchanged=true",log)
            check(len(released) == 1, f"{case}: actual release and original replay marker")
            if released:
                request, source = released[0]
                entries = [entry for entry in facts.get("shared_stock_ledgers", []) if entry["source_line"] == source]
                check(sorted((entry["bucket"],entry["delta_micros"]) for entry in entries) == [("RESERVED",-200_000),("RESERVED",500_000)],
                      f"{case}: original source reserve 0.5 and sole release 0.2")
                check(len([item for item in commands if item["request_id"] == request]) == 1,
                      f"{case}: one original release command")
                release_ledgers = [entry for entry in facts.get("inventory_ledgers", []) if entry["reason_type"] == "SHARED_STOCK_RELEASE"]
                check(len(release_ledgers) == 1 and release_ledgers[0]["delta_on_hand_micros"] == 0 and release_ledgers[0]["delta_reserved_micros"] == -200_000,
                      f"{case}: exact reservation-only canonical release")
        extended.append({"tenant":tenant,"case":case,"work_order":work_order,"actual_http_events":len(actual_http)})

summary = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?(\d+) filtered out; finished in ([0-9.]+)s", log)
check(len(summary) == 1, "one terminal registered Rust test result")
if summary:
    status, passed, failed, ignored, filtered, seconds = summary[0]
    check(int(ignored) == 0, "no ignored test acceptance")
    check((int(passed), int(failed)) == ((0,4) if args.original else (args.tests,0)), "actual registered result counts")
result = {"created_at":datetime.now(timezone.utc).isoformat(),"status":"PASS" if not errors else "FAIL",
          "mode":"REPRODUCED_UNSAFE_ORIGINAL" if args.original else "FIXED_CORE_ACCEPTANCE",
          "assertions":assertions,"protected_existing_tenants":len(before),"selected_tables":56,
          "core_cases":core,"extended_cases":extended,"test_summary":summary,"before":{"path":args.before,"sha256":before_sha},
          "after":{"path":args.after,"sha256":after_sha},"errors":errors,
          "balance_reconstruction":"Excludes only a compatibility MFG_RECEIPT ledger whose real receipt/stock reference and all quantity/key/reason/time fields exactly match its canonical inventory ledger",
          "verifier_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          "scope":"Four unsafe-original/fixed core cases; for >=14 tests all fourteen fresh fixture groups with independent canonical quantities, real HTTP, lineage and twenty-blocked race. Other full-suite cases use their actual Rust assertions. Not whole ERP, production authentication, formal clients or performance"}
with destination.open("x",encoding="utf-8") as stream:
    json.dump(result,stream,ensure_ascii=False,indent=2)
    stream.write("\n")
print(json.dumps({"status":result["status"],"assertions":assertions,"protected":len(before),"errors":errors},ensure_ascii=False))
raise SystemExit(bool(errors))
