"""Independently bind definite material rejection HTTP facts to stored QA effects."""
import argparse
import hashlib
import json
import re
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
p = argparse.ArgumentParser()
p.add_argument('--mode', choices=['fixed', 'edge', 'full'], default='fixed')
args = p.parse_args()
phase = 'material-rejection-' + args.mode
prefix = 'free-erp-material-rejection-oct10-' + args.mode
log = (BASE / (prefix + '-tests.log')).read_text(encoding='utf-8-sig')
summary = re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out; finished in ([\d.]+)s', log)
if not summary:
    raise SystemExit('missing actual terminal result')
before = json.loads((BASE / ('free-erp-complete-schema-oct8-' + phase + '-before.json')).read_text())
after = json.loads((BASE / ('free-erp-complete-schema-oct8-' + phase + '-after.json')).read_text())
checks = []
def check(name, actual, expected):
    row = {'name': name, 'pass': actual == expected}
    if not row['pass']:
        row.update(actual=actual, expected=expected)
    checks.append(row)
expected = 604 if args.mode == 'full' else 5 if args.mode == 'edge' else 4
check('actual tests/no failures/no ignores', tuple(map(int, summary.group(2,3,4))), (expected,0,0))
if args.mode == 'full':
    check('all registered cases executed', int(summary[5]), 0)
check('compiled source window unchanged', after['source_sha256'], before['source_sha256'])
for table, hashes in before['protected_table_hashes'].items():
    check('protected historical rows:' + table, after['protected_table_hashes'].get(table), hashes)
helpers = runpy.run_path(str(BASE / 'free-erp-product-web-fixture.py'))
http = []
for line in log.splitlines():
    if line.startswith('QA production_release_http ') and '/material-documents' in line:
        match = re.match(r'QA production_release_http tenant=([a-f0-9-]{36}) path=(.*?) input=(.*?) status=(\d+) response=(.*)', line)
        if match:
            tenant, path, body, status, response = match.groups()
            http.append((tenant,path,json.loads(body),int(status),json.loads(response)))
rejected = [row for row in http if row[4].get('code') == 'material_document_rejected_before_commit']
check('actual matched rejection declarations', len(rejected), 2 if args.mode == 'fixed' else 4)
expected_ids = ['confirm-reject','reject-source']
if args.mode != 'fixed':
    expected_ids += ['version-reject','adoption-reject']
check('all rejected original IDs', sorted(r[2]['request_id'] for r in rejected), sorted(expected_ids))
tenants = {r[0] for r in rejected}
for request in ['sql-failure','known-save']:
    tenants.update(t for t,path,b,status,r in http if b.get('request_id') == request and 'rejection_envelope=true' in path)
check('owned protocol fixtures', len(tenants), 4 if args.mode == 'fixed' else 5)
for tenant,path,body,status,response in rejected:
    request = body['request_id']
    fields = json.loads(json.dumps(body))
    for line in fields.get('lines',[]):
        for key in ['batch_id','original_issue_id','reason']:
            line.setdefault(key,None)
    check('decoded exact DTO:' + request, response['fields'], fields)
    check('matched request/committed:' + request, (status,response['request_id'],response['committed']), (200,request,False))
    context = response['context']
    order = path.split('/')[1]
    document = body.get('document_id') or path.split('/')[3]
    actual = json.loads(helpers['query']("SELECT json_build_object('tenant_id',tenant_id,'work_order_id',work_order_id,'document_id',document_id,'actor_user_id',(SELECT actor_user_id FROM mfg_material_document_revisions r WHERE r.tenant_id=d.tenant_id AND r.document_id=d.document_id ORDER BY version_no LIMIT 1)) FROM mfg_material_documents d WHERE tenant_id=" + helpers['literal'](tenant) + " AND document_id=" + helpers['literal'](document) + ';'))
    check('actual tenant/actor/work-order/document:' + request, context, actual)
    check('path work order:' + request, context['work_order_id'], order)
    count = int(helpers['query']("SELECT count(*) FROM command_receipts WHERE tenant_id=" + helpers['literal'](tenant) + " AND request_id=" + helpers['literal'](request) + ';'))
    check('rejected command never stored:' + request, count, 0)
    count = int(helpers['query']("SELECT count(*) FROM mfg_material_document_revisions WHERE tenant_id=" + helpers['literal'](tenant) + " AND request_id=" + helpers['literal'](request) + ';'))
    check('no rejected immutable revision:' + request, count, 0)
for request,status in [('sql-failure',500),('known-save',409),('known-save',403)]:
    rows = [r for r in http if r[2].get('request_id') == request and r[3] == status and 'rejection_envelope=true' in r[1]]
    check('ordinary unknown/auth/replay status:' + request + ':' + str(status), len(rows), 1)
    for row in rows:
        check('no unsafe unlock:' + request + ':' + str(status), 'committed' in row[4], False)
scope = ','.join(helpers['literal'](t) for t in sorted(tenants))
for table in ['mfg_material_document_effects','mfg_material_moves','mfg_material_issue_values','mfg_material_value_allocations']:
    count = int(helpers['query']('SELECT count(*) FROM ' + table + ' WHERE tenant_id IN (' + scope + ');'))
    check('rejections and drafts produce no stock/value:' + table, count, 0)
balances = json.loads(helpers['query']("SELECT json_agg(json_build_object('tenant',tenant_id,'hand',on_hand_micros::text,'reserved',reserved_micros::text,'frozen',frozen_micros::text,'transit',in_transit_micros::text)) FROM inventory_balances WHERE tenant_id IN (" + scope + ');'))
check('actual stock row count', len(balances), len(tenants))
for row in balances:
    check('stock stayed intact:' + row['tenant'], [row[k] for k in ['hand','reserved','frozen','transit']], ['1000000','0','0','0'])
for tenant in sorted(tenants):
    facts = json.loads(helpers['query']("SELECT json_build_object('quantity',coalesce(sum(qty_micros),0)::text,'value',coalesce(sum(amount_cent),0)::text,'rows',count(*)) FROM shared_item_costs WHERE tenant_id=" + helpers['literal'](tenant) + ';'))
    check('actual opening cost intact:' + tenant, facts, {'quantity':'1000000','value':'1200','rows':1})
correction = [r for r in http if r[2].get('request_id') == 'fresh-correction' and r[3] == 200]
check('one real new-ID correction', len(correction), 1)
for tenant,path,body,status,response in correction:
    snapshot = response['snapshot']
    stored = json.loads(helpers['query']("SELECT snapshot FROM mfg_material_document_revisions WHERE tenant_id=" + helpers['literal'](tenant) + " AND request_id='fresh-correction';"))
    check('actual correction revision stored', stored, snapshot)
    check('correction preserves draft and exact quantity', (snapshot['version_no'],snapshot['status'],snapshot['posted'],snapshot['lines'][0]['qty_micros']), (2,'DRAFT',False,'500000'))
failures = [c for c in checks if not c['pass']]
report = {'status':'PASS' if not failures else 'PARTIAL','checks':len(checks),'failures':failures,'assertions':checks,
          'tests':{'passed':int(summary[2]),'failed':int(summary[3]),'ignored':int(summary[4]),'filtered':int(summary[5]),'seconds':float(summary[6])},
          'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),
          'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),
          'actual_rejection_declarations':len(rejected),'actual_protocol_fixtures':len(tenants),
          'scope':'opt-in definite material business rejection; not pending UI, browser/native acceptance, Worker, fresh install, restore, Release or hosting'}
if args.mode == 'full':
    manifest = json.loads((BASE / 'free-erp-material-rejection-oct10-openapi-before.json').read_text())
    actual = {name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest() for name in manifest}
    check('OpenAPI window unchanged', actual, manifest)
    report.update(status='PASS' if all(c['pass'] for c in checks) else 'PARTIAL', checks=len(checks), failures=[c for c in checks if not c['pass']])
dest = BASE / (prefix + '-independent.json')
if dest.exists():
    raise SystemExit('preserve original proof')
dest.write_text(json.dumps(report,indent=2) + '\n',encoding='utf-8')
print(report['status'],report['checks'],'checks;',report['tests'])
if report['failures']:
    raise SystemExit(1)
