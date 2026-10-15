"""Reconcile retained real browser captures with guarded direct SQL snapshots."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'output/playwright/product-web'


def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def main():
    fixture = load(ROOT / 'qa/acceptance/free-erp-product-web-oct8-fixture.json')
    first = load(ROOT / 'qa/acceptance/free-erp-product-web-oct8-before.json')
    second = load(ROOT / 'qa/acceptance/free-erp-product-web-oct8-after.json')
    before = load(ROOT / 'qa/acceptance/free-erp-product-web-oct8-toolbar-before.json')
    after = load(ROOT / 'qa/acceptance/free-erp-product-web-oct8-toolbar-after.json')
    loaded, sku, saved, readback = [load(BASE / name) for name in ['loaded.json', 'sku-query-fixed.json', 'saved.json', 'readback.json']]
    lost, recovered = [load(BASE / name) for name in ['toolbar-response-lost.json', 'toolbar-recovered.json']]
    checkpoint = load(BASE / 'toolbar-indexeddb-complete.json')[0]['checkpoint']
    checks = []

    def equal(name, actual, expected):
        checks.append({'name': name, 'pass': actual == expected})
        if actual != expected:
            checks[-1].update(actual=actual, expected=expected)

    def product(snapshot, key='product_id'):
        return next(row for row in snapshot['facts']['products'] if row['product_id'] == fixture[key])

    equal('first source window stable', first['source_sha256'], second['source_sha256'])
    equal('repaired source window stable', before['source_sha256'], after['source_sha256'])
    for name, status in [('login', loaded['login_status']), ('products', loaded['query_status']), ('options', loaded['options_status']),
                         ('SKU', sku['status']), ('save', saved['status']), ('readback', readback['status']),
                         ('committed lost response', lost['status']), ('original receipt read', recovered['status'])]:
        equal(name + ' HTTP status', status, 200)
    equal('complete initial product count', loaded['query']['total'], 2)
    equal('complete readback product count', readback['result']['total'], 2)
    equal('actual SKU', sku['result']['data'][0]['sku_id'], fixture['sku_id'])
    equal('actual SKU parent', sku['result']['data'][0]['product_id'], fixture['product_id'])
    equal('SKU minor units', sku['result']['data'][0]['cost_cent'], 230)
    for label, original, final in [('first', first, second), ('repaired', before, after)]:
        equal(label + ' other product full row unchanged', product(original, 'other_product_id'), product(final, 'other_product_id'))
        equal(label + ' SKU full row unchanged', original['facts']['skus'], final['facts']['skus'])
        equal(label + ' only intended product fields changed',
              [key for key in product(final) if product(final)[key] != product(original)[key]], ['name', 'updated_at'])
    equal('first persisted name', product(second)['name'], fixture['edited_name'])
    equal('first HTTP readback name', next(row for row in readback['result']['data'] if row['product_id'] == fixture['product_id'])['name'], fixture['edited_name'])
    equal('first expected original timestamp', saved['command']['expected_version'], loaded['query']['data'][0]['updated_at'])
    equal('first exactly one operation', len(second['facts']['grid_product_operations']), 1)
    equal('repaired exactly two operations', len(after['facts']['grid_product_operations']), 2)
    equal('original receipt equals committed response', recovered['receipt'], lost['receipt'])
    equal('repaired final product name', product(after)['name'], 'QA UI recovered product')
    for label, call, snapshot in [('first', saved, second), ('repaired', lost, after)]:
        operation = next(row for row in snapshot['facts']['grid_product_operations'] if row['operation_id'] == call['command']['operation_id'])
        equal(label + ' operation actor', operation['actor_user_id'], fixture['user_id'])
        equal(label + ' original command retained', operation['command'], call['command'])
        equal(label + ' original receipt retained', operation['receipt'], call['receipt'])
        equal(label + ' actual applied row', call['receipt']['outcome']['Applied']['row'], fixture['product_id'])
        for table in ['operation_logs', 'audit_logs']:
            row = next(row for row in snapshot['facts'][table] if row.get('metadata', row.get('after_json'))['operation_id'] == call['command']['operation_id'])
            equal(label + table + ' actor', row.get('actor_user_id', row.get('user_id')), fixture['user_id'])
            equal(label + table + ' action', row['action'], 'products.update')
    for table in ['operation_logs', 'audit_logs']:
        equal(table + ' exactly two committed records', len(after['facts'][table]), 2)
    equal('actual initial password session', len(second['facts']['auth_sessions']), 1)
    equal('actual initial session actor', second['facts']['auth_sessions'][0]['user_id'], fixture['user_id'])
    equal('IndexedDB completed original identity', checkpoint['batch']['items'][0]['command'], lost['command'])
    applied = checkpoint['batch']['items'][0]['state']['Complete']['Applied']
    equal('IndexedDB completed real version', applied['version'], lost['receipt']['outcome']['Applied']['version'])
    equal('IndexedDB actual actor scope', checkpoint['scope']['actor'], fixture['user_id'])
    failures = [check for check in checks if not check['pass']]
    report = {'status': 'PASS' if not failures else 'FAIL', 'checks': len(checks), 'failures': failures,
              'input_count_each_window': len(first['source_sha256']), 'scope': 'two recorded real browser product windows; direct SQL and IndexedDB; not whole ERP',
              'assertions': checks}
    (ROOT / 'qa/acceptance/free-erp-product-web-oct8-independent-final.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(report['status'], report['checks'], 'checks', len(failures), 'failures')
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
