"""Capture all 69 selected tables without altering earlier real-login evidence."""
import argparse
import hashlib
from pathlib import Path
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
parser = argparse.ArgumentParser()
parser.add_argument("stage")
args = parser.parse_args()
if not re.fullmatch(r"[a-z0-9-]+", args.stage):
    raise SystemExit("invalid stage")
SQL = BASE / "free-erp-real-login-readback.sql"
if hashlib.sha256(SQL.read_bytes()).hexdigest() != "35e6bebae0f92c9ad5f8966e88472c29af7bc1aab9d050d52363538478709848":
    raise SystemExit("review the shared immutable readback SQL before use")
destination = BASE / ("free-erp-session-pool-" + args.stage + ".jsonl")
error_path = BASE / ("free-erp-session-pool-" + args.stage + ".stderr.log")
if destination.exists() or error_path.exists():
    raise SystemExit("refusing to replace prior snapshot evidence")
psql = ["G:/PostgreSQL/17/bin/psql.exe", "-X", "-q", "-t", "-A", "-h", "127.0.0.1", "-p", "15438",
        "-U", "erp_grid_test", "-d", "erp_shared_contract_test", "-v", "ON_ERROR_STOP=1"]
with destination.open("xb") as output, error_path.open("xb") as errors:
    result = subprocess.run(psql + ["-f", str(SQL)], cwd=ROOT, stdout=output, stderr=errors, check=False)
sha = hashlib.sha256()
with destination.open("rb") as stream:
    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
        sha.update(chunk)
print(json.dumps({"stage": args.stage, "exit_code": result.returncode,
                  "bytes": destination.stat().st_size, "sha256": sha.hexdigest()}))
raise SystemExit(result.returncode)
