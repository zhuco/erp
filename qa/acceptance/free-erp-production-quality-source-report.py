"""Assemble immutable repair evidence without overstating current launch readiness."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-production-quality-source-"
destination = QA / f"{PREFIX}repair.json"
if destination.exists():
    raise SystemExit("refusing to replace historical repair evidence")


def read(name):
    return json.loads((QA / f"{PREFIX}{name}").read_text(encoding="utf-8-sig"))


def sha(path):
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


reports = {stage: read(f"{stage}-independent.json") for stage in [
    "original", "fixed-focused", "fixed-focused-timezone", "fixed-expanded", "fixed-final",
    "fixed-full", "full-retry", "full-retry-legacy-command", "direct", "direct-current",
]}
schema = read("migration-independent.json")
chain = read("chain-independent.json")
drafts = read("first-applied-ddl.json")
docs = read("document-delivery-report-after.json")
openapi = read("openapi-current-report.json")
paths = set()
for stage in reports:
    paths.add(QA / f"{PREFIX}{stage}-independent.json")
    for record in reports[stage].get("artifacts", {}).values():
        paths.add(ROOT / record["path"])
for name in ["migration-independent.json", "chain-independent.json", "first-applied-ddl.json",
             "openapi-current-report.json"]:
    paths.add(QA / f"{PREFIX}{name}")
for report in [chain, openapi]:
    for record in report.get("artifacts", {}).values():
        paths.add(ROOT / record["path"])
for draft in drafts["files"]:
    paths.add(ROOT / draft["archive"])
for pattern in [f"{PREFIX}document-*.json", f"{PREFIX}document-*.log", f"{PREFIX}document-*.txt",
                f"{PREFIX}format-*.log", f"{PREFIX}*-migration*.log",
                f"{PREFIX}*apply.sql"]:
    paths.update(QA.glob(pattern))
for name in ["clippy.log", "direct-first-verifier.py.txt", "original-inputs.json",
             "original-readback.sql.txt", "readback-first.sql.txt", "original-before.stderr.log",
             "original-before.jsonl", "migration-before.jsonl",
             "original-retry-before.stderr.log", "migration-before.stderr.log",
             "migration-retry-before.stderr.log", "readback.sql", "capture.py", "verify.py",
             "schema-verify.py", "chain-verify.py", "direct-verify.py", "report.py"]:
    path = QA / f"{PREFIX}{name}"
    if path.is_file():
        paths.add(path)
for pattern in [f"{PREFIX}original-*.rs.txt", f"{PREFIX}first-applied-*.txt"]:
    paths.update(QA.glob(pattern))
paths.add(QA / "free-erp-specification-docs-verify.py")
for name in docs["document_sha256"]:
    paths.add(ROOT / name)
for name in ["migrations/0291_inspection_source_bindings.sql",
             "migrations/0292_inspection_command_identity.sql",
             "migrations/0293_inspection_bounded_command_keys.sql",
             "crates/erp-repo/src/shared_contract/inspection_sources.rs",
             "crates/erp-repo/src/shared_contract/production_receipt_release.rs",
             "crates/erp-repo/src/shared_contract.rs", "crates/erp-api/src/manufacturing.rs",
             "crates/erp-api/src/procurement_handlers.rs",
             "crates/erp-api/tests/free_erp_acceptance.rs",
             "crates/erp-api/tests/free_erp_acceptance/production_quality_sources.rs",
             "crates/erp-api/tests/free_erp_acceptance/production_receipt_release.rs",
             "crates/erp-api/tests/shared_contract.rs"]:
    paths.add(ROOT / name)

source_windows = {}
for stage in ["original-retry", "fixed-focused", "fixed-expanded", "fixed-final", "fixed-full",
              "full-retry", "direct"]:
    manifests = {}
    for when in ["before", "after"]:
        label = "original-after" if stage == "original-retry" and when == "after" else f"{stage}-{when}"
        path = QA / f"free-erp-inventory-consumers-production-quality-source-{label}-source.json"
        paths.add(path)
        manifests[when] = json.loads(path.read_text(encoding="utf8"))["files"]
    before, after = manifests["before"], manifests["after"]
    current_drift = [name for name, value in after.items()
                     if not (ROOT / name).is_file() or sha(ROOT / name) != value]
    changed = [name for name in sorted(set(before) | set(after)) if before.get(name) != after.get(name)]
    source_windows[stage] = {"before_count": len(before), "after_count": len(after),
                             "changed_during_window": changed, "changed_since_after": current_drift,
                             "qualification": "PARTIAL" if changed or current_drift else "STABLE_INPUTS"}

raw = (QA / f"{PREFIX}clippy.log").read_bytes()
lint = raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
lint_packages = {package: int(count) for package, count in re.findall(
    r"could not compile `([^`]+)` \(lib\) due to (\d+) previous errors", lint)}
old_root = (QA / f"{PREFIX}original-crates_erp-repo_src_shared_contract.rs.txt").read_text(encoding="utf-8-sig")
current_root = (ROOT / "crates/erp-repo/src/shared_contract.rs").read_text(encoding="utf-8-sig")
legacy_signatures = {}
for name in ["freeze_inspection_template", "save_frozen_inspection_reading"]:
    original, current = [re.search(r"pub async fn " + name + r"\s*\((.*?)\)\s*->", source, re.S)
                         for source in [old_root, current_root]]
    legacy_signatures[name] = {"lint": "too_many_arguments", "original_signature_unchanged":
                              bool(original and current and re.sub(r"\s+", "", original[1]) ==
                                   re.sub(r"\s+", "", current[1]))}

milestones = {stage: {key: report[key] for key in ["status", "terminal_tests", "assertions",
               "protected_existing_enterprises", "complete_selected_tables", "errors",
               "build_failed_before_tests"] if key in report} for stage, report in reports.items()}
for stage, report in reports.items():
    milestones[stage]["case_count"] = len(report.get("cases", []))

format_log = QA / f"{PREFIX}format-delivery-retry-check.log"
format_raw = format_log.read_bytes()
format_text = format_raw.decode("utf16" if format_raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")
document_drift = [name for name, value in docs["document_sha256"].items() if sha(ROOT / name) != value]
document_source_drift = [name for name, value in docs["referenced_source_sha256"].items()
                         if sha(ROOT / name) != value]
artifact_records = {}
for path in sorted(paths):
    resolved = path.resolve()
    if not resolved.is_relative_to(ROOT.resolve()) or not resolved.is_file():
        raise SystemExit(f"invalid evidence path: {path}")
    relative = resolved.relative_to(ROOT.resolve()).as_posix()
    artifact_records[relative] = {"bytes": resolved.stat().st_size, "sha256": sha(resolved)}

report = {
    "created_at": datetime.now(timezone.utc).isoformat(),
    "status": "PARTIAL",
    "scope": "inspection source correctness, immutable frozen standards, quality approval and more detailed free manufacturing/trade ERP specification; full ERP and official hosted launch remain unfinished",
    "runtime_result": reports["full-retry-legacy-command"]["terminal_tests"],
    "new_source_test_functions": 28,
    "protected_identity": {"database": "erp_shared_contract_test", "role": "erp_grid_test",
                           "host": "127.0.0.1", "port": 15438},
    "milestones": milestones,
    "qa_migration": schema,
    "fresh_subset_chain": {key: chain[key] for key in ["status", "scope", "tests", "errors",
                                                      "source_before", "source_after"]},
    "first_actually_applied_draft_identity": drafts,
    "unreleased_ddl_and_forward_repair": "Unreleased 0291/0292 now use only existing (tenant_id,request_id) command keys and deferred exact full payload/effect/SUCCEEDED verification; 0293 repairs already-applied QA drafts. No canonical text truncation or fake sales work order. NOT VALID owner shape leaves historical validation pending.",
    "format": {"status": "PASS" if "QA controlled_format_exit=0" in format_text else "FAIL",
               "scope": "six affected leaf modules through the fixed build entry; shared roots were not globally formatted",
               "log": format_log.relative_to(ROOT).as_posix()},
    "strict_clippy": {"status": "FAIL", "actually_reported_package_error_counts": lint_packages,
                      "api_lint": "blocked by dependencies", "affected_original_many_argument_signatures": legacy_signatures,
                      "new_owned_leaf_diagnostics": [], "scope_qualification":
                      "The two touched root signatures remain prior lint debt; absence of new leaf diagnostics does not mean the root or API passed."},
    "documentation": {"status": docs["status"], "checks": docs["checks"], "errors": docs["errors"],
                      "exact_report": f"qa/acceptance/{PREFIX}document-delivery-report-after.json",
                      "document_changes_since_validation": document_drift,
                      "referenced_source_changes_since_validation": document_source_drift,
                      "qualification": "PARTIAL" if document_drift or document_source_drift else "CURRENT_REFERENCED_INPUTS",
                      "runtime_inferred_from_documentation": False},
    "openapi": {key: openapi[key] for key in ["status", "scope", "steps", "changed_during_static_window",
                                             "inputs_before", "inputs_after"]},
    "source_input_qualification": source_windows,
    "failed_verification_provenance": {
        "first_focused": "Same exact created_at instant appeared in UTC versus +08; corrected only timestamp normalization, all other fields remain exact; first FAIL retained.",
        "first_expanded": "25 passed / 3 failed: global long-text command indexes overflowed and sales readings incorrectly required a manufacturing owner. Fixed exact key/guard structure and actual nullable sales ownership; first FAIL and applied DDL retained.",
        "first_full_build": "Procurement function awaited without async; fixed declaration. No tests ran in that first attempt; terminal_tests remains null.",
        "first_full_readback": "Deliberately pre-seeded unknown-original-success command was incorrectly treated as a new rejection write; now exact original identity and no new release/approval results are independently required.",
        "first_direct_readback": "First verifier incorrectly expected one version for two new template requests and used mfg_qty_balances. Existing behavior produces versions 1/2; actual held quantity is in inventory_balances precision columns. Failed verifier bytes retained; business behavior was not changed to suit verification.",
        "documentation_checks": "Intermediate failures retain the invalid nested symbol spelling, GET table parser correction and concurrently added finance routes; final declared fields and registration are checked exactly.",
    },
    "remaining_gates": {
        "quality_source_release_performance": "NOT_RUN: seven candidate groups; no registered dedicated target; debug durations are not percentiles or performance acceptance",
        "serialized_snapshot_size_and_peak_memory": "NOT_RUN: field/count budgets are implemented, serialized-size admission and peak-memory measurements remain pending",
        "full_erp_new_and_upgrade_migration_chain": "NOT_RUN: rollback-only nine-parent subset and guarded QA forward changes do not certify the complete chain",
        "real_password_roles_seats_sessions": "NOT_RUN: synthetic QA JWT is not real account acceptance",
        "formal_native_and_wasm_quality_ui": "NOT_RUN",
        "official_hosted_production_launch": "NOT_RUN",
        "out_of_range_failed_reading_and_persisted_reviewer": "NOT_COMPLETE",
        "purchase_inspection_actual_source_and_reinspection_attachments": "NOT_COMPLETE",
        "remaining_case_parts": "Different receipts on one work order, sales freeze-versus-SHIP, old parent-version/partial-copy failures, missing/mismatched original command commit failures, and 20-way mixed approval/reading/release still need dedicated scenarios",
        "finance_source_closure": "New routes are documented statically; client supplied margin/cost amounts and source strings do not certify authoritative revenue, cost books or financial close",
    },
    "complete_erp_feature_ids": [],
    "goal_completion": "not achieved; Q01/Q02/M11 and hosted release remain open",
    "artifact_count": len(artifact_records),
    "artifacts": artifact_records,
}
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "runtime": report["runtime_result"],
                  "document_status": report["documentation"]["status"],
                  "document_qualification": report["documentation"]["qualification"],
                  "format": report["format"]["status"], "clippy": report["strict_clippy"]["status"],
                  "artifacts": len(artifact_records)}, ensure_ascii=False))
