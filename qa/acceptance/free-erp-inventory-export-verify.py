"""Independent PostgreSQL facts and CSV checks, without invoking ERP code."""
from collections import defaultdict
from datetime import datetime, timezone
import argparse
from decimal import Decimal
import csv
import hashlib
import json
from pathlib import Path
import re
import uuid

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
parser = argparse.ArgumentParser()
parser.add_argument("--before", default="free-erp-inventory-export-final-qa-before.json")
parser.add_argument("--after", default="free-erp-inventory-export-final-qa-after.json")
parser.add_argument("--log", default="free-erp-inventory-export-refined-native-full.log")
parser.add_argument("--output", default="free-erp-inventory-export-independent.json")
parser.add_argument("--expected-total", type=int, default=86)
args = parser.parse_args()
for name in [args.before, args.after, args.log, args.output]:
    if not re.fullmatch(r"free-erp-inventory-export-[a-z0-9-]+\.(json|log)", name):
        raise SystemExit("QA evidence path outside admitted filenames")

def load(name):
    return json.loads((base / name).read_text(encoding="utf-8-sig"))

before = load(args.before)
after = load(args.after)
identity = {"database": "erp_shared_contract_test", "role": "erp_grid_test", "host": "127.0.0.1", "port": 15438}
errors = []
checks = 0

def check(condition, label):
    global checks
    checks += 1
    if not condition:
        errors.append(label)

for name, facts in [("before", before), ("after", after)]:
    check(all(facts.get(key) == value for key, value in identity.items()), f"{name} protected identity")
old = {item["tenant_id"]: item for item in before["tenants"]}
current = {item["tenant_id"]: item for item in after["tenants"]}
for tenant, facts in old.items():
    check(current.get(tenant) == facts, f"existing QA tenant changed: {tenant}")
fresh = [item for item in after["tenants"] if item["tenant_id"] not in old and item["name"] == "QA-INVENTORY-EXPORT"]
check(len(fresh) == 18, "18 newly isolated inventory-export tenants")
log = (base / args.log).read_text(encoding="utf-8-sig")
passed = set(re.findall(r"test inventory_exports::(\w+) \.\.\. ok", log))
expected_tests = set(re.findall(r"#\[tokio::test\]\s+async fn (\w+)", (root / "crates/erp-api/tests/free_erp_acceptance/inventory_exports.rs").read_text(encoding="utf8")))
check(passed == expected_tests and len(passed) == 18, "every declared export acceptance executed and passed")
check(f"{args.expected_total} passed; 0 failed; 0 ignored" in log, "selected registered acceptance count")

buckets = ["on_hand", "reserved", "frozen", "in_transit"]
headers = ["sku_id", "sku_code", "warehouse_id", "on_hand", "reserved", "available", "frozen", "in_transit", "quantity_scale", "snapshot_at"]
artifacts = []
known = unknown = recovered = legacy = 0

def decimal_text(micros):
    whole, fractional = divmod(micros, 1_000_000)
    return f"{whole}.{fractional:06d}".rstrip("0") if fractional else str(whole)

for fixture in fresh:
    tenant = fixture["tenant_id"]
    ledger_sums = defaultdict(lambda: [0, 0, 0, 0])
    for ledger in fixture["ledgers"]:
        key = (ledger["warehouse_id"], ledger["sku_id"])
        for index, bucket in enumerate(buckets):
            delta = ledger[f"delta_{bucket}_micros"]
            check(isinstance(delta, int), f"canonical ledger delta missing {tenant}/{key}/{bucket}")
            if isinstance(delta, int):
                ledger_sums[key][index] += delta
    stock = {}
    for balance in fixture["balances"]:
        key = (balance["warehouse_id"], balance["sku_id"])
        values = [balance[f"{bucket}_micros"] for bucket in buckets]
        if any(value is None for value in values):
            unknown += 1
            check(all(value is None for value in values), f"unknown key partially activated {tenant}/{key}")
            check(not any((row["warehouse_id"], row["sku_id"]) == key for row in fixture["activations"]), f"unknown export key activated {tenant}/{key}")
            continue
        known += 1
        check(values == ledger_sums[key], f"balance differs from original precision ledger {tenant}/{key}")
        check(all(value >= 0 for value in values) and values[0] - values[1] - values[2] >= 0, f"negative canonical bucket {tenant}/{key}")
        stock[key] = values
    check(not fixture["outbox"], f"unmapped export fixture wrote outbox {tenant}")
    skus = {item["sku_id"]: item for item in fixture["skus"]}
    for job in fixture["jobs"]:
        params = job["params_json"]
        if params.get("schema") != 2:
            legacy += 1
            check(params == {} and job["status"] == "DONE", f"unrecognized legacy fixture {job['job_id']}")
            continue
        job_id = job["job_id"]
        check(uuid.UUID(job_id).int != 0 and params["request_id"] == job_id, f"original request identity {job_id}")
        warehouses = params["warehouse_ids"]
        check(warehouses == sorted(set(warehouses)), f"held warehouse identity {job_id}")
        audit = [row for row in fixture["audits"] if row["entity_id"] == job_id and row["action"] == "EXPORT.JOB.CREATE"]
        check(len(audit) == 1 and audit[0]["user_id"] == params["actor_user_id"], f"one actor audit for admitted job {job_id}")
        artifact = params["artifact"]
        path = root / "crates/erp-api/exports" / tenant / f"{job_id}.csv"
        # This run uses the default package-relative directory. A configured
        # alternate directory must be explicitly supplied in a future verifier.
        check(path.is_relative_to(root), f"artifact path admission {job_id}")
        if not artifact:
            check(not path.exists(), f"unprepared or rejected job published a file {job_id}")
            continue
        check(path.is_file() and not path.is_symlink(), f"regular published file {job_id}")
        if not path.is_file():
            continue
        check(path.stat().st_size == artifact["bytes"] <= 32 * 1024 * 1024, f"fixture file length {job_id}")
        digest = hashlib.sha256()
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(65536), b""):
                digest.update(chunk)
        actual_digest = digest.hexdigest()
        if job["status"] == "FAILED" and "hash mismatch" in job["last_error"]:
            check(actual_digest != artifact["sha256"], f"corruption fixture still refused {job_id}")
            artifacts.append({"job_id": job_id, "path": path.relative_to(root).as_posix(), "state": "INTENTIONALLY_CORRUPT_AND_REFUSED", "bytes": artifact["bytes"]})
            continue
        check(job["status"] == "DONE" and job["progress"] == 100, f"complete publication state {job_id}")
        check(actual_digest == artifact["sha256"], f"published receipt SHA256 {job_id}")
        check(artifact["schema"] == 2 and artifact["quantity_scale"] == 6, f"receipt quantity version {job_id}")
        if job["attempts"] > 1:
            recovered += 1
        with path.open(encoding="utf8", newline="") as source:
            reader = csv.DictReader(source)
            check(reader.fieldnames == headers, f"CSV columns {job_id}")
            rows = list(reader)
        check(len(rows) == artifact["rows"], f"CSV row count {job_id}")
        selected = []
        query = params["query"]
        for key, quantities in stock.items():
            warehouse, sku = key
            if warehouse not in warehouses or (query["warehouse_id"] and query["warehouse_id"] != warehouse) or (query["sku_id"] and query["sku_id"] != sku):
                continue
            master = skus[sku]
            if any(query[field] and query[field] != master.get(field) for field in ["sku_code", "barcode"]):
                continue
            if query["tag"] and not any(query["tag"].casefold() in tag.casefold() for tag in master.get("tags", [])):
                continue
            available = Decimal(quantities[0] - quantities[1] - quantities[2]) / 1_000_000
            if (query["available_min"] is not None and available < Decimal(query["available_min"])) or (query["available_max"] is not None and available > Decimal(query["available_max"])):
                continue
            selected.append(key)
        actual_keys = [(row["warehouse_id"], row["sku_id"]) for row in rows]
        check(actual_keys == sorted(selected), f"all matching held keys with stable order {job_id}")
        for row in rows:
            key = (row["warehouse_id"], row["sku_id"])
            q = stock[key]
            values = dict(zip(buckets, q))
            values["available"] = q[0] - q[1] - q[2]
            for field, micros in values.items():
                check(row[field] == decimal_text(micros), f"CSV exact basic units {job_id}/{key}/{field}")
            check(row["quantity_scale"] == "6" and datetime.fromisoformat(row["snapshot_at"]) == datetime.fromisoformat(artifact["snapshot_at"].replace("Z", "+00:00")), f"CSV snapshot {job_id}/{key}")
            code = skus[row["sku_id"]]["sku_code"] or ""
            expected_code = "'" + code if code == "=1+1" else code
            check(row["sku_code"] == expected_code, f"CSV formula-safe master code {job_id}/{key}")
        artifacts.append({"job_id": job_id, "path": path.relative_to(root).as_posix(), "state": "VERIFIED", "rows": artifact["rows"], "bytes": artifact["bytes"], "sha256": actual_digest})
        check(not list(path.parent.glob("*.inventory.tmp")), f"own completed temporary files removed {job_id}")

check(legacy == 1, "one deliberately ownerless legacy job")
check(unknown == 1, "one unknown quantity key remains unactivated")
check(any(item.get("rows") == 1002 for item in artifacts), "independent 1002-row file")
check(recovered >= 2, "receipt reuse and invalid-configuration recovery fixtures")
report = {"updated_at": datetime.now(timezone.utc).isoformat(), "status": "PASS" if not errors else "FAIL",
    "checks": checks, "old_tenants_unchanged": len(old), "new_export_tenants": len(fresh),
    "canonical_keys": known, "unknown_keys_not_activated": unknown, "artifacts": artifacts, "errors": errors,
    "inputs": {"before":args.before,"after":args.after,"log":args.log,"expected_total":args.expected_total},
    "limits": ["controlled QA facts, not production migration/hosting", "synthetic JWT and own-fixture scheduler claims", "CSV values compared with unchanged controlled opening facts; not arbitrary accounting-cutoff reconstruction"]}
(base / args.output).write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({key: report[key] for key in ["status", "checks", "old_tenants_unchanged", "new_export_tenants", "canonical_keys", "unknown_keys_not_activated", "errors"]}, ensure_ascii=False))
raise SystemExit(bool(errors))
