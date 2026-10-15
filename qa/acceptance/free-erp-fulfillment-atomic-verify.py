"""Independent, immutable readback for real OMS routes; never writes business data."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

BASE = Path(__file__).resolve().parent
ROOT = BASE.parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('stage')
parser.add_argument('before')
parser.add_argument('after')
parser.add_argument('log')
args = parser.parse_args()
if not all(re.fullmatch(r'[a-z0-9-]+', value) for value in vars(args).values()):
    raise SystemExit('invalid artifact label')
destination = BASE / f'free-erp-fulfillment-{args.stage}-independent.json'
if destination.exists():
    raise SystemExit('refusing to replace independent evidence')
errors = []
checks = 0


def check(condition, label):
    global checks
    checks += 1
    if not condition:
        errors.append(label)


def canonical(rows):
    # Only the unordered top-level table result is sorted. All columns and
    # nested JSON/arrays remain part of the comparison.
    return sorted(json.dumps(row, sort_keys=True, separators=(',', ':')) for row in rows)


def read_record(stream):
    for line in stream:
        if line.strip():
            return json.loads(line)
    return None


log_path = BASE / f'free-erp-fulfillment-{args.log}.log'
raw = log_path.read_bytes()
log = raw.decode('utf-16' if raw[:2] in (b'\xff\xfe', b'\xfe\xff') else 'utf-8-sig')
fixtures = {}
for tenant, actor, order, root, shop, warehouse, sku in re.findall(
        r'QA fulfillment fixture tenant=(\S+) actor=(\S+) order=(\S+) root=(\S+) shop=(\S+) warehouse=(\S+) sku=(\S+)', log):
    fixtures[tenant] = dict(actor=actor, order=order, root=root, shop=shop, warehouse=warehouse, sku=sku)
http = []
for tenant, suffix, status, body in re.findall(
        r'QA fulfillment_http tenant=(\S+) suffix=(\S+) status=(\d+) body=(.+)', log):
    http.append(dict(tenant_id=tenant, suffix=suffix, status=int(status), body=json.loads(body)))
results = re.findall(r'test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s', log)
check(len(results) == 1, 'one actual test summary')
if results:
    status, passed, failed, ignored, seconds = results[0]
    summary = dict(status=status, passed=int(passed), failed=int(failed), ignored=int(ignored), seconds=float(seconds))
    if args.stage == 'atomic-original':
        check(summary['passed'] == 0 and summary['failed'] == 13 and summary['ignored'] == 0, 'thirteen original authority/record failures reproduced')
    else:
        check(summary['status'] == 'ok' and summary['failed'] == 0 and summary['ignored'] == 0, 'registered tests passed without ignores')
else:
    summary = None

old_count = unchanged = 0
old_changes = []
concurrent = []
owned = {}
with (BASE / f'free-erp-fulfillment-{args.before}.jsonl').open(encoding='utf-8-sig') as previous_stream, \
        (BASE / f'free-erp-fulfillment-{args.after}.jsonl').open(encoding='utf-8-sig') as current_stream:
    old_catalog, new_catalog = read_record(previous_stream), read_record(current_stream)
    for catalog in (old_catalog, new_catalog):
        check({key: catalog[key] for key in ('database', 'role', 'host', 'port')} == {
            'database': 'erp_shared_contract_test', 'role': 'erp_grid_test', 'host': '127.0.0.1', 'port': 15438}, 'protected QA identity')
    check(old_catalog['catalog'] == new_catalog['catalog'], 'schema unchanged during test window')
    previous, current = read_record(previous_stream), read_record(current_stream)
    while previous is not None or current is not None:
        if previous is None or (current is not None and current['tenant_id'] < previous['tenant_id']):
            if current['tenant_id'] in fixtures:
                owned[current['tenant_id']] = current
            else:
                concurrent.append(dict(tenant_id=current['tenant_id'], name=current['name']))
            current = read_record(current_stream)
            continue
        old_count += 1
        if current is None or previous['tenant_id'] < current['tenant_id']:
            check(False, f'missing old tenant {previous["tenant_id"]}')
            previous = read_record(previous_stream)
            continue
        differences = []
        for table, rows in previous['facts'].items():
            checks += 1
            if canonical(rows) != canonical(current['facts'][table]):
                differences.append(table)
        if previous['name'] != current['name']:
            differences.append('tenant name')
        if differences:
            old_changes.append(dict(tenant_id=previous['tenant_id'], name=previous['name'], tables=differences))
        else:
            unchanged += 1
        previous, current = read_record(previous_stream), read_record(current_stream)
check(set(owned) == set(fixtures), 'all owned fixtures independently captured')
check(all(row['tenant_id'] in fixtures for row in http), 'every actual HTTP belongs to a captured fixture')
counts = Counter()
for tenant, captured in owned.items():
    facts = captured['facts']
    counts.update({table: len(rows) for table, rows in facts.items()})
    fixture = fixtures[tenant]
    for root in facts['inventory_order_reservation_sources']:
        components = [row for row in facts['inventory_order_reservation_components'] if row['source_id'] == root['source_id']]
        check(len(components) == root['component_count'], f'{tenant}: complete frozen components')
        for component in components:
            physical = [row for row in facts['inventory_reservations'] if row['order_item_id'] == component['order_item_id']]
            check(len(physical) == 1, f'{tenant}: unique physical child')
            if physical:
                source = physical[0]
                check(source['reservation_source_id'] == root['source_id'] and source['warehouse_id'] == root['warehouse_id']
                      and source['sku_id'] == component['sku_id'] and source['qty'] == component['qty']
                      and source['order_id'] == root['order_id'], f'{tenant}: root/map/source identity')
    for balance in facts['inventory_balances']:
        key = (balance['warehouse_id'], balance['sku_id'])
        ledgers = [row for row in facts['inventory_ledgers'] if (row['warehouse_id'], row['sku_id']) == key]
        for column in ('on_hand', 'reserved', 'frozen', 'in_transit'):
            check(sum(row[f'delta_{column}_micros'] for row in ledgers) == balance[f'{column}_micros'],
                  f'{tenant}: exact {column} independently reconciles from every ledger')
    for cogs in facts['order_cogs']:
        source = next((row for row in facts['inventory_reservations'] if row['order_item_id'] == cogs['order_item_id']), None)
        shipment = next((row for row in facts['shipments'] if row['shipment_id'] == cogs['shipment_id']), None)
        check(source is not None and shipment is not None, f'{tenant}: COGS has source and shipment')
        if source and shipment:
            check(source['status'] == 'CONSUMED' and source['order_id'] == cogs['order_id'] == shipment['order_id']
                  and source['warehouse_id'] == shipment['warehouse_id'] and source['sku_id'] == cogs['sku_id']
                  and source['qty'] == cogs['qty'] and shipment['status'] == 'SHIPPED', f'{tenant}: COGS cites this consumed source')
            check(cogs['unit_cost_cent'] * cogs['qty'] == cogs['total_cost_cent'], f'{tenant}: exact COGS multiplication')

artifacts = {}
for path in [log_path, BASE / f'free-erp-fulfillment-{args.before}.jsonl', BASE / f'free-erp-fulfillment-{args.after}.jsonl', Path(__file__)]:
    with path.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    artifacts[path.name] = dict(bytes=path.stat().st_size, sha256=digest)
report = dict(updated_at=datetime.now(timezone.utc).isoformat(), status='FAIL' if errors else 'PARTIAL' if old_changes else 'PASS',
              checks=checks, stage=args.stage, test=summary, original_failures_expected=args.stage == 'atomic-original',
              old_tenants=old_count, unchanged_old_tenants=unchanged, changed_old_tenants=old_changes,
              other_new_tenants_outside_fulfillment=concurrent, owned_fixtures=fixtures, owned_row_counts=dict(counts),
              actual_http=http, artifacts=artifacts, errors=errors,
              qualification='Private QA real Axum routes with synthetic JWT; no real login, native/Web, Release load, full migration, public hosting or recovery acceptance.')
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps({key: report[key] for key in ('status', 'checks', 'old_tenants', 'unchanged_old_tenants', 'owned_row_counts', 'errors')}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
