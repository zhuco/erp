"""Independently verify cost input wiring, original rows and stored cost effects."""
import hashlib
import json
import re
import runpy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
manifest = json.loads((BASE / 'free-erp-cost-input-oct9-calls.json').read_text())
checks = []


def check(name, actual, expected):
    row = {'name': name, 'pass': actual == expected}
    if not row['pass']:
        row.update(actual=actual, expected=expected)
    checks.append(row)


def tokens(text):
    # Preserve strings verbatim, ignoring only formatting and comments.
    found = re.findall(r'r#+".*?"#+|"(?:\\.|[^"\\])*"|\w+|[^\s]',
                       re.sub(r'//[^\r\n]*', '', text), re.S)
    return [value for index, value in enumerate(found)
            if not (value == ',' and index + 1 < len(found) and found[index + 1] in [')', '}', ']'])]


def balanced(text, start, closing):
    stack = [closing]
    i = start + 1
    while stack:
        if text[i] == '"':
            i += 1
            while text[i] != '"':
                i += 2 if text[i] == '\\' else 1
        elif text[i] in '({[':
            stack.append({'(': ')', '{': '}', '[': ']'}[text[i]])
        elif text[i] == stack[-1]:
            stack.pop()
        i += 1
    return i - 1


def fields(body):
    parts = []
    start = i = 0
    while i < len(body):
        if body[i] == '"':
            i += 1
            while body[i] != '"':
                i += 2 if body[i] == '\\' else 1
        elif body[i] in '({[':
            i = balanced(body, i, {'(': ')', '{': '}', '[': ']'}[body[i]])
        elif body[i] == ',':
            parts.append(body[start:i].strip())
            start = i + 1
        i += 1
    parts.append(body[start:].strip())
    result = {}
    for part in filter(None, parts):
        match = re.fullmatch(r'([a-z_][a-z0-9_]*)\s*(?::(.*))?', part, re.S)
        if not match or match[1] in result:
            raise ValueError('invalid or duplicate named field')
        result[match[1]] = match[2].strip() if match[2] is not None else match[1]
    return result


for file in manifest['files']:
    source = (ROOT / file['file']).read_text(encoding='utf-8')
    methods = {row['method'] for row in file['calls']}
    original_file = BASE / 'cost-input-source-before' / (file['file'].replace('/', '-') + '.txt')
    check('original caller snapshot:' + file['file'],
          hashlib.sha256(original_file.read_bytes()).hexdigest(), file['before_sha256'])

    def mask_calls(text):
        matches = list(re.finditer(r'\.(' + '|'.join(sorted(methods)) + r')\s*\(', text))
        for match in reversed(matches):
            start = text.index('(', match.start())
            end = balanced(text, start, ')')
            text = text[:start + 1] + 'INPUT' + text[end:]
        return tokens(text)

    check('outside call arguments unchanged:' + file['file'],
          mask_calls(source), mask_calls(original_file.read_text(encoding='utf-8')))
    calls = list(re.finditer(r'\.(' + '|'.join(sorted(methods)) + r')\s*\(', source))
    check('call count:' + file['file'], len(calls), len(file['calls']))
    for index, (match, original) in enumerate(zip(calls, file['calls'])):
        start = source.index('(', match.start())
        end = balanced(source, start, ')')
        constructor = source[start + 1:end].strip().rstrip(',').strip()
        brace = constructor.index('{')
        close = balanced(constructor, brace, '}')
        actual = fields(constructor[brace + 1:close])
        expected = dict(zip(original['named_fields'], original['original_expressions']))
        check(f"method:{file['file']}:{index}", match[1], original['method'])
        check(f"original arguments:{file['file']}:{index}",
              {key: tokens(value) for key, value in actual.items()},
              {key: tokens(value) for key, value in expected.items()})

log = (BASE / 'free-erp-cost-input-oct9-tests.log').read_text(encoding='utf-8-sig')
summary = re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)
if not summary:
    raise SystemExit('no terminal successful cost test result')
check('nine actual cost cases', tuple(map(int, summary.groups())), (9, 0, 0))
before = json.loads((BASE / 'free-erp-complete-schema-oct8-cost-input-before.json').read_text())
after = json.loads((BASE / 'free-erp-complete-schema-oct8-cost-input-after.json').read_text())
check('same source window', after['source_sha256'], before['source_sha256'])
for table, hashes in before['protected_table_hashes'].items():
    check('old rows unchanged:' + table, after['protected_table_hashes'].get(table), hashes)

tenants = sorted(set(re.findall(r'QA item cost tenant=([a-f0-9-]{36})', log)))
check('nine fresh cost tenants', len(tenants), 9)
helpers = runpy.run_path(str(BASE / 'free-erp-product-web-fixture.py'))
scope = ','.join(helpers['literal'](tenant) for tenant in tenants)
rows = json.loads(helpers['query']("SELECT coalesce(json_agg(json_build_object('item',item_code,'kind',kind,'qty',qty_micros,'amount',amount_cent,'source',source_ref) ORDER BY item_code,entry_sequence),'[]'::json) FROM shared_item_costs WHERE tenant_id IN (" + scope + ');'))
for item, kind, expected in [
    ('QA-SUBCENT', 'ISSUE', [(3000000, 2)]),
    ('QA-FRACTION', 'RETURN', [(1, 1), (1, 0), (1, 1)]),
    ('QA-CONCURRENT', 'RETURN', [(400000, 480)]),
    ('QA-REPLAY', 'RECEIPT', [(1000000, 1200)]),
    ('QA-PAYLOAD', 'RECEIPT', [(1000000, 100)]),
    ('QA-MAX', 'RECEIPT', [(9223372036854775807, 0)]),
]:
    check('stored quantity/cents:' + item + ':' + kind,
          [(row['qty'], row['amount']) for row in rows if row['item'] == item and row['kind'] == kind], expected)
check('all returns cite original issue',
      all(row['source'] == 'issue' for row in rows if row['kind'] == 'RETURN'), True)
failures = [row for row in checks if not row['pass']]
report = {'status': 'PASS' if not failures else 'PARTIAL', 'checks': len(checks),
          'failures': failures, 'assertions': checks, 'call_sites': sum(len(row['calls']) for row in manifest['files']),
          'protected_tenants': len(before['protected_tenants']),
          'protected_tables': len(before['protected_table_hashes']),
          'protected_rows': sum(map(len, before['protected_table_hashes'].values())),
          'source_inputs': len(before['source_sha256']), 'stored_cost_rows': len(rows),
          'scope': 'nine cost cases and input mapping; not full ERP, UI, Worker or Release'}
destination = BASE / 'free-erp-cost-input-oct9-independent.json'
if destination.exists():
    raise SystemExit('refusing to overwrite evidence')
destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(report['status'], len(checks), 'checks;', report['call_sites'], 'call sites')
if failures:
    raise SystemExit(1)
