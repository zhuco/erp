"""Preserve original local OMS cost and batch inputs before any repair."""
from pathlib import Path
import hashlib
import json

root = Path(__file__).resolve().parents[2]
base = root / 'qa/acceptance'
destination = base / 'free-erp-fulfillment-cost-batch-original-owned.json'
if destination.exists():
    raise SystemExit('refusing original evidence overwrite')
paths = [
    'crates/erp-repo/src/fulfillment.rs',
    'crates/erp-repo/src/fulfillment/store.rs',
    'crates/erp-repo/src/fulfillment/costs.rs',
    'crates/erp-repo/src/inventory.rs',
    'crates/erp-repo/src/inventory_batch_consumption.rs',
    'crates/erp-repo/src/lib.rs',
    'crates/erp-api/src/inventory_views.rs',
    'crates/erp-api/tests/free_erp_acceptance.rs',
    'crates/erp-api/tests/free_erp_acceptance/fulfillment_cost_batches.rs',
]
report = {}
for name in paths:
    source = root / name
    if not source.exists():
        report[name] = None
        continue
    data = source.read_bytes()
    archive = base / ('free-erp-fulfillment-cost-batch-original-' + name.replace('/', '_') + '.txt')
    if archive.exists():
        raise SystemExit('refusing original input archive overwrite')
    archive.write_bytes(data)
    report[name] = hashlib.sha256(data).hexdigest()
destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
print(json.dumps({'preserved': sum(value is not None for value in report.values()), 'inputs': len(report)}))
