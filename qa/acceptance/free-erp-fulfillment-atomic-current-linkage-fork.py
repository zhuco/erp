"""Run the same two-way linkage check on the current full-regression window."""
from pathlib import Path

base = Path(__file__).resolve().parent
source = (base / 'free-erp-fulfillment-atomic-linkage.py').read_text(encoding='utf8')
source = source.replace('free-erp-fulfillment-atomic-final-facts-v3.json', 'free-erp-fulfillment-atomic-current-facts-v4.json')
source = source.replace('free-erp-fulfillment-atomic-final-after.jsonl', 'free-erp-fulfillment-atomic-current-after.jsonl')
source = source.replace('free-erp-fulfillment-atomic-final-linkage.json', 'free-erp-fulfillment-atomic-current-linkage.json')
destination = base / 'free-erp-fulfillment-atomic-linkage-current.py'
if destination.exists():
    raise SystemExit('refusing linkage validator overwrite')
destination.write_text(source, encoding='utf8')
print('created current immutable linkage validator')
