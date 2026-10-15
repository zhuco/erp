"""Bind the actual test windows and current review to immutable input manifests."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
output = BASE / "free-erp-inspection-gates-source-audit.json"
if output.exists():
    raise SystemExit("historical audit exists")

def read(stage):
    path = BASE / f"free-erp-inventory-consumers-inspection-gates-{stage}-source.json"
    raw = path.read_bytes()
    value = json.loads(raw)
    return value["files"], {"path":path.relative_to(ROOT).as_posix(),"sha256":hashlib.sha256(raw).hexdigest(),
                            "count":len(value["files"]),"recorded_at":value["updated_at"]}

leaves = ["crates/erp-repo/src/shared_contract/production_receipt_release.rs",
          "crates/erp-repo/src/shared_contract/trade_stock.rs",
          "crates/erp-api/tests/free_erp_acceptance/inspection_gates.rs"]
windows = []
for start,end in [("original-before","original-after"),
                  ("quality-unfixed-retry-before","quality-unfixed-retry-after"),
                  ("fixed-expanded-before","fixed-expanded-after"),
                  ("fixed-final-before","fixed-final-after"),("full-before","full-after")]:
    before,before_meta = read(start)
    after,after_meta = read(end)
    drift = [{"path":name,"before":before.get(name),"after":after.get(name)}
             for name in sorted(set(before)|set(after)) if before.get(name)!=after.get(name)]
    own_drift = [item for item in drift if item["path"] in leaves]
    windows.append({"start":before_meta,"end":after_meta,"changed_inputs":drift,
                    "tracked_leaf_drift":own_drift,
                    "window_source_qualification":"PARTIAL" if drift else "UNCHANGED_TRACKED_INPUTS"})

baseline,baseline_meta = read("full-after")
current = {}
for name in baseline:
    file = ROOT / name
    current[name] = hashlib.sha256(file.read_bytes()).hexdigest() if file.is_file() else None
drift = [{"path":name,"tested_after":baseline.get(name),"current":current.get(name)}
         for name in sorted(set(baseline)|set(current)) if baseline.get(name)!=current.get(name)]
result = {"created_at":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL" if drift or any(
          window["changed_inputs"] for window in windows) else "PASS",
          "scope":"Native/Web source, schema, build-entry and OpenAPI inputs; actual test facts remain bound to their windows, not later concurrent changes",
          "test_windows":windows,"full_after":baseline_meta,"current_full_after_drift":drift,
          "current_tracked_leaf_drift":[item for item in drift if item["path"] in leaves],
          "current_input_sha256":current,"tracked_implementation_leaves":leaves}
with output.open("x",encoding="utf-8") as stream:
    json.dump(result,stream,ensure_ascii=False,indent=2)
    stream.write("\n")
print(json.dumps({"status":result["status"],"windows":[{"start":item["start"]["path"],
                 "changed":len(item["changed_inputs"]),"leaf_changed":len(item["tracked_leaf_drift"])}
                 for item in windows],"current_drift":[item["path"] for item in drift],
                 "current_leaf_drift":result["current_tracked_leaf_drift"]},ensure_ascii=False))
