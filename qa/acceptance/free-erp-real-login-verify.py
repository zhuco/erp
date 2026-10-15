"""Independent role/session/stock facts; preserve every selected older QA row."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-real-login-"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--mode", choices=["original", "repaired"], required=True)
parser.add_argument("--before", required=True)
parser.add_argument("--after", required=True)
parser.add_argument("--log", required=True)
parser.add_argument("--source-before", required=True)
parser.add_argument("--source-after", required=True)
parser.add_argument("--legacy-fixture")
parser.add_argument("--full", action="store_true")
parser.add_argument("--expected-regression-failures", type=int, default=0)
args = parser.parse_args()
target = BASE / (PREFIX + args.stage + "-independent.json")
if not re.fullmatch(r"[a-z0-9-]+", args.stage) or target.exists():
    raise SystemExit("invalid or existing evidence")
log_path = ROOT / args.log
log = log_path.read_text(encoding="utf-8-sig")
columns_path = BASE / (PREFIX + "timestamp-catalog.json")
columns = json.loads(columns_path.read_text(encoding="utf-8-sig"))
fixtures = re.findall(r"QA real_login_fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+) case=([^ ]+) session=([a-f0-9-]+)", log)
provenance = re.findall(r"QA inventory_command_v2_provenance tenant=([a-f0-9-]+) request_id=([^ ]+) original_session=([a-f0-9-]+) attempted_session=([a-f0-9-]+) db_guard=true unchanged=true", log)
owned = {row[0] for row in fixtures}
legacy = json.loads((ROOT / args.legacy_fixture).read_text(encoding="utf-8-sig")) if args.legacy_fixture else None
errors, checks = [], 0


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)


def canonical(table, values):
    result = []
    for original in values:
        row = dict(original)
        for key in columns.get(table, []):
            if row.get(key) is not None:
                row[key] = datetime.fromisoformat(row[key]).astimezone(timezone.utc).isoformat(timespec="microseconds")
        result.append(row)
    return sorted(result, key=lambda row: json.dumps(row, sort_keys=True, ensure_ascii=False))


def signature(row):
    normalized = {"tenant_id": row["tenant_id"], "name": row["name"],
                  "facts": {table: canonical(table, values) for table, values in row["facts"].items()}}
    return hashlib.sha256(json.dumps(normalized, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()


def capture(path, retain):
    values, current, digest, metadata = {}, {}, hashlib.sha256(), None
    with (ROOT / path).open("rb") as stream:
        for raw in stream:
            digest.update(raw)
            if not raw.strip():
                continue
            row = json.loads(raw)
            if "database" in row:
                metadata = row
                continue
            tenant = row["tenant_id"]
            check(tenant not in values, "unique enterprise " + tenant)
            check(len(row["facts"]) == 69, "all 69 selected tables " + tenant)
            values[tenant] = signature(row)
            if tenant in retain:
                current[tenant] = row
    check(metadata is not None and metadata.get("database") == "erp_shared_contract_test"
          and metadata.get("role") == "erp_grid_test" and metadata.get("host") == "127.0.0.1"
          and metadata.get("port") == 15438 and metadata.get("selected_tables") == 69,
          "protected QA identity " + path)
    return values, current, digest.hexdigest()


retain = owned | {row[0] for row in provenance} | ({legacy["tenant_id"]} if legacy else set())
before, old_current, before_sha = capture(args.before, retain)
after, current, after_sha = capture(args.after, retain)
changed_older = []
for tenant, old_signature in before.items():
    if legacy and tenant == legacy["tenant_id"]:
        old_facts, new_facts = old_current[tenant]["facts"], current.get(tenant, {}).get("facts", {})
        for table, old_rows in old_facts.items():
            new_rows = canonical(table, new_facts.get(table, []))
            originals = canonical(table, old_rows)
            if table == "auth_sessions":
                check(all(row in new_rows for row in originals), "legacy original session rows unchanged")
                added = [row for row in new_rows if row not in originals]
                check(len(added) == 1 and added[0]["client_label"] == "QA-restored-v1",
                      "one explicitly requested fresh legacy login session")
            else:
                check(new_rows == originals, "legacy all original columns unchanged " + table)
        continue
    if after.get(tenant) != old_signature:
        changed_older.append(tenant)
    check(after.get(tenant) == old_signature, "older enterprise all columns unchanged " + tenant)

check(len(fixtures) == len(owned) == 3, "three fresh real-password cases executed")
expected_cases = {"cross-device-original", "same-device-ceiling", "password-new-session"}
check({row[4] for row in fixtures} == expected_cases, "all three distinct boundary cases")
observed = {}
legacy_output = None
for tenant, actor, warehouse, sku, case, session in fixtures:
    check(tenant not in before and tenant in current, case + " fresh owned enterprise")
    facts = current.get(tenant, {}).get("facts", {})
    users = facts.get("users", [])
    check(len(users) == 1 and users[0]["user_id"] == actor and users[0]["status"] == "ACTIVE",
          case + " one actual active business user")
    check(len(users) == 1 and users[0]["password_hash"].startswith("$argon2id$v=19$"),
          case + " genuine salted password hash")
    check(any(row["perm_code"] == "WMS.INVENTORY.WRITE" for row in facts.get("role_permissions", [])),
          case + " actual canonical database write permission")
    sessions = facts.get("auth_sessions", [])
    check(all(row["user_id"] == actor and row["tenant_id"] == tenant for row in sessions), case + " actual session ownership")
    balances = [row for row in facts.get("inventory_balances", []) if row["warehouse_id"] == warehouse and row["sku_id"] == sku]
    check(len(balances) == 1, case + " exact authoritative inventory key")
    hand = balances[0]["on_hand_micros"] if balances else None
    receipts = facts.get("command_receipts", [])
    if case == "cross-device-original":
        matches = re.findall(r"QA real_login_cross_device tenant=" + tenant + r" request_id=([^ ]+) session_a=([^ ]+) session_b=([^ ]+) hand=(\d+) receipts=(\d+) original_effects=(\d+) original_session=([^ ]+) retry_status=(\d+)", log)
        check(len(matches) == 1, "independent cross-device observation logged")
        if not matches:
            continue
        request, a, b, logged_hand, count, effects, original_session, status = matches[0]
        originals = [row for row in receipts if row["request_id"] == request]
        ledgers = [row for row in facts.get("inventory_ledgers", []) if row.get("ref_id") == request]
        check(a != b and a == session and original_session == a, "separate sessions retain original submitter")
        check(hand == int(logged_hand) == 6_900_000 and len(receipts) == int(count) == 2 and len(ledgers) == int(effects) == 1,
              "two real commands and exactly one original effect")
        check(len(originals) == 1 and originals[0]["original_result"]["input"]["session_id"] == a,
              "original stored session provenance")
        check(int(status) == (409 if args.mode == "original" else 200), "cross-device actual HTTP outcome")
        observed[case] = {"retry_status": int(status), "on_hand_micros": hand, "receipt_count": len(receipts), "original_effects": len(ledgers)}
        if args.mode == "original" and originals:
            original = originals[0]
            check(original["original_result"]["version"] == 1, "actual pre-repair v1 snapshot")
            legacy_output = {"tenant_id": tenant, "actor_user_id": actor, "warehouse_id": warehouse, "sku_id": sku,
                             "request_id": request, "email": users[0]["email"], "payload_hash": original["payload_hash"],
                             "original_result": original["original_result"], "source_log": args.log}
    elif case == "same-device-ceiling":
        check(len(sessions) == 8 and all(row["status"] == "ACTIVE" for row in sessions), "actual ceiling remains eight sessions")
        check(hand == 4_900_000 and not receipts, "session admission has no business inventory effect")
        reused = re.findall(r"QA real_login_session_ceiling tenant=" + tenant + r" session=([^ ]+) active=(\d+)", log)
        check(len(reused) == 1 and reused[0] == (session, "8"), "same named client remains usable at the ceiling")
        observed[case] = {"session_count": len(sessions), "same_client_reused": bool(reused)}
    else:
        epoch = facts.get("tenants", [{}])[0].get("credentials_epoch")
        check(epoch == 1, "password and credential generation committed")
        check(len(sessions) == (1 if args.mode == "original" else 2), "password replacement session count")
        check(hand == (4_900_000 if args.mode == "original" else 5_900_000), "actual password replacement stock outcome")
        observations = re.findall(r"QA real_login_password_epoch tenant=" + tenant + r" old_session=([^ ]+) new_session=([^ ]+) tenant_epoch=(\d+) session_epoch=(\d+) hand=(\d+) status=(\d+)", log)
        check(len(observations) == 1, "password generation observation logged")
        if observations:
            old, new, current_epoch, session_epoch, logged_hand, status = observations[0]
            check((old == new) if args.mode == "original" else (old != new), "password replacement current session identity")
            check(int(current_epoch) == 1 and int(session_epoch) == (0 if args.mode == "original" else 1), "actual session credential generation")
            check(int(status) == (401 if args.mode == "original" else 200) and int(logged_hand) == hand, "actual password replacement HTTP outcome")
            observed[case] = {"tenant_epoch": int(current_epoch), "session_epoch": int(session_epoch), "status": int(status), "on_hand_micros": hand}

failed_names = sorted(set(re.findall(r"^test ([^ ]+) \.\.\. FAILED$", log, re.M))
                      | set(re.findall(r"^    ([a-zA-Z0-9_:]+)$", log, re.M)))
core_names = {
    "account_login::real_login_after_password_change_uses_current_epoch_session",
    "account_login::real_login_named_device_reuses_session_at_ceiling",
    "account_login::real_login_two_devices_replay_original_inventory_effect",
    "inventory_command_edges::inventory_command_v2_original_session_is_immutable",
}
if args.full:
    check(args.mode == "repaired" and not legacy, "full regression has self-contained real-login fixtures")
    check(len(failed_names) == args.expected_regression_failures and not (set(failed_names) & core_names),
          "all declared failures explicit and outside the four current core cases")
    expression = r"test result: " + (r"FAILED\." if failed_names else r"ok\.")
    expression += r" \d+ passed; " + str(args.expected_regression_failures) + r" failed; 0 ignored; 0 measured; 0 filtered out;"
    check(re.search(expression, log) is not None, "complete registered regression, no ignores or filters")
    for name in core_names:
        check(re.search(r"^test " + re.escape(name) + r" \.\.\. ok$", log, re.M) is not None, "actual core success " + name)
    normal = re.findall(r"QA (?:inventory command fixture|location command fixture) tenant=([a-f0-9-]+) actor=([a-f0-9-]+)", log)
    password_logins = set(re.findall(r"QA real_login_password tenant=([a-f0-9-]+) actor=([a-f0-9-]+)", log))
    check(bool(normal) and all((tenant, actor) in password_logins for tenant, actor in normal),
          "all migrated normal stock fixtures reached real password login")
    check(len(provenance) == 1, "one actual immutable v2 provenance case")
    for tenant, request, original_session, attempted_session in provenance:
        facts = current.get(tenant, {}).get("facts", {})
        check(tenant not in before and bool(facts), "fresh owned provenance enterprise")
        check(original_session != attempted_session, "provenance uses two actual sessions")
        sessions = facts.get("auth_sessions", [])
        check({original_session, attempted_session} <= {row["session_id"] for row in sessions},
              "both provenance sessions exist independently")
        originals = [row for row in facts.get("command_receipts", []) if row["request_id"] == request]
        check(len(originals) == 1, "one immutable original provenance receipt")
        if originals:
            snapshot = originals[0]["original_result"]
            check(snapshot["version"] == 2 and snapshot["input"]["session_id"] == original_session,
                  "v2 snapshot preserves original submitter")
            operations = [row for row in facts.get("operation_logs", []) if row["log_id"] == snapshot["operation_log_id"]]
            audits = [row for row in facts.get("audit_logs", []) if row["audit_id"] == snapshot["audit_id"]]
            check(len(operations) == 1 and operations[0]["metadata"]["submitter_session_id"] == original_session,
                  "independent operation log anchors original session")
            check(len(audits) == 1 and audits[0]["after_json"]["submitter_session_id"] == original_session,
                  "independent audit log anchors original session")
            ledgers = [row for row in facts.get("inventory_ledgers", []) if row.get("ref_id") == request]
            check(len(ledgers) == 1, "provenance replays preserve one actual inventory effect")
else:
    check(re.search(r"test result: " + (r"FAILED\. 1 passed; 2 failed; 0 ignored;" if args.mode == "original" else r"ok\. 3 passed; 0 failed; 0 ignored;"), log) is not None,
          "three actual cases, no ignored tests, expected phase result")
if args.mode == "original":
    failed = set(re.findall(r"^test (account_login::[^ ]+) \.\.\. FAILED$", log, re.M))
    check(failed == {"account_login::real_login_after_password_change_uses_current_epoch_session",
                     "account_login::real_login_two_devices_replay_original_inventory_effect"},
          "exact two demonstrated regressions; the session ceiling case passed")
source_paths = [ROOT / args.source_before, ROOT / args.source_after]
source_before, source_after = [json.loads(path.read_text(encoding="utf-8-sig"))["files"] for path in source_paths]
drift = [name for name in sorted(set(source_before) | set(source_after)) if source_before.get(name) != source_after.get(name)]
if legacy:
    check("QA real_login_legacy_original tenant=" + legacy["tenant_id"] in log, "real preserved v1 replay executed")
status = "FAIL" if errors else ("PARTIAL" if drift or (args.full and failed_names) else "PASS")
report = {"utc": datetime.now(timezone.utc).isoformat(), "status": status, "mode": args.mode,
          "scope": "Real password HTTP handlers plus PostgreSQL; no live desktop/Web, network, complete release or performance qualification",
          "checks": checks, "protected_tables": 69, "older_enterprises": len(before), "older_enterprises_changed": changed_older,
          "observations": observed, "errors": errors, "source_input_drift": drift,
          "full_regression": args.full, "whole_regression_status": "FAILED" if failed_names else "PASSED",
          "regression_failures": failed_names,
          "before_sha256": before_sha, "after_sha256": after_sha, "log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest(),
          "source_before_sha256": hashlib.sha256(source_paths[0].read_bytes()).hexdigest(),
          "source_after_sha256": hashlib.sha256(source_paths[1].read_bytes()).hexdigest(),
          "timestamp_catalog_sha256": hashlib.sha256(columns_path.read_bytes()).hexdigest()}
with target.open("x", encoding="utf8") as output:
    output.write(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
if legacy_output:
    legacy_target = BASE / (PREFIX + "original-legacy-fixture.json")
    if legacy_target.exists():
        if json.loads(legacy_target.read_text(encoding="utf-8-sig")) != legacy_output:
            raise SystemExit("historical fixture differs; never replace its evidence")
    else:
        with legacy_target.open("x", encoding="utf8") as output:
            output.write(json.dumps(legacy_output, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({key: report[key] for key in ["status", "checks", "older_enterprises", "observations", "errors", "source_input_drift"]}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
