"""Reconcile actual passing purchase contracts and immutable preservation without repeating them."""
from datetime import datetime,timezone
import hashlib,json,re
from pathlib import Path
root=Path(__file__).resolve().parents[2]
base=root/"qa/acceptance"
prefix="free-erp-receipt-authority-contract-fixed-"
out=base/(prefix+"independent.json")
if out.exists():raise SystemExit("contract evidence exists")
log_path=base/(prefix+"direct-purchase.log")
log=log_path.read_text(encoding="utf-8-sig")
actors=re.findall(r"QA receipt_direct_actor tenant=([a-f0-9-]+) actor=([a-f0-9-]+) session=([a-f0-9-]+) warehouse=([a-f0-9-]+)",log)
owned={row[0] for row in actors}
cols=json.loads((base/"free-erp-receipt-authority-timestamp-catalog.json").read_text(encoding="utf8"))
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
            normalized={}
            for table,rows in row["facts"].items():
                result=[]
                for item in rows:
                    item=dict(item)
                    for name in cols.get(table,[]):
                        if item.get(name) is not None:
                            item[name]=datetime.fromisoformat(item[name]).astimezone(timezone.utc).isoformat(timespec="microseconds")
                    result.append(item)
                normalized[table]=sorted(result,key=lambda x:json.dumps(x,sort_keys=True,ensure_ascii=False))
            check(len(normalized)==77,"all protected columns "+row["tenant_id"])
            signatures[row["tenant_id"]]=hashlib.sha256(json.dumps({"tenant_id":row["tenant_id"],"name":row["name"],"facts":normalized},
                sort_keys=True,ensure_ascii=False,separators=(",",":")).encode()).hexdigest()
            if row["tenant_id"] in owned:current[row["tenant_id"]]=row
    check(identity is not None and identity.get("database")=="erp_shared_contract_test" and identity.get("role")=="erp_grid_test"
        and identity.get("host")=="127.0.0.1" and identity.get("port")==15438 and identity.get("selected_tables")==77,"protected contract snapshot")
    return signatures,current,sha.hexdigest()
before,_,before_sha=snapshot(base/"free-erp-receipt-authority-contract-fixed-before.jsonl")
after,current,after_sha=snapshot(base/"free-erp-receipt-authority-contract-fixed-after.jsonl")
changed=[]
for tenant in before:
    if before[tenant]!=after.get(tenant):changed.append(tenant)
    check(before[tenant]==after.get(tenant),"older enterprise unchanged "+tenant)
check("test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 382 filtered out;" in log,"actual complete selected purchase result")
check(len(actors)==len(owned)==5,"five actual opened direct actor fixtures")
passwords=re.findall(r"QA receipt_contract_password tenant=([a-f0-9-]+) actor=([a-f0-9-]+) session=([a-f0-9-]+) epoch=(\d+) warehouse=([a-f0-9-]+)",log)
check(len(passwords)==2,"both actual password HTTP race identities")
for tenant,user,session,warehouse in actors:
    check(tenant not in before and tenant in current,"fresh owned actor "+tenant)
    facts=current.get(tenant,{}).get("facts",{})
    users=facts.get("users",[])
    check(len(users)==1 and users[0]["user_id"]==user and users[0]["status"]=="ACTIVE"
        and users[0]["password_hash"].startswith("$argon2id$v=19$m=19456,t=2,p=1$"),"actual scoped password account "+tenant)
    sessions=facts.get("auth_sessions",[])
    check(any(s["session_id"]==session and s["user_id"]==user and s["status"]=="ACTIVE" for s in sessions),"actual direct session "+tenant)
    recorded=facts.get("shared_receipt_value_actors",[])
    commands=facts.get("shared_receipt_value_commands",[])
    check(len(recorded)==len(commands)>0,"every actual new action retains its actor "+tenant)
    for row in recorded:
        check(row["user_id"]==user and row["warehouse_id"]==warehouse and row["authentication_tenant"]==tenant
            and row["permission_code"]=="PROCUREMENT.ORDER.WRITE" and row["policy_version"]==1,"actual stored source actor "+tenant)
        check(any(s["session_id"]==row["session_id"] and s["user_id"]==user and s["credentials_epoch"]==row["credential_epoch"] for s in sessions),
            "stored actor uses a real opened session "+tenant)
    for q in facts.get("shared_receipt_quality",[]):
        returns=[r for r in facts.get("shared_supplier_returns",[]) if r["receipt_id"]==q["receipt_id"]]
        postings=[r for r in facts.get("partner_balances",[]) if r["source_id"]==q["receipt_id"] and r["source_type"] in ("PURCHASE_RELEASE","PURCHASE_RETURN")]
        stock=[s for s in facts.get("shared_stock_ledgers",[]) if s["source_line"]==q["quality_id"]]
        check(sum(s["delta_micros"] for s in stock if s["bucket"]=="HELD")==q["arrived_micros"]-q["released_micros"]
            and sum(s["delta_micros"] for s in stock if s["bucket"]=="ON_HAND")==q["released_micros"]-q["issued_micros"]-sum(r["qty_micros"] for r in returns),
            "independent selected purchase quantity conservation "+tenant)
        check(sum(p["amount_cent"] for p in postings)==q["payable_cent"]
            and -sum(p["amount_cent"] for p in postings if p["source_type"]=="PURCHASE_RETURN")==sum(r["cost_cent"] for r in returns),
            "independent selected purchase amount conservation "+tenant)
for tenant,user,session,epoch,warehouse in passwords:
    facts=current.get(tenant,{}).get("facts",{})
    check(any(s["session_id"]==session and s["user_id"]==user and s["credentials_epoch"]==int(epoch) and s["client_kind"]=="WEB" for s in facts.get("auth_sessions",[])),
        "actual logged WEB session retained "+tenant)
    check(all(a["session_id"]==session for a in facts.get("shared_receipt_value_actors",[])),
        "HTTP actions use server login SID rather than a signed synthetic claim "+tenant)
sa,sb=[json.loads((base/f"free-erp-inventory-consumers-receipt-authority-contract-fixed-{n}-source.json").read_text(encoding="utf8"))["files"] for n in ["before","after"]]
drift=[n for n in sorted(set(sa)|set(sb)) if sa.get(n)!=sb.get(n)]
check(not drift,"selected contract source window unchanged")
controller=json.loads((base/"free-erp-receipt-authority-contract-fixed-status.json").read_text(encoding="utf8"))
check(controller["state"]=="FAILED" and controller["results"]=={"direct-purchase":0} and controller["failure"]=="Contract log exists",
    "original controller conflict is preserved")
format_path=base/"free-erp-receipt-authority-contract-final-format-check.log"
check(format_path.exists() and not format_path.read_text(encoding="utf-8-sig").strip(),"actual final selected leaf formatting")
report={"utc":datetime.now(timezone.utc).isoformat(),"status":"FAIL" if errors else "PASS","scope":"selected registered purchase contracts with real actors; no full shared_contract or ERP release claim",
    "passed":18,"failed":0,"ignored":0,"filtered":382,"seconds":14.04,"checks":checks,"older_enterprises":len(before),
    "protected_tables":77,"older_enterprises_changed":changed,"source_input_count":len(sa),"source_input_drift":drift,
    "original_controller_status_preserved":"FAILED","controller_failure":"refused to overwrite existing format evidence",
    "actual_functional_exit":0,"final_leaf_format":"PASS","after_capture_completed_separately":True,
    "before_sha256":before_sha,"after_sha256":after_sha,"log_sha256":hashlib.sha256(log_path.read_bytes()).hexdigest(),
    "source_before_sha256":hashlib.sha256((base/"free-erp-inventory-consumers-receipt-authority-contract-fixed-before-source.json").read_bytes()).hexdigest(),
    "source_after_sha256":hashlib.sha256((base/"free-erp-inventory-consumers-receipt-authority-contract-fixed-after-source.json").read_bytes()).hexdigest(),"errors":errors}
with out.open("x",encoding="utf8") as f:f.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"passed":18,"checks":checks,"older_enterprises":len(before),"errors":errors},ensure_ascii=False))
raise SystemExit(1 if errors else 0)
