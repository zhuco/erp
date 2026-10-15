"""Stream independent complete QA facts and verify actual OMS cost/batch outcomes."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

base = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('stage', choices=('original', 'focus', 'full'))
args = parser.parse_args()
prefix = 'free-erp-fulfillment-cost-batch-'
destination = base / (prefix + args.stage + '-independent.json')
if destination.exists():
    raise SystemExit('refusing independent evidence overwrite')
errors = []
checks = 0

def check(condition, label):
    global checks
    checks += 1
    if not condition:
        errors.append(label)

def canonical(rows):
    return sorted(json.dumps(row, sort_keys=True, separators=(',', ':')) for row in rows)

def read_record(stream):
    for line in stream:
        if line.strip():
            return json.loads(line)
    return None

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

log_path = base / (prefix + args.stage + '.log')
raw = log_path.read_bytes()
log = raw.decode('utf-16' if raw[:2] in (b'\xff\xfe', b'\xfe\xff') else 'utf-8-sig')
fixtures = {tenant: dict(case=case, order=order, warehouse=warehouse, sku=sku) for tenant, case, order, warehouse, sku in re.findall(
    r'QA fulfillment cost_batch_fixture tenant=(\S+) case=(\S+) order=(\S+) warehouse=(\S+) sku=(\S+)', log)}
results = re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s', log)
check(len(results) == 1, 'one actual registered test summary')
summary = None
if results:
    status, passed, failed, ignored, seconds = results[0]
    summary = dict(status=status, passed=int(passed), failed=int(failed), ignored=int(ignored), seconds=float(seconds))
    if args.stage == 'original':
        check(summary['failed'] > 0 and summary['passed'] + summary['failed'] == 14 and summary['ignored'] == 0,
              'fourteen original real-route cases executed and failures reproduced')
    else:
        check(summary['status'] == 'ok' and summary['failed'] == summary['ignored'] == 0,
              'current registered tests passed without ignores')
        check(summary['passed'] == (14 if args.stage == 'focus' else 216), 'complete expected target/filter')
check(len(fixtures) == 14 and len({row['case'] for row in fixtures.values()}) == 14, 'fourteen distinct actual case fixtures')
paths = [base / f'free-erp-fulfillment-cost-batch-{args.stage}-{side}.jsonl' for side in ('before', 'after')]
owned = {}
old_count = unchanged_old = 0
old_changes = []
other_new = []
with paths[0].open(encoding='utf-8-sig') as previous_stream, paths[1].open(encoding='utf-8-sig') as current_stream:
    old_catalog, new_catalog = read_record(previous_stream), read_record(current_stream)
    for catalog in (old_catalog, new_catalog):
        check({key: catalog[key] for key in ('database', 'role', 'host', 'port')} == {
            'database': 'erp_shared_contract_test', 'role': 'erp_grid_test', 'host': '127.0.0.1', 'port': 15438}, 'protected actual database identity')
    check(old_catalog['catalog'] == new_catalog['catalog'], 'original table catalog unchanged')
    previous, current = read_record(previous_stream), read_record(current_stream)
    while previous is not None or current is not None:
        if previous is None or (current is not None and current['tenant_id'] < previous['tenant_id']):
            if current['tenant_id'] in fixtures:
                owned[current['tenant_id']] = current
            else:
                other_new.append(dict(tenant_id=current['tenant_id'], name=current['name']))
            current = read_record(current_stream)
            continue
        old_count += 1
        if current is None or previous['tenant_id'] < current['tenant_id']:
            check(False, 'old tenant missing: ' + previous['tenant_id'])
            previous = read_record(previous_stream)
            continue
        tenant_changed = []
        check(set(previous['facts']) == set(current['facts']), 'same complete selected fact tables')
        for table, rows in previous['facts'].items():
            equal = canonical(rows) == canonical(current['facts'][table])
            check(equal, f'old tenant/table changed: {previous["tenant_id"]}/{table}')
            if not equal:
                tenant_changed.append(table)
        if tenant_changed:
            old_changes.append(dict(tenant_id=previous['tenant_id'], tables=tenant_changed))
        else:
            unchanged_old += 1
        previous, current = read_record(previous_stream), read_record(current_stream)
check(set(owned) == set(fixtures), 'every actual case captured by independent psql')
observations = []
totals = Counter()
success_cases = {'known_zero_cost', 'original_cost_replay', 'exact_fefo'}
for tenant, capture in owned.items():
    f = fixtures[tenant]
    case = f['case']
    facts = capture['facts']
    totals.update({table: len(rows) for table, rows in facts.items()})
    balances = facts['inventory_balances']
    for balance in balances:
        ledger = [row for row in facts['inventory_ledgers'] if row['sku_id'] == balance['sku_id'] and row['warehouse_id'] == balance['warehouse_id']]
        for column in ('on_hand', 'reserved', 'frozen', 'in_transit'):
            exact = balance[column + '_micros']
            total = sum(row['delta_' + column + '_micros'] if row['delta_' + column + '_micros'] is not None else row['delta_' + column] * 1_000_000 for row in ledger)
            check(exact == total, f'{tenant}/{column} exact ledger recomputation')
            check(balance[column] == exact // 1_000_000, f'{tenant}/{column} whole projection')
        check(balance['on_hand_micros'] >= balance['reserved_micros'] + balance['frozen_micros'] >= 0, f'{tenant}/valid SKU facts')
    check(len(facts['shipments']) == 1, f'{case}/one actual shipment')
    shipment = facts['shipments'][0]
    shipped = shipment['status'] == 'SHIPPED'
    events = [row for row in facts['outbox_events'] if row['event_type'] == 'SHIPMENT_SHIPPED']
    cogs = facts['order_cogs']
    observations.append(dict(tenant_id=tenant, case=case, shipment_id=shipment['shipment_id'],
        status=shipment['status'], costs=[{key: row[key] for key in ('sku_id', 'qty', 'unit_cost_cent', 'total_cost_cent', 'ref_type', 'ref_id')} for row in cogs],
        batches=facts['inventory_batch_balances']))
    if args.stage == 'original':
        continue
    expected_success = case in success_cases
    check(shipped == expected_success, f'{case}/correct committed shipment outcome')
    check(len(events) == int(expected_success), f'{case}/one success event or no false event')
    check(all(row['status'] == ('CONSUMED' if expected_success else 'RESERVED') for row in facts['inventory_reservations']), f'{case}/physical sources')
    check(len(cogs) == int(expected_success or case == 'existing_cost_fact'), f'{case}/mandatory original COGS cardinality')
    if expected_success:
        for cost in cogs:
            check(cost['qty'] > 0 and cost['unit_cost_cent'] >= 0 and cost['qty'] * cost['unit_cost_cent'] == cost['total_cost_cent'], f'{case}/exact cost arithmetic')
            check(cost['shipment_id'] == shipment['shipment_id'] and cost['order_id'] == f['order'], f'{case}/actual source identity')
        expected_cost = 0 if case == 'known_zero_cost' else 2500
        check(cogs[0]['unit_cost_cent'] == cogs[0]['total_cost_cent'] == expected_cost, f'{case}/known zero or immutable original cost')
    expected_hand = 4_900_000 if case == 'cost_overflow' else 1_900_000 if expected_success else 2_900_000
    expected_reserved = 2_900_000 if case == 'cost_overflow' else 900_000 if expected_success else 1_900_000
    for balance in balances:
        check((balance['on_hand_micros'], balance['reserved_micros']) == (expected_hand, expected_reserved), f'{case}/actual inventory after action')
    if case == 'exact_fefo':
        by_id = {row['batch_id']: row for row in facts['inventory_batch_balances']}
        expected = {'EARLY': (100_000, 100_000), 'NO-EXPIRY': (800_000, 500_000), 'EXPIRED': (1_000_000, 300_000)}
        check(len(by_id) == 3, 'three exact FEFO batches')
        for metadata in facts['sku_batches']:
            row = by_id[metadata['batch_id']]
            check((row['on_hand_micros'], row['reserved_micros']) == expected[metadata['batch_no']], 'actual exact FEFO remainder and held quantity')
            check(row['on_hand'] == row['on_hand_micros'] // 1_000_000 and row['reserved'] == row['reserved_micros'] // 1_000_000, 'exact batch legacy projection')
        check(sum(row['on_hand_micros'] for row in by_id.values()) == 1_900_000, 'exact batch/global conservation')
if args.stage == 'original':
    by_case = {row['case']: row for row in observations}
    check(by_case['unknown_cost']['costs'][0]['unit_cost_cent'] == 0, 'original missing cost falsely recorded as zero')
    check(by_case['negative_cost']['costs'][0]['total_cost_cent'] == -1, 'original negative cost committed')
    check(by_case['cost_overflow']['costs'][0]['total_cost_cent'] == 9_223_372_036_854_775_807, 'original overflow saturated to i64 max')
    check(by_case['existing_cost_fact']['status'] == 'SHIPPED' and by_case['existing_cost_fact']['costs'][0]['qty'] == 99, 'original conflict silently skips mandatory actual cost')
    check(by_case['unknown_batch_precision']['status'] == 'SHIPPED' and by_case['unknown_batch_precision']['batches'][0]['on_hand_micros'] is None, 'original unknown batch precision ships through integer path')
    check(by_case['exact_fefo']['status'] == 'CREATED', 'original exact eligible FEFO path cannot commit')
source_paths = [base / f'free-erp-inventory-consumers-fulfillment-cost-batch-{args.stage}-{side}-source.json' for side in ('before', 'after')]
sources = [json.loads(path.read_text(encoding='utf8'))['files'] for path in source_paths]
owned_inputs = set(json.loads((base / (prefix + 'original-owned.json')).read_text(encoding='utf8')))
changed = [name for name in sorted(set(sources[0]) | set(sources[1])) if sources[0].get(name) != sources[1].get(name)]
owned_changed = [name for name in changed if name in owned_inputs]
check(not owned_changed, 'owned source stable during actual test')
artifacts = {path.name: dict(bytes=path.stat().st_size, sha256=digest(path)) for path in paths + source_paths + [log_path]}
report = dict(updated_at=datetime.now(timezone.utc).isoformat(), status='FAIL' if errors else 'PASS', stage=args.stage,
    checks=checks, test=summary, old_tenants=old_count, unchanged_old_tenants=unchanged_old, changed_old_tenants=old_changes,
    own_case_count=len(owned), owned_row_counts=dict(totals), observations=observations, other_new_tenants=other_new,
    source_window=dict(owned_changed=owned_changed, other_changed=[name for name in changed if name not in owned_inputs]),
    performance='NOT_RUN; registered debug test duration is not P95', artifacts=artifacts, errors=errors)
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps({key: report[key] for key in ('status', 'checks', 'test', 'old_tenants', 'unchanged_old_tenants', 'own_case_count', 'source_window', 'errors')}))
raise SystemExit(1 if errors else 0)
