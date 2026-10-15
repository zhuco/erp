"""Preserve v2 and correct a report-only variable shadow in its fault loop."""
from pathlib import Path

base = Path(__file__).resolve().parent
source = (base / 'free-erp-fulfillment-atomic-verify-v2.py').read_text(encoding='utf8')
old = 'for tenant, action, table, schema, unchanged in faults:'
if source.count(old) != 1:
    raise SystemExit('nonunique report-counter correction')
source = source.replace(old, 'for tenant, action, table, schema, fault_unchanged in faults:')
source = source.replace("f'free-erp-fulfillment-{args.stage}-facts-v2.json'", "f'free-erp-fulfillment-{args.stage}-facts-v3.json'")
bad = base / 'free-erp-fulfillment-atomic-final-facts-v2.json'
archive = base / 'free-erp-fulfillment-atomic-final-facts-v2-counter-invalid.json'
destination = base / 'free-erp-fulfillment-atomic-verify-v3.py'
if destination.exists() or archive.exists():
    raise SystemExit('refusing evidence overwrite')
bad.rename(archive)
destination.write_text(source, encoding='utf8')
print('preserved v2 evidence; v3 keeps the independently counted old-tenant total')
