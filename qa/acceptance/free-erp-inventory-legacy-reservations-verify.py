"""Bounded read-only verification of actual legacy routes and protected QA facts."""
import argparse
from collections import Counter, defaultdict
from datetime import datetime, timezone
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import re
from uuid import UUID

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
PREFIX = 'free-erp-inventory-legacy-reservations'
parser = argparse.ArgumentParser()
parser.add_argument('--stage', choices=['original', 'focused-v2', 'full', 'full-v2'], required=True)
parser.add_argument('--revision', choices=['v2'])
args = parser.parse_args()
STAGE = args.stage
revision = f'-{args.revision}' if args.revision else ''
DESTINATION = BASE / f'{PREFIX}-{STAGE}-independent{revision}.json'
if DESTINATION.exists():
    raise SystemExit('refusing to replace historical evidence')


def text(path):
    data = path.read_bytes()
    return data.decode('utf-16' if data.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig')


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':'))


def facts_hash(row):
    # SQL ordering of nullable source columns can tie. Compare complete row sets,
    # preserving all nested business-array order and every value/column.
    ordered = {key: sorted(value, key=canonical) if isinstance(value, list) else value
               for key, value in row.items()}
    return hashlib.sha256(canonical(ordered).encode('utf8')).hexdigest()


def streamed(path, keep):
    hashes, rows = {}, {}
    with path.open(encoding='utf-8-sig') as stream:
        identity = json.loads(next(stream))
        for line in stream:
            row = json.loads(line)
            tenant = row['tenant_id']
            if tenant in hashes:
                raise ValueError('duplicate tenant in independent facts')
            hashes[tenant] = facts_hash(row)
            if tenant in keep:
                rows[tenant] = row
    return identity, hashes, rows


def old_json(path):
    # The historical aggregate is retained as-is. Decode one tenant at a time,
    # avoiding a second 150 MiB whole-database object while Rust is building.
    hashes = {}
    decoder = json.JSONDecoder()
    with path.open(encoding='utf-8-sig') as stream:
        buffer = ''
        while not (match := re.search(r'"tenants"\s*:\s*\[', buffer)):
            chunk = stream.read(65536)
            if not chunk:
                raise ValueError('historical tenants array missing')
            buffer += chunk
        prefix, buffer = buffer[:match.end() - 1], buffer[match.end():]
        while True:
            buffer = buffer.lstrip()
            if buffer.startswith(','):
                buffer = buffer[1:].lstrip()
            if buffer.startswith(']'):
                identity = json.loads(prefix + '[]' + buffer[1:] + stream.read())
                identity.pop('tenants')
                return identity, hashes, {}
            try:
                row, end = decoder.raw_decode(buffer)
            except json.JSONDecodeError:
                chunk = stream.read(65536)
                if not chunk:
                    raise ValueError('truncated historical tenant facts') from None
                buffer += chunk
                continue
            tenant = row['tenant_id']
            if tenant in hashes:
                raise ValueError('duplicate historical tenant')
            hashes[tenant] = facts_hash(row)
            buffer = buffer[end:]


def micros(value):
    scaled = Decimal(value) * 1_000_000
    if scaled != scaled.to_integral_value():
        raise ValueError('quantity exceeds six decimal digits')
    return int(scaled)


def instant(value):
    return datetime.fromisoformat(value.replace('Z', '+00:00')).astimezone(timezone.utc)


def child_id(parent, sku):
    return str(UUID(bytes=hashlib.sha1(UUID(parent).bytes + UUID(sku).bytes).digest()[:16], version=5))


checks, errors, counts = 0, [], Counter()


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)


log = text(
    BASE / (f'{PREFIX}-before-repair.log' if STAGE == 'original' else f'{PREFIX}-{STAGE}.log'))
fixtures = {tenant: {'actor': actor, 'warehouse': warehouse, 'sku': sku, 'source': source}
            for tenant, actor, warehouse, sku, source in re.findall(
                r'QA legacy reservation fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+) source=([a-f0-9-]+)', log)}
kit = {tenant: {'parent': parent, 'first': first, 'second': second, 'source': source}
       for tenant, parent, first, second, source in re.findall(
           r'QA legacy reservation kit tenant=([a-f0-9-]+) parent=([a-f0-9-]+) first=([a-f0-9-]+) second=([a-f0-9-]+) source=([a-f0-9-]+)', log)}
if STAGE == 'original':
    before_id, old, _ = old_json(BASE / f'{PREFIX}-qa-before-repair.json')
    after_id, now, owned = streamed(BASE / f'{PREFIX}-qa-original-after-stream.jsonl', fixtures)
else:
    before_id, old, _ = streamed(BASE / f'{PREFIX}-qa-before-{STAGE}.jsonl', set())
    after_id, now, owned = streamed(BASE / f'{PREFIX}-qa-after-{STAGE}.jsonl', fixtures)
for identity in [before_id, after_id]:
    check(identity == {'database': 'erp_shared_contract_test', 'role': 'erp_grid_test',
                       'host': '127.0.0.1', 'port': 15438}, 'protected read-only database identity')
for tenant, digest in old.items():
    check(now.get(tenant) == digest, f'pre-existing complete tenant row sets changed: {tenant}')
check(len(fixtures) == (7 if STAGE == 'original' else 14) and fixtures.keys().isdisjoint(old), 'new owned legacy fixtures')
results = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s', log)
expected = (3, 7, 0) if STAGE == 'original' else (17 if STAGE == 'focused-v2' else 159, 0, 0)
check(bool(results) and tuple(map(int, results[-1][:3])) == expected, 'actual registered test result')
tests = re.findall(r'test inventory_legacy_reservation_(?:atomicity|edges)::(\S+) \.\.\. (ok|FAILED)', log)
check(len(tests) == len({name for name, _ in tests}) == (7 if STAGE == 'original' else 14), 'all owned named scenarios')
http, by_tenant = [], defaultdict(list)
for tenant, actor, action, request, headers, status, body in re.findall(
        r'QA location command HTTP tenant=([a-f0-9-]+) actor=([a-f0-9-]+) action=(\S+) request=(\{.*?\}|null) headers=(\[.*?\]) status=(\d{3}) [^\n]*? body=(\{.*\})', log):
    if tenant not in fixtures:
        continue
    item = {'actor': actor, 'action': action, 'request': json.loads(request), 'status': int(status), 'body': json.loads(body)}
    check(headers == '[]' and actor == fixtures[tenant]['actor'], 'actual legacy HTTP authenticated actor/no synthetic original header')
    by_tenant[tenant].append(item)
    http.append(item)

if STAGE != 'original':
    mixed = re.findall(r'QA legacy reservation mixed tenant=([a-f0-9-]+) inbound=(\d+) reserved=(\d+)', log)
    check(len(mixed) == 1 and int(mixed[0][1]) == 10 and 1 <= int(mixed[0][2]) <= 10, 'actual mixed physical-lock outcome')
    mixed_count = int(mixed[0][2]) if mixed else -100
    for tenant, f in fixtures.items():
        row = owned[tenant]
        check(row['name'] == 'QA-INVENTORY-LEGACY-RESERVATION-ATOMICITY', 'owned QA name')
        check(not row['shared_ledgers'] and not row['reservation_events'], 'legacy does not fabricate location/source projections')
        check(not row['orders'] and not row['order_items'], 'synthetic source is not a verified sales order')
        operations = {value['log_id']: value for value in row['operations']}
        audits = {value['audit_id']: value for value in row['audits']}
        ledgers = {value['ledger_id']: value for value in row['ledgers']}
        sources = {value['order_item_id']: value for value in row['reservations']}
        current, at_effect, groups = {}, {}, defaultdict(list)
        buckets = ['on_hand', 'reserved', 'frozen', 'in_transit']
        for ledger in row['ledgers']:
            groups[(ledger['warehouse_id'], ledger['sku_id'])].append(ledger)
            counts[ledger['reason_type']] += 1
        for key, effects in groups.items():
            quantities = {bucket: sum(effect[f'delta_{bucket}_micros'] for effect in effects) for bucket in buckets}
            quantities['available'] = quantities['on_hand'] - quantities['reserved'] - quantities['frozen']
            current[key] = quantities
            if mixed and tenant == mixed[0][0]:
                # occurred_at is captured before lock admission; transaction now()
                # also cannot establish lock order. Verify the unique exact state
                # chain carried by immutable receipts/logs against signed deltas.
                initial = [effect for effect in effects if effect['reason_type'] == 'QA_LEGACY_RESERVATION_OPENING']
                check(len(initial) == 1, 'one physical opening in mixed fixture')
                opening = {bucket: initial[0][f'delta_{bucket}_micros'] for bucket in buckets}
                opening['available'] = opening['on_hand'] - opening['reserved'] - opening['frozen']
                at_effect[initial[0]['ledger_id']] = opening
                for receipt in row['command_receipts']:
                    at_effect[receipt['effect_id']] = {bucket: micros(value) for bucket, value in receipt['original_result']['balance']['quantities'].items()}
                for operation in row['operations']:
                    if operation['action'] != 'INVENTORY_RESERVED':
                        continue
                    matching_effects = [effect for effect in effects if effect['reason_type'] == 'RESERVATION_CREATE' and effect['ref_id'] == operation['ref_id']]
                    check(len(matching_effects) == 1, 'mixed source log identifies one exact effect')
                    if matching_effects:
                        at_effect[matching_effects[0]['ledger_id']] = {bucket: micros(value) for bucket, value in operation['metadata']['result']['quantities'].items()}
                remaining = [effect for effect in effects if effect['reason_type'] != 'QA_LEGACY_RESERVATION_OPENING']
                cursor = opening
                while remaining:
                    candidates = [effect for effect in remaining if effect['ledger_id'] in at_effect and all(
                        at_effect[effect['ledger_id']][bucket] - effect[f'delta_{bucket}_micros'] == cursor[bucket] for bucket in buckets)]
                    check(len(candidates) == 1, 'all actual mixed effects form one complete exact physical state chain')
                    if len(candidates) != 1:
                        break
                    effect = candidates[0]
                    cursor = at_effect[effect['ledger_id']]
                    check(cursor['available'] == cursor['on_hand'] - cursor['reserved'] - cursor['frozen'] and all(value >= 0 for value in cursor.values()), 'mixed post-effect availability and nonnegative buckets')
                    remaining.remove(effect)
                check(not remaining and cursor == quantities, 'complete mixed state chain reaches current independently summed stock')
            else:
                # Controlled nonmixed fixtures have one opening, one whole reserve,
                # and optionally one release per physical SKU; no timestamp order.
                rank = {'QA_LEGACY_RESERVATION_OPENING': 0, 'RESERVATION_CREATE': 1, 'RESERVATION_RELEASE': 2}
                cursor = {bucket: 0 for bucket in buckets}
                for effect in sorted(effects, key=lambda value: rank[value['reason_type']]):
                    for bucket in buckets:
                        cursor[bucket] += effect[f'delta_{bucket}_micros']
                    cursor['available'] = cursor['on_hand'] - cursor['reserved'] - cursor['frozen']
                    check(all(value >= 0 for value in cursor.values()), 'exact nonnegative isolated fixture quantity sequence')
                    at_effect[effect['ledger_id']] = dict(cursor)
        for balance in row['balances']:
            exact = current[(balance['warehouse_id'], balance['sku_id'])]
            check(all(exact[bucket] == balance[f'{bucket}_micros'] for bucket in ['on_hand', 'reserved', 'frozen', 'in_transit']), 'canonical balance equals complete exact ledger sum')
        for source, value in sources.items():
            effects = [effect for effect in row['ledgers'] if effect['ref_type'] == 'ORDER_ITEM' and effect['ref_id'] == source]
            create = [effect for effect in effects if effect['reason_type'] == 'RESERVATION_CREATE']
            release = [effect for effect in effects if effect['reason_type'] == 'RESERVATION_RELEASE']
            check(len(create) == 1 and len(release) == (1 if value['status'] == 'RELEASED' else 0), 'one source create/full release')
            check(value['qty'] == 1 and value['qty_micros'] == value['original_qty_micros'] == 1_000_000, 'legacy original quantity remains one whole unit at terminal state')
            check(all(value[key] is None for key in ['source_type', 'source_id', 'location_id', 'batch_code', 'quality_status', 'source_context', 'released_micros', 'source_request_id']), 'no invented legacy location/current remainder/original identity')
            for effect in effects:
                check(effect['warehouse_id'] == value['warehouse_id'] and effect['sku_id'] == value['sku_id'], 'source and canonical physical identity')
                check(effect['delta_reserved_micros'] == (1_000_000 if effect['reason_type'] == 'RESERVATION_CREATE' else -1_000_000)
                      and all(effect[f'delta_{bucket}_micros'] == 0 for bucket in ['on_hand', 'frozen', 'in_transit']), 'legacy changes only exact reserved bucket')
        successful = defaultdict(list)
        for item in by_tenant[tenant]:
            if item['status'] != 200 or item['action'] == 'inbounds':
                continue
            request, result = item['request'], item['body']['record']
            check('request_id' not in item['body'] and 'effect_id' not in item['body'], 'legacy returns current balance without fabricated original receipt')
            check(micros(result['quantities']['available']) == micros(result['quantities']['on_hand']) - micros(result['quantities']['reserved']) - micros(result['quantities']['frozen']), 'HTTP exact available arithmetic')
            successful[(item['action'], request['order_item_id'])].append(item)
        audited = [value for value in row['operations'] if value['action'] in ['INVENTORY_RESERVED', 'INVENTORY_RELEASED']]
        check(len(audited) == len(successful), 'one log per actual legacy source action, retries add none')
        for (action, source), responses in successful.items():
            expected_action = 'INVENTORY_RESERVED' if action == 'reservations' else 'INVENTORY_RELEASED'
            logs = [value for value in audited if value['action'] == expected_action and value['ref_id'] == source]
            check(len(logs) == 1, 'one authenticated log for source action')
            if not logs:
                continue
            operation = logs[0]
            metadata = operation['metadata']
            matching = [value for value in row['audits'] if value['action'] == ('INVENTORY.RESERVE' if action == 'reservations' else 'INVENTORY.RELEASE') and value['after_json'] == metadata]
            check(len(matching) == 1, 'matching required audit exact metadata')
            check(operation['actor_type'] == 'USER' and operation['actor_user_id'] == f['actor']
                  and operation['ref_type'] == 'ORDER_ITEM' and bool(matching) and matching[0]['user_id'] == f['actor'], 'both required logs bind actual actor/source')
            request = responses[0]['request']
            check(metadata['warehouse_id'] == f['warehouse'] and metadata['sku_id'] == request['sku_id'] and metadata['order_item_id'] == source and metadata['qty'] == 1, 'required log requested context')
            if action == 'reservations':
                check(metadata['order_id'] == request.get('order_id'), 'original optional order context in required logs')
            result = metadata['result']
            child = child_id(source, result['sku_id']) if tenant in kit else source
            expected_reason = 'RESERVATION_CREATE' if action == 'reservations' else 'RESERVATION_RELEASE'
            effects = [value for value in row['ledgers'] if value['ref_id'] == child and value['reason_type'] == expected_reason]
            check(len(effects) == 1 and result['warehouse_id'] == f['warehouse'], 'log outcome binds actual canonical child/source effect')
            if effects:
                check({key: micros(value) for key, value in result['quantities'].items()} == at_effect[effects[0]['ledger_id']], 'independent exact post-effect logged result')
            check(any(response['body']['record']['quantities'] == result['quantities'] for response in responses), 'at least first HTTP response matches committed log outcome')
        for receipt in row['command_receipts']:
            snapshot, request = receipt['original_result'], receipt['request_id']
            command = snapshot['input']
            ordered = {key: command[key] for key in ['tenant_id', 'actor_user_id', 'request_id', 'kind', 'warehouse_id', 'sku_id', 'quantity_delta', 'reason', 'reference_id']}
            fingerprint = hashlib.sha256(json.dumps(['inventory-movement-v1', ordered], ensure_ascii=False, separators=(',', ':')).encode('utf8')).hexdigest()
            check(command['kind'] == receipt['movement_kind'] == 'INBOUND' and command['quantity_delta'] == 1 and 'location' not in command and 'reservation_source' not in command, 'actual mixed original inbound scope')
            check(receipt['status'] == 'SUCCEEDED' and snapshot['version'] == 1 and receipt['payload_hash'] == fingerprint, 'independent ordered original inbound fingerprint')
            check(all(receipt[key] == command[key] for key in ['request_id', 'actor_user_id', 'warehouse_id', 'sku_id']) and command['tenant_id'] == tenant, 'full original identity binds receipt')
            effect = ledgers[receipt['effect_id']]
            check(effect['ledger_id'] == snapshot['ledger_id'] and effect['ref_id'] == request and effect['reason_type'] == 'PURCHASE_RECEIPT' and effect['delta_on_hand_micros'] == 1_000_000, 'one exact canonical original inbound')
            q = snapshot['balance']['quantities']
            check({key: micros(value) for key, value in q.items()} == at_effect[effect['ledger_id']], 'original immutable quantities equal independent post-effect ledger sum')
            metadata = {'request_id': request, 'payload_hash': fingerprint, 'movement_kind': 'INBOUND', 'warehouse_id': command['warehouse_id'], 'sku_id': command['sku_id'], 'batch_id': None, 'qty_delta': 1, 'reason': None, 'reference_id': None, 'reason_type': 'PURCHASE_RECEIPT', 'quantities': q}
            op, audit = operations[snapshot['operation_log_id']], audits[snapshot['audit_id']]
            check(op['metadata'] == audit['after_json'] == metadata and op['actor_user_id'] == audit['user_id'] == f['actor'], 'actual original inbound required log pair')
        for value in row['outbox']:
            payload = value['payload']
            check(value['event_type'] == 'INVENTORY_CHANGED' and payload['tenant_id'] == tenant and payload['quantity_scale'] == 1_000_000, 'real producer availability event')
            quantities = {bucket: int(payload[f'{bucket}_micros']) for bucket in ['on_hand', 'reserved', 'frozen', 'in_transit', 'available']}
            check(quantities['available'] == quantities['on_hand'] - quantities['reserved'] - quantities['frozen'], 'exact producer available arithmetic')
            effects = [effect for effect in row['ledgers'] if effect['sku_id'] == payload['sku_id'] and instant(effect['occurred_at']) == instant(payload['occurred_at'])]
            check(len(effects) == 1 and at_effect[effects[0]['ledger_id']] == quantities, 'producer event binds one actual canonical effect')
        if tenant in kit:
            info = kit[tenant]
            check(len(sources) == 2 and set(sources) == {child_id(info['source'], info[key]) for key in ['first', 'second']}, 'independent stable kit child identities')
            check(all(value['sku_id'] != info['parent'] for value in row['balances']), 'no fabricated kit parent stock')
        for key in ['balances', 'ledgers', 'reservations', 'operations', 'audits', 'outbox', 'command_receipts']:
            counts[key] += len(row[key])
    expected_counts = {'balances': 16, 'ledgers': 44 + mixed_count, 'reservations': 14 + mixed_count, 'operations': 24 + mixed_count, 'audits': 24 + mixed_count, 'outbox': 10, 'command_receipts': 10}
    check(all(counts[key] == value for key, value in expected_counts.items()), 'exact committed source/log/outbox scenario totals')
    rollbacks = re.findall(r'QA legacy reservation rollback tenant=([a-f0-9-]+) action=(\S+) table=(\S+) before=(\{.*\}) after=(\{.*\})', log)
    check(len(rollbacks) == 6, 'six actual late database log failures including both kit components')
    for tenant, action, table, before, after in rollbacks:
        check(json.loads(before) == json.loads(after), 'actual complete before/after rollback facts')
        failed = [item for item in by_tenant[tenant] if item['status'] == 500 and item['action'] == action]
        check(len(failed) == 1 and 'check option' in failed[0]['body']['message'] and table in failed[0]['body']['message'], 'specified PostgreSQL view rejection reached actual required log')
    status_counts = Counter(item['status'] for item in http)
    check(status_counts == {200: 64 + mixed_count, 400: 8, 403: 2, 409: 30 - mixed_count, 500: 6}, 'all 110 actual HTTP responses and explicit rejection classes')
else:
    status_counts = Counter(item['status'] for item in http)
    counts['original_failed_scenarios'] = sum(result == 'FAILED' for _, result in tests)
    check(counts['original_failed_scenarios'] == 7, 'seven unmodified counterexamples reproduced')

report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'FAIL' if errors else 'PASS',
          'stage': STAGE, 'checks': checks, 'errors': errors, 'pre_existing_tenants_unchanged': len(old),
          'owned_fixtures': len(fixtures), 'committed_counts': dict(counts), 'http_status_counts': dict(status_counts),
          'registered_tests': None if not results else {'passed': int(results[-1][0]), 'failed': int(results[-1][1]), 'ignored': int(results[-1][2]), 'seconds': float(results[-1][3])},
          'facts_comparison': 'Complete tenant top-level row sets; nested business arrays preserved; one tenant decoded at a time. Mixed effects follow the complete exact state chain, never business or transaction timestamps.',
          'limits': ['Synthetic JWT/protected private QA only; not real account, native/Web, public deployment, recovery or Release performance.',
                     'Legacy UUID sources have no original command receipt; terminal qty is original whole quantity, not current remainder.',
                     'Kit parent quantity/frozen expansion and changed-BOM release are separate unresolved contracts.']}
DESTINATION.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps({key: report[key] for key in ['status', 'checks', 'errors', 'owned_fixtures', 'committed_counts', 'http_status_counts']}, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
