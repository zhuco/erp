"""Deliver the scoped repair with retained failures and explicit unresolved gates."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-command-readback-"
destination = BASE / (PREFIX + "repair.json")
if destination.exists():
    raise SystemExit("refusing to replace completed repair evidence")

def read(name):
    return json.loads((BASE / (PREFIX + name)).read_text(encoding="utf-8-sig"))

def result(name):
    log = (BASE / (PREFIX + name)).read_text(encoding="utf-8-sig")
    matches = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out; finished in ([\d.]+)s",log)
    return [{"status":status,"passed":int(passed),"failed":int(failed),"ignored":int(ignored),"filtered":int(filtered),"seconds":float(seconds)}
            for status,passed,failed,ignored,filtered,seconds in matches]

artifacts = {}
for file in sorted(BASE.glob(PREFIX + "*")):
    if not file.is_file() or file == destination:
        continue
    digest = hashlib.sha256()
    with file.open("rb") as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b""):
            digest.update(chunk)
    artifacts[file.relative_to(ROOT).as_posix()] = {"bytes":file.stat().st_size,"sha256":digest.hexdigest()}
docs = read("docs-delivery.json")
report = {"utc":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL",
    "scope":"Read-only command results, client status observation, actual material effect recovery and the detailed Grok implementation/performance specification.",
    "fixed":["Lost response no longer posts caller-described stock or completes the original command.",
             "Client observations cannot change the server status or occupy a missing business identity.",
             "Actual typed material receipts can be read without fake generic metadata and retain their real effect and work-order IDs.",
             "Current owned, unexpired, current-epoch sessions are checked; unproven legacy generic terminal results require reconciliation without data rewriting."],
    "actual_tests":{"original":result("original-test.log"),"focused":result("fixed-focused-test.log"),"full_registered_target":result("full-test.log"),"domain_contract":result("domain-test.log")},
    "independent":{"original":read("original-independent.json"),"focused":read("fixed-focused-verified-independent.json"),"full":read("full-independent.json")},
    "retained_evidence_issue":"The first fact comparator compared UTC and +08 timestamps as text; that failure and the original verifier remain. Only catalog-confirmed timestamptz columns are normalized to precise UTC instants in the corrected comparator.",
    "format":"PASS: three owned Rust leaves and two changed domain leaves via the unified entry; no whole-parent reformat or cache cleanup.",
    "strict_clippy":{"status":"FAIL","repository_lib_errors":240,"repository_lib_test_errors":242,"api_blocked_by_dependencies":{"repository":240,"jd":5,"taobao":7,"pdd":6,"douyin":9}},
    "contract":{"v2_gate":"PASS","focused":read("fixed-focused-openapi.json"),"full":read("full-openapi.json")},
    "source_qualification":read("source-audit.json"),
    "documentation":{"fixed_snapshot":read("docs-fifth.json")["status"],"delivery":docs["status"],"delivery_errors":docs["errors"],
        "command_tasks":docs["checks"]["command_readback_implementation_tasks"],"command_samples":docs["checks"]["command_readback_acceptance_samples"],"command_performance_groups":docs["checks"]["command_readback_performance_groups"],
        "source_fact_checks":docs["checks"]["command_readback_current_source_facts_verified"]},
    "remaining":["Authoritative sources, actors, current warehouse scope and immutable original results across all business domains.",
                 "Normal login/session middleware, real native/Web clients, actual process/disconnect recovery and official hosting.",
                 "Registered shared_contract target with normative legacy cases; whole lint and a stable current source baseline.",
                 "New generic document route inventory and current OpenAPI/source alignment during concurrent edits.",
                 "Release latency/throughput, bounded responses, shared-pool overload and peak resident memory.",
                 "Independent audit of client-provided atomic effects and durable task facts."],
    "data_boundary":"Only owned private QA fixtures on 127.0.0.1:15438/erp_shared_contract_test. Existing enterprises' selected sixty-table complete facts are preserved; no production deployment or historical repair.",
    "artifacts":artifacts}
destination.write_text(json.dumps(report,ensure_ascii=False,indent=2)+"\n",encoding="utf8")
print(json.dumps({"status":report["status"],"artifacts":len(artifacts),"tests":report["actual_tests"],"documentation":report["documentation"]},ensure_ascii=False))
