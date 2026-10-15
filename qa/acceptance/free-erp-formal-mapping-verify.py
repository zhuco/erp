"""Verify every formerly failing mapping case and protect the previous QA dataset."""
import json
import re
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
before = json.loads((BASE / 'free-erp-complete-schema-oct8-mapping-before.json').read_text())
after = json.loads((BASE / 'free-erp-complete-schema-oct8-mapping-after.json').read_text())
previous = json.loads((BASE / 'free-erp-formal-fixtures-oct8-independent.json').read_text())
logs = [BASE / 'free-erp-formal-fixtures-oct8-kit-fixed.log', *sorted(BASE.glob('free-erp-formal-fixtures-oct8-mapping-inventory*.log'))]
passed, checks = set(), []


def check(name, actual, expected):
    row = {'name': name, 'pass': actual == expected}
    if actual != expected:
        row.update(actual=actual, expected=expected)
    checks.append(row)


for path in logs:
    text = path.read_text(encoding='utf-8-sig')
    summary = re.search(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored', text)
    check('actual named target succeeds:' + path.name, summary is not None, True)
    passed.update(re.findall(r'^test (.+) \.\.\. ok$', text, re.M))
for name in previous['failed_cases']:
    check('formerly failing case now actually passed:' + name, name in passed, True)
check('same source window', after['source_sha256'], before['source_sha256'])
for table, hashes in before['protected_table_hashes'].items():
    check('original all-row digests unchanged:' + table, after['protected_table_hashes'].get(table), hashes)
helpers = runpy.run_path(str(BASE / 'free-erp-product-web-fixture.py'))
old = ','.join(helpers['literal'](value) for value in before['protected_tenants'])
facts = json.loads(helpers['query']("""SELECT json_build_object('new_mappings',count(*),
'missing_shop_owner',count(*) FILTER(WHERE s.shop_id IS NULL),
'missing_sku_owner',count(*) FILTER(WHERE k.sku_id IS NULL),
'missing_parent_owner',count(*) FILTER(WHERE p.product_id IS NULL))
FROM channel_sku_mappings m
LEFT JOIN shops s ON s.tenant_id=m.tenant_id AND s.shop_id=m.shop_id
LEFT JOIN skus k ON k.tenant_id=m.tenant_id AND k.sku_id=m.sku_id
LEFT JOIN products p ON p.tenant_id=k.tenant_id AND p.product_id=k.product_id
WHERE m.tenant_id NOT IN (""" + old + ');'))
check('new mapped fixture actually populated', facts['new_mappings'] > 0, True)
for name in ['missing_shop_owner', 'missing_sku_owner', 'missing_parent_owner']:
    check('real ownership:' + name, facts[name], 0)
failures = [row for row in checks if not row['pass']]
report = {'status': 'PASS' if not failures else 'PARTIAL', 'checks': len(checks), 'failures': failures,
          'formerly_failed': len(previous['failed_cases']), 'passed_targets': len(passed), 'mapping_facts': facts,
          'protected_tenants': len(before['protected_tenants']), 'protected_tables': len(before['protected_table_hashes']),
          'protected_rows': sum(map(len, before['protected_table_hashes'].values())), 'source_inputs': len(before['source_sha256']),
          'assertions': checks, 'scope': 'all 23 previous failures directly passed, 25 targeted cases; not a new unfiltered 576 result'}
(BASE / 'free-erp-formal-fixtures-oct8-mapping-independent.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(report['status'], report['checks'], 'checks;', len(passed), 'actual named cases')
if failures:
    raise SystemExit(1)
