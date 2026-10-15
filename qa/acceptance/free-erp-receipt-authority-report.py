"""Freeze actual current-authority evidence with the remaining workflow failure explicit."""
from datetime import datetime,timezone
import hashlib,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"qa/acceptance"
PREFIX="free-erp-receipt-authority-"
target=BASE/(PREFIX+"repair.json")
if target.exists():raise SystemExit("authority repair exists")
def obj(name):
    return json.loads((BASE/(PREFIX+name+".json")).read_text(encoding="utf-8-sig"))
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def require(ok,message):
    if not ok:raise SystemExit(message)
original=obj("original-independent")
first=obj("first-independent")
full=obj("full-independent")
contract=obj("contract-fixed-independent")
status=obj("full-status")
require(original["status"]=="CONFIRMED_UNSAFE","original effects unconfirmed")
require(all(x["status"]=="PASS" for x in (first,full,contract)),"independent results incomplete")
require(status["state"]=="COMPLETED" and status["functional_exit"]==status["independent_exit"]==0,"full controller incomplete")
sa=json.loads((BASE/"free-erp-inventory-consumers-receipt-authority-full-after-source.json").read_text(encoding="utf-8-sig"))["files"]
sb=json.loads((BASE/"free-erp-inventory-consumers-receipt-authority-contract-fixed-after-source.json").read_text(encoding="utf-8-sig"))["files"]
changed=[n for n in sorted(set(sa)|set(sb)) if sa.get(n)!=sb.get(n)]
require(changed==["crates/erp-api/tests/shared_contract.rs","crates/erp-api/tests/shared_contract_receipt_actor.rs"],
    "unexpected post-full changes "+str(changed))
require(all((ROOT/n).is_file() and sha(ROOT/n)==h for n,h in sb.items()),"current source drift")
repo=(BASE/(PREFIX+"contract-repo-clippy.log")).read_text(encoding="utf-8-sig")
api=(BASE/(PREFIX+"contract-api-clippy.log")).read_text(encoding="utf-8-sig")
repo_errors=re.findall(r"could not compile \x60erp-repo\x60 \((lib(?: test)?)\) due to (\d+) previous errors",repo)
adapter_errors=re.findall(r"could not compile \x60(erp-adapter-[^\x60]+)\x60 \(lib\) due to (\d+) previous errors",api)
require(dict(repo_errors)=={"lib":"297","lib test":"299"} and sum(int(x[1]) for x in adapter_errors)==27,"strict lint evidence differs")
files=["original-test.log","original-independent.json","first-test.log","first-independent.json",
    "full-test.log","full-independent.json","full-status.json","migrate.log","contract-direct-purchase.log","contract-status.json",
    "contract-fixed-direct-purchase.log","contract-fixed-status.json","contract-fixed-independent.json",
    "contract-final-format-check.log","contract-repo-clippy.log","contract-api-clippy.log",
    "contract-workflow-direct-purchase.log","contract-workflow-status.json"]
report={"utc":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL",
    "scope":"current receipt release/return/issue authority and retained actors; not full ERP completion",
    "implementation":["mandatory authenticated actor on all three repository methods","verified token epoch Extension, never business body",
        "actual user/session/epoch/role/source warehouse locked after source queue","late expiry recheck and immutable actor snapshot",
        "real second device replay retains original actor/result","current database grant without cached claim or admin bypass"],
    "original":{"status":"CONFIRMED_UNSAFE","fixtures":9,"tests_passed":0,"tests_failed":4,
        "independent_checks":original["checks"],"older_enterprises":original["older_enterprises"]},
    "functional":{"focused":8,"focused_seconds":17.96,"full":511,"full_seconds":726.59,"failed":0,"ignored":0,"filtered":0},
    "independent":{"status":"PASS","checks":full["checks"],"older_enterprises":full["older_enterprises"],
        "protected_tables":77,"source_inputs":len(sa),"source_window_drift":[]},
    "selected_purchase_contract":{"status":"PASS","passed":18,"failed":0,"ignored":0,"filtered":382,"seconds":14.04,
        "checks":contract["checks"],"older_enterprises":contract["older_enterprises"],"tables":77,
        "source_inputs":len(sb),"source_window_drift":[],"original_failed_controller_preserved":True},
    "post_full_changes":{"files":changed,
        "scope":"affected test identity helper and two synthetic HTTP fixtures only; production inputs unchanged; selected target validated separately"},
    "format":"PASS for seven selected leaves",
    "strict_clippy":{"status":"FAIL","repo":dict(repo_errors),"adapters":dict(adapter_errors)},
    "extra_shared_workflow":{"status":"FAIL","passed":0,"failed":1,"filtered":399,
        "failure":"held mfg quantity ledger is physical 6000000 while legacy test expects zero; branch precedes both affected receipt actor helpers",
        "not_counted_as_pass":True},
    "not_qualified":["entire shared workflow/ERP","original purchase price and missing-vs-known-zero arrival policy",
        "all other business write IAM","new actor table in backup/restore/exit export","native/Web/public deployment",
        "Release timing/CPU/RSS/endurance","strict full repository/API Clippy"],
    "artifacts":{"qa/acceptance/"+PREFIX+n:sha(BASE/(PREFIX+n)) for n in files},
    "migration":{"0299":"applied/read back only on erp_shared_contract_test|erp_grid_test|127.0.0.1|15438; no old identity fabricated"},
    "qualified_current_source_sha256":sb}
with target.open("x",encoding="utf8") as f:f.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"focused":8,"full":511,"selected_contract":18,
    "independent_checks":full["checks"],"older_enterprises":full["older_enterprises"],"extra_shared_workflow":"FAIL"},ensure_ascii=False))
