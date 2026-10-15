"""Preserve and independently confirm the actual unauthorized receipt effects."""
from datetime import datetime,timezone
import hashlib,json,re
from pathlib import Path
root=Path(__file__).resolve().parents[2]
base=root/"qa/acceptance"
target=base/"free-erp-receipt-authority-original-independent.json"
if target.exists():raise SystemExit("original evidence exists")
log_path=base/"free-erp-receipt-authority-original-test.log"
log=log_path.read_text(encoding="utf-8-sig")
fixtures=re.findall(r"QA trade_facts_fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) case=(authority-[^ ]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+)",log)
owned={x[0] for x in fixtures}
cols=json.loads((base/"free-erp-receipt-values-timestamp-catalog.json").read_text(encoding="utf8"))
checks,errors=0,[]
def check(ok,message):
    global checks
    checks+=1
    if not ok:errors.append(message)
def snapshot(path):
    sha=hashlib.sha256();signatures={};current={};identity=None
    with path.open("rb") as f:
        for raw in f:
            sha.update(raw)
            if not raw.strip():continue
            row=json.loads(raw)
            if "database" in row:identity=row;continue
            facts={}
            for table,rows in row["facts"].items():
                normalized=[]
                for item in rows:
                    item=dict(item)
                    for name in cols.get(table,[]):
                        if item.get(name) is not None:
                            item[name]=datetime.fromisoformat(item[name]).astimezone(timezone.utc).isoformat(timespec="microseconds")
                    normalized.append(item)
                facts[table]=sorted(normalized,key=lambda x:json.dumps(x,sort_keys=True,ensure_ascii=False))
            check(len(facts)==76,"all original selected columns "+row["tenant_id"])
            signatures[row["tenant_id"]]=hashlib.sha256(json.dumps({"tenant_id":row["tenant_id"],"name":row["name"],"facts":facts},
                sort_keys=True,ensure_ascii=False,separators=(",",":")).encode()).hexdigest()
            if row["tenant_id"] in owned:current[row["tenant_id"]]=row
    check(identity is not None and identity.get("database")=="erp_shared_contract_test" and identity.get("role")=="erp_grid_test"
        and identity.get("host")=="127.0.0.1" and identity.get("port")==15438 and identity.get("selected_tables")==76,"protected original snapshot")
    return signatures,current,sha.hexdigest()
before,_,before_sha=snapshot(base/"free-erp-receipt-values-authority-original-before.jsonl")
after,current,after_sha=snapshot(base/"free-erp-receipt-values-authority-original-after.jsonl")
changed=[]
for tenant in before:
    if before[tenant]!=after.get(tenant):changed.append(tenant)
    check(before[tenant]==after.get(tenant),"old enterprise unchanged "+tenant)
check(len(fixtures)==len(owned)==9,"nine fresh original authority fixtures")
check("test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 503 filtered out;" in log,"actual original normative failures")
observations={}
for tenant,actor,case,warehouse,sku in fixtures:
    check(tenant not in before and tenant in current,case+" fresh original fixture")
    facts=current.get(tenant,{}).get("facts",{})
    users=facts.get("users",[])
    check(len(users)==1 and users[0]["user_id"]==actor and users[0]["password_hash"].startswith("$argon2id$v=19$m=19456,t=2,p=1$"),case+" actual password user")
    sessions=facts.get("auth_sessions",[])
    check(len(sessions)==1 and sessions[0]["user_id"]==actor,case+" actual server session")
    commands=facts.get("shared_receipt_value_commands",[])
    count=1 if case.startswith("authority-wait-") else 3
    check(len(commands)==count and all(c["qty_micros"]==100_000 for c in commands),case+" independently present unauthorized effects")
    for c in commands:
        q=[q for q in facts.get("shared_receipt_quality",[]) if q["quality_id"]==c["quality_id"]]
        check(len(q)==1,case+" actual mutated original quality")
        stock=[r for r in facts.get("shared_stock_ledgers",[]) if r["ledger_id"]==c["stock_id"]]
        check(len(stock)==1 and stock[0]["request_id"]==c["request_id"] and stock[0]["warehouse_id"]==warehouse,
            case+" actual posted source despite authority failure")
    markers=re.findall(r"QA receipt_authority_result tenant="+tenant+r" case=([^ \r\n]+)([^\r\n]*)",log)
    check(len(markers)==1,case+" actual HTTP failure marker")
    if markers:
        detail=markers[0][1]
        if case.startswith("authority-wait-"):
            check("writer_observed=true" in detail and "status=200" in detail and "zero_effects=false" in detail,case+" waiting writer proceeded after committed revocation")
        else:
            check("statuses=[(200, false), (200, false), (200, false)]" in detail,case+" all original operations incorrectly succeeded")
    observations[case]={"actual_commands":len(commands),"marker":markers}
sa,sb=[json.loads((base/f"free-erp-inventory-consumers-receipt-authority-original-{n}-source.json").read_text(encoding="utf8"))["files"] for n in ["before","after"]]
drift=[n for n in sorted(set(sa)|set(sb)) if sa.get(n)!=sb.get(n)]
check(not drift,"original source window unchanged")
report={"utc":datetime.now(timezone.utc).isoformat(),"status":"FAIL" if errors else "CONFIRMED_UNSAFE","checks":checks,
    "older_enterprises":len(before),"older_enterprises_changed":changed,"protected_tables":76,"source_input_count":len(sa),
    "source_input_drift":drift,"observations":observations,"errors":errors,"before_sha256":before_sha,
    "after_sha256":after_sha,"log_sha256":hashlib.sha256(log_path.read_bytes()).hexdigest()}
with target.open("x",encoding="utf8") as f:f.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"checks":checks,"older_enterprises":len(before),"errors":errors},ensure_ascii=False))
raise SystemExit(1 if errors else 0)
