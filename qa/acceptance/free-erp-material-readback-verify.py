"""Verify independent PostgreSQL material readbacks, without invoking Rust code."""

from datetime import datetime, timezone
import json
from pathlib import Path

base = Path(__file__).resolve().parent
before = json.loads((base / "free-erp-material-readback.json").read_text(encoding="utf-8-sig"))
after = json.loads((base / "free-erp-material-readback-after-probe.json").read_text(encoding="utf-8-sig"))
probe = json.loads((base / "free-erp-material-trigger-probe.json").read_text(encoding="utf-8-sig"))
assert before == after, "Rollback probe changed selected business facts"
assert (before["database"], before["role"], before["host"], before["port"]) == (
    "erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438
)
expected_effects = {
    "wrong_sku": (1, 1),
    "replay_original_snapshot": (2, 2),
    "missing_foreign_source": (1, 1),
    "late_stock_failure": (0, 0),
    "same_sku_source_after_opening_correction": (3, 3),
    "twenty_same_identity": (1, 1),
    "whole_batch_return": (2, 2),
}
assert len(before["samples"]) == len(expected_effects)
checks = []
for sample in before["samples"]:
    case = sample["test_case"]
    assert (len(sample["moves"]), sample["command_receipts"]) == expected_effects[case], case
    precise = {row["sku_code"]: row for row in sample["precise_balances"]}
    legacy = {row["sku_code"]: row for row in sample["legacy_balances"]}
    assert set(precise) == {"QA-A", "QA-B"} and set(legacy) == set(precise), case
    for row in precise.values():
        assert row["on_hand_micros"] == row["ledger_micros"], (case, row)
    if case != "late_stock_failure":
        for row in legacy.values():
            assert row["on_hand"] == row["ledger_on_hand"], (case, row)
        for sku, row in precise.items():
            assert row["on_hand_micros"] == legacy[sku]["on_hand"] * 1_000_000, (case, sku)
    else:
        # This test deliberately starts with legacy A=0 and manufacturing A=1.
        # Its failure must preserve the inconsistent starting facts, not repair them.
        assert precise["QA-A"]["on_hand_micros"] == 1_000_000
        assert legacy["QA-A"]["on_hand"] == 0 and legacy["QA-A"]["ledger_on_hand"] == 1
    issues = {row["move_id"]: row for row in sample["moves"] if row["kind"] in ("ISSUE", "SUPPLEMENT")}
    spent = {}
    for move in sample["moves"]:
        assert move["warehouse_id"] and move["sku_id"] and move["qty_micros"] > 0
        if move["kind"] in ("RETURN", "CONSUME"):
            original = issues[move["original_issue_id"]]
            assert all(move[key] == original[key] for key in (
                "work_order_id", "warehouse_id", "sku_id", "batch_id"
            ))
            spent[original["move_id"]] = spent.get(original["move_id"], 0) + move["qty_micros"]
    for source, quantity in spent.items():
        assert quantity <= issues[source]["qty_micros"]
    if case == "replay_original_snapshot":
        original = next(iter(issues.values()))
        assert (original["shop_snapshot_micros"], original["consumed_snapshot_micros"]) == (1_000_000, 0)
    if case == "same_sku_source_after_opening_correction":
        assert len(issues) == 2 and sum(spent.values()) == 1_000_000
        assert precise["QA-A"]["on_hand_micros"] == 0
        assert precise["QA-B"]["on_hand_micros"] == 1_000_000
    if case == "whole_batch_return":
        assert sorted(row["on_hand"] for row in sample["batch_balances"]) == [0, 1]
    checks.append({
        "case": case, "status": "PASS", "moves": len(sample["moves"]),
        "receipts": sample["command_receipts"], "precise_ledger_balance": "PASS",
        "legacy_ledger_balance": "INTENTIONALLY_INCONSISTENT_SYNTHETIC_FAILURE_FIXTURE"
        if case == "late_stock_failure" else "PASS",
    })
assert before["foreign_tenant_effects"] == 0
history = before["historical_failed_run"]
assert (history["move_count"], history["unknown_identity_count"], history["issued_micros"], history["consumed_micros"]) == (
    2, 2, 1_000_000, 250_000
)
assert len(probe["probes"]) == 7
assert all(row["expected"] == row["observed"] and row["status"] == "PASS" for row in probe["probes"])
report = {
    "updated_at": datetime.now(timezone.utc).isoformat(), "status": "PASS",
    "scope": "Seven selected new QA tenants, physical/source identity, whole stock/ledger equality, rollback and unchanged historical unknown facts. Not a fractional manufacturing/trade shared-stock acceptance.",
    "checks": checks, "database_trigger_rejections": len(probe["probes"]),
    "probe_rollback_business_facts_unchanged": True,
    "historical_unknown_identity_rows_preserved": 2, "whole_inventory_only": True,
}
(base / "free-erp-material-readback-validation.json").write_text(
    json.dumps(report, indent=2) + "\n", encoding="utf-8"
)
print("PASS: seven material readbacks, seven database rejections, unchanged rollback and historical facts")
