"""Independent actual work orders/BOM rows/warehouse grants and protection."""
import argparse
import json
import re
import runpy
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'qa/acceptance'
p=argparse.ArgumentParser();m=p.add_mutually_exclusive_group();m.add_argument('--fixed',action='store_true');m.add_argument('--full',action='store_true');args=p.parse_args()
prefix='free-erp-manufacturing-catalog-full-oct10' if args.full else 'free-erp-manufacturing-catalog-oct10-fixed' if args.fixed else 'free-erp-manufacturing-catalog-oct10'
phase='manufacturing-catalog-full' if args.full else 'manufacturing-catalog-fixed' if args.fixed else 'manufacturing-catalog'
log=(BASE/(prefix+'-full.log' if args.full else prefix+'-tests.log')).read_text(encoding='utf-8-sig')
summary=re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s',log)
if not summary:raise SystemExit('no actual test terminal')
before=json.loads((BASE/('free-erp-complete-schema-oct8-'+phase+'-before.json')).read_text());after=json.loads((BASE/('free-erp-complete-schema-oct8-'+phase+'-after.json')).read_text())
checks=[]
def check(name,actual,expected):
    row={'name':name,'pass':actual==expected}
    if not row['pass']:row.update(actual=actual,expected=expected)
    checks.append(row)
check('actual cases/no failures/no ignores',tuple(map(int,summary.group(2,3,4))),(599 if args.full else 4,0,0))
check('same source window',after['source_sha256'],before['source_sha256'])
for table,hashes in before['protected_table_hashes'].items():check('all old row values:'+table,after['protected_table_hashes'].get(table),hashes)
tenants=sorted(set(re.findall(r'QA manufacturing_catalog_fixture tenant=([a-f0-9-]{36})',log)));check('five own catalog fixtures',len(tenants),5)
helpers=runpy.run_path(str(BASE/'free-erp-product-web-fixture.py'));scope=','.join(helpers['literal'](t) for t in tenants)
orders=json.loads(helpers['query']("SELECT json_agg(json_build_object('tenant',tenant_id,'work_order_id',work_order_id,'work_order_no',work_order_no,'product_code',product_code,'status',status,'bom_id',bom_id,'order_qty_micros',order_qty_micros::text,'issued_micros',issued_micros::text,'consumed_micros',consumed_micros::text)) FROM mfg_work_orders WHERE tenant_id IN ("+scope+');'))
lookup={(r['tenant'],r['work_order_id']):{k:v for k,v in r.items() if k!='tenant'} for r in orders};contexts=[];pages=[]
for line in log.splitlines():
    if line.startswith('QA manufacturing_catalog tenant=') and ' status=200 response=' in line:
        tenant=re.search(r'tenant=([a-f0-9-]{36})',line)[1];value=json.loads(line.split(' response=',1)[1]);path=line.split(' path=',1)[1].split(' status=',1)[0]
        if path.startswith('work-order-options'):
            pages.append((tenant,path,value))
            for row in value['data']:check('actual order business fields:'+tenant+':'+row['work_order_id'],row,lookup[(tenant,row['work_order_id'])])
        else:contexts.append((tenant,value))
for tenant,value in contexts:
    order=value['work_order'];check('actual context header:'+tenant,order,lookup[(tenant,order['work_order_id'])])
    if order['bom_id']:
        rows=json.loads(helpers['query']("SELECT coalesce(json_agg(json_build_object('line_no',line_no,'parent_code',parent_code,'component_code',component_code,'qty_per_micros',qty_per_micros::text,'unit_code',unit_code,'scrap_ppm',scrap_ppm,'supply_kind',supply_kind) ORDER BY line_no),'[]'::json) FROM shared_mfg_bom_version_lines WHERE tenant_id="+helpers['literal'](tenant)+" AND version_id="+helpers['literal'](order['bom_id'])+';'))
        check('complete actual frozen material lines:'+tenant,[{k:r[k] for k in rows[0]} for r in value['materials']],rows)
        for material in value['materials']:
            if material['sku_id']:
                code=helpers['query']("SELECT sku_code FROM skus WHERE tenant_id="+helpers['literal'](tenant)+" AND sku_id="+helpers['literal'](material['sku_id'])+';').strip()
                check('real SKU mapping:'+tenant+':'+str(material['line_no']),code,material['component_code'])
    # Grant revocation within its own test is recorded in the later response; original reads retain original facts.
    for warehouse in value['warehouses']:
        actual=json.loads(helpers['query']("SELECT json_build_object('warehouse_id',warehouse_id,'code',code,'name',name) FROM warehouses WHERE tenant_id="+helpers['literal'](tenant)+" AND warehouse_id="+helpers['literal'](warehouse['warehouse_id'])+';'))
        check('actual warehouse business label:'+tenant,warehouse,actual)
missing=[value for _,value in contexts if any(r['component_code']=='UNMAPPED-MATERIAL' for r in value['materials'])]
check('unmapped original lines preserved',bool(missing) and all(any(r['component_code']=='UNMAPPED-MATERIAL' and r['sku_id'] is None for r in v['materials']) and v['needs_reconciliation'] for v in missing),True)
literal=[value for _,path,value in pages if 'query=WO_%25_' in path and 'limit=2' in path]
check('two actual literal bounded pages',len(literal),2)
for index,page in enumerate(literal):check('literal full total:'+str(index),page['total'],3)
check('three stable literal business IDs',len({r['work_order_id'] for page in literal for r in page['data']}),3)
check('safe strings beyond JS integer range',any(int(r['order_qty_micros'])>9007199254740991 for page in literal for r in page['data']),True)
for table in ['mfg_material_moves','mfg_material_document_effects','mfg_material_issue_values','mfg_material_value_allocations','shared_item_costs']:
    count=int(helpers['query']('SELECT count(*) FROM '+table+' WHERE tenant_id IN ('+scope+');'));check('catalog reads do not post:'+table,count,0)
failures=[c for c in checks if not c['pass']]
report={'status':'PASS' if not failures else 'PARTIAL','checks':len(checks),'failures':failures,'assertions':checks,
        'tests':{'passed':int(summary[2]),'failed':int(summary[3]),'ignored':int(summary[4]),'seconds':float(summary[5])},
        'protected_tenants':len(before['protected_tenants']),'protected_tables':len(before['protected_table_hashes']),
        'protected_rows':sum(map(len,before['protected_table_hashes'].values())),'source_inputs':len(before['source_sha256']),
        'actual_work_orders':len(orders),'successful_contexts':len(contexts),'actual_pages':len(pages),
        'scope':'actual work-order option queries and material context; not material UI, BOM publication/release, MRP, Worker, Release or hosted ERP'}
dest=BASE/(prefix+'-independent.json')
if dest.exists():raise SystemExit('preserve independent evidence')
dest.write_text(json.dumps(report,indent=2)+'\n');print(report['status'],len(checks),'checks;',report['tests'])
if failures:raise SystemExit(1)
