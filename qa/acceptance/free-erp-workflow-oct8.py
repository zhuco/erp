"""Read-only workflow evidence; reuses the reviewed 77-table protection query."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-workflow-oct8-"
parser = argparse.ArgumentParser()
parser.add_argument("action", choices=["capture", "verify"])
parser.add_argument("stage")
parser.add_argument("--recheck", action="store_true")
args = parser.parse_args()
if not re.fullmatch(r"[a-z0-9-]+", args.stage):
    raise SystemExit("invalid evidence stage")

def exclusive(path, value):
    with path.open("x", encoding="utf8") as output:
        json.dump(value, output, ensure_ascii=False, indent=2)
        output.write("\n")

if args.action == "capture":
    source = BASE / "free-erp-receipt-authority-readback.sql"
    sql = source.read_text(encoding="utf-8-sig")
    if sql.count("'mfg_reports'") != 3 or sql.count("'selected_tables',77") != 1:
        raise SystemExit("review changed source snapshot query")
    sql = sql.replace("'mfg_reports'", "'mfg_reports','mfg_exceptions'")
    sql = sql.replace("'selected_tables',77", "'selected_tables',78")
    if args.stage.startswith("arrival"):
        sql = sql.replace("'mfg_reports'", "'mfg_reports','shared_purchase_revisions','shared_purchase_lines'")
        sql = sql.replace("'selected_tables',78", "'selected_tables',80")
    query = BASE / (PREFIX + args.stage + ".sql")
    with query.open("x", encoding="utf8") as output:
        output.write(sql)
    destination = BASE / (PREFIX + args.stage + ".jsonl")
    errors = BASE / (PREFIX + args.stage + ".stderr.log")
    command = ["G:/PostgreSQL/17/bin/psql.exe", "-X", "-q", "-t", "-A",
               "-h", "127.0.0.1", "-p", "15438", "-U", "erp_grid_test",
               "-d", "erp_shared_contract_test", "-v", "ON_ERROR_STOP=1", "-f", str(query)]
    with destination.open("xb") as output, errors.open("xb") as error:
        result = subprocess.run(command, cwd=ROOT, stdout=output, stderr=error, check=False)
    print(json.dumps({"stage": args.stage, "exit_code": result.returncode,
                      "bytes": destination.stat().st_size}))
    raise SystemExit(result.returncode)

report = BASE / (PREFIX + args.stage + ("-recheck" if args.recheck else "") + "-independent.json")
if report.exists():
    raise SystemExit("refusing to replace independent evidence")
checks, failures = 0, []
log = (BASE / (PREFIX + args.stage + "-test.log")).read_text(encoding="utf-8-sig")
owned = set(re.findall(r"QA (?:workflow_reported_scrap|workflow_frozen_quality|purchase_arrival) tenant=([0-9a-f-]+)", log))
def check(ok, message):
    global checks
    checks += 1
    if not ok:
        failures.append(message)

def snapshot(path):
    signatures, rows, digest, identity = {}, {}, hashlib.sha256(), None
    table_count = 80 if args.stage.startswith("arrival") else 78
    with path.open("rb") as stream:
        for raw in stream:
            digest.update(raw)
            if not raw.strip():
                continue
            value = json.loads(raw)
            if "database" in value:
                identity = value
                continue
            tenant = value["tenant_id"]
            check(tenant not in signatures, "unique enterprise " + tenant)
            check(len(value["facts"]) == table_count, "all selected tables " + tenant)
            signatures[tenant] = hashlib.sha256(json.dumps(value, sort_keys=True,
                ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
            # Compare every older enterprise by its full-row digest, keeping only
            # this batch's observed facts in memory rather than the entire database.
            if tenant in owned:
                rows[tenant] = value["facts"]
    check(identity is not None and identity.get("database") == "erp_shared_contract_test"
          and identity.get("role") == "erp_grid_test" and identity.get("host") == "127.0.0.1"
          and identity.get("port") == 15438 and identity.get("selected_tables") == table_count,
          "protected snapshot " + path.name)
    return signatures, rows, digest.hexdigest()

before, _, before_sha = snapshot(BASE / (PREFIX + args.stage + "-before.jsonl"))
after, rows, after_sha = snapshot(BASE / (PREFIX + args.stage + "-after.jsonl"))
changed = [tenant for tenant, digest in before.items() if after.get(tenant) != digest]
for tenant in before:
    check(tenant not in changed, "previous enterprise unchanged " + tenant)
matches = re.findall(r"QA workflow_reported_scrap tenant=([0-9a-f-]+) work_order=([0-9a-f-]+) report=([0-9a-f-]+) rejected=1000000 scrap=1000000 cost=1000 replay=true refused_receipts=0", log)
if not args.stage.startswith("arrival"):
    check(len(matches) == 1, "one actual workflow scrap observation")
for tenant, order, report_id in matches:
    facts = rows.get(tenant, {})
    actual_reports = [r for r in facts.get("mfg_reports", []) if r["report_id"] == report_id]
    check(len(actual_reports) == 1 and actual_reports[0]["work_order_id"] == order
          and actual_reports[0]["rejected_micros"] == 1_000_000
          and actual_reports[0]["good_micros"] == 0, "actual original rejected output")
    scraps = [r for r in facts.get("mfg_exceptions", []) if r["work_order_id"] == order and r["kind"] == "SCRAP"]
    check(len(scraps) == 1 and scraps[0]["qty_micros"] == 1_000_000
          and scraps[0]["cost_cent"] == 1_000, "one retained scrap destination")
    commands = facts.get("command_receipts", [])
    check(not any(r["request_id"] in {"ex-scrap-missing", "ex-scrap-over"} for r in commands),
          "no committed receipt for refused scrap")
    check(sum(r["request_id"] == "ex-scrap" for r in commands) == 1,
          "one immutable scrap command despite replay")
quality = re.findall(r"QA workflow_frozen_quality tenant=([0-9a-f-]+) work_order=([0-9a-f-]+) receipt=([0-9a-f-]+) freeze=([0-9a-f-]+) readings=3 proofs=1 available=0 status=HOLD replay=true", log)
if args.stage.startswith("frozen-quality"):
    check(len(quality) == 1, "one actual frozen quality observation")
for tenant, order, receipt, freeze in quality:
    facts = rows.get(tenant, {})
    readings = [r for r in facts.get("mfg_quality_items", []) if r["freeze_id"] == freeze]
    check(len(readings) == 3 and {r["item_code"] for r in readings} == {"width", "length", "length-pass"}
          and all(r["passed"] and r["reading_present"] and r["reading_micros"] == 1_000_000
                  and r["min_micros"] <= r["reading_micros"] <= r["max_micros"] for r in readings),
          "three actual frozen readings satisfy their original standards")
    bindings = [r for r in facts.get("shared_inspection_source_bindings", []) if r["freeze_id"] == freeze]
    check(len(bindings) == 1 and bindings[0]["source_id"] == order, "one owned immutable standard binding")
    proofs = [r for r in facts.get("mfg_quality_release_proofs", []) if r["receipt_id"] == receipt]
    check(len(proofs) == 1 and proofs[0]["work_order_id"] == order, "one original approval proof despite replay")
    held = [r for r in facts.get("mfg_receipts", []) if r["receipt_id"] == receipt]
    check(len(held) == 1 and held[0]["status"] == "HOLD" and held[0]["available_micros"] == 0,
          "approval alone cannot make held physical stock issuable")
source_paths = [BASE / ("free-erp-inventory-consumers-workflow-oct8-" + args.stage + suffix + "-source.json")
                for suffix in ("-before", "-after")]
sources = [json.loads(p.read_text(encoding="utf8"))["files"] for p in source_paths]
drift = [p for p in sorted(set(sources[0]) | set(sources[1])) if sources[0].get(p) != sources[1].get(p)]
check(not drift, "registered source inputs stable")
result = re.search(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; (\d+) filtered out;", log)
expected_passes = 521 if args.stage == "arrival-proof-full" else 10 if args.stage.startswith("arrival-proof") else 7 if args.stage.startswith("arrival") else 1
functional = result is not None and int(result[1]) == expected_passes
if args.stage == "arrival-proof-full":
    check(result is not None and int(result[2]) == 0, "complete registered suite without filtered scenarios")
original_confirmed = False
proof_counterexample = args.stage in {"arrival-proof-original", "arrival-proof-before-fix"}
if args.stage.startswith("arrival"):
    observations = re.findall(r"QA purchase_arrival tenant=([a-f0-9-]+) case=(arrival-[a-z-]+) status=(\d+) qualities=(\d+) quantity=(\d+) unit_prices=(\[[^\r\n]*\])", log)
    expected = {"arrival-missing-price":422, "arrival-forged-zero":409, "arrival-forged-price":409,
                "arrival-known-zero":200, "arrival-closed-source":409,
                "arrival-changed-command":409, "arrival-original-replay":200}
    if args.stage.startswith("arrival-proof"):
        expected.update({"arrival-legacy-replay":200, "arrival-legacy-mismatch":409, "arrival-missing-original-proof":409})
    check(len(observations) == len(expected) and {r[1] for r in observations} == set(expected), "all actual purchase arrival observations")
    original_confirmed = args.stage.startswith("arrival-original")
    for tenant, case, status, count, quantity, prices in observations:
        f = rows.get(tenant, {})
        q = f.get("shared_receipt_quality", [])
        check(len(q) == int(count) and sum(r["arrived_micros"] for r in q) == int(quantity), "arrival observation agrees with original rows " + case)
        check([r["unit_cost_cent"] for r in q] == json.loads(prices), "actual unit prices " + case)
        if original_confirmed:
            check(int(status) == 200 and len(q) == 1, "original admitted unsafe request " + case)
        else:
            required_status = 200 if proof_counterexample and case == "arrival-missing-original-proof" else expected[case]
            check(int(status) == required_status, "price and current revision admission " + case)
            if expected[case] != 200 and case not in {"arrival-changed-command", "arrival-legacy-mismatch", "arrival-missing-original-proof"}:
                check(not q and not f.get("shared_purchase_receipts") and not f.get("shared_stock_ledgers"), "refused request zero quantity effects " + case)
            else:
                check(len(q) == 1 and q[0]["arrived_micros"] == 1_000_000 and q[0]["unit_cost_cent"] == (0 if case == "arrival-known-zero" else 200), "original price and quantity preserved " + case)
        for quality_row in q:
            revisions = [r for r in f.get("shared_purchase_revisions", []) if r["po_id"] == quality_row["po_id"]]
            lines = [r for r in f.get("shared_purchase_lines", []) if r["revision_id"] in {r["revision_id"] for r in revisions}]
            check(bool(lines) and all(r["price_cent"] == (0 if case == "arrival-known-zero" else 200) for r in lines), "original accepted source price remains unchanged " + case)
        if case.startswith("arrival-legacy"):
            old = [r for r in f.get("command_receipts", []) if r["request_id"] == "QA-legacy-arrival"]
            check(len(old) == 1 and old[0]["payload_hash"] == "legacy-arrival-v1" and len(q)==1
                  and old[0]["effect_id"] == q[0]["quality_id"], "original weak hash and original result retained " + case)
        if case == "arrival-missing-original-proof":
            commands = {r["request_id"]:r for r in f.get("command_receipts", [])}
            valid, unproven = commands.get("QA-valid-arrival", {}), commands.get("QA-unproven-arrival", {})
            check(bool(valid) and bool(unproven) and valid["payload_hash"] == unproven["payload_hash"]
                  and valid["effect_id"] != unproven["effect_id"] and unproven["effect_id"] not in {r["quality_id"] for r in q},
                  "same v2 intent hash does not prove this other result exists")
            check(not any(r["request_id"] == "QA-unproven-arrival" for r in f.get("shared_stock_ledgers", [])),
                  "no manufactured stock effect for unproven result")
    if original_confirmed:
        check("test result: FAILED. 2 passed; 5 failed; 0 ignored;" in log, "five actual unsafe original counterexamples")
    if proof_counterexample:
        check("test result: FAILED. 9 passed; 1 failed; 0 ignored;" in log, "actual matching-hash missing-effect counterexample")
value = {"utc": datetime.now(timezone.utc).isoformat(),
         "status": ("CONFIRMED_UNSAFE" if original_confirmed or proof_counterexample else "PASS") if (functional or original_confirmed or proof_counterexample) and not failures else "PARTIAL",
         "functional_pass": functional, "checks": checks, "failures": failures,
         "retained_observed_enterprises": len(rows),
         "previous_enterprises": len(before), "protected_tables": 80 if args.stage.startswith("arrival") else 78,
         "changed_previous_enterprises": changed, "source_inputs": len(sources[0]),
         "source_drift": drift, "before_sha256": before_sha, "after_sha256": after_sha,
         "scope": ("ordinary-password purchase arrival price/current revision/original-result guards; QA only, not full source binding, write IAM, native/Web/public deployment or Release qualification"
                   if args.stage.startswith("arrival") else "registered shared workflow, actual rejected output, frozen inspection proof and previous enterprise protection; no public/native/Web/Release qualification")}
exclusive(report, value)
print(json.dumps({k: value[k] for k in ("status", "functional_pass", "checks", "failures", "source_drift")}))
raise SystemExit(0 if (functional or original_confirmed or proof_counterexample) and not failures else 1)
