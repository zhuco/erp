import json,re,runpy
from pathlib import Path
from datetime import datetime
B=Path('qa/acceptance');before=json.loads((B/'free-erp-complete-schema-oct8-report-source-before.json').read_text());after=json.loads((B/'free-erp-complete-schema-oct8-report-source-after.json').read_text());log=(B/'free-erp-report-source-oct10-tests.log').read_text(encoding='utf-8-sig');checks=[]
def ck(n,a,e):
 r={'name':n,'pass':a==e}
 if a!=e:r.update(actual=a,expected=e)
 checks.append(r)
ck('actual terminal six focused cases',bool(re.search(r'test result: ok\. 6 passed; 0 failed; 0 ignored;',log)),True)
ck('source window stable',after['source_sha256'],before['source_sha256'])
for table,rows in before['protected_table_hashes'].items():ck('old rows '+table,after['protected_table_hashes'].get(table),rows)
tenants=set(re.findall(r'QA report_sources tenant=([a-f0-9-]{36})',log));ck('ten new owned source fixtures',len(tenants),10)
h=runpy.run_path(str(B/'free-erp-product-web-fixture.py'));q=h['query'];l=h['literal'];scope=','.join(l(t) for t in tenants)
reports=json.loads(q("select coalesce(json_agg(row_to_json(r)),'[]'::json) from mfg_reports r where tenant_id in("+scope+');'))
outcomes=json.loads(q("select coalesce(json_agg(row_to_json(r)),'[]'::json) from mfg_operation_report_outcomes r where tenant_id in("+scope+');'))
ck('unknown/corrupt source fixtures retained',len(reports),1007);ck('five genuine committed outcomes only',len(outcomes),5)
for line in log.splitlines():
 if 'QA reporting_context ' not in line or ' status=200 response=' not in line:continue
 tenant=re.search(r'tenant=([a-f0-9-]{36})',line)[1];value=json.loads(line.split(' response=',1)[1]);time=datetime.fromisoformat(value['observed_at'].replace('Z','+00:00'));current=[r for r in reports if r['tenant_id']==tenant and datetime.fromisoformat(r['created_at'])<=time]
 work=json.loads(q("select to_jsonb(w) from mfg_work_orders w where tenant_id="+l(tenant)+' and work_order_id='+l(value['work_order_id'])+';'))
 ck('actual order business fields',(value['work_order_no'],value['product_code'],value['order_qty_micros']),(work['work_order_no'],work['product_code'],str(work['order_qty_micros'])))
 for op in value['operations']:
  selected=[r for r in current if r['operation_code']==op['operation_code']];good=sum(r['good_micros'] for r in selected);bad=sum(r['rejected_micros'] for r in selected);rew=sum(r['rework_micros'] for r in selected);mins=sum(r['minutes'] for r in selected);ceil=work['order_qty_micros'] if not op['predecessor_code'] else sum(r['good_micros'] for r in current if r['operation_code']==op['predecessor_code'])
  ck('actual complete-source quantities '+op['operation_code'],[op[k] for k in ['good_micros','rejected_micros','rework_micros','minutes','ceiling_micros','remaining_micros']],list(map(str,[good,bad,rew,mins,ceil,ceil-good-bad])))
ck('original recovery remained available on valid original request',bool(re.search(r'tail=/commands/.*status=200',log)),True)
f=[c for c in checks if not c['pass']];result={'status':'PASS' if not f else 'PARTIAL','checks':len(checks),'failures':f,'assertions':checks,'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),'scope':'Six complete-source handler/PG cases; not current full625, strict lint, reporting UI, worker or hosting.'};p=B/'free-erp-report-source-oct10-independent.json';assert not p.exists();p.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(result['status'],len(checks),'checks',result['protected_tenants'],'old tenants')
if f:print(json.dumps(f,ensure_ascii=False)[:5000]);raise SystemExit(1)
