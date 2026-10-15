"""Independently verify the four registered legacy inspection HTTP entry points."""
from datetime import datetime, timezone
import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-production-quality-source-"
parser = argparse.ArgumentParser()
parser.add_argument("stage", nargs="?", default="direct")
args = parser.parse_args()
if not re.fullmatch(r"[a-z0-9-]+", args.stage):
    raise SystemExit("invalid immutable verification stage")
destination = QA / f"{PREFIX}{args.stage}-independent.json"
if destination.exists():
    raise SystemExit("refusing to replace historical direct evidence")
paths = {
    "before": QA / f"{PREFIX}full-retry-after.jsonl",
    "after": QA / f"{PREFIX}direct-after.jsonl",
    **{f"test_{case}": QA / f"{PREFIX}direct-{case}-test.log"
       for case in ["review", "template", "freeze", "reading"]},
}
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


cases = {}
for label in ["review", "template", "freeze", "reading"]:
    raw = paths[f"test_{label}"].read_bytes()
    log = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
    summaries = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
                          r"\d+ measured; (\d+) filtered out; finished in (\S+)", log)
    pairs = re.findall(r"QA quality_source_direct case=(\S+) tenant=(\S+) first_request=(\S+) "
                       r"first_status=(\d+) second_request=(\S+) second_status=(\d+)", log)
    require(len(summaries) == 1 and summaries[0][0:4] == ("ok", "1", "0", "0"),
            f"{label}: one actual passing registered case without ignored tests")
    require(len(pairs) == 1 and pairs[0][0] == label, f"{label}: actual two HTTP statuses captured")
    if len(pairs) == 1:
        case, tid, first, first_status, second, second_status = pairs[0]
        require(sorted([int(first_status), int(second_status)]) ==
                ([200, 200] if label == "template" else [200, 422]),
                f"{label}: both actual responses have the required outcome")
        cases[tid] = {"case": case, "requests": [
            {"request_id": first, "status": int(first_status)},
            {"request_id": second, "status": int(second_status)}],
            "terminal": summaries[0] if summaries else None}

old = {}
seen = set()
owned = {}
identity = ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438)
table_count = 0
for when in ["before", "after"]:
    with paths[when].open(encoding="utf8") as stream:
        header = json.loads(next(stream))
        require(tuple(header[key] for key in ["database", "role", "host", "port"]) == identity,
                f"{when}: protected database identity")
        for line in stream:
            row = json.loads(line)
            tid = row["tenant_id"]
            table_count = len(row["facts"])
            fingerprint = hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False,
                                                    separators=(",", ":")).encode()).hexdigest()
            if when == "before":
                old[tid] = fingerprint
            else:
                seen.add(tid)
                if tid in old:
                    require(old[tid] == fingerprint, f"existing complete enterprise changed: {tid}")
                if tid in cases:
                    owned[tid] = row["facts"]
require(not set(old) - seen, "no existing enterprises removed")
require(len(cases) == 4 and set(cases) <= set(owned), "all four actual new scenarios independently read back")
require(not set(cases) & set(old), "all four scenario enterprises are new")

for tid, case in cases.items():
    label = case["case"]
    data = owned.get(tid, {})
    commands = {row["request_id"]: row for row in data.get("command_receipts", [])}
    actual = [commands.get(request["request_id"]) for request in case["requests"]
              if request["status"] == 200]
    require(len(actual) == (2 if label == "template" else 1) and all(
            row and row["status"] == "SUCCEEDED" for row in actual), f"{label}: actual original success rows")
    require(all(request["request_id"] not in commands for request in case["requests"]
                if request["status"] != 200), f"{label}: refused request has no command")
    templates = data.get("shared_inspection_templates", [])
    template_lines = data.get("shared_inspection_template_lines", [])
    if label == "template":
        require(len(templates) == 2 and sorted(row["version_no"] for row in templates) == [1, 2],
                "template: two distinct new requests create actual sequential versions 1 and 2")
        require(len(template_lines) == 2 and all(sum(line["template_id"] == template["template_id"]
                for line in template_lines) == 1 for template in templates),
                "template: each actual immutable version owns one complete line")
        require(len(actual) == 2 and actual[0] and actual[1]
                and {actual[0]["effect_id"], actual[1]["effect_id"]} ==
                {template["template_id"] for template in templates}
                and actual[0]["payload_hash"] == actual[1]["payload_hash"],
                "template: each same-content command cites its own actual original version")
        continue
    require(len(templates) == 1 and templates[0]["version_no"] == 1, f"{label}: one actual template version")
    require(len(template_lines) == 1, f"{label}: one actual original template line")
    freezes = data.get("shared_inspection_freezes", [])
    bindings = data.get("shared_inspection_source_bindings", [])
    orders = {row["work_order_id"]: row for row in data.get("mfg_work_orders", [])}
    require(len(freezes) == len(bindings) == 1, f"{label}: one actual freeze and typed source")
    if len(freezes) != 1 or len(bindings) != 1:
        continue
    frozen, bound = freezes[0], bindings[0]
    require(all(bound[key] == frozen[key] for key in ["tenant_id", "freeze_id", "template_id",
            "version_no", "source_kind", "source_id", "item_code", "batch_code"]),
            f"{label}: complete original frozen identity")
    owner = orders.get(bound["work_order_id"], {})
    require(bound["source_kind"] == "WORK_ORDER" and bound["source_id"] == bound["work_order_id"]
            and owner.get("product_code") == bound["item_code"], f"{label}: actual work order and product")
    lines = [{key: row[key] for key in ["item_code", "required", "min_micros", "max_micros", "sample_count"]}
             for row in data.get("shared_inspection_freeze_lines", [])]
    require(bound["line_count"] == 1 and lines == bound["frozen_lines"], f"{label}: complete original standards")
    original = commands.get(bound["request_id"], {})
    require(original.get("payload_hash") == bound["payload_hash"]
            and original.get("effect_id") == frozen["freeze_id"] and original.get("status") == "SUCCEEDED",
            f"{label}: actual original command owns the frozen copy")
    if label == "freeze":
        require(len(actual) == 1 and actual[0] and actual[0]["effect_id"] == frozen["freeze_id"],
                "freeze: only winning request owns the actual frozen effect")
    elif label == "reading":
        readings = data.get("mfg_quality_items", [])
        require(len(readings) == 1 and readings[0]["reading_micros"] == 0
                and readings[0]["reading_present"] and readings[0]["passed"]
                and readings[0]["freeze_id"] == frozen["freeze_id"]
                and readings[0]["source_id"] == owner.get("work_order_id"),
                "reading: actual zero measurement is present and linked to the true source")
        require(len(readings) == 1 and actual and actual[0]
                and actual[0]["effect_id"] == readings[0]["item_id"],
                "reading: winning command owns the actual reading effect")
    elif label == "review":
        approvals = data.get("mfg_quality_releases", [])
        proofs = data.get("mfg_quality_release_proofs", [])
        receipts = data.get("mfg_receipts", [])
        require(len(approvals) == len(proofs) == len(receipts) == 1,
                "review: one actual approval, proof and original receipt")
        if len(approvals) == len(proofs) == len(receipts) == 1:
            approval, proof, receipt = approvals[0], proofs[0], receipts[0]
            require(approval["result"] == "RELEASED" and approval["release_id"] == proof["release_id"]
                    and approval["receipt_id"] == proof["receipt_id"] == receipt["receipt_id"]
                    and approval["work_order_id"] == proof["work_order_id"] == receipt["work_order_id"],
                    "review: approval proof uses the actual receipt and parent work order")
            require(actual and actual[0] and actual[0]["effect_id"] == proof["release_id"]
                    and actual[0]["request_id"] == proof["request_id"]
                    and actual[0]["payload_hash"] == proof["payload_hash"],
                    "review: winning request owns the exact original proof")
            require(proof["snapshot"]["quality"]["required_item_count"] == 1
                    and len(proof["snapshot"]["quality"]["source_bindings"]) == 1
                    and proof["snapshot"]["quality"]["source_bindings"][0]["freeze_id"] == frozen["freeze_id"],
                    "review: required reading and original source included in actual quality proof")
            require(receipt["status"] == "HOLD" and receipt["available_micros"] == 0
                    and receipt["qty_micros"] == 1_000_000,
                    "review: actual receipt stays held and unavailable")
        balances = data.get("inventory_balances", [])
        require(len(balances) == 1 and balances[0]["on_hand_micros"] == 1_000_000
                and balances[0]["frozen_micros"] == 1_000_000
                and balances[0]["reserved_micros"] == 0
                and balances[0]["on_hand"] == balances[0]["frozen"] == 1,
                "review: authoritative inventory precision and whole-unit projection remain held")
        require(not data.get("mfg_receipt_release_commands") and not data.get("mfg_receipt_release_effects"),
                "review: no inventory-release effect was fabricated")

report = {
    "created_at": datetime.now(timezone.utc).isoformat(),
    "status": "FAIL" if errors else "PASS",
    "scope": "four registered HTTP entry points, two actual responses each and independent complete QA facts; not a 20-way overlap proof, real password, benchmark or hosted UI acceptance",
    "assertions": checks,
    "protected_existing_enterprises": len(old),
    "complete_selected_tables": table_count,
    "cases": cases,
    "errors": errors,
    "artifacts": {name: {"path": path.relative_to(ROOT).as_posix(), "bytes": path.stat().st_size,
                         "sha256": digest(path)} for name, path in paths.items()},
}
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "cases": len(cases), "assertions": checks,
                  "old_enterprises": len(old), "errors": errors}, ensure_ascii=False))
raise SystemExit(bool(errors))
