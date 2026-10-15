"""Bind current owned code, complete independent facts, format, contract and docs."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
base = root / 'qa/acceptance'
prefix = 'free-erp-fulfillment-cost-batch-'
destination = base / (prefix + 'repair.json')
if destination.exists():
    raise SystemExit('refusing delivery evidence overwrite')

def read(name):
    return json.loads((base / name).read_text(encoding='utf-8-sig'))

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

errors = []
proofs = {stage: read(prefix + stage + '-independent.json') for stage in ('original', 'focus', 'full')}
for stage, proof in proofs.items():
    if proof['status'] != 'PASS' or proof['errors'] or proof['changed_old_tenants'] or proof['old_tenants'] != proof['unchanged_old_tenants']:
        errors.append('independent facts failed: ' + stage)
    if proof['source_window']['owned_changed']:
        errors.append('owned source changed in actual test: ' + stage)
if (proofs['original']['test']['passed'], proofs['original']['test']['failed']) != (2, 12):
    errors.append('fourteen original cases did not reproduce twelve failures')
if proofs['focus']['test']['passed'] != 14 or proofs['full']['test']['passed'] != 216 or proofs['full']['test']['failed'] or proofs['full']['test']['ignored']:
    errors.append('complete current target/filter did not pass')
owned = read(prefix + 'original-owned.json')
source = {name: digest(root / name) for name in sorted(owned)}
tested = read('free-erp-inventory-consumers-fulfillment-cost-batch-full-after-source.json')['files']
for name, sha in source.items():
    if tested.get(name) != sha:
        errors.append('current owned source differs from the full test: ' + name)
formatted = read(prefix + 'format.json')
if formatted['status'] != 'PASS' or len(formatted['leaf_files']) != 6:
    errors.append('six affected leaves were not checked by the fixed entry')
for name, sha in formatted['leaf_files'].items():
    if source.get(name) != sha:
        errors.append('formatted current leaf drift: ' + name)
lint = read(prefix + 'clippy-diagnostics.json')
if lint['observed_owned_diagnostics'] or lint['source_window']['owned_changed']:
    errors.append('new owned lint diagnostics or source drift')
contract = read(prefix + 'openapi-final.json')
if contract['status'] != 'PASS' or contract['errors']:
    errors.append('actual source/generated shipment contract failed')
for name, sha in contract['source_sha256'].items():
    if digest(root / name) != sha:
        errors.append('current contract input drifted: ' + name)
documents = ['docs/FREE_ERP_CHECKLIST.md', 'docs/FREE_ERP_DEVELOPMENT_SPEC.md',
             'docs/PRODUCT_SCOPE.md', 'docs/ROADMAP.md', 'docs/README.md', 'docs/UI_COMPONENT_LIBRARY.md']
document_hashes = {name: digest(root / name) for name in documents}
doc_proof = read('free-erp-specification-docs.json')
if doc_proof['status'] != 'PASS' or doc_proof['document_sha256'] != document_hashes:
    errors.append('current document verification failed or drifted')
artifact_paths = list(base.glob(prefix + '*'))
artifact_paths += list(base.glob('free-erp-inventory-consumers-fulfillment-cost-batch-*-source.json'))
artifact_paths += [base / 'free-erp-fulfillment-readback.sql', base / 'free-erp-specification-docs.json', base / 'free-erp-specification-docs-verify.py']
artifacts = {path.name: dict(bytes=path.stat().st_size, sha256=digest(path))
             for path in sorted(set(artifact_paths)) if path.is_file() and path != destination}
report = dict(updated_at=datetime.now(timezone.utc).isoformat(), status='FAIL' if errors else 'PARTIAL',
    implementation='Original OMS known nonnegative checked COGS and exact owned unexpired/unreserved FEFO batches share required stock/source/status/log/event commit.',
    original_policy='Old integer 1 remains one whole base unit, converted to 1000000 micro-units.',
    owned_source_sha256=source, document_sha256=document_hashes,
    test_windows={stage: proof['test'] for stage, proof in proofs.items()},
    full_independent_readback={key: proofs['full'][key] for key in ('status', 'checks', 'old_tenants', 'unchanged_old_tenants', 'own_case_count', 'owned_row_counts', 'source_window')},
    all_test_source_windows={stage: proof['source_window'] for stage, proof in proofs.items()},
    strict_clippy=lint, format=formatted, contract=contract, document_checks=doc_proof['checks'],
    qualification='Protected local QA, synthetic JWT, actual Axum routes and PostgreSQL. Owned nine inputs match full-test and fixed-format snapshots; other active shared inputs changed. Not real login/native/Web/public/whole-release qualification.',
    performance='NOT_RUN; debug test duration is not P95/throughput or memory acceptance.',
    verification_history='Original verifier preserved before correcting final overflow-rejection fixture initial hand arithmetic from 3.9 to 4.9; original failure facts/logs unchanged.',
    outstanding_gates=[
        'All batch inbound/adjustment/manufacturing/other writers and original migration/reconciliation policy',
        'Immutable actual batch consumption provenance and batch reservation ownership',
        'Cost method/period/original return cost and unknown/provisional-cost workflow',
        'Actual batch configuration/metadata/expiry lock races and expiry policy at late commit',
        'Role/session revocation and internal worker execution policy',
        'Waybill/WMS/aftersales/review/cancel outer workflow atomicity or compensation',
        'Durable original fulfillment command/resend receipt and GET recovery',
        'Real password login, one-seat multi-device native/Web operation and failure recovery',
        'Full migration/recovery and strict dependency/API Clippy',
        'Release performance/resources and official public-hosted capacity/operations'],
    artifacts=artifacts, errors=errors)
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps({'status': report['status'], 'owned_inputs': len(source), 'artifacts': len(artifacts), 'errors': errors}))
raise SystemExit(1 if errors else 0)
