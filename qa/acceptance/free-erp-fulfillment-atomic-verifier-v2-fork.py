"""Extend readback with exact persisted event time and original version-guard evidence."""
from pathlib import Path

base = Path(__file__).resolve().parent
source = (base / 'free-erp-fulfillment-atomic-verify.py').read_text(encoding='utf8')
source = source.replace("destination = BASE / f'free-erp-fulfillment-{args.stage}-independent.json'",
                        "destination = BASE / f'free-erp-fulfillment-{args.stage}-facts-v2.json'")
source = source.replace("    else:\n        check(summary['status'] == 'ok'", "    elif args.stage == 'atomic-time-original':\n        check(summary['passed'] == 0 and summary['failed'] == 2 and summary['ignored'] == 0, 'both original persisted-time/version-guard failures reproduced')\n    else:\n        check(summary['status'] == 'ok'", 1)
source = source.replace("and source['qty'] == cogs['qty'] and shipment['status'] == 'SHIPPED'",
                        "and source['qty'] == cogs['qty'] and (shipment['status'] == 'SHIPPED' or (args.stage == 'atomic-time-original' and shipment['status_version'] == 100))")
source = source.replace("original_failures_expected=args.stage == 'atomic-original'",
                        "original_failures_expected=args.stage in ('atomic-original', 'atomic-time-original')")
extra = r'''
from datetime import datetime
import calendar

def exact_timestamp(raw):
    match = re.fullmatch(r'(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?(Z|[+-]\d\d:\d\d)', raw)
    if match is None:
        raise ValueError('invalid captured persisted time')
    whole, fraction, zone = match.groups()
    parsed = datetime.fromisoformat(whole + ('+00:00' if zone == 'Z' else zone))
    return calendar.timegm(parsed.utctimetuple()), int((fraction or '').ljust(9, '0'))

temporal_differences = []
false_transitions = []
for tenant, captured in owned.items():
    facts = captured['facts']
    for event in facts['outbox_events']:
        if event['event_type'] != 'SHIPMENT_SHIPPED':
            continue
        payload = event['payload']
        matches = [row for row in facts['shipments'] if row['shipment_id'] == payload['shipment_id']]
        check(len(matches) == 1, f'{tenant}: shipped event has exactly one persisted header')
        if len(matches) != 1:
            continue
        shipment = matches[0]
        check(payload['tenant_id'] == tenant and payload['order_id'] == shipment['order_id']
              and payload['warehouse_id'] == shipment['warehouse_id']
              and payload['shop_id'] == shipment['shop_id']
              and payload['carrier_code'] == shipment['carrier_code']
              and payload['tracking_no'] == shipment['tracking_no'], f'{tenant}: event cites complete stored shipment identity')
        if shipment['status'] != 'SHIPPED' or shipment['shipped_at'] is None:
            false_transitions.append(dict(tenant_id=tenant, shipment_id=shipment['shipment_id'], status=shipment['status'], version=shipment['status_version']))
            if args.stage != 'atomic-time-original':
                check(False, f'{tenant}: no success event for an uncommitted shipment transition')
        elif exact_timestamp(payload['shipped_at']) != exact_timestamp(shipment['shipped_at']):
            temporal_differences.append(dict(tenant_id=tenant, shipment_id=shipment['shipment_id'], stored=shipment['shipped_at'], event=payload['shipped_at']))
            if args.stage == 'atomic-final':
                check(False, f'{tenant}: event uses exact persisted time without nanosecond reconstruction')
        if args.stage != 'atomic-time-original':
            operations = [row for row in facts['operation_logs'] if row['action'] == 'OMS.SHIPMENT_SHIPPED' and row['ref_id'] == shipment['shipment_id']]
            audits = [row for row in facts['audit_logs'] if row['action'] == 'OMS.SHIPMENT.SHIPPED' and row['entity_id'] == shipment['shipment_id']]
            check(len(operations) == len(audits) == 1, f'{tenant}: one mandatory operation and audit per committed shipment')
            if len(operations) == len(audits) == 1:
                check(operations[0]['actor_type'] == 'USER' and operations[0]['actor_user_id'] == audits[0]['user_id'] == fixtures[tenant]['actor'], f'{tenant}: both records cite authenticated fixture actor')
                check(operations[0]['metadata']['shipment_id'] == audits[0]['after_json']['shipment_id'] == shipment['shipment_id'], f'{tenant}: both records cite actual shipment')

faults = re.findall(r'QA fulfillment mandatory_failure tenant=(\S+) action=(\S+) table=(\S+) schema=(\S+) unchanged=(\w+)', log)
races = re.findall(r'QA fulfillment authority_race tenant=(\S+) case=(\S+) (.+)', log)
if args.stage in ('atomic-full', 'atomic-final'):
    check(len(faults) == 7 and all(row[4] == 'true' for row in faults), 'all seven actual INSERT failures independently retain complete before facts')
    check(len(races) == 2 and all('unchanged=true' in row[2] for row in races), 'both real PostgreSQL authority-lock competitions drained and proved')
    for tenant, action, table, schema, unchanged in faults:
        responses = [row for row in http if row['tenant_id'] == tenant and row['status'] == 500]
        check(len(responses) == 1 and 'check option' in str(responses[0]['body']) and table in str(responses[0]['body']), f'{tenant}: actual {action}/{table} fault and successful recovery')
if args.stage == 'atomic-time-original':
    check(len(temporal_differences) == 1, 'original nanosecond event differs from actual persisted microsecond time')
    check(len(false_transitions) == 1 and false_transitions[0]['version'] == 100, 'original zero-row version-guard transition still emits success')
'''
anchor = '\nartifacts = {}'
if source.count(anchor) != 1:
    raise SystemExit('nonunique verifier extension boundary')
source = source.replace(anchor, '\n' + extra + anchor)
source = source.replace("actual_http=http, artifacts=artifacts, errors=errors,",
                        "actual_http=http, mandatory_faults=faults, authority_races=races, temporal_differences=temporal_differences, false_transitions=false_transitions, artifacts=artifacts, errors=errors,")
destination = base / 'free-erp-fulfillment-atomic-verify-v2.py'
if destination.exists():
    raise SystemExit('refusing to replace v2 verifier')
destination.write_text(source, encoding='utf8')
print('created exact-time and version-guard independent verifier')
