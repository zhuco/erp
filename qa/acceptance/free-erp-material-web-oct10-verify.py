"""Independent browser commands, actual material value effects and protected old rows."""
import hashlib,json,re,runpy
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'qa/acceptance'
checks=[]
def check(name,actual,expected):
 r={'name':name,'pass':actual==expected}
 if not r['pass']:r.update(actual=actual,expected=expected)
 checks.append(r)
before=json.loads((BASE/'free-erp-complete-schema-oct8-material-ui-web-before.json').read_text())
after=json.loads((BASE/'free-erp-complete-schema-oct8-material-ui-web-after.json').read_text())
for table,hashes in before['protected_table_hashes'].items():check('protected all old tenant rows:'+table,after['protected_table_hashes'].get(table),hashes)
changed=[n for n,h in before['source_sha256'].items() if after['source_sha256'].get(n)!=h]
allowed={'apps/erp-ui/src/page_registry.rs','apps/erp-ui/src/workspace_shell.rs','apps/erp-ui/src/remote_view/material_documents.rs','apps/erp-ui/src/remote_view/material_documents_ui.rs'}
check('browser repairs change only declared UI files',set(changed)<=allowed,True)
verified=json.loads((BASE/'free-erp-material-ui-oct10-inputs-verified-before.json').read_text())
check('final gate input hashes remain current',all(hashlib.sha256((ROOT/n).read_bytes()).hexdigest()==h for n,h in verified.items()),True)
gates=json.loads((BASE/'free-erp-material-ui-oct10-gates-verified.json').read_text(encoding='utf-8-sig'))
check('current native/Web test/lint/check/package gates',gates,{'native_test':'PASS','native_clippy':'PASS','web_check':'PASS','web_clippy':'PASS','web_package':'PASS'})
log=(BASE/'free-erp-material-ui-oct10-native-verified.log').read_text(encoding='utf-8-sig')
results=[tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out;',log)]
check('all affected native package tests',sum(r[0] for r in results),397)
check('no failed/ignored/filtered native package tests',all(r[1:]==(0,0,0) for r in results),True)
proof=json.loads((BASE/'free-erp-material-web-oct10-proof.json').read_text());f=json.loads((BASE/'free-erp-material-web-oct10-fixture.json').read_text());h=runpy.run_path(str(BASE/'free-erp-product-web-fixture.py'));t=h['literal'](f['tenant_id'])
check('owned new browser fixture',h['query']('SELECT name FROM tenants WHERE tenant_id='+t+';'),'QA-MATERIAL-WEB-OCT10')
check('actual successful save response was dropped',proof['dropped'],True)
request=proof['original']['request_id'];posts=[r for r in proof['requests'] if r['method']=='POST'];original_posts=[r for r in posts if json.loads(r['body']).get('request_id')==request]
check('lost-response original sent only once',len(original_posts),1)
check('reloaded browser GETs exact original identity',any(r['method']=='GET' and 'request_id='+request in r['url'] for r in proof['requests']),True)
original=json.loads(h['query']('SELECT snapshot FROM mfg_material_document_revisions WHERE tenant_id='+t+' AND request_id='+h['literal'](request)+';'))
check('actual dropped response equals immutable saved result',original,proof['original'])
check('original draft was not a posting result',(original['status'],original['posted'],original['lines'][0]['qty_micros']),('DRAFT',False,'250000'))
heads=json.loads(h['query']("SELECT json_agg(json_build_object('id',document_id,'version',version_no,'status',status,'snapshot',posted_snapshot,'request',posted_request_id)) FROM mfg_material_documents WHERE tenant_id="+t+';'))
check('four actual browser documents',len(heads),4)
check('three actual posted documents',sum(d['status']=='POSTED' for d in heads),3)
rows=json.loads(h['query']("SELECT json_agg(json_build_object('kind',e.snapshot->>'kind','qty',m.qty_micros::text,'move',m.move_id,'line',e.line_id,'snapshot',e.snapshot)) FROM mfg_material_document_effects e JOIN mfg_material_moves m ON m.tenant_id=e.tenant_id AND m.move_id=e.move_id WHERE e.tenant_id="+t+';'))
check('three actual stock/value effects',len(rows),3)
expect={'ISSUE':('250000','300','750000'),'CONSUME':('125000','150','750000'),'RETURN':('125000','150','875000')}
for row in rows:
 s=row['snapshot'];check('actual quantity/value/original stock:'+row['kind'],(row['qty'],s['value']['amount_cent'],s['stock']['on_hand_micros']),expect[row['kind']]);check('effect binds real movement:'+row['kind'],(s['move_id'],s['line_id']),(row['move'],row['line']))
actual=json.loads(h['query']("SELECT json_build_object('stock',(SELECT on_hand_micros::text FROM inventory_balances WHERE tenant_id="+t+"),'qty',sum(CASE WHEN kind IN('RECEIPT','RETURN') THEN qty_micros WHEN kind='ISSUE' THEN -qty_micros ELSE 0 END)::text,'value',sum(CASE WHEN kind IN('RECEIPT','RETURN','ADJUST') THEN amount_cent WHEN kind='ISSUE' THEN -amount_cent ELSE 0 END)::text) FROM shared_item_costs WHERE tenant_id="+t+';'))
check('actual stock and cost conservation',actual,{'stock':'875000','qty':'875000','value':'1050'})
revisions=json.loads(h['query']('SELECT json_agg(snapshot ORDER BY document_id,version_no) FROM mfg_material_document_revisions WHERE tenant_id='+t+';'))
draft=[d for d in heads if d['status']=='DRAFT'][0];history=[s for s in revisions if s['document_id']==draft['id']]
check('explicit review/latest adoption history',[(s['version_no'],s['lines'][0]['qty_micros'],s['source']['order_qty_micros']) for s in history],[(1,'1000000','1000000'),(2,'1000000','2000000'),(3,'500000','2000000'),(4,'600000','2000000'),(5,'250000','2000000')])
check('real explicit source adoption retained old source',history[1]['source_adoption']['previous_source']['order_qty_micros'],'1000000')
check('concurrent actual server v4 matches captured response',history[3],proof['concurrent']['response']['snapshot'])
for s in revisions:
 receipt=json.loads(h['query']("SELECT json_build_object('status',status,'effect',effect_id) FROM command_receipts WHERE tenant_id="+t+" AND request_id="+h['literal'](s['request_id'])+';'))
 check('immutable revision has original success receipt:'+s['request_id'],receipt,{'status':'SUCCEEDED','effect':s['revision_id']})
for row in posts:
 body=json.loads(row['body']);request_id=body['request_id']
 if '/confirm?' in row['url']:continue
 if request_id not in {s['request_id'] for s in revisions}:
  check('rejected original identity has no success receipt:'+request_id,int(h['query']('SELECT count(*) FROM command_receipts WHERE tenant_id='+t+' AND request_id='+h['literal'](request_id)+';')),0)
failures=[c for c in checks if not c['pass']]
report={'status':'PASS' if not failures else 'PARTIAL','checks':len(checks),'failures':failures,'assertions':checks,'native_tests':397,'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'global_source_inputs':len(after['source_sha256']),'browser_window_source_changes':changed,'verified_final_ui_inputs':len(verified),'browser_material_requests':len(proof['requests']),'posted_documents':3,'draft_latest_version':5,'scope':'real Edge/WASM to owned API/PostgreSQL with IndexedDB reload recovery; not native desktop interaction, batches/location/rework, 100 features, Worker, Release or public hosting'}
p=BASE/'free-erp-material-web-oct10-independent.json';assert not p.exists();p.write_text(json.dumps(report,indent=2)+'\n');print(report['status'],len(checks),'independent checks')
if failures:raise SystemExit(1)
