"""Bind original failures, actual routes, independently read facts and current source/docs."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
base = root / 'qa/acceptance'
destination = base / 'free-erp-fulfillment-atomic-repair.json'
if destination.exists():
    raise SystemExit('refusing delivery evidence overwrite')

def read(name):
    return json.loads((base / name).read_text(encoding='utf-8-sig'))

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

owned = set(read('free-erp-fulfillment-atomic-original-owned.json'))
owned.update(['crates/erp-repo/src/fulfillment/context.rs', 'crates/erp-api/src/lib.rs'])
current = {name: digest(root / name) for name in sorted(owned)}
tested = read('free-erp-inventory-consumers-fulfillment-atomic-current-after-source.json')['files']
errors = [f'current owned source differs from tested input: {name}' for name in owned if current[name] != tested.get(name)]
windows = {stage: read(f'free-erp-fulfillment-atomic-{stage}-source-window.json')
           for stage in ('original', 'focus', 'full', 'time-original', 'final', 'clippy', 'current')}
for stage, window in windows.items():
    if window['owned_changed']:
        errors.append(f'owned inputs changed during {stage}')
proofs = {
    'original': read('free-erp-fulfillment-atomic-original-independent.json'),
    'focus': read('free-erp-fulfillment-atomic-focus-independent.json'),
    'full': read('free-erp-fulfillment-atomic-full-independent.json'),
    'time_original': read('free-erp-fulfillment-atomic-time-original-facts-v2.json'),
    'final': read('free-erp-fulfillment-atomic-final-facts-v3.json'),
    'current': read('free-erp-fulfillment-atomic-current-facts-v4.json'),
}
for name, proof in proofs.items():
    if proof['errors'] or proof['changed_old_tenants']:
        errors.append(f'independent proof failed or changed old tenant facts: {name}')
latest = proofs['current']
if latest['test']['passed'] != 202 or latest['test']['failed'] != 0 or latest['test']['ignored'] != 0:
    errors.append('current full 202-test registered target did not pass')
if latest['unchanged_old_tenants'] != latest['old_tenants'] or latest['false_transitions'] or latest['temporal_differences']:
    errors.append('current readback does not preserve old facts and exact committed transitions')
locations = read('free-erp-fulfillment-atomic-current-document-locations.json')
if locations['status'] != 'PASS' or len(locations['authenticated_handlers']) != 8:
    errors.append('current authenticated handler wiring verification failed')
for name, sha in locations['source_sha256'].items():
    if digest(root / name) != sha:
        errors.append(f'current location source drift: {name}')
linkage = read('free-erp-fulfillment-atomic-current-linkage.json')
if linkage['status'] != 'PASS':
    errors.append('current two-way shipment/event/record linkage did not pass')
documents = ['docs/FREE_ERP_CHECKLIST.md', 'docs/FREE_ERP_DEVELOPMENT_SPEC.md',
             'docs/PRODUCT_SCOPE.md', 'docs/ROADMAP.md', 'docs/README.md', 'docs/UI_COMPONENT_LIBRARY.md']
doc_hashes = {name: digest(root / name) for name in documents}
doc_proof = read('free-erp-specification-docs.json')
if doc_proof['status'] != 'PASS' or doc_proof['document_sha256'] != doc_hashes:
    errors.append('current docs or their verification drifted')
diagnostics = read('free-erp-fulfillment-atomic-clippy-diagnostics.json')
artifacts = {}
paths = list(base.glob('free-erp-fulfillment-atomic-*'))
paths += list(base.glob('free-erp-inventory-consumers-fulfillment-atomic-*-source.json'))
paths += [base / 'free-erp-fulfillment-readback.sql', base / 'free-erp-specification-docs.json',
          base / 'free-erp-specification-docs-verify.py']
for path in sorted(set(paths)):
    if path.is_file() and path != destination:
        artifacts[path.name] = {'bytes': path.stat().st_size, 'sha256': digest(path)}
report = {
    'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'FAIL' if errors else 'PARTIAL',
    'implementation': 'Authenticated current-user/warehouse admission and mandatory OMS stock/source/COGS/status/log/event commit; guarded persisted-time return',
    'owned_source_sha256': current, 'document_sha256': doc_hashes,
    'source_windows': windows,
    'test_windows': {name: proof['test'] for name, proof in proofs.items()},
    'current_independent_readback': {key: latest[key] for key in ('status', 'checks', 'old_tenants', 'unchanged_old_tenants', 'owned_row_counts', 'mandatory_faults', 'authority_races', 'temporal_differences', 'false_transitions')},
    'current_linkage': linkage, 'strict_clippy': diagnostics,
    'format': {'fixed_entry_leaf_check': 'PASS', 'leaf_files': 10, 'root_workspace_fmt': 'NOT_CLAIMED'},
    'artifact_history': {'final_v2': 'counter-invalid report preserved; v3 only repairs shadowed report variable and rereads immutable facts',
                         'current_v4': 'same exact-time/fault/race gates after removing three redundant RequestContext borrows'},
    'outstanding_gates': ['Role/session/credentials revocation bound to transaction', 'Internal worker policy',
        'Waybill and outer WMS/aftersales/review/cancel workflow commit or compensation',
        'Durable original command/resend identity and GET recovery', 'Unknown cost and exact eligible batches',
        'Current source query and native/Web real login/edit/submit/recovery', 'Full migrations and recovery',
        'Strict dependency/API Clippy', 'Release P04/P05/P12 load and resources', 'Official public-hosted capacity and operations'],
    'performance_status': 'NOT_RUN; debug test seconds are not latency/throughput metrics',
    'qualification': 'Protected local QA with synthetic JWT and real Axum routes/SQL faults; not real account, seat/session, native/Web or launch acceptance',
    'artifacts': artifacts, 'errors': errors,
}
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps({'status': report['status'], 'owned_inputs': len(current), 'artifacts': len(artifacts), 'errors': errors}))
raise SystemExit(1 if errors else 0)
