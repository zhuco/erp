"""Read-only final identity checks; no build, SQL, or historical replacement."""
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
target = base / f"free-erp-inventory-location-outbound-delivery{suffix}.json"
if target.exists():
    raise SystemExit("preserve historical delivery qualification")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(name):
    return json.loads((base / name).read_text(encoding="utf8"))


proof_name = "free-erp-inventory-location-outbound-repair.json"
docs_name = "free-erp-specification-docs.json"
manifest_name = "free-erp-inventory-consumers-location-outbound-delivery-source.json"
proof = read(proof_name)
docs = read(docs_name)
current = read(manifest_name)["files"]
tested = read("free-erp-inventory-consumers-location-outbound-full-after-source.json")["files"]
errors = []
references = set()


def verify_references(value):
    if isinstance(value, dict):
        if "path" in value and "sha256" in value:
            path = root / value["path"]
            references.add(value["path"])
            if not path.is_file() or sha(path) != value["sha256"]:
                errors.append(f"historical evidence changed: {value['path']}")
        for child in value.values():
            verify_references(child)
    elif isinstance(value, list):
        for child in value:
            verify_references(child)


verify_references(proof)
owned_changes = [p for p, digest in proof["current_owned_source_sha256"].items()
                 if sha(root / p) != digest]
doc_changes = [p for p, digest in docs["document_sha256"].items() if sha(root / p) != digest]
errors.extend(f"owned source changed: {p}" for p in owned_changes)
errors.extend(f"document changed since validation: {p}" for p in doc_changes)
if docs["status"] != "PASS":
    errors.append("document validation failed")
reference_changes = [p for p, digest in docs["referenced_source_sha256"].items()
                     if not (root / p).is_file() or sha(root / p) != digest]
complete_source_changes = [p for p in sorted(set(tested) | set(current))
                           if tested.get(p) != current.get(p)]
report = {
    "updated_at": datetime.now(timezone.utc).isoformat(), "status": "FAIL" if errors else "PARTIAL",
    "qualification_checks": {
        "status": "FAIL" if errors else "PASS", "historical_evidence_files_unchanged": len(references),
        "owned_source_files_unchanged": len(proof["current_owned_source_sha256"]) - len(owned_changes),
        "owned_source_changes": owned_changes, "document_files_unchanged": len(docs["document_sha256"]) - len(doc_changes),
        "document_changes": doc_changes, "referenced_sources_changed_since_doc_gate": reference_changes,
        "errors": errors,
    },
    "repair_evidence": {"path": f"qa/acceptance/{proof_name}", "sha256": sha(base / proof_name)},
    "documentation": {"path": f"qa/acceptance/{docs_name}", "sha256": sha(base / docs_name), "status": docs["status"]},
    "delivery_source_manifest": {"path": f"qa/acceptance/{manifest_name}", "sha256": sha(base / manifest_name)},
    "complete_source_changes_since_observed_run": complete_source_changes,
    "limits": "Observed tests and independent facts pass, but complete source changed in its run window, strict Clippy failed, later OpenAPI is unqualified, and real native/Web, Release, migration/restore and hosted launch remain unpassed.",
    "runtime_or_build_actions_executed_by_this_check": 0,
}
if args.stage == "refined":
    previous_name = "free-erp-inventory-location-outbound-delivery.json"
    previous = read(previous_name)
    report["previous_qualification"] = {
        "path": f"qa/acceptance/{previous_name}", "sha256": sha(base / previous_name),
        "status": previous["status"], "errors": previous["qualification_checks"]["errors"],
        "reason_for_recheck": "Preserve actual post-gate ROADMAP drift; revalidate current merged documents once.",
    }
target.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "checks": report["qualification_checks"],
                  "complete_source_drift": complete_source_changes}, ensure_ascii=False))
if errors:
    raise SystemExit(1)
