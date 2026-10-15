"""Immutable owned input archives for OMS authority and commit repair."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
base = root / 'qa/acceptance'
paths = [
    'crates/erp-repo/src/fulfillment.rs',
    'crates/erp-repo/src/fulfillment/repo.rs',
    'crates/erp-repo/src/fulfillment/store.rs',
    'crates/erp-repo/src/data_scopes.rs',
    'crates/erp-repo/src/outbox.rs',
    'crates/erp-api/src/auth_context.rs',
    'crates/erp-api/src/inventory_views.rs',
    'crates/erp-api/src/orders/fulfillment.rs',
    'crates/erp-api/src/orders/review_flow.rs',
    'crates/erp-api/src/after_sales_handlers.rs',
    'crates/erp-api/tests/free_erp_acceptance.rs',
    'crates/erp-api/tests/free_erp_acceptance/fulfillment_atomicity.rs',
]
result = {}
for rel in paths:
    source = root / rel
    data = source.read_bytes()
    archived = base / ('free-erp-fulfillment-atomic-original-' + rel.replace('/', '__') + '.txt')
    if archived.exists():
        raise SystemExit('refusing to replace original source')
    archived.write_bytes(data)
    result[rel] = {'sha256': hashlib.sha256(data).hexdigest(), 'archive': archived.name}
output = base / 'free-erp-fulfillment-atomic-original-owned.json'
if output.exists():
    raise SystemExit('refusing to replace original input manifest')
output.write_text(json.dumps(result, indent=2) + '\n', encoding='utf8')
print(json.dumps({'owned_inputs': len(result)}))
