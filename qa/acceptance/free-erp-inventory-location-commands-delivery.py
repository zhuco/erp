"""Qualify this delivery without replacing historical proof or running builds."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
parser = argparse.ArgumentParser()
parser.add_argument("--stage", choices=("initial", "refined"), default="initial")
args = parser.parse_args()
suffix = "" if args.stage == "initial" else "-refined"
destination = base / f"free-erp-inventory-location-commands-delivery{suffix}.json"
if destination.exists():
    raise SystemExit("refusing to replace a historical delivery qualification")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(name):
    return json.loads((base / name).read_text(encoding="utf8"))


proof_name = "free-erp-inventory-location-commands-repair.json"
docs_name = "free-erp-specification-docs.json"
proof = load(proof_name)
docs = load(docs_name)
errors = []
checked_evidence = set()


def check_evidence(value):
    if isinstance(value, dict):
        if "path" in value and "sha256" in value:
            relative = value["path"]
            path = root / relative
            checked_evidence.add(relative)
            if not path.is_file() or sha(path) != value["sha256"]:
                errors.append(f"historical evidence changed: {relative}")
        for child in value.values():
            check_evidence(child)
    elif isinstance(value, list):
        for child in value:
            check_evidence(child)


check_evidence(proof)
owned = proof["implementation"]["current_source_sha256"]
owned_drift = [p for p, digest in owned.items() if sha(root / p) != digest]
errors.extend(f"owned source changed since proof: {p}" for p in owned_drift)
document_drift = [p for p, digest in docs["document_sha256"].items()
                  if sha(root / p) != digest]
errors.extend(f"document changed since validation: {p}" for p in document_drift)
if docs["status"] != "PASS" or docs["errors"]:
    errors.append("documentation gate did not pass")
reference_drift = [p for p, digest in docs["referenced_source_sha256"].items()
                   if not (root / p).is_file() or sha(root / p) != digest]
tested = load("free-erp-inventory-consumers-location-full-after-source.json")["files"]
source_drift = [p for p, digest in tested.items()
                if not (root / p).is_file() or sha(root / p) != digest]

report = {
    "updated_at": datetime.now(timezone.utc).isoformat(),
    "status": "FAIL" if errors else "PARTIAL",
    "qualification_checks": {
        "status": "FAIL" if errors else "PASS",
        "historical_evidence_files_unchanged": len(checked_evidence),
        "owned_current_source_files_unchanged": len(owned) - len(owned_drift),
        "owned_changed_since_proof": owned_drift,
        "document_files_unchanged": len(docs["document_sha256"]) - len(document_drift),
        "documents_changed_since_gate": document_drift,
        "documentation_references_changed_since_gate": reference_drift,
        "errors": errors,
    },
    "historical_functional_proof": {"path": f"qa/acceptance/{proof_name}",
                                    "sha256": sha(base / proof_name)},
    "documentation_gate": {"path": f"qa/acceptance/{docs_name}",
                           "sha256": sha(base / docs_name), "status": docs["status"]},
    "complete_test_source_changes_since_run": source_drift,
    "limits": [
        "The complete source changed during the observed test run; its current qualification remains PARTIAL.",
        "The one post-test formatting correction is recorded in the historical proof; no business rerun is implied.",
        "Strict Clippy is FAIL; later shared OpenAPI changes are not certified by its historical passing gate.",
        "Release performance, real native/Web sessions, full migration/restore and hosted launch remain unpassed.",
    ],
    "runtime_tests_or_builds_executed_by_this_check": 0,
}
if args.stage == "refined":
    prior_name = "free-erp-inventory-location-commands-delivery.json"
    prior = load(prior_name)
    report["prior_qualification"] = {
        "path": f"qa/acceptance/{prior_name}", "sha256": sha(base / prior_name),
        "status": prior["status"], "errors": prior["qualification_checks"]["errors"],
        "reason_for_recheck": "Preserve the actual ROADMAP drift failure, then validate the current merged documents once.",
    }
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "checks": report["qualification_checks"],
                  "complete_source_drift_count": len(source_drift)}, ensure_ascii=False))
if errors:
    raise SystemExit(1)
