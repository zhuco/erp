"""Independently reconcile saved HTTP responses against protected psql facts.

No imports from the ERP implementation, no database writes, no fabricated timing.
POST bodies were not logged by this test batch: the explicit case names below
identify their selected stock; this is not an independent request-body audit.
"""
from collections import Counter, defaultdict
from datetime import datetime, timezone
from decimal import Decimal, localcontext
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import parse_qs, urlsplit

base = Path(__file__).resolve().parent
prefix = "free-erp-inventory-read-"


def read(name):
    return json.loads((base / (prefix + name)).read_text("utf-8-sig"))


checks = 0


def check(condition, message):
    global checks
    if not condition:
        raise AssertionError(message)
    checks += 1


def scaled(text, scale):
    check(isinstance(text, str) and re.fullmatch(r"-?\d+(?:\.\d+)?", text),
          "plain decimal string required")
    with localcontext() as context:
        context.prec = 100
        value = Decimal(text) * 10 ** scale
    check(value == value.to_integral_value(), "excess fractional precision")
    return int(value)


fields = ["on_hand", "reserved", "frozen", "in_transit"]
before, after = read("final-qa-before.json"), read("final-qa-after.json")
identity = {"database": "erp_shared_contract_test", "role": "erp_grid_test",
            "host": "127.0.0.1", "port": 15438}
for snapshot in (before, after):
    check(all(snapshot[key] == value for key, value in identity.items()),
          "protected QA identity")
old = {f["tenant_id"]: f for f in before["fixtures"]}
facts = {f["tenant_id"]: f for f in after["fixtures"]}
for tenant, fixture in old.items():
    check(facts.get(tenant) == fixture, "pre-existing fixture changed: " + tenant)

log = (base / (prefix + "final-native-full.log")).read_text("utf8")
check("68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out" in log,
      "complete registered integration result")
current = None
cases, fixture_keys, responses = {}, {}, defaultdict(list)
for line in log.splitlines():
    fixture = re.fullmatch(r"QA inventory read tenant=([\w-]+) warehouse=([\w-]+) sku=([\w-]+)", line)
    if fixture:
        current, warehouse, sku = fixture.groups()
        fixture_keys[current] = (warehouse, sku)
    result = re.fullmatch(r"test inventory_reads::(\w+) \.\.\. ok", line)
    if result:
        check(current is not None and current not in cases, "unique fixture per case")
        cases[current] = result[1]
    response = re.fullmatch(r"QA inventory HTTP tenant=([\w-]+) path=(\S+) status=(\d+) [\w ]+ body=(.*)", line)
    if response:
        tenant, path, status, body = response.groups()
        responses[tenant].append((path, int(status), json.loads(body)))
check(len(cases) == 13 and set(cases) == set(facts) - set(old), "13 newly owned fixtures")
check(set(responses) == set(cases), "HTTP evidence for every new fixture")

canonical, unknown, legacy_fallback = 0, 0, 0
for tenant in cases:
    fixture = facts[tenant]
    skus = {s["sku_id"] for s in fixture["skus"]}
    warehouses = {w["warehouse_id"] for w in fixture["warehouses"]}
    activations = {(a["warehouse_id"], a["sku_id"]) for a in fixture["activations"]}
    check(not fixture["outbox"], "unmapped fixture reads did not enqueue outbox")
    check(not fixture["source_maps"], "reads did not invent source mappings")
    for balance in fixture["balances"]:
        key = (balance["warehouse_id"], balance["sku_id"])
        check(balance["tenant_id"] == tenant and key[0] in warehouses and key[1] in skus,
              "balance ownership")
        micros = [balance[name + "_micros"] for name in fields]
        if any(value is None for value in micros):
            unknown += 1
            check(all(value is None for value in micros) and key not in activations,
                  "unknown balance remains unactivated")
            check(balance["on_hand"] in (2, 99), "original unknown whole amount retained")
            continue
        canonical += 1
        check(key in activations, "canonical fixture has real activation")
        check(all(value >= 0 for value in micros) and micros[0] - micros[1] - micros[2] >= 0,
              "canonical bucket validity")
        ledgers = [l for l in fixture["ledgers"]
                   if (l["warehouse_id"], l["sku_id"]) == key]
        for name, value in zip(fields, micros):
            check(sum(l["delta_" + name + "_micros"] for l in ledgers) == value,
                  "canonical balance equals independently summed ledger: " + name)
            check(balance[name] == value // 1_000_000, "whole-unit compatibility projection")
    for ledger in fixture["ledgers"]:
        check(ledger["tenant_id"] == tenant and ledger["warehouse_id"] in warehouses
              and ledger["sku_id"] in skus, "ledger ownership")
        values = [ledger["delta_" + name + "_micros"] for name in fields]
        check(all(v is None for v in values) or all(v is not None for v in values),
              "no mixed ledger precision")
        legacy_fallback += all(v is None for v in values)


def unit_cost(fixture, sku_id):
    skus = {s["sku_id"]: s for s in fixture["skus"]}
    sku = skus[sku_id]
    if sku["sku_kind"] != "KIT":
        cost = sku["cost_cent"]
        return cost if cost is not None and cost >= 0 else None
    components = [b for b in fixture["boms"] if b["bundle_sku_id"] == sku_id]
    if not components:
        return None
    value = 0
    for component in components:
        item = skus.get(component["component_sku_id"])
        if item is None or item["cost_cent"] is None or item["cost_cent"] < 0 or component["qty"] <= 0:
            return None
        value += item["cost_cent"] * component["qty"]
    return value


statuses = Counter()
for tenant, http in responses.items():
    fixture, case = facts[tenant], cases[tenant]
    initial_warehouse, initial_sku = fixture_keys[tenant]
    restricted = "restricted_warehouse" in case
    if restricted:
        check({g["warehouse_id"] for g in fixture["grants"]} == {initial_warehouse},
              "stored warehouse grant")
    for path, status, body in http:
        statuses[str(status)] += 1
        route = urlsplit(path).path
        query = {k: values[0] for k, values in parse_qs(urlsplit(path).query, keep_blank_values=True).items()}
        if status != 200:
            if status == 403:
                check(restricted and query.get("warehouse_id") != initial_warehouse,
                      "explicit outside warehouse refused")
            elif status == 400:
                check("invalid_ranges" in case, "invalid decimal range refused")
            elif status == 422:
                expected = ("inventory_safety_stock_invalid" if "invalid_policy" in case else
                            "inventory_read_value_overflow" if "cost_overflow" in case else
                            "inventory_cost_missing")
                check(body["code"] == expected, "typed quantity/policy/value rejection")
                if "missing_sku_and_kit_cost" in case:
                    missing = sum(unit_cost(fixture, b["sku_id"]) is None for b in fixture["balances"])
                    check(body["missing_count"] == missing == 2, "missing estimate independently counted")
            elif status == 409:
                check(body["code"] in {"inventory_aggregate_precision_required",
                                       "inventory_quantity_reconciliation_required"},
                      "typed precision/reconciliation rejection")
                if body["code"] == "inventory_aggregate_precision_required":
                    check(body["required_schema"] == 2, "legacy report upgrade is explicit")
            else:
                raise AssertionError("unexpected HTTP status")
            continue

        balances = [b for b in fixture["balances"] if b["on_hand_micros"] is not None]
        if restricted:
            balances = [b for b in balances if b["warehouse_id"] == initial_warehouse]
        if query.get("warehouse_id"):
            balances = [b for b in balances if b["warehouse_id"] == query["warehouse_id"]]
        if query.get("sku_id"):
            balances = [b for b in balances if b["sku_id"] == query["sku_id"]]
        if route == "/reports/inventory/aggregate":
            if "aggregate_scales" in case:
                balances = [b for b in balances if (b["sku_id"] == initial_sku) == (body["schema"] == 2)]
            if "unknown_history" in case:
                balances = [b for b in balances if b["sku_id"] == initial_sku]
            if "wide_aggregate" in case:
                balances = [b for b in balances if b["sku_id"] != initial_sku]
            check(body["schema"] in (1, 2) and body["measure"] in ("quantity", "cost"), "report metadata")
            scale = (6 if body["measure"] == "quantity" else 8) if body["schema"] == 2 else (
                0 if body["measure"] == "quantity" else 2)
            check(body["amount_scale"] == scale, "report amount scale")
            check(body["valuation_source"] == ("sku_master_estimate" if body["measure"] == "cost" else None),
                  "report valuation provenance")
            groups = defaultdict(list)
            warehouses = {w["warehouse_id"]: w["code"] for w in fixture["warehouses"]}
            skus = {s["sku_id"]: s for s in fixture["skus"]}
            for balance in balances:
                value = balance["on_hand_micros"]
                if body["measure"] == "cost":
                    value *= unit_cost(fixture, balance["sku_id"])
                if body["schema"] == 1:
                    check(value % 1_000_000 == 0, "legacy report exact whole-unit compatibility")
                    value //= 1_000_000
                groups[(warehouses[balance["warehouse_id"]], skus[balance["sku_id"]]["sku_kind"])].append(value)
            check(len(body["buckets"]) == len(groups), "all authorized report groups")
            summaries = [(b["summary"], groups[(b["warehouse_code"], b["kind"])]) for b in body["buckets"]]
            summaries.append((body["total"], [v for group in groups.values() for v in group]))
            for summary, values in summaries:
                check(summary["count"] == len(values), "report row count")
                check(int(summary["sum"]) == sum(values), "report exact sum")
                check(int(summary["min"]) == min(values) and int(summary["max"]) == max(values),
                      "report exact extrema")
            check(datetime.fromisoformat(body["as_of"].replace("Z", "+00:00")) <= datetime.now(timezone.utc),
                  "real report snapshot clock")
            continue

        if route == "/inventory/ledgers":
            available = [l for l in fixture["ledgers"]
                         if not restricted or l["warehouse_id"] == initial_warehouse]
            check(body["total"] == len(available), "full authorized ledger count")
            by_id = {l["ledger_id"]: l for l in available}
            for row in body["items"]:
                source = by_id[row["ledger_id"]]
                precise = source["delta_on_hand_micros"] is not None
                check(row["quantities"]["source"] == ("canonical_micros" if precise else "legacy_whole_units"),
                      "ledger precision provenance")
                for field in fields:
                    name = "delta_" + field
                    expected = source[name + "_micros"] if precise else source[name] * 1_000_000
                    check(scaled(row["quantities"][name], 6) == expected, "ledger exact amount")
                    check(row[name] == source[name], "original ledger integer semantics")
            continue

        skus = {s["sku_id"]: s for s in fixture["skus"]}
        for name in ("barcode", "tag", "sku_code"):
            if query.get(name):
                balances = [b for b in balances if (query[name].lower() in " ".join(skus[b["sku_id"]]["tags"]).lower()
                            if name == "tag" else skus[b["sku_id"]].get(name) == query[name])]
        by_key = defaultdict(list)
        for balance in balances:
            key = (balance["warehouse_id"], balance["sku_id"]) if route == "/inventory" else balance["sku_id"]
            by_key[key].append(balance)
        aggregates = {}
        for key, group in by_key.items():
            values = {name: sum(b[name + "_micros"] for b in group) for name in fields}
            values["available"] = values["on_hand"] - values["reserved"] - values["frozen"]
            if any(query.get(bound) and (values["available"] < scaled(query[bound], 6) if bound == "available_min"
                                        else values["available"] > scaled(query[bound], 6))
                   for bound in ("available_min", "available_max")):
                continue
            if query.get("available_below_safety") == "true":
                # Successful calls preceded deliberate policy corruption in this fixture.
                threshold = 100001 if "invalid_policy" in case else 1000000
                if values["available"] >= threshold:
                    continue
            aggregates[key] = values
        if route != "/inventory":
            check(body["total"] == len(aggregates), "full filtered SKU count before pagination")
        rows = body["data"] if route == "/inventory" else body["items"]
        check(len(rows) <= min(2000, max(1, int(query.get("limit", "50")))), "bounded page size")
        for row in rows:
            key = (row["warehouse_id"], row["sku_id"]) if route == "/inventory" else row["sku_id"]
            expected = aggregates[key]
            for name, value in expected.items():
                check(scaled(row["quantities"][name], 6) == value, "exact read quantity: " + name)
                check(row[name] == value // 1_000_000, "projection after exact arithmetic: " + name)
            if route == "/inventory":
                check(row["quantity_state"] == "precise", "public list precise state")
                check(row["cost_cent"] == unit_cost(fixture, row["sku_id"]), "nullable master cost")
            if route == "/inventory/costs":
                cost = unit_cost(fixture, row["sku_id"])
                check(row["valuation_source"] == "sku_master_estimate", "cost estimate provenance")
                check(row["unit_cost_cent"] == cost, "nullable unit cost")
                if cost is None:
                    check(row["estimated_value"] is None and row["total_cost_cent"] is None,
                          "unknown cost never becomes zero")
                else:
                    raw = expected["on_hand"] * cost
                    check(scaled(row["estimated_value"], 8) == raw, "exact sub-cent or wide estimate")
                    check(row["total_cost_cent"] == raw // 1_000_000, "old cent compatibility")
        if "cursor" in query:
            check(body["next_cursor"] is None, "last page has no phantom cursor")
            previous = [b for p, s, b in http if p == "/inventory?limit=1" and s == 200][0]
            check(previous["data"][0]["sku_id"] != rows[0]["sku_id"], "real cursor page differs")

inputs_before = read("final-native-before-source.json")["files"]
inputs_after = read("final-native-after-source.json")["files"]
drift = sorted(name for name in inputs_before.keys() | inputs_after.keys()
               if inputs_before.get(name) != inputs_after.get(name))
leaves = ["crates/erp-repo/src/inventory_reads.rs", "crates/erp-repo/src/inventory.rs",
          "crates/erp-repo/src/inventory_grid_report.rs", "crates/erp-api/src/inventory_views.rs",
          "crates/erp-api/src/inventory_print_report_handlers/inventory_core.rs",
          "crates/erp-api/src/settings_payloads.rs", "crates/erp-api/tests/free_erp_acceptance/inventory_reads.rs"]
for name in leaves:
    check(inputs_before[name] == inputs_after[name], "stable selected read input: " + name)
evidence = {"result": "PASS", "checks": checks, "protected_identity": identity,
            "new_fixtures": len(cases), "pre_existing_fixtures_unchanged": len(old),
            "canonical_balance_keys": canonical, "unknown_keys_unchanged": unknown,
            "http_responses": sum(statuses.values()), "http_status_counts": dict(statuses),
            "native_registered_tests": 68, "native_input_count": len(inputs_before),
            "native_source_certification": "PARTIAL" if drift else "PASS",
            "native_input_drift": drift, "selected_read_inputs_stable": leaves,
            "legacy_all_null_ledger_rows_observed": legacy_fallback,
            "limits": ["synthetic JWT route calls are not password/session acceptance",
                       "POST selected scopes are matched by explicit cases, not logged request bodies",
                       "new whole-unit ledger inserts have canonical fields populated by the QA trigger",
                       "empty outbox proves only unmapped fixture read safety, not mapped-channel execution",
                       "no Release performance, real native/Web interaction, migration chain or hosting acceptance"],
            "evidence_sha256": {name: hashlib.sha256((base / (prefix + name)).read_bytes()).hexdigest()
                                for name in ["final-native-full.log", "final-qa-before.json", "final-qa-after.json",
                                             "final-native-before-source.json", "final-native-after-source.json",
                                             "final-native-executable.json"]}}
destination = base / (prefix + "independent-readback.json")
destination.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
print(json.dumps({k: evidence[k] for k in ["result", "checks", "new_fixtures", "http_responses",
                                         "pre_existing_fixtures_unchanged", "native_source_certification"]}))
