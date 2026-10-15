"""Immutable source inputs for legacy stock consumers; no build or data mutation."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument("stage")
args = parser.parse_args()
if not re.fullmatch(r"[a-z0-9-]+", args.stage):
    raise SystemExit("invalid stage")
paths = []
for directory in ["crates", "apps"]:
    paths += list((root / directory).rglob("*.rs"))
    paths += list((root / directory).rglob("Cargo.toml"))
paths += list((root / "migrations").glob("*.sql"))
paths += [root / name for name in ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
    ".cargo/config.toml", "AGENTS.md", "scripts/Invoke-ErpBuild.ps1", "scripts/Build-ErpWeb.ps1",
    "openapi/src/components.yaml", "openapi/src/paths.yaml", "openapi/src/openapi.yaml",
    "openapi/erp_api_v1.yaml", "openapi/channel_ingest_v1.yaml"]]
files = {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
    for p in sorted(set(paths)) if p.is_file() and "target" not in p.relative_to(root).parts}
destination = root / "qa/acceptance" / f"free-erp-inventory-consumers-{args.stage}-source.json"
if destination.exists():
    raise SystemExit("refusing to replace historical input manifest")
destination.write_text(json.dumps({"updated_at": datetime.now(timezone.utc).isoformat(),
    "scope": "Native/Web Rust, schema, lock, build-entry and OpenAPI inputs", "files": files},
    indent=2, ensure_ascii=False) + "\n", encoding="utf8")
print(json.dumps({"stage": args.stage, "count": len(files)}))
