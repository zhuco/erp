"""Independently recompute protected QA stock and actual HTTP results; no writes to DB."""
import argparse
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import parse_qs, urlsplit

base = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("--before", default="free-erp-inventory-consumers-full-before-qa.json")
parser.add_argument("--after", default="free-erp-inventory-consumers-full-after-qa.json")
parser.add_argument("--log", default="free-erp-inventory-consumers-full-native.log")
parser.add_argument("--output", default="free-erp-inventory-consumers-independent.json")
parser.add_argument("--expected-total", type=int, default=101)
args = parser.parse_args()
paths = {}
for field in ["before", "after", "log", "output"]:
    name = getattr(args, field)
    if not re.fullmatch(r"free-erp-inventory-consumers-[a-z0-9-]+\.(json|log)", name):
        raise SystemExit("unapproved evidence filename")
    paths[field] = base / name

def decoded(path):
    data = path.read_bytes()
    return data.decode("utf-16" if data.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")

before = json.loads(decoded(paths["before"]))
after = json.loads(decoded(paths["after"]))
log = decoded(paths["log"])
checks = 0
errors = []

def check(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)

identity = {"database": "erp_shared_contract_test", "role": "erp_grid_test", "host": "127.0.0.1", "port": 15438}
for facts in [before, after]:
    check(all(facts.get(k) == v for k, v in identity.items()), "wrong protected QA identity")
old = {t["tenant_id"]: t for t in before["tenants"]}
current = {t["tenant_id"]: t for t in after["tenants"]}
unchanged = 0
for tenant, record in old.items():
    now = current.get(tenant, {})
    # Earlier focused manifests predate the added shared/reservation arrays.
    same = all(now.get(field) == value for field, value in record.items())
    check(same, f"existing tenant changed: {tenant}")
    unchanged += int(same)

result = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", log)
check(bool(result), "registered acceptance did not pass")
if result:
    check(tuple(map(int, result[-1])) == (args.expected_total, 0, 0), "unexpected registered result")
fixture_pattern = r"QA inventory consumer fixture tenant=([0-9a-f-]+) actor=([0-9a-f-]+) warehouse=([0-9a-f-]+) denied=([0-9a-f-]+) sku=([0-9a-f-]+)"
fixtures = {m[0]: dict(zip(["actor", "warehouse", "denied", "sku"], m[1:]))
            for m in re.findall(fixture_pattern, log)}
check(len(fixtures) >= 12, "missing actual inventory consumer fixtures")
canonical = 0
unknown = 0
for tenant, f in fixtures.items():
    check(tenant not in old, f"fixture reused an existing tenant: {tenant}")
    data = current.get(tenant)
    check(data is not None and data["name"] == "QA-INVENTORY-CONSUMERS", f"missing fixture facts: {tenant}")
    if data is None:
        continue
    check(any(g["user_id"] == f["actor"] and g["warehouse_id"] == f["warehouse"] for g in data["grants"]), "actor grant missing")
    for row in data["balances"]:
        key = (row["warehouse_id"], row["sku_id"])
        columns = ["on_hand", "reserved", "frozen", "in_transit"]
        values = [row.get(field + "_micros") for field in columns]
        if all(v is None for v in values):
            unknown += 1
            check(not any((a["warehouse_id"], a["sku_id"]) == key for a in data["activations"]), "unknown key was activated by read")
            continue
        canonical += 1
        check(all(isinstance(v, int) and v >= 0 for v in values), "incomplete or negative canonical buckets")
        for field, exact in zip(columns, values):
            check(row[field] == exact // 1_000_000, f"whole projection mismatch: {tenant} {key} {field}")
            sources = [l for l in data["ledgers"] if (l["warehouse_id"], l["sku_id"]) == key]
            total = sum(l.get("delta_" + field + "_micros") if l.get("delta_" + field + "_micros") is not None
                        else l["delta_" + field] * 1_000_000 for l in sources)
            check(total == exact, f"balance differs from independent source sum: {tenant} {key} {field}")
        check(values[0] - values[1] - values[2] >= 0, "overreserved stock")

def raw_balance(tenant, row):
    matches = [b for b in current[tenant]["balances"] if b["warehouse_id"] == row["warehouse_id"] and b["sku_id"] == row["sku_id"]]
    check(len(matches) == 1, "response physical key missing or ambiguous")
    return matches[0] if len(matches) == 1 else None

def verify_quantities(tenant, row):
    balance = raw_balance(tenant, row)
    if balance is None:
        return
    amounts = {name: balance[name + "_micros"] for name in ["on_hand", "reserved", "frozen", "in_transit"]}
    amounts["available"] = amounts["on_hand"] - amounts["reserved"] - amounts["frozen"]
    for name, exact in amounts.items():
        text = row.get("quantities", {}).get(name)
        check(isinstance(text, str), f"missing exact response quantity: {name}")
        if isinstance(text, str):
            check(Decimal(text) * 1_000_000 == exact, f"HTTP quantity mismatch: {tenant} {name}")
        whole = row.get(name, row.get("available_whole") if name == "available" else None)
        check(whole == exact // 1_000_000, f"response compatibility projection mismatch: {tenant} {name}")
    return amounts

http_results = 0
for tenant, path, status, encoded in re.findall(r"QA inventory consumer HTTP tenant=([0-9a-f-]+) path=(\S+) status=(\d+) [^\r\n]*? body=(\{[^\r\n]+\})", log):
    if tenant not in fixtures:
        continue
    http_results += 1
    body = json.loads(encoded)
    if int(status) != 200:
        check("items" not in body and "record" not in body, "refused request leaked business data")
        continue
    if "record" in body:
        verify_quantities(tenant, body["record"])
        check(body["record"]["quantity_state"] == "precise", "write receipt lacks precise state")
        continue
    check(body.get("schema") == 2 and body.get("sales_basis") == "gross_posted_sales_ship"
          and body.get("turnover_basis") == "closing_stock_cover", "report basis or schema missing")
    check(bool(body.get("snapshot_at")), "report snapshot missing")
    check(body["total"] >= len(body["items"]), "report pagination count invalid")
    query = parse_qs(urlsplit(path).query)
    warning = Decimal(query.get("warning_quantity", query.get("warning_threshold", ["5"]))[0]) * 1_000_000
    for row in body["items"]:
        check(row["warehouse_id"] == fixtures[tenant]["warehouse"], "ungranted warehouse leaked")
        amounts = verify_quantities(tenant, row)
        if amounts is None:
            continue
        check(row["warning"] == (amounts["available"] <= warning), "warning rounded before comparison")
        check(row["out_of_stock"] == (amounts["available"] <= 0), "fractional available stock became out-of-stock")
        if row["sales_state"] == "source_classification_required":
            check(all(row[name] is None for name in ["sold_qty", "sold_quantity", "turnover_days"]), "unknown sales became zero")
            check(any(l.get("ref_type") == "shared_stock_ledger" for l in current[tenant]["ledgers"]), "unknown state lacks independent source")
            continue
        start = datetime.fromisoformat(query["start_at"][0].replace("Z", "+00:00")) if "start_at" in query else None
        end = datetime.fromisoformat(query["end_at"][0].replace("Z", "+00:00")) if "end_at" in query else None
        sources = [l for l in current[tenant]["ledgers"] if l["warehouse_id"] == row["warehouse_id"] and l["sku_id"] == row["sku_id"]
                   and l["reason_type"] == "SALES_SHIP"]
        if start is not None and end is not None:
            sources = [l for l in sources if start <= datetime.fromisoformat(l["occurred_at"]) <= end]
        sold = sum(-l["delta_on_hand_micros"] for l in sources if l["delta_on_hand_micros"] < 0)
        check(Decimal(row["sold_quantity"]) * 1_000_000 == sold, "shipment quantity differs from physical ledger")
        check(row["sold_qty"] == sold // 1_000_000, "shipment projection rounded up")
        if sold == 0:
            check(row["turnover_days"] is None, "zero shipments invented stock cover")
        elif start is not None and end is not None:
            days = Decimal(str((end - start).total_seconds())) / Decimal(86400)
            expected = days * Decimal(amounts["on_hand"]) / Decimal(sold)
            check(abs(Decimal(str(row["turnover_days"])) - expected) < Decimal("0.00000001"), "stock-cover estimate mismatch")

for tenant, encoded in re.findall(r"QA inventory consumer (?:balance|writer) tenant=([0-9a-f-]+) available=\d+ body=(\{[^\r\n]+\})", log):
    if tenant in fixtures:
        verify_quantities(tenant, json.loads(encoded))
for tenant, action, encoded in re.findall(r"QA inventory consumer reservation tenant=([0-9a-f-]+) action=(\w+) body=(\{[^\r\n]+\})", log):
    if tenant in fixtures:
        verify_quantities(tenant, json.loads(encoded))
        expected = "RELEASED" if action == "release" else "CONSUMED"
        check(any(r["status"] == expected for r in current[tenant]["reservations"]), "reservation final state missing")
check(http_results >= 20, "not enough actual HTTP observations")
check(unknown >= 3, "unknown-stock refusal fixtures missing")
report = {"updated_at": datetime.now(timezone.utc).isoformat(), "status": "FAIL" if errors else "PASS",
          "checks": checks, "old_tenants_unchanged": unchanged, "new_consumer_tenants": len(fixtures),
          "canonical_keys": canonical, "unknown_keys_not_activated": unknown,
          "http_results": http_results, "expected_registered_total": args.expected_total,
          "inputs": {name: {"path": str(path.relative_to(base)), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                     for name, path in paths.items() if name != "output"}, "errors": errors,
          "limits": "Protected QA and synthetic JWT; source certification, real clients, Release performance and public hosting are separate gates"}
paths["output"].write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({k: report[k] for k in ["status", "checks", "old_tenants_unchanged", "new_consumer_tenants", "canonical_keys", "unknown_keys_not_activated", "http_results", "errors"]}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
