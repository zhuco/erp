"""Independent 65-table evidence for precise atomic material effects and real HTTP."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"qa/acceptance"
PREFIX="free-erp-atomic-effects-"
parser=argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--before",required=True)
parser.add_argument("--after",required=True)
parser.add_argument("--log",required=True)
parser.add_argument("--expected-regression-failures",type=int,default=0,
                    help="Keep the overall regression FAILED while independently checking the atomic cases")
args=parser.parse_args()
target=BASE/(PREFIX+args.stage+"-independent.json")
if target.exists() or not re.fullmatch(r"[a-z0-9-]+",args.stage): raise SystemExit("invalid or existing evidence")
log_path=ROOT/args.log
log=log_path.read_text(encoding="utf-8-sig")
fixtures=re.findall(r"QA atomic_effect_fixture tenant=([a-f0-9-]+) case=([^ ]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+) work_order=([a-f0-9-]+)",log)
owned={row[0] for row in fixtures}
errors,checks=[],0
catalog_path=BASE/(PREFIX+"repair-timestamp-catalog.json")
columns=json.loads(catalog_path.read_text(encoding="utf-8-sig"))
def check(condition,message):
    global checks
    checks+=1
    if not condition: errors.append(message)
def canonical(table,rows):
    result=[]
    for original in rows:
        row=dict(original)
        for col in columns.get(table,[]):
            if row.get(col) is not None:
                row[col]=datetime.fromisoformat(row[col]).astimezone(timezone.utc).isoformat(timespec="microseconds")
        result.append(row)
    return sorted(result,key=lambda row:json.dumps(row,sort_keys=True,ensure_ascii=False))
def capture(name,retain):
    values,current,digest={},{},hashlib.sha256()
    metadata=None
    with (ROOT/name).open("rb") as stream:
        for raw in stream:
            digest.update(raw)
            if not raw.strip(): continue
            row=json.loads(raw)
            if "database" in row: metadata=row; continue
            tenant=row["tenant_id"]
            check(tenant not in values,"unique snapshot enterprise "+tenant)
            check(len(row["facts"])==65,"complete snapshot selected tables "+tenant)
            values[tenant]=hashlib.sha256(raw.rstrip(b"\r\n")).hexdigest()
            if tenant in retain: current[tenant]=row
    check(metadata is not None and metadata.get("database")=="erp_shared_contract_test" and metadata.get("role")=="erp_grid_test"
          and metadata.get("host")=="127.0.0.1" and metadata.get("port")==15438 and metadata.get("selected_tables")==65,
          "protected QA identity "+name)
    return values,current,digest.hexdigest()
before,_,before_sha=capture(args.before,set())
after,current,after_sha=capture(args.after,owned)
for tenant,old in before.items(): check(after.get(tenant)==old,"existing enterprise all selected columns unchanged "+tenant)
check(len(owned)==22 and len(fixtures)==22,"twenty-two separate atomic cases executed")
finals={tenant:json.loads(data) for tenant,case,data in re.findall(r"QA atomic_effect_final tenant=([a-f0-9-]+) case=([^ ]+) facts=(.+)",log)}
for tenant,case,request,status,unchanged,input_json,response_json,facts_json in re.findall(r"QA atomic_effect_result tenant=([a-f0-9-]+) case=([^ ]+) request=([^ ]+) status=(\d+) unchanged=(\w+) input=(.+?) response=(.+?) after=(.+)",log):
    finals[tenant]=json.loads(facts_json)
    check(int(status)==(403 if case=="warehouse-scope" else 422) and unchanged=="true",case+" original counterexample now refused without mutation")
http=[]
for tenant,path,input_json,status,response_json in re.findall(r"QA production_release_http tenant=([a-f0-9-]+) path=([^ ]+) input=(.+?) status=(\d+) response=(.+)",log):
    if path.endswith("/atomic-effects"):
        http.append((tenant,json.loads(input_json),int(status),json.loads(response_json)))
check(len(http)>=22,"real atomic HTTP responses recorded")
for tenant,case,actor,warehouse,sku,order in fixtures:
    check(tenant not in before and tenant in current,case+" fresh owned enterprise")
    row=current.get(tenant,{})
    facts=row.get("facts",{})
    check(row.get("name")=="QA-ERP-ATOMIC-EFFECTS",case+" owned QA identity")
    final=finals.get(tenant)
    check(final is not None,case+" final SQL facts logged")
    for table,rows in (final or {}).items(): check(table in facts and canonical(table,facts.get(table,[]))==canonical(table,rows),case+" independent SQL match "+table)
    docs=facts.get("atomic_source_documents",[])
    expected=case in {"fractional-original","minimum-quantity","zero-cost","revoked-replay","own-reservation","twenty-commands"} or case.startswith("fault-")
    check(len(docs)==(1 if expected else 0),case+" actual document count")
    check(len(facts.get("mfg_material_moves",[]))==len(docs),case+" one material movement per actual document")
    check(len(facts.get("atomic_material_values",[]))==len(docs),case+" one exact value per actual document")
    for doc in docs:
        snapshot=doc.get("snapshot") or {}
        moves=[m for m in facts.get("mfg_material_moves",[]) if m["move_id"]==doc.get("move_id")]
        values=[v for v in facts.get("atomic_material_values",[]) if v["document_id"]==doc["document_id"]]
        check(doc.get("work_order_id")==order and doc.get("warehouse_id")==warehouse and doc.get("sku_id")==sku and doc.get("actor_user_id")==actor,case+" real source and authenticated actor")
        check(snapshot.get("document_id")==doc["document_id"] and snapshot.get("request_id")==doc.get("request_id") and snapshot.get("qty_micros")==str(doc["qty_micros"]),case+" original snapshot identity")
        check(len(moves)==len(values)==1,case+" actual material and value links")
        if moves and values:
            exact=doc["qty_micros"]*values[0]["unit_cost_cent"]
            check(str(values[0]["value_cent_micros"])==snapshot.get("value_cent_micros")==str(exact),case+" exact quantity times server cost")
            check(doc["amount_cent"]==snapshot.get("amount_cent")==((exact+500000)//1000000),case+" rounded cent projection only")
            for name in ["issued","returned","consumed","shop"]:
                check(snapshot.get("material",{}).get(name+"_micros")==str(moves[0].get("material_"+name+"_micros")),case+" original material balance "+name)
        ledger=[l for l in facts.get("inventory_ledgers",[]) if l["ledger_id"]==snapshot.get("inventory_ledger_id")]
        check(len(ledger)==1 and ledger[0].get("ref_id")==doc.get("move_id") and ledger[0].get("delta_on_hand_micros")==-doc["qty_micros"],case+" actual canonical precise inventory effect")
        op=[o for o in facts.get("operation_logs",[]) if o["log_id"]==snapshot.get("operation_log_id")]
        audit=[a for a in facts.get("audit_logs",[]) if a["audit_id"]==snapshot.get("audit_id")]
        outbox=[e for e in facts.get("outbox_events",[]) if e["outbox_id"]==snapshot.get("outbox_id")]
        posting=[p for p in facts.get("atomic_posting_events",[]) if p["event_id"]==snapshot.get("posting_id")]
        base={k:v for k,v in snapshot.items() if k not in {"operation_log_id","audit_id","outbox_id","posting_id"}}
        event={k:v for k,v in snapshot.items() if k not in {"outbox_id","posting_id"}}
        check(len(op)==1 and op[0].get("actor_user_id")==actor and op[0].get("metadata")==base,case+" real mandatory operation log")
        check(len(audit)==1 and audit[0].get("user_id")==actor and audit[0].get("after_json")==base,case+" real mandatory audit log")
        check(len(outbox)==1 and outbox[0].get("payload")==event,case+" real outbox original payload")
        check(len(posting)==1 and posting[0].get("amount_cent")==doc["amount_cent"] and posting[0].get("document_id")==doc["document_id"],case+" real pending event")
        receipts=[r for r in facts.get("command_receipts",[]) if r["request_id"]==doc.get("request_id")]
        check(len(receipts)==1 and receipts[0].get("effect_id")==doc["document_id"] and receipts[0].get("status")=="SUCCEEDED",case+" receipt and all effects persisted together")
    for _,input,status,response in [item for item in http if item[0]==tenant]:
        if status==200:
            matches=[d for d in docs if d["document_id"]==response.get("effect_id")]
            check(len(matches)==1 and response.get("snapshot")==matches[0].get("snapshot"),case+" successful HTTP returns exact independent original snapshot")
    if case=="twenty-commands":
        replies=[item for item in http if item[0]==tenant]
        check(len(replies)==20 and sum(item[3].get("replay")==False for item in replies)==1 and all(item[2]==200 for item in replies),"twenty real replies one original and nineteen replays")
faults=re.findall(r"QA atomic_effect_fault tenant=([a-f0-9-]+) table=([^ ]+) actual_marker=(\w+)",log)
check(len(faults)==5 and all(marker=="true" for _,_,marker in faults),"five real mandatory SQL insertion faults reached")
failed_names=re.findall(r"^test (.+) \.\.\. FAILED$",log,re.M)
if args.expected_regression_failures:
    check(len(failed_names)==args.expected_regression_failures and all(not name.startswith("atomic_effects::") for name in failed_names),"all declared regression failures are explicit and outside the atomic cases")
    check(re.search(r"test result: FAILED\. \d+ passed; "+str(args.expected_regression_failures)+r" failed; 0 ignored; 0 measured; 0 filtered out;",log) is not None,"whole regression remains actually FAILED with no filtering or ignores")
else:
    check(re.search(r"test result: ok\. \d+ passed; 0 failed; 0 ignored;",log) is not None,"actual tests have no failure or ignore")
status="FAIL" if errors else ("PARTIAL" if failed_names else "PASS")
report={"utc":datetime.now(timezone.utc).isoformat(),"status":status,"atomic_fact_verification":"PASS" if not errors else "FAIL",
        "whole_regression_status":"FAILED" if failed_names else "PASSED", "regression_failures":failed_names,
        "scope":"Single-line atomic material ISSUE; no full manufacturing/ordinary login/UI/Release certification",
        "checks":checks,"existing_enterprises":len(before),"protected_tables":65,"fixtures":len(fixtures),"atomic_http_requests":len(http),"real_faults":len(faults),"errors":errors,
        "before_sha256":before_sha,"after_sha256":after_sha,"log_sha256":hashlib.sha256(log_path.read_bytes()).hexdigest(),"timestamp_catalog_sha256":hashlib.sha256(catalog_path.read_bytes()).hexdigest()}
with target.open("x",encoding="utf8") as output: output.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({key:report[key] for key in ["status","checks","existing_enterprises","fixtures","atomic_http_requests","errors"]},ensure_ascii=False))
raise SystemExit(0 if not errors else 1)
