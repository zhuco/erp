"""Independently check both directions of committed shipment/event/record linkage."""
from collections import Counter
import hashlib
import json
from pathlib import Path

base = Path(__file__).resolve().parent
proof = json.loads((base / 'free-erp-fulfillment-atomic-final-facts-v3.json').read_text(encoding='utf8'))
owned = set(proof['owned_fixtures'])
errors = []
counts = Counter()
checks = 0
with (base / 'free-erp-fulfillment-atomic-final-after.jsonl').open(encoding='utf-8-sig') as stream:
    for line in stream:
        row = json.loads(line)
        if row.get('tenant_id') not in owned:
            continue
        facts = row['facts']
        for shipment in facts['shipments']:
            events = [event for event in facts['outbox_events'] if event['event_type'] == 'SHIPMENT_SHIPPED'
                      and event['payload']['shipment_id'] == shipment['shipment_id']]
            expected = 1 if shipment['status'] == 'SHIPPED' else 0
            checks += 1
            if len(events) != expected:
                errors.append(f"{row['tenant_id']}:{shipment['shipment_id']}: success event cardinality")
            counts['committed_shipped_headers'] += expected
            counts['shipment_shipped_events'] += len(events)
            counts['unshipped_headers'] += not expected
            if expected:
                for table, action, key in [('operation_logs', 'OMS.SHIPMENT_SHIPPED', 'ref_id'),
                                           ('audit_logs', 'OMS.SHIPMENT.SHIPPED', 'entity_id')]:
                    records = [record for record in facts[table] if record['action'] == action
                               and record[key] == shipment['shipment_id']]
                    checks += 1
                    if len(records) != 1:
                        errors.append(f"{row['tenant_id']}:{shipment['shipment_id']}: mandatory {table} cardinality")
                    counts[table] += len(records)
result = {'status': 'FAIL' if errors else 'PASS', 'checks': checks, 'counts': dict(counts), 'errors': errors,
          'readback_sha256': proof['artifacts']['free-erp-fulfillment-atomic-final-after.jsonl']['sha256'],
          'validator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
destination = base / 'free-erp-fulfillment-atomic-final-linkage.json'
if destination.exists():
    raise SystemExit('refusing linkage evidence overwrite')
destination.write_text(json.dumps(result, indent=2) + '\n', encoding='utf8')
print(json.dumps(result))
raise SystemExit(1 if errors else 0)
