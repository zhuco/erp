"""Verify guarded schema prerequisites and complete old facts, one tenant at a time."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
destination = BASE / 'free-erp-fulfillment-schema-independent.json'
if destination.exists():
    raise SystemExit('refusing to replace schema verification')
prepared = json.loads((BASE / 'free-erp-fulfillment-schema-inputs.json').read_text(encoding='utf8'))
errors = []
checks = 0


def canonical(rows):
    return sorted(json.dumps(row, sort_keys=True, separators=(',', ':')) for row in rows)


def record(stream):
    for line in stream:
        if line.strip():
            return json.loads(line)
    return None


old_count = 0
unchanged = 0
additional = []
changed = []
with (BASE / 'free-erp-fulfillment-before-schema.jsonl').open(encoding='utf-8-sig') as old_stream, \
     (BASE / 'free-erp-fulfillment-after-schema.jsonl').open(encoding='utf-8-sig') as new_stream:
    old_catalog = record(old_stream)
    new_catalog = record(new_stream)
    for catalog in [old_catalog, new_catalog]:
        checks += 1
        if {key: catalog[key] for key in ['database','role','host','port']} != {
            'database':'erp_shared_contract_test','role':'erp_grid_test','host':'127.0.0.1','port':15438}:
            errors.append('protected QA identity')
    for table in ['order_index','shipments','shipment_items','order_cogs','warehouse_rules','waybills','shipment_cancel_requests']:
        checks += 1
        if any(row['table']==table for row in old_catalog['catalog']) or not any(row['table']==table for row in new_catalog['catalog']):
            errors.append(f'expected absent prerequisite became present: {table}')
    for table, columns in prepared['nullable_compatibility_columns'].items():
        for name in columns:
            checks += 1
            rows = [row for row in new_catalog['catalog'] if row['table']==table and row['column']==name]
            if len(rows)!=1 or rows[0]['nullable']!='YES' or rows[0]['default'] is not None:
                errors.append(f'nullable compatibility column: {table}.{name}')
    previous = record(old_stream)
    current = record(new_stream)
    while previous is not None or current is not None:
        if previous is None or (current is not None and current['tenant_id']<previous['tenant_id']):
            additional.append({'tenant_id':current['tenant_id'],'name':current['name']})
            current=record(new_stream)
            continue
        old_count += 1
        if current is None or previous['tenant_id']<current['tenant_id']:
            errors.append(f'missing old tenant {previous["tenant_id"]}')
            previous=record(old_stream)
            continue
        differences=[]
        for table, rows in current['facts'].items():
            stripped=[]
            for row in rows:
                normalized=dict(row)
                for name in prepared['nullable_compatibility_columns'].get(table,[]):
                    if normalized.pop(name,None) is not None:
                        differences.append(f'{table}.{name}: not NULL')
                stripped.append(normalized)
            checks += 1
            if canonical(stripped)!=canonical(previous['facts'][table]):
                differences.append(table)
        if previous['name']!=current['name']:
            differences.append('tenant name')
        if differences:
            changed.append({'tenant_id':previous['tenant_id'],'name':previous['name'],'differences':differences})
        else:
            unchanged += 1
        previous=record(old_stream)
        current=record(new_stream)
for name, expected in prepared['source_sha256'].items():
    checks += 1
    if hashlib.sha256((ROOT / name).read_bytes()).hexdigest()!=expected:
        errors.append(f'migration fragment changed: {name}')
checks += 1
if hashlib.sha256((BASE / 'free-erp-fulfillment-schema-qa.sql').read_bytes()).hexdigest()!=prepared['script_sha256']:
    errors.append('prepared QA script changed')
report={'updated_at':datetime.now(timezone.utc).isoformat(), 'status':'FAIL' if errors else 'PARTIAL' if changed else 'PASS',
        'checks':checks, 'old_tenants':old_count, 'unchanged_old_tenants':unchanged,
        'concurrent_new_tenants':additional, 'changed_old_tenants':changed, 'errors':errors,
        'qualification':'Guarded private QA prerequisites and nullable preservation; no historical backfill, production apply or full migration acceptance.'}
destination.write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(json.dumps({key:report[key] for key in ['status','checks','old_tenants','unchanged_old_tenants','changed_old_tenants','errors']},ensure_ascii=False))
raise SystemExit(1 if errors else 0)
