"""Independent material document effects, actual quantities/values and protection."""
import argparse
import hashlib
import json
import re
import runpy
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'qa/acceptance'
parser=argparse.ArgumentParser();mode=parser.add_mutually_exclusive_group();mode.add_argument('--edge',action='store_true');mode.add_argument('--full',action='store_true');args=parser.parse_args()
prefix='free-erp-material-confirm-full-oct10' if args.full else 'free-erp-material-confirm-oct10-edge' if args.edge else 'free-erp-material-confirm-oct9-fixed'
phase='material-confirm-full' if args.full else 'material-confirm-edge' if args.edge else 'material-confirm-fixed'
log=(BASE/(prefix+'-full.log' if args.full else prefix+'-tests.log')).read_text(encoding='utf-8-sig')
summary=re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s',log)
if not summary:raise SystemExit('no actual terminal result')
before=json.loads((BASE/('free-erp-complete-schema-oct8-'+phase+'-before.json')).read_text())
after=json.loads((BASE/('free-erp-complete-schema-oct8-'+phase+'-after.json')).read_text())
checks=[]
def check(name,actual,expected):
    row={'name':name,'pass':actual==expected}
    if not row['pass']:row.update(actual=actual,expected=expected)
    checks.append(row)
check('registered cases/zero failures/zero ignores',tuple(map(int,summary.group(2,3,4))),(591 if args.full else 8 if args.edge else 6,0,0))
check('source window unchanged',after['source_sha256'],before['source_sha256'])
for table,hashes in before['protected_table_hashes'].items():check('all old row values:'+table,after['protected_table_hashes'].get(table),hashes)
migration=json.loads((BASE/'free-erp-material-confirm-oct9-migration.json').read_text())
check('actual 322-file QA append',(migration['status'],migration['migration_count']),('PASS',322))
check('confirmation migration unchanged',hashlib.sha256((ROOT/'migrations/0313_material_document_confirmation.sql').read_bytes()).hexdigest(),migration['sha256'])
tenants=sorted(set(re.findall(r'QA material_confirm tenant=([a-f0-9-]{36})',log)))
check('own material confirmation fixtures',len(tenants),11 if args.edge or args.full else 9)
helpers=runpy.run_path(str(BASE/'free-erp-product-web-fixture.py'));scope=','.join(helpers['literal'](t) for t in tenants)
heads=json.loads(helpers['query']("SELECT coalesce(json_agg(json_build_object('tenant',d.tenant_id,'document',d.document_id,'order',d.work_order_id,'version',d.version_no,'snapshot',d.posted_snapshot,'request',d.posted_request_id,'receipt_status',c.status,'effect',c.effect_id)),'[]'::json) FROM mfg_material_documents d JOIN command_receipts c ON c.tenant_id=d.tenant_id AND c.request_id=d.posted_request_id WHERE d.status='POSTED' AND d.tenant_id IN ("+scope+');'))
stored={(h['tenant'],h['document']):h for h in heads};success=[]
for line in log.splitlines():
    if line.startswith('QA production_release_http ') and '/confirm ' in line and ' status=200 response=' in line:
        tenant=re.search(r'tenant=([a-f0-9-]{36})',line)[1];response=json.loads(line.split(' response=',1)[1]);snapshot=response['snapshot'];success.append((tenant,snapshot))
        head=stored[(tenant,snapshot['document_id'])]
        check('actual original posted result:'+tenant+':'+snapshot['request_id'],snapshot,head['snapshot'])
        check('original success effect:'+snapshot['document_id'],(head['receipt_status'],head['effect']),('SUCCEEDED',snapshot['document_id']))
rows=json.loads(helpers['query']("SELECT coalesce(json_agg(json_build_object('tenant',e.tenant_id,'document',e.document_id,'line',e.line_id,'move',e.move_id,'snapshot',e.snapshot,'kind',m.kind,'quantity',m.qty_micros::text,'order',m.work_order_id,'warehouse',m.warehouse_id,'sku',m.sku_id,'batch',m.batch_id,'original',m.original_issue_id,'issue_value',v.total_cent::text,'allocated_value',a.cost_cent::text)),'[]'::json) FROM mfg_material_document_effects e JOIN mfg_material_moves m USING(tenant_id,move_id) LEFT JOIN mfg_material_issue_values v ON v.tenant_id=e.tenant_id AND v.issue_id=e.move_id LEFT JOIN mfg_material_value_allocations a ON a.tenant_id=e.tenant_id AND a.allocation_id=e.move_id WHERE e.tenant_id IN ("+scope+');'))
effects={(r['tenant'],r['move']):r for r in rows}
for head in heads:
    snapshot=head['snapshot'];check('posted header identity:'+head['document'],(snapshot['document_id'],snapshot['work_order_id'],snapshot['version_no'],snapshot['request_id'],snapshot['status'],snapshot['posted']),(head['document'],head['order'],head['version'],head['request'],'POSTED',True))
    check('all actual lines applied:'+head['document'],len(snapshot['effects']),len(snapshot['lines']))
    for effect in snapshot['effects']:
        row=effects[(head['tenant'],effect['move_id'])];check('immutable effect snapshot:'+row['move'],row['snapshot'],effect)
        check('real movement identity:'+row['move'],tuple(row[k] for k in ['line','kind','quantity','order','warehouse','sku','batch','original']),
              (effect['line_id'],effect['kind'],effect['qty_micros'],head['order'],effect['warehouse_id'],effect['sku_id'],effect['batch_id'],effect['original_issue_id']))
        actual=row['issue_value'] if effect['kind'] in ['ISSUE','SUPPLEMENT'] else row['allocated_value'];check('actual original cents:'+row['move'],actual,effect['value']['amount_cent'])
    body=dict(snapshot)
    for key in ['operation_log_id','audit_id','event_id']:body.pop(key)
    actual=json.loads(helpers['query']("SELECT json_build_object('operation',(SELECT metadata FROM operation_logs WHERE tenant_id="+helpers['literal'](head['tenant'])+" AND log_id="+helpers['literal'](snapshot['operation_log_id'])+"),'audit',(SELECT after_json FROM audit_logs WHERE tenant_id="+helpers['literal'](head['tenant'])+" AND audit_id="+helpers['literal'](snapshot['audit_id'])+"),'event',(SELECT payload FROM outbox_events WHERE tenant_id="+helpers['literal'](head['tenant'])+" AND outbox_id="+helpers['literal'](snapshot['event_id'])+"));"))
    for key in ['operation','audit','event']:check('actual required '+key+':'+head['document'],actual[key],body)
sample=[(t,s) for t,s in success if s['request_id']=='return']
check('one real consume/return sample',len(sample),1)
tenant,returned=sample[0];issue=next(s for t,s in success if t==tenant and s['request_id']=='issue');consume=next(s for t,s in success if t==tenant and s['request_id']=='consume')
check('issue/consume/return original values',[s['effects'][0]['value']['amount_cent'] for s in [issue,consume,returned]],['300','150','150'])
check('stock is not deducted twice',[s['effects'][0]['stock']['on_hand_micros'] for s in [issue,consume,returned]],['750000','750000','875000'])
current=json.loads(helpers['query']("SELECT json_build_object('hand',(SELECT on_hand_micros::text FROM inventory_balances WHERE tenant_id="+helpers['literal'](tenant)+"),'qty',coalesce(sum(case when kind in('RECEIPT','RETURN') then qty_micros when kind='ISSUE' then -qty_micros else 0 end),0)::text,'value',coalesce(sum(case when kind in('RECEIPT','RETURN','ADJUST') then amount_cent when kind='ISSUE' then -amount_cent else 0 end),0)::text) FROM shared_item_costs WHERE tenant_id="+helpers['literal'](tenant)+';'))
check('actual warehouse/value conservation',(current['hand'],current['qty'],current['value']),('875000','875000','1050'))
check('100 actual lines succeeded',any(len(s['effects'])==100 for _,s in success),True)
if args.edge or args.full:
    check('known zero cost actually posted',any(s['request_id']=='zero-confirm' and s['effects'][0]['value']['amount_cent']=='0' for _,s in success),True)
    check('opposite order confirmations both succeeded',sorted({s['request_id'] for _,s in success if s['request_id'] in ['forward','reverse']}),['forward','reverse'])
failures=[r for r in checks if not r['pass']]
report={'status':'PASS' if not failures else 'PARTIAL','checks':len(checks),'failures':failures,'assertions':checks,
        'tests':{'passed':int(summary[2]),'failed':int(summary[3]),'ignored':int(summary[4]),'seconds':float(summary[5])},
        'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),
        'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),
        'successful_posted_responses':len(success),'actual_documents':len(heads),'actual_line_effects':len(rows),
        'scope':'real material confirmation and original value/quantity results; not UI, Worker, Release, fresh 322-file install or hosted product'}
destination=BASE/(prefix+'-independent.json')
if destination.exists():raise SystemExit('preserve independent evidence')
destination.write_text(json.dumps(report,indent=2)+'\n');print(report['status'],len(checks),'checks;',report['tests'])
if failures:raise SystemExit(1)
