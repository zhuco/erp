"""Verify full-target outcome, original-row preservation and explicit parent fixtures."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
log = (BASE / 'free-erp-formal-fixtures-oct8-full.log').read_text(encoding='utf-8-sig')
summary = re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s', log)
if not summary:
    raise SystemExit('full registered test process has not produced a terminal summary')
before = json.loads((BASE / 'free-erp-complete-schema-oct8-fixtures-before.json').read_text())
after = json.loads((BASE / 'free-erp-complete-schema-oct8-fixtures-after.json').read_text())
checks = []


def check(name, actual, expected):
    row = {'name': name, 'pass': actual == expected}
    if actual != expected:
        row.update(actual=actual, expected=expected)
    checks.append(row)


check('same complete source window', after['source_sha256'], before['source_sha256'])
for table, hashes in before['protected_table_hashes'].items():
    check('all old row values retained:' + table, after['protected_table_hashes'].get(table), hashes)
check('no tests omitted or ignored', int(summary[4]), 0)
check('all 576 registered cases delivered', int(summary[2]) + int(summary[3]), 576)
check('full actual result succeeds', int(summary[3]), 0)
edits = json.loads((BASE / 'free-erp-formal-fixtures-oct8-edits.json').read_text())
for name in sorted({row['file'] for row in edits['edits']}):
    source = (ROOT / name).read_text(encoding='utf-8')
    for statement in re.findall(r'sqlx::query\(\s*"((?:[^"\\]|\\.)*)"', source, re.S):
        for columns in re.findall(r'insert into skus\s*\(([^)]*)\)', statement, re.I):
            check('formal SKU parent declared:' + name, 'product_id' in columns, True)
failures = [row for row in checks if not row['pass']]
failed_cases = re.findall(r'^test (.+) \.\.\. FAILED$', log, re.M)
report = {'status': 'PASS' if not failures else 'PARTIAL', 'checks': len(checks), 'failures': failures,
          'tests': {'passed': int(summary[2]), 'failed': int(summary[3]), 'ignored': int(summary[4]), 'seconds': float(summary[5])},
          'failed_cases': failed_cases, 'protected_tenants': len(before['protected_tenants']),
          'protected_tables': len(before['protected_table_hashes']), 'protected_rows': sum(map(len, before['protected_table_hashes'].values())),
          'source_inputs': len(before['source_sha256']), 'assertions': checks,
          'scope': 'actual complete registered debug API target on formal QA schema plus all original-row hashes; not UI/Release/whole ERP'}
(BASE / 'free-erp-formal-fixtures-oct8-independent.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(report['status'], report['tests'], len(failures), 'verification failures')
if failures:
    raise SystemExit(1)
