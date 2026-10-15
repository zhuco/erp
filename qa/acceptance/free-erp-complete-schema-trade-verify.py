"""Independently verify the formal-schema trade run and original protected rows."""
import json
import re
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
helpers = runpy.run_path(str(BASE / 'free-erp-product-web-fixture.py'))
log = (BASE / 'free-erp-complete-schema-oct8-trade-fixed.log').read_text(encoding='utf-8-sig')
before = json.loads((BASE / 'free-erp-complete-schema-oct8-fixed-before.json').read_text())
after = json.loads((BASE / 'free-erp-complete-schema-oct8-fixed-after.json').read_text())
owned = sorted(set(re.findall(r'QA trade_facts_fixture tenant=([a-f0-9-]{36})', log)))
if len(owned) != 10 or not re.search(r'9 passed; 0 failed; 0 ignored', log):
    raise SystemExit('requires the complete recorded nine-case formal trade run')
scope = ','.join(helpers['literal'](tenant) for tenant in owned)
tables = ['products', 'skus', 'trade_sales_documents', 'trade_sales_document_revisions', 'trade_sales_line_bindings',
          'shared_sales_revisions', 'shared_sales_lines', 'command_receipts', 'operation_logs', 'audit_logs', 'outbox_events']
members = []
for table in tables:
    suffix = "AND action='TRADE_SALES_SAVE'" if table == 'operation_logs' else "AND action='trade.sales.save'" if table == 'audit_logs' else "AND event_type='trade.sales.saved'" if table == 'outbox_events' else ''
    members.append(f"'{table}',(SELECT coalesce(json_agg(to_jsonb(t)),'[]'::json) FROM {table} t WHERE tenant_id IN ({scope}) {suffix})")
facts = json.loads(helpers['query']('SELECT json_build_object(' + ','.join(members) + ');'))
(BASE / 'free-erp-complete-schema-oct8-trade-facts.json').write_text(json.dumps(facts, indent=2) + '\n', encoding='utf-8')
checks = []


def check(name, actual, expected):
    record = {'name': name, 'pass': actual == expected}
    if actual != expected:
        record.update(actual=actual, expected=expected)
    checks.append(record)


check('source window stable', before['source_sha256'], after['source_sha256'])
for table, hashes in before['protected_table_hashes'].items():
    check('all original rows unchanged:' + table, after['protected_table_hashes'].get(table), hashes)
products = {(row['tenant_id'], row['product_id']): row for row in facts['products']}
skus = {(row['tenant_id'], row['sku_id']): row for row in facts['skus']}
for key, sku in skus.items():
    check('actual SKU product ownership:' + key[1], (sku['tenant_id'], sku['product_id']) in products, True)
for meta in facts['trade_sales_document_revisions']:
    tenant, revision = meta['tenant_id'], meta['revision_id']
    original = next(row for row in facts['shared_sales_revisions'] if row['tenant_id'] == tenant and row['revision_id'] == revision)
    lines = [row for row in facts['shared_sales_lines'] if row['tenant_id'] == tenant and row['revision_id'] == revision]
    bindings = [row for row in facts['trade_sales_line_bindings'] if row['tenant_id'] == tenant and row['revision_id'] == revision]
    receipt = next(row for row in facts['command_receipts'] if row['tenant_id'] == tenant and row['request_id'] == meta['request_id'])
    check('shared revision identity:' + revision, original['so_id'], meta['so_id'])
    check('one binding per real line:' + revision, sorted(row['line_no'] for row in bindings), sorted(row['line_no'] for row in lines))
    total = sum(int(row['qty_micros']) * row['price_cent'] // 1000000 - row['discount_cent'] + row['tax_cent'] for row in lines)
    check('independent original net cents:' + revision, total, meta['snapshot']['total_cent'])
    check('original succeeded command effect:' + revision, (receipt['status'], receipt['effect_id']), ('SUCCEEDED', revision))
    for table, reference, payload in [('operation_logs', 'ref_id', 'metadata'), ('audit_logs', 'entity_id', 'after_json')]:
        logs = [row for row in facts[table] if row['tenant_id'] == tenant and row[reference] == revision]
        check(table + ' exactly one:' + revision, len(logs), 1)
        if logs:
            check(table + ' actual frozen snapshot:' + revision, logs[0][payload], meta['snapshot'])
    events = [row for row in facts['outbox_events'] if row['tenant_id'] == tenant and row['payload'].get('revision_id') == revision]
    check('one actual saved event:' + revision, len(events), 1)
    if events:
        check('actual event original identity:' + revision, events[0]['payload'], {'so_id': meta['so_id'], 'revision_id': revision, 'request_id': meta['request_id']})
    for binding in bindings:
        check('actual frozen SKU owner:' + revision + ':' + str(binding['line_no']), (tenant, binding['sku_id']) in skus, True)
failures = [record for record in checks if not record['pass']]
report = {'status': 'PASS' if not failures else 'FAIL', 'checks': len(checks), 'failures': failures,
          'passed': 9, 'failed': 0, 'ignored': 0, 'owned_tenants': owned, 'protected_tenants': len(before['protected_tenants']),
          'protected_tables': len(before['protected_table_hashes']), 'protected_rows': sum(map(len, before['protected_table_hashes'].values())),
          'source_inputs': len(before['source_sha256']), 'assertions': checks,
          'scope': 'formal migration schema nine recorded real-password trade cases; not all trade/manufacturing/ERP'}
(BASE / 'free-erp-complete-schema-oct8-trade-independent.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(report['status'], report['checks'], 'checks', len(failures), 'failures')
if failures:
    raise SystemExit(1)
