import json,re,runpy,sys
from pathlib import Path
B=Path(__file__).resolve().parent;mode=sys.argv[1]
assert mode in ['fixed','fixed-full','repaired','repaired-full']
prefix='quality-rows-'+mode
before=json.loads((B/f'free-erp-complete-schema-oct8-{prefix}-before.json').read_text());after=json.loads((B/f'free-erp-complete-schema-oct8-{prefix}-after.json').read_text());log=(B/f'free-erp-quality-rows-oct10-tests-{mode}.log').read_text(encoding='utf-8-sig') if mode in ['fixed','repaired'] else (B/'free-erp-quality-rows-oct10-full.log').read_text(encoding='utf-8-sig');checks=[]
def check(n,a,e):
 r={'name':n,'pass':a==e}
 if a!=e:r.update(actual=a,expected=e)
 checks.append(r)
check('actual terminal tests',bool(re.search(r'test result: ok\. '+str(2 if mode in ['fixed','repaired'] else 627)+r' passed; 0 failed; 0 ignored;',log)),True)
check('same source inputs',before['source_sha256'],after['source_sha256'])
for table,rows in before['protected_table_hashes'].items():check('old full rows '+table,after['protected_table_hashes'].get(table),rows)
tenants=set(re.findall(r'QA quality_rows tenant=([a-f0-9-]{36})',log));check('four new owned companies',len(tenants),4)
h=runpy.run_path(str(B/'free-erp-product-web-fixture.py'));q=h['query'];l=h['literal'];scope=','.join(l(t) for t in tenants)
jobs=json.loads(q("select coalesce(json_agg(row_to_json(j)),'[]'::json) from export_jobs j where tenant_id in("+scope+');'))
check('three actual export jobs only',len(jobs),3)
export=[j for j in jobs if j['job_type']=='PRODUCT_EXPORT' and j['status']=='RUNNING'];check('named fields preserved',[(j['status'],j['output_url'],j['progress'],j['attempts'],j['last_error']) for j in export],[('RUNNING','qa-owned://pending.csv',37,2,'QA retry detail')])
ordinary=[j for j in jobs if j['job_type']=='QA-QUALITY-ROWS'];check('ordinary job nullable output',[(j['output_url'],j['progress'],j['attempts']) for j in ordinary],[(None,0,0)])
batches=json.loads(q("select coalesce(json_agg(row_to_json(b)),'[]'::json) from grid_product_copy_batches b where tenant_id in("+scope+');'));check('two scoped batch records',len(batches),2)
for b in batches:check('exact complete counters '+b['tenant_id'],[b[k] for k in ['total','applied','rejected','cancelled','pending','unknown','cancel_requested']],[10,2,1,1,4,2,True])
fail=[c for c in checks if not c['pass']];r={'status':'PASS' if not fail else 'PARTIAL','checks':len(checks),'failures':fail,'assertions':checks,'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),'own_jobs':jobs,'own_batches':batches,'scope':'Existing named-row export/batch contracts and all protected old rows; no new business page or hosting claim'};p=B/f'free-erp-quality-rows-oct10-{mode}-independent.json';assert not p.exists();p.write_text(json.dumps(r,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(r['status'],len(checks),'checks')
if fail:print(json.dumps(fail,ensure_ascii=False)[:5000]);raise SystemExit(1)
