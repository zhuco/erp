"""Bundle and check current OpenAPI inputs, retaining exact static evidence."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
QA = ROOT / "qa/acceptance"
PREFIX = "free-erp-production-quality-source-openapi-current-"
destination = QA / f"{PREFIX}report.json"
names = ["openapi/src/openapi.yaml", "openapi/src/components.yaml", "openapi/src/paths.yaml",
         "crates/erp-api/src/manufacturing.rs"]
steps = ["openapi_bundle", "verify_openapi_gate", "verify_openapi_gate_v2"]
logs = {step: QA / f"{PREFIX}{step}.log" for step in steps}
copies = {name: QA / f"{PREFIX}before-{name.replace('/', '_')}.txt" for name in names}
if any(path.exists() for path in [destination, *logs.values(), *copies.values()]):
    raise SystemExit("refusing to replace historical OpenAPI evidence")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


before = {}
for name in names:
    raw = (ROOT / name).read_bytes()
    before[name] = hashlib.sha256(raw).hexdigest()
    with copies[name].open("xb") as stream:
        stream.write(raw)
results = {}
for step in steps:
    with logs[step].open("xb") as stream:
        run = subprocess.run(["C:/Program Files/nodejs/node.exe", f"scripts/{step}.mjs"],
                             cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT, check=False)
    results[step] = {"exit_code": run.returncode, "log": logs[step].relative_to(ROOT).as_posix()}
    if step == "openapi_bundle" and run.returncode != 0:
        break
after = {name: sha(ROOT / name) for name in names}
changed = [name for name in names if before[name] != after[name]]
all_gates = len(results) == 3 and all(row["exit_code"] == 0 for row in results.values())
report = {
    "created_at": datetime.now(timezone.utc).isoformat(),
    "status": "FAIL" if not all_gates else "PARTIAL" if changed else "PASS",
    "scope": "current source bundle and two static OpenAPI gates; no runtime, client or hosted acceptance",
    "steps": results,
    "inputs_before": before,
    "inputs_after": after,
    "changed_during_static_window": changed,
    "artifacts": {name: {"path": path.relative_to(ROOT).as_posix(), "bytes": path.stat().st_size,
                         "sha256": sha(path)} for name, path in {
        **{f"input_{name}": path for name, path in copies.items()},
        **logs, "generated_bundle": ROOT / "openapi/erp_api_v1.yaml",
        "gate_first": ROOT / "scripts/verify_openapi_gate.mjs",
        "gate_second": ROOT / "scripts/verify_openapi_gate_v2.mjs",
        "runner": Path(__file__),
    }.items() if path.is_file()},
}
destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"status": report["status"], "steps": results,
                  "changed_during_static_window": changed}, ensure_ascii=False))
raise SystemExit(0 if all_gates else 1)
