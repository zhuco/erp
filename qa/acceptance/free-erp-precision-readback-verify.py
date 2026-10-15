"""Compare independent QA readback with the actual precision test fixtures."""

from datetime import datetime, timezone
from decimal import Decimal
import argparse
import hashlib
import json
from pathlib import Path
import re

base = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("--stage", choices=("second", "third", "fourth", "fifth", "atp"), default="second")
stage = parser.parse_args().stage
extended_stage = stage in ("fourth", "fifth", "atp")
readback_file = base / ("free-erp-precision-readback.json" if stage == "second"
                        else f"free-erp-precision-{stage}-readback.json")
baseline_file = base / ("free-erp-precision-qa-final-before.json" if stage == "second"
                        else f"free-erp-precision-{stage}-qa-before.json")
log_file = base / f"free-erp-precision-full-{stage}.log"
readback = json.loads(readback_file.read_text(encoding="utf-8-sig"))
baseline = json.loads(baseline_file.read_text(encoding="utf-8-sig"))
log = log_file.read_text(encoding="utf-8-sig")
errors = []
checks = 0


def check(condition, label):
    global checks
    checks += 1
    if not condition:
        errors.append(label)


identity = ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438)
for source in (readback, baseline):
    check(tuple(source[key] for key in ("database", "role", "host", "port")) == identity,
          "Protected QA identity")
old_ids = set(baseline["tenant_ids"])
fresh = {row["tenant_id"]: row for row in readback["tenants"] if row["tenant_id"] not in old_ids}
cases = {}
pending = []
for line in log.splitlines():
    fixture = re.search(r"QA inventory precision tenant=([\w-]+) order=([\w-]+)", line)
    if fixture:
        pending.append({"tenant_id": fixture[1], "order_id": fixture[2]})
    passed = re.search(r"test inventory_precision::(\w+) \.\.\. ok", line)
    if passed:
        cases[passed[1]] = pending
        pending = []
expected = {
    "precision_direct_writers_cannot_bypass_sources_or_rewrite_history": (750000, 500000, 0, 750000),
    "precision_fractional_batch_issue_and_return_share_physical_stock": (875000, 0, 0, 875000),
    "precision_half_reserved_keeps_a_whole_trade_sale_available": (500000, 500000, 0, 500000),
    "precision_http_whole_outbound_reports_conflict_without_stock_effect": (1900000, 900000, 900000, 1900000),
    "precision_manufacturing_quarter_blocks_trade_whole_unit": (0, 0, 0, 0),
    "precision_manufacturing_source_reservation_replays_original_quantity_after_spending": (500000, 200000, 0, 500000),
    "precision_mirrored_whole_history_is_mapped_without_double_counting": (1000000, 0, 0, 1000000),
    "precision_reserved_and_frozen_are_subtracted_before_rounding": (1900000, 900000, 900000, 1900000),
    "precision_twenty_fractional_outgoings_never_overdraw_one_balance": (100000, 0, 0, 100000),
}
blocked = {
    "precision_unknown_offsetting_manufacturing_history_still_requires_sources",
    "precision_qa_named_opening_without_proven_source_is_rejected",
    "precision_unmirrored_manufacturing_history_requires_reconciliation",
}
proven_history_case = "precision_proven_fractional_history_activates_without_rounding_away_stock"
shared_history_case = "precision_unproven_shared_history_requires_reconciliation"
shared_fence_case = "precision_shared_writer_waits_for_activation_then_cannot_bypass_it"
if extended_stage:
    expected[shared_fence_case] = (750000, 0, 0, 750000)
    blocked.add(shared_history_case)
atp_unknown_case = "precision_atp_unknown_history_and_foreign_sku_are_explicit"
if stage == "atp":
    for name in (
        "precision_atp_decimal_requests_and_invalid_inputs_do_not_write_stock",
        "precision_atp_fefo_preserves_fractions_and_excludes_expired_batches",
        "precision_atp_subtracts_fractional_reservation_and_freeze_before_projection",
    ):
        expected[name] = (1900000, 900000, 900000, 1900000)
    expected["precision_atp_batch_missing_precision_and_mismatch_require_reconciliation"] = (1000000, 0, 0, 1000000)
    expected["precision_atp_batch_budget_rejects_without_silent_truncation"] = (1001000000, 0, 0, 1001000000)
expected_cases = set(expected) | blocked
if extended_stage:
    expected_cases.add(proven_history_case)
if stage == "atp":
    expected_cases.add(atp_unknown_case)
check(set(cases) == expected_cases, "Actual complete log contains all expected precision cases")
fixtures = [fixture for rows in cases.values() for fixture in rows]
check({fixture["tenant_id"] for fixture in fixtures} == set(fresh), "Only fresh fixtures read back")
check(len(fresh) == (23 if stage == "atp" else 17 if extended_stage else 13), "Expected fresh test and foreign fixtures")
for name, rows in cases.items():
    check(len(rows) == (2 if name.startswith("precision_direct") or name == proven_history_case else 1),
          f"{name}: fixture count")
    if not rows or rows[0]["tenant_id"] not in fresh:
        errors.append(f"{name}: fixture missing from independent readback")
        continue
    tenant = fresh[rows[0]["tenant_id"]]
    check(len(tenant["balances"]) == 1, f"{name}: physical balance count")
    balance = tenant["balances"][0]
    check(tenant["shared_stock_count"] == (1 if name == shared_history_case else 0),
          f"{name}: expected shared source facts only")
    if stage == "atp" and name == atp_unknown_case:
        check(balance["on_hand_micros"] is None and tenant["activation_count"] == 0,
              "ATP historical GET does not perform activation")
        check(balance["legacy_on_hand"] == "1" and balance["ledger"]["count"] == 1,
              "ATP original unactivated physical key remains unchanged")
        check(tenant["sku_count"] == 3 and tenant["orphan_ledger_count"] == 1,
              "Empty and orphan-history SKUs remain distinct without fabricated balances")
        check(len(tenant["commands"]) == 0, "ATP GET creates no command receipts")
        continue
    if name in blocked:
        check(balance["on_hand_micros"] is None and tenant["activation_count"] == 0,
              f"{name}: unproven history remains inactive")
        check(balance["legacy_on_hand"] == "1" and balance["ledger"]["count"] == 1
              and len(tenant["commands"]) == 0, f"{name}: rejection left no stock or receipt effect")
        source_field = "shared_sources" if name == shared_history_case else "manufacturing_sources"
        check(any(source["inventory_source_count"] != 1 for source in tenant[source_field]),
              f"{name}: independently visible source gap")
        if name == shared_history_case:
            check(len(tenant["shared_sources"]) == 1
                  and tenant["shared_sources"][0]["delta_micros"] == "-500000"
                  and tenant["shared_sources"][0]["inventory_source_count"] == 0,
                  "Unproven shared half-unit source remains visible and has no canonical mirror")
        continue
    if name == proven_history_case:
        for fixture_index, fixture in enumerate(rows):
            history_tenant = fresh[fixture["tenant_id"]]
            check(len(history_tenant["balances"]) == 1, "Proven history: one physical key")
            history_balance = history_tenant["balances"][0]
            exact_after = (750000, 2000000)[fixture_index]
            check(int(history_balance["on_hand_micros"]) == exact_after
                  and int(history_balance["ledger"]["on_hand_micros"]) == exact_after,
                  f"Proven history #{fixture_index}: preserve sourced fractions")
            check(history_tenant["activation_count"] == 1
                  and history_balance["source_map_count"] == (2, 3)[fixture_index],
                  f"Proven history #{fixture_index}: original sources mapped once")
            check(history_tenant["shared_stock_count"] == 0
                  and all(source["inventory_source_count"] == 1
                          for source in history_tenant["manufacturing_sources"]),
                  f"Proven history #{fixture_index}: no second stock or missing physical source")
            for bucket in ("on_hand", "reserved", "frozen", "in_transit"):
                exact = int(history_balance[bucket + "_micros"])
                check(exact >= 0 and exact == int(history_balance["ledger"][bucket + "_micros"]),
                      f"Proven history #{fixture_index}: {bucket} ledger conservation")
                check(int(history_balance["legacy_" + bucket]) == exact // 1000000,
                      f"Proven history #{fixture_index}: {bucket} whole compatibility")
            check(int(history_balance["available_micros"]) == exact_after,
                  f"Proven history #{fixture_index}: exact available")
        continue
    actual = tuple(int(balance[field]) for field in ("on_hand_micros", "reserved_micros", "frozen_micros"))
    actual += (int(balance["ledger"]["on_hand_micros"]),)
    check(actual == expected[name], f"{name}: physical stock and ledger expected {expected[name]}, got {actual}")
    check(tenant["activation_count"] == 1, f"{name}: one source-based activation")
    for bucket in ("on_hand", "reserved", "frozen", "in_transit"):
        exact = int(balance[bucket + "_micros"])
        check(exact >= 0, f"{name}: nonnegative {bucket}")
        check(exact == int(balance["ledger"][bucket + "_micros"]), f"{name}: {bucket} ledger equality")
        check(int(balance["legacy_" + bucket]) == exact // 1000000, f"{name}: {bucket} whole projection")
    check(int(balance["available_micros"]) == actual[0] - actual[1] - actual[2] >= 0,
          f"{name}: subtract reservation and freeze before projection")
    check(all(source["inventory_source_count"] == 1 for source in tenant["manufacturing_sources"]),
          f"{name}: each physical manufacturing source has one canonical effect")
    check(all(command["status"] == "SUCCEEDED" for command in tenant["commands"]),
          f"{name}: original commands persisted terminal results")
    if name == "precision_fractional_batch_issue_and_return_share_physical_stock":
        check(len(tenant["batch_balances"]) == 1 and tenant["batch_balances"][0]["on_hand_micros"] == "875000",
              "Fractional batch independently agrees with canonical stock")
    if name == "precision_mirrored_whole_history_is_mapped_without_double_counting":
        check(balance["source_map_count"] == 2, "Opening and old issue sources both mapped without double counting")
    if name == "precision_manufacturing_source_reservation_replays_original_quantity_after_spending":
        own = [row for row in tenant["reservations"] if row["source_id"] == rows[0]["order_id"]]
        other = [row for row in tenant["reservations"] if row["source_id"] != rows[0]["order_id"]]
        check(len(own) == 1 and (own[0]["qty_micros"], own[0]["original_qty_micros"], own[0]["status"])
              == ("0", "500000", "CONSUMED"), "Original reservation preserved after full spending")
        check(len(other) == 1 and other[0]["qty_micros"] == "200000", "Other work-order reservation preserved")
        check(len(tenant["commands"]) == 4, "Reservation retries created no extra command effects")
    if stage == "atp" and name.startswith("precision_atp_"):
        check(balance["ledger"]["count"] == 2 and len(tenant["commands"]) == 0,
              f"{name}: quotes add no physical ledger or command effects")
        if "fefo_preserves" in name:
            batches = {row["batch_no"]: row for row in tenant["batch_balances"]}
            check(set(batches) == {"EXPIRED", "EARLY", "NO-EXPIRY"}, "Actual FEFO batch identities")
            check({key: (row["on_hand_micros"], row["reserved_micros"]) for key, row in batches.items()} == {
                "EXPIRED": ("1600000", "0"), "EARLY": ("250000", "200000"), "NO-EXPIRY": ("50000", "0")
            }, "All batch physical and reserved facts remain unchanged")
            check(datetime.fromisoformat(batches["EXPIRED"]["expire_at"]).timestamp() < datetime.now(timezone.utc).timestamp(),
                  "The excluded batch really is expired")
            check(batches["NO-EXPIRY"]["expire_at"] is None, "Nonexpiring last FEFO batch")
        if "batch_missing_precision" in name:
            check(sorted(int(row["on_hand_micros"]) for row in tenant["batch_balances"]) == [1000000, 9223372036854775807],
                  "Final mismatch is independently visible beyond the signed 64-bit sum")
        if "batch_budget" in name:
            check(len(tenant["batch_balances"]) == 1001, "Complete 1001-batch fixture retained")
            check(sum(int(row["on_hand_micros"]) for row in tenant["batch_balances"]) == actual[0],
                  "Rejected budget fixture is physically consistent; not a fabricated inventory error")
check(bool(re.search(r"blocked_chain=2 accepted=1 rejected=19", log)), "Two actual blocking-chain waiters, one success and nineteen genuine insufficiencies")
if extended_stage:
    check(bool(re.search(r"QA precision shared fence tenant=[\w-]+ blocked=1 bypass_effects=0", log)),
          "A real blocked shared writer leaves no bypass effects after activation")
    foreign_fixture = cases["precision_direct_writers_cannot_bypass_sources_or_rewrite_history"][1]
    foreign_tenant = fresh[foreign_fixture["tenant_id"]]
    check(foreign_tenant["activation_count"] == 0 and foreign_tenant["shared_stock_count"] == 0,
          "Rejected foreign-warehouse write does not activate or change foreign sources")
    check(len(foreign_tenant["balances"]) == 1
          and foreign_tenant["balances"][0]["on_hand_micros"] is None
          and foreign_tenant["balances"][0]["legacy_on_hand"] == "1"
          and len(foreign_tenant["commands"]) == 0,
          "Foreign fixture preserves its original balance and zero command effects")
if stage == "atp":
    quoted = []
    for line in log.splitlines():
        match = re.search(r"QA precise ATP tenant=([\w-]+) request=(\S+) status=(\d{3}).*? body=(.*)$", line)
        if match:
            quoted.append((match[1], match[2], int(match[3]), json.loads(match[4])))
    check(len(quoted) == 19, "All real ATP requests and rejection responses recorded")
    for tenant_id, requested, status, body in quoted:
        check(tenant_id in fresh, "Actual ATP request belongs to the independently read fixture")
        if status != 200:
            continue
        q = {key: int(Decimal(value) * 1000000) for key, value in body["quantities"].items()}
        check(q["request_qty"] == int(Decimal(requested) * 1000000), "Original decimal request retained")
        check(q["atp_qty"] == min(q["request_qty"], q["available"])
              and q["shortage_qty"] == q["request_qty"] - q["atp_qty"], "Exact ATP and shortage conservation")
        check(q["available"] <= q["on_hand"] - q["reserved"] - q["frozen"] and q["available"] >= 0,
              "Actual quote cannot exceed precise stock after reservations and freezes")
        check(body["available"] == q["available"] // 1000000 and body["atp_qty"] == q["atp_qty"] // 1000000
              and body["shortage_qty"] == q["request_qty"] // 1000000 - q["atp_qty"] // 1000000,
              "Legacy whole callers receive conservative projections")
        if body["allocations"]:
            allocated = sum(int(Decimal(row["quantity"]) * 1000000) for row in body["allocations"])
            check(allocated == q["atp_qty"], "Exact batch allocations sum to promised quantity")
            check(all(row["batch_no"] != "EXPIRED" for row in body["allocations"]), "No expired batch in actual recommendation")
full_test_count = 55 if stage == "atp" else 46 if extended_stage else 43
check(bool(re.search(rf"test result: ok\. {full_test_count} passed; 0 failed; 0 ignored;", log)),
      f"Full {full_test_count}-test result")
report = {
    "updated_at": datetime.now(timezone.utc).isoformat(),
    "status": "FAIL" if errors else "PASS",
    "scope": f"Independent protected-QA physical balances, four ledger buckets, original manufacturing/shared sources, source reservations, batch stock and command effects from the {stage} full run; not public login, whole ERP or performance acceptance",
    "checks": checks, "fresh_fixture_count": len(fresh), "precision_cases": len(cases),
    "case_fixtures": cases, "errors": errors,
    "input_sha256": {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
                     for path in (readback_file, baseline_file, log_file)},
}
(base / ("free-erp-precision-readback-validation.json" if stage == "second"
         else f"free-erp-precision-{stage}-readback-validation.json")).write_text(
    json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({key: report[key] for key in ("status", "checks", "fresh_fixture_count", "precision_cases", "errors")}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
