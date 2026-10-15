"""Bounded verification of original failures, migration and real kit stock facts."""
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
PREFIX = 'free-erp-inventory-kit-sources'
IDENTITY = {'database': 'erp_shared_contract_test', 'role': 'erp_grid_test',
            'host': '127.0.0.1', 'port': 15438}
checks, errors, counts = 0, [], Counter()


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)


def text(path):
    raw = path.read_bytes()
    return raw.decode('utf-16' if raw.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig')


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':'))


def facts_hash(row):
    # Sort only top-level record sets; keep every field and nested array order.
    value = {key: sorted(item, key=canonical) if isinstance(item, list) else item
             for key, item in row.items()}
    return hashlib.sha256(canonical(value).encode('utf8')).hexdigest()


def streamed(path, keep=(), migration=False):
    hashes, rows = {}, {}
    with path.open(encoding='utf-8-sig') as stream:
        identity = json.loads(next(stream))
        for line in stream:
            row = json.loads(line)
            tenant = row['tenant_id']
            check(tenant not in hashes, 'no duplicate independent tenant')
            if migration:
                for source in row['reservations']:
                    check('reservation_source_id' in source and source['reservation_source_id'] is None,
                          f'only explicit NULL historical attachment added: {tenant}')
                    source.pop('reservation_source_id')
            hashes[tenant] = facts_hash(row)
            if tenant in keep:
                rows[tenant] = row
    check(identity == IDENTITY, f'protected read-only identity: {path.name}')
    return hashes, rows


def unchanged(before, after):
    for tenant, digest in before.items():
        check(after.get(tenant) == digest, f'complete pre-existing tenant facts unchanged: {tenant}')
    counts['unchanged_preexisting_tenants'] = len(before)
    counts['additional_tenants'] = len(set(after) - set(before))


def child_id(root, sku):
    return str(UUID(bytes=hashlib.sha1(UUID(root).bytes + UUID(sku).bytes).digest()[:16], version=5))


def http_items(log, fixtures):
    by_tenant = defaultdict(list)
    for tenant, actor, action, request, headers, status, result in re.findall(
        r'QA location command HTTP tenant=([a-f0-9-]+) actor=([a-f0-9-]+) action=(\S+) request=(\{.*?\}|null) headers=(\[.*?\]) status=(\d{3}) [^\n]*? body=(\{.*\})', log):
        if tenant in fixtures:
            check(headers == '[]' and actor == fixtures[tenant]['actor'], 'real authenticated actor and legacy route')
            by_tenant[tenant].append({'action': action, 'request': json.loads(request),
                                     'status': int(status), 'result': json.loads(result)})
    counts['actual_owned_http_requests'] = sum(map(len, by_tenant.values()))
    return by_tenant


def fixtures_in(log):
    fixtures = {tenant: {'actor': actor, 'warehouse': warehouse, 'parent': parent,
                         'root': root, 'children': [first, second], 'kind': 'kit'}
                for tenant, actor, warehouse, parent, root, first, second in re.findall(
        r'QA kit source fixture tenant=([a-f0-9-]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) parent=([a-f0-9-]+) root=([a-f0-9-]+) first=([a-f0-9-]+) second=([a-f0-9-]+)', log)}
    for tenant, actor, warehouse, sku, root in re.findall(
        r'QA kit source (?:direct|historical-direct) tenant=([a-f0-9-]+) actor=([a-f0-9-]+) warehouse=([a-f0-9-]+) sku=([a-f0-9-]+) root=([a-f0-9-]+)', log):
        fixtures[tenant] = {'actor': actor, 'warehouse': warehouse, 'parent': sku,
                            'root': root, 'children': [sku], 'kind': 'direct'}
    return fixtures


def original():
    log = text(BASE / f'{PREFIX}-before-repair.log')
    fixtures = fixtures_in(log)
    check(len(fixtures) == 5, 'five original fixtures')
    before, _ = streamed(BASE / f'{PREFIX}-qa-before-repair.jsonl')
    after, owned = streamed(BASE / f'{PREFIX}-qa-original-after.jsonl', fixtures)
    unchanged(before, after)
    check(fixtures.keys().isdisjoint(before), 'five original fixtures are new')
    results = re.findall(r'test result: FAILED\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s', log)
    check(bool(results) and tuple(map(int, results[-1][:3])) == (0, 5, 0), 'original five genuine failures')
    tests = re.findall(r'test inventory_kit_sources::(kit_snapshot_\S+) \.\.\. (ok|FAILED)', log)
    check(len(tests) == 5 and all(state == 'FAILED' for _, state in tests), 'all five named original counterexamples')
    http = http_items(log, fixtures)
    check(counts['actual_owned_http_requests'] == 10, 'original ten actual route requests')
    scenarios = Counter()
    for tenant, f in fixtures.items():
        row = owned[tenant]
        check(row['name'] == 'QA-INVENTORY-KIT-SOURCES', 'original fixture tenant label')
        requests = http[tenant]
        check(len(requests) == 2 and requests[0]['status'] == 200, 'original initial reserve succeeds')
        last = requests[-1]
        if f['kind'] == 'direct':
            check(last['action'] == 'releases' and last['status'] == 500, 'original direct reclassification failure')
            scenarios['direct-reclassification'] += 1
        elif last['action'] == 'releases':
            check(last['status'] == 500, 'original missing BOM release fails')
            scenarios['deleted-bom'] += 1
        elif last['request']['quantity'] == 2:
            check(last['status'] == 200, 'original changed root qty wrongly accepted')
            scenarios['changed-quantity'] += 1
        elif last['request']['sku_id'] != f['parent']:
            check(last['status'] == 200, 'original different parent wrongly accepted')
            scenarios['changed-parent'] += 1
        else:
            check(last['status'] == 200 and len(row['reservations']) == 3, 'original changed BOM appended third child')
            scenarios['changed-components'] += 1
        check(all(source['status'] == 'RESERVED' for source in row['reservations']), 'original erroneous terminal/source facts preserved')
        for sku in f['children']:
            source_id = f['root'] if f['kind'] == 'direct' else child_id(f['root'], sku)
            check(any(s['sku_id'] == sku and s['order_item_id'] == source_id and s['qty_micros'] == 1_000_000
                      for s in row['reservations']), 'original exact child identity/qty')
    check(scenarios == Counter({'direct-reclassification': 1, 'deleted-bom': 1,
                               'changed-quantity': 1, 'changed-parent': 1, 'changed-components': 1}),
          'all five distinct original defects are bound to real requests')
    manifest = json.loads(text(BASE / 'free-erp-inventory-consumers-kit-sources-original-after-source.json'))['files']
    names = ['crates/erp-repo/src/inventory.rs', 'crates/erp-repo/src/lib.rs',
             'crates/erp-api/src/inventory_views.rs',
             'crates/erp-api/tests/free_erp_acceptance/inventory_kit_sources.rs',
             'crates/erp-api/tests/free_erp_acceptance.rs']
    for name in names:
        archive = BASE / f'{PREFIX}-original-{name.replace("/", "_")}.txt'
        check(hashlib.sha256(archive.read_bytes()).hexdigest() == manifest[name], f'original actual source archive: {name}')
    counts['owned_tenants'] = len(fixtures)
    counts['original_scenarios'] = dict(scenarios)


def migration():
    before, _ = streamed(BASE / f'{PREFIX}-qa-before-migration.jsonl')
    after, _ = streamed(BASE / f'{PREFIX}-qa-after-migration.jsonl', migration=True)
    unchanged(before, after)
    with (BASE / f'{PREFIX}-qa-schema-after-migration.jsonl').open(encoding='utf-8-sig') as stream:
        check(json.loads(next(stream)) == IDENTITY, 'schema exact QA identity')
        schema = json.loads(next(stream))
        check(not stream.read().strip(), 'single complete schema snapshot')
    check(schema['rows']['roots'] == schema['rows']['components'] == 0, 'no invented historical root or component snapshot')
    check(schema['rows']['reservations'] == schema['rows']['unmapped'], 'all historical rows retain explicit NULL attachment')
    check(all(c['validated'] for c in schema['constraints']), 'schema constraints validated')
    triggers = {t['name']: t for t in schema['triggers']}
    for name in ['inventory_order_reservation_root_immutable', 'inventory_order_reservation_component_immutable',
                 'inventory_order_reservation_attachment_immutable', 'inventory_order_reservation_root_complete',
                 'inventory_order_reservation_components_complete', 'inventory_order_reservation_children_complete']:
        check(name in triggers and triggers[name]['enabled'] == 'O', f'actual enabled source guard: {name}')
    constraints = schema['constraints']
    check(any(c['table'] == 'inventory_order_reservation_components' and c['deferrable'] and c['initially_deferred']
              and 'REFERENCES inventory_reservations' in c['definition'] for c in constraints), 'actual deferred child attachment FK')
    applied = text(BASE / f'{PREFIX}-migration.log')
    check('COMMIT' in applied and 'ERROR' not in applied, 'original successful protected migration log')
    index = text(BASE / f'{PREFIX}-index.log')
    check('CREATE INDEX' in index and 'COMMIT' in index and 'ERROR' not in index, 'protected reverse-index correction log')
    counts['historical_source_rows_kept_unmapped'] = schema['rows']['unmapped']


def micros(value):
    quantity = Decimal(value) * 1_000_000
    if quantity != quantity.to_integral_value():
        raise ValueError('quantity exceeds six decimal digits')
    return int(quantity)


def instant(value):
    return datetime.fromisoformat(value.replace('Z', '+00:00')).astimezone(timezone.utc)


def repaired(stage):
    log = text(BASE / f'{PREFIX}-{stage}.log')
    fixtures = fixtures_in(log)
    check(len(fixtures) == 18, 'all eighteen new owned fixture tenants')
    before, _ = streamed(BASE / f'{PREFIX}-qa-before-{stage}.jsonl')
    after, owned = streamed(BASE / f'{PREFIX}-qa-after-{stage}.jsonl', fixtures)
    unchanged(before, after)
    check(fixtures.keys().isdisjoint(before), 'owned fixtures are new; old failing fixtures remain intact')
    results = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s', log)
    expected = 15 if stage == 'focused' else 174
    check(bool(results) and tuple(map(int, results[-1][:3])) == (expected, 0, 0), 'actual complete registered test outcome')
    tests = re.findall(r'test inventory_kit_source(?:s|_edges)::(kit_snapshot_\S+) \.\.\. (ok|FAILED)', log)
    check(len(tests) == len(set(name for name, _ in tests)) == 15 and all(state == 'ok' for _, state in tests),
          'all fifteen named source scenarios actually ran')
    http = http_items(log, fixtures)
    buckets = ['on_hand', 'reserved', 'frozen', 'in_transit']
    rank = {'QA_LEGACY_RESERVATION_OPENING': 0, 'QA_KIT_SOURCE_OPENING': 0,
            'QA_KIT_SOURCE_THIRD': 0, 'QA_KIT_SOURCE_SHORTAGE': 1, 'RESERVE': 2,
            'RESERVATION_CREATE': 2, 'RESERVATION_RELEASE': 3, 'SALES_SHIP': 3}
    for tenant, f in fixtures.items():
        row = owned[tenant]
        check(row['name'] == 'QA-INVENTORY-KIT-SOURCES', 'actual owned fixture name')
        check(not row['orders'] and not row['order_items'], 'synthetic UUID and order components do not invent a real order')
        check(not row['shared_ledgers'] and not row['reservation_events'] and not row['command_receipts'],
              'legacy branch does not invent location projections/events/original receipts')
        ledgers = row['ledgers']
        by_key = defaultdict(list)
        at_effect = {}
        for effect in ledgers:
            by_key[(effect['warehouse_id'], effect['sku_id'])].append(effect)
            check(effect['reason_type'] in rank, 'all exact canonical fixture effects have defined causal roles')
            counts[f'effect_{effect["reason_type"]}'] += 1
        current = {}
        for key, effects in by_key.items():
            check(sum(rank.get(e['reason_type'], -1) == 0 for e in effects) == 1, 'one actual controlled physical opening')
            # Each controlled SKU has one opening/create/terminal action. Parallel
            # retries do not add effects; never infer lock order from timestamps.
            cursor = {bucket: 0 for bucket in buckets}
            for effect in sorted(effects, key=lambda e: rank[e['reason_type']]):
                for bucket in buckets:
                    cursor[bucket] += effect[f'delta_{bucket}_micros']
                cursor['available'] = cursor['on_hand'] - cursor['reserved'] - cursor['frozen']
                check(all(value >= 0 for value in cursor.values()), 'exact causal quantity buckets stay nonnegative')
                at_effect[effect['ledger_id']] = dict(cursor)
            current[key] = dict(cursor)
        for balance in row['balances']:
            key = (balance['warehouse_id'], balance['sku_id'])
            check(key in current and all(balance[f'{bucket}_micros'] == current[key][bucket] for bucket in buckets),
                  'current authoritative balance equals every signed precise ledger delta')
            check(all(balance[bucket] == balance[f'{bucket}_micros'] // 1_000_000 for bucket in buckets),
                  'legacy whole fields remain floor projections of exact micro quantities')
            if f['kind'] == 'kit':
                check(balance['sku_id'] != f['parent'], 'kit root has no fabricated parent stock balance')
        sources = {source['order_item_id']: source for source in row['reservations']}
        roots = {root['source_id']: root for root in row['reservation_roots']}
        components = row['reservation_components']
        for root_id, root in roots.items():
            check(root['tenant_id'] == tenant and root['warehouse_id'] == f['warehouse']
                  and root['sku_id'] == f['parent'] and root['original_qty'] == 1,
                  'frozen original root tenant/warehouse/parent/request qty')
            relevant = [c for c in components if c['source_id'] == root_id]
            check(len(relevant) == root['component_count'] == (1 if f['kind'] == 'direct' else 2),
                  'exact complete root component cardinality')
            check({c['sku_id'] for c in relevant} == set(f['children']), 'later catalog changes do not alter the applied SKU set')
            for component in relevant:
                expected_id = root['root_order_item_id'] if root['definition_kind'] == 'DIRECT' else child_id(root['root_order_item_id'], component['sku_id'])
                check(component['tenant_id'] == tenant and component['order_item_id'] == expected_id and component['qty'] == 1,
                      'independent raw-UUIDv5 component identity and original whole qty')
                source = sources.get(expected_id)
                check(source is not None and source['reservation_source_id'] == root_id
                      and source['sku_id'] == component['sku_id'] and source['warehouse_id'] == root['warehouse_id']
                      and source['order_id'] == root['order_id'], 'mapped physical child matches frozen original root context')
                check(component['bom_version'] == (None if root['definition_kind'] == 'DIRECT' else 7 if root['definition_kind'] == 'ORDER_COMPONENTS' else 1),
                      'applied actual component version is frozen')
            requests = [item for item in http[tenant] if item['action'] == 'reservations' and item['status'] == 200
                        and item['request']['order_item_id'] == root['root_order_item_id']]
            check(bool(requests) and all(item['request']['quantity'] == root['original_qty'] and item['request']['sku_id'] == root['sku_id']
                                         and item['request'].get('order_id') == root['order_id'] for item in requests),
                  'frozen root binds an actual accepted initial request and all accepted replays')
        check(len(components) == sum(root['component_count'] for root in roots.values()), 'no unattached/new component snapshot')
        for source_id, source in sources.items():
            check(source['qty'] == 1 and source['qty_micros'] == source['original_qty_micros'] == 1_000_000,
                  'terminal legacy quantity preserves exact original whole unit')
            check(all(source[key] is None for key in ['source_type', 'source_id', 'location_id', 'batch_code', 'quality_status',
                                                      'source_context', 'released_micros', 'source_request_id']),
                  'no invented original actor/location/new protocol identity/current remainder')
            effects = [e for e in ledgers if e['ref_type'] == 'ORDER_ITEM' and e['ref_id'] == source_id]
            create = [e for e in effects if e['reason_type'] in ['RESERVE', 'RESERVATION_CREATE']]
            released = [e for e in effects if e['reason_type'] == 'RESERVATION_RELEASE']
            consumed = [e for e in effects if e['reason_type'] == 'SALES_SHIP']
            check(len(create) == 1 and len(released) == (source['status'] == 'RELEASED')
                  and len(consumed) == (source['status'] == 'CONSUMED'), 'one original create and at most one actual terminal effect')
            for effect in effects:
                check(effect['warehouse_id'] == source['warehouse_id'] and effect['sku_id'] == source['sku_id'], 'source effect exact physical key')
                expected_reserved = 1_000_000 if effect in create else -1_000_000
                expected_hand = -1_000_000 if effect in consumed else 0
                check(effect['delta_reserved_micros'] == expected_reserved and effect['delta_on_hand_micros'] == expected_hand
                      and effect['delta_frozen_micros'] == effect['delta_in_transit_micros'] == 0, 'exact original reserve/release/consume effect')
            if source['reservation_source_id'] is not None:
                check(source['reservation_source_id'] in roots, 'every mapped source belongs to a real complete root')
        operations = row['operations']
        check(len(operations) == len(row['audits']), 'all required operation/audit pairs commit together')
        for operation in operations:
            metadata = operation['metadata']
            action = 'reservations' if operation['action'] == 'INVENTORY_RESERVED' else 'releases'
            check(operation['action'] in ['INVENTORY_RESERVED', 'INVENTORY_RELEASED']
                  and operation['actor_type'] == 'USER' and operation['actor_user_id'] == f['actor']
                  and operation['ref_type'] == 'ORDER_ITEM', 'required log action and actual authenticated actor')
            matches = [a for a in row['audits'] if a['action'] == ('INVENTORY.RESERVE' if action == 'reservations' else 'INVENTORY.RELEASE')
                       and a['after_json'] == metadata and a['user_id'] == f['actor']]
            check(len(matches) == 1, 'one matching actual required audit')
            root_uuid = operation['ref_id']
            check(metadata['warehouse_id'] == f['warehouse'] and metadata['sku_id'] == f['parent']
                  and metadata['order_item_id'] == root_uuid and metadata['qty'] == 1, 'required root log retains requested intent')
            successful = [item for item in http[tenant] if item['action'] == action and item['status'] == 200
                          and item['request']['order_item_id'] == root_uuid]
            check(bool(successful), 'committed required log binds actual successful original HTTP action')
            if action == 'reservations':
                check(all(metadata['order_id'] == item['request'].get('order_id') for item in successful), 'actual original order context retained')
            result = metadata['result']
            child = root_uuid if f['kind'] == 'direct' else child_id(root_uuid, result['sku_id'])
            reason = 'RESERVATION_CREATE' if action == 'reservations' else 'RESERVATION_RELEASE'
            effects = [e for e in ledgers if e['ref_id'] == child and e['reason_type'] == reason]
            check(len(effects) == 1 and result['warehouse_id'] == f['warehouse'], 'logged result is the actual last physical child effect')
            if effects:
                check({key: micros(value) for key, value in result['quantities'].items()} == at_effect[effects[0]['ledger_id']],
                      'independently reconstructed exact log outcome')
            check(any(item['result']['record']['quantities'] == result['quantities'] for item in successful), 'first actual response matches committed log outcome')
        grouped = defaultdict(list)
        for item in http[tenant]:
            if item['status'] == 200:
                quantities = item['result']['record']['quantities']
                check(micros(quantities['available']) == micros(quantities['on_hand']) - micros(quantities['reserved']) - micros(quantities['frozen']),
                      'actual HTTP response exact availability arithmetic')
                check(item['result'].get('request_id') is None and 'effect_id' not in item['result'], 'legacy response does not fabricate immutable command receipt')
                grouped[(item['action'], item['request']['order_item_id'])].append(item)
        for (action, root_uuid), responses in grouped.items():
            expected_action = 'INVENTORY_RESERVED' if action == 'reservations' else 'INVENTORY_RELEASED'
            logs = [op for op in operations if op['action'] == expected_action and op['ref_id'] == root_uuid]
            old_direct = f'QA kit source historical-direct tenant={tenant} ' in log and action == 'reservations'
            check(len(logs) == (0 if old_direct else 1), 'all successful replays add zero duplicate root logs')
        for event in row['outbox']:
            payload = event['payload']
            check(event['event_type'] == 'INVENTORY_CHANGED' and payload['tenant_id'] == tenant and payload['quantity_scale'] == 1_000_000,
                  'actual producer event and micro-unit scale')
            quantities = {bucket: int(payload[f'{bucket}_micros']) for bucket in buckets + ['available']}
            check(quantities['available'] == quantities['on_hand'] - quantities['reserved'] - quantities['frozen'], 'actual producer exact availability')
            effects = [e for e in ledgers if e['sku_id'] == payload['sku_id'] and instant(e['occurred_at']) == instant(payload['occurred_at'])]
            check(len(effects) == 1 and at_effect[effects[0]['ledger_id']] == quantities, 'actual Outbox binds its precise canonical producer effect')
            if len(effects) == 1:
                counts[f'outbox_effect_{effects[0]["reason_type"]}'] += 1
        for key in ['balances', 'ledgers', 'reservations', 'reservation_roots', 'reservation_components', 'operations', 'audits', 'outbox']:
            counts[key] += len(row[key])
    faults = re.findall(r'QA kit source log rollback tenant=([a-f0-9-]+) action=(\S+) table=(\S+) before=(\{.*\}) after=(\{.*\})', log)
    check(len(faults) == 4 and {(action, table) for _, action, table, _, _ in faults}
          == {(action, table) for action in ['reservations', 'releases'] for table in ['operation_logs', 'audit_logs']}, 'four actual specified late log faults')
    for tenant, action, table, initial, final in faults:
        check(json.loads(initial) == json.loads(final), 'all children/root/list/stock/logs/Outbox match complete fault checkpoint')
        failed = [item for item in http[tenant] if item['action'] == action and item['status'] == 500]
        check(len(failed) == 1 and 'check option' in failed[0]['result']['message'] and table in failed[0]['result']['message'],
              'actual PostgreSQL required-log check-option rejection')
    shortage = re.findall(r'QA kit source shortage tenant=([a-f0-9-]+) late=([a-f0-9-]+) before=(\{.*\}) after=(\{.*\})', log)
    check(len(shortage) == 1, 'one real later sorted component shortage')
    for tenant, late, initial, final in shortage:
        initial, final = json.loads(initial), json.loads(final)
        check(initial == final and not final['sources'] and not final['components'] and not final['reservations'], 'late physical shortage rolls back every earlier child and root')
        check(late == max(fixtures[tenant]['children']), 'insufficient component is actually later in physical execution order')
    guards = re.findall(r'QA kit source database guard tenant=([a-f0-9-]+) case=(\S+) rejected=true error=([^\n]+)', log)
    check(len(guards) == len({case for _, case, _ in guards}) == 10 and all('Some(Database(' in error for _, _, error in guards),
          'ten actual immediate/deferred database guards reached real PostgreSQL errors')
    same = re.findall(r'QA kit source twenty same tenant=([a-f0-9-]+) action=(\S+) quantity=(\d+) completed=(\d+)', log)
    check(len(same) == 3 and all(int(completed) == 20 for _, _, _, completed in same), 'actual three twenty-way root/action/quantity barriers')
    for tenant, action, quantity, _ in same:
        requests = [item for item in http[tenant] if item['action'] == action and item['request']['quantity'] == int(quantity)]
        check(len(requests) == (21 if action == 'releases' else 20)
              and all(item['status'] == (409 if quantity == '2' else 200) for item in requests), 'every same-root request response, including terminal replay')
    different = re.findall(r'QA kit source twenty different tenant=([a-f0-9-]+) success=(\d+) conflicts=(\d+)', log)
    check(len(different) == 1 and tuple(map(int, different[0][1:])) == (1, 19), 'twenty distinct roots cannot over-occupy or leave partial groups')
    for tenant, _, _ in different:
        requests = http[tenant]
        check(len(requests) == len({item['request']['order_item_id'] for item in requests}) == 20
              and Counter(item['status'] for item in requests) == {200: 1, 409: 19}, 'all actual different-root requests and conflicts retained')
    for tenant, root in re.findall(r'QA kit source historical-kit tenant=([a-f0-9-]+) root=([a-f0-9-]+) status=review', log):
        row = owned[tenant]
        check(not row['reservation_roots'] and not row['reservation_components'] and len(row['reservations']) == 2
              and all(s['reservation_source_id'] is None for s in row['reservations']), 'historical kit is left unmapped; no guessed frozen root')
        check(len(http[tenant]) == 2 and all(item['status'] == 409 and item['result']['code'] == 'reservation_source_reconciliation_required' for item in http[tenant]),
              'both actual historical root actions explicitly require review')
    limit = re.findall(r'QA kit source limit tenant=([a-f0-9-]+) components=1001 rejected=true', log)
    check(len(limit) == 1, 'one actual 1001-component rejection')
    for tenant in limit:
        row = owned[tenant]
        check(len(row['sku_boms']) == 1001 and not row['reservation_roots'] and not row['reservation_components'] and not row['reservations'], 'actual cap fixture has real catalog components and zero partial source')
        check(len(http[tenant]) == 1 and http[tenant][0]['status'] == 422 and http[tenant][0]['result']['code'] == 'reservation_components_limit', 'explicit actual cap HTTP outcome')
    consume = re.findall(r'QA kit source consume tenant=([a-f0-9-]+) root=([a-f0-9-]+) sku=([a-f0-9-]+)', log)
    check(len(consume) == 1, 'one actual public repository frozen-group consume/replay')
    for tenant, _, _ in consume:
        row = owned[tenant]
        check(len(row['reservations']) == 2 and all(s['status'] == 'CONSUMED' for s in row['reservations'])
              and len([e for e in row['ledgers'] if e['reason_type'] == 'SALES_SHIP']) == 2, 'consume/replay has only two exact terminal effects')
    expected_counts = {'balances': 35, 'ledgers': 80, 'reservations': 30, 'reservation_roots': 14,
                       'reservation_components': 27, 'operations': 21, 'audits': 21, 'outbox': 39}
    check(all(counts[key] == value for key, value in expected_counts.items()), 'complete exact scenario totals including rollback/legacy/parallel cases')
    # The actual precise producer emits whenever exact available changes and
    # an ACTIVE mapping exists. Include ten mapped component release events;
    # the first verifier's preserved failure incorrectly omitted those events.
    check({reason: counts[f'outbox_effect_{reason}'] for reason in ['RESERVATION_CREATE', 'RESERVATION_RELEASE', 'RESERVE', 'QA_KIT_SOURCE_SHORTAGE']}
          == {'RESERVATION_CREATE': 26, 'RESERVATION_RELEASE': 10, 'RESERVE': 2, 'QA_KIT_SOURCE_SHORTAGE': 1},
          'all actual producer effect classes including availability increases')
    check(counts['actual_owned_http_requests'] == 114, 'all 114 actual owned requests retained')
    check(Counter(item['status'] for items in http.values() for item in items) == {200: 63, 409: 46, 422: 1, 500: 4},
          'all actual success/input/conflict/late-log fault outcomes retained')
    counts['owned_tenants'] = len(fixtures)
    counts['registered_tests'] = expected
    counts['test_seconds'] = float(results[-1][3]) if results else None
    counts['http_status_counts'] = dict(Counter(item['status'] for items in http.values() for item in items))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--stage', choices=['original', 'migration', 'focused', 'full'], required=True)
    parser.add_argument('--revision', choices=['v2'])
    args = parser.parse_args()
    revision = f'-{args.revision}' if args.revision else ''
    destination = BASE / f'{PREFIX}-{args.stage}-independent{revision}.json'
    if destination.exists():
        raise SystemExit('refusing to replace historical verification')
    if args.stage in ['focused', 'full']:
        repaired(args.stage)
    else:
        {'original': original, 'migration': migration}[args.stage]()
    report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'stage': args.stage,
              'status': 'FAIL' if errors else 'PASS', 'checks': checks, 'counts': dict(counts),
              'errors': errors,
              'scope': 'complete protected QA row sets, original actual route failures and source integrity; not production/performance'}
    destination.write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n', encoding='utf8')
    print(json.dumps(report, ensure_ascii=False))
    raise SystemExit(bool(errors))


if __name__ == '__main__':
    main()
