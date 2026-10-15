import json,runpy,hashlib
from pathlib import Path
B=Path('qa/acceptance');h=runpy.run_path(str(B/'free-erp-product-web-fixture.py'));q=h['query'];l=h['literal'];m=Path('migrations/0314_operation_report_outcomes.sql');sql=m.read_text();name=m.name;digest=hashlib.sha256(m.read_bytes()).hexdigest()
expected={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in Path('migrations').glob('*.sql') if p!=m}
installed=json.loads(q("select json_object_agg(filename,sha256) from qa_acceptance_migration_progress;"));assert installed==expected
trial=q(sql+"\nSELECT json_build_object('trial_table',to_regclass('mfg_operation_report_outcomes')::text); ROLLBACK;")
assert q("SELECT to_regclass('mfg_operation_report_outcomes') is null;")=='t'
p=B/'free-erp-operation-report-oct10-migration-trial.json';assert not p.exists();p.write_text(json.dumps({'status':'PASS','filename':name,'sha256':digest,'installed_before':len(installed),'trial':json.loads(trial),'rolled_back':True},indent=2)+'\n')
q(sql+'\nINSERT INTO qa_acceptance_migration_progress(filename,sha256) VALUES('+l(name)+','+l(digest)+');')
assert q('SELECT sha256 FROM qa_acceptance_migration_progress WHERE filename='+l(name)+';')==digest
p=B/'free-erp-operation-report-oct10-migration-apply.json';assert not p.exists();p.write_text(json.dumps({'status':'PASS','filename':name,'sha256':digest,'installed_after':len(installed)+1,'database':'erp_free_ui_acceptance','historical_outcomes_created':0},indent=2)+'\n');print('trial rollback and protected QA apply PASS',len(installed)+1,'migrations')
