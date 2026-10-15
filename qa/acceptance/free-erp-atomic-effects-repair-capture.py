"""Read-only complete 65-table QA snapshots after the atomic material schema change."""
import argparse
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
psql = ["G:/PostgreSQL/17/bin/psql.exe", "-X", "-q", "-t", "-A", "-h", "127.0.0.1", "-p", "15438", "-U", "erp_grid_test", "-d", "erp_shared_contract_test", "-v", "ON_ERROR_STOP=1"]
sql = BASE / (PREFIX + "repair-readback.sql")
if args.action == "prepare":
    raw = (BASE / (PREFIX + "readback.sql")).read_text(encoding="utf-8-sig")
    names = ["atomic_material_values", "shared_mfg_bom_versions", "shared_mfg_bom_version_lines", "mfg_boms", "mfg_bom_items"]
    raw = raw.replace("'shared_inspection_reading_reviews'", "'shared_inspection_reading_reviews'," + ",".join("'" + name + "'" for name in names))
    raw = raw.replace("'selected_tables',60", "'selected_tables',65")
    with sql.open("x", encoding="utf8") as output:
        output.write(raw)
    catalogue = """begin transaction read only;
do $$ begin if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test' or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then raise exception 'protected atomic effects QA required'; end if; end $$;
select jsonb_object_agg(table_name,columns) from (select table_name,jsonb_agg(column_name order by column_name) columns from information_schema.columns where table_schema='public' and data_type='timestamp with time zone' group by table_name) c;
rollback;
"""
    result = subprocess.run(psql + ["-c", catalogue], cwd=ROOT, capture_output=True, check=True)
    values = json.loads(result.stdout.decode("utf8").strip())
    with (BASE / (PREFIX + "repair-timestamp-catalog.json")).open("x", encoding="utf8") as output:
        output.write(json.dumps(values, ensure_ascii=False) + "\n")
    print(json.dumps({"selected_tables": 65, "sql_sha256": hashlib.sha256(sql.read_bytes()).hexdigest()}))
else:
    if not args.stage or not re.fullmatch(r"[a-z0-9-]+", args.stage):
        raise SystemExit("invalid stage")
    path = BASE / (PREFIX + args.stage + ".jsonl")
    with path.open("xb") as output, (BASE / (PREFIX + args.stage + ".stderr.log")).open("xb") as errors:
        result = subprocess.run(psql + ["-f", str(sql)], cwd=ROOT, stdout=output, stderr=errors, check=False)
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024*1024), b""):
            digest.update(chunk)
    print(json.dumps({"stage": args.stage, "exit_code": result.returncode, "bytes": path.stat().st_size, "sha256": digest.hexdigest(), "sql_sha256": hashlib.sha256(sql.read_bytes()).hexdigest()}))
    raise SystemExit(result.returncode)
