"""Independent owned login facts, old-enterprise protection, and source windows."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-session-pool-"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--mode", choices=["original","repaired"], required=True)
parser.add_argument("--before", required=True)
parser.add_argument("--after", required=True)
parser.add_argument("--log", required=True)
parser.add_argument("--source-before", required=True)
parser.add_argument("--source-after", required=True)
parser.add_argument("--full", action="store_true")
parser.add_argument("--expected-regression-failures", type=int, default=0)
args = parser.parse_args()
target = BASE / (PREFIX + args.stage + "-independent.json")
if not re.fullmatch(r"[a-z0-9-]+", args.stage) or target.exists():
    raise SystemExit("invalid or existing evidence")
log_path = ROOT / args.log
log = log_path.read_text(encoding="utf-8-sig")
fixtures = re.findall(r"QA session_pool_fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) initial_session=([a-f0-9-]+) case=([^ ]+) max_connections=(\d+) tag=([^\r\n ]+)",log)
owned = {row[0] for row in fixtures}
catalog_path = BASE / "free-erp-real-login-timestamp-catalog.json"
catalog = json.loads(catalog_path.read_text(encoding="utf-8-sig"))
errors,checks = [],0


def check(ok,message):
    global checks
    checks += 1
    if not ok:
        errors.append(message)


def canonical(table,values):
    rows = []
    for original in values:
        row = dict(original)
        for name in catalog.get(table,[]):
            if row.get(name) is not None:
                row[name] = datetime.fromisoformat(row[name]).astimezone(timezone.utc).isoformat(timespec="microseconds")
        rows.append(row)
    return sorted(rows,key=lambda row:json.dumps(row,sort_keys=True,ensure_ascii=False))


def snapshot(path):
    sha = hashlib.sha256()
    signatures,retained,identity = {},{},None
    with (ROOT/path).open("rb") as stream:
        for raw in stream:
            sha.update(raw)
            if not raw.strip():
                continue
            row = json.loads(raw)
            if "database" in row:
                identity = row
                continue
            tenant = row["tenant_id"]
            check(tenant not in signatures,"unique enterprise "+tenant)
            check(len(row["facts"]) == 69,"all selected tables "+tenant)
            facts = {name:canonical(name,values) for name,values in row["facts"].items()}
            signatures[tenant] = hashlib.sha256(json.dumps({"tenant_id":tenant,"name":row["name"],"facts":facts},sort_keys=True,ensure_ascii=False,separators=(",",":")).encode()).hexdigest()
            if tenant in owned:
                retained[tenant] = row
    check(identity is not None and identity.get("database")=="erp_shared_contract_test"
          and identity.get("role")=="erp_grid_test" and identity.get("host")=="127.0.0.1"
          and identity.get("port")==15438 and identity.get("selected_tables")==69,
          "protected QA identity "+path)
    return signatures,retained,sha.hexdigest()


before,_,before_sha = snapshot(args.before)
after,current,after_sha = snapshot(args.after)
changed = [tenant for tenant in before if before[tenant] != after.get(tenant)]
for tenant in before:
    check(before[tenant] == after.get(tenant),"older enterprise all selected columns unchanged "+tenant)
expected = {"one-connection","two-concurrent","cancelled","password-changed-during-login","disabled-during-login"}
check(len(fixtures)==len(owned)==5,"all five fresh real-login fixtures")
check({row[3] for row in fixtures}==expected,"five distinct exact boundary cases")
observations,hashes = {},{}
for tenant,actor,initial,case,limit,tag in fixtures:
    check(tenant not in before and tenant in current,case+" fresh owned enterprise")
    facts = current.get(tenant,{}).get("facts",{})
    users = facts.get("users",[])
    check(len(users)==1 and users[0]["user_id"]==actor,case+" unique actual business user")
    if len(users)!=1:
        continue
    hashes[case] = users[0]["password_hash"]
    check(hashes[case].startswith("$argon2id$v=19$m=19456,t=2,p=1$"),case+" actual default Argon2 hash, not a low-cost timing fixture")
    check(users[0]["status"]==("DISABLED" if case=="disabled-during-login" else "ACTIVE"),case+" actual committed user status")
    check(any(row["perm_code"]=="WMS.INVENTORY.WRITE" for row in facts.get("role_permissions",[])),case+" actual database role permissions")
    check(any(row["user_id"]==actor for row in facts.get("user_roles",[])),case+" actual user role binding")
    tenants = facts.get("tenants",[])
    epoch = tenants[0]["credentials_epoch"] if len(tenants)==1 else None
    check(epoch==(1 if "during-login" in case else 0),case+" actual committed credential generation")
    matches = re.findall(r"QA session_pool_result tenant="+tenant+r" case="+case+r" statuses=\[([^\]]*)\] total=(\d+) active=(\d+) held_locks=(\d+)([^\r\n]*)",log)
    check(len(matches)==1,case+" exact actual HTTP/database observation")
    if len(matches)!=1:
        continue
    statuses,total,active,locks,detail = matches[0]
    statuses = [int(value) for value in statuses.split(",") if value]
    total,active,locks = int(total),int(active),int(locks)
    sessions = facts.get("auth_sessions",[])
    check(len(sessions)==total and all(row["tenant_id"]==tenant and row["user_id"]==actor for row in sessions),case+" independent session count and ownership")
    check(sum(row["status"]=="ACTIVE" and row["credentials_epoch"]==epoch and users[0]["status"]=="ACTIVE" for row in sessions)==active,case+" independent current-generation active count")
    check(initial in {row["session_id"] for row in sessions},case+" original login session preserved")
    check(limit==("1" if case=="one-connection" else "2") and tag=="qa_login_pool_"+tenant,case+" actual bounded owned pool")
    if case=="one-connection":
        check(statuses==([500] if args.mode=="original" else [200]),case+" actual outcome")
        check(total==active==(1 if args.mode=="original" else 2),case+" actual effects")
        if args.mode=="original":
            check("pool_timeout=true" in detail,"actual connection-acquire failure, not unrelated 500")
    elif case=="two-concurrent":
        check(sorted(statuses)==([200,500] if args.mode=="original" else [200,200]),case+" exact concurrent outcomes")
        check(total==active==(2 if args.mode=="original" else 3),case+" independent admitted sessions")
    elif case=="cancelled":
        check(not statuses and total==active==1,case+" cancellation has no session effect")
        check(locks==(1 if args.mode=="original" else 0),case+" independent live lock observation before closing owned pool")
        check("actual_wait=true" in detail and "cancelled=true" in detail,case+" real PostgreSQL wait and task cancellation")
        idle = re.search(r"idle=(\d+) size=(\d+)",detail)
        check(idle is not None and idle[1]==idle[2],case+" all owned connections returned to idle")
    else:
        check(statuses==([200] if args.mode=="original" else [401]),case+" actual post-commit outcome")
        check(total==(2 if args.mode=="original" else 1),case+" no unproven new session")
        check("actual_wait=true" in detail and "current_epoch=1" in detail,case+" actual credential transaction wait")
        check(("minted_current_epoch=true" if args.mode=="original" else "minted_current_epoch=false") in detail,case+" genuine server-signed generation observation")
        check(("sessions_unchanged=false" if args.mode=="original" else "sessions_unchanged=true") in detail,case+" all original session columns checked")
    check(locks==0 if case!="cancelled" else True,case+" no normal-path leaked advisory lock")
    allowed = {"tenants","users","roles","user_roles","role_permissions","auth_sessions"}
    check(all(not rows for name,rows in facts.items() if name not in allowed),case+" no inventory, command, order or other selected business effects")
    observations[case] = {"http_statuses":statuses,"total_sessions":total,"current_active_sessions":active,"held_locks_before_pool_close":locks,"pool_limit":int(limit),"actual_wait":"actual_wait=true" in detail}
check(len(set(value for case,value in hashes.items() if case!="password-changed-during-login"))==1,"four unchanged users retain the actual original salted QA hash")
check(hashes.get("password-changed-during-login") not in {value for case,value in hashes.items() if case!="password-changed-during-login"},"actual password replacement committed")
failed_names = sorted(set(re.findall(r"^test ([^ ]+) \.\.\. FAILED$",log,re.M))
                      | set(re.findall(r"^    ([a-zA-Z0-9_:]+)$",log,re.M)))
if args.full:
    word = "FAILED" if args.expected_regression_failures else "ok"
    check(args.mode=="repaired" and re.search(r"test result: "+word+r"\. \d+ passed; "+str(args.expected_regression_failures)+r" failed; 0 ignored; 0 measured; 0 filtered out;",log) is not None,"complete actual registered regression without ignores or filters")
    check(len(failed_names)==args.expected_regression_failures and not any(name.startswith("session_pool::") for name in failed_names),"all full failures explicitly counted and outside the five core cases")
else:
    check(re.search(r"test result: "+(r"FAILED\. 0 passed; 5 failed;" if args.mode=="original" else r"ok\. 5 passed; 0 failed;")+r" 0 ignored; 0 measured; \d+ filtered out;",log) is not None,"five actual cases, expected phase outcome")
paths = [ROOT/args.source_before,ROOT/args.source_after]
a,b = [json.loads(path.read_text(encoding="utf-8-sig"))["files"] for path in paths]
drift = [name for name in sorted(set(a)|set(b)) if a.get(name)!=b.get(name)]
report = {"utc":datetime.now(timezone.utc).isoformat(),"status":"FAIL" if errors else ("PARTIAL" if drift or (args.full and failed_names) else "PASS"),
          "scope":"Actual password HTTP handlers and PostgreSQL. Lifecycle changes use the shipped user/epoch repository transaction. No live desktop, browser, public hosting or Release performance.",
          "mode":args.mode,"full_regression":args.full,"checks":checks,"protected_tables":69,
          "whole_regression_status":"FAILED" if args.full and failed_names else ("PASSED" if args.full else "NOT_APPLICABLE"),
          "regression_failures":failed_names if args.full else [],
          "older_enterprises":len(before),"older_enterprises_changed":changed,"observations":observations,
          "errors":errors,"source_input_drift":drift,"before_sha256":before_sha,"after_sha256":after_sha,
          "log_sha256":hashlib.sha256(log_path.read_bytes()).hexdigest(),
          "source_before_sha256":hashlib.sha256(paths[0].read_bytes()).hexdigest(),
          "source_after_sha256":hashlib.sha256(paths[1].read_bytes()).hexdigest(),
          "timestamp_catalog_sha256":hashlib.sha256(catalog_path.read_bytes()).hexdigest()}
with target.open("x",encoding="utf8") as output:
    output.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
summary = {key:report[key] for key in ["status","checks","older_enterprises","observations","source_input_drift"]}
summary["error_count"] = len(errors)
summary["errors_preview"] = errors[:8]
print(json.dumps(summary,ensure_ascii=False))
raise SystemExit(1 if errors else 0)
