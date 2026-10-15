"""Protect existing complete QA tenants without exporting their row values."""
import hashlib
import json
import runpy
import sys
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
helpers = runpy.run_path(str(BASE / 'free-erp-product-web-fixture.py'))
phase = sys.argv[1]
if phase not in ['grid-input-before', 'grid-input-after', 'grid-input-full-before', 'grid-input-full-after', 'before', 'after', 'fixed-before', 'fixed-after', 'manufacturing-before', 'manufacturing-after', 'focused-before', 'focused-after', 'late-after', 'fixtures-before', 'fixtures-after', 'mapping-before', 'mapping-after', 'runtime-before', 'runtime-after', 'admission-before', 'admission-after', 'protected-before', 'protected-after', 'resumed-before', 'resumed-after', 'dates-before', 'dates-after', 'lint-before', 'lint-after', 'cost-input-before', 'cost-input-after', 'cost-full-before', 'cost-full-after', 'trade-finance-full-before', 'trade-finance-full-after', 'material-draft-before', 'material-draft-after', 'material-draft-fixed-before', 'material-draft-fixed-after', 'material-draft-full-before', 'material-draft-full-after', 'material-confirm-before', 'material-confirm-after', 'material-confirm-fixed-before', 'material-confirm-fixed-after', 'material-confirm-full-before', 'material-confirm-full-after', 'material-confirm-edge-before', 'material-confirm-edge-after', 'material-source-before', 'material-source-after', 'material-source-fixed-before', 'material-source-fixed-after', 'material-source-full-before', 'material-source-full-after', 'manufacturing-catalog-before', 'manufacturing-catalog-after', 'manufacturing-catalog-fixed-before', 'manufacturing-catalog-fixed-after', 'manufacturing-catalog-full-before', 'manufacturing-catalog-full-after', 'material-rejection-before', 'material-rejection-after', 'material-rejection-fixed-before', 'material-rejection-fixed-after', 'material-rejection-full-before', 'material-rejection-full-after', 'material-rejection-edge-before', 'material-rejection-edge-after', 'material-ui-web-before', 'material-ui-web-after', 'material-batch-before', 'material-batch-after', 'material-batch-fixed-before', 'material-batch-fixed-after', 'material-batch-full-before', 'material-batch-full-after', 'material-batch-repaired-before', 'material-batch-repaired-after', 'material-batch-ui-before', 'material-batch-ui-after', 'operation-report-before', 'operation-report-after', 'operation-report-full-before', 'operation-report-full-after', 'operation-report-final-before', 'operation-report-final-after', 'operation-report-fixed-full-before', 'operation-report-fixed-full-after', 'operation-report-recovered-full-before', 'operation-report-recovered-full-after','report-source-before','report-source-after','report-source-full-before','report-source-full-after','report-aggregate-before','report-aggregate-after','report-aggregate-full-before','report-aggregate-full-after','quality-rows-before','quality-rows-after','quality-rows-full-before','quality-rows-full-after','quality-rows-fixed-before','quality-rows-fixed-after','quality-rows-fixed-full-before','quality-rows-fixed-full-after','quality-rows-repaired-before','quality-rows-repaired-after','quality-rows-repaired-full-before','quality-rows-repaired-full-after','install-restore-before','install-restore-after','upgrade-v323-before','upgrade-v323-after']:
    raise SystemExit('unregistered snapshot phase')
destination = BASE / ('free-erp-complete-schema-oct8-' + phase + '.json')
if destination.exists():
    raise SystemExit('refusing to overwrite evidence')
if phase.endswith('before'):
    tenant_ids = json.loads(helpers['query']("SELECT coalesce(json_agg(tenant_id ORDER BY tenant_id),'[]'::json) FROM tenants;"))
else:
    previous = BASE / ('free-erp-complete-schema-oct8-' + ('focused-before' if phase == 'late-after' else phase.replace('after', 'before')) + '.json')
    tenant_ids = json.loads(previous.read_text(encoding='utf-8'))['protected_tenants']
tables = json.loads(helpers['query']("""SELECT coalesce(json_agg(c.relname ORDER BY c.relname),'[]'::json)
FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
WHERE n.nspname='public' AND c.relkind IN ('r','p') AND NOT c.relispartition
AND EXISTS(SELECT 1 FROM pg_attribute a WHERE a.attrelid=c.oid AND a.attname='tenant_id' AND NOT a.attisdropped);"""))
ids = ','.join(helpers['literal'](value) for value in tenant_ids)
scope = 'tenant_id IN (' + ids + ')' if ids else 'false'
members = []
for table in tables:
    quoted = '"' + table.replace('"', '""') + '"'
    members.append(f"SELECT {helpers['literal'](table)} AS name,coalesce(json_agg(hash ORDER BY hash),'[]'::json) AS hashes FROM (SELECT md5(to_jsonb(t)::text) AS hash FROM public.{quoted} t WHERE {scope}) q")
# One repeatable-read snapshot, bounded planner/JSON memory per eight tables.
# The protected table/tenant/column range is unchanged.
guard = """DO $$ BEGIN
 IF current_database()<>'erp_free_ui_acceptance' OR current_user<>'erp_grid_test'
 OR inet_server_addr()<>'127.0.0.1'::inet OR inet_server_port()<>15438 THEN
 RAISE EXCEPTION 'protected complete QA identity required'; END IF;
 IF (SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database())
 IS DISTINCT FROM 'ERP owned free UI QA baseline 2026-10-08; no production' THEN
 RAISE EXCEPTION 'QA ownership marker mismatch'; END IF; END $$;
"""
statements = ["BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;", guard]
for offset in range(0,len(members),8):
 statements.append('SELECT json_object_agg(name,hashes) FROM ('+'\nUNION ALL\n'.join(members[offset:offset+8])+') facts;')
statements.append('COMMIT;')
result = subprocess.run(['G:/PostgreSQL/17/bin/psql.exe','-X','-qAt','-v','ON_ERROR_STOP=1','-d',helpers['DSN']],
 input='\n'.join(statements),encoding='utf-8',capture_output=True)
if result.returncode: raise RuntimeError(result.stderr)
rows = {}
for line in result.stdout.splitlines():
 if line.strip():
  part=json.loads(line)
  if set(part)&set(rows):raise RuntimeError('duplicate protected snapshot table')
  rows.update(part)
if set(rows)!=set(tables):raise RuntimeError('incomplete protected snapshot')

sources = set((ROOT / 'migrations').glob('*.sql'))
for directory in [ROOT / 'crates', ROOT / 'apps/erp-ui/src']:
    for suffix in ['*.rs', '*.toml', '*.js']:
        sources.update(directory.rglob(suffix))
sources.add(ROOT / 'scripts/ErpCacheMaintenance.ps1')
sources.update(ROOT / name for name in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'scripts/Invoke-ErpBuild.ps1', 'scripts/ErpBuildStorage.ps1'])
report = {'phase': phase, 'database': 'erp_free_ui_acceptance', 'protected_tenants': tenant_ids,
          'protected_table_hashes': rows,
          'source_sha256': {file.relative_to(ROOT).as_posix(): hashlib.sha256(file.read_bytes()).hexdigest() for file in sorted(sources) if file.is_file()}}
destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(phase, len(tenant_ids), 'protected tenants,', len(rows), 'tables,', len(report['source_sha256']), 'source hashes')
