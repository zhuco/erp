"""Independent readback for the actual cost-command fence regression."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
PREFIX = 'free-erp-inventory-legacy-reservations'
destination = BASE / f'{PREFIX}-cost-fence-independent.json'
if destination.exists():
    raise SystemExit('refusing to replace historical cost evidence')


def text(path):
    data = path.read_bytes()
    return data.decode('utf-16' if data.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig')


checks, errors = 0, []


def check(condition, message):
    global checks
    checks += 1
    if not condition:
        errors.append(message)


facts = json.loads(text(BASE / f'{PREFIX}-cost-fence-readback.json'))
check(tuple(facts[key] for key in ['database', 'role', 'host', 'port']) ==
      ('erp_shared_contract_test', 'erp_grid_test', '127.0.0.1', 15438), 'protected database identity')
check(facts['tenant']['name'] == 'QA-COST-RETURN', 'owned cost fixture')
log = text(BASE / f'{PREFIX}-cost-fence.log')
fixtures = re.findall(r'QA item cost tenant=([a-f0-9-]+)', log)
check(fixtures == [facts['tenant']['tenant_id']], 'actual named single fixture')
check('test item_cost::cost_history_orders_effects_after_fences_not_transaction_start ... ok' in log
      and 'test result: ok. 1 passed; 0 failed; 0 ignored;' in log, 'actual registered PostgreSQL waiter/history scenario')
entries = facts['entries']
check(len(entries) == 2 and [entry['kind'] for entry in entries] == ['RECEIPT', 'ISSUE'], 'two immutable effects in sequence order')
quantity, value = 0, 0
for entry in entries:
    check(entry['item_code'] == 'QA-ORDER' and entry['qty_micros'] == 1_000_000
          and entry['amount_cent'] == 100 and not entry['estimated'], 'exact millionth quantity and original cost')
    direction = 1 if entry['kind'] == 'RECEIPT' else -1
    quantity += direction * entry['qty_micros']
    value += direction * entry['amount_cent']
    check(quantity >= 0 and value >= 0, 'independent sequence position is nonnegative')
check((quantity, value) == (0, 0), 'original issue completely consumes the later-admitted receipt')
check(entries[0]['entry_sequence'] < entries[1]['entry_sequence']
      and datetime.fromisoformat(entries[1]['created_at']) < datetime.fromisoformat(entries[0]['created_at']),
      'older issue transaction clock must not invert later physical effect sequence')
receipts = {row['request_id']: row for row in facts['receipts']}
check(set(receipts) == {'seed', 'delayed-issue'}, 'two actual original identities')
for request, entry in zip(['seed', 'delayed-issue'], entries, strict=True):
    receipt = receipts[request]
    check(receipt['status'] == 'SUCCEEDED' and receipt['payload_hash'] == request
          and receipt['effect_id'] == str(entry['entry_id']), 'receipt binds actual original cost effect')
before = json.loads((BASE / 'free-erp-inventory-consumers-legacy-reservations-cost-fence-before-source.json').read_text(encoding='utf8'))['files']
after = json.loads((BASE / 'free-erp-inventory-consumers-legacy-reservations-cost-fence-after-source.json').read_text(encoding='utf8'))['files']
cost_path = 'crates/erp-repo/src/shared_contract/item_cost.rs'
check(before[cost_path] == after[cost_path], 'cost leaf stable during actual regression')
source = (ROOT / cost_path).read_text(encoding='utf8')
check(source.count('lock_cost_command(&mut tx, tenant_id, request_id).await?;') == 4, 'all four cost writer/fence call sites')
report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'FAIL' if errors else 'PASS',
          'checks': checks, 'errors': errors, 'tenant_id': facts['tenant']['tenant_id'],
          'independent_position': {'quantity_micros': quantity, 'value_cent': value},
          'source_sha256_at_test': before[cost_path],
          'readback_sha256': hashlib.sha256((BASE / f'{PREFIX}-cost-fence-readback.json').read_bytes()).hexdigest(),
          'limits': ['One protected QA fixture and debug registered test; not production cost coverage or Release performance.']}
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps(report, ensure_ascii=False))
raise SystemExit(1 if errors else 0)
