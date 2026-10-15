"""Trial and apply one immutable migration only to the protected PostgreSQL QA."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "qa/acceptance"
PREFIX = "free-erp-atomic-effects-"
parser = argparse.ArgumentParser()
parser.add_argument("action", choices=["trial", "apply"])
args = parser.parse_args()
migration = ROOT / "migrations/0295_atomic_material_sources.sql"
if len(list((ROOT / "migrations").glob("0295*"))) != 1:
    raise SystemExit("migration ordinal collision")
raw = migration.read_bytes()
guard = """\\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected atomic effects QA identity required';
 end if;
 if to_regclass('public.atomic_material_values') is not null then
  raise exception 'atomic material migration already exists';
 end if;
end $$;
select jsonb_build_object('stage','before','documents',coalesce(jsonb_agg(to_jsonb(d) order by d.tenant_id,d.document_id),'[]')) from atomic_source_documents d;
"""
tail = """
select jsonb_build_object('stage','after','documents',coalesce(jsonb_agg(to_jsonb(d)-'work_order_id'-'warehouse_id'-'sku_id'-'actor_user_id'-'move_id'-'request_id'-'snapshot' order by d.tenant_id,d.document_id),'[]')) from atomic_source_documents d;
select jsonb_build_object('values_table',to_regclass('public.atomic_material_values')::text,'triggers',
 (select jsonb_agg(tgname order by tgname) from pg_trigger where tgname in ('atomic_material_original','atomic_material_value_original','atomic_material_commit')));
""" + ("rollback;\n" if args.action == "trial" else "commit;\n")
stem = PREFIX + "migration-" + args.action
sql = BASE / (stem + ".sql")
with sql.open("xb") as output:
    output.write(guard.encode("utf8") + raw + tail.encode("utf8"))
log, stderr = BASE / (stem + ".log"), BASE / (stem + ".stderr.log")
with log.open("xb") as output, stderr.open("xb") as errors:
    result = subprocess.run(["G:/PostgreSQL/17/bin/psql.exe", "-X", "-q", "-t", "-A", "-h", "127.0.0.1", "-p", "15438", "-U", "erp_grid_test", "-d", "erp_shared_contract_test", "-v", "ON_ERROR_STOP=1", "-f", str(sql)], cwd=ROOT, stdout=output, stderr=errors, check=False)
rows = [json.loads(line) for line in log.read_text(encoding="utf-8-sig").splitlines() if line.strip().startswith("{")]
preserved = len(rows) == 3 and rows[0]["documents"] == rows[1]["documents"]
report = {"utc": datetime.now(timezone.utc).isoformat(), "status": "PASS" if result.returncode == 0 and preserved else "FAIL", "action": args.action,
 "database": "erp_shared_contract_test", "role": "erp_grid_test", "host": "127.0.0.1", "port": 15438,
 "old_documents_preserved": preserved, "old_documents": len(rows[0]["documents"]) if rows else 0,
 "schema": rows[2] if len(rows) == 3 else None, "exit_code": result.returncode,
 "migration_sha256": hashlib.sha256(raw).hexdigest(), "guarded_sql_sha256": hashlib.sha256(sql.read_bytes()).hexdigest()}
with (BASE / (stem + ".json")).open("x", encoding="utf8") as output:
    output.write(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(report, ensure_ascii=False))
raise SystemExit(0 if report["status"] == "PASS" else 1)
