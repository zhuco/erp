"""Check real pending/recovery/conflict captures, SQL facts and screenshot geometry."""
import json
from datetime import datetime
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'output/playwright/pending-ui'


def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def main():
    fixture = load(ROOT / 'qa/acceptance/free-erp-product-web-oct8-fixture.json')
    before = load(ROOT / 'qa/acceptance/free-erp-product-web-oct8-pending-before.json')
    after = load(ROOT / 'qa/acceptance/free-erp-product-web-oct8-pending-after.json')
    lost = load(BASE / 'response-lost.json')
    recovered = load(BASE / 'recovered.json')
    peer = load(BASE / 'peer-write.json')
    rejected = load(BASE / 'conflict-rejected.json')
    latest = load(BASE / 'conflict-latest.json')
    saved = load(BASE / 'rebased-save.json')
    pending = load(BASE / 'pending-indexeddb.json')[0]['checkpoint']
    complete = load(BASE / 'complete-indexeddb.json')[0]['checkpoint']
    checks = []

    def equal(name, actual, expected):
        record = {'name': name, 'pass': actual == expected}
        if actual != expected:
            record.update(actual=actual, expected=expected)
        checks.append(record)

    def moment(value):
        return datetime.fromisoformat(value.replace('Z', '+00:00'))

    equal('complete Rust/WASM source window stable', before['source_sha256'], after['source_sha256'])
    equal('real committed response', lost['status'], 200)
    equal('original receipt HTTP', recovered['status'], 200)
    equal('recovered exact receipt', recovered['receipt'], lost['receipt'])
    equal('pending original command', pending['batch']['items'][0]['command'], lost['command'])
    equal('persisted uncertain outcome', pending['batch']['items'][0]['state'], 'Unknown')
    equal('definite version rejection HTTP receipt', rejected['status'], 200)
    equal('version conflict code', rejected['receipt']['outcome']['Rejected']['code'], 'VERSION_CONFLICT')
    equal('stale expected version', moment(rejected['command']['expected_version']), moment(lost['receipt']['outcome']['Applied']['version']))
    equal('peer used last confirmed version', moment(peer['command']['expected_version']), moment(lost['receipt']['outcome']['Applied']['version']))
    equal('latest real query HTTP', latest['status'], 200)
    equal('latest query real peer value', latest['product']['name'], 'QA UI peer update')
    equal('latest query real peer version', moment(latest['product']['updated_at']), moment(peer['receipt']['outcome']['Applied']['version']))
    equal('new command after explicit review', saved['command']['operation_id'] == rejected['command']['operation_id'], False)
    equal('review preserves user input', saved['command']['values'], rejected['command']['values'])
    equal('review explicitly adopts peer version', moment(saved['command']['expected_version']), moment(peer['receipt']['outcome']['Applied']['version']))
    equal('reviewed save HTTP', saved['status'], 200)
    equal('reviewed actual product identity', saved['receipt']['outcome']['Applied']['row'], fixture['product_id'])
    final = next(row for row in after['facts']['products'] if row['product_id'] == fixture['product_id'])
    original = next(row for row in before['facts']['products'] if row['product_id'] == fixture['product_id'])
    equal('independent final value', final['name'], 'QA UI reviewed draft')
    equal('only intended product fields changed', [key for key in final if final[key] != original[key]], ['name', 'updated_at'])
    equal('independent final version', moment(final['updated_at']), moment(saved['receipt']['outcome']['Applied']['version']))
    for table, key in [('products', 'product_id'), ('skus', 'sku_id')]:
        untouched = [row for row in before['facts'][table] if table != 'products' or row[key] != fixture['product_id']]
        for row in untouched:
            equal('unchanged full row ' + table + ':' + row[key], next(item for item in after['facts'][table] if item[key] == row[key]), row)
    old_ops = {row['operation_id']: row for row in before['facts']['grid_product_operations']}
    operations = {row['operation_id']: row for row in after['facts']['grid_product_operations']}
    equal('exact four new original operations', len(operations) - len(old_ops), 4)
    for identity, row in old_ops.items():
        equal('prior original operation preserved ' + identity, operations[identity], row)
    for name, call in [('lost', lost), ('peer', peer), ('conflict', rejected), ('review', saved)]:
        stored = operations[call['command']['operation_id']]
        equal(name + ' original command in database', stored['command'], call['command'])
        equal(name + ' actual original receipt', stored['receipt'], call['receipt'])
        equal(name + ' actor', stored['actor_user_id'], fixture['user_id'])
    for table in ['operation_logs', 'audit_logs']:
        equal(table + ' only three actual new effects', len(after['facts'][table]) - len(before['facts'][table]), 3)
        equal(table + ' rejection has no business effect', any(row.get('metadata', row.get('after_json'))['operation_id'] == rejected['command']['operation_id'] for row in after['facts'][table]), False)
    peer_session = next(row for row in after['facts']['auth_sessions'] if row['session_id'] == peer['session_id'])
    equal('peer uses actual separate session label', peer_session['client_label'], 'QA-owned-peer-version-check')
    equal('peer same allowed login account', peer_session['user_id'], fixture['user_id'])
    equal('completed checkpoint retains reviewed command', complete['batch']['items'][0]['command'], saved['command'])
    equal('completed checkpoint actual version', complete['batch']['items'][0]['state']['Complete']['Applied']['version'], saved['receipt']['outcome']['Applied']['version'])
    geometry = {}
    for name in ['02-normal', '03-editor', '04-pending', '05-pending-ready', '06-recovered', '11-final-saved']:
        with Image.open(BASE / (name + '.png')) as source:
            image = source.convert('RGB')
            # Stable product-code glyphs: scan above both rows so a shifted
            # second row cannot be mistaken for the first row.
            ys = [y for y in range(210, 300) if any(all(value < 125 for value in image.getpixel((x, y))) for x in range(334, 446))]
            geometry[name] = min(ys)
        equal(name + ' first row remains at baseline', geometry[name], 239)
    failures = [check for check in checks if not check['pass']]
    report = {'status': 'PASS' if not failures else 'FAIL', 'checks': len(checks), 'failures': failures,
              'input_count': len(before['source_sha256']), 'geometry_first_code_ink_y': geometry, 'assertions': checks,
              'scope': 'one real browser plus a real QA API peer session; controlled response loss; not two GUI clients or complete ERP'}
    (ROOT / 'qa/acceptance/free-erp-pending-ui-oct8-independent.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(report['status'], report['checks'], 'checks', len(failures), 'failures')
    if failures:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
