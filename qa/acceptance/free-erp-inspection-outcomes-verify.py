"""Independent original/fixed measurement facts; complete older QA facts stay immutable."""
import argparse
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re
from uuid import UUID

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-inspection-outcomes-"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--before")
parser.add_argument("--after")
parser.add_argument("--test-stage")
parser.add_argument("--original", action="store_true")
parser.add_argument("--migration", action="store_true")
parser.add_argument("--migration-rollback", action="store_true")
args = parser.parse_args()
for value in [args.stage, args.before, args.after, args.test_stage]:
    if value and not re.fullmatch(r"[a-z0-9-]+", value):
        raise SystemExit("invalid immutable evidence stage")
output = QA / f"{PREFIX}{args.stage}-independent.json"
if output.exists():
    raise SystemExit("refusing to replace historical evidence")
paths = {key: QA / f"{PREFIX}{label}.jsonl" for key, label in [
    ("before", args.before or args.stage + "-before"),
    ("after", args.after or args.stage + "-after")
]}
errors = []
checks = 0

def require(ok, message):
    global checks
    checks += 1
    if not ok:
        errors.append(message)

def digest(path):
    sha = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            sha.update(block)
    return sha.hexdigest()

log = ""
fixtures = {}
http = []
competitions = []
faults = []
sql_guards = []
command_guards = []
summary = None
if not (args.migration or args.migration_rollback):
    paths["test"] = QA / f"{PREFIX}{args.test_stage or args.stage}-test.log"
    raw = paths["test"].read_bytes()
    log = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
    for tid, case, actor, order, receipt, freeze in re.findall(
            r"QA inspection_outcome_fixture tenant=(\S+) case=(\S+) actor=(\S+) work_order=(\S+) receipt=(\S+) freeze=(\S+)", log):
        fixtures[tid] = dict(case=case, actor=actor, order=order, receipt=receipt, freeze=freeze)
    for line_number, line in enumerate(log.splitlines()):
        match = re.search(r"QA production_release_http tenant=(\S+) path=(\S+) input=(.*) status=(\d+) response=(.*)", line)
        if match:
            http.append(dict(tenant=match[1], path=match[2], input=json.loads(match[3]), line_number=line_number,
                             status=int(match[4]), response=json.loads(match[5])))
        match = re.search(r"QA inspection_outcome_competition tenant=(\S+) case=(\S+) blocker=(\d+) chain=(.*) responses=(\d+)", line)
        if match:
            competitions.append(dict(tenant=match[1], case=match[2], blocker=int(match[3]), chain=json.loads(match[4]),
                                     responses=int(match[5]), line_number=line_number))
        match = re.search(r"QA inspection_outcome_actual_fault tenant=(\S+) table=(\S+) request=(\S+) marker=(\d+) called=(\w+) status=(\d+) response=(.*)", line)
        if match:
            faults.append(dict(tenant=match[1], table=match[2], request=match[3], marker=int(match[4]),
                               called=match[5], status=int(match[6]), response=json.loads(match[7])))
        match = re.search(r"QA inspection_outcome_sql_guard tenant=(\S+) statement=(.*) refused=(\w+)", line)
        if match:
            sql_guards.append(dict(tenant=match[1], statement=match[2], refused=match[3]))
        match = re.search(r"QA inspection_outcome_command_guard tenant=(\S+) case=(\S+) request=(\S+) item=(\S+) committed=(\w+)(.*)", line)
        if match:
            command_guards.append(dict(tenant=match[1], case=match[2], request=match[3], item=match[4],
                                       committed=match[5], diagnostic=match[6]))
    summaries = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out; finished in (\S+)", log)
    require(len(summaries) == 1, "one actual terminal test result required")
    if summaries:
        state, passed, failed, ignored, filtered, elapsed = summaries[0]
        summary = dict(status=state, passed=int(passed), failed=int(failed), ignored=int(ignored),
                       filtered=int(filtered), elapsed=elapsed)
        require(int(ignored) == 0, "ignored tests cannot pass acceptance")
        require(int(failed) == (3 if args.original else 0), "original three failures or fixed zero failures required")
        require(("QA controlled_test_exit=1" if args.original else "QA controlled_test_exit=0") in log,
                "controlled process terminal marker required")
    require(bool(fixtures), "new actual HTTP measurement fixtures required")

old = {}
seen = set()
owned = {}
headers = {}
table_count = 0
all_events = all_reviews = 0
for when in ["before", "after"]:
    with paths[when].open(encoding="utf8") as stream:
        headers[when] = json.loads(next(stream))
        require(tuple(headers[when][key] for key in ["database", "role", "host", "port"]) ==
                ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438), f"{when}: protected QA identity")
        for line in stream:
            row = json.loads(line)
            tid = row["tenant_id"]
            table_count = len(row["facts"])
            sha = hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False,
                                           separators=(",", ":")).encode()).hexdigest()
            if when == "before":
                old[tid] = sha
            else:
                seen.add(tid)
                if tid in old:
                    require(sha == old[tid], f"complete older facts changed: {tid}")
                if tid in fixtures:
                    owned[tid] = row["facts"]
                all_events += len(row["facts"]["shared_inspection_reading_events"])
                all_reviews += len(row["facts"]["shared_inspection_reading_reviews"])
require(table_count == 54, "all 54 selected tables and all their columns required")
require(not set(old) - seen, "no older enterprises removed")
require(set(fixtures) <= set(owned), "all measurement fixtures independently read back")
require(not set(old) & set(fixtures), "every fixture belongs to this fresh QA run")
if args.migration:
    require(all_events == 0 and all_reviews == 0, "no guessed history or review backfill")
    new_tables = {"shared_inspection_reading_events", "shared_inspection_reading_reviews"}
    require(not new_tables & {c["table"] for c in headers["before"]["catalog"]}, "new tables absent before actual migration")
    require(new_tables <= {c["table"] for c in headers["after"]["catalog"]}, "new tables present after actual migration")
if args.migration_rollback:
    new_tables = {"shared_inspection_reading_events", "shared_inspection_reading_reviews"}
    require(all_events == 0 and all_reviews == 0, "failed migration left no guessed history")
    for when in ["before", "after"]:
        require(not new_tables & {c["table"] for c in headers[when]["catalog"]}, f"{when}: failed new tables absent")
    paths["apply_log"] = QA / f"{PREFIX}migration-apply.log"
    raw = paths["apply_log"].read_bytes()
    diagnostic = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
    require("QA migration_exit=3" in diagnostic and 'no unique constraint matching given keys' in diagnostic,
            "actual first migration failure preserved")

for tid, meta in fixtures.items():
    facts = owned.get(tid, {})
    calls = [c for c in http if c["tenant"] == tid]
    events = facts.get("shared_inspection_reading_events", [])
    reviews = facts.get("shared_inspection_reading_reviews", [])
    items = {q["item_id"]: q for q in facts.get("mfg_quality_items", [])}
    commands = {q["request_id"]: q for q in facts.get("command_receipts", [])}
    freezes = {q["freeze_id"]: q for q in facts.get("shared_inspection_freezes", [])}
    lines = {(q["freeze_id"], q["item_code"]): q for q in facts.get("shared_inspection_freeze_lines", [])}
    successful = {c["input"].get("request_id") for c in calls if c["status"] == 200}
    for c in calls:
        rid = c["input"].get("request_id")
        if c["status"] != 200 and rid and rid not in successful:
            require(rid not in commands, f"{tid}: rejected command {rid} has no receipt")
    if args.original:
        require(not events and not reviews, f"{tid}: original code created no provenance")
        reading_calls = [c for c in calls if c["path"].endswith("/readings")]
        require(len(reading_calls) == 1, f"{tid}: original real measurement call preserved")
        if reading_calls:
            expected = 200 if meta["case"] == "outcome-reviewer-provenance" else 422
            require(reading_calls[0]["status"] == expected, f"{tid}: actual original failure mode")
    for e in events:
        item = items.get(e["item_id"], {})
        line = lines.get((e["freeze_id"], e["item_code"]), {})
        command = commands.get(e["request_id"], {})
        require(bool(item) and bool(line), f"{tid}: event owns actual frozen reading and line")
        for key in ["freeze_id", "item_code", "required", "min_micros", "max_micros"]:
            require(item.get(key) == (e.get(key) if key in ["freeze_id", "item_code"] else line.get(key)),
                    f"{tid}: immutable standard {key}")
        freeze = freezes.get(e["freeze_id"], {})
        for key in ["source_kind", "source_id", "batch_code"]:
            require(item.get(key) == freeze.get(key), f"{tid}: immutable actual source {key}")
        expected_verdict = "MISSING" if not item.get("reading_present") else (
            "PASSED" if line.get("min_micros", 1) <= item.get("reading_micros", 0) <= line.get("max_micros", -1) else "FAILED")
        expected_pass = not line.get("required") if not item.get("reading_present") else expected_verdict == "PASSED"
        require(e["verdict"] == expected_verdict and item.get("passed") == expected_pass,
                f"{tid}: measured outcome independently recomputed")
        require(command.get("status") == "SUCCEEDED" and command.get("effect_id") == e["item_id"]
                and command.get("payload_hash") == e["payload_hash"], f"{tid}: full original successful command")
        require(e["actor_user_id"] == meta["actor"] and e["actor_source"] == "USER", f"{tid}: authenticated submitting actor")
        require(e["snapshot"].get("actor_user_id") == meta["actor"]
                and e["snapshot"].get("verdict") == expected_verdict, f"{tid}: durable original result")
        if e["attempt_no"] == 1:
            require(e["previous_item_id"] is None and not e["review_required"], f"{tid}: original policy preserved")
        else:
            previous = next((p for p in events if p["item_id"] == e["previous_item_id"]), {})
            require(previous.get("attempt_no", -100) + 1 == e["attempt_no"]
                    and previous.get("freeze_id") == e["freeze_id"] and previous.get("item_code") == e["item_code"],
                    f"{tid}: append-only reinspection chain")
            require(e["review_required"] and bool(e["reason"].strip()), f"{tid}: reinspection requires reason and actual review")
        matching = [c for c in calls if c["status"] == 200 and c["input"].get("request_id") == e["request_id"]]
        require(bool(matching), f"{tid}: original measurement actual HTTP preserved")
        for c in matching:
            reading = c["input"].get("reading_qty")
            require(item.get("reading_present") == (reading is not None), f"{tid}: real measured presence")
            if reading is not None:
                require(item.get("reading_micros") == int(Decimal(reading) * 1_000_000), f"{tid}: precise original decimal")
            payload = [1, str(UUID(e["freeze_id"])), c["input"]["item_code"].strip(),
                       None if reading is None else int(Decimal(reading) * 1_000_000),
                       c["input"]["inspector"].strip(), c["input"]["reviewer"].strip()]
            if c["path"].endswith("/reinspections"):
                payload = [2, "REINSPECTION", *payload[1:], str(UUID(c["input"]["previous_item_id"])), c["input"]["reason"].strip()]
            canonical = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
            require(canonical == e["payload_hash"], f"{tid}: canonical full measurement payload independently recomputed")
            require(c["response"].get("reading") == e["snapshot"], f"{tid}: HTTP original snapshot and replay")
    for review in reviews:
        e = next((p for p in events if p["item_id"] == review["item_id"]), {})
        command = commands.get(review["request_id"], {})
        require(bool(e) and review["reviewer_user_id"] == meta["actor"], f"{tid}: actual reviewer account and original reading")
        require(command.get("status") == "SUCCEEDED" and command.get("effect_id") == review["review_id"]
                and command.get("payload_hash") == review["payload_hash"], f"{tid}: original review command")
        require(review["decision"] != "APPROVE" or e.get("verdict") == "PASSED", f"{tid}: an approval cannot change measured failure")
        matching = [c for c in calls if c["status"] == 200 and c["input"].get("request_id") == review["request_id"]]
        require(bool(matching), f"{tid}: original review has actual HTTP evidence")
        for c in matching:
            canonical = json.dumps([2, "READING_REVIEW", str(UUID(e["freeze_id"])), str(UUID(review["item_id"])),
                c["input"]["decision"].strip().upper(), c["input"]["reason"].strip()], ensure_ascii=False, separators=(",", ":"))
            require(canonical == review["payload_hash"], f"{tid}: canonical full review payload independently recomputed")
            require(c["response"].get("review") == review["snapshot"], f"{tid}: original review response and replay")
    if not args.original:
        for c in calls:
            if c["status"] == 200 and c["input"].get("request_id") and c["path"].endswith(("/readings", "/reinspections")):
                require(any(e["item_id"] == c["response"].get("effect_id") for e in events), f"{tid}: no successful unbound measurement")
        receipt = next((q for q in facts.get("mfg_receipts", []) if q["receipt_id"] == meta["receipt"]), {})
        released = any(c["status"] == 200 and c["path"].endswith("/release") for c in calls)
        require(receipt.get("status") == ("RELEASED" if released else "HOLD"), f"{tid}: true physical receipt state")
        require(receipt.get("available_micros") == (1_250_000 if released else 0), f"{tid}: exact original held quantity")
        for c in calls:
            if c["status"] == 200 and "/readings?" in c["path"]:
                result = c["response"]
                history = result.get("history", [])
                expected = sorted((e for e in events if e["freeze_id"] == result.get("freeze_id")
                    and e["item_code"] == result.get("item_code")), key=lambda e:e["attempt_no"])
                require(result.get("attempt_limit") == 20 and len(history) == len(expected), f"{tid}: complete bounded actual history")
                for row, e in zip(history, expected):
                    require(row.get("reading") == e["snapshot"] and row.get("request_id") == e["request_id"], f"{tid}: original ordered history")
                    review = next((r["snapshot"] for r in reviews if r["item_id"] == e["item_id"]), None)
                    require(row.get("review") == review, f"{tid}: actual review in read-only history")
        release_commands = facts.get("mfg_receipt_release_commands", [])
        require(len(release_commands) == int(released), f"{tid}: one original physical release")
        for release in release_commands:
            quality = release["snapshot"].get("quality", {})
            require(release["qty_micros"] == 1_250_000 and release["available_total_micros"] == 1_250_000,
                    f"{tid}: original physical available result")
            require({e["item_id"] for e in quality.get("reading_events", [])} == {e["item_id"] for e in events},
                    f"{tid}: release proof retains all measured events")
            require({r["review_id"] for r in quality.get("reading_reviews", [])} == {r["review_id"] for r in reviews},
                    f"{tid}: release proof retains actual reviews")
            all_readings = {q["item_id"]:q for q in quality.get("readings", [])}
            effective = quality.get("effective_readings", [])
            for e in events:
                require(all_readings.get(e["item_id"], {}).get("reading_micros") == items[e["item_id"]]["reading_micros"],
                        f"{tid}: release proof retains original failed values")
                if not any(n["freeze_id"] == e["freeze_id"] and n["item_code"] == e["item_code"]
                           and n["attempt_no"] > e["attempt_no"] for n in events):
                    selected = [q for q in effective if q["item_id"] == e["item_id"]]
                    require(len(selected) == 1 and selected[0]["passed"] and selected[0].get("review_allows_release") is not False,
                            f"{tid}: latest effective true passing result")
            effects = facts.get("mfg_receipt_release_effects", [])
            require(len(effects) == 2 and {q["quality_status"] for q in effects} == {"HOLD", "GOOD"}, f"{tid}: exact paired physical effects")

if not args.original and summary and summary["passed"] >= 27:
    require(len(competitions) == 4, "all four actual competition observations required")
    for competition in competitions:
        tid = competition["tenant"]
        graph = {row["pid"]:row["blocked_by"] for row in competition["chain"]}
        require(len(graph) == 20 and competition["responses"] == 20, f"{tid}: twenty unique real blocked requests and responses")
        def reaches_blocker(pid, visited):
            if pid == competition["blocker"]:
                return True
            if pid in visited:
                return False
            return any(reaches_blocker(parent, visited | {pid}) for parent in graph.get(pid, []))
        require(all(reaches_blocker(pid, set()) for pid in graph), f"{tid}: actual blocking graph reaches held owner")
        race_calls = [c for c in http if c["tenant"] == tid and c["line_number"] < competition["line_number"]
            and (c["input"].get("reason") == "Concurrent reinspection" or c["path"].endswith(("/reviews", "/release")))]
        require(len(race_calls) == 20, f"{tid}: every competing request response recorded")
        require(all(c["status"] in [200, 409, 422] for c in race_calls), f"{tid}: competition only completed or bounded business refusal")
    require(len(faults) == 3 and {f["table"] for f in faults} == {
        "shared_inspection_reading_events", "shared_inspection_reading_reviews", "audit_logs"}, "all three actual INSERT faults required")
    for fault in faults:
        tid = fault["tenant"]
        require(fault["marker"] == 1 and fault["called"] == "true" and fault["status"] == 500,
                f"{tid}: actual nontransactional fault hit counter and server error")
        require("QA_INSPECTION_OUTCOME_INSERT_FAILURE" not in json.dumps(fault["response"]), f"{tid}: internal database error not disclosed")
        calls = [c for c in http if c["tenant"] == tid and c["input"].get("request_id") == fault["request"]]
        require([c["status"] for c in calls] == [500, 200, 200], f"{tid}: actual failure, original ID retry and replay")
    require(len(sql_guards) == 7 and all(g["refused"] == "true" for g in sql_guards), "all seven actual immutable SQL mutations refused")
    require(len(command_guards) == 5 and {g["case"] for g in command_guards} == {
        "missing-command", "wrong-payload", "wrong-effect", "missing-snapshot-actor", "foreign-review-user"}, "five actual command and actor insertion guards required")
    for guard in command_guards:
        tid = guard["tenant"]
        facts = owned.get(tid, {})
        require(guard["committed"] == "false" and guard["request"] not in {
            c["request_id"] for c in facts.get("command_receipts", [])}, f"{tid}: actual rejected transaction has no receipt")
        if guard["case"] != "foreign-review-user":
            require(guard["item"] not in {c["item_id"] for c in facts.get("mfg_quality_items", [])}, f"{tid}: original rejected reading also rolled back")

report = dict(created_at=datetime.now(timezone.utc).isoformat(), status="FAIL" if errors else "PASS",
              scope="Independent actual HTTP, immutable reading/review facts and old QA preservation; no real login, UI or performance claim",
              tests=summary, protected_existing_enterprises=len(old), complete_selected_tables=table_count,
              fixture_count=len(fixtures), actual_http_count=len(http), independent_assertions=checks, errors=errors,
              actual_competition_observations=competitions, actual_insert_faults=faults,
              immutable_sql_guards=sql_guards, original_command_guards=command_guards,
              inputs={key: {"path":p.relative_to(ROOT).as_posix(), "sha256":digest(p)} for key,p in paths.items()},
              fixtures=fixtures)
output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({key:report[key] for key in ["status", "tests", "protected_existing_enterprises", "independent_assertions", "errors"]}))
raise SystemExit(bool(errors))
