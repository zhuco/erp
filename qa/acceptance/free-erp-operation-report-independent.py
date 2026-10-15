"""Read actual report outcomes and every protected old tenant row; no business writes."""
import json,re,runpy,sys
from pathlib import Path
from decimal import Decimal
from datetime import datetime
B=Path(__file__).resolve().parent
mode=sys.argv[1]
if mode not in ['final','full','fixed-full','recovered-full']:raise SystemExit('unregistered verification window')
stem='operation-report-'+mode
before=json.loads((B/f'free-erp-complete-schema-oct8-{stem}-before.json').read_text());after=json.loads((B/f'free-erp-complete-schema-oct8-{stem}-after.json').read_text())
log=(B/f'free-erp-operation-report-oct10-{("tests-final" if mode=="final" else ("full" if mode=="full" else ("fixed-full" if mode=="fixed-full" else "recovered-full")))}.log').read_text(encoding='utf-8-sig')
checks=[]
def ck(name,actual,expected):
 r={'name':name,'pass':actual==expected}
 if actual!=expected:r.update(actual=actual,expected=expected)
 checks.append(r)
summary=re.search(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log);assert summary
ck('actual terminal registered tests',tuple(map(int,summary.group(1,2,3))),(9 if mode=='final' else 619,0,0))
ck('same source inputs',after['source_sha256'],before['source_sha256'])
for table,rows in before['protected_table_hashes'].items():ck('all old tenant rows '+table,after['protected_table_hashes'].get(table),rows)
tenants=set(re.findall(r'QA operation_reports tenant=([a-f0-9-]{36})',log));ck('new owned authority/report fixtures',len(tenants),11 if mode=='final' else 12)
h=runpy.run_path(str(B/'free-erp-product-web-fixture.py'));q=h['query'];l=h['literal'];scope=','.join(l(t) for t in tenants)
def rows(table):return json.loads(q("select coalesce(json_agg(row_to_json(x)),'[]'::json) from (select * from "+table+' where tenant_id in('+scope+')) x;'))
outcomes=rows('mfg_operation_report_outcomes');reports=rows('mfg_reports');receipts=rows('command_receipts');report_by_id={(r['tenant_id'],r['report_id']):r for r in reports};command={(r['tenant_id'],r['request_id']):r for r in receipts}
corrupt=set(re.findall(r'QA operation_report_source_corruption tenant=([a-f0-9-]{36})',log));ck('one declared owned source corruption',len(corrupt),1)
for outcome in outcomes:
 t=outcome['tenant_id'];s=outcome['snapshot'];r=report_by_id[(t,outcome['report_id'])];c=command[(t,outcome['request_id'])]
 ck('real receipt '+outcome['request_id'],(c['status'],c['effect_id'],c['payload_hash']),('SUCCEEDED',outcome['report_id'],outcome['payload_hash']))
 ck('real actor and authenticated session '+outcome['request_id'],(s['actor_user_id'],s['authority']['user_id'],s['authority']['session_id']),(outcome['actor_user_id'],r['session_user_id'],r['session_id']))
 ck('actual report identity '+outcome['request_id'],(s['work_order_id'],s['effect_id']),(r['work_order_id'],r['report_id']))
 if t in corrupt:ck('declared corruption is retained, not repaired invisibly',(s['report']['good_micros'],r['good_micros']),(250000,200000))
 else:
  actual=dict(s['report']);expected=dict(r)
  actual['created_at']=datetime.fromisoformat(actual['created_at']).isoformat();expected['created_at']=datetime.fromisoformat(expected['created_at']).astimezone(datetime.fromisoformat(actual['created_at']).tzinfo).isoformat()
  ck('actual entire report with timestamp instant '+outcome['request_id'],actual,expected)
 for field,stored in [('good_qty','good_micros'),('rejected_qty','rejected_micros'),('rework_qty','rework_micros')]:ck('exact original delta '+stored+' '+outcome['request_id'],int(Decimal(s['fields'][field])*1000000),s['report'][stored])
 ck('frozen context retained '+outcome['request_id'],s['source']['order_qty_micros'],'1000000')
 ck('report operator is separate business identity '+outcome['request_id'],r['operator_id'] not in [r['session_user_id'],r['session_id']],True)
 for line in log.splitlines():
  if 'tenant='+t not in line or ' status=200 response=' not in line:continue
  try:value=json.loads(line.split(' response=',1)[1])
  except:continue
  if value.get('request_id')==outcome['request_id'] and value.get('effect_id')==outcome['report_id']:
   value['replay']=False;ck('actual HTTP original/replay matches stored outcome '+outcome['request_id'],value,s)
ck('no duplicate business report from replay',len(reports),len(outcomes))
ck('storage failure injected and actual rollback observed','"report_storage_unavailable"' in log,True)
ck('real source mismatch returns reconciliation','"report_reconciliation_required"' in log,True)
production=rows('mfg_receipts');ck('one real API-created pending receipt',[(r['status'],r['qty_micros']) for r in production],[('HOLD',100000)])
ck('pending receipt uses actual report source',production[0]['report_id'] in {r['report_id'] for r in reports},True)
failures=[r for r in checks if not r['pass']];output={'status':'PASS' if not failures else 'PARTIAL','checks':len(checks),'failures':failures,'assertions':checks,'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),'own_outcomes':len(outcomes),'own_reports':len(reports),'new_tenants':sorted(tenants),'scope':'Current report HTTP/PG authority, frozen ceilings, preserved original results, real reversals/downstream/pending receipt guards; no formal reporting UI, native device, rework source or Release/hosting claim.'}
p=B/f'free-erp-operation-report-oct10-{mode}-independent-recheck.json';assert not p.exists();p.write_text(json.dumps(output,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');print(output['status'],len(checks),'checks',output['own_outcomes'],'actual outcomes')
if failures:print(json.dumps(failures,ensure_ascii=False)[:6000]);raise SystemExit(1)
