"""Independent 73-table preservation and real-account receipt source checks."""
import argparse
from datetime import datetime,timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT/"qa/acceptance"
PREFIX = "free-erp-trade-facts-"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
parser.add_argument("--before",required=True)
parser.add_argument("--after",required=True)
parser.add_argument("--log",required=True)
parser.add_argument("--source-before",required=True)
parser.add_argument("--source-after",required=True)
parser.add_argument("--full",action="store_true")
parser.add_argument("--first-three",action="store_true")
args = parser.parse_args()
target = BASE/(PREFIX+args.stage+"-independent.json")
if not re.fullmatch(r"[a-z0-9-]+",args.stage) or target.exists():
    raise SystemExit("invalid or existing evidence")
log_path = ROOT/args.log
log = log_path.read_text(encoding="utf-8-sig")
fixtures = re.findall(r"QA trade_facts_fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) case=([^ ]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+)",log)
owned = {row[0] for row in fixtures}
columns_path = BASE/(PREFIX+"timestamp-catalog.json")
columns = json.loads(columns_path.read_text(encoding="utf-8-sig"))
checks,errors = 0,[]


def check(ok,message):
    global checks
    checks += 1
    if not ok:
        errors.append(message)


def canonical(table,values):
    result = []
    for original in values:
        row = dict(original)
        for name in columns.get(table,[]):
            if row.get(name) is not None:
                row[name] = datetime.fromisoformat(row[name]).astimezone(timezone.utc).isoformat(timespec="microseconds")
        result.append(row)
    return sorted(result,key=lambda row:json.dumps(row,sort_keys=True,ensure_ascii=False))


def snapshot(path):
    signatures,current,sha,identity = {},{},hashlib.sha256(),None
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
            check(len(row["facts"])==73,"all selected tables "+tenant)
            facts = {name:canonical(name,values) for name,values in row["facts"].items()}
            signatures[tenant] = hashlib.sha256(json.dumps({"tenant_id":tenant,"name":row["name"],"facts":facts},sort_keys=True,ensure_ascii=False,separators=(",",":")).encode()).hexdigest()
            if tenant in owned:
                current[tenant] = row
    check(identity is not None and identity.get("database")=="erp_shared_contract_test"
          and identity.get("role")=="erp_grid_test" and identity.get("host")=="127.0.0.1"
          and identity.get("port")==15438 and identity.get("selected_tables")==73,"protected QA identity "+path)
    return signatures,current,sha.hexdigest()


before,_,before_sha = snapshot(args.before)
after,current,after_sha = snapshot(args.after)
changed = []
for tenant in before:
    if before[tenant]!=after.get(tenant):
        changed.append(tenant)
    check(before[tenant]==after.get(tenant),"older enterprise all selected columns unchanged "+tenant)
expected = {"partial-rounding-same-sku","known-zero-missing-posting","payable-conflict","foreign"}
if not args.first_three:
    expected |= {"current-authority","storage-fault"}
check(len(fixtures)==len(owned)==len(expected),"all distinct fresh actual account fixtures")
check({row[2] for row in fixtures}==expected,"all named receipt boundary cases")
observations = {}
for tenant,actor,case,warehouse,sku in fixtures:
    check(tenant not in before and tenant in current,case+" fresh owned enterprise")
    facts = current.get(tenant,{}).get("facts",{})
    users = facts.get("users",[])
    check(len(users)==1 and users[0]["user_id"]==actor and users[0]["status"]=="ACTIVE",case+" one active actual business user")
    check(len(users)==1 and users[0]["password_hash"].startswith("$argon2id$v=19$m=19456,t=2,p=1$"),case+" default real salted password")
    sessions = facts.get("auth_sessions",[])
    check(len(sessions)==1 and sessions[0]["user_id"]==actor and sessions[0]["credentials_epoch"]==0,case+" server-opened actual session")
    qualities = facts.get("shared_receipt_quality",[])
    if case=="foreign":
        check(not qualities,"foreign account fixture has no copied receipt")
        continue
    markers = re.findall(r"QA trade_facts_result tenant="+tenant+r" case="+case+r" quality=([a-f0-9-]+)([^\r\n]*)",log)
    check(len(markers)==1,case+" actual read outcome marker")
    if len(markers)!=1:
        continue
    quality,detail = markers[0]
    check("zero_writes=true" in detail,case+" actual per-request independent zero-write assertion")
    rows = [row for row in qualities if row["quality_id"]==quality]
    check(len(rows)==1,case+" stored actual source identity")
    if not rows:
        continue
    q = rows[0]
    receipt = q["receipt_id"]
    originals = [row for row in facts.get("shared_purchase_receipts",[]) if row["receipt_id"]==receipt]
    check(len(originals)==1 and originals[0]["qty_micros"]==q["arrived_micros"],case+" original receipt quantity binding")
    source_stock = [row for row in facts.get("shared_stock_ledgers",[]) if row.get("source_line")==quality]
    held = sum(row["delta_micros"] for row in source_stock if row["bucket"]=="HELD")
    hand = sum(row["delta_micros"] for row in source_stock if row["bucket"]=="ON_HAND" and row["quality_status"]=="GOOD")
    returns = [row for row in facts.get("shared_supplier_returns",[]) if row["receipt_id"]==receipt]
    returned_qty = sum(row["qty_micros"] for row in returns)
    returned_cost = sum(row["cost_cent"] for row in returns)
    postings = [row for row in facts.get("partner_balances",[]) if row["source_id"]==receipt and row["kind"]=="AP"]
    gross = sum(row["amount_cent"] for row in postings if row["source_type"]=="PURCHASE_RELEASE")
    returned_posted = -sum(row["amount_cent"] for row in postings if row["source_type"]=="PURCHASE_RETURN")
    check(returned_posted==returned_cost,case+" actual source return posting")
    check(held==q["arrived_micros"]-q["released_micros"] and hand==q["released_micros"]-returned_qty-q["issued_micros"],case+" independently reconstructed source stock")
    if case=="partial-rounding-same-sku":
        check((q["arrived_micros"],q["released_micros"],returned_qty,q["issued_micros"],gross,returned_cost,q["payable_cent"],held,hand)==(7_000_000,5_000_000,1_250_000,500_000,7996,1999,5997,2_000_000,3_250_000),"exact non-sample partial posting conservation")
        check(sum(row["amount_cent"] for row in postings)==q["payable_cent"],"actual AP source balance")
        check(len(qualities)==2 and sum(row["delta_micros"] for row in facts.get("shared_stock_ledgers",[]) if row["bucket"]=="ON_HAND")>hand,"same SKU other receipt excluded from result")
        check("status=200" in detail,"partial actual HTTP success")
    elif case=="known-zero-missing-posting":
        check(gross==returned_cost==q["payable_cent"]==0 and any(row["source_type"]=="PURCHASE_RELEASE" for row in postings),"known zero has an actual source posting")
        check(len(qualities)==2,"separate missing-posting fixture, no deletion of original zero posting")
        check(("rejected=422" if args.first_three else "rejected=409") in detail and "status=200" in detail,"zero success and missing source actual denial")
    elif case=="payable-conflict":
        check(q["payable_cent"]-sum(row["amount_cent"] for row in postings)==1,"actual one-cent stored conflict")
        check(("rejected=422" if args.first_three else "rejected=409") in detail,"actual conflict denial")
    elif case=="current-authority":
        check("warehouse_rejected=404" in detail and "role_rejected=404" in detail and "old_token=true" in detail,"same genuine old token refused after stored authority changes")
        check(not facts.get("role_permissions",[]) and any(row["user_id"]==actor and row["warehouse_id"]==warehouse for row in facts.get("user_warehouse_access",[])),"independent final role revocation and restored warehouse")
    elif case=="storage-fault":
        check("rejected=500" in detail and "schema=qa_receipt_fault_" in detail,"actual owned SQL fault classified as unavailable")
        check(sum(row["amount_cent"] for row in postings)==q["payable_cent"],"storage failure leaves balanced facts intact")
    observations[case] = {"quality_id":quality,"source_held_micros":held,"source_on_hand_micros":hand,
                          "released_cost_cent":gross,"return_cost_cent":returned_cost,"stored_payable_cent":q["payable_cent"]}
if args.full:
    check(re.search(r"test result: ok\. \d+ passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;",log) is not None,"actual whole registered regression, no ignores or filters")
else:
    count = 3 if args.first_three else 5
    check(re.search(r"test result: ok\. "+str(count)+r" passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out;",log) is not None,"all actual selected source/authority scenarios")
paths = [ROOT/args.source_before,ROOT/args.source_after]
a,b = [json.loads(path.read_text(encoding="utf-8-sig"))["files"] for path in paths]
drift = [name for name in sorted(set(a)|set(b)) if a.get(name)!=b.get(name)]
report = {"utc":datetime.now(timezone.utc).isoformat(),"status":"FAIL" if errors else ("PARTIAL" if drift else "PASS"),
          "scope":"Genuine password identity and shipped receipt GET over seeded owned source facts; no claim of full procurement writes, native/Web or Release",
          "full_regression":args.full,"checks":checks,"protected_tables":73,"older_enterprises":len(before),
          "older_enterprises_changed":changed,"observations":observations,"errors":errors,"source_input_drift":drift,
          "before_sha256":before_sha,"after_sha256":after_sha,"log_sha256":hashlib.sha256(log_path.read_bytes()).hexdigest(),
          "source_before_sha256":hashlib.sha256(paths[0].read_bytes()).hexdigest(),"source_after_sha256":hashlib.sha256(paths[1].read_bytes()).hexdigest(),
          "timestamp_catalog_sha256":hashlib.sha256(columns_path.read_bytes()).hexdigest()}
with target.open("x",encoding="utf8") as output:
    output.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"checks":checks,"older_enterprises":len(before),"error_count":len(errors),"errors_preview":errors[:8],"source_input_drift":drift},ensure_ascii=False))
raise SystemExit(1 if errors else 0)
