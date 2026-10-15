"""Formal-schema full target, all original rows, and actual allocation timestamps."""
import argparse
import json
import re
import runpy
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
parser = argparse.ArgumentParser()
mode = parser.add_mutually_exclusive_group()
mode.add_argument('--protected-full', action='store_true')
mode.add_argument('--resumed-full', action='store_true')
mode.add_argument('--lint-full', action='store_true')
mode.add_argument('--cost-input-full', action='store_true')
mode.add_argument('--trade-finance-full', action='store_true')
args = parser.parse_args()
run_prefix = 'free-erp-trade-finance-input-full-oct9' if args.trade_finance_full else 'free-erp-cost-input-full-oct9' if args.cost_input_full else 'free-erp-repo-lint-full-oct9' if args.lint_full else 'free-erp-resumed-full-oct9' if args.resumed_full else 'free-erp-protected-full-oct9' if args.protected_full else 'free-erp-runtime-tables-oct8'
phase_prefix = 'trade-finance-full' if args.trade_finance_full else 'cost-full' if args.cost_input_full else 'lint' if args.lint_full else 'resumed' if args.resumed_full else 'protected' if args.protected_full else 'runtime'
log = (BASE / (run_prefix + '-full.log')).read_text(encoding='utf-8-sig')
summary = re.search(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;.*?finished in ([\d.]+)s', log)
if not summary:
    raise SystemExit('full process has no terminal result; not qualified')
before = json.loads((BASE / ('free-erp-complete-schema-oct8-' + phase_prefix + '-before.json')).read_text())
after = json.loads((BASE / ('free-erp-complete-schema-oct8-' + phase_prefix + '-after.json')).read_text())
install = json.loads((BASE / 'free-erp-install-migration-status.json').read_text())
declared = json.loads((BASE / 'free-erp-runtime-tables-oct8-inputs.json').read_text())
checks = []


def check(name, actual, expected):
    row = {'name': name, 'pass': actual == expected}
    if actual != expected:
        row.update(actual=actual, expected=expected)
    checks.append(row)


check('same full source window', after['source_sha256'], before['source_sha256'])
for table, hashes in before['protected_table_hashes'].items():
    check('all old row values retained:' + table, after['protected_table_hashes'].get(table), hashes)
check('full 577 registered cases passed', (int(summary[2]), int(summary[3]), int(summary[4])), (577, 0, 0))
check('actual fresh 320-file install', (install['status'], install['applied_count'], install['previously_applied']), ('PASS', 320, 0))
fixture = (ROOT / 'crates/erp-api/tests/free_erp_acceptance/login_sessions.rs').read_text(encoding='utf-8')
for table in declared['tables']:
    check('no fixture CREATE for runtime dependency:' + table, 'create table if not exists ' + table in fixture, False)
shipments = []
for line in log.splitlines():
    if line.startswith('QA fulfillment_http ') and ' status=200 body=' in line and '/allocate' in line:
        response = json.loads(line.split(' body=', 1)[1])
        shipments.extend(response.get('shipments', []))
helpers = runpy.run_path(str(BASE / 'free-erp-product-web-fixture.py'))
ids = sorted({row['shipment_id'] for row in shipments})
if not ids:
    raise SystemExit('no recorded real successful allocation calls')
scope = ','.join(helpers['literal'](value) for value in ids)
actual = json.loads(helpers['query']("SELECT json_agg(json_build_object('shipment_id',shipment_id,'created_at',created_at)) FROM shipments WHERE shipment_id IN (" + scope + ');'))
stored = {row['shipment_id']: row for row in actual}
for row in shipments:
    check('actual returned stored creation:' + row['shipment_id'],
          datetime.fromisoformat(row['created_at'].replace('Z', '+00:00')),
          datetime.fromisoformat(stored[row['shipment_id']]['created_at'].replace('Z', '+00:00')))
failures = [row for row in checks if not row['pass']]
# Serialize timestamps only for an actual failed comparison.
for failure in failures:
    for key in ['actual', 'expected']:
        if isinstance(failure.get(key), datetime):
            failure[key] = failure[key].isoformat()
report = {'status': 'PASS' if not failures else 'PARTIAL', 'checks': len(checks), 'failures': failures,
          'tests': {'passed': int(summary[2]), 'failed': int(summary[3]), 'ignored': int(summary[4]), 'seconds': float(summary[5])},
          'protected_tenants': len(before['protected_tenants']), 'protected_tables': len(before['protected_table_hashes']),
          'protected_rows': sum(map(len, before['protected_table_hashes'].values())), 'source_inputs': len(before['source_sha256']),
          'recorded_allocation_responses': len(shipments), 'distinct_actual_shipments': len(ids), 'assertions': checks,
          'scope': 'actual full debug registered backend target and empty SQL install; not native/Web UI, Worker, Release or whole ERP'}
(BASE / (run_prefix + '-independent.json')).write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(report['status'], report['checks'], 'checks;', report['tests'])
if failures:
    raise SystemExit(1)
