"""Independent exact quantities, source events and real HTTP; never write QA data."""
import argparse
from collections import Counter
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / 'qa/acceptance'
parser = argparse.ArgumentParser()
parser.add_argument('--stage', choices=['focused-v2', 'full'], required=True)
stage = parser.parse_args().stage
prefix = 'free-erp-inventory-location-reservations'
destination = base / f'{prefix}-{stage}-independent.json'
if destination.exists():
    raise SystemExit('refusing to replace historical readback')


def decoded(path):
    data = path.read_bytes()
    return data.decode('utf-16' if data.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig')


def micros(text):
    value = Decimal(text) * 1_000_000
    if value != value.to_integral_value():
        raise ValueError('more than six decimal digits')
    return int(value)


def instant(text):
    return datetime.fromisoformat(text.replace('Z', '+00:00')).astimezone(timezone.utc).isoformat()


checks = 0
errors = []
counts = Counter()


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)


before = json.loads(decoded(base / f'{prefix}-qa-before-{stage}.json'))
after = json.loads(decoded(base / f'{prefix}-qa-after-{stage}.json'))
for facts in [before, after]:
    check((facts['database'], facts['role'], facts['host'], facts['port']) ==
          ('erp_shared_contract_test', 'erp_grid_test', '127.0.0.1', 15438), 'protected database identity')
old = {row['tenant_id']: row for row in before['tenants']}
now = {row['tenant_id']: row for row in after['tenants']}
for tenant, row in old.items():
    check(now.get(tenant) == row, f'pre-existing tenant changed: {tenant}')
log = decoded(base / f'{prefix}-{stage}.log')
results = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s', log)
check(bool(results) and tuple(map(int, results[-1][:3])) == (9 if stage == 'focused-v2' else 145, 0, 0), 'registered target result')
tests = re.findall(r'test inventory_location_reservation(?:s|_edges)::(\S+) \.\.\. ok', log)
check(len(set(tests)) == len(tests) == 9, 'all nine source-reservation scenarios')
fixtures = set(re.findall(r'QA location reservation fixture tenant=([a-f0-9-]+)', log))
check(len(fixtures) == 10 and fixtures.isdisjoint(old), 'ten new owned fixtures')
originals = {}
for tenant_id in sorted(fixtures):
    tenant = now[tenant_id]
    check(tenant['name'] == 'QA-INVENTORY-LOCATION-RESERVATIONS', 'owned QA tenant')
    ledgers = {row['ledger_id']: row for row in tenant['ledgers']}
    shared = {row['ledger_id']: row for row in tenant['shared_ledgers']}
    sources = {row['reservation_id']: row for row in tenant['reservations']}
    events = {row['request_id']: row for row in tenant['reservation_events']}
    operations = {row['log_id']: row for row in tenant['operations']}
    audits = {row['audit_id']: row for row in tenant['audits']}
    for receipt in tenant['command_receipts']:
        snapshot = receipt['original_result']
        command = snapshot['input']
        kind = command['kind']
        request = command['request_id']
        counts[kind] += 1
        originals[(tenant_id, request)] = receipt
        location = command['location']
        ordered = {key: command[key] for key in ['tenant_id', 'actor_user_id', 'request_id', 'kind',
                   'warehouse_id', 'sku_id', 'quantity_delta', 'reason', 'reference_id']}
        ordered['location'] = {key: location[key] for key in ['location_id', 'batch_code', 'quality_status']}
        source_input = command.get('reservation_source')
        if source_input is not None:
            ordered['reservation_source'] = {key: source_input[key] for key in ['source_line', 'order_id']}
        fingerprint = hashlib.sha256(json.dumps(['inventory-movement-v1', ordered], ensure_ascii=False,
                                                separators=(',', ':')).encode('utf8')).hexdigest()
        check(fingerprint == receipt['payload_hash'], f'independent full fingerprint: {request}')
        check(receipt['status'] == 'SUCCEEDED' and snapshot['version'] == 1, 'immutable completed original')
        for key in ['request_id', 'warehouse_id', 'sku_id', 'actor_user_id']:
            check(receipt[key] == command[key], f'receipt binds {key}')
        check(command['tenant_id'] == tenant_id and receipt['movement_kind'] == kind, 'tenant and movement kind')
        effect = snapshot['location_effect'] if kind == 'INBOUND' else snapshot['reservation_effect']
        primary = ledgers[snapshot['ledger_id']]
        projection = shared[effect['ledger_id']]
        delta = command['quantity_delta'] * 1_000_000
        check(snapshot['ledger_id'] == receipt['effect_id'] and primary['ref_type'] == 'shared_stock_ledger'
              and primary['ref_id'] == effect['ledger_id'], 'unique canonical projection identity')
        check((primary['warehouse_id'], primary['sku_id']) == (command['warehouse_id'], command['sku_id']), 'canonical physical key')
        check(all(projection[key] == location[key] for key in ['location_id', 'batch_code', 'quality_status'])
              and projection['warehouse_id'] == command['warehouse_id'] and projection['sku_id'] == command['sku_id'], 'full location physical key')
        check(projection['request_id'] == request and projection['delta_micros'] == delta
              and instant(effect['created_at']) == instant(projection['created_at']), 'exact shared delta and timestamp')
        reason = {'INBOUND': 'PURCHASE_RECEIPT', 'RESERVE': 'RESERVATION_CREATE', 'RELEASE': 'RESERVATION_RELEASE'}[kind]
        action = {'INBOUND': 'INVENTORY_INBOUND', 'RESERVE': 'INVENTORY_RESERVED', 'RELEASE': 'INVENTORY_RELEASED'}[kind]
        check(primary['reason_type'] == reason and primary['delta_frozen_micros'] == primary['delta_in_transit_micros'] == 0, 'canonical reason and zero unrelated delta')
        if kind == 'INBOUND':
            check(delta > 0 and primary['delta_on_hand_micros'] == delta and primary['delta_reserved_micros'] == 0
                  and projection['bucket'] == 'ON_HAND' and projection['source_line'] == '', 'actual canonical inbound')
        else:
            source = sources[effect['reservation_id']]
            event = events[request]
            check(primary['delta_reserved_micros'] == delta and primary['delta_on_hand_micros'] == 0
                  and projection['bucket'] == 'RESERVED' and location['quality_status'] == 'GOOD', 'reserve/release changes only reserved')
            check((delta > 0 if kind == 'RESERVE' else delta < 0), 'reserve/release direction')
            check(source['source_type'] == 'LOCATION_COMMAND' and source['source_context'] == source_input
                  and source['source_id'] == projection['source_line'] == effect['source_line'] == source_input['source_line'], 'immutable original source context')
            check(source['warehouse_id'] == command['warehouse_id'] and source['sku_id'] == command['sku_id']
                  and all(source[key] == location[key] for key in ['location_id', 'batch_code', 'quality_status']), 'formal source physical key')
            check(source['original_qty_micros'] == micros(effect['original_quantity']), 'immutable original quantity')
            check(micros(effect['original_quantity']) == micros(effect['remaining_quantity']) + micros(effect['released_quantity']), 'original source arithmetic')
            check(event['ledger_id'] == effect['ledger_id'] and event['reservation_id'] == effect['reservation_id']
                  and event['delta_micros'] == delta and event['remaining_micros'] == micros(effect['remaining_quantity'])
                  and event['released_micros'] == micros(effect['released_quantity'])
                  and event['location_reserved_micros'] == micros(effect['location_reserved'])
                  and instant(event['created_at']) == instant(effect['created_at']), 'original source event, not later source row')
            counts['SOURCE_EVENTS'] += 1
        q = snapshot['balance']['quantities']
        check(micros(q['available']) == micros(q['on_hand']) - micros(q['reserved']) - micros(q['frozen']), 'original exact available arithmetic')
        check(all(micros(value) >= 0 for value in q.values()), 'original quantities nonnegative')
        metadata = {'request_id': request, 'payload_hash': fingerprint, 'movement_kind': kind,
                    'warehouse_id': command['warehouse_id'], 'sku_id': command['sku_id'], 'batch_id': None,
                    'qty_delta': command['quantity_delta'], 'reason': command['reason'], 'reference_id': command['reference_id'],
                    'reason_type': reason, 'quantities': q, 'location': location}
        if source_input is not None:
            metadata['reservation_source'] = source_input
        operation = operations[snapshot['operation_log_id']]
        audit = audits[snapshot['audit_id']]
        check(operation['metadata'] == audit['after_json'] == metadata, 'two original required logs')
        check(operation['actor_user_id'] == audit['user_id'] == command['actor_user_id']
              and operation['action'] == action and audit['action'] == f'INVENTORY.{reason}', 'authenticated original log actor and kind')
    for source in sources.values():
        if source['source_type'] != 'LOCATION_COMMAND':
            counts['LEGACY_SOURCE_FIXTURES'] += 1
            continue
        related = [event for event in events.values() if event['reservation_id'] == source['reservation_id']]
        check(sum(event['delta_micros'] for event in related) == source['qty_micros'], 'source remaining equals immutable event sum')
        check(sum(max(0, event['delta_micros']) for event in related) == source['original_qty_micros'], 'one source original reserve')
        check(source['qty_micros'] + source['released_micros'] == source['original_qty_micros'], 'current exact source arithmetic')
        check(source['status'] == ('RESERVED' if source['qty_micros'] else 'RELEASED'), 'current source terminal state')
        check(source['order_id'] is None and source['order_item_id'] is None, 'manual reference is not a fabricated sales-order transition')
    for balance in tenant['balances']:
        related = [row for row in ledgers.values() if row['warehouse_id'] == balance['warehouse_id'] and row['sku_id'] == balance['sku_id']]
        for bucket in ['on_hand', 'reserved', 'frozen', 'in_transit']:
            check(sum(row[f'delta_{bucket}_micros'] for row in related) == balance[f'{bucket}_micros'], f'current {bucket} equals canonical ledger sum')
    for event in tenant['outbox']:
        check(event['payload']['available_micros'] in ('3000000', '4000000'), 'one atomic availability effect per successful reserve/release')
        counts['OUTBOX'] += 1
check({key: counts[key] for key in ['INBOUND', 'RESERVE', 'RELEASE', 'SOURCE_EVENTS', 'LEGACY_SOURCE_FIXTURES', 'OUTBOX']} ==
      {'INBOUND': 10, 'RESERVE': 14, 'RELEASE': 6, 'SOURCE_EVENTS': 20, 'LEGACY_SOURCE_FIXTURES': 1, 'OUTBOX': 4}, 'exact committed scenarios')

http_counts = Counter()
for tenant, actor, action, request_text, headers, status, body_text in re.findall(
        r'QA location command HTTP tenant=([a-f0-9-]+) actor=([a-f0-9-]+) action=(\S+) request=(\{.*?\}|null) headers=(\[.*?\]) status=(\d{3}) [^\n]*? body=(\{.*\})', log):
    if tenant not in fixtures:
        continue
    request, body = json.loads(request_text), json.loads(body_text)
    http_counts[status] += 1
    if status == '200' and (action.startswith('commands/') or action in ('reservations', 'releases', 'inbounds')):
        receipt = originals[(tenant, body['request_id'])]
        snapshot = receipt['original_result']
        check(body['effect_id'] == receipt['effect_id'] and body['payload_hash'] == receipt['payload_hash'], 'HTTP original effect and fingerprint')
        check(body['operation_log_id'] == snapshot['operation_log_id'] and body['audit_id'] == snapshot['audit_id'], 'HTTP original required log IDs')
        check(body['record']['quantities'] == snapshot['balance']['quantities'], 'HTTP original warehouse quantities')
        key = 'location_record' if snapshot['input']['kind'] == 'INBOUND' else 'reservation_record'
        expected = snapshot['location_effect' if key == 'location_record' else 'reservation_effect']
        check(all(body[key][field] == value for field, value in expected.items() if field != 'created_at')
              and instant(body[key]['created_at']) == instant(expected['created_at']), 'HTTP immutable original location/source result')
        check(body['bucket'] == ('ON_HAND' if key == 'location_record' else 'RESERVED'), 'HTTP projection bucket')
    if status == '500':
        check('check option' in body['message'] and any(table in body['message'] for table in ['operation_logs', 'audit_logs']), 'actual late PostgreSQL log failure')
check(http_counts['500'] == 4, 'four actual late log faults')
check(len(re.findall(r'QA location reservation guard tenant=', log)) == 6, 'six actual source/event guard rejections')
report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'FAIL' if errors else 'PASS',
          'stage': stage, 'checks': checks, 'errors': errors, 'pre_existing_tenants_unchanged': len(old),
          'owned_fixtures': len(fixtures), 'committed_counts': dict(counts), 'http_status_counts': dict(http_counts),
          'registered_tests': None if not results else {'passed': int(results[-1][0]), 'failed': int(results[-1][1]),
          'ignored': int(results[-1][2]), 'seconds': float(results[-1][3])},
          'limits': ['Protected PostgreSQL and synthetic JWT only; not real login, desktop/Web, production or Release load acceptance.']}
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n', encoding='utf8')
print(json.dumps({key: report[key] for key in ['status', 'checks', 'errors', 'owned_fixtures', 'committed_counts', 'http_status_counts']}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
