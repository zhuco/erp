"""Assemble immutable repair evidence; read sources and QA artifacts without changing data."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
destination = base / "free-erp-production-release-repair.json"
if destination.exists():
    raise SystemExit("refusing to replace historical repair evidence")

def read_json(name):
    return json.loads((base / name).read_text(encoding="utf-8-sig"))

def digest_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()

def read_log(name):
    raw = (base / name).read_bytes()
    return raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")

def compare_manifests(stage, before_name, after_name):
    before = read_json(f"free-erp-inventory-consumers-production-release-{before_name}-source.json")["files"]
    after = read_json(f"free-erp-inventory-consumers-production-release-{after_name}-source.json")["files"]
    changed = sorted(name for name in set(before) | set(after) if before.get(name) != after.get(name))
    current = {name: digest_file(root / name) if (root / name).is_file() else None for name in after}
    drift = sorted(name for name in after if after[name] != current[name])
    return dict(stage=stage, before_count=len(before), after_count=len(after),
                changed_during_window=changed, changed_since_after=drift,
                qualification="PARTIAL" if changed or drift else "STABLE_RECORDED_INPUTS",
                scope="complete enumerated Rust/schema/lock/build-entry/OpenAPI inputs; no release or deployment certification")

def full_set_preservation(stage):
    before = {}
    seen = set()
    changed = []
    identities = []
    for when in ["before", "after"]:
        with (base / f"free-erp-production-release-{stage}-{when}.jsonl").open(encoding="utf8") as stream:
            header = json.loads(next(stream))
            identities.append(tuple(header[key] for key in ["database", "role", "host", "port"]))
            for line in stream:
                row = json.loads(line)
                tenant = row["tenant_id"]
                digest = hashlib.sha256(json.dumps(row, sort_keys=True, ensure_ascii=False,
                                                   separators=(",", ":")).encode()).hexdigest()
                if when == "before":
                    before[tenant] = digest
                else:
                    seen.add(tenant)
                    if tenant in before and before[tenant] != digest:
                        changed.append(tenant)
    missing = sorted(set(before) - seen)
    protected = all(identity == ("erp_shared_contract_test", "erp_grid_test", "127.0.0.1", 15438) for identity in identities)
    return dict(status="PASS" if protected and not changed and not missing else "FAIL",
                protected_identity_verified=protected, existing_enterprises=len(before),
                complete_selected_tables=45, changed_existing_enterprises=changed,
                missing_existing_enterprises=missing)

reports = {stage: read_json(f"free-erp-production-release-{stage}-independent.json") for stage in
           ["original", "fixed-focused", "fixed-expanded", "fixed-full", "direct"]}
docs = read_json("free-erp-production-release-docs-final.json")
contract = read_json("free-erp-production-release-openapi-final.json")
errors = []
for stage in ["original", "fixed-focused", "fixed-full", "direct"]:
    if reports[stage]["status"] != "PASS":
        errors.append(f"Independent evidence did not pass: {stage}")
if reports["original"]["terminal_tests"]["failed"] != 9:
    errors.append("Original nine failures must remain recorded")
if tuple(reports["fixed-full"]["terminal_tests"][field] for field in ["passed", "failed", "ignored", "filtered"]) != (263, 0, 0, 0):
    errors.append("Actual full registered target differs from 263/0/0/0")
if len(reports["fixed-full"]["cases"]) != 24:
    errors.append("All 24 new release cases must be independently read back")
for name, report in [("documentation", docs), ("current OpenAPI", contract)]:
    if report["status"] != "PASS":
        errors.append(f"Static {name} check did not pass")
preservation = {stage: full_set_preservation(stage) for stage in ["fixed-full", "direct"]}
for stage, report in preservation.items():
    if report["status"] != "PASS":
        errors.append(f"Complete set preservation failed: {stage}")
clippy = read_log("free-erp-production-release-clippy.log")
lint_errors = dict(re.findall(r"could not compile `([^`]+)` \(lib\) due to (\d+) previous errors", clippy))
lint_errors = {name: int(count) for name, count in lint_errors.items()}
format_log = read_log("free-erp-production-release-format-final.log")
if "Diff in " in format_log or "Rustfmt failed" in format_log:
    errors.append("Final affected-leaf formatting failed")
archived_inputs = read_json("free-erp-production-release-original-inputs.json")
for entry in archived_inputs["files"]:
    if digest_file(root / entry["archive"]) != entry["sha256"]:
        errors.append(f"Original source archive changed: {entry['archive']}")
source_qualification = {
    "original": compare_manifests("original compiled inputs", "original-precompile", "original-after"),
    "focused": compare_manifests("fixed focused", "fixed-focused-before", "fixed-focused-after"),
    "expanded": compare_manifests("fixed expanded first attempt", "fixed-expanded-before", "fixed-expanded-after"),
    "full": compare_manifests("fixed full", "fixed-full-before", "fixed-full-after"),
    "direct": compare_manifests("existing direct HTTP target", "direct-before", "direct-after"),
}
key_files = ["crates/erp-repo/src/shared_contract/production_receipt_release.rs",
             "crates/erp-repo/src/shared_contract.rs", "crates/erp-api/src/auth_context.rs",
             "crates/erp-api/src/manufacturing.rs", "crates/erp-api/tests/free_erp_acceptance.rs",
             "crates/erp-api/tests/free_erp_acceptance/production_receipt_release.rs",
             "migrations/0290_production_receipt_release_sources.sql"]
key_input_bindings = {}
for name in key_files:
    stage_hashes = {stage: read_json(f"free-erp-inventory-consumers-production-release-{stage}-source.json")["files"].get(name)
                    for stage in ["fixed-full-before", "fixed-full-after", "direct-before", "direct-after", "delivery"]}
    current = digest_file(root / name)
    key_input_bindings[name] = dict(sha256=stage_hashes, current_sha256=current,
                                  full_window_stable=stage_hashes["fixed-full-before"] == stage_hashes["fixed-full-after"],
                                  current_matches_full_after=current == stage_hashes["fixed-full-after"])
document_drift = [name for name, digest in docs["document_sha256"].items() if digest_file(root / name) != digest]
contract_drift = [name for name, digest in contract["source_sha256"].items() if digest_file(root / name) != digest]
artifacts = sorted(set(base.glob("free-erp-production-release-*"))
                   | set(base.glob("free-erp-inventory-consumers-production-release-*-source.json")))
artifact_hashes = {path.relative_to(root).as_posix(): dict(bytes=path.stat().st_size, sha256=digest_file(path))
                   for path in artifacts if path.is_file() and path != destination}
report = {
    "created_at": datetime.now(timezone.utc).isoformat(),
    "status": "FAIL" if errors else "PARTIAL",
    "scope": "Production receipt whole-release repair and detailed development specification. Complete free ERP and launch remain unfinished.",
    "runtime_result": reports["fixed-full"]["terminal_tests"],
    "new_scenario_count": len(reports["fixed-full"]["cases"]),
    "protected_identity": dict(database="erp_shared_contract_test", role="erp_grid_test", host="127.0.0.1", port=15438),
    "milestones": {name: {key: report[key] for key in ["status", "terminal_tests", "protected_existing_enterprises", "complete_selected_tables", "errors"]}
                   for name, report in reports.items()},
    "first_expanded_failure": "One setup attempted to overwrite a proven precise ledger and was correctly refused by an existing guard; replaced with a separate unproven legacy-balance scenario without disabling guards.",
    "first_expanded_verifier_correction": "The original verifier also compared two intentional 500 fault responses with later successful snapshots; failed output retained, subsequent verifier separates rejected/fault responses from successful original results.",
    "complete_existing_sets_preserved": preservation,
    "source_input_qualification": source_qualification, "key_input_bindings": key_input_bindings,
    "actual_build_entry": "scripts/Invoke-ErpBuild.ps1 with existing fixed cache, lock and admission checks",
    "actual_registered_targets": [
        "-Action test -Scope api -IntegrationTest free_erp_acceptance",
        "-Action test -Scope api -IntegrationTest shared_contract -IntegrationFilter production_release_http_race_read_back"],
    "format": dict(status="PASS", exit_code=0, affected_leaf_files=[name for name in key_files
                    if name in ["crates/erp-repo/src/shared_contract/production_receipt_release.rs", "crates/erp-api/src/auth_context.rs",
                                "crates/erp-api/src/manufacturing.rs", "crates/erp-api/tests/free_erp_acceptance/production_receipt_release.rs"]],
                   qualification="Recorded fixed-entry check; subsequent concurrent edits are not certified"),
    "strict_clippy": dict(status="FAIL", entry_wrapper_exit_code=1, underlying_cargo_exit_code=101,
                          dependency_error_counts=lint_errors, api_lint_reached=False,
                          qualification="No suppression or bypass. Absence of a new-leaf diagnostic does not certify dependency or API lint."),
    "documentation": dict(status=docs["status"], checks=docs["checks"], document_sha256=docs["document_sha256"],
                          changes_since_validation=document_drift,
                          input_qualification="PARTIAL" if document_drift else "STABLE_RECORDED_DOCUMENTS"),
    "openapi": dict(status=contract["status"], source_sha256=contract["source_sha256"],
                    changes_since_validation=contract_drift,
                    qualification="PARTIAL" if contract_drift else "STABLE_RECORDED_STATIC_CONTRACT"),
    "migration": dict(name="0290_production_receipt_release_sources.sql", applied="PROTECTED_QA_ONLY",
                      historical_rows_backfilled=False, original_result_inferred=False,
                      production_upgrade_and_restore="NOT_RUN"),
    "observed_behavior": [
        "Server checks original held physical source, effective report, exact ledgers and required frozen readings before release.",
        "Current ACTIVE user and original warehouse scope are locked through commit; replay also checks current authority.",
        "One release has two real owned HOLD/GOOD effects, net on-hand zero and frozen reduced by the held quantity.",
        "Mandatory operation/audit records, original result snapshot, ledger effects and global receipt commit together.",
        "Actual PostgreSQL blockers prove both 20-way groups; all responses are drained and independently checked.",
        "Actual operation/audit INSERT faults roll back the full command; original ID retries and replays without a second release."],
    "not_completed": [
        "Initial receipt original-result snapshot, final-operation and product/SKU binding.",
        "Partial releases and original released/remaining quantity contract.",
        "Verified UUID batch mapping and original value/cost, serial and subsequent production source links.",
        "Necessary inventory notifications/Outbox, snapshot byte bounds and measured peak-memory budgets.",
        "Full quality HTTP workflow, real login/seat/session recovery and native/Web DataGrid pages.",
        "Strict dependency/API Clippy baseline and complete current-source build after concurrent changes.",
        "Registered Release receipt performance workloads, physical resource measurements and production migration/restore.",
        "Official hosted public service, paid entitlements and complete M11/Q02/free launch acceptance."],
    "performance": dict(status="NOT_RUN", general_metrics=22, receipt_specific_groups=7,
                        debug_elapsed_is_not_release_latency=True),
    "original_source_archive_sha256": archived_inputs,
    "artifact_sha256": artifact_hashes, "errors": errors,
}
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False)+"\n", encoding="utf8")
print(json.dumps({"status": report["status"], "runtime": report["runtime_result"],
                  "complete_existing_sets_preserved": preservation, "clippy": lint_errors,
                  "documents": docs["status"], "openapi": contract["status"],
                  "artifacts": len(artifact_hashes), "errors": errors}, ensure_ascii=False))
raise SystemExit(bool(errors))
