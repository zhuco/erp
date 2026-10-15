"""Independent integer reconstruction of real receipt writes and 76-table preservation."""
import argparse
from datetime import datetime,timezone
import hashlib
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"qa/acceptance"
PREFIX="free-erp-receipt-values-"
parser=argparse.ArgumentParser()
parser.add_argument("stage")
for name in ["before","after","log","source-before","source-after"]:
    parser.add_argument("--"+name,required=True)
parser.add_argument("--full",action="store_true")
args=parser.parse_args()
target=BASE/(PREFIX+args.stage+"-independent.json")
if not re.fullmatch(r"[a-z0-9-]+",args.stage) or target.exists():
    raise SystemExit("invalid or existing evidence")
log_path=ROOT/args.log
log=log_path.read_text(encoding="utf-8-sig")
fixtures=re.findall(r"QA trade_facts_fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) case=(value-[^ ]+|return-half-cent|return-semantic-conflict) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+)",log)
owned={row[0] for row in fixtures}
catalog=BASE/(PREFIX+"timestamp-catalog.json")
columns=json.loads(catalog.read_text(encoding="utf-8-sig"))
checks,errors=0,[]

def check(ok,message):
    global checks
    checks+=1
    if not ok:
        errors.append(message)

def utc(value,pg=False):
    text=datetime.fromisoformat(value).astimezone(timezone.utc).isoformat(timespec="microseconds")
    if pg:
        text=text[:-6].rstrip("0").rstrip(".")+text[-6:]
    return text

def canonical(table,values):
    rows=[]
    for item in values:
        row=dict(item)
        for name in columns.get(table,[]):
            if row.get(name) is not None:
                row[name]=utc(row[name])
        rows.append(row)
    return sorted(rows,key=lambda r:json.dumps(r,sort_keys=True,ensure_ascii=False))

def snapshot(path):
    signatures,current,sha,identity={}, {},hashlib.sha256(),None
    with (ROOT/path).open("rb") as stream:
        for raw in stream:
            sha.update(raw)
            if not raw.strip():
                continue
            row=json.loads(raw)
            if "database" in row:
                identity=row
                continue
            tenant=row["tenant_id"]
            check(tenant not in signatures,"unique enterprise "+tenant)
            check(len(row["facts"])==76,"all selected columns "+tenant)
            facts={name:canonical(name,items) for name,items in row["facts"].items()}
            signatures[tenant]=hashlib.sha256(json.dumps({"tenant_id":tenant,"name":row["name"],"facts":facts},
                sort_keys=True,ensure_ascii=False,separators=(",",":")).encode()).hexdigest()
            if tenant in owned:
                current[tenant]=row
    check(identity is not None and identity.get("database")=="erp_shared_contract_test"
        and identity.get("role")=="erp_grid_test" and identity.get("host")=="127.0.0.1"
        and identity.get("port")==15438 and identity.get("selected_tables")==76,"protected snapshot "+path)
    return signatures,current,sha.hexdigest()

def pg_row_text(table,item,exclude=()):
    # Validate against the actual retained SQL fingerprints below. No float money.
    row={name:value for name,value in item.items() if name not in exclude}
    for name in columns.get(table,[]):
        if row.get(name) is not None:
            row[name]=utc(row[name],pg=True)
    keys=sorted(row,key=lambda name:(len(name.encode("utf8")),name.encode("utf8")))
    return json.dumps({name:row[name] for name in keys},ensure_ascii=False,separators=(", ",": "))

def digest(table,rows,key,exclude=()):
    text="|".join(pg_row_text(table,row,exclude) for row in sorted(rows,key=lambda item:item[key]))
    return hashlib.md5(text.encode("utf8"),usedforsecurity=False).hexdigest()

before,_,before_sha=snapshot(args.before)
after,current,after_sha=snapshot(args.after)
changed=[]
for tenant in before:
    if before[tenant]!=after.get(tenant):
        changed.append(tenant)
    check(before[tenant]==after.get(tenant),"older enterprise 76 tables unchanged "+tenant)
expected={"return-half-cent","return-semantic-conflict","value-later-release","value-half-releases",
    "value-issue-return","value-zero-history","value-twenty-race","value-legacy-replay","value-rollback","value-source-identity"}
check(len(fixtures)==len(owned)==10,"ten distinct actual account fixtures")
check({row[2] for row in fixtures}==expected,"all named receipt write boundary cases")
result=re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; (\d+) filtered out;",log)
check(len(result)==1,"one actual passing registered result")
if result:
    check((int(result[0][0])>=503 and result[0][1]=="0") if args.full else int(result[0][0])==10,
        "complete required registered case count")
observations={}
for tenant,actor,case,warehouse,sku in fixtures:
    check(tenant not in before and tenant in current,case+" fresh owned enterprise")
    facts=current.get(tenant,{}).get("facts",{})
    users=facts.get("users",[])
    check(len(users)==1 and users[0]["user_id"]==actor and users[0]["status"]=="ACTIVE",case+" actual active user")
    check(len(users)==1 and users[0]["password_hash"].startswith("$argon2id$v=19$m=19456,t=2,p=1$"),case+" genuine salted password")
    sessions=facts.get("auth_sessions",[])
    check(len(sessions)==1 and sessions[0]["user_id"]==actor and sessions[0]["credentials_epoch"]==0,
        case+" actual server-opened session")
    check(any(row["user_id"]==actor and row["warehouse_id"]==warehouse for row in facts.get("user_warehouse_access",[])),
        case+" genuine warehouse grant")
    commands=facts.get("shared_receipt_value_commands",[])
    receipts={row["request_id"]:row for row in facts.get("command_receipts",[])}
    allocations=facts.get("shared_receipt_cost_allocations",[])
    all_lots=facts.get("shared_receipt_cost_lots",[])
    stock_by_id={row["ledger_id"]:row for row in facts.get("shared_stock_ledgers",[])}
    ap_by_id={row["balance_id"]:row for row in facts.get("partner_balances",[])}
    returns_by_id={row["return_id"]:row for row in facts.get("shared_supplier_returns",[])}
    qualities={row["quality_id"]:row for row in facts.get("shared_receipt_quality",[])}
    stock_ids={row["stock_id"] for row in commands}|{row["held_id"] for row in commands if row["held_id"]}
    balance_ids={row["balance_id"] for row in commands if row["balance_id"]}
    return_ids={row["return_id"] for row in commands if row["return_id"]}
    for command in commands:
        request=command["request_id"]
        q=qualities.get(command["quality_id"])
        r=receipts.get(request)
        check(q is not None and r is not None,case+" immutable command has its quality and original receipt "+request)
        if q is None or r is None:
            continue
        identity=["receipt-value-v1",tenant,q["quality_id"],command["action"],command["qty_micros"],request]
        expected_hash=hashlib.sha256(json.dumps(identity,ensure_ascii=False,separators=(",",":")).encode()).hexdigest()
        check(r["status"]=="SUCCEEDED" and r["payload_hash"]==command["payload_hash"]==expected_hash
            and r["effect_id"]==str(command["result"]),case+" original business identity and result "+request)
        s=stock_by_id.get(command["stock_id"])
        delta=command["qty_micros"] if command["action"]=="RELEASE" else -command["qty_micros"]
        check(s is not None and s["source_line"]==q["quality_id"] and s["request_id"]==request
            and (s["warehouse_id"],s["location_id"],s["sku_id"],s["batch_code"])==
                (q["warehouse_id"],q["location_id"],q["sku_id"],q["batch_code"])
            and s["bucket"]=="ON_HAND" and s["quality_status"]=="GOOD" and s["delta_micros"]==delta,
            case+" exact original physical effect "+request)
        selected=[row for row in allocations if row["request_id"]==request]
        if command["action"]=="RELEASE":
            h=stock_by_id.get(command["held_id"])
            check(h is not None and h["source_line"]==q["quality_id"] and h["request_id"]==request
                and h["bucket"]=="HELD" and h["quality_status"]=="HELD" and h["delta_micros"]==-command["qty_micros"],
                case+" exact held release "+request)
            lot=[row for row in all_lots if row["request_id"]==request]
            check(len(lot)==1 and lot[0]["qty_micros"]==command["qty_micros"] and lot[0]["cost_cent"]==command["amount_cent"],
                case+" actual frozen release cost "+request)
            check(not selected,case+" release does not consume a lot "+request)
        else:
            check(sum(row["qty_micros"] for row in selected)==command["qty_micros"]
                and sum(row["cost_cent"] for row in selected)==command["amount_cent"]
                and all(row["quality_id"]==q["quality_id"] and row["action"]==command["action"] for row in selected),
                case+" allocations are the actual consumed quantity and cents "+request)
        if command["action"]!="ISSUE":
            p=ap_by_id.get(command["balance_id"])
            amount=command["amount_cent"] if command["action"]=="RELEASE" else -command["amount_cent"]
            check(p is not None and p["source_id"]==q["receipt_id"] and p["partner_code"]==q["supplier_code"]
                and p["kind"]=="AP" and p["currency"]=="CNY" and p["source_type"]=="PURCHASE_"+command["action"]
                and p["amount_cent"]==amount,case+" exact actual source AP "+request)
        else:
            check(command["balance_id"] is None and command["return_id"] is None,
                case+" issue cannot refund supplier AP "+request)
        if command["action"]=="RETURN":
            x=returns_by_id.get(command["return_id"])
            check(x is not None and x["receipt_id"]==q["receipt_id"] and x["qty_micros"]==command["qty_micros"]
                and x["cost_cent"]==command["amount_cent"],case+" exact actual supplier return "+request)
    for q in qualities.values():
        quality=q["quality_id"]
        physical=[row for row in stock_by_id.values() if row["source_line"]==quality]
        returns=[row for row in returns_by_id.values() if row["receipt_id"]==q["receipt_id"]]
        postings=[row for row in ap_by_id.values() if row["source_id"]==q["receipt_id"] and row["kind"]=="AP"
            and row["currency"]=="CNY" and row["partner_code"]==q["supplier_code"]]
        returned=sum(row["qty_micros"] for row in returns)
        returned_cent=sum(row["cost_cent"] for row in returns)
        gross=sum(row["amount_cent"] for row in postings if row["source_type"]=="PURCHASE_RELEASE")
        offset=-sum(row["amount_cent"] for row in postings if row["source_type"]=="PURCHASE_RETURN")
        held=sum(row["delta_micros"] for row in physical if row["bucket"]=="HELD")
        hand=sum(row["delta_micros"] for row in physical if row["bucket"]=="ON_HAND")
        check((held,hand)==(q["arrived_micros"]-q["released_micros"],q["released_micros"]-returned-q["issued_micros"]),
            case+" independently rebuilt original quantities "+quality)
        check(q["payable_cent"]==gross-returned_cent and returned_cent==offset and 0<=returned_cent<=gross,
            case+" independently rebuilt actual AP "+quality)
        lots=[row for row in all_lots if row["quality_id"]==quality]
        selected_commands=[row for row in commands if row["quality_id"]==quality]
        if not lots:
            check(not selected_commands,case+" unknown history has no fabricated lot "+quality)
            continue
        legacy=[row for row in lots if row["kind"]=="LEGACY_BALANCE"]
        check(len(legacy)==1,case+" exactly one frozen known legacy origin "+quality)
        if len(legacy)!=1:
            continue
        origin=legacy[0]["legacy_facts"]
        check(legacy[0]["qty_micros"]==origin["released"]-origin["returned_qty"]
            and legacy[0]["cost_cent"]==origin["release_cost"]-origin["returned_cost"],
            case+" original remaining quantity and actual value "+quality)
        old_stock=[row for row in physical if row["ledger_id"] not in stock_ids]
        old_postings=[row for row in ap_by_id.values() if row["source_id"]==q["receipt_id"]
            and row["source_type"] in ("PURCHASE_RELEASE","PURCHASE_RETURN") and row["balance_id"] not in balance_ids]
        old_returns=[row for row in returns if row["return_id"] not in return_ids]
        rebuilt_fingerprint={"stock":digest("shared_stock_ledgers",old_stock,"ledger_id"),
            "postings":digest("partner_balances",old_postings,"balance_id",("settled_cent","due_date")),
            "returns":digest("shared_supplier_returns",old_returns,"return_id")}
        check(origin["fingerprint"]==rebuilt_fingerprint,case+" independent canonical UTC source fingerprint "+quality)
        released=[row for row in selected_commands if row["action"]=="RELEASE"]
        restored=[row for row in selected_commands if row["action"]=="RETURN"]
        issued=[row for row in selected_commands if row["action"]=="ISSUE"]
        check(q["released_micros"]==origin["released"]+sum(row["qty_micros"] for row in released)
            and returned==origin["returned_qty"]+sum(row["qty_micros"] for row in restored)
            and q["issued_micros"]==sum(row["qty_micros"] for row in issued),
            case+" no unpriced quantity consumption "+quality)
        check(gross==origin["release_cost"]+sum(row["amount_cent"] for row in released)
            and returned_cent==origin["returned_cost"]+sum(row["amount_cent"] for row in restored),
            case+" original actual gross and returned values "+quality)
        remaining_qty,remaining_cent=0,0
        for lot in sorted(lots,key=lambda row:row["ordinal"]):
            consumed=[row for row in allocations if row["quality_id"]==quality and row["lot_id"]==lot["lot_id"]]
            qty,cent=0,0
            for row in sorted(consumed,key=lambda item:item["ordinal"]):
                qty+=row["qty_micros"]
                cent+=row["cost_cent"]
                check(lot["qty_micros"]>0 and 0<qty<=lot["qty_micros"]
                    and cent==(lot["cost_cent"]*qty+lot["qty_micros"]//2)//lot["qty_micros"],
                    case+" exact integer cumulative original-lot remainder "+row["request_id"])
            check(0<=qty<=lot["qty_micros"] and 0<=cent<=lot["cost_cent"]
                and (qty!=lot["qty_micros"] or cent==lot["cost_cent"]),
                case+" no overconsumed lot value "+lot["lot_id"])
            remaining_qty+=lot["qty_micros"]-qty
            remaining_cent+=lot["cost_cent"]-cent
        check(remaining_qty==hand and remaining_cent==gross-returned_cent-sum(row["amount_cent"] for row in issued),
            case+" independent remaining value and quantities "+quality)
    markers=re.findall(r"QA receipt_values_result tenant="+tenant+r" case=([^ ]+) quality=([a-f0-9-]+)([^\r\n]*)",log)
    check(len(markers)==1,case+" genuine result marker")
    observations[case]={"quality_count":len(qualities),"stored_commands":len(commands),
        "frozen_lots":len(all_lots),"allocations":len(allocations),"marker":markers[0] if markers else None}
sources=[ROOT/args.source_before,ROOT/args.source_after]
a,b=[json.loads(path.read_text(encoding="utf-8-sig"))["files"] for path in sources]
drift=[name for name in sorted(set(a)|set(b)) if a.get(name)!=b.get(name)]
report={"utc":datetime.now(timezone.utc).isoformat(),"status":"FAIL" if errors else ("PARTIAL" if drift else "PASS"),
    "scope":"Real password/HTTP receipt release, return and issue; independent exact source values and protected QA preservation. No native/Web or Release qualification.",
    "full_regression":args.full,"registered_result":result,"checks":checks,"protected_tables":76,
    "older_enterprises":len(before),"older_enterprises_changed":changed,"observations":observations,
    "errors":errors,"source_input_drift":drift,"source_input_count":len(a),
    "before_sha256":before_sha,"after_sha256":after_sha,
    "log_sha256":hashlib.sha256(log_path.read_bytes()).hexdigest(),
    "source_before_sha256":hashlib.sha256(sources[0].read_bytes()).hexdigest(),
    "source_after_sha256":hashlib.sha256(sources[1].read_bytes()).hexdigest(),
    "timestamp_catalog_sha256":hashlib.sha256(catalog.read_bytes()).hexdigest()}
with target.open("x",encoding="utf8") as output:
    output.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"checks":checks,"older_enterprises":len(before),
    "error_count":len(errors),"errors_preview":errors[:10],"source_input_drift":drift},ensure_ascii=False))
raise SystemExit(1 if errors else 0)
