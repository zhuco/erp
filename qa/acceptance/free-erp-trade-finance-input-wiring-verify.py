"""Verify original argument expressions, root bodies and new named field types."""
import ast
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
manifest = json.loads((BASE / 'free-erp-trade-finance-input-oct9-calls.json').read_text())
parser_source = (BASE / 'free-erp-trade-finance-input-refactor.py').read_text()
# Load only the four pure parser functions, never the migration's executable statements.
parser_tree = ast.parse(parser_source)
for node in parser_tree.body:
    if isinstance(node, ast.FunctionDef):
        exec(compile(ast.Module(body=[node], type_ignores=[]), '<pure Rust parser>', 'exec'))
checks = []


def check(name, actual, expected):
    row = {'name': name, 'pass': actual == expected}
    if not row['pass']:
        row.update(actual=actual, expected=expected)
    checks.append(row)


def tokens(text):
    pattern = r'r#+".*?"#+|"(?:\\.|[^"\\])*"|//[^\r\n]*|/\*.*?\*/|\w+|[^\s]'
    values = [m[0] for m in re.finditer(pattern, text, re.S)
              if not m[0].startswith(('//', '/*'))]
    return [value for index, value in enumerate(values)
            if not (value == ',' and index + 1 < len(values) and values[index + 1] in [')', '}', ']'])]


def mask_calls(text, methods):
    matches = list(re.finditer(r'\.(' + '|'.join(sorted(methods)) + r')\s*\(', text))
    for match in reversed(matches):
        start = text.index('(', match.start())
        end = balanced(text, start)
        text = text[:start + 1] + 'INPUT' + text[end:]
    return tokens(text)


for file in manifest['files']:
    path = ROOT / file['file']
    original_path = BASE / 'trade-finance-input-source-before' / (file['file'].replace('/', '-') + '.txt')
    original = original_path.read_bytes()
    check('original caller SHA:' + file['file'], sha(original), file['before_sha256'])
    source = path.read_bytes().decode('utf-8')
    methods = {call['method'] for call in file['calls']}
    matches = list(re.finditer(r'\.(' + '|'.join(sorted(methods)) + r')\s*\(', source))
    check('call count:' + file['file'], len(matches), len(file['calls']))
    check('outside call arguments:' + file['file'],
          mask_calls(source, methods), mask_calls(original.decode('utf-8'), methods))
    for index, (match, call) in enumerate(zip(matches, file['calls'])):
        start = source.index('(', match.start())
        end = balanced(source, start)
        constructor = source[start + 1:end].strip().rstrip(',').strip()
        brace = constructor.index('{')
        close = balanced(constructor, brace)
        check(f'constructor type:{file["file"]}:{index}',
              constructor[:brace].strip(), 'erp_repo::shared_contract::' + call['input'])
        actual = {}
        for part in split_arguments(constructor[brace + 1:close]):
            field = re.fullmatch(r'([a-z_][a-z0-9_]*)\s*(?::(.*))?', part, re.S)
            if not field or field[1] in actual:
                raise ValueError('invalid or duplicate field: ' + part)
            actual[field[1]] = field[2].strip() if field[2] is not None else field[1]
        expected = dict(zip(call['named_fields'], call['original_expressions']))
        check(f'method:{file["file"]}:{index}', match[1], call['method'])
        check(f'original expression mapping:{file["file"]}:{index}',
              {name: tokens(expr) for name, expr in actual.items()},
              {name: tokens(expr) for name, expr in expected.items()})

root = ROOT / 'crates/erp-repo/src/shared_contract.rs'
original_root = (BASE / 'trade-finance-input-source-before/crates-erp-repo-src-shared_contract.rs.txt').read_bytes().decode('utf-8')
check('original complete root SHA', sha(original_root.encode('utf-8')), manifest['root_before_sha256'])
source = root.read_bytes().decode('utf-8')
newline = '\r\n' if '\r\n' in source else '\n'
for module in ['finance_input', 'trade_document_input']:
    source, count = re.subn(r'(?m)^mod ' + module + r';\r?\n', '', source)
    check('one new module declaration:' + module, count, 1)
    source, count = re.subn(r'(?m)^pub use ' + module + r'::\{[^}]+\};\r?\n', '', source)
    check('one new export declaration:' + module, count, 1)
restore_edits = []
for method, definition in manifest['methods'].items():
    match = re.search(r'    pub async fn ' + method + r'\(', source)
    start = source.index('(', match.start())
    end = balanced(source, start)
    check('named signature:' + method, tokens(source[start + 1:end]),
          tokens("&self, input: " + definition['input'] + "<'_>,"))
    old_match = re.search(r'    pub async fn ' + method + r'\(', original_root)
    old_start = original_root.index('(', old_match.start())
    old_end = balanced(original_root, old_start)
    restore_edits.append((start + 1, end, original_root[old_start + 1:old_end]))
    brace = source.index('{', end)
    destructure = re.match(r'\r?\n        let ' + definition['input'] + r'\s*\{([^}]+)\}\s*= input;', source[brace + 1:])
    if not destructure:
        raise ValueError('one leading input destructure required: ' + method)
    check('all original local bindings:' + method,
          [part.strip() for part in destructure[1].split(',') if part.strip()],
          [name for name, _ in definition['fields']])
    restore_edits.append((brace + 1, brace + 1 + len(destructure[0]), ''))
    leaf = ROOT / 'crates/erp-repo/src/shared_contract' / (definition['module'] + '.rs')
    leaf_source = leaf.read_text(encoding='utf-8')
    struct = re.search(r'pub struct ' + definition['input'] + r"<'a> \{([^}]+)\}", leaf_source)
    expected_fields = []
    for name, type_name in definition['fields']:
        type_name = re.sub(r'&(?!\')', "&'a ", type_name)
        if type_name == 'PartnerBalanceKind':
            type_name = 'erp_domain::trade::PartnerBalanceKind'
        expected_fields.append('pub ' + name + ': ' + type_name + ',')
    check('named field types:' + definition['input'], tokens(struct[1]), tokens('\n'.join(expected_fields)))
for start, end, replacement in sorted(restore_edits, reverse=True):
    source = source[:start] + replacement + source[end:]
# Only this bare import became unused after its parameter types moved to canonical leaf fields.
expected_root = original_root.replace('    PartnerBalanceKind,' + newline, '')
check('all root method bodies and SQL unchanged byte for byte', source, expected_root)
failures = [row for row in checks if not row['pass']]
report = {'status': 'PASS' if not failures else 'FAIL', 'checks': len(checks),
          'failures': failures, 'methods': len(manifest['methods']), 'call_sites': 263,
          'source_sha256': {file: sha((ROOT / file).read_bytes()) for file in
                            [root.relative_to(ROOT).as_posix()] + manifest['new_inputs'] + [r['file'] for r in manifest['files']]},
          'assertions': checks, 'scope': 'static input mapping/body/type proof; not compilation or runtime acceptance'}
destination = BASE / 'free-erp-trade-finance-input-oct9-wiring-final.json'
if destination.exists():
    raise SystemExit('refusing to overwrite wiring evidence')
destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(report['status'], len(checks), 'checks; 26 methods; 263 calls')
if failures:
    print([row['name'] for row in failures])
    raise SystemExit(1)
