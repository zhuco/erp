"""Independent readback for existing grid input contracts and protected old rows."""
import hashlib,json,re,runpy,sys
from pathlib import Path
B=Path(__file__).resolve().parent;ROOT=B.parents[1];mode=sys.argv[1]
assert mode in ('focused','full')
prefix='grid-input' if mode=='focused' else 'grid-input-full'
before=json.loads((B/f'free-erp-complete-schema-oct8-{prefix}-before.json').read_text())
after=json.loads((B/f'free-erp-complete-schema-oct8-{prefix}-after.json').read_text())
log=(B/f'free-erp-grid-input-oct10-{mode}.log').read_text(encoding='utf-8-sig');checks=[]
def check(name,actual,expected):
 r={'name':name,'pass':actual==expected}
 if actual!=expected:r.update(actual=actual,expected=expected)
 checks.append(r)
check('terminal functional result',bool(re.search(r'test result: ok\. '+str(1 if mode=='focused' else 627)+r' passed; 0 failed; 0 ignored; 0 measured;',log)),True)
check('same protected table set',sorted(after['protected_table_hashes']),sorted(before['protected_table_hashes']))
check('same source inputs',after['source_sha256'],before['source_sha256'])
check('current source inputs', {f:hashlib.sha256((ROOT/f).read_bytes()).hexdigest() for f in after['source_sha256']},after['source_sha256'])
for table,rows in before['protected_table_hashes'].items():check('old full rows '+table,after['protected_table_hashes'].get(table),rows)
markers=re.findall(r'grid-page-ok tenant=([a-f0-9-]{36}) foreign=([a-f0-9-]{36}) actor=([a-f0-9-]{36}) query=([a-f0-9-]{36}) row=(\S+)',log)
check('one actual grid fixture',len(markers),1)
h=runpy.run_path(str(B/'free-erp-product-web-fixture.py'));q=h['query'];l=h['literal']
if len(markers)==1:
 owner,foreign,actor,query_id,row_id=markers[0]
 check('new owner outside protected tenants',owner in before['protected_tenants'],False)
 check('new foreign outside protected tenants',foreign in before['protected_tenants'],False)
 actual=json.loads(q("select json_build_object('query',(select to_jsonb(t) from grid_queries t where tenant_id="+l(owner)+" and query_id="+l(query_id)+"),'selection',(select to_jsonb(t) from grid_selections t where tenant_id="+l(owner)+" and query_id="+l(query_id)+"),'gate',(select decision from grid_batch_gates where tenant_id="+l(owner)+" and current_query='status=open' and requested_query='status=closed'),'foreign_queries',(select count(*) from grid_queries where tenant_id="+l(foreign)+"),'receipts',(select count(*) from command_receipts where tenant_id="+l(owner)+" and request_id in ('u02h-page','u02h-cross','u02h-stale')));"))
 row=actual['query'] or {};sel=actual['selection'] or {}
 check('exact query values',[row.get(k) for k in ['page_code','filter_text','sort_key','tie_break','page_offset','page_limit','loaded_count','matched_count']],['ORDERS','status=open','date','ROW_ID',500,500,1,1000])
 check('exact selection values',[sel.get(k) for k in ['query_id','selection_kind','row_id','snapshot_text']],[query_id,'CROSS_PAGE','row-501',''])
 check('changed query gate',actual['gate'],'BLOCKED')
 check('foreign queries',actual['foreign_queries'],0)
 check('three original command receipts',actual['receipts'],3)
else:actual=None
failures=[c for c in checks if not c['pass']]
r={'status':'PASS' if not failures else 'FAIL','checks':len(checks),'failures':failures,'assertions':checks,'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),'grid_actual':actual,'limits':'Existing query/selection protocol persistence and functional QA only; not an actual1000-order server pagination or desktop/Web UI acceptance; no hosting/performance claim'}
p=B/f'free-erp-grid-input-oct10-{mode}-independent.json';assert not p.exists();p.write_text(json.dumps(r,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(r['status'],len(checks),'checks')
if failures:print(json.dumps(failures,ensure_ascii=False)[:3000]);raise SystemExit(1)
