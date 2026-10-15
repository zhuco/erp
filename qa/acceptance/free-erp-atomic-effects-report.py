"""Publish a qualified, immutable checkpoint; never turn the failed full suite into success."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"qa/acceptance"
PREFIX="free-erp-atomic-effects-"
target=BASE/(PREFIX+"repair.json")
if target.exists(): raise SystemExit("refusing to replace historical checkpoint")
def digest(path):
    sha=hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b""): sha.update(chunk)
    return sha.hexdigest()
def load(leaf): return json.loads((BASE/(PREFIX+leaf)).read_text(encoding="utf-8-sig"))
stable=["crates/erp-repo/src/shared_contract/atomic_effects.rs",
        "crates/erp-repo/src/shared_contract/material_moves.rs",
        "crates/erp-repo/src/shared_contract/production_receipt_release.rs",
        "crates/erp-api/src/manufacturing/atomic_effects.rs",
        "crates/erp-api/tests/free_erp_acceptance/atomic_effects.rs",
        "migrations/0295_atomic_material_sources.sql"]
windows={}
for label,start,end in [("original","original-formatted","original-after"),
                        ("focused","repaired-third-before","repaired-third-after"),
                        ("full","full-before","full-after")]:
    def manifest(stage):
        return json.loads((BASE/f"free-erp-inventory-consumers-atomic-effects-{stage}-source.json").read_text(encoding="utf8"))["files"]
    before,after=manifest(start),manifest(end)
    changed=[name for name in sorted(set(before)|set(after)) if before.get(name)!=after.get(name)]
    current={name:{"before":before.get(name),"after":after.get(name),"current":digest(ROOT/name)} for name in stable if name in before}
    windows[label]={"status":"PARTIAL" if changed else "PASS","changed_inputs":changed,
                    "own_leaves":current,"own_leaves_consistent":all(row["before"]==row["after"]==row["current"] for row in current.values())}
log=(BASE/(PREFIX+"full-test.log")).read_text(encoding="utf-8-sig")
summary=re.search(r"test result: FAILED\. (\d+) passed; (\d+) failed; (\d+) ignored; 0 measured; (\d+) filtered out; finished in ([0-9.]+)s",log)
assert summary and tuple(map(int,summary.groups()[:4]))==(316,53,0,0)
diagnostics={}
for scope in ["repo","api"]:
    text=(BASE/(PREFIX+scope+"-clippy.log")).read_text(encoding="utf-8-sig")
    diagnostics[scope]={"status":"FAILED","compiler_summaries":re.findall(r"^error: could not compile[^\r\n]+",text,re.M),
                        "existing_material_history_type_complexity": "material_moves.rs:320" in text,
                        "new_atomic_leaf_diagnostic": "shared_contract\\atomic_effects.rs:" in text or "manufacturing\\atomic_effects.rs:" in text}
artefacts={}
for path in sorted(BASE.glob(PREFIX+"*")):
    if path.is_file() and path!=target:
        artefacts[path.relative_to(ROOT).as_posix()]={"bytes":path.stat().st_size,"sha256":digest(path)}
report={"utc":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL",
        "scope":"One authenticated-source precise ISSUE, exact value declaration and original response; ongoing free ERP goal",
        "original_counterexamples":load("original-independent.json"),
        "focused":{"status":"PASS","passed":22,"failed":0,"ignored":0,"seconds":20.60,"independent":load("repaired-third-independent.json")},
        "full_regression":{"status":"FAILED","passed":316,"failed":53,"ignored":0,"filtered":0,"seconds":float(summary[5]),"independent":load("full-independent.json")},
        "contract":load("repaired-fourth-contract.json"),
        "migration_trial":load("migration-trial.json"),"migration_apply":load("migration-apply.json"),
        "source_windows":windows,"strict_clippy":diagnostics,"artifacts":artefacts,
        "prior_failures_preserved":["Original four unsafe HTTP results","API leaf request type visibility compile error","20 pass and 2 QA readback assertion failures","Contract parser treated uuid::Uuid namespace as fields","Document check observed parallel auth edit","Complete regression 53 explicit failures","Initial full-after connection refusal and zero-byte snapshot"],
        "database_capture_recovery":"Initial full-after was connection refused. Existing QA independently returned; the attempted start precondition rejected because pg_ctl reported the registered cluster already running. No start, reset, cleanup or process termination was executed. Retried protected-identity capture retained all prior facts.",
        "not_run":["Normal password/current roles and opened-session manufacturing acceptance","Full 100-line material documents and batch/multilevel/frozen-release requirements","Original inventory cost layers and ISSUE/CONSUME/RETURN value transfer","Actual durable Worker material event consumption","Native/Web business page closure","Entire migration chain and production deployment","Registered shared_contract integration target","Release latency/load/request-size/peak-memory/public-hosting gates"],
        "next_work":["Migrate old inventory QA helpers to genuine opened sessions and database roles; retain current admission guards","Verify the real caller and authorization/credential epochs in every manufacturing entry","Complete whole material documents, original cost transfer and Worker task execution per chapter 17","Resolve strict Clippy and stable source/current contract gates before release"]}
with target.open("x",encoding="utf8") as output: output.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"artifacts":len(artefacts),"source_changes":{key:len(value["changed_inputs"]) for key,value in windows.items()},"own_leaves_consistent":{key:value["own_leaves_consistent"] for key,value in windows.items()}},ensure_ascii=False))
