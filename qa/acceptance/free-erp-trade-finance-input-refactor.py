"""One registered, byte-preserving named-input migration; never builds or touches DB."""
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
MODULE = ROOT / 'crates/erp-repo/src/shared_contract.rs'
METHODS = {
    'save_purchase_revision': ('trade_document_input', 'TradePurchaseRevisionInput'),
    'save_sales_revision': ('trade_document_input', 'TradeSalesRevisionInput'),
    'plan_sales_delivery': ('trade_document_input', 'SalesDeliveryPlanInput'),
    'advance_sales_delivery': ('trade_document_input', 'SalesDeliveryAdvanceInput'),
    'save_quote_revision': ('trade_document_input', 'TradeQuoteRevisionInput'),
    'convert_quote': ('trade_document_input', 'TradeQuoteConversionInput'),
    'save_trade_purchase': ('trade_document_input', 'TradePurchaseInput'),
    'save_trade_sale': ('trade_document_input', 'TradeSaleInput'),
    'save_shared_partner': ('trade_document_input', 'SharedPartnerInput'),
    'save_partner_contact': ('trade_document_input', 'PartnerContactInput'),
    'save_scaled_trade_sale': ('trade_document_input', 'ScaledTradeSaleInput'),
    'confirm_document_balance': ('finance_input', 'DocumentBalanceInput'),
    'save_advance_balance': ('finance_input', 'AdvanceBalanceInput'),
    'save_opening_balance': ('finance_input', 'OpeningBalanceInput'),
    'save_partner_balance': ('finance_input', 'PartnerBalanceInput'),
    'save_cited_invoice': ('finance_input', 'CitedInvoiceInput'),
    'save_cited_late_invoice': ('finance_input', 'CitedInvoiceInput'),
    'save_invoice_record': ('finance_input', 'InvoiceRecordInput'),
    'save_cost_fact': ('finance_input', 'CostFactInput'),
    'save_posted_cost_reconciliation': ('finance_input', 'PostedCostReconciliationInput'),
    'save_cost_reconciliation': ('finance_input', 'CostReconciliationInput'),
    'save_period_adjustment': ('finance_input', 'PeriodAdjustmentInput'),
    'save_cited_expense_allocation': ('finance_input', 'CitedExpenseAllocationInput'),
    'save_expense_allocation': ('finance_input', 'ExpenseAllocationInput'),
    'save_return_cash_offset': ('finance_input', 'ReturnCashOffsetInput'),
    'save_cash_entry': ('finance_input', 'CashEntryInput'),
}


def skip_string(source, i):
    if source[i] != '"':
        raise ValueError('string start expected')
    i += 1
    while source[i] != '"':
        i += 2 if source[i] == '\\' else 1
    return i + 1


def balanced(source, start):
    pairs = {'(': ')', '{': '}', '[': ']'}
    stack = [pairs[source[start]]]
    i = start + 1
    while stack:
        raw = re.match(r'r(#+)?"', source[i:]) if source[i] == 'r' else None
        if raw:
            end = '"' + (raw[1] or '')
            i = source.index(end, i + len(raw[0])) + len(end)
            continue
        if source.startswith('//', i):
            i = source.index('\n', i)
            continue
        if source.startswith('/*', i):
            i = source.index('*/', i + 2) + 2
            continue
        if source[i] == '"':
            i = skip_string(source, i)
            continue
        if source[i] in pairs:
            stack.append(pairs[source[i]])
        elif source[i] == stack[-1]:
            stack.pop()
        i += 1
    return i - 1


def split_arguments(source):
    result = []
    start = i = 0
    while i < len(source):
        if source[i] == '"':
            i = skip_string(source, i)
            continue
        if source[i] in '({[':
            i = balanced(source, i) + 1
            continue
        if source[i] == ',':
            result.append(source[start:i].strip())
            start = i + 1
        i += 1
    result.append(source[start:].strip())
    return list(filter(None, result))


def sha(data):
    return hashlib.sha256(data).hexdigest()


destination = BASE / 'free-erp-trade-finance-input-oct9-calls.json'
backup_dir = BASE / 'trade-finance-input-source-before'
if destination.exists() or backup_dir.exists():
    raise SystemExit('registered migration already has evidence; do not rerun')
original = MODULE.read_bytes()
source = original.decode('utf-8')
newline = '\r\n' if b'\r\n' in original else '\n'
definitions = {}
edits = []
inputs = {}
for method, (module, input_name) in METHODS.items():
    pattern = r'    pub async fn ' + method + r'\('
    matches = list(re.finditer(pattern, source))
    if len(matches) != 1:
        raise ValueError('unique public method required: ' + method)
    match = matches[0]
    start = source.index('(', match.start())
    end = balanced(source, start)
    # Each registered original signature has one field per line; generic type commas stay intact.
    parameters = []
    for line in source[start + 1:end].splitlines():
        line = line.strip().rstrip(',')
        if not line or line == '&self':
            continue
        name, type_name = line.split(':', 1)
        if not re.fullmatch(r'[a-z_][a-z0-9_]*', name):
            raise ValueError('unsupported parameter: ' + line)
        parameters.append((name, type_name.strip()))
    if len(parameters) < 7:
        raise ValueError('unregistered short signature: ' + method)
    definitions[method] = {'input': input_name, 'module': module, 'fields': parameters}
    input_fields = [(name, re.sub(r'&(?!\')', "&'a ", type_name)) for name, type_name in parameters]
    # Root imports are explicit; leaf inputs use the canonical domain type.
    input_fields = [(name, 'erp_domain::trade::PartnerBalanceKind' if type_name == 'PartnerBalanceKind' else type_name)
                    for name, type_name in input_fields]
    key = (module, input_name)
    if key in inputs and inputs[key] != input_fields:
        raise ValueError('shared input shape mismatch')
    inputs[key] = input_fields
    signature = newline + '        &self,' + newline + f"        input: {input_name}<'_>," + newline + '    '
    edits.append((start + 1, end, signature))
    brace = source.index('{', end)
    fields = ','.join(name for name, _ in parameters)
    destructure = newline + f'        let {input_name} {{ {fields} }} = input;'
    edits.append((brace + 1, brace + 1, destructure))

# Prepare every caller edit before writing anything, and retain exact old expressions.
call_records = []
caller_edits = {}
pattern = r'\.(' + '|'.join(sorted(METHODS)) + r')\s*\('
for file in (ROOT / 'crates').rglob('*.rs'):
    data = file.read_bytes()
    text = data.decode('utf-8')
    changes = []
    calls = []
    for match in re.finditer(pattern, text):
        method = match[1]
        start = text.index('(', match.start())
        end = balanced(text, start)
        args = split_arguments(text[start + 1:end])
        field_names = [name for name, _ in definitions[method]['fields']]
        if len(args) != len(field_names):
            raise ValueError(f'{file}:{method} expected {len(field_names)} arguments, got {len(args)}')
        input_name = definitions[method]['input']
        prefix = '' if file == MODULE else 'erp_repo::shared_contract::'
        constructor = prefix + input_name + ' { ' + ', '.join(
            name if name == expr else name + ': ' + expr for name, expr in zip(field_names, args)
        ) + ' }'
        changes.append((start + 1, end, constructor))
        calls.append({'method': method, 'input': input_name,
                      'named_fields': field_names, 'original_expressions': args})
    if changes:
        if file == MODULE:
            edits.extend(changes)
        else:
            caller_edits[file] = (data, text, changes)
        call_records.append({'file': file.relative_to(ROOT).as_posix(), 'calls': calls,
                             'before_sha256': sha(data)})
if sum(len(row['calls']) for row in call_records) != 263:
    raise ValueError('registered call inventory changed')
declarations = []
leaf_sources = {}
for module in sorted({module for module, _ in inputs}):
    names = [name for owner, name in inputs if owner == module]
    declarations.append('mod ' + module + ';' + newline + 'pub use ' + module + '::{ ' + ', '.join(names) + ' };' + newline)
    code = "//! Internal Rust commands; HTTP DTOs and receipt identities retain their contracts.\nuse erp_types::TenantId;\nuse uuid::Uuid;\n\n"
    for name in names:
        code += "pub struct " + name + "<'a> {\n"
        code += ''.join('    pub ' + field + ': ' + type_name + ',\n' for field, type_name in inputs[(module, name)])
        code += '}\n\n'
    leaf = MODULE.parent / 'shared_contract' / (module + '.rs')
    if leaf.exists():
        raise ValueError('input module already exists: ' + str(leaf))
    leaf_sources[leaf] = code.replace('\n', newline).encode('utf-8')
anchor = 'mod item_cost_input;' + newline
if source.count(anchor) != 1:
    raise ValueError('unique module declaration anchor required')
at = source.index(anchor) + len(anchor)
edits.append((at, at, ''.join(declarations)))

backup_dir.mkdir()
all_changed = {MODULE: original, **{file: data for file, (data, _, _) in caller_edits.items()}}
for file, data in all_changed.items():
    (backup_dir / (file.relative_to(ROOT).as_posix().replace('/', '-') + '.txt')).write_bytes(data)
for start, end, replacement in sorted(edits, reverse=True):
    source = source[:start] + replacement + source[end:]
MODULE.write_bytes(source.encode('utf-8'))
for file, (data, text, changes) in caller_edits.items():
    for start, end, replacement in sorted(changes, reverse=True):
        text = text[:start] + replacement + text[end:]
    file.write_bytes(text.encode('utf-8'))
for file, data in leaf_sources.items():
    file.write_bytes(data)
for row in call_records:
    row['after_sha256'] = sha((ROOT / row['file']).read_bytes())
report = {'scope': '26 named trade and finance methods, original expressions preserved; no SQL/HTTP changes',
          'methods': definitions, 'root_before_sha256': sha(original),
          'files': call_records, 'new_inputs': [p.relative_to(ROOT).as_posix() for p in leaf_sources]}
destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print('26 methods, 25 input structs, 263 named calls prepared with original source backups')
