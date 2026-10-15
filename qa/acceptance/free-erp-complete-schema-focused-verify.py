"""Read the final manufacturing originals and preserve all pre-existing QA rows."""
import json
import re
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
helpers = runpy.run_path(str(BASE / 'free-erp-product-web-fixture.py'))
before = json.loads((BASE / 'free-erp-complete-schema-oct8-focused-before.json').read_text())
after = json.loads((BASE / 'free-erp-complete-schema-oct8-late-after.json').read_text())
log = (BASE / 'free-erp-complete-schema-oct8-production-final.log').read_text(encoding='utf-8-sig')
owned = sorted(set(re.findall(r'QA production_release_fixture tenant=([a-f0-9-]{36})', log)))
assert owned and '24 passed; 0 failed; 0 ignored' in log
scope = ','.join(helpers['literal'](value) for value in owned)
tables = ['products', 'skus', 'mfg_receipts', 'mfg_receipt_release_commands', 'mfg_receipt_release_effects', 'command_receipts']
members = [f"'{table}',(SELECT coalesce(json_agg(to_jsonb(t)),'[]'::json) FROM {table} t WHERE tenant_id IN ({scope}))" for table in tables]
facts = json.loads(helpers['query']('SELECT json_build_object(' + ','.join(members) + ');'))
(BASE / 'free-erp-complete-schema-oct8-production-facts.json').write_text(json.dumps(facts, indent=2) + '\n', encoding='utf-8')
checks = []


def check(name, actual, expected):
    row = {'name': name, 'pass': actual == expected}
    if actual != expected:
        row.update(actual=actual, expected=expected)
    checks.append(row)


check('all current source hashes stable', before['source_sha256'], after['source_sha256'])
for table, hashes in before['protected_table_hashes'].items():
    check('all original full-row digests unchanged:' + table, after['protected_table_hashes'].get(table), hashes)
products = {(row['tenant_id'], row['product_id']) for row in facts['products']}
for sku in facts['skus']:
    check('real SKU parent ownership:' + sku['sku_id'], (sku['tenant_id'], sku['product_id']) in products, True)
commands = {(row['tenant_id'], row['request_id']): row for row in facts['mfg_receipt_release_commands']}
for key, row in commands.items():
    effects = [item for item in facts['mfg_receipt_release_effects'] if (item['tenant_id'], item['request_id']) == key]
    check('original effect pair:' + key[1], sorted(item['quality_status'] for item in effects), ['GOOD', 'HOLD'])
    check('original stock transfer net zero:' + key[1], sum(item['delta_on_hand_micros'] for item in effects), 0)
    check('original frozen stock released:' + key[1], sum(item['delta_frozen_micros'] for item in effects), -row['qty_micros'])
    check('snapshot source quantity:' + key[1], int(row['snapshot']['qty_micros']), row['qty_micros'])
    receipt = next(item for item in facts['command_receipts'] if (item['tenant_id'], item['request_id']) == key)
    check('successful original receipt:' + key[1], (receipt['status'], receipt['effect_id']), ('SUCCEEDED', row['receipt_id']))
for line in log.splitlines():
    if not line.startswith('QA production_release_http ') or '/release input=' not in line or ' status=200 response=' not in line:
        continue
    tenant = re.search(r'tenant=([a-f0-9-]{36})', line)[1]
    body = json.loads(line.split(' response=', 1)[1])
    original = commands[(tenant, body['request_id'])]
    check('actual HTTP original receipt identity:' + body['request_id'], body['effect_id'], original['receipt_id'])
    check('actual HTTP original available quantity:' + body['request_id'], int(body['available_micros_exact']), original['available_total_micros'])
for suffix, count in [('production-final', 24), ('bom', 2), ('password-migrated', 1), ('material-final', 7), ('guard-test', 1)]:
    text = (BASE / f'free-erp-complete-schema-oct8-{suffix}.log').read_text(encoding='utf-8-sig')
    check('actual registered result:' + suffix, f'{count} passed; 0 failed; 0 ignored' in text, True)
failures = [row for row in checks if not row['pass']]
report = {'status': 'PASS' if not failures else 'FAIL', 'checks': len(checks), 'failures': failures,
          'protected_tenants': len(before['protected_tenants']), 'protected_tables': len(before['protected_table_hashes']),
          'protected_rows': sum(map(len, before['protected_table_hashes'].values())), 'source_inputs': len(before['source_sha256']),
          'owned_production_tenants': owned, 'original_release_commands': len(commands), 'assertions': checks,
          'scope': 'recorded 319-migration targeted cases plus SQL original release effects and old-row protection; not full 576 qualification'}
(BASE / 'free-erp-complete-schema-oct8-focused-independent.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(report['status'], report['checks'], 'checks', len(failures), 'failures')
if failures:
    raise SystemExit(1)
