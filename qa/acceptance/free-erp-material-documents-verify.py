"""Independent original results, physical non-posting and old-row/source protection."""
import argparse
import hashlib
import json
import re
import runpy
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'qa/acceptance'
parser=argparse.ArgumentParser();parser.add_argument('--full',action='store_true');args=parser.parse_args()
prefix='free-erp-material-documents-full-oct9' if args.full else 'free-erp-material-documents-oct9-fixed'
phase='material-draft-full' if args.full else 'material-draft-fixed'
log=(BASE/(prefix+'-full.log' if args.full else 'free-erp-material-documents-oct9-fixed-tests.log')).read_text(encoding='utf-8-sig')
summary=re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s',log)
if not summary:raise SystemExit('no terminal process result')
before=json.loads((BASE/('free-erp-complete-schema-oct8-'+phase+'-before.json')).read_text())
after=json.loads((BASE/('free-erp-complete-schema-oct8-'+phase+'-after.json')).read_text())
checks=[]
def check(name,actual,expected):
    row={'name':name,'pass':actual==expected}
    if not row['pass']:row.update(actual=actual,expected=expected)
    checks.append(row)
check('actual cases/zero failures/zero ignores',tuple(map(int,summary.group(2,3,4))),(583 if args.full else 6,0,0))
check('same complete source window',after['source_sha256'],before['source_sha256'])
for table,hashes in before['protected_table_hashes'].items():
    check('all old row values:'+table,after['protected_table_hashes'].get(table),hashes)
append=json.loads((BASE/'free-erp-material-documents-oct9-migration.json').read_text())
check('actual 321-file QA append', (append['status'],append['migration_count']),('PASS',321))
check('new migration source unchanged',hashlib.sha256((ROOT/'migrations/0312_material_document_drafts.sql').read_bytes()).hexdigest(),append['sha256'])
tenants=sorted(set(re.findall(r'QA material_document tenant=([a-f0-9-]{36})',log)))
check('six own document fixtures',len(tenants),6)
helpers=runpy.run_path(str(BASE/'free-erp-product-web-fixture.py'))
scope=','.join(helpers['literal'](t) for t in tenants)
rows=json.loads(helpers['query']("SELECT json_agg(json_build_object('tenant',r.tenant_id,'revision',r.revision_id,'request',r.request_id,'snapshot',r.snapshot,'receipt_status',c.status,'effect',c.effect_id)) FROM mfg_material_document_revisions r JOIN command_receipts c USING(tenant_id,request_id) WHERE r.tenant_id IN ("+scope+');'))
stored={(r['tenant'],r['revision']):r for r in rows}
successful=[]
for line in log.splitlines():
    if line.startswith('QA production_release_http ') and '/material-documents ' in line and ' status=200 response=' in line:
        tenant=re.search(r'tenant=([a-f0-9-]{36})',line)[1]
        response=json.loads(line.split(' response=',1)[1]);snapshot=response['snapshot'];successful.append(snapshot)
        row=stored[(tenant,snapshot['revision_id'])]
        check('actual original snapshot:'+tenant+':'+snapshot['request_id'],row['snapshot'],snapshot)
        check('original receipt effect:'+snapshot['revision_id'],row['effect'],snapshot['revision_id'])
        check('draft cannot claim posting:'+snapshot['revision_id'],(snapshot['status'],snapshot['posted']),('DRAFT',False))
check('only deliberate incomplete receipt remains',[(r['request'],r['receipt_status']) for r in rows if r['receipt_status']!='SUCCEEDED'],[('original','RUNNING')])
for table in ['mfg_material_moves','mfg_material_issue_values','mfg_material_value_allocations','shared_item_costs']:
    count=int(helpers['query']('SELECT count(*) FROM '+table+' WHERE tenant_id IN ('+scope+');'))
    check('no draft business posting:'+table,count,0)
physical=json.loads(helpers['query']("SELECT json_agg(json_build_object('tenant',tenant_id,'hand',on_hand_micros,'reserved',reserved_micros,'frozen',frozen_micros,'transit',in_transit_micros)) FROM inventory_balances WHERE tenant_id IN ("+scope+');'))
check('one genuine opening balance per fixture',len(physical),6)
for row in physical:check('actual physical opening retained:'+row['tenant'],tuple(row[k] for k in ['hand','reserved','frozen','transit']),(4900000,900000,900000,2300000))
heads=json.loads(helpers['query']("SELECT json_agg(json_build_object('tenant',d.tenant_id,'document',d.document_id,'version',d.version_no,'status',d.status,'current',r.snapshot,'lines',(SELECT json_agg(json_build_object('line_id',l.line_id,'kind',l.kind,'warehouse_id',l.warehouse_id,'sku_id',l.sku_id,'batch_id',l.batch_id,'original_issue_id',l.original_issue_id,'unit_code',l.unit_code,'qty_micros',l.qty_micros::text,'reason',l.reason) ORDER BY l.ordinal) FROM mfg_material_document_lines l WHERE l.tenant_id=d.tenant_id AND l.document_id=d.document_id))) FROM mfg_material_documents d JOIN mfg_material_document_revisions r USING(tenant_id,document_id,version_no) WHERE d.tenant_id IN ("+scope+');'))
check('six actual document heads',len(heads),6)
for head in heads:
    check('real current lines equal saved revision:'+head['document'],head['lines'],head['current']['lines'])
    check('head version/state equals revision:'+head['document'],(head['version'],head['status']),(head['current']['version_no'],'DRAFT'))
if args.full:
    check('actual 100-line document reached success',any(len(s['lines'])==100 for s in successful),True)
failures=[r for r in checks if not r['pass']]
report={'status':'PASS' if not failures else 'PARTIAL','checks':len(checks),'failures':failures,'assertions':checks,
    'tests':{'passed':int(summary[2]),'failed':int(summary[3]),'ignored':int(summary[4]),'seconds':float(summary[5])},
    'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),
    'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),
    'successful_draft_responses':len(successful),'distinct_revisions':len(stored),
    'scope':'actual material drafts and registered backend; not whole-document material confirmation, UI, Worker, fresh 321-file install, Release or hosting'}
destination=BASE/(prefix+'-independent.json')
if destination.exists():raise SystemExit('preserve prior independent evidence')
destination.write_text(json.dumps(report,indent=2)+'\n');print(report['status'],len(checks),'checks',report['tests'])
if failures:raise SystemExit(1)
