"""Independent original source adoption, real scoped pages and data protection."""
import argparse
import json
import re
import runpy
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'qa/acceptance'
p=argparse.ArgumentParser();m=p.add_mutually_exclusive_group();m.add_argument('--fixed',action='store_true');m.add_argument('--full',action='store_true');args=p.parse_args()
prefix='free-erp-material-source-full-oct10' if args.full else 'free-erp-material-source-oct10-fixed' if args.fixed else 'free-erp-material-source-oct10'
phase='material-source-full' if args.full else 'material-source-fixed' if args.fixed else 'material-source'
log=(BASE/(prefix+'-full.log' if args.full else prefix+'-tests.log')).read_text(encoding='utf-8-sig')
summary=re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s',log)
if not summary:raise SystemExit('no actual test terminal')
before=json.loads((BASE/('free-erp-complete-schema-oct8-'+phase+'-before.json')).read_text());after=json.loads((BASE/('free-erp-complete-schema-oct8-'+phase+'-after.json')).read_text())
checks=[]
def check(name,actual,expected):
    row={'name':name,'pass':actual==expected}
    if not row['pass']:row.update(actual=actual,expected=expected)
    checks.append(row)
check('actual cases/no failures/no ignores',tuple(map(int,summary.group(2,3,4))),(595 if args.full else 4,0,0))
check('same source window',after['source_sha256'],before['source_sha256'])
for table,hashes in before['protected_table_hashes'].items():check('all old rows:'+table,after['protected_table_hashes'].get(table),hashes)
tenants=sorted(set(re.findall(r'QA material_source tenant=([a-f0-9-]{36})',log)));check('four own source fixtures',len(tenants),4)
helpers=runpy.run_path(str(BASE/'free-erp-product-web-fixture.py'));scope=','.join(helpers['literal'](t) for t in tenants)
stored=json.loads(helpers['query']("SELECT json_agg(json_build_object('tenant',r.tenant_id,'revision',r.revision_id,'snapshot',r.snapshot,'effect',c.effect_id,'receipt_status',c.status)) FROM mfg_material_document_revisions r JOIN command_receipts c USING(tenant_id,request_id) WHERE r.tenant_id IN ("+scope+');'))
lookup={(r['tenant'],r['revision']):r for r in stored};pages=[];success=0
for line in log.splitlines():
    if line.startswith('QA production_release_http ') and '/material-documents' in line and ' status=200 response=' in line:
        tenant=re.search(r'tenant=([a-f0-9-]{36})',line)[1]
        if tenant not in tenants:continue
        response=json.loads(line.split(' response=',1)[1]);snapshot=response['snapshot'];row=lookup[(tenant,snapshot['revision_id'])];success+=1
        check('real original revision:'+tenant+':'+snapshot['request_id'],row['snapshot'],snapshot)
        check('real save receipt:'+snapshot['revision_id'],(row['receipt_status'],row['effect']),('SUCCEEDED',snapshot['revision_id']))
        check('adoption stays a draft:'+snapshot['revision_id'],(snapshot['status'],snapshot['posted']),('DRAFT',False))
    if line.startswith('QA material_source_get ') and '/material-documents?' in line and ' status=200 response=' in line:
        tenant=re.search(r'tenant=([a-f0-9-]{36})',line)[1];response=json.loads(line.split(' response=',1)[1]);pages.append((tenant,line,response))
page_tenants=sorted({t for t,l,r in pages if 'limit=3&status=DRAFT' in l})
check('one true paging fixture',len(page_tenants),1)
tenant=page_tenants[0]
scope_row=json.loads(helpers['query']("SELECT json_build_object('headers',count(*),'visible',count(*) FILTER(WHERE EXISTS(SELECT 1 FROM mfg_material_document_lines l WHERE l.tenant_id=d.tenant_id AND l.document_id=d.document_id) AND NOT EXISTS(SELECT 1 FROM mfg_material_document_lines l WHERE l.tenant_id=d.tenant_id AND l.document_id=d.document_id AND NOT EXISTS(SELECT 1 FROM user_warehouse_access a WHERE a.tenant_id=l.tenant_id AND a.warehouse_id=l.warehouse_id)))) FROM mfg_material_documents d WHERE d.tenant_id="+helpers['literal'](tenant)+';'))
check('actual full range and current scope',scope_row,{'headers':8,'visible':6})
matching=[r for t,l,r in pages if t==tenant and 'limit=3&status=DRAFT' in l]
check('two actual bounded draft pages',len(matching),2)
for index,page in enumerate(matching):check('full total before page:'+str(index),(page['total'],len(page['data'])),(6,3))
check('stable IDs across pages',len({r['document_id'] for page in matching for r in page['data']}),6)
adopted=[r for r in stored if r['snapshot'].get('source_adoption')]
check('two genuine adoption revisions',len(adopted),2)
for row in adopted:
    snapshot=row['snapshot'];check('prior source remains explicit:'+row['revision'],snapshot['source_adoption']['previous_source']['order_qty_micros'],'10000000')
    check('new source review token retained:'+row['revision'],len(snapshot['source_adoption']['review_token']),64)
check('adopted 30M source actually stored',any(r['snapshot']['source']['order_qty_micros']=='30000000' for r in adopted),True)
check('adopted 20M source after real fault retry stored',any(r['snapshot']['source']['order_qty_micros']=='20000000' for r in adopted),True)
for table in ['mfg_material_moves','mfg_material_document_effects','mfg_material_issue_values','mfg_material_value_allocations','shared_item_costs']:
    count=int(helpers['query']('SELECT count(*) FROM '+table+' WHERE tenant_id IN ('+scope+');'));check('no source review/adoption posting:'+table,count,0)
balances=json.loads(helpers['query']("SELECT json_agg(json_build_object('tenant',tenant_id,'hand',on_hand_micros,'reserved',reserved_micros,'frozen',frozen_micros,'transit',in_transit_micros)) FROM inventory_balances WHERE tenant_id IN ("+scope+');'))
check('four genuine initial stock rows',len(balances),4)
for row in balances:check('stock unchanged:'+row['tenant'],tuple(row[k] for k in ['hand','reserved','frozen','transit']),(4900000,900000,900000,2300000))
failures=[c for c in checks if not c['pass']]
report={'status':'PASS' if not failures else 'PARTIAL','checks':len(checks),'failures':failures,'assertions':checks,
        'tests':{'passed':int(summary[2]),'failed':int(summary[3]),'ignored':int(summary[4]),'seconds':float(summary[5])},
        'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),
        'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),
        'source_revisions':len(stored),'successful_original_responses':success,'actual_adoption_revisions':len(adopted),
        'scope':'actual explicit source review/adoption and full-authorized bounded pages; not formal UI, Worker, Release or hosted ERP'}
dest=BASE/(prefix+'-independent.json')
if dest.exists():raise SystemExit('preserve original independent evidence')
dest.write_text(json.dumps(report,indent=2)+'\n');print(report['status'],len(checks),'checks;',report['tests'])
if failures:raise SystemExit(1)
