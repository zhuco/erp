"""Check declared OMS implementation locations and every authenticated HTTP binding."""
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / 'qa/acceptance'
locations = {
    'crates/erp-api/src/auth_context.rs': ['fulfillment_actor'],
    'crates/erp-repo/src/fulfillment/context.rs': ['authenticated', 'authority', 'require', 'record', 'operation'],
    'crates/erp-repo/src/fulfillment/repo.rs': ['with_actor'],
    'crates/erp-repo/src/fulfillment/store.rs': ['with_actor', 'allocate_order_internal', 'record_allocation', 'ship_oms_shipment', 'release_order_reservations'],
    'crates/erp-repo/src/data_scopes.rs': ['replace_user_warehouses'],
    'crates/erp-api/src/orders/fulfillment.rs': ['submit_order_shipment', 'ship_oms_shipment', 'handle_oms_allocation', 'handle_oms_resend', 'handle_oms_release'],
    'crates/erp-api/src/after_sales_handlers.rs': ['create_after_sales_exchange_shipment', 'create_after_sales_resend_shipment'],
    'crates/erp-api/src/orders/review_flow.rs': ['rollback_order_to_review'],
}
errors = []
records = []
bindings = []
for name, symbols in locations.items():
    path = root / name
    text = path.read_text(encoding='utf8')
    for symbol in symbols:
        pattern = re.search(r'\b(?:async\s+)?fn\s+' + re.escape(symbol) + r'\s*\(', text)
        if not pattern:
            errors.append(f'{name}:{symbol}: missing declaration')
        records.append({'file': name, 'symbol': symbol,
                        'line': text[:pattern.start()].count('\n') + 1 if pattern else None})
    if name.startswith('crates/erp-api/src/') and name != 'crates/erp-api/src/auth_context.rs':
        for symbol in symbols:
            start = text.index(f'async fn {symbol}(')
            ends = [pos for pos in (text.find('\npub(', start + 8), text.find('\nasync fn ', start + 8), text.find('\nfn ', start + 8)) if pos >= 0]
            body = text[start:min(ends) if ends else len(text)]
            count = body.count('.with_actor(ctx.fulfillment_actor(')
            if count != 1:
                errors.append(f'{name}:{symbol}: authenticated binding count {count}')
            bindings.append({'file': name, 'handler': symbol, 'bindings': count})
report = {'status': 'FAIL' if errors else 'PASS', 'declarations': records, 'authenticated_handlers': bindings,
          'source_sha256': {name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in locations}, 'errors': errors}
destination = base / 'free-erp-fulfillment-atomic-document-locations.json'
if destination.exists():
    raise SystemExit('refusing location evidence overwrite')
destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf8')
print(json.dumps({'status': report['status'], 'declarations': len(records), 'authenticated_handlers': len(bindings), 'errors': errors}))
raise SystemExit(1 if errors else 0)
