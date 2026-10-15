"""Immutable input and read-only protected PostgreSQL capture for atomic manufacturing."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-atomic-effects-"
parser = argparse.ArgumentParser()
parser.add_argument("action", choices=["prepare", "snapshot"])
parser.add_argument("stage", nargs="?")
args = parser.parse_args()
sql_path = BASE / (PREFIX + "readback.sql")

def write_new(path, content):
    with path.open("xb") as output:
        output.write(content)

if args.action == "prepare":
    raw = (BASE / "free-erp-command-readback-readback.sql").read_bytes()
    write_new(sql_path, raw)
    files = ["crates/erp-repo/src/shared_contract.rs", "crates/erp-api/src/manufacturing.rs",
             "crates/erp-domain/src/manufacturing.rs", "crates/erp-domain/src/contract_tests.rs",
             "crates/erp-api/tests/free_erp_acceptance.rs", "crates/erp-api/tests/free_erp_acceptance/atomic_effects.rs",
             "crates/erp-repo/src/shared_contract/material_moves.rs", "migrations/0132_atomic_effects.sql",
             "migrations/0273_atomic_effect_races.sql"]
    manifest = {}
    for index, name in enumerate(files):
        data = (ROOT / name).read_bytes()
        archived = BASE / (PREFIX + f"original-input-{index}.txt")
        write_new(archived, data)
        manifest[name] = {"sha256": hashlib.sha256(data).hexdigest(), "copy": archived.relative_to(ROOT).as_posix()}
    write_new(BASE / (PREFIX + "original-inputs.json"), (json.dumps({
        "utc": datetime.now(timezone.utc).isoformat(), "files": manifest,
        "capture_sql_sha256": hashlib.sha256(raw).hexdigest(), "selected_tables": 60,
    }, ensure_ascii=False, indent=2) + "\n").encode("utf8"))
    print(json.dumps({"sources": len(manifest), "required_tables": 60}))
else:
    if not args.stage or not re.fullmatch(r"[a-z0-9-]+", args.stage):
        raise SystemExit("invalid stage")
    output = BASE / (PREFIX + args.stage + ".jsonl")
    errors = BASE / (PREFIX + args.stage + ".stderr.log")
    with output.open("xb") as facts, errors.open("xb") as diagnostics:
        result = subprocess.run(["G:/PostgreSQL/17/bin/psql.exe", "-X", "-q", "-t", "-A",
            "-h", "127.0.0.1", "-p", "15438", "-U", "erp_grid_test", "-d", "erp_shared_contract_test",
            "-v", "ON_ERROR_STOP=1", "-f", str(sql_path)], cwd=ROOT, stdout=facts, stderr=diagnostics, check=False)
    digest = hashlib.sha256()
    with output.open("rb") as facts:
        for chunk in iter(lambda: facts.read(1024 * 1024), b""):
            digest.update(chunk)
    print(json.dumps({"stage": args.stage, "exit_code": result.returncode, "bytes": output.stat().st_size,
                      "sha256": digest.hexdigest(), "sql_sha256": hashlib.sha256(sql_path.read_bytes()).hexdigest()}))
    raise SystemExit(result.returncode)
