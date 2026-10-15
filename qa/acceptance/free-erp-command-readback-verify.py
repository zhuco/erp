"""Independent protected PostgreSQL facts and real HTTP outcomes for command reads."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-command-readback-"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--before", required=True)
parser.add_argument("--after", required=True)
parser.add_argument("--original", action="store_true")
parser.add_argument("--tests", type=int, default=4)
parser.add_argument("--log")
args = parser.parse_args()
if not re.fullmatch(r"[a-z0-9-]+", args.stage):
    raise SystemExit("invalid stage")
destination = BASE / (PREFIX + args.stage + "-independent.json")
if destination.exists():
    raise SystemExit("refusing to replace historical evidence")
log_path = ROOT / args.log if args.log else BASE / (PREFIX + args.stage + "-test.log")
log = log_path.read_text(encoding="utf-8-sig")
fixtures = re.findall(r"QA command_readback_fixture tenant=([a-f0-9-]+) case=([a-z0-9-]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+) work_order=([a-f0-9-]+)", log)
markers = re.findall(r"QA command_readback_result tenant=([a-f0-9-]+) case=([a-z0-9-]+) request=([^ ]+) status=(\d+) unchanged=(true|false) response_status=([^\r\n]+)", log)
captured_before = {tenant: json.loads(value) for tenant, value in re.findall(r"QA command_readback_before tenant=([a-f0-9-]+) facts=([^\r\n]+)", log)}
errors, checks = [], 0
timestamp_catalog_path = BASE / (PREFIX + "timestamp-catalog.json")
timestamp_columns = json.loads(timestamp_catalog_path.read_text(encoding="utf-8-sig"))

def canonical_rows(table, rows):
    if rows is None:
        return None
    result = []
    for source in rows:
        row = dict(source)
        for column in timestamp_columns.get(table, []):
            value = row.get(column)
            if value is not None:
                # Normalize only catalog-confirmed timestamptz columns. Preserve every other value.
                row[column] = datetime.fromisoformat(value).astimezone(timezone.utc).isoformat(timespec="microseconds")
        result.append(row)
    return sorted(result, key=lambda row: json.dumps(row, sort_keys=True, ensure_ascii=False))

def check(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)

def capture(path, retained):
    fingerprints, owned, metadata = {}, {}, None
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
            check(tenant not in fingerprints, f"unique snapshot enterprise {tenant}")
            check(len(row["facts"]) == 60, f"sixty selected fact tables {tenant}")
            fingerprints[tenant] = hashlib.sha256(raw.rstrip(b"\r\n")).hexdigest()
            if tenant in retained:
                owned[tenant] = row
    check(metadata is not None and metadata.get("database") == "erp_shared_contract_test"
          and metadata.get("role") == "erp_grid_test" and metadata.get("host") == "127.0.0.1"
          and metadata.get("port") == 15438 and metadata.get("selected_tables") == 60,
          "protected database identity and complete selected catalog")
    return fingerprints, owned, digest.hexdigest()

owned = {row[0] for row in fixtures}
before, _, before_sha = capture(ROOT / args.before, set())
after, current, after_sha = capture(ROOT / args.after, owned)
for tenant, fingerprint in before.items():
    check(after.get(tenant) == fingerprint, f"existing enterprise full selected facts unchanged {tenant}")
check(len(fixtures) == args.tests and len(owned) == args.tests, "one independent fixture per requested case")
check(len(markers) == args.tests, "all requested real HTTP assertions recorded")
core = {"accepted-lookup", "client-success", "material-get", "material-lost"}
check(core <= {row[1] for row in fixtures}, "all four original counterexamples executed")
expected_original = {"accepted-lookup": (200, "SUCCEEDED", False), "client-success": (200, "SUCCEEDED", False),
                     "material-get": (404, "null", True), "material-lost": (422, "null", True)}
expected_fixed = {"accepted-lookup": (200, "ACCEPTED"), "client-success": (409, "null"),
                  "material-get": (200, "SUCCEEDED"), "material-lost": (200, "SUCCEEDED"),
                  "accepted-observations": (200, "ACCEPTED"), "client-transitions": (409, "null"),
                  "missing-unknown": (404, "null"), "material-observe": (200, "SUCCEEDED"),
                  "material-payload": (409, "null"), "material-client-fields": (200, "SUCCEEDED"),
                  "expired-session": (403, "null"), "stale-session": (403, "null"), "foreign-session": (403, "null"),
                  "legacy-generic-terminal": (409, "null"), "material-concurrent": (200, "SUCCEEDED")}
outcomes = []
for tenant, case, request, status, unchanged, response_status in markers:
    check(tenant not in before and tenant in current, f"{case}: fresh isolated enterprise")
    row = current.get(tenant, {})
    check(row.get("name") == "QA-ERP-COMMAND-READBACK", f"{case}: fixture ownership")
    facts = row.get("facts", {})
    original = captured_before.get(tenant)
    check(original is not None, f"{case}: actual pre-request independent facts logged")
    if args.original:
        expected, server_status, preserved = expected_original[case]
        check(int(status) == expected and response_status.strip('"') == server_status, f"{case}: original real counterexample HTTP")
        check((unchanged == "true") == preserved, f"{case}: original observed mutation")
    else:
        if case in expected_fixed:
            expected, server_status = expected_fixed[case]
            check(int(status) == expected and response_status.strip('"') == server_status, f"{case}: repaired original HTTP assertion")
        check(unchanged == "true", f"{case}: request observer reports no mutation")
        if original:
            for table, old_value in original.items():
                check(table in facts and canonical_rows(table,facts[table]) == canonical_rows(table,old_value),
                      f"{case}: independent current facts match pre-request {table}")
        check(not [ledger for ledger in facts.get("inventory_ledgers", []) if ledger.get("reason_type") == "COMMAND_LOOKUP"],
              f"{case}: lookup never creates an inventory effect")
    receipts = [receipt for receipt in facts.get("command_receipts", []) if receipt["request_id"] == request]
    if case in core:
        check(len(receipts) == 1, f"{case}: exactly one original receipt")
    if case == "accepted-lookup" and receipts:
        lookup_rows = [ledger for ledger in facts.get("inventory_ledgers", []) if ledger.get("reason_type") == "COMMAND_LOOKUP"]
        check(receipts[0]["status"] == ("SUCCEEDED" if args.original else "ACCEPTED"), "accepted lookup keeps original status after repair")
        check(len(lookup_rows) == (1 if args.original else 0), "lookup stock posting original versus repair")
        if lookup_rows:
            check(lookup_rows[0]["delta_on_hand"] == 3 and lookup_rows[0]["delta_on_hand_micros"] == 3_000_000,
                  "original arbitrary caller quantity becomes actual stock")
    if case == "client-success" and receipts:
        check(receipts[0]["status"] == ("SUCCEEDED" if args.original else "ACCEPTED"), "client report has no authority after repair")
        check(receipts[0]["effect_id"] == (row.get("facts", {}).get("command_objects", [{}])[0].get("object_id") if args.original else ""),
              "open-command object placeholder is not a posted business effect")
    if case.startswith("material-") and receipts:
        check(receipts[0]["status"] == "SUCCEEDED", f"{case}: actual business success remains unchanged")
        moves = [move for move in facts.get("mfg_material_moves", []) if str(move.get("move_id")) == receipts[0]["effect_id"]]
        check(len(moves) == 1 and moves[0].get("qty_micros") == 100_000 and moves[0].get("kind") == "ISSUE",
              f"{case}: original receipt links one real precise material effect")
        check(not facts.get("command_objects"), f"{case}: typed business command requires no fake generic open")
    outcomes.append({"tenant_id": tenant, "case": case, "request_id": request, "actual_http": int(status),
                     "unchanged": unchanged == "true", "response_status": response_status})
if args.original:
    check("test result: FAILED. 1 passed; 4 failed; 0 ignored;" in log, "four unsafe or unavailable results; one existing matching-filter test passes")
else:
    check(re.search(r"test result: ok\. \d+ passed; 0 failed; 0 ignored;", log) is not None, "actual repaired regression has zero failure and ignore")
report = {"utc": datetime.now(timezone.utc).isoformat(), "status": "PASS" if not errors else "FAIL",
          "scope": "Original unsafe outcomes" if args.original else "Read-only command results and original typed effect",
          "checks": checks, "existing_enterprises": len(before), "protected_tables": 60,
          "tests": args.tests, "cases": outcomes, "errors": errors,
          "before_sha256": before_sha, "after_sha256": after_sha,
          "log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest()}
report["timestamp_catalog_sha256"] = hashlib.sha256(timestamp_catalog_path.read_bytes()).hexdigest()
report["timestamp_comparison"] = "Only actual PostgreSQL timestamptz columns are compared as exact UTC instants; all other fields and nested payloads are preserved."
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
print(json.dumps({key: report[key] for key in ["status", "checks", "existing_enterprises", "tests", "errors"]}, ensure_ascii=False))
raise SystemExit(0 if not errors else 1)
