"""Independent current-authority outcomes, original business facts and 77-table protection."""
import argparse
from datetime import datetime,timezone
import hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"qa/acceptance"
PREFIX="free-erp-receipt-authority-"
p=argparse.ArgumentParser()
p.add_argument("stage")
for n in ("before","after","log","source-before","source-after"):
    p.add_argument("--"+n,required=True)
p.add_argument("--full",action="store_true")
a=p.parse_args()
target=BASE/(PREFIX+a.stage+"-independent.json")
if not re.fullmatch(r"[a-z0-9-]+",a.stage) or target.exists():
    raise SystemExit("invalid or existing evidence")
log_path=ROOT/a.log
log=log_path.read_text(encoding="utf-8-sig")
fixtures=re.findall(r"QA trade_facts_fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) case=(authority-[^ ]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+)",log)
owned={x[0] for x in fixtures}
catalog_path=BASE/(PREFIX+"timestamp-catalog.json")
cols=json.loads(catalog_path.read_text(encoding="utf8"))
checks,errors=0,[]
def check(ok,message):
    global checks
    checks+=1
    if not ok:errors.append(message)
def canonical(table,rows):
    result=[]
    for original in rows:
        row=dict(original)
        for name in cols.get(table,[]):
            if row.get(name) is not None:
                row[name]=datetime.fromisoformat(row[name]).astimezone(timezone.utc).isoformat(timespec="microseconds")
        result.append(row)
    return sorted(result,key=lambda row:json.dumps(row,sort_keys=True,ensure_ascii=False))
def snapshot(path):
    signatures,current,sha,identity={}, {},hashlib.sha256(),None
    with (ROOT/path).open("rb") as f:
        for raw in f:
            sha.update(raw)
            if not raw.strip():continue
            row=json.loads(raw)
            if "database" in row:
                identity=row;continue
            tenant=row["tenant_id"]
            check(tenant not in signatures,"unique enterprise "+tenant)
            check(len(row["facts"])==77,"all selected source columns "+tenant)
            facts={t:canonical(t,r) for t,r in row["facts"].items()}
            signatures[tenant]=hashlib.sha256(json.dumps({"tenant_id":tenant,"name":row["name"],"facts":facts},
                sort_keys=True,ensure_ascii=False,separators=(",",":")).encode()).hexdigest()
            if tenant in owned:current[tenant]=row
    check(identity is not None and identity.get("database")=="erp_shared_contract_test"
        and identity.get("role")=="erp_grid_test" and identity.get("host")=="127.0.0.1" and identity.get("port")==15438
        and identity.get("selected_tables")==77,"protected snapshot "+path)
    return signatures,current,sha.hexdigest()
before,_,before_sha=snapshot(a.before)
after,current,after_sha=snapshot(a.after)
changed=[]
for tenant in before:
    if before[tenant]!=after.get(tenant):changed.append(tenant)
    check(before[tenant]==after.get(tenant),"old enterprise 77 tables unchanged "+tenant)
expected={"authority-warehouse","authority-role","authority-source-warehouse","authority-two-devices",
    "authority-new-grant","authority-claims","authority-late-expiry"}|{"authority-wait-"+k for k in ["warehouse","role","session","epoch","disabled","expired"]}
check({x[2] for x in fixtures}==expected and len(fixtures)==len(owned)==13,"thirteen actual owned account fixtures")
results=re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; (\d+) filtered out;",log)
check(len(results)==1,"unique actual passing registered result")
if results:check(int(results[0][0])>=511 and results[0][1]=="0" if a.full else int(results[0][0])==8,"required actual case count")
observations={}
negative=expected-{"authority-two-devices","authority-new-grant"}
for tenant,actor,case,warehouse,sku in fixtures:
    check(tenant not in before and tenant in current,case+" fresh owned account")
    facts=current.get(tenant,{}).get("facts",{})
    users=facts.get("users",[])
    check(len(users)==1 and users[0]["user_id"]==actor,case+" one actual business user")
    check(len(users)==1 and users[0]["password_hash"].startswith("$argon2id$v=19$m=19456,t=2,p=1$"),case+" genuine default salted password")
    check(len(users)==1 and users[0]["status"]==("DISABLED" if case=="authority-wait-disabled" else "ACTIVE"),case+" actual user lifecycle")
    sessions=facts.get("auth_sessions",[])
    count=2 if case in ("authority-two-devices","authority-new-grant") else 1
    check(len(sessions)==count and all(s["user_id"]==actor and s["credentials_epoch"]==0 for s in sessions),case+" actual opened sessions")
    qualities=facts.get("shared_receipt_quality",[])
    actors=facts.get("shared_receipt_value_actors",[])
    commands=facts.get("shared_receipt_value_commands",[])
    returns=facts.get("shared_supplier_returns",[])
    postings=facts.get("partner_balances",[])
    stock=facts.get("shared_stock_ledgers",[])
    if case in negative:
        for table in ("shared_receipt_value_commands","shared_receipt_cost_lots","shared_receipt_cost_allocations","shared_receipt_value_actors","command_receipts","shared_supplier_returns"):
            check(not facts.get(table,[]),case+" independently zero business effects "+table)
        check(len(qualities)==(3 if case in ("authority-warehouse","authority-role","authority-source-warehouse") else 1),case+" actual original receipt count")
        for q in qualities:
            check((q["arrived_micros"],q["released_micros"],q["issued_micros"],q["payable_cent"])==(2_000_000,1_000_000,0,1),case+" original quality unchanged")
    else:
        check(len(qualities)==len(commands)==len(actors)==len(returns)==1,case+" one actual effect and retained actor")
        if actors and commands and qualities:
            record=actors[0];command=commands[0];q=qualities[0]
            check(record["request_id"]==command["request_id"] and record["user_id"]==actor
                and record["authentication_tenant"]==tenant and record["warehouse_id"]==warehouse
                and record["credential_epoch"]==0 and record["policy_version"]==1,
                case+" original source and verified actor snapshot")
            check(record["session_id"] in {s["session_id"] for s in sessions},case+" actual original opened session")
            check(command["qty_micros"]==100_000 and command["amount_cent"]==0 and command["result"]==0,
                case+" actual source return quantity and cent tail")
            if case=="authority-two-devices":
                check(any(s["session_id"]==record["session_id"] and s["status"]=="REVOKED" for s in sessions)
                    and any(s["session_id"]!=record["session_id"] and s["status"]=="ACTIVE" for s in sessions),
                    "successful original actor retained after legitimate session revocation")
            else:
                check(record["permission_code"]=="PROCUREMENT.ORDER.WRITE","current database grant recorded instead of stale token list")
    for q in qualities:
        original=[r for r in facts.get("shared_purchase_receipts",[]) if r["receipt_id"]==q["receipt_id"]]
        check(len(original)==1 and original[0]["qty_micros"]==q["arrived_micros"],case+" original receipt binding")
        r=[x for x in returns if x["receipt_id"]==q["receipt_id"]]
        ps=[x for x in postings if x["source_id"]==q["receipt_id"] and x["kind"]=="AP" and x["currency"]=="CNY" and x["partner_code"]==q["supplier_code"]]
        ss=[x for x in stock if x["source_line"]==q["quality_id"]]
        check(sum(x["delta_micros"] for x in ss if x["bucket"]=="HELD")==q["arrived_micros"]-q["released_micros"]
            and sum(x["delta_micros"] for x in ss if x["bucket"]=="ON_HAND")==q["released_micros"]-q["issued_micros"]-sum(x["qty_micros"] for x in r),
            case+" independent actual stock conservation")
        check(sum(x["amount_cent"] for x in ps)==q["payable_cent"]
            and -sum(x["amount_cent"] for x in ps if x["source_type"]=="PURCHASE_RETURN")==sum(x["cost_cent"] for x in r),
            case+" independent actual AP conservation")
    marker=re.findall(r"QA receipt_authority_result tenant="+tenant+r" case=([^ \r\n]+)([^\r\n]*)",log)
    check(len(marker)==1,case+" actual outcome marker")
    if marker:
        detail=marker[0][1]
        if case.startswith("authority-wait-"):
            check("writer_observed=true" in detail and "zero_effects=true" in detail,case+" actual specific waiting writer and zero writes")
            expected_status=404 if case.endswith("warehouse") else 403 if case.endswith("role") else 401
            check(f"status={expected_status}" in detail,case+" actual current authority denial")
        elif case in ("authority-warehouse","authority-role","authority-source-warehouse"):
            expect=403 if case=="authority-role" else 404
            check(f"statuses=[({expect}, true), ({expect}, true), ({expect}, true)]" in detail,case+" all three operations refuse without effects")
        elif case=="authority-late-expiry":
            check("delayed_actor_insert=true" in detail and "status=401" in detail and "zero_effects=true" in detail,"actual expiry observed after effect SQL")
        elif case=="authority-claims":
            check("missing_sid=401" in detail and "wrong_epoch=401" in detail and "admin_bypass=false" in detail,"admin claim is not a session/epoch authorization")
        elif case=="authority-two-devices":
            check("original_actor_preserved=true" in detail and "zero_replay_writes=true" in detail,"different real session replay preserves original result/actor")
        elif case=="authority-new-grant":
            check("status=200" in detail and "old_claim_lacks_grant=true" in detail,"current grant takes effect for existing valid token")
    observations[case]={"qualities":len(qualities),"commands":len(commands),"actors":len(actors),"sessions":len(sessions),"outcome":marker}
source_paths=[ROOT/a.source_before,ROOT/a.source_after]
sa,sb=[json.loads(path.read_text(encoding="utf-8-sig"))["files"] for path in source_paths]
drift=[n for n in sorted(set(sa)|set(sb)) if sa.get(n)!=sb.get(n)]
report={"utc":datetime.now(timezone.utc).isoformat(),"status":"FAIL" if errors else "PARTIAL" if drift else "PASS",
    "scope":"Real password current receipt write authority, actual lock waits, original actor and exact business readback; no full ERP/native/Web/Release qualification",
    "full":a.full,"registered_result":results,"checks":checks,"protected_tables":77,"older_enterprises":len(before),
    "older_enterprises_changed":changed,"source_input_count":len(sa),"source_input_drift":drift,"observations":observations,"errors":errors,
    "before_sha256":before_sha,"after_sha256":after_sha,"log_sha256":hashlib.sha256(log_path.read_bytes()).hexdigest(),
    "source_before_sha256":hashlib.sha256(source_paths[0].read_bytes()).hexdigest(),
    "source_after_sha256":hashlib.sha256(source_paths[1].read_bytes()).hexdigest(),
    "timestamp_catalog_sha256":hashlib.sha256(catalog_path.read_bytes()).hexdigest()}
with target.open("x",encoding="utf8") as f:f.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"checks":checks,"older_enterprises":len(before),"errors_preview":errors[:10],"error_count":len(errors),"source_input_drift":drift},ensure_ascii=False))
raise SystemExit(1 if errors else 0)
