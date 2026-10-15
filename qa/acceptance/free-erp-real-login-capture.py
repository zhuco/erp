"""Immutable, read-only QA snapshots including actual role and session facts."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[2]
base = root / "qa/acceptance"
prefix = "free-erp-real-login-"
parser = argparse.ArgumentParser()
parser.add_argument("action", choices=["prepare", "snapshot"])
parser.add_argument("stage", nargs="?")
args = parser.parse_args()
psql = ["G:/PostgreSQL/17/bin/psql.exe", "-X", "-q", "-t", "-A", "-h", "127.0.0.1", "-p", "15438",
        "-U", "erp_grid_test", "-d", "erp_shared_contract_test", "-v", "ON_ERROR_STOP=1"]
sql = base / (prefix + "readback.sql")
if args.action == "prepare":
    source = base / "free-erp-atomic-effects-repair-readback.sql"
    raw = source.read_text(encoding="utf-8-sig")
    if raw.count("'mfg_bom_items'") != 3 or "'selected_tables',65" not in raw:
        raise SystemExit("Review the source snapshot schema before extending it")
    names = ["roles", "user_roles", "role_permissions", "auth_sessions"]
    raw = raw.replace("'mfg_bom_items'", "'mfg_bom_items'," + ",".join("'" + name + "'" for name in names))
    raw = raw.replace("'selected_tables',65", "'selected_tables',69")
    with sql.open("x", encoding="utf-8") as output:
        output.write(raw)
    catalog_sql = """begin transaction read only;
do $$ begin if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438
then raise exception 'protected real-login QA required'; end if; end $$;
select jsonb_object_agg(table_name,columns) from (select table_name,jsonb_agg(column_name order by column_name) columns
from information_schema.columns where table_schema='public' and data_type='timestamp with time zone' group by table_name) c;
rollback;
"""
    result = subprocess.run(psql + ["-c", catalog_sql], cwd=root, capture_output=True, check=True)
    catalog = json.loads(result.stdout.decode("utf-8").strip())
    with (base / (prefix + "timestamp-catalog.json")).open("x", encoding="utf-8") as output:
        json.dump(catalog, output, ensure_ascii=False, indent=2)
        output.write("\n")
    print(json.dumps({"selected_tables": 69, "sql_sha256": hashlib.sha256(sql.read_bytes()).hexdigest()}))
else:
    if not args.stage or not re.fullmatch(r"[a-z0-9-]+", args.stage):
        raise SystemExit("invalid immutable snapshot stage")
    path = base / (prefix + args.stage + ".jsonl")
    with path.open("xb") as output, (base / (prefix + args.stage + ".stderr.log")).open("xb") as errors:
        result = subprocess.run(psql + ["-f", str(sql)], cwd=root, stdout=output, stderr=errors, check=False)
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    print(json.dumps({"stage": args.stage, "exit_code": result.returncode, "bytes": path.stat().st_size,
                      "sha256": digest.hexdigest(), "sql_sha256": hashlib.sha256(sql.read_bytes()).hexdigest()}))
    raise SystemExit(result.returncode)
