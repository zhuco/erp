"""Preserve failed orchestration while independently confirming its finished run."""
from datetime import datetime,timezone
import hashlib
import json
from pathlib import Path
import re

BASE = Path(__file__).resolve().parent
PREFIX = "free-erp-session-pool-oct7-"
target = BASE/(PREFIX+"detached-reconciled.json")
if target.exists():
    raise SystemExit("refusing to replace reconciliation evidence")
status_path = BASE/(PREFIX+"detached-status.json")
log_path = BASE/(PREFIX+"detached-test.log")
facts_path = BASE/(PREFIX+"detached-independent.json")
status = json.loads(status_path.read_text(encoding="utf-8-sig"))
facts = json.loads(facts_path.read_text(encoding="utf-8-sig"))
terminal = re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in ([\d.]+)s",log_path.read_text(encoding="utf-8-sig"))
if len(terminal)!=1 or status["functional_exit"]!=0 or facts["errors"] or facts["whole_regression_status"]!="PASSED":
    raise SystemExit("actual full terminal result and independent readback not both successful")
sha = lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
report = {"utc":datetime.now(timezone.utc).isoformat(),"status":"PASS","original_orchestration":status,
          "scope":"Reconcile the actual completed registered run; do not rewrite the controller failure or infer release/performance",
          "actual_full_passed":int(terminal[0][0]),"seconds":float(terminal[0][1]),
          "independent_status":facts["status"],"independent_checks":facts["checks"],"protected_older_enterprises":facts["older_enterprises"],
          "orchestration_error":"JavaScript patch string had dropped regex backslashes; source detection was corrected and the exact PowerShell regex has been checked",
          "status_sha256":sha(status_path),"log_sha256":sha(log_path),"independent_sha256":sha(facts_path)}
with target.open("x",encoding="utf8") as output:
    output.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({name:report[name] for name in ["status","actual_full_passed","independent_status","independent_checks","protected_older_enterprises"]}))
