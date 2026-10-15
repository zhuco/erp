"""Preserve delivery-time source drift without rewriting historical repair proof."""

import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path

root = Path(__file__).resolve().parents[2]
base = root / "qa" / "acceptance"


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(name: str) -> dict:
    return json.loads((base / name).read_text(encoding="utf-8-sig"))


def evidence(name: str) -> dict:
    return {"path": (base / name).relative_to(root).as_posix(), "sha256": sha(base / name)}


proof_name = "free-erp-inventory-commands-repair.json"
contracts_name = "free-erp-inventory-commands-final-openapi-inputs.json"
docs_name = "free-erp-specification-docs.json"
archive_name = "free-erp-specification-docs-before-command-delivery.json"
destination = base / "free-erp-inventory-commands-delivery-qualification.json"
if destination.exists() or (base / archive_name).exists():
    raise SystemExit("refusing to overwrite delivery qualification or documentation snapshot")

proof = read(proof_name)
contracts = read(contracts_name)
docs = read(docs_name)
with (base / archive_name).open("xb") as archive:
    archive.write((base / docs_name).read_bytes())

owned_after = {name: sha(root / name) for name in proof["implementation"]["source_sha256"]}
owned_changed = [name for name, digest in proof["implementation"]["source_sha256"].items()
                 if owned_after[name] != digest]
contract_after = {name: sha(root / name) for name in contracts["after"]}
contract_changed = [name for name, digest in contracts["after"].items()
                    if contract_after[name] != digest]

report = {
    "status": "PARTIAL",
    "updated_at": datetime.now(timezone.utc).isoformat(),
    "scope": "Delivery identity qualification only; no new runtime, build, OpenAPI regeneration or load test.",
    "historical_repair_evidence": evidence(proof_name),
    "delivery_identity_check": {
        "status": "FAIL" if owned_changed or contract_changed else "PASS",
        "owned_source_changed_since_repair": owned_changed,
        "openapi_changed_since_final_gate": contract_changed,
        "owned_source_sha256": owned_after,
        "openapi_sha256": contract_after,
        "meaning": "Drift fails binding of historical gate results to current files; it does not infer a functional failure.",
    },
    "whole_current_source_certification": proof["source_certification"]["commands-full"]["status"],
    "historical_functional_result": proof["registered_native"],
    "historical_independent_result": {
        "status": proof["independent"]["status"], "checks": proof["independent"]["checks"],
        "evidence": evidence("free-erp-inventory-commands-independent.json"),
    },
    "historical_openapi_result": proof["quality_gates"]["openapi"],
    "current_openapi_certification": "PARTIAL" if contract_changed else proof["quality_gates"]["openapi"]["status"],
    "openapi_gate_after_delivery_drift": {
        "status": "NOT_RUN", "reason": "Shared source and bundle changed after the captured gate; retain historical evidence and record drift without rewriting concurrent work.",
    },
    "strict_clippy": proof["quality_gates"]["strict_backend_clippy"],
    "documentation_before_delivery_note": {
        "status": docs["status"], "checks": docs["checks"], "evidence": evidence(archive_name),
    },
    "final_documentation_report": {
        "path": (base / docs_name).relative_to(root).as_posix(),
        "note": "Latest documentation report is checked separately after adding the delivery note; this field does not assert its future result.",
    },
    "limits": [
        "Historical evidence files are unchanged; the failed delivery identity check remains explicit.",
        "No new Rust build, data mutation, cache cleanup or interruption of concurrent work.",
        "Real native/Web workflows, hosted recovery and Release percentile/resource targets remain unqualified.",
    ],
}
with destination.open("x", encoding="utf-8", newline="\n") as output:
    json.dump(report, output, ensure_ascii=False, indent=2)
    output.write("\n")
print(json.dumps({"status": report["status"], "delivery_identity_check": report["delivery_identity_check"]["status"],
                  "owned_changed": owned_changed, "openapi_changed": contract_changed,
                  "documentation_before_note": docs["status"]}, ensure_ascii=False))
