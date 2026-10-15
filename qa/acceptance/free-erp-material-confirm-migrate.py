"""Append only the registered material draft migration to the owned complete QA DB."""
import hashlib
import json
import runpy
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'qa/acceptance'
helpers=runpy.run_path(str(BASE/'free-erp-product-web-fixture.py'))
ledger=json.loads(helpers['query']("SELECT json_object_agg(filename,sha256) FROM qa_acceptance_migration_progress;"))
files={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT/'migrations').glob('*.sql')}
name='0313_material_document_confirmation.sql'
if set(ledger)-set(files) or any(files.get(n)!=h for n,h in ledger.items()):
    raise SystemExit('existing formal migration identities or SHA changed')
if set(files)-set(ledger)-{name}:
    raise SystemExit('unregistered missing formal migrations')
destination=BASE/'free-erp-material-confirm-oct9-migration.json'
if destination.exists():
    raise SystemExit('preserve migration evidence')
raw=(ROOT/'migrations'/name).read_text(encoding='utf-8')
if name not in ledger:
    q=helpers['literal']
    helpers['query']("BEGIN; SET LOCAL lock_timeout='10s'; SET LOCAL statement_timeout='60s';\n"+raw+
       "\nINSERT INTO qa_acceptance_migration_progress(filename,sha256) VALUES("+q(name)+','+q(files[name])+"); COMMIT; SELECT 'material-draft-migration-applied';")
actual=json.loads(helpers['query']("SELECT json_object_agg(filename,sha256) FROM qa_acceptance_migration_progress;"))
if actual!=files:raise SystemExit('formal migration ledger differs from source')
report={'status':'PASS','scope':'owned complete QA append; not fresh 321-file install, production upgrade or restore',
        'database':'erp_free_ui_acceptance','new_file':name,'sha256':files[name],'migration_count':len(files),'prior_count':len(ledger)}
destination.write_text(json.dumps(report,indent=2)+'\n');print(report)
