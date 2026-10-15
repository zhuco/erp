"""Qualify actual build windows and live-source drift without overwriting other work."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
owned = ["crates/erp-repo/src/shared_contract/command_readback.rs",
         "crates/erp-api/src/manufacturing/command_readback.rs",
         "crates/erp-api/tests/free_erp_acceptance/command_readback.rs"]
windows = {}
for stage, before, after in [
    ("original", "original-test-before", "original-test-after"),
    ("focused", "fixed-focused-before", "fixed-focused-after"),
    ("full", "full-before", "full-after"),
]:
    paths = [BASE / f"free-erp-inventory-consumers-command-readback-{item}-source.json" for item in [before,after]]
    first, second = [json.loads(path.read_text(encoding="utf8"))["files"] for path in paths]
    changed = [name for name in sorted(set(first) | set(second)) if first.get(name) != second.get(name)]
    windows[stage] = {"before_manifest":paths[0].relative_to(ROOT).as_posix(),
                      "after_manifest":paths[1].relative_to(ROOT).as_posix(),
                      "input_count":len(second),"changed_inputs":changed,
                      "status":"PARTIAL" if changed else "PASS",
                      "owned_leaf_unchanged":{name:first.get(name)==second.get(name) for name in owned if name in first}}

latest = json.loads((BASE / "free-erp-inventory-consumers-command-readback-full-after-source.json").read_text(encoding="utf8"))["files"]
live_drift = [name for name,digest in latest.items()
              if not (ROOT/name).is_file() or hashlib.sha256((ROOT/name).read_bytes()).hexdigest()!=digest]
current_owned = {name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in owned}
destination = BASE / "free-erp-command-readback-source-audit.json"
if destination.exists():
    raise SystemExit("refusing to replace source qualification")
report = {"utc":datetime.now(timezone.utc).isoformat(),"status":"PARTIAL" if live_drift or any(window["changed_inputs"] for window in windows.values()) else "PASS",
          "scope":"Actual registered tests belong to their captured source windows. Concurrent parent edits are preserved and not certified as one frozen current product.",
          "windows":windows,"live_drift_after_full":live_drift,"current_owned_leaf_sha256":current_owned}
destination.write_text(json.dumps(report,ensure_ascii=False,indent=2)+"\n",encoding="utf8")
print(json.dumps({"status":report["status"],"window_changes":{stage:len(window["changed_inputs"]) for stage,window in windows.items()},
                  "live_drift":live_drift,"owned":{stage:window["owned_leaf_unchanged"] for stage,window in windows.items()}},ensure_ascii=False))
