"""Apply scoped API wiring without rewriting unrelated shared work."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]

def edit_functions(relative, functions, transform):
    path = root / relative
    text = path.read_text(encoding='utf8')
    for name in functions:
        anchor = f'async fn {name}('
        if text.count(anchor) != 1:
            raise SystemExit(f'nonunique function: {relative}:{name}')
        start = text.index(anchor)
        boundaries = [p for p in [text.find('\npub(', start + len(anchor)),
                                  text.find('\nasync fn ', start + len(anchor)),
                                  text.find('\nfn ', start + len(anchor))] if p >= 0]
        end = min(boundaries) if boundaries else len(text)
        before = text[start:end]
        after = transform(name, before)
        if before == after:
            raise SystemExit(f'no scoped wiring change: {relative}:{name}')
        text = text[:start] + after + text[end:]
    path.write_text(text, encoding='utf8')

def fulfillment(name, text):
    old = 'FulfillmentRepo::new(state.db.pool());'
    if text.count(old) != 1:
        raise SystemExit(f'nonunique repo binding: {name}')
    text = text.replace(old, 'FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(&req_ctx));')
    if name == 'ship_oms_shipment':
        anchor = '    if let Err(resp) = ensure_permission(&ctx, "shipping.write") {\n        return resp.into_response();\n    }'
        if text.count(anchor) != 1:
            raise SystemExit('shipping permission boundary')
        text = text.replace(anchor, anchor + '\n    if let Err(resp) = ensure_permission(&ctx, "inventory.write") {\n        return resp.into_response();\n    }')
        start = text.index('        let outbox_repo =')
        end = text.index('\n    let items =', start)
        text = text[:start] + '    }\n' + text[end:]
    elif name in ('handle_oms_allocation', 'handle_oms_resend', 'handle_oms_release'):
        start = text.index('    let log_metadata =')
        end = text.index('    let response =', start)
        text = text[:start] + text[end:]
    return text

edit_functions('crates/erp-api/src/orders/fulfillment.rs', [
    'submit_order_shipment', 'ship_oms_shipment', 'handle_oms_allocation', 'handle_oms_resend', 'handle_oms_release'
], fulfillment)

def after_sales(name, text):
    extension = '    Extension(ctx): Extension<AuthContext>,\n'
    if text.count(extension) != 1:
        raise SystemExit(f'actor extension: {name}')
    text = text.replace(extension, extension + '    Extension(req_ctx): Extension<RequestContext>,\n')
    repo = 'FulfillmentRepo::new(state.db.pool());'
    if text.count(repo) != 1:
        raise SystemExit(f'after-sales repo binding: {name}')
    text = text.replace(repo, 'FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(&req_ctx));')
    call = text.index('let result = match fulfillment_repo')
    error = 'Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),'
    error_at = text.index(error, call)
    text = text[:error_at] + 'Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),' + text[error_at + len(error):]
    return text

edit_functions('crates/erp-api/src/after_sales_handlers.rs', [
    'create_after_sales_exchange_shipment', 'create_after_sales_resend_shipment'
], after_sales)

def reopen(name, text):
    repo = 'FulfillmentRepo::new(state.db.pool());'
    if text.count(repo) != 1:
        raise SystemExit('review repo binding')
    text = text.replace(repo, 'FulfillmentRepo::new(state.db.pool()).with_actor(ctx.fulfillment_actor(&req_ctx));')
    call = text.index('let release_result = match fulfillment_repo')
    error = 'Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response(),'
    error_at = text.index(error, call)
    return text[:error_at] + 'Err(e) => return crate::inventory_views::inventory_error_response(e).into_response(),' + text[error_at + len(error):]

edit_functions('crates/erp-api/src/orders/review_flow.rs', ['rollback_order_to_review'], reopen)
print('Wired 8 authenticated OMS/API callers; removed 4 post-commit mandatory record blocks')
