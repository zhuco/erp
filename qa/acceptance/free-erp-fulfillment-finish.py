"""Bind source windows, immutable QA evidence and current implementation documents."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
DESTINATION = BASE / 'free-erp-fulfillment-repair.json'
if DESTINATION.exists():
    raise SystemExit('refusing to replace delivery evidence')
OWNED = [
    'crates/erp-repo/src/fulfillment/store.rs', 'crates/erp-repo/src/inventory.rs',
    'crates/erp-repo/src/inventory_order_consumers.rs', 'crates/erp-repo/src/inventory_order_reservations.rs',
    'crates/erp-repo/src/lib.rs', 'crates/erp-api/src/orders/fulfillment.rs',
    'crates/erp-api/tests/free_erp_acceptance.rs', 'crates/erp-api/tests/free_erp_acceptance/fulfillment_sources.rs',
]
DOCUMENTS = ['docs/FREE_ERP_CHECKLIST.md', 'docs/FREE_ERP_DEVELOPMENT_SPEC.md',
             'docs/PRODUCT_SCOPE.md', 'docs/ROADMAP.md', 'docs/README.md', 'docs/UI_COMPONENT_LIBRARY.md']


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def read(name):
    return json.loads((BASE / name).read_text(encoding='utf-8-sig'))


owned_now = {name: digest(ROOT / name) for name in OWNED}
documents = {name: digest(ROOT / name) for name in DOCUMENTS}
docs_report = read('free-erp-specification-docs.json')
if docs_report['status'] != 'PASS':
    raise SystemExit('current documentation verification did not pass')
if docs_report['document_sha256'] != documents:
    raise SystemExit('documentation changed after verification')
errors = []
windows = {}
for stage in ('original', 'focused-v1', 'focused-v2', 'full', 'full-final'):
    before = read(f'free-erp-inventory-consumers-fulfillment-before-{stage}-source.json')['files']
    after = read(f'free-erp-inventory-consumers-fulfillment-after-{stage}-source.json')['files']
    changed = [name for name in sorted(before.keys() | after.keys()) if before.get(name) != after.get(name)]
    window = dict(before_inputs=len(before), after_inputs=len(after),
                  owned_changes=[name for name in changed if name in OWNED],
                  shared_changes=[name for name in changed if name not in OWNED])
    windows[stage] = window
    if window['owned_changes']:
        errors.append(f'owned source changed during {stage}')
    if stage == 'full-final':
        for name, expected in owned_now.items():
            if after.get(name) != expected:
                errors.append(f'current owned input differs from final tested snapshot: {name}')

clippy_logs = {}
for name in ('clippy', 'clippy-final'):
    path = BASE / f'free-erp-fulfillment-{name}.log'
    raw = path.read_bytes()
    text = raw.decode('utf-16' if raw[:2] in (b'\xff\xfe', b'\xfe\xff') else 'utf-8-sig')
    diagnostics = []
    for block in re.split(r'(?m)(?=^error:)', text):
        if any(source.replace('/', '\\') in block for source in OWNED):
            diagnostics.append('\n'.join(block.splitlines()[:14]))
    clippy_logs[name] = dict(status='FAIL' if 'Cargo clippy failed' in text else 'NOT_CONFIRMED',
                             compiler_summaries=re.findall(r'could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors?', text),
                             owned_file_diagnostics=diagnostics)

independent = {stage: read(f'free-erp-fulfillment-{stage}-independent.json')
               for stage in ('original', 'focused-v1', 'focused-v2', 'full', 'full-final')}
for stage, report in independent.items():
    if report['status'] == 'FAIL' or report['errors']:
        errors.append(f'independent facts failed: {stage}')
original_owned = read('free-erp-fulfillment-original-owned.json')
for name, expected in original_owned.items():
    archive = BASE / ('free-erp-fulfillment-original-' + name.replace('/', '_') + '.txt')
    if digest(archive) != expected:
        errors.append(f'original source archive changed: {name}')

# Include every existing file of this batch. Files and original failures remain
# immutable; the binding only adds one record and never edits business data.
artifacts = {path.name: dict(bytes=path.stat().st_size, sha256=digest(path))
             for path in sorted(BASE.glob('free-erp-fulfillment-*')) if path.is_file()}
manifest = read('free-erp-inventory-consumers-fulfillment-after-full-final-source.json')['files']
current = read('free-erp-inventory-consumers-fulfillment-delivery-source.json')['files']
for name, expected in current.items():
    if not (ROOT / name).is_file() or digest(ROOT / name) != expected:
        raise SystemExit('current source changed after delivery capture; no record written')
current_changes = {name: dict(tested_sha256=manifest.get(name), current_sha256=current.get(name))
                   for name in sorted(manifest.keys() | current.keys()) if manifest.get(name) != current.get(name)}
limits = [
    'Strict backend Clippy failed in shared repository/dependencies; API lint may not be reached.',
    'Source windows include concurrent shared inputs; complete source qualification remains PARTIAL.',
    'Mandatory OMS operation/audit logs and ShipmentShipped still include postcommit best-effort paths.',
    'Current transaction-bound warehouse permissions, durable resend identity and complete command receipts remain pending.',
    'Cost unknown/overflow, precise batch/quality dimensions and total plan memory bounds need separate acceptance.',
    'Trade sales-orders and manufacturing material consumers are separate from these real OMS fixtures.',
    'Real password/free seats/multi-device, native/Web, full migrations/recovery, Release load and public hosting are NOT_RUN here.',
]
report = dict(updated_at=datetime.now(timezone.utc).isoformat(), status='FAIL' if errors else 'PARTIAL',
              qualification='Real private-QA OMS repair and detailed development specification; no free ERP launch or performance completion.',
              source_windows=windows, owned_source_sha256=owned_now, documents_sha256=documents,
              current_shared_input_changes=current_changes, schema_prerequisites=read('free-erp-fulfillment-schema-independent.json'),
              independent_facts=independent, strict_clippy=clippy_logs, documentation_verification=docs_report,
              artifacts=artifacts, remaining_gates=limits, errors=errors)
if owned_now != {name: digest(ROOT / name) for name in OWNED} or documents != {name: digest(ROOT / name) for name in DOCUMENTS}:
    raise SystemExit('owned inputs or documents changed while binding; no record written')
DESTINATION.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps(dict(status=report['status'], owned_inputs=len(owned_now), artifacts=len(artifacts),
                     final_test=independent['full-final']['test'], errors=errors), ensure_ascii=False))
raise SystemExit(1 if errors else 0)
