"""Read complete protected facts and independently check actual frozen-source ownership."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-production-quality-source-"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--before")
parser.add_argument("--after")
parser.add_argument("--test-stage")
parser.add_argument("--original", action="store_true")
parser.add_argument("--build-failed", action="store_true")
args = parser.parse_args()
for value in [args.stage, args.before, args.after, args.test_stage]:
    if value and not re.fullmatch(r"[a-z0-9-]+", value):
        raise SystemExit("invalid immutable verification stage")
destination = QA / f"{PREFIX}{args.stage}-independent.json"
if destination.exists():
    raise SystemExit("refusing to replace historical evidence")
paths = {
    "before": QA / f"{PREFIX}{args.before or args.stage + '-before'}.jsonl",
    "after": QA / f"{PREFIX}{args.after or args.stage + '-after'}.jsonl",
    "test": QA / f"{PREFIX}{args.test_stage or args.stage}-test.log",
}
raw = paths["test"].read_bytes()
log = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
fixtures = {}
for tid, case, actor, warehouse, sku, order, receipt, report, inspection in re.findall(
        r"QA production_release_fixture tenant=(\S+) case=(\S+) actor=(\S+) warehouse=(\S+) sku=(\S+) work_order=(\S+) receipt=(\S+) report=(\S+) inspection=(\S+)", log):
    fixtures[tid] = dict(case=case, actor=actor, warehouse=warehouse, sku=sku, order=order,
                         receipt=receipt, report=report, inspection=inspection)
http = []
for line in log.splitlines():
    match = re.search(r"QA production_release_http tenant=(\S+) path=(\S+) input=(.*) status=(\d+) response=(.*)", line)
    if match:
        http.append(dict(tenant=match[1], path=match[2], input=json.loads(match[3]),
                         status=int(match[4]), response=json.loads(match[5])))
errors = []
checks = 0

def require(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)

def digest(path):
    sha = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha.update(block)
    return sha.hexdigest()

old = {}
seen = set()
owned = {}
headers = {}
table_count = 0
for when in ["before", "after"]:
    with paths[when].open(encoding="utf8") as stream:
        headers[when] = json.loads(next(stream))
        require(tuple(headers[when][key] for key in ["database", "role", "host", "port"]) ==
                ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), f"{when}: protected database identity")
        for line in stream:
            row = json.loads(line)
            table_count = len(row["facts"])
            tid = row["tenant_id"]
            fingerprint = hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False,
                                                    separators=(",", ":")).encode()).hexdigest()
            if when == "before":
                old[tid] = fingerprint
            else:
                seen.add(tid)
                if tid in old:
                    require(old[tid] == fingerprint, f"existing complete enterprise facts changed: {tid}")
                if tid in fixtures:
                    owned[tid] = row["facts"]
require(not set(old) - seen, f"existing enterprises removed: {sorted(set(old) - seen)}")
require(set(fixtures) <= set(owned), "all new fixtures independently read back")
require(not set(fixtures) & set(old), "every fixture enterprise is new")
summaries = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out; finished in (\S+)", log)
if args.build_failed:
    require(not summaries and "could not compile" in log, "build failed before tests; no fabricated terminal pass")
else:
    require(len(summaries) == 1, "one actually executed terminal summary")
terminal = None
if summaries:
    label, passed, failed, ignored, filtered, elapsed = summaries[-1]
    terminal = dict(label=label, passed=int(passed), failed=int(failed), ignored=int(ignored),
                    filtered=int(filtered), elapsed=elapsed)
    if args.original:
        require((terminal["passed"], terminal["failed"], terminal["ignored"]) == (0, 9, 0),
                "retain nine actually failed original counterexamples")
        require(not headers["before"]["inspection_bindings_present"] and not headers["after"]["inspection_bindings_present"],
                "original bound-source table was absent")
    else:
        require(terminal["passed"] >= 9 and terminal["failed"] == 0 and terminal["ignored"] == 0,
                "actual regressions pass without ignored cases")
        require(headers["before"]["inspection_bindings_present"] and headers["after"]["inspection_bindings_present"],
                "typed bound-source schema exists")

def minimal(line):
    return {key: line[key] for key in ["item_code", "required", "min_micros", "max_micros", "sample_count"]}

def original_value(value, key=""):
    # SQLx uses UTC while protected psql uses the database zone; compare timestamp instants.
    if isinstance(value, dict):
        return {name: original_value(item, name) for name, item in value.items()}
    if isinstance(value, list):
        return [original_value(item) for item in value]
    if key == "created_at" and isinstance(value, str):
        return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc).isoformat()
    return value

cases = []
for tid, fixture in fixtures.items():
    data = owned.get(tid, {})
    requests = [entry for entry in http if entry["tenant"] == tid]
    freezes = {row["freeze_id"]: row for row in data.get("shared_inspection_freezes", [])}
    bindings = {row["freeze_id"]: row for row in data.get("shared_inspection_source_bindings", [])}
    receipts = {row["receipt_id"]: row for row in data.get("mfg_receipts", [])}
    orders = {row["work_order_id"]: row for row in data.get("mfg_work_orders", [])}
    templates = {row["template_id"]: row for row in data.get("shared_inspection_templates", [])}
    commands = {row["request_id"]: row for row in data.get("command_receipts", [])}
    if not args.original:
        for freeze_id, binding in bindings.items():
            frozen = freezes.get(freeze_id)
            require(frozen is not None, f"{fixture['case']}: actual frozen parent exists")
            if frozen is None:
                continue
            require(all(binding[key] == frozen[key] for key in
                        ["tenant_id", "template_id", "version_no", "source_kind", "source_id", "item_code", "batch_code"]),
                    f"{fixture['case']}: complete frozen parent identity")
            line_copy = sorted([minimal(row) for row in data.get("shared_inspection_freeze_lines", [])
                                if row["freeze_id"] == freeze_id], key=lambda row: row["item_code"])
            require(1 <= binding["line_count"] <= 1000 and len(line_copy) == binding["line_count"]
                    and line_copy == binding["frozen_lines"], f"{fixture['case']}: complete original item copy")
            template = templates.get(binding["template_id"])
            require(template is not None and template["version_no"] == binding["version_no"],
                    f"{fixture['case']}: actual owned template version")
            command = commands.get(binding["request_id"])
            require(command is not None and command["payload_hash"] == binding["payload_hash"]
                    and command["effect_id"] == freeze_id and command["status"] == "SUCCEEDED",
                    f"{fixture['case']}: actual original command owns this copy")
            source = binding["source_id"]
            kind = binding["source_kind"]
            if kind == "WORK_ORDER":
                parent = orders.get(source)
                require(parent is not None and binding["work_order_id"] == source
                        and binding["receipt_id"] is None and parent["product_code"] == binding["item_code"],
                        f"{fixture['case']}: actual work order and product")
            elif kind == "PRODUCTION_RECEIPT":
                parent = receipts.get(source)
                owner = orders.get(binding["work_order_id"])
                require(parent is not None and owner is not None and binding["receipt_id"] == source
                        and parent["work_order_id"] == binding["work_order_id"]
                        and owner["product_code"] == binding["item_code"] and parent["batch_code"] == binding["batch_code"],
                        f"{fixture['case']}: actual receipt owner, product and batch")
            elif kind == "SALES_ORDER":
                revision = next((row for row in data.get("shared_sales_revisions", [])
                                 if row["revision_id"] == binding["sales_revision_id"]), None)
                require(revision is not None and revision["so_id"] == source and any(
                    line["revision_id"] == revision["revision_id"] and line["item_code"] == binding["item_code"]
                    and line["qty_micros"] > 0 for line in data.get("shared_sales_lines", [])),
                    f"{fixture['case']}: actual sales revision and positive item")
            else:
                require(False, f"{fixture['case']}: unsupported bound source")
        for index, request in enumerate(requests):
            if request["path"] == "quality/inspection-freezes" and request["status"] == 200:
                require(request["response"].get("effect_id") in bindings, f"{fixture['case']}: successful HTTP has actual binding")
            if request["status"] in [403, 404, 409, 422]:
                request_id = request["input"].get("request_id")
                prior = [entry for entry in requests[:index] if entry["status"] == 200
                         and entry["input"].get("request_id") == request_id]
                # This registered scenario deliberately inserts an old successful
                # command without an original result before the HTTP request.
                # Its actual assertion compares complete facts before/after the
                # 409; independently confirm its exact identity and no invented
                # release result rather than treating that old row as a new write.
                legacy_original = (fixture["case"] == "unknown-original-result"
                                   and request["status"] == 409
                                   and request["path"].endswith("/release")
                                   and request["response"].get("error", {}).get("code")
                                   == "production_release_reconciliation_required")
                if legacy_original:
                    command = commands.get(request_id, {})
                    require(command.get("status") == "SUCCEEDED"
                            and command.get("effect_id") == fixture["receipt"]
                            and command.get("payload_hash") == json.dumps(
                                [1, fixture["order"], fixture["receipt"]], separators=(",", ":")),
                            "unknown original: exact deliberately pre-existing command retained")
                    require(not data.get("mfg_receipt_release_commands")
                            and not data.get("mfg_receipt_release_effects")
                            and not data.get("mfg_quality_release_proofs"),
                            "unknown original: no invented original release or quality proof")
                    receipt = receipts.get(fixture["receipt"], {})
                    require(receipt.get("status") == "HOLD" and receipt.get("available_micros") == 0,
                            "unknown original: actual receipt remains unavailable")
                    require(not any(row.get("action") == "MFG_RECEIPT_RELEASE" for row in
                                    data.get("operation_logs", []) + data.get("audit_logs", [])),
                            "unknown original: no fabricated release operation or audit")
                require(request_id not in commands or bool(prior) or legacy_original,
                        f"{fixture['case']}: rejection creates no new command; an already successful original may remain")
        for proof in data.get("mfg_receipt_release_commands", []) + data.get("mfg_quality_release_proofs", []):
            quality = proof["snapshot"]["quality"]
            for bound in quality.get("source_bindings", []):
                require(original_value(bound) == original_value(bindings.get(bound["freeze_id"])),
                        f"{fixture['case']}: original quality snapshot cites actual immutable binding")
            for frozen in quality.get("frozen", []):
                require(original_value(frozen["freeze"]) == original_value(freezes.get(frozen["freeze"]["freeze_id"])),
                        f"{fixture['case']}: original quality snapshot cites actual frozen header")
        if fixture["case"] == "source-receipt-passed":
            proof = data.get("mfg_receipt_release_commands", [])
            require(len(proof) == 1 and proof[0]["snapshot"]["quality"]["required_item_count"] == 1
                    and len(proof[0]["snapshot"]["quality"]["source_bindings"]) == 1,
                    "receipt pass preserves actual required count and original bound source")
    cases.append(dict(tenant_id=tid, **fixture, binding_count=len(bindings), http=requests,
                      quality_proof_count=len(data.get("mfg_quality_release_proofs", []))))
expected = {"source-unknown-kind", "source-wrong-product", "source-missing-receipt", "source-receipt-required",
            "source-case-normalization", "source-receipt-passed", "source-empty-legacy-freeze",
            "source-immutable-header", "source-immutable-line"}
if not args.build_failed:
    require(expected <= {item["case"] for item in cases}, "all original nine scenarios actually executed")
report = dict(created_at=datetime.now(timezone.utc).isoformat(), status="FAIL" if errors else "PASS",
              scope="actual frozen-source correctness and complete QA readback, not benchmark, UI or hosted deployment",
              original_counterexamples=args.original, terminal_tests=terminal, assertions=checks,
              build_failed_before_tests=args.build_failed,
              snapshot_timestamp_comparison="created_at compares the exact UTC instant; all other fields exact; raw facts preserved",
              protected_existing_enterprises=len(old), complete_selected_tables=table_count,
              cases=cases, errors=errors, artifacts={name: dict(path=path.relative_to(ROOT).as_posix(),
                  bytes=path.stat().st_size, sha256=digest(path)) for name, path in paths.items()})
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "tests": terminal, "old_enterprises": len(old),
                  "cases": len(cases), "assertions": checks, "errors": errors}, ensure_ascii=False))
raise SystemExit(bool(errors))
