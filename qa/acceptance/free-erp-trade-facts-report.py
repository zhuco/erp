"""Collect current receipt reconciliation evidence without replacing history."""
from datetime import datetime,timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT/"qa/acceptance"
PREFIX = "free-erp-trade-facts-"
TARGET = BASE/(PREFIX+"repair.json")
if TARGET.exists():
    raise SystemExit("refusing to replace trade repair evidence")


def load(name):
    return json.loads((BASE/name).read_text(encoding="utf-8-sig"))


def sha(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for raw in iter(lambda:stream.read(1048576),b""):
            value.update(raw)
    return value.hexdigest()


def tests(name):
    text = (BASE/name).read_text(encoding="utf-8-sig")
    rows = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in ([\d.]+)s",text)
    if not rows:
        raise SystemExit("actual terminal result required: "+name)
    # Domain cargo test also has a separate zero-example doctest result.
    status,passed,failed,ignored,measured,filtered,seconds = max(rows,key=lambda row:int(row[1])+int(row[2]))
    return {"status":"PASS" if status=="ok" else "FAIL","passed":int(passed),"failed":int(failed),
            "ignored":int(ignored),"measured":int(measured),"filtered_out":int(filtered),"seconds":float(seconds),"log":name}


full = load(PREFIX+"full-independent.json")
api = load(PREFIX+"authority-independent.json")
first = load(PREFIX+"api-first-independent.json")
run = load(PREFIX+"full-status.json")
quality = load(PREFIX+"quality-status.json")
contract = load(PREFIX+"openapi.json")
docs = load(PREFIX+"docs-delivery.json")
phases = {key:tests(PREFIX+name) for key,name in {
    "original_non_sample":"original-test.log","complete_domain":"domain-test.log",
    "first_three_real_account":"api-test.log","final_five_real_account":"authority-api-test.log",
    "legacy_original_sample":"legacy-receipt-test.log","complete_registered":"full-test.log",
}.items()}
if run["state"]!="COMPLETED" or run["functional_exit"]!=0 or run["independent_exit"]!=0 or quality["state"]!="COMPLETED":
    raise SystemExit("current controlled actions are not successfully terminal")
if any(report["errors"] for report in [api,first,full]) or contract["status"]!="PASS" or docs["status"]!="PASS":
    raise SystemExit("unexplained current fact, contract or document errors")
whole = phases["complete_registered"]
if whole["status"]!="PASS" or whole["ignored"] or whole["filtered_out"]:
    raise SystemExit("complete registered baseline not actually passed")
source_names = [
    "crates/erp-domain/src/trade.rs","crates/erp-repo/src/shared_contract.rs",
    "crates/erp-repo/src/shared_contract/receipt_offsets.rs","crates/erp-api/src/onboarding.rs",
    "crates/erp-api/tests/free_erp_acceptance.rs","crates/erp-api/tests/free_erp_acceptance/trade_facts.rs",
    "crates/erp-api/tests/free_erp_acceptance/login_sessions.rs","migrations/0296_receipt_offset_read_indexes.sql",
    "openapi/src/components.yaml","openapi/src/paths.yaml","openapi/erp_api_v1.yaml",
]
artifacts = [PREFIX+name for name in [
    "original-format.log","original-test.log","domain-format.log","repo-format.log","domain-test.log","domain-clippy.log",
    "api-format.log","api-test.log","api-first-independent.json","authority-api-format.log","authority-repo-format.log",
    "authority-api-test.log","authority-independent.json","legacy-receipt-test.log","index-apply.log",
    "openapi-first.json","openapi-gate.log","openapi.json","full-test.log","full-independent.json","full-status.json",
    "full-events.jsonl","quality-status.json","format-final.log","current-domain-clippy.log",
    "current-repo-clippy.log","current-api-clippy.log","docs-first.json","docs-final.json","docs-delivery.json",
    "readback.sql","timestamp-catalog.json","capture.py","verify.py","openapi-verify.mjs","report.py",
    "full-run.ps1","quality-run.ps1",
]]
artifacts += ["free-erp-inventory-consumers-trade-facts-"+stage+"-source.json" for stage in [
    "api-before","api-after","authority-before","authority-after","full-before","full-after",
]]
report = {
    "utc":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL",
    "scope":"Remove fixture-only receipt read rules, reconcile source stock/AP in one statement, enforce current read identity and expose typed failure; ongoing ERP goal",
    "controlled_database":{"name":"erp_shared_contract_test","role":"erp_grid_test","host":"127.0.0.1","port":15438,"selected_tables":73},
    "functional_results":phases,"independent_first_three":first,"independent_final_five":api,"independent_full":full,
    "run":run,"quality":quality,"contract":contract,"documentation":{"status":docs["status"],"checks":docs["checks"]},
    "repairs":[
        "Four named domain fact records replace long positional lists; direct callers use explicit unit/cost/source field names",
        "Receipt quantity/stock/actual posting conservation replaces fixed six-arrived/two-returned fixture equality",
        "One SQL statement binds the original tenant receipt/quality, source stock, supplier returns and actual CNY AP postings",
        "Actual per-release rounded postings remain authoritative; do not recompute/overwrite source gross from combined quantity",
        "Known zero has real posting rows; absent source does not become a zero result",
        "Stored current user/session generation, orders permission and warehouse grant participate in the receipt statement",
        "Invalid/missing/scope/reconciliation/storage failures are explicit; genuine SQL fault returns 500 with no business writes",
        "Receipt and posting lookup indexes added and read back only in protected QA; production/full migrations not qualified",
    ],
    "preserved_failures":[
        "Original mathematically balanced seven/five/two domain case returned InvalidText; original log retained",
        "First source-schema verifier addressed the fragment without its schemas root; error record retained and corrected without altering runtime outcomes",
    ],
    "boundaries":[
        "API fixtures seed owned stored source facts and use real password/server-issued identity; full procurement/release/return write workflow is not inferred",
        "Revocation setups alter only owned QA authorization rows; no claim of all RBAC mutation HTTP or write-lock races",
        "Source SQL fault uses only an owned schema/view and preserves public business data",
        "Debug durations and index existence are not P95/P99, Release or RSS qualification",
    ],
    "remaining_gates":{
        "domain_strict_clippy":"PASS","repo_api_strict_clippy":"FAIL",
        "complete_procurement_and_cost_writes":"PARTIAL","all_domain_authority_write_wait_races":"NOT_COMPLETE",
        "full_migration_chain_and_production_index_upgrade":"NOT_RUN",
        "real_native_wasm_business":"NOT_RUN","release_performance_and_resources":"NOT_RUN","official_hosting_release":"NOT_RUN",
    },
    "current_related_source_sha256":{name:sha(ROOT/name) for name in source_names},
    "artifact_sha256":{"qa/acceptance/"+name:sha(BASE/name) for name in artifacts},
}
with TARGET.open("x",encoding="utf8") as output:
    output.write(json.dumps(report,ensure_ascii=False,indent=2)+"\n")
print(json.dumps({"status":report["status"],"full":whole,"protected_older_enterprises":full["older_enterprises"],
                  "checks":full["checks"],"source_input_drift":full["source_input_drift"],
                  "quality":quality["results"]},ensure_ascii=False))
