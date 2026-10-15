"""Read-only original source, exact effect and older enterprise verification."""
import argparse,hashlib,json,re,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'qa/acceptance'; PREFIX='free-erp-cost-allocation-oct8-'
p=argparse.ArgumentParser();p.add_argument('action',choices=['capture','verify']);p.add_argument('stage');p.add_argument('--recheck',action='store_true');a=p.parse_args()
if not re.fullmatch('[a-z0-9-]+',a.stage):raise SystemExit('invalid immutable stage')
if a.action=='capture':
    sql=(BASE/'free-erp-receipt-authority-readback.sql').read_text(encoding='utf-8-sig')
    extra=['mfg_exceptions','shared_purchase_revisions','shared_purchase_lines','mfg_quality_holds','mfg_quality_dispositions','mfg_cost_runs','mfg_receipt_disposition_commands','mfg_receipt_disposition_costs','mfg_rework_material_values','mfg_material_cost_bases','mfg_material_issue_values','mfg_material_value_allocations','shared_item_costs','mfg_cost_commit_authorities','shared_work_order_routes','operation_wip','mfg_wip_balances','mfg_cost_allocations']
    assert sql.count("'mfg_reports'")==3 and sql.count("'selected_tables',77")==1
    sql=sql.replace("'mfg_reports'","'mfg_reports',"+','.join("'"+x+"'" for x in extra)).replace("'selected_tables',77","'selected_tables',95")
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
            check(len(row['facts'])==95,'95 selected tables '+tenant)
            signatures[tenant]=hashlib.sha256(json.dumps(row,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
            if tenant in owned:rows[tenant]=row['facts']
    check(identity and identity['selected_tables']==95 and identity['database']=='erp_shared_contract_test' and identity['role']=='erp_grid_test' and identity['port']==15438 and identity['host']=='127.0.0.1','protected identity '+name)
    return signatures,rows
before,_=read('before');after,rows=read('after')
changed=[t for t,v in before.items() if after.get(t)!=v]
for tenant in before:check(tenant not in changed,'previous enterprise unchanged '+tenant)
for tenant,facts in rows.items():
    commands=facts['mfg_receipt_disposition_commands'];balances=facts['inventory_balances'];ledgers=facts['inventory_ledgers']
    for balance in balances:
        canonical=[r for r in ledgers if r['warehouse_id']==balance['warehouse_id'] and r['sku_id']==balance['sku_id'] and not (r.get('ref_type')=='mfg_material_move' and r.get('reason_type')=='MFG_RECEIPT' and any(str(c['receipt_id'])==r.get('ref_id') for c in facts['mfg_receipts'])) and r['ledger_id'] not in {l['ledger_id'] for l in ledgers if l.get('ref_type')=='mfg_material_move' and any(c['ledger_id']!=l['ledger_id'] and c.get('ref_type')=='shared_stock_ledger' and c.get('ref_id')==l.get('ref_id') and c['delta_on_hand_micros']==l['delta_on_hand_micros'] and c['delta_frozen_micros']==l['delta_frozen_micros'] for c in ledgers)}]
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
        check(all(sum(c['cost_cent'] for c in commands if c['receipt_id']==s['receipt_id'] and c['kind'] in ['SCRAP','RETURN_SUPPLIER','REWORK'])-sum(v['cost_cent'] for v in facts['mfg_rework_material_values'] if v['receipt_id']==s['receipt_id'] and v['kind']=='RETURN')<=s['total_cent'] for s in costs),'original cost does not multiply '+tenant)
    values=facts['mfg_rework_material_values']
    for value in values:
        move=[r for r in facts['mfg_material_moves'] if r['move_id']==value['move_id']]
        check(len(move)==1 and move[0]['qty_micros']==value['qty_micros'] and move[0]['kind']==value['kind'] and move[0]['original_issue_id']==value['original_issue_id'],'actual rework material '+tenant)
        originals=[c for c in commands if c['disposition_id']==value['disposition_id']]
        check(len(originals)==1 and originals[0]['material_move_id']==value['original_issue_id'],'original rework value source '+tenant)
        if value['kind']=='RETURN':
            effects=[r for r in ledgers if r.get('ref_id')==value['move_id'] and r.get('ref_type')=='mfg_material_move']
            check(len(effects)==1 and effects[0]['delta_on_hand_micros']==effects[0]['delta_frozen_micros']==value['qty_micros'],'returned material stays frozen '+tenant)
        else:
            check(not any(r.get('ref_id')==value['move_id'] and r.get('ref_type')=='mfg_material_move' for r in ledgers),'consume does not remove warehouse twice '+tenant)
        check(sum(v['cost_cent'] for v in values if v['original_issue_id']==value['original_issue_id'])<=originals[0]['cost_cent'],'allocated rework value bounded '+tenant)
        ops=[r for r in facts['operation_logs'] if r['action']=='REWORK_MATERIAL' and r.get('ref_id')==value['move_id']]
        audit=[r for r in facts['audit_logs'] if r['action']=='manufacturing.rework.material' and r.get('entity_id')==value['move_id']]
        check(len(ops)==len(audit)==1,'current actual rework actor logs '+tenant)
        events=[r for r in facts['outbox_events'] if r['event_type']=='manufacturing.rework.material' and r['payload'].get('move_id')==value['move_id']]
        check(len(events)==1,'required original rework event '+tenant)
    for basis in facts['mfg_material_cost_bases']:
        runs=[r for r in facts['mfg_cost_runs'] if r['cost_run_id']==basis['cost_run_id']]
        check(len(runs)==1 and runs[0]['material_cent']==basis['material_cent']==basis['snapshot']['material_cent'],'frozen cost producer basis '+tenant)
        source=basis['snapshot']['rework_consumptions']
        check(sum(r['value_cent'] for r in source)==basis['rework_cent'],'actual consumed original value '+tenant)
        for entry in source:
            allocations=[r for r in values if r['move_id']==entry['move_id']]
            check(len(allocations)==1 and allocations[0]['kind']=='CONSUME' and allocations[0]['cost_cent']==entry['value_cent'] and allocations[0]['qty_micros']==entry['qty_micros'],'cost references immutable consumption '+tenant)
    for allocation in facts['mfg_material_value_allocations']:
        issue=[r for r in facts['mfg_material_moves'] if r['move_id']==allocation['issue_id']]
        check(len(issue)==1,'original ordinary issue exists '+tenant)
        total=sum(v['qty_micros'] for v in facts['mfg_material_value_allocations'] if v['issue_id']==allocation['issue_id'])
        check(total<=issue[0]['qty_micros'],'ordinary consumed and returned quantity bounded '+tenant)
        if allocation['cost_cent'] is not None:
            original=[r for r in facts['mfg_material_issue_values'] if r['issue_id']==allocation['issue_id']]
            if original and original[0]['total_cent'] is not None:
                check(sum(v['cost_cent'] for v in facts['mfg_material_value_allocations'] if v['issue_id']==allocation['issue_id'])<=original[0]['total_cent'],'ordinary original value bounded '+tenant)
    for basis in facts['mfg_material_cost_bases']:
        ordinary=basis['snapshot'].get('ordinary_consumptions',[])
        check(sum(v['cost_cent'] for v in ordinary)==basis['snapshot'].get('ordinary_cent',0),'ordinary consumed cost sum '+tenant)
        check(basis['material_cent']==basis['rework_cent']+basis['snapshot'].get('ordinary_cent',0),'cost totals by actual sources '+tenant)
        for value in ordinary:
            actual=[r for r in facts['mfg_material_value_allocations'] if r['allocation_id']==value['allocation_id']]
            check(len(actual)==1 and actual[0]['kind']=='CONSUME' and actual[0]['cost_cent']==value['cost_cent'],'actual ordinary allocation in cost basis '+tenant)
    for lot in facts['mfg_material_issue_values']:
        if lot['state']=='VALUED':
            book=[r for r in facts['shared_item_costs'] if r['source_ref']=='MFG:'+lot['issue_id'] and r['kind']=='ISSUE']
            check(len(book)==1 and book[0]['qty_micros']==lot['qty_micros'] and book[0]['amount_cent']==lot['total_cent'] and book[0]['item_code']==lot['snapshot']['price']['sku_code'],'actual original SKU cost-book debit '+tenant)
            returned=[v for v in facts['mfg_material_value_allocations'] if v['issue_id']==lot['issue_id'] and v['kind']=='RETURN' and v['cost_cent'] is not None]
            credits=[r for r in facts['shared_item_costs'] if r['source_ref']=='MFG:'+lot['issue_id'] and r['kind']=='RETURN']
            check(sum(r['amount_cent'] for r in credits)==sum(v['cost_cent'] for v in returned) and sum(r['qty_micros'] for r in credits)==sum(v['qty_micros'] for v in returned),'actual original-value return to cost book '+tenant)
        movements=[lot['issue_id']]+[v['allocation_id'] for v in facts['mfg_material_value_allocations'] if v['issue_id']==lot['issue_id']]
        for movement in movements:
            if not any(m['move_id']==movement for m in facts['mfg_material_moves']):continue
            ops=[r for r in facts['operation_logs'] if r['action']=='MATERIAL_SOURCE_VALUE' and r.get('ref_id')==movement]
            audit=[r for r in facts['audit_logs'] if r['action']=='manufacturing.material.source_value' and r.get('entity_id')==movement]
            event=[r for r in facts['outbox_events'] if r['event_type']=='manufacturing.material.source_value' and r['payload'].get('move_id')==movement]
            check(len(ops)==len(audit)==len(event)==1,'material value mandatory logs and event '+tenant)
    for authority in facts['mfg_cost_commit_authorities']:
        run=[r for r in facts['mfg_cost_runs'] if r['cost_run_id']==authority['cost_run_id']]
        command=[r for r in facts['command_receipts'] if r['request_id']==authority['request_id']]
        check(len(run)==len(command)==1 and command[0]['effect_id']==authority['cost_run_id'],'actual cost authority and original command '+tenant)
        proof=authority['snapshot']['quantities']
        check(run[0]['completed_qty_micros']==proof['good_micros'],'cost quantity frozen from final output '+tenant)
        ops=[r for r in facts['operation_logs'] if r['action']=='MFG_COST_COMMIT' and r.get('ref_id')==authority['cost_run_id']]
        audit=[r for r in facts['audit_logs'] if r['action']=='manufacturing.cost.commit' and r.get('entity_id')==authority['cost_run_id']]
        event=[r for r in facts['outbox_events'] if r['event_type']=='manufacturing.cost.committed' and r['payload'].get('cost_run_id')==authority['cost_run_id']]
        check(len(ops)==len(audit)==len(event)==1 and ops[0]['actor_user_id']==authority['user_id'],'actual current cost actor and common commit '+tenant)
    for allocation in facts['mfg_cost_allocations']:
        run=[r for r in facts['mfg_cost_runs'] if r['cost_run_id']==allocation['cost_run_id']]
        check(len(run)==1,'allocation original run '+tenant)
        check(allocation['finished_cent']+allocation['wip_cent']+allocation['rejected_cent']==allocation['total_cent']==run[0]['total_cent'],'three destinations conserve one pool '+tenant)
        check(run[0]['unfinished_cent']==allocation['wip_cent'] and run[0]['defect_cent']==allocation['rejected_cent'],'run destinations agree '+tenant)
        check((allocation['finished_qty_micros']>0)==(allocation['unit_cent'] is not None) and run[0]['unit_cent']==allocation['unit_cent'],'undefined unit price remains null '+tenant)
        check(allocation['policy']=='QUANTITY_SHARE_V1','explicit immutable allocation policy '+tenant)
source=[json.loads((BASE/f'free-erp-inventory-consumers-cost-allocation-oct8-{a.stage}-{phase}-source.json').read_text(encoding='utf8'))['files'] for phase in ['before','after']]
drift=[f for f in set(source[0])|set(source[1]) if source[0].get(f)!=source[1].get(f)]
check(not drift,'source inputs stable')
result=re.search(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; 0 measured; (\d+) filtered out;',log)
check(result is not None and int(result[2])==0 and int(result[3])==0,'actual registered suite result')
report={'status':'PASS' if not errors else 'PARTIAL','checks':checks,'errors':errors,'previous_enterprises':len(before),'protected_tables':95,'changed_previous_enterprises':changed,'owned_observed_enterprises':len(owned),'source_inputs':len(source[0]),'source_drift':drift,'passed':int(result[1]) if result else None,'failed':int(result[2]) if result else None,'ignored':int(result[3]) if result else None,'filtered':int(result[4]) if result else None,'scope':'source-linked quality decisions and canonical inventory evidence; not full quality/cost/ERP/public/UI/Release acceptance'}
with destination.open('x',encoding='utf8') as f:json.dump(report,f,indent=2);f.write('\n')
print(json.dumps(report));raise SystemExit(0 if not errors else 1)
