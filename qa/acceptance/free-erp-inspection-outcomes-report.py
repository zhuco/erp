"""Bind immutable runtime, independent facts, contracts and document evidence."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-inspection-outcomes-"
destination = QA / f"{PREFIX}repair.json"
if destination.exists():
    raise SystemExit("refusing historical aggregate overwrite")


def load(name):
    return json.loads((QA / f"{PREFIX}{name}.json").read_text(encoding="utf8"))


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def log(name):
    raw = (QA / f"{PREFIX}{name}.log").read_bytes()
    return raw.decode("utf16" if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else "utf-8-sig")


original = load("original-encoding-corrected-independent")
focused = load("fixed-focused-independent")
first_expanded = load("fixed-expanded-strengthened-independent")
expanded = load("fixed-permissions-independent")
full = load("fixed-full-independent")
direct = load("direct-reading-independent")
docs = load("docs-delivery")
openapi = load("openapi-full-current")
schema = load("migration-retry-independent")
rollback = load("first-migration-rollback-independent")
errors = []
for name, report in [("expanded", expanded), ("full", full), ("direct", direct),
                     ("documents", docs), ("openapi", openapi), ("migration", schema), ("migration rollback", rollback)]:
    if report["status"] != "PASS":
        errors.append(f"Unpassed required evidence: {name}")
if full["tests"] != {**full["tests"], "status":"ok", "passed":318, "failed":0, "ignored":0, "filtered":0}:
    errors.append("Full registered baseline does not contain 318 real passing cases")
if "QA format_exit=0" not in log("final-format-check"):
    errors.append("Affected leaf format check did not actually pass")
ddl = ROOT / "migrations/0294_inspection_reading_outcomes.sql"
if sha(ddl) != sha(QA / f"{PREFIX}applied-0294.sql.txt"):
    errors.append("Current 0294 differs from actually applied QA DDL bytes")

source_windows = []
for before_label, after_label in [
    ("original-before", "original-after"), ("fixed-focused-before", "fixed-focused-after"),
    ("fixed-expanded-before", "fixed-expanded-after"), ("fixed-permissions-before", "fixed-permissions-after"),
    ("fixed-full-before", "fixed-full-after"), ("direct-reading-retry-before", "direct-reading-after")]:
    paths = [QA / f"free-erp-inventory-consumers-inspection-outcomes-{label}-source.json" for label in [before_label,after_label]]
    manifests = [json.loads(path.read_text(encoding="utf8")) for path in paths]
    a,b = [report["files"] for report in manifests]
    changed = [name for name in sorted(set(a)|set(b)) if a.get(name) != b.get(name)]
    source_windows.append(dict(before=paths[0].relative_to(ROOT).as_posix(), after=paths[1].relative_to(ROOT).as_posix(),
                               input_counts=[len(a),len(b)], changed_inputs=changed,
                               qualification="PARTIAL" if changed else "STABLE_RECORDED_INPUTS"))

doc_drift = [name for name,digest in docs["document_sha256"].items() if sha(ROOT / name) != digest]
source_drift = [name for name,digest in docs["referenced_source_sha256"].items() if sha(ROOT / name) != digest]
lint = log("clippy-backend")
package_lints = re.findall(r"could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors", lint)
large_hashes = {}
for path in QA.glob(f"{PREFIX}*independent.json"):
    report = json.loads(path.read_text(encoding="utf8"))
    for entry in report.get("inputs", {}).values():
        if isinstance(entry, dict) and "path" in entry and "sha256" in entry:
            large_hashes[entry["path"]] = entry["sha256"]
artifacts = {}
paths = set(QA.glob(f"{PREFIX}*")) | set(QA.glob("free-erp-inventory-consumers-inspection-outcomes-*-source.json"))
for path in sorted(paths):
    if not path.is_file() or path == destination:
        continue
    name = path.relative_to(ROOT).as_posix()
    artifacts[name] = {"bytes":path.stat().st_size, "sha256":large_hashes.get(name) or sha(path)}

report = {
    "created_at":datetime.now(timezone.utc).isoformat(), "status":"FAIL" if errors else "PARTIAL",
    "scope":"Real failed/missing measurements, explicit reinspection, actual account review, immutable original results, bounded read-only history and physical release. Broad free manufacturing/trade ERP goal remains open.",
    "original":{k:original[k] for k in ["status", "tests", "protected_existing_enterprises", "independent_assertions"]},
    "first_expanded_failure":{k:first_expanded[k] for k in ["status", "tests", "errors"]},
    "fixed_focused":{k:focused[k] for k in ["status", "tests", "independent_assertions"]},
    "expanded":{k:expanded[k] for k in ["status", "tests", "fixture_count", "protected_existing_enterprises", "complete_selected_tables", "independent_assertions"]},
    "full":{k:full[k] for k in ["status", "tests", "fixture_count", "protected_existing_enterprises", "complete_selected_tables", "independent_assertions"]},
    "direct_reading":{k:direct[k] for k in ["status", "tests", "protected_existing_enterprises", "selected_complete_tables", "independent_assertions"]},
    "independent_scope":"Complete older 54-table facts and this batch's 29 new enterprises; other registered runtime cases retain their actual test assertions, not newly invented independent certification.",
    "migration":{"status":schema["status"], "rollback_status":rollback["status"], "current_matches_actual_applied_0294":sha(ddl)==sha(QA / f"{PREFIX}applied-0294.sql.txt"),
                 "scope":"Protected QA 0294 transaction only; first missing users composite key failure fully rolled back; no historical measurement/review backfill; whole fresh/upgrade chains and production are NOT_RUN"},
    "format":{"status":"PASS" if "QA format_exit=0" in log("final-format-check") else "FAIL", "scope":"Nine affected Rust leaf modules through fixed build entry; shared roots not globally formatted"},
    "strict_clippy":{"status":"FAIL" if "QA clippy_exit=1" in lint else "PASS", "actually_reported_errors":package_lints,
                     "api_lint":"Blocked by dependencies; not passed", "new_repo_leaf_diagnostics_reported":"shared_contract/inspection_outcomes.rs" in lint,
                     "qualification":"No reported new leaf diagnosis does not certify unvisited API or any failing package"},
    "openapi":{"status":openapi["status"], "checked_successful_http_bodies":openapi["checked_successful_http_bodies"], "scope":openapi["scope"]},
    "documents":{"status":docs["status"], "checks":docs["checks"], "document_drift_since_check":doc_drift,
                 "referenced_source_drift_since_check":source_drift, "qualification":"PARTIAL" if doc_drift or source_drift else "CURRENT_REFERENCED_INPUTS",
                 "current_exact_report":"qa/acceptance/free-erp-inspection-outcomes-docs-delivery.json", "runtime_inferred_from_document_check":False},
    "source_windows":source_windows,
    "failed_provenance":{
        "migration":"First actual 0294 failed for absent users composite key and rolled back; initial migration expectation report FAIL and corrected rollback-specific PASS both preserved",
        "original_readback":"First codec spelling error prevented verification; archived bytes retained and same actual logs/facts decoded correctly afterward",
        "expanded":"24 passed/3 failed: unintended manufacturing-write prerequisite for quality.read and sales test using SUBMIT instead of REVISE; no changed original logs or weakened business invariants",
        "direct_compile":"First direct build did not execute tests because a concurrent export test moved jwt before later encode; concurrent source already contained the clone correction when inspected. This task did not claim that external repair",
        "openapi":"First dedicated contract check failed on missing actual reading 400 status; contract corrected then 95 and 99 actual successful bodies passed",
        "documents":"Earlier snapshots failed as parallel report/export routes appeared; actual routes and DTO fields were explicitly documented and checked, not silently skipped"},
    "remaining":{"overall_erp":"100-feature product, formal trade/manufacturing closure and official-hosted launch remain open",
                 "quality":"Multisample completeness, units/tolerances/calibration, attachments, purchase sources, normal role/session authentication, formal native/Web UI and bounded continuation beyond twenty attempts/1000 evidence rows",
                 "release_performance":"NOT_RUN; quality-specific Release target not registered; debug total times and per-request QA pools do not establish percentiles, production shared-pool backpressure or public-network capacity",
                 "whole_migration_and_production":"NOT_RUN"},
    "artifacts":artifacts, "errors":errors,
}
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status":report["status"], "artifacts":len(artifacts), "documents":docs["status"],
                  "document_drift":doc_drift, "source_drift_count":len(source_drift), "errors":errors}))
raise SystemExit(bool(errors))
