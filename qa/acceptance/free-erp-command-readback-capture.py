"""Immutable, read-only facts and source capture for the command readback repair."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-command-readback-"
parser = argparse.ArgumentParser()
parser.add_argument("action", choices=["prepare", "snapshot"])
parser.add_argument("stage", nargs="?")
args = parser.parse_args()
sql_path = BASE / (PREFIX + "readback.sql")
if args.action == "prepare":
    if sql_path.exists():
        raise SystemExit("refusing to replace the original capture specification")
    original = (BASE / "free-erp-inspection-gates-readback.sql").read_text(encoding="utf8")
    extra = ["command_objects", "browser_sessions", "atomic_source_documents", "atomic_posting_events"]
    source = original.replace("'command_receipts','orders'", ",".join(f"'{name}'" for name in extra) + ",'command_receipts','orders'")
    tables = re.findall(r"'([a-z_]+)'", source.split("from unnest(array[", 1)[1].split("]) name", 1)[0])
    guard = "do $$ declare name text; begin foreach name in array array[" + ",".join(f"'{name}'" for name in tables) + "] loop if to_regclass(format('public.%I',name)) is null then raise exception 'required snapshot table missing: %',name; end if; end loop; end $$;\n"
    source = source.replace("select jsonb_build_object('database'", guard + "select jsonb_build_object('selected_tables'," + str(len(tables)) + ",'database'", 1)
    sql_path.write_text(source, encoding="utf8")
    files = ["crates/erp-repo/src/shared_contract.rs", "crates/erp-api/src/manufacturing.rs",
             "crates/erp-domain/src/receipt.rs", "crates/erp-domain/src/contract_tests.rs",
             "crates/erp-api/tests/free_erp_acceptance.rs", "crates/erp-api/tests/free_erp_acceptance/command_readback.rs",
             "migrations/0133_command_lookup.sql", "migrations/0140_browser_security.sql", "migrations/0274_command_lookup_races.sql"]
    manifest = {}
    for index, name in enumerate(files):
        data = (ROOT / name).read_bytes()
        archived = BASE / (PREFIX + f"original-input-{index}.txt")
        if archived.exists():
            raise SystemExit("refusing to replace an original source")
        archived.write_bytes(data)
        manifest[name] = {"sha256": hashlib.sha256(data).hexdigest(), "copy": archived.relative_to(ROOT).as_posix()}
    (BASE / (PREFIX + "original-inputs.json")).write_text(json.dumps({"utc": datetime.now(timezone.utc).isoformat(), "files": manifest, "tables": tables}, ensure_ascii=False, indent=2) + "\n", encoding="utf8")
    print(json.dumps({"sources": len(manifest), "required_tables": len(tables)}))
else:
    if not args.stage or not re.fullmatch(r"[a-z0-9-]+", args.stage):
        raise SystemExit("invalid stage")
    output = BASE / (PREFIX + args.stage + ".jsonl")
    errors = BASE / (PREFIX + args.stage + ".stderr.log")
    if output.exists() or errors.exists():
        raise SystemExit("refusing to replace historical facts")
    with output.open("wb") as facts, errors.open("wb") as diagnostics:
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
