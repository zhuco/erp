import json,re,runpy,hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];B=ROOT/'qa/acceptance'
before=json.loads((B/'free-erp-complete-schema-oct8-material-batch-full-before.json').read_text());after=json.loads((B/'free-erp-complete-schema-oct8-material-batch-full-after.json').read_text())
log=(B/'free-erp-material-batch-oct10-full.log').read_text(encoding='utf-8-sig');checks=[]
def check(n,a,e):
 r={'name':n,'pass':a==e}
 if not r['pass']:r.update(actual=a,expected=e)
 checks.append(r)
summary=re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',log);assert summary
check('actual scoped tests',tuple(map(int,summary.group(1,2,3))),(609,0,0))
check('compiled backend query window source input equality',after['source_sha256'],before['source_sha256'])
for table,rows in before['protected_table_hashes'].items():check('protected old rows:'+table,after['protected_table_hashes'].get(table),rows)
h=runpy.run_path(str(B/'free-erp-product-web-fixture.py'));tenants=set(re.findall(r'QA material_batch_options tenant=([a-f0-9-]{36})',log));check('five owned batch fixtures',len(tenants),5)
quotes=[]
for line in log.splitlines():
 if line.startswith('QA material_batch_get ') and ' status=200 response=' in line:
  tenant=re.search(r'tenant=([a-f0-9-]{36})',line)[1];value=json.loads(line.split(' response=',1)[1]);quotes.append((tenant,value))
  for batch in value['data']:
   actual=json.loads(h['query']("SELECT json_build_object('batch_id',q.batch_id,'batch_no',b.batch_no,'hand',q.on_hand_micros::text,'reserved',q.reserved_micros::text,'expire',b.expire_at) FROM inventory_batch_balances q LEFT JOIN sku_batches b ON b.tenant_id=q.tenant_id AND b.batch_id=q.batch_id AND b.sku_id=q.sku_id WHERE q.tenant_id="+h['literal'](tenant)+" AND q.batch_id="+h['literal'](batch['batch_id'])+';'))
   # The unknown fixture later explicitly changed A; retain that quote's original observed value separately.
   if batch['batch_no']!='RAW_%_A' or not value['needs_reconciliation']:
    check('real exact quantity:'+tenant+':'+batch['batch_id'],(batch['on_hand_micros'],batch['reserved_micros']),(actual['hand'],actual['reserved']))
   check('real batch identity/number:'+batch['batch_id'],(batch['batch_id'],batch['batch_no']),(actual['batch_id'],actual['batch_no']))
   if value['needs_reconciliation']:check('whole invalid context is not a selectable quote:'+batch['batch_id'],(batch['state'],batch['available_micros']),('RECONCILIATION_REQUIRED',None))
check('literal full count and one-row pages',sum(v['total']==2 and len(v['data'])==1 and not v['needs_reconciliation'] for _,v in quotes),2)
check('expiry retained',any(any(b['state']=='EXPIRED' for b in v['data']) for _,v in quotes),True)
check('actual 1001 complete-context rejection','status=422 response={"code":"inventory_batch_limit"}' in log,True)
scope=','.join(h['literal'](t) for t in tenants)
effects=json.loads(h['query']("SELECT json_agg(json_build_object('tenant',tenant_id,'kind',snapshot->>'kind','batch',snapshot->>'batch_id','move',move_id,'qty',snapshot->>'qty_micros','value',snapshot->'value'->>'amount_cent')) FROM mfg_material_document_effects WHERE tenant_id IN ("+scope+');'))
check('three genuinely posted selected batch effects',len(effects),3)
check('actual batch UUID preserved',len({e['batch'] for e in effects}),1)
check('real issue/consume/return values',sorted((e['kind'],e['qty'],e['value']) for e in effects),[('CONSUME','125000','150'),('ISSUE','250000','300'),('RETURN','125000','150')])
t=h['literal'](effects[0]['tenant']);batch=h['literal'](effects[0]['batch']);actual=json.loads(h['query']("SELECT json_build_object('hand',(SELECT on_hand_micros::text FROM inventory_balances WHERE tenant_id="+t+"),'batch',(SELECT on_hand_micros::text FROM inventory_batch_balances WHERE tenant_id="+t+" AND batch_id="+batch+"),'value',(SELECT sum(CASE WHEN kind IN('RECEIPT','RETURN','ADJUST') THEN amount_cent WHEN kind='ISSUE' THEN -amount_cent ELSE 0 END)::text FROM shared_item_costs WHERE tenant_id="+t+'));'))
check('real warehouse/batch/value conservation',actual,{'hand':'875000','batch':'275000','value':'1050'})
failed=[c for c in checks if not c['pass']];report={'status':'PASS' if not failed else 'PARTIAL','checks':len(checks),'failures':failed,'assertions':checks,'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),'scope':'five real route/PG batch option and selected-batch posting tests; not current UI, full610/609, Worker, native device, Release or hosting'}
p=B/'free-erp-material-batch-oct10-full-independent.json';assert not p.exists();p.write_text(json.dumps(report,indent=2)+'\n');print(report['status'],len(checks),'checks')
if failed:raise SystemExit(1)
