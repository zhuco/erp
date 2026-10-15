"""Recompute actual mutation HTTP/SQL facts independently; never write to the DB."""
import argparse
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re

base = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("--before", default="free-erp-inventory-mutations-before-qa.json")
parser.add_argument("--after", default="free-erp-inventory-mutations-after-qa.json")
parser.add_argument("--log", default="free-erp-inventory-mutations-full-native.log")
parser.add_argument("--output", default="free-erp-inventory-mutations-independent.json")
parser.add_argument("--expected-total", type=int, default=104)
args = parser.parse_args()
paths = {}
for field in ["before", "after", "log", "output"]:
    name = getattr(args, field)
    if not re.fullmatch(r"free-erp-inventory-mutations-[a-z0-9-]+\.(json|log)", name):
        raise SystemExit("unapproved evidence filename")
    paths[field] = base / name
if paths["output"].exists():
    raise SystemExit("refusing to replace independent evidence")


def decoded(path):
    raw = path.read_bytes()
    return raw.decode("utf-16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")


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


timestamp_fields = {"activated_at", "updated_at", "occurred_at", "created_at",
                    "next_retry_at", "last_attempt_at", "published_at"}


def canonical_instants(value):
    if isinstance(value, list):
        return [canonical_instants(item) for item in value]
    if isinstance(value, dict):
        return {key: datetime.fromisoformat(item).astimezone(timezone.utc).isoformat()
                if key in timestamp_fields and isinstance(item, str)
                else canonical_instants(item) for key, item in value.items()}
    return value


identity = {"database": "erp_shared_contract_test", "role": "erp_grid_test", "host": "127.0.0.1", "port": 15438}
for data in [before, after]:
    check(all(data.get(k) == v for k, v in identity.items()), "wrong protected QA identity")
old = {t["tenant_id"]: t for t in before["tenants"]}
current = {t["tenant_id"]: t for t in after["tenants"]}
unchanged = 0
for tenant, facts in old.items():
    same = current.get(tenant) == facts
    check(same, f"existing tenant changed: {tenant}")
    unchanged += int(same)
results = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", log)
check(bool(results), "registered acceptance did not pass")
if results:
    check(tuple(map(int, results[-1])) == (args.expected_total, 0, 0), "unexpected registered result")

fixtures = {tenant: {"actor": actor, "warehouse": warehouse, "sku": sku}
    for tenant, actor, warehouse, sku in re.findall(
        r"QA mutation fixture tenant=([0-9a-f-]+) actor=([0-9a-f-]+) warehouse=([0-9a-f-]+) sku=([0-9a-f-]+)", log)}
check(len(fixtures) == 9, "nine actual mutation fixtures required")
observations = {}
http_count = 0
for tenant, path, failure, status, encoded in re.findall(
    r"QA mutation HTTP tenant=([0-9a-f-]+) path=(\S+) failure=(\w+) status=(\d+) [^\r\n]*? body=(\{[^\r\n]+\})", log):
    if tenant not in fixtures:
        continue
    http_count += 1
    check(tenant not in observations, "fixture reused for multiple mutation attempts")
    body = json.loads(encoded)
    action = path.rsplit("/", 1)[-1]
    observations[tenant] = (action, failure, int(status), body)
    check(action in ["adjustments", "inbounds", "outbounds"], "unexpected route")
    check(path == f"/v1/tenants/{tenant}/inventory/{action}", "actual route/tenant mismatch")
    check(int(status) == (200 if failure == "none" else 500), "unexpected HTTP result")
    if failure != "none":
        check(failure in ["operation_logs", "audit_logs"] and "check option" in body.get("text", ""), "did not reach actual owned log failure")
        check("record" not in body, "failed mutation returned a success balance")
check(http_count == 9, "nine actual HTTP observations required")

refusals = 0
for tenant, action, table, previous, final in re.findall(
    r"QA mutation refusal tenant=([0-9a-f-]+) action=(\w+) table=(\w+) before=(\{[^\r\n]+\}) after=(\{[^\r\n]+\})", log):
    refusals += 1
    left, right = json.loads(previous), json.loads(final)
    check(left == right, f"failed write committed a partial fact: {tenant}")
    data = current.get(tenant, {})
    check(all(canonical_instants(data.get(k)) == canonical_instants(value)
              for k, value in right.items()), "logged refusal differs from independent public SQL facts")
    check(observations.get(tenant, ())[:2] == (action, table), "failure facts do not match actual route")
check(refusals == 6, "six route-by-log-failure observations required")

for tenant, fixture in fixtures.items():
    check(tenant not in old, "old tenant reused for a mutation fixture")
    data = current.get(tenant)
    check(data is not None and data["name"] == "QA-INVENTORY-MUTATION-ATOMICITY", "fixture missing in independent facts")
    if data is None or tenant not in observations:
        continue
    action, failure, status, body = observations[tenant]
    check(any(g["user_id"] == fixture["actor"] and g["warehouse_id"] == fixture["warehouse"] for g in data["grants"]), "warehouse grant missing")
    check(len(data["balances"]) == 1, "physical balance missing or ambiguous")
    row = data["balances"][0]
    check(row["warehouse_id"] == fixture["warehouse"] and row["sku_id"] == fixture["sku"], "physical key mismatch")
    expected_hand = 2_900_000 if failure != "none" else (1_900_000 if action == "outbounds" else 3_900_000)
    amounts = {"on_hand": expected_hand, "reserved": 900_000, "frozen": 900_000, "in_transit": 2_300_000}
    amounts["available"] = amounts["on_hand"] - amounts["reserved"] - amounts["frozen"]
    for field in ["on_hand", "reserved", "frozen", "in_transit"]:
        exact = row[field + "_micros"]
        check(exact == amounts[field], f"unexpected exact bucket: {tenant} {field}")
        check(row[field] == exact // 1_000_000, "integer projection mismatch")
        total = sum(l["delta_" + field + "_micros"] for l in data["ledgers"])
        check(total == exact, "physical bucket differs from independent ledger sum")
    check(len(data["ledgers"]) == (1 if failure != "none" else 2), "wrong number of posted stock effects")
    check(len(data["operations"]) == len(data["audits"]) == (0 if failure != "none" else 1), "required log cardinality mismatch")
    check(len(data["outbox"]) == 0, "unmapped fixture unexpectedly generated a channel event")
    if failure != "none":
        continue
    quantities = body["record"]["quantities"]
    for field, amount in amounts.items():
        check(Decimal(quantities[field]) * 1_000_000 == amount, "HTTP exact quantity differs from public SQL facts")
        check(body["record"][field] == amount // 1_000_000, "HTTP compatibility projection mismatch")
    operation, audit = data["operations"][0], data["audits"][0]
    check(operation["actor_user_id"] == audit["user_id"] == fixture["actor"], "actor not preserved in both logs")
    for payload in [operation["metadata"], audit["after_json"]]:
        check(payload["warehouse_id"] == fixture["warehouse"] and payload["sku_id"] == fixture["sku"], "log physical identity mismatch")
        check(payload["quantities"] == quantities, "log lacks actual precise committed quantities")
        check(payload["qty_delta"] == (-1 if action == "outbounds" else 1), "log delta mismatch")

report = {"updated_at": datetime.now(timezone.utc).isoformat(), "status": "FAIL" if errors else "PASS",
    "checks": checks, "old_tenants_unchanged": unchanged, "new_mutation_tenants": len(fixtures),
    "http_results": http_count, "actual_log_refusals": refusals, "successes": http_count - refusals,
    "expected_registered_total": args.expected_total, "errors": errors,
    "inputs": {name: {"path": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
        for name, path in paths.items() if name != "output"},
    "timestamp_comparison": "Known SQL timestamp fields compare full instants in UTC; SQLx UTC and psql +08:00 representations are preserved in raw inputs. No quantity or business field is removed.",
    "verifier_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    "limits": "Protected QA, synthetic JWT and no batch/mapped-channel effects. Command identity, commit-result recovery, whole-source certification, clients and Release performance remain separate gates"}
paths["output"].write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({k: report[k] for k in ["status", "checks", "old_tenants_unchanged", "new_mutation_tenants", "http_results", "actual_log_refusals", "successes", "errors"]}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
