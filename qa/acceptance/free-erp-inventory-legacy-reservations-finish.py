"""Bind actual logs/source windows/readback; never infer public or performance acceptance."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
PREFIX = 'free-erp-inventory-legacy-reservations'
parser = argparse.ArgumentParser()
parser.add_argument('--mode', choices=['proof', 'delivery'], required=True)
MODE = parser.parse_args().mode
OWNED = [
    'crates/erp-repo/src/inventory.rs', 'crates/erp-repo/src/inventory_reservation_audit.rs',
    'crates/erp-repo/src/lib.rs', 'crates/erp-api/src/lib.rs',
    'crates/erp-api/src/repo_ports.rs', 'crates/erp-api/src/inventory_views.rs',
    'crates/erp-api/src/inventory_print_report_handlers/stock_movement.rs',
    'crates/erp-api/tests/free_erp_acceptance.rs',
    'crates/erp-api/tests/free_erp_acceptance/inventory_legacy_reservation_atomicity.rs',
    'crates/erp-api/tests/free_erp_acceptance/inventory_legacy_reservation_edges.rs',
    'crates/erp-repo/src/shared_contract/item_cost.rs',
]
TOOLS = [f'qa/acceptance/{PREFIX}-verify.py', f'qa/acceptance/{PREFIX}-stream-readback.sql',
         f'qa/acceptance/{PREFIX}-cost-readback.sql', f'qa/acceptance/{PREFIX}-cost-verify.py',
         f'qa/acceptance/{PREFIX}-finish.py', 'qa/acceptance/free-erp-inventory-consumers-source.py',
         'qa/acceptance/free-erp-specification-docs-verify.py']
LIMITS = [
    'Source windows include all actual native/Web Rust, schema, lock, build-entry and OpenAPI inputs; dependency drift remains PARTIAL.',
    'Strict backend Clippy still fails; absence of new reservation diagnostics does not pass dependencies or unreached API lint.',
    'Protected private PostgreSQL and synthetic JWT/debug tests only, not real login, one-seat/multi-device, desktop/Web or public service.',
    'Current source queries/UI, whole sales/work-order transitions and source consumption/cancellation remain open.',
    'Kit parent quantity/frozen component expansion and changed-BOM release remain open; tested kit log rollback does not establish them.',
    'Fractional request input, all legacy consumers, commit/network recovery, full migration/restore and inventory Release performance remain open.',
]


def read(name):
    return json.loads((BASE / name).read_text(encoding='utf-8-sig'))


def decoded(name):
    data = (BASE / name).read_bytes()
    return data.decode('utf-16' if data.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig')


def sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def source(stage):
    return read(f'free-erp-inventory-consumers-legacy-reservations-{stage}-source.json')['files']


def window(stage, owned):
    before, after = source(f'{stage}-before'), source(f'{stage}-after')
    changed = sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))
    own_changed = [name for name in owned if before.get(name) != after.get(name) or name not in before]
    return {'status': 'PARTIAL' if changed else 'PASS', 'input_count_before': len(before),
            'input_count_after': len(after), 'owned_input_count': len(owned),
            'changed_owned_inputs': own_changed, 'all_input_drift': changed,
            'owned_sha256_after': {name: after[name] for name in owned}}


def result(name):
    values = re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;[^\n]*finished in ([\d.]+)s', decoded(name))
    if not values:
        raise ValueError('no terminal registered test result: ' + name)
    value = values[-1]
    return {'status': 'PASS' if value[0] == 'ok' else 'FAIL', 'passed': int(value[1]),
            'failed': int(value[2]), 'ignored': int(value[3]), 'seconds_debug': float(value[4])}


if MODE == 'proof':
    destination = BASE / f'{PREFIX}-repair.json'
    if destination.exists():
        raise SystemExit('refusing to replace immutable proof')
    stages = {stage: read(f'{PREFIX}-{stage}-independent' + ('-v2' if stage != 'original' else '') + '.json')
              for stage in ['original', 'focused-v2', 'full', 'full-v2']}
    cost = read(f'{PREFIX}-cost-fence-independent.json')
    contract_before = read(f'{PREFIX}-openapi-before.json')
    contract_after = read(f'{PREFIX}-openapi-after.json')
    contract_changed = [name for name in contract_before if name != 'openapi/erp_api_v1.yaml'
                        and contract_before[name] != contract_after[name]]
    if 'OpenAPI gate verification passed' not in decoded(f'{PREFIX}-openapi-gate.log'):
        raise ValueError('terminal OpenAPI verification missing')
    windows = {stage: window(stage, OWNED[:-1] if stage in ['focused', 'focused-v2'] else OWNED)
               for stage in ['focused', 'focused-v2', 'full', 'cost-fence', 'full-v2']}
    current_after = source('full-v2-after')
    current_changed = [name for name in OWNED if sha(ROOT / name) != current_after[name]]
    log = decoded(f'{PREFIX}-backend-clippy-v2.log')
    lint_errors = [{'crate': crate, 'target': target, 'errors': int(count)} for crate, target, count in re.findall(
        r'could not compile `([^`]+)` \(([^)]+)\) due to (\d+) previous errors', log)]
    if not lint_errors:
        raise ValueError('strict lint terminal output missing')
    histories = [f'{PREFIX}-{suffix}' for suffix in [
        'before-repair.log', 'focused.log', 'focused-v2.log', 'full.log', 'full-v2.log',
        'qa-before-repair.json', 'qa-original-after.json', 'qa-original-after-stream.jsonl',
        'qa-before-focused.jsonl', 'qa-after-focused.jsonl',
        'qa-before-focused-v2.jsonl', 'qa-after-focused-v2.jsonl',
        'qa-before-full.jsonl', 'qa-after-full.jsonl',
        'qa-before-full-v2.jsonl', 'qa-after-full-v2.jsonl',
        'original-independent.json', 'focused-v2-independent.json',
        'focused-v2-independent-v2.json', 'full-independent-v2.json', 'full-v2-independent-v2.json',
        'verifier-v1.py.txt', 'cost-fence-original.rs.txt', 'cost-fence.log',
        'cost-fence-readback.json', 'cost-fence-independent.json',
        'backend-clippy.log', 'backend-clippy-v2.log', 'fmt-initial.log', 'fmt-v2.log', 'fmt-v3.log',
        'openapi-before.json', 'openapi-after.json', 'openapi-bundle.log', 'openapi-gate.log',
        'cost-fence-fmt.log', 'docs-verifier-original.py.txt',
        'original-crates_erp-repo_src_inventory.rs.txt', 'original-crates_erp-repo_src_lib.rs.txt',
        'original-crates_erp-api_src_repo_ports.rs.txt',
        'original-crates_erp-api_src_inventory_print_report_handlers_stock_movement.rs.txt',
        'pre-v2-crates_erp-api_src_inventory_views.rs.txt', 'pre-v2-crates_erp-api_src_lib.rs.txt',
    ]]
    histories += [f'free-erp-inventory-consumers-legacy-reservations-{stage}-source.json' for stage in
                  ['before-repair', 'original-after'] + [f'{stage}-{side}' for stage in windows for side in ['before', 'after']]]
    passed = stages['original']['status'] == stages['focused-v2']['status'] == stages['full-v2']['status'] == cost['status'] == 'PASS'
    passed = passed and not current_changed and not any(value['changed_owned_inputs'] for value in windows.values())
    report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'PARTIAL',
              'artifact_qualification': 'PASS' if passed else 'FAIL',
              'scope': 'Legacy UUID-source reserve/release atomic required logs, physical lock order, canonical aliases/current permission; cost command fence call sites restored.',
              'original_counterexamples': result(f'{PREFIX}-before-repair.log'),
              'first_focused': result(f'{PREFIX}-focused.log'),
              'focused': result(f'{PREFIX}-focused-v2.log'),
              'first_full': result(f'{PREFIX}-full.log'),
              'cost_fence': result(f'{PREFIX}-cost-fence.log'),
              'full': result(f'{PREFIX}-full-v2.log'), 'independent': stages, 'cost_readback': cost,
              'source_windows': windows, 'changed_owned_after_full': current_changed,
              'owned_sha256': {name: sha(ROOT / name) for name in OWNED},
              'verification_tools_sha256': {name: sha(ROOT / name) for name in TOOLS},
              'historical_artifacts_sha256': {f'qa/acceptance/{name}': sha(BASE / name) for name in histories},
              'strict_clippy': {'status': 'FAIL', 'errors': lint_errors,
                                'new_reservation_leaf_diagnostics': re.findall(r'inventory_reservation_audit\.rs:\d+:\d+', log),
                                'unreached_dependency_targets_are_not_passed': True},
              'openapi': {'status': 'PARTIAL' if contract_changed else 'PASS',
                          'before': contract_before, 'after': contract_after,
                          'changed_input_sources': contract_changed,
                          'bundler_and_gate_commands': 'PASS',
                          'generated_bundle_changes_are_expected_and_preserved': True},
              'evidence_repairs': [
                  'Original aggregate readback disconnected with server closed connection and produced a zero-byte file; preserved. Read-only streaming recovery verified the same protected identity and existing row sets; no restart/cleanup or fixture rewrite.',
                  'First independent verifier sorted pre-lock occurred_at and falsely rejected mixed effects. Failed report/source retained; v2 verifies the complete exact state chain against signed deltas and independent final stock.',
                  'First focused rejected changed order context as 400; mapping corrected to conflict 409 without loosening the test.',
                  'First full failed the real cost advisory waiter because four existing lock_cost_command calls were missing; restored before item/period fences, with unchanged test and independent original cost/sequence readback.',
              ], 'database_changes': 'No new DDL or migration this batch. Only newly-owned isolated QA fixtures/fault views; earlier failed samples preserved.',
              'performance': {'status': 'NOT_RUN', 'metric_ids': ['ERP-P04', 'ERP-P05', 'ERP-P12'],
                              'debug_seconds_are_not_percentiles_or_throughput': True},
              'remaining_gates': LIMITS}
    destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
    print(json.dumps({key: report[key] for key in ['status', 'artifact_qualification', 'full', 'changed_owned_after_full', 'strict_clippy']}, ensure_ascii=False))
    raise SystemExit(0 if passed else 1)

destination = BASE / f'{PREFIX}-delivery.json'
if destination.exists():
    raise SystemExit('refusing to replace delivery evidence')
proof = read(f'{PREFIX}-repair.json')
documentation = read('free-erp-specification-docs.json')
owned_changed = [name for name, digest in proof['owned_sha256'].items() if sha(ROOT / name) != digest]
tools_changed = [name for name, digest in proof['verification_tools_sha256'].items() if sha(ROOT / name) != digest]
docs_changed = [name for name, digest in documentation['document_sha256'].items() if sha(ROOT / name) != digest]
history_changed = [name for name, digest in proof['historical_artifacts_sha256'].items() if sha(ROOT / name) != digest]
full = source('full-v2-after')
delivery = source('delivery')
drift = sorted(name for name in full.keys() | delivery.keys() if full.get(name) != delivery.get(name))
report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'PARTIAL',
          'artifact_qualification': 'PASS' if documentation['status'] == proof['artifact_qualification'] == 'PASS'
             and not any([owned_changed, tools_changed, docs_changed, history_changed]) else 'FAIL',
          'owned_inputs': len(OWNED), 'verification_tools': len(TOOLS), 'documents': len(documentation['document_sha256']),
          'historical_artifacts': len(proof['historical_artifacts_sha256']),
          'changed_owned_inputs': owned_changed, 'changed_verification_tools': tools_changed,
          'changed_documents': docs_changed, 'changed_historical_artifacts': history_changed,
          'all_input_count': len(delivery), 'delivery_dependency_drift_since_full_readback': drift,
          'documentation_checks': documentation['checks'],
          'proof_sha256': sha(BASE / f'{PREFIX}-repair.json'),
          'documentation_report_sha256': sha(BASE / 'free-erp-specification-docs.json'), 'limits': LIMITS}
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
print(json.dumps({key: report[key] for key in ['status', 'artifact_qualification', 'owned_inputs', 'documents', 'historical_artifacts', 'changed_owned_inputs', 'changed_documents']}, ensure_ascii=False))
raise SystemExit(0 if report['artifact_qualification'] == 'PASS' else 1)
