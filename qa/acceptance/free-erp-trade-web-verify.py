"""Independently reconcile recorded real browser calls against owned QA SQL facts."""
import hashlib
import json
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / 'output/playwright/trade-web'
fixture_api = runpy.run_path(str(ROOT / 'qa/acceptance/free-erp-trade-web-fixture.py'))
fixture = json.loads(fixture_api['MANIFEST'].read_text(encoding='utf-8'))
query, literal = fixture_api['query'], fixture_api['literal']


def load(name):
    return json.loads((ARTIFACTS / name).read_text(encoding='utf-8-sig'))


def main():
    tenant = literal(fixture['tenant_id'])
    assert query(f'SELECT name FROM tenants WHERE tenant_id={tenant};') == 'QA-TRADE-WEB-OCT8'
    tables = {
        'documents': ('trade_sales_documents', ''),
        'headers': ('trade_sales_document_revisions', ''),
        'revisions': ('shared_sales_revisions', ''),
        'lines': ('shared_sales_lines', ''),
        'bindings': ('trade_sales_line_bindings', ''),
        'receipts': ('command_receipts', ''),
        'operations': ('operation_logs', "AND action='TRADE_SALES_SAVE'"),
        'audits': ('audit_logs', "AND action='trade.sales.save'"),
        'outbox': ('outbox_events', "AND event_type='trade.sales.saved'"),
    }
    members = [f"'{key}',(SELECT coalesce(json_agg(to_jsonb(t)),'[]'::json) FROM {table} t WHERE tenant_id={tenant} {suffix})"
               for key, (table, suffix) in tables.items()]
    facts = json.loads(query('SELECT json_build_object(' + ','.join(members) + ');'))
    (ARTIFACTS / 'independent-db-facts.json').write_text(json.dumps(facts, indent=2) + '\n', encoding='utf-8')
    checks = []

    def equal(name, actual, expected):
        checks.append({'name': name, 'pass': actual == expected, 'actual': actual, 'expected': expected})

    reject = load('rejection.json')
    first = load('saved-v1.json')
    lost = load('committed-response-lost.json')
    recovered = load('recovered-v2.json')
    lookup = load('number-lookup.json')
    rejected_checkpoint = load('rejection-indexeddb.json')[0]['checkpoint']
    pending_checkpoint = load('pending-indexeddb.json')[0]['checkpoint']
    final_checkpoint = load('recovered-indexeddb.json')[0]['checkpoint']
    equal('rejection HTTP envelope', reject['status'], 200)
    equal('rejection explicit no commit', reject['result']['committed'], False)
    equal('rejection binds full original payload', reject['result']['fields'], reject['fields'])
    equal('rejection archived original bytes in IndexedDB', rejected_checkpoint['rejected']['original']['body'], reject['fields'])
    equal('rejection permits editing after durable transition', (rejected_checkpoint['pending'], rejected_checkpoint['draft']['changed']), (None, True))
    equal('corrected command retains order identity', first['fields']['so_id'], reject['fields']['so_id'])
    equal('corrected command changes original request', first['fields']['request_id'] == reject['fields']['request_id'], False)
    equal('pending IndexedDB binds undelivered request', pending_checkpoint['pending']['body'], lost['fields'])
    equal('pending client retains last confirmed version', pending_checkpoint['draft']['version'], 1)
    equal('relogin HTTP accepted', recovered['login_status'], 200)
    equal('original result HTTP accepted', recovered['recovery_status'], 200)
    equal('original result equals actual committed API document', recovered['result'], lost['result']['document'])
    equal('recovery uses same request identity', recovered['result']['header']['request_id'], lost['fields']['request_id'])
    equal('recovery durable pending cleared', final_checkpoint['pending'], None)
    equal('recovery durable draft clean', final_checkpoint['draft']['changed'], False)
    equal('recovery durable exact quantity/version', (final_checkpoint['draft']['lines'][0]['qty'], final_checkpoint['draft']['version']), ('1.250001', 2))
    equal('lookup HTTP accepted', lookup['status'], 200)
    equal('business-number lookup equals original recovered document', lookup['result'], recovered['result'])
    equal('exactly one managed business document', len(facts['documents']), 1)
    for key in tables:
        if key != 'documents':
            equal(f'exactly two committed {key}', len(facts[key]), 2)
    equal('database absent rejected command', any(row['request_id'] == reject['fields']['request_id'] for row in facts['receipts']), False)
    for call in [first, lost]:
        doc = call['result']['document']
        header = doc['header']
        request = call['fields']['request_id']
        version = header['version_no']
        equal(f'v{version} HTTP accepted', call['status'], 200)
        revision = next(row for row in facts['revisions'] if row['revision_id'] == header['revision_id'])
        line = next(row for row in facts['lines'] if row['revision_id'] == header['revision_id'])
        binding = next(row for row in facts['bindings'] if row['revision_id'] == header['revision_id'])
        meta = next(row for row in facts['headers'] if row['request_id'] == request)
        receipt = next(row for row in facts['receipts'] if row['request_id'] == request)
        equal(f'v{version} revision order/version/status', (revision['so_id'], revision['version_no'], revision['status']), (header['so_id'], version, 'SUBMITTED'))
        equal(f'v{version} original receipt successful effect', (receipt['status'], receipt['effect_id']), ('SUCCEEDED', header['revision_id']))
        equal(f'v{version} quantity independently equals submitted base units', int(line['qty_micros']), 1250000 if version == 1 else 1250001)
        equal(f'v{version} independent line net cents', int(line['qty_micros']) * line['price_cent'] // 1000000 - line['discount_cent'] + line['tax_cent'], header['total_cent'])
        equal(f'v{version} exact total', header['total_cent'], 293)
        equal(f'v{version} real SKU binding', binding['sku_id'], fixture['sku_id'])
        equal(f'v{version} real customer binding', meta['partner_id'], fixture['partner_id'])
        equal(f'v{version} recorded authenticated actor', meta['snapshot']['actor']['user_id'], fixture['user_id'])
        equal(f'v{version} preserved snapshot contents', meta['snapshot'], {key: value for key, value in header.items() if key not in ['so_id', 'revision_id', 'version_no', 'status', 'request_id']})
        audit = next(row for row in facts['audits'] if row['entity_id'] == header['revision_id'])
        operation = next(row for row in facts['operations'] if row['ref_id'] == header['revision_id'])
        event = next(row for row in facts['outbox'] if row['payload']['request_id'] == request)
        equal(f'v{version} audit actor', audit['user_id'], fixture['user_id'])
        equal(f'v{version} complete audit snapshot', audit['after_json'], meta['snapshot'])
        equal(f'v{version} complete operation snapshot', operation['metadata'], meta['snapshot'])
        equal(f'v{version} Outbox original identity', event['payload'], {'so_id': header['so_id'], 'revision_id': header['revision_id'], 'request_id': request})
        equal(f'v{version} submitted line semantic fields',
              [line[key] for key in ['item_code', 'price_cent', 'discount_cent', 'tax_cent', 'due_date', 'address', 'source_kind']],
              [call['fields']['items'][0][key] for key in ['sku_code', 'price_cent', 'discount_cent', 'tax_cent', 'due_date', 'address', 'source_kind']])
    equal('all two operations are actual user actions', [(row['actor_type'], row['actor_user_id']) for row in facts['operations']], [('USER', fixture['user_id'])] * 2)
    source_paths = [
        'apps/erp-ui/src/remote_view/trade_sales.rs', 'apps/erp-ui/src/remote_view/trade_sales/model.rs',
        'crates/erp-repo/src/shared_contract/sales_documents.rs', 'crates/erp-api/src/sales_orders/documents.rs',
        'build/cargo/debug/erp-api.exe', 'apps/erp-ui/pkg/erp_ui.js', 'apps/erp-ui/pkg/erp_ui_bg.wasm',
    ]
    failures = [check for check in checks if not check['pass']]
    report = {'status': 'PASS' if not failures else 'FAIL', 'checks': len(checks), 'failures': failures,
              'tenant_id': fixture['tenant_id'], 'scope': 'one owned QA enterprise, real Web HTTP/IndexedDB and direct psql facts',
              'assertions': checks,
              'sha256': {path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest() for path in source_paths}}
    (ROOT / 'qa/acceptance/free-erp-trade-web-oct8-independent.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(report['status'], report['checks'], 'checks', len(failures), 'failures')
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
