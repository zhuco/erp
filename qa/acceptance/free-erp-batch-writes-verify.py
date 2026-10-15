"""Independent complete-row readback; preserve each immutable execution report."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
qa = root / "qa/acceptance"
p = argparse.ArgumentParser()
p.add_argument("stage")
p.add_argument("--original", action="store_true")
p.add_argument("--report-id")
args = p.parse_args()
if not re.fullmatch(r"[a-z0-9-]+", args.stage):
    raise SystemExit("invalid stage")
report_id = args.report_id or args.stage
if not re.fullmatch(r"[a-z0-9-]+", report_id):
    raise SystemExit("invalid report id")
destination = qa / f"free-erp-batch-writes-{report_id}-independent.json"
if destination.exists():
    raise SystemExit("refusing to replace historical report")
files = {name: qa / f"free-erp-batch-writes-{args.stage}-{name}{suffix}"
         for name, suffix in [("before", ".jsonl"), ("after", ".jsonl"), ("test", ".log")]}
raw = files["test"].read_bytes()
log = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
fixtures = {}
for tenant, case, actor, warehouse, sku, a, b in re.findall(
    r"QA batch_write_fixture tenant=(\S+) case=(\S+) actor=(\S+) warehouse=(\S+) sku=(\S+) a=(\S+) b=(\S+)", log):
    fixtures[tenant] = {"case": case, "actor": actor, "warehouse": warehouse, "sku": sku, "a": a, "b": b}
http = []
for line in log.splitlines():
    m = re.search(r"QA batch_write_http tenant=(\S+) path=(\S+) method=(\S+) input=(.*) status=(\d+) response=(.*)", line)
    if m:
        http.append({"tenant": m[1], "path": m[2], "method": m[3], "input": json.loads(m[4]),
                     "status": int(m[5]), "response": json.loads(m[6])})
errors = []
def require(ok, message):
    if not ok:
        errors.append(message)

headers = {}
before = {}
after_owned = {}
old_changes = []
all_counts = {}
for name in ["before", "after"]:
    counts = Counter()
    with files[name].open(encoding="utf8") as stream:
        header = json.loads(next(stream))
        headers[name] = header
        require((header["database"], header["role"], header["host"], header["port"]) ==
                ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), f"{name}: protected identity")
        for line in stream:
            row = json.loads(line)
            tid = row["tenant_id"]
            counts["tenants"] += 1
            counts["rows"] += sum(len(v) for v in row["facts"].values())
            digest = hashlib.sha256(json.dumps(row, ensure_ascii=False, sort_keys=True,
                                              separators=(",", ":")).encode()).hexdigest()
            if name == "before":
                before[tid] = digest
            else:
                if tid in before and before[tid] != digest:
                    old_changes.append(tid)
                if tid in fixtures:
                    after_owned[tid] = row["facts"]
    all_counts[name] = dict(counts)
require(not old_changes, f"preexisting enterprise complete facts changed: {old_changes}")
require(set(fixtures) <= set(after_owned), "fixture readback is incomplete")
require(not (set(fixtures) & set(before)), "test fixtures must be new enterprises")
case_results = []
for tid, f in fixtures.items():
    facts = after_owned.get(tid, {})
    requests = [h for h in http if h["tenant"] == tid]
    balances = [r for r in facts.get("inventory_balances", []) if r["sku_id"] == f["sku"]]
    ledgers = [r for r in facts.get("inventory_ledgers", []) if r["sku_id"] == f["sku"]]
    quantities = [r for r in facts.get("inventory_batch_balances", []) if r["sku_id"] == f["sku"]]
    if balances:
        for column in ["on_hand", "reserved", "frozen", "in_transit"]:
            total = sum(r[f"delta_{column}_micros"] if r[f"delta_{column}_micros"] is not None
                        else r[f"delta_{column}"] * 1_000_000 for r in ledgers)
            require(balances[0][f"{column}_micros"] == total, f"{f['case']}: exact SKU {column} ledger total")
    children = facts.get("inventory_batch_movements", [])
    by_ledger = {r["ledger_id"]: r for r in ledgers}
    child_sums = Counter()
    for c in children:
        parent = by_ledger.get(c["inventory_ledger_id"])
        require(parent is not None, f"{f['case']}: actual parent ledger missing")
        if parent:
            require(all(c[k] == parent[k] for k in ["tenant_id", "warehouse_id", "sku_id", "reason_type", "ref_type", "ref_id"]),
                    f"{f['case']}: child source or enterprise key mismatch")
        for bucket in ["on_hand", "reserved"]:
            require(c[f"after_{bucket}_micros"] - c[f"before_{bucket}_micros"] == c[f"delta_{bucket}_micros"],
                    f"{f['case']}: immutable child exact arithmetic")
        m = c["batch_metadata_snapshot"]
        require(m["tenant_id"] == tid and m["sku_id"] == f["sku"] and m["batch_id"] == c["batch_id"],
                f"{f['case']}: original batch metadata ownership")
        child_sums[c["inventory_ledger_id"]] += c["delta_on_hand_micros"]
    for ledger, total in child_sums.items():
        require(by_ledger[ledger]["delta_on_hand_micros"] == total, f"{f['case']}: child sum differs from actual parent")
    known = all(r["on_hand_micros"] is not None and r["reserved_micros"] is not None for r in quantities)
    exact_total = sum(r["on_hand_micros"] for r in quantities) if known else None
    case_results.append({"tenant": tid, **f, "http_statuses": [r["status"] for r in requests],
        "http_count": len(requests), "batch_precision_known": known,
        "batch_on_hand_micros": exact_total,
        "sku_on_hand_micros": balances[0]["on_hand_micros"] if balances else None,
        "children": len(children) if headers["after"]["batch_movements_present"] else None,
        "batch_rows": quantities, "http": requests})
cases = {r["case"]: r for r in case_results}
expected_cases = {"legacy_inbounds", "legacy_outbounds", "legacy_adjustments", "foreign_batch", "foreign_owner",
                  "wrong_sku", "unknown_precision", "batch_reserved", "expired_outbound", "new_empty_batch",
                  "original_result", "concurrent_original", "material_unknown", "material_return_consume"}
require(expected_cases <= set(cases), "all 13 regression case fixtures must be independently read")
results = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out; finished in (\S+)", log)
require(len(results) == 1, "one actual terminal test summary required")
if results:
    label, passed, failed, ignored, filtered, elapsed = results[0]
    test_result = dict(label=label, passed=int(passed), failed=int(failed), ignored=int(ignored), filtered=int(filtered), elapsed=elapsed)
else:
    test_result = None
if args.original:
    require(test_result is not None and test_result["passed"] == 0 and test_result["failed"] == 13 and test_result["ignored"] == 0,
            "original run must retain the 13 actually executed failures")
    require(not headers["before"]["batch_movements_present"] and not headers["after"]["batch_movements_present"],
            "original schema must record missing batch provenance, never treat absence as zero-effect success")
    if expected_cases <= set(cases):
        for case in ["legacy_inbounds", "legacy_outbounds", "legacy_adjustments", "batch_reserved", "expired_outbound"]:
            require(cases[case]["http_statuses"] == [500], f"{case}: original legacy exact write failed with 500")
        for case in ["foreign_batch", "wrong_sku", "unknown_precision", "new_empty_batch", "material_unknown"]:
            require(cases[case]["http_statuses"] == [200], f"{case}: original bug returned actual 200")
        require(cases["original_result"]["http_statuses"] == [400], "original UUID command was rejected")
        require(cases["concurrent_original"]["http_statuses"] == [400] * 20, "all 20 original concurrent results captured")
        require(cases["material_return_consume"]["http_statuses"] == [200] * 3, "actual material issue/return/consume outcomes")
        require(cases["material_unknown"]["batch_on_hand_micros"] == 3_750_000 and
                cases["material_unknown"]["sku_on_hand_micros"] == 4_650_000,
                "original material write guessed unknown history and broke batch-to-SKU totals")
else:
    require(test_result is not None and test_result["failed"] == 0 and test_result["ignored"] == 0,
            "fixed execution must finish with no ignored or failed case")
    require(headers["after"]["batch_movements_present"], "fixed run requires real child provenance schema")
    for r in case_results:
        if r["case"] not in {"unknown_precision", "material_unknown"}:
            require(r["batch_precision_known"], f"{r['case']}: known exact batches required")
        if r["case"] == "inconsistent_totals":
            require(r["http_statuses"] == [409] and r["batch_on_hand_micros"] == 4_800_000
                    and r["sku_on_hand_micros"] == 4_900_000 and r["children"] == 0,
                    "deliberate inconsistent history remains unchanged and is rejected")
        elif r["batch_precision_known"]:
            require(r["batch_on_hand_micros"] == r["sku_on_hand_micros"], f"{r['case']}: exact batch/SKU sum")
        else:
            require(all(h["status"] == 409 for h in r["http"]), f"{r['case']}: unknown exact history must be rejected")
    if "concurrent_original" in cases:
        require(cases["concurrent_original"]["http_statuses"] == [200] * 20 and cases["concurrent_original"]["children"] == 1,
                "20 original concurrent submissions need one actual physical batch effect")
    for case, table in [("child_failure", "inventory_batch_movements"), ("audit_failure", "audit_logs")]:
        if case in cases:
            require(cases[case]["http_statuses"] == [500, 200, 200] and cases[case]["children"] == 1,
                    f"{case}: actual fault, same-ID retry and immutable replay")
            require(f"table={table} actual_marker=true unchanged=true" in log,
                    f"{case}: real required INSERT reached and complete rollback observed")
    if "metadata_lock" in cases:
        require(cases["metadata_lock"]["http_statuses"] == [409] and cases["metadata_lock"]["children"] == 0,
                "metadata lock: expired admission has no stock effect")
        require("actual_wait=true status=409 physical_unchanged=true" in log,
                "actual PostgreSQL lock wait for the owned HTTP query required")
    if "capacity_limit" in cases:
        require(cases["capacity_limit"]["http_statuses"] == [200, 422] and
                len(cases["capacity_limit"]["batch_rows"]) == 1001 and cases["capacity_limit"]["children"] == 1,
                "1000 complete rows admitted; 1001 rejects without another batch effect")
    if "fefo_split" in cases:
        require(cases["fefo_split"]["children"] == 2 and cases["fefo_split"]["sku_on_hand_micros"] == 3_900_000,
                "FEFO actual two-child debit")
    if "immutable_source" in cases:
        require("mutations_rejected=3 unchanged=true" in log, "original child and parent mutations rejected")
    if "exact_large" in cases:
        require(cases["exact_large"]["children"] == 1 and cases["exact_large"]["sku_on_hand_micros"] == 9_007_199_259_900_000,
                "exact large quantity read independently")
        for h in cases["exact_large"]["http"]:
            child = h["response"].get("batch_effects", [{}])[0]
            require(child.get("delta_on_hand_micros") == "9007199255000000" and
                    child.get("after_on_hand_micros") == "9007199256900000",
                    "large exact API quantities must use original decimal strings")
source_before = qa / f"free-erp-inventory-consumers-batch-writes-{args.stage}-run-before-source.json"
source_after = qa / f"free-erp-inventory-consumers-batch-writes-{args.stage}-after-source.json"
source_drift = None
if source_before.exists() and source_after.exists():
    a = json.loads(source_before.read_text(encoding="utf8"))["files"]
    b = json.loads(source_after.read_text(encoding="utf8"))["files"]
    source_drift = [k for k in sorted(set(a) | set(b)) if a.get(k) != b.get(k)]
require(source_drift is not None, "before/after build source manifests required")
status = "FAIL" if errors else ("PARTIAL" if source_drift else "PASS")
report = {"created_at": datetime.now(timezone.utc).isoformat(), "stage": args.stage, "status": status,
          "scope": "original bug proof" if args.original else "independent exact batch effects",
          "runtime_tests": test_result, "data_counts": all_counts, "complete_fact_tables": 37,
          "preexisting_enterprises": len(before), "preexisting_fact_changes": old_changes,
          "source_drift": source_drift, "headers": headers, "cases": case_results, "errors": errors,
          "artifacts": {k: {"path": v.relative_to(root).as_posix(), "bytes": v.stat().st_size,
                            "sha256": hashlib.sha256(v.read_bytes()).hexdigest()} for k, v in files.items()},
          "release_performance": "NOT_RUN", "desktop_web_acceptance": "NOT_RUN"}
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
print(json.dumps({"status": status, "errors": errors, "runtime_tests": test_result,
                  "old_enterprises": len(before), "source_drift": source_drift}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
