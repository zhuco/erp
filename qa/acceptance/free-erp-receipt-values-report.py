"""Freeze the actual receipt value unit; functional success is not release qualification."""
from datetime import datetime,timezone
import hashlib,json,re
from pathlib import Path

root=Path(__file__).resolve().parents[2]
base=root/"qa/acceptance"
prefix="free-erp-receipt-values-"
target=base/(prefix+"repair.json")
if target.exists():
    raise SystemExit("repair evidence already exists")

def read(name):
    return json.loads((base/(prefix+name+".json")).read_text(encoding="utf-8-sig"))

def log(name):
    return (base/(prefix+name+".log")).read_text(encoding="utf-8-sig")

def require(ok,message):
    if not ok:
        raise SystemExit(message)

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

edges=read("edges-pwsh-independent")
full=read("full-independent")
full_status=read("full-status")
edge_status=read("edges-pwsh-status")
quality=read("quality-status")
require(full["status"]==edges["status"]=="PASS","independent source results did not pass")
require(full_status["state"]==edge_status["state"]=="COMPLETED"
    and full_status["functional_exit"]==full_status["independent_exit"]==0
    and edge_status["functional_exit"]==edge_status["independent_exit"]==0,"actual controllers did not complete")
require(full["registered_result"]==[["503","0"]] and edges["registered_result"]==[["10","493"]],"actual regression sizes differ")
require(full["protected_tables"]==76 and not full["older_enterprises_changed"] and not full["source_input_drift"],"full source/preservation is incomplete")
require(quality["state"]=="COMPLETED" and quality["results"]=={"format":0,"repo-clippy":1,"api-clippy":1},"quality gates differ")
original=log("original-readback-test")
require("amount_a=1 amount_b=1 payable=-1 returned_cost=2 returned_qty=1000000 ap=-1 stock=0" in original
    and "first=200 retry=200 unchanged=true" in original,"actual original unsafe results unavailable")
source_a,source_b=[json.loads((base/f"free-erp-inventory-consumers-receipt-values-readback-{stage}-source.json").read_text(encoding="utf-8-sig"))["files"]
    for stage in ("before","after")]
require(source_a==source_b,"original counterexample source window changed")
original_snippet=base/(prefix+"original-methods-bytes.txt")
require(sha(original_snippet)=="b04825d3442bf8947ca4eb4d69f5c65d82c025f6f961ad5fe783ac931cecd4a8","original method bytes differ")
repo_log=log("quality-repo-clippy")
api_log=log("quality-api-clippy")
repo_errors=re.findall(r"could not compile \x60erp-repo\x60 \((lib(?: test)?)\) due to (\d+) previous errors",repo_log)
adapter_errors=re.findall(r"could not compile \x60(erp-adapter-[^\x60]+)\x60 \(lib\) due to (\d+) previous errors",api_log)
require(dict(repo_errors)=={"lib":"297","lib test":"299"} and sum(int(x[1]) for x in adapter_errors)==27,"strict lint evidence differs")
own_diagnostics=[]
for text in [repo_log,api_log]:
    own_diagnostics+=re.findall(r"(?m)^\s*-->\s*(.*(?:receipt_values|receipt_offsets|receipt_value_edges)\.rs:\d+:\d+)",text)
require(not own_diagnostics,"new receipt leaf diagnostics need review")
metadata_files=[
    "original-test.log","original-authorized-test.log","original-readback-test.log","original-methods-bytes.txt",
    "first-test.log","edges-test.log","edges-status.json","edges-run.stdout.log","edges-run.stderr.log",
    "edges-pwsh-test.log","edges-pwsh-status.json","edges-pwsh-independent.json",
    "full-test.log","full-status.json","full-independent.json","migrate.log","timezone-migrate.log",
    "quality-status.json","quality-format.log","quality-repo-clippy.log","quality-api-clippy.log",
    "readback.sql","timestamp-catalog.json","run.ps1","quality.ps1","capture.py","verify.py","report.py",
]
artifacts={f"qa/acceptance/{prefix+name}":sha(base/(prefix+name)) for name in metadata_files}
source_paths=[
    "crates/erp-repo/src/shared_contract.rs","crates/erp-repo/src/shared_contract/receipt_values.rs",
    "crates/erp-repo/src/shared_contract/receipt_offsets.rs","crates/erp-domain/src/trade.rs",
    "crates/erp-api/src/procurement_handlers.rs","crates/erp-api/tests/free_erp_acceptance.rs",
    "crates/erp-api/tests/free_erp_acceptance/receipt_values.rs","crates/erp-api/tests/free_erp_acceptance/receipt_value_edges.rs",
    "crates/erp-api/tests/free_erp_acceptance/trade_facts.rs",
    "migrations/0297_purchase_receipt_value_sources.sql","migrations/0298_purchase_receipt_fingerprint_timezone.sql",
]
manifest=json.loads((base/"free-erp-inventory-consumers-receipt-values-full-after-source.json").read_text(encoding="utf-8-sig"))["files"]
require(all(manifest.get(name)==sha(root/name) for name in source_paths),"final relevant source changed after qualification")
report={
    "utc":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL",
    "objective":"T02 receipt-local original release costs, shared issue/return remainder and semantic command identity",
    "implemented":["frozen actual release lots and append-only FIFO allocations","exact cumulative original cent conservation",
        "atomic stock/AP/return/cost/command receipt","canonical request content identity and evidence-bound old weak-hash replay",
        "unknown legacy issue cost refuses new posting","source-aware read reconciliation and UTC fingerprints",
        "mixed actions reject; actual storage failures 500 and transaction rollback"],
    "original":{"status":"CONFIRMED_UNSAFE","two_half_returns":{"source_cent":1,"credited_cent":2,"payable":-1,"ap":-1,"stock":0},
        "changed_quantity_same_id":{"actual_status":200,"replay":True},"source_input_count":len(source_a),"source_window_equal":True,
        "fixture_permission_and_sql_errors_retained":True},
    "functional":{"status":"PASS","new_cases":10,"new_cases_seconds":13.48,
        "full_cases":503,"failed":0,"ignored":0,"filtered":0,"full_seconds":556.83},
    "independent":{"status":"PASS","checks":full["checks"],"older_enterprises":full["older_enterprises"],
        "tables":76,"older_enterprises_changed":[],"source_input_count":full["source_input_count"],"source_input_drift":[],
        "original_source_fingerprints":"independent canonical UTC PostgreSQL JSON reconstruction",
        "cost_arithmetic":"independent exact Python integers over actual frozen lots and chronological allocations"},
    "quality":{"format":"PASS","repo_clippy":"FAIL","repo_errors":dict(repo_errors),
        "api_clippy":"FAIL","adapter_errors":dict(adapter_errors),"new_leaf_diagnostics":own_diagnostics,
        "no_package_pass_inferred":True},
    "migrations":{"applied_only_to":"erp_shared_contract_test|erp_grid_test|127.0.0.1|15438","0297":"three tables, source relations, commit checks and immutability read back",
        "0298":"UTC function setting read back; old fingerprints match in UTC and Asia/Shanghai sessions; no historical data rewritten"},
    "first_background_stage":{"status":"NO_FUNCTIONAL_RESULT","powershell":"5","native_stderr_interruption":True,
        "after_snapshot_equals_before":True,"missing_checker_stderr_retained":True,"replacement_stage":"edges-pwsh",
        "no_cache_cleanup_or_lock_bypass":True},
    "not_qualified":["original arrival purchase-price authorization and missing-vs-known-zero policy",
        "current actor/session/credential epoch/role/actual warehouse and in-flight write revocation",
        "full authoritative inventory/cost/general-ledger workflow","new tables in backup/restore/exit export",
        "native and Web business UI","official hosted deployment and production migrations","Release ERP-P timing/RSS/concurrency endurance",
        "entire ERP strict Clippy and complete free product"],
    "before_sha256":full["before_sha256"],"after_sha256":full["after_sha256"],
    "artifacts":artifacts,"qualified_source_sha256":{name:sha(root/name) for name in source_paths},
}
with target.open("x",encoding="utf8") as output:
    output.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"functional":"PASS","full_cases":503,"independent":"PASS",
    "checks":full["checks"],"older_enterprises":full["older_enterprises"],"strict_clippy":"FAIL"},ensure_ascii=False))
