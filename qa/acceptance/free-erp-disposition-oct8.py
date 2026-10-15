"""Read-only original source, exact effect and older enterprise verification."""
import argparse,hashlib,json,re,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'qa/acceptance'; PREFIX='free-erp-disposition-oct8-'
p=argparse.ArgumentParser();p.add_argument('action',choices=['capture','verify']);p.add_argument('stage');p.add_argument('--recheck',action='store_true');a=p.parse_args()
if not re.fullmatch('[a-z0-9-]+',a.stage):raise SystemExit('invalid immutable stage')
if a.action=='capture':
    sql=(BASE/'free-erp-receipt-authority-readback.sql').read_text(encoding='utf-8-sig')
    extra=['mfg_exceptions','shared_purchase_revisions','shared_purchase_lines','mfg_quality_holds','mfg_quality_dispositions','mfg_cost_runs','mfg_receipt_disposition_commands','mfg_receipt_disposition_costs']
    assert sql.count("'mfg_reports'")==3 and sql.count("'selected_tables',77")==1
    sql=sql.replace("'mfg_reports'","'mfg_reports',"+','.join("'"+x+"'" for x in extra)).replace("'selected_tables',77","'selected_tables',85")
    script=BASE/(PREFIX+a.stage+'.sql')
    with script.open('x',encoding='utf8') as f:f.write(sql)
    out=BASE/(PREFIX+a.stage+'.jsonl');err=BASE/(PREFIX+a.stage+'.stderr.log')
    with out.open('xb') as f,err.open('xb') as e:
        result=subprocess.run(['G:/PostgreSQL/17/bin/psql.exe','-X','-q','-t','-A','-h','127.0.0.1','-p','15438','-U','erp_grid_test','-d','erp_shared_contract_test','-v','ON_ERROR_STOP=1','-f',str(script)],stdout=f,stderr=e,check=False)
    print(json.dumps({'stage':a.stage,'exit':result.returncode,'bytes':out.stat().st_size}));raise SystemExit(result.returncode)
destination=BASE/(PREFIX+a.stage+('-recheck' if a.recheck else '')+'-independent.json')
if destination.exists():raise SystemExit('refusing to replace evidence')
log=(BASE/(PREFIX+a.stage+'-test.log')).read_text(encoding='utf-8-sig')
owned=set(re.findall(r'QA source_disposition_fixture tenant=([a-f0-9-]+)',log))
checks=0;errors=[]
def check(ok,msg):
    global checks
    checks+=1
    if not ok:errors.append(msg)
def read(name):
    signatures={};rows={};identity=None
    with (BASE/(PREFIX+a.stage+'-'+name+'.jsonl')).open('rb') as stream:
        for raw in stream:
            if not raw.strip():continue
            row=json.loads(raw)
            if 'database' in row:identity=row;continue
            tenant=row['tenant_id'];check(tenant not in signatures,'unique tenant '+tenant)
            check(len(row['facts'])==85,'85 selected tables '+tenant)
            signatures[tenant]=hashlib.sha256(json.dumps(row,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
            if tenant in owned:rows[tenant]=row['facts']
    check(identity and identity['selected_tables']==85 and identity['database']=='erp_shared_contract_test' and identity['role']=='erp_grid_test' and identity['port']==15438 and identity['host']=='127.0.0.1','protected identity '+name)
    return signatures,rows
before,_=read('before');after,rows=read('after')
changed=[t for t,v in before.items() if after.get(t)!=v]
for tenant in before:check(tenant not in changed,'previous enterprise unchanged '+tenant)
for tenant,facts in rows.items():
    commands=facts['mfg_receipt_disposition_commands'];balances=facts['inventory_balances'];ledgers=facts['inventory_ledgers']
    for balance in balances:
        canonical=[r for r in ledgers if r['warehouse_id']==balance['warehouse_id'] and r['sku_id']==balance['sku_id'] and r['ref_type']!='mfg_material_move']
        for field in ['on_hand','reserved','frozen','in_transit']:
            check(sum(r['delta_'+field+'_micros'] for r in canonical)==balance[field+'_micros'],'exact canonical conservation '+tenant+' '+field)
    for command in commands:
        receipt=[r for r in facts['command_receipts'] if r['request_id']==command['request_id']]
        check(len(receipt)==1 and receipt[0]['payload_hash']==command['payload_hash'] and receipt[0]['effect_id']==command['disposition_id'],'original receipt identity '+tenant)
        effects=command['snapshot']['effects']
        check(len(effects)==(1 if command['kind'] in ['SCRAP','RETURN_SUPPLIER','REWORK'] else 2),'exact effect leg count '+tenant)
        for effect in effects:
            stocks=[r for r in facts['shared_stock_ledgers'] if r['ledger_id']==effect['stock_ledger_id']]
            inv=[r for r in ledgers if r['ledger_id']==effect['inventory_ledger_id']]
            check(len(stocks)==len(inv)==1 and stocks[0]['delta_micros']==inv[0]['delta_on_hand_micros']==effect['delta_micros'],'actual source effect '+tenant)
        ops=[r for r in facts['operation_logs'] if r['action']=='QUALITY_RECEIPT_DISPOSITION' and r.get('metadata',{}).get('result',{}).get('disposition_id')==command['disposition_id']]
        check(len(ops)==1,'required original operator log '+tenant)
        if command['kind']=='REWORK':
            moves=[r for r in facts['mfg_material_moves'] if r['move_id']==command['material_move_id']]
            check(len(moves)==1 and moves[0]['qty_micros']==command['qty_micros'] and moves[0]['work_order_id']==command['rework_work_order_id'],'actual rework issue '+tenant)
    if commands and any(c['kind']=='SCRAP' for c in commands):
        costs=facts['mfg_receipt_disposition_costs']
        check(all(sum(c['cost_cent'] for c in commands if c['receipt_id']==s['receipt_id'] and c['kind'] in ['SCRAP','RETURN_SUPPLIER','REWORK'])<=s['total_cent'] for s in costs),'original cost does not multiply '+tenant)
source=[json.loads((BASE/f'free-erp-inventory-consumers-disposition-oct8-{a.stage}-{phase}-source.json').read_text(encoding='utf8'))['files'] for phase in ['before','after']]
drift=[f for f in set(source[0])|set(source[1]) if source[0].get(f)!=source[1].get(f)]
check(not drift,'source inputs stable')
result=re.search(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; (\d+) filtered out;',log)
check(result is not None,'actual registered suite result')
report={'status':'PASS' if not errors else 'PARTIAL','checks':checks,'errors':errors,'previous_enterprises':len(before),'protected_tables':85,'changed_previous_enterprises':changed,'owned_observed_enterprises':len(owned),'source_inputs':len(source[0]),'source_drift':drift,'passed':int(result[1]) if result else None,'filtered':int(result[2]) if result else None,'scope':'source-linked quality decisions and canonical inventory evidence; not full quality/cost/ERP/public/UI/Release acceptance'}
with destination.open('x',encoding='utf8') as f:json.dump(report,f,indent=2);f.write('\n')
print(json.dumps(report));raise SystemExit(0 if not errors else 1)
