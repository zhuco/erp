import json,runpy
from pathlib import Path
B=Path('qa/acceptance');before=json.loads((B/'free-erp-complete-schema-oct8-material-batch-ui-before.json').read_text());after=json.loads((B/'free-erp-complete-schema-oct8-material-batch-ui-after.json').read_text());checks=[]
def ck(n,a,e):
 r={'name':n,'pass':a==e}
 if a!=e:r.update(actual=a,expected=e)
 checks.append(r)
ck('source inputs unchanged',after['source_sha256'],before['source_sha256'])
for table,rows in before['protected_table_hashes'].items():ck('old tenant rows '+table,after['protected_table_hashes'].get(table),rows)
h=runpy.run_path(str(B/'free-erp-product-web-fixture.py'));f=json.loads((B/'free-erp-material-batch-web-oct10-fixture.json').read_text());t=h['literal'](f['tenant_id']);q=h['query']
def rows(table):return json.loads(q("SELECT coalesce(json_agg(row_to_json(x)),'[]'::json) FROM (SELECT * FROM "+table+' WHERE tenant_id='+t+') x;'))
effects=rows('mfg_material_document_effects');docs=rows('mfg_material_documents');stock=rows('inventory_balances');batches=rows('inventory_batch_balances');cost=rows('shared_item_costs')
ck('one real confirmed browser document',[(d['status'],d['version_no']) for d in docs],[('POSTED',1)])
ck('one actual material effect',len(effects),1);e=effects[0]['snapshot'];ck('selected batch and precise quantity/value', [e['batch_id'],e['kind'],e['qty_micros'],e['value']['amount_cent'],e['stock']['on_hand_micros']],[f['batch_id'],'ISSUE','250000','300','750000'])
ck('actual warehouse quantity',[r['on_hand_micros'] for r in stock],[750000]);ck('actual selected batch quantity',[r['on_hand_micros'] for r in batches if r['batch_id']==f['batch_id']],[750000]);ck('actual remaining known book value',sum((1 if r['kind']=='RECEIPT' else -1)*r['amount_cent'] for r in cost),900)
ck('original stored posted effect',docs[0]['posted_snapshot']['effects'],[e]);ck('original frozen line batch',docs[0]['posted_snapshot']['lines'][0]['batch_id'],f['batch_id'])
for p in ['batch-picker.png','batch-issue-confirmed.png','batch-expired-disabled.png']:ck('browser screenshot '+p,(Path('output/playwright/material-documents')/p).is_file(),True)
failed=[r for r in checks if not r['pass']];report={'status':'PASS' if not failed else 'PARTIAL','checks':len(checks),'failures':failed,'assertions':checks,'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),'database_facts':{'effects':effects,'documents':docs,'stock':stock,'batches':batches,'cost':cost},'scope':'One real Edge WASM browser selected batch issue .25, expired zero-stock batch visible and disabled. No browser batch consume/return, native device, Worker, Release or hosting claim.'};p=B/'free-erp-material-batch-web-oct10-independent.json';assert not p.exists();p.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(report['status'],len(checks),'checks',report['protected_tenants'],'tenants',report['protected_rows'],'old rows')
if failed:print(json.dumps(failed)[:4000]);raise SystemExit(1)
