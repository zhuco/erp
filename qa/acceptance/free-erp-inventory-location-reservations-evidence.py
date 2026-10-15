"""Bind actual source windows, failures and readbacks; no build or data mutation."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
base = root / 'qa/acceptance'
prefix = 'free-erp-inventory-location-reservations'
destination = base / f'{prefix}-repair.json'
if destination.exists():
    raise SystemExit('refusing to replace historical repair evidence')


def read_json(name):
    return json.loads((base / name).read_text(encoding='utf-8-sig'))


def log(name):
    value = (base / name).read_bytes()
    return value.decode('utf-16' if value.startswith((b'\xff\xfe', b'\xfe\xff')) else 'utf-8-sig')


def digest(path):
    sha = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1_048_576), b''):
            sha.update(block)
    return sha.hexdigest()


owned = ['crates/erp-repo/src/'+name for name in ['lib.rs','inventory_movement_commands.rs',
         'inventory_location_commands.rs','inventory_location_reservations.rs']]
owned += ['crates/erp-api/src/lib.rs', 'crates/erp-api/src/middleware/idempotency.rs']
owned += ['crates/erp-api/src/inventory_print_report_handlers/'+name for name in
          ['inventory_commands.rs','stock_movement.rs','inventory_core.rs']]
owned += ['crates/erp-api/tests/free_erp_acceptance.rs']
owned += ['crates/erp-api/tests/free_erp_acceptance/'+name for name in
          ['inventory_location_reservations.rs','inventory_location_reservation_edges.rs']]
owned += ['migrations/0286_inventory_location_reservations.sql']
windows = {}
for stage in ['focused-v2', 'full']:
    before = read_json(f'free-erp-inventory-consumers-location-reservations-{stage}-before-source.json')['files']
    after = read_json(f'free-erp-inventory-consumers-location-reservations-{stage}-after-source.json')['files']
    changes = sorted(name for name in before.keys() | after.keys() if before.get(name) != after.get(name))
    unstable_owned = [name for name in owned if before.get(name) != after.get(name)]
    windows[stage] = {'status': 'PARTIAL' if changes else 'PASS', 'inputs': len(before),
                     'changed_inputs': changes, 'changed_owned_inputs': unstable_owned,
                     'owned_after_sha256': {name: after.get(name) for name in owned}}
    if unstable_owned:
        raise SystemExit('owned source changed during a qualified test window')
focused = read_json(f'{prefix}-focused-v2-independent.json')
full = read_json(f'{prefix}-full-independent.json')
if focused['status'] != 'PASS' or full['status'] != 'PASS':
    raise SystemExit('independent protected readback has not passed')
original = log(f'{prefix}-before-repair.log')
failed = log(f'{prefix}-focused.log')
clippy = log(f'{prefix}-strict-clippy.log')
clippy_counts = {f'{name} ({target})': int(count) for name, target, count in
                 re.findall(r'could not compile `([^`]+)` \(([^)]+)\).*?due to (\d+) previous errors?', clippy)}
owned_diagnostics = []
for match in re.finditer(r'(?m)^error(?:\[[^\]]+\])?:[^\n]*', clippy):
    block = clippy[match.start():match.start()+2000].split('\nerror', 1)[0]
    if any(name.replace('/', '\\') in block or name in block for name in owned):
        owned_diagnostics.append(block[:700])
fmt = log(f'{prefix}-fmt-v2.log')
if 'Nine-leaf fixed-entry fmt check PASS' not in fmt:
    raise SystemExit('fixed-entry format check did not pass')
historical = [f'{prefix}-{name}' for name in ['before-repair.log','focused.log','focused-v2.log','full.log',
              'focused-v2-independent.json','full-independent.json','migration.log','migration-v2.log',
              'qa-migration.sql','qa-migration-v2.sql','qa-context-check.sql','context-check.log',
              'context-probe.sql','context-probe.json','migration-independent.json',
              'schema-readback.sql','schema.json','strict-clippy.log','fmt-v2.log','openapi-inputs.json','openapi-bundle.log','openapi-gate.log']]
silent = {f'{prefix}-migration-v2.log',f'{prefix}-context-check.log'}
missing = [name for name in historical if not (base/name).exists()]
if set(missing)-silent:
    raise SystemExit('missing required evidence: '+str(missing))
historical = [name for name in historical if (base/name).exists()]
schema=read_json(f'{prefix}-schema.json')
if (schema['database'],schema['role'],schema['host'],schema['port']) != ('erp_shared_contract_test','erp_grid_test','127.0.0.1',15438):
    raise SystemExit('schema identity differs')
report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'PARTIAL',
          'scope': 'Formal GOOD source reserve and partial release on canonical stock; original command readback. Not complete I02, sales/production consumption or launch.',
          'before_repair': {'passed': 0, 'failed': 2, 'seconds': .79,
                           'counterexamples': ['Canonical inbound followed by valid source reserve returned 409 from the shared-stock guard.',
                           'Structured release omitted original ID but returned 200 and released a legacy source.'],
                           'actual_result_found': '0 passed; 2 failed;' in original},
          'first_focused_attempt': {'passed': 7, 'failed': 2, 'seconds': 9.27,
                                  'actual_result_found': '7 passed; 2 failed;' in failed,
                                  'followup': 'Header ID still entered generic response caching; added transactional-handler bypass. Owned second SKU distinguishes source conflict from out-of-enterprise 403.'},
          'focused': focused, 'full': full,
          'schema': {'migration': 'migrations/0286_inventory_location_reservations.sql',
                     'target': 'erp_grid_test@127.0.0.1:15438/erp_shared_contract_test',
                     'old_facts': read_json(f'{prefix}-migration-independent.json'),
                     'initial_attempt': 'Failed composite shared-ledger foreign key; original transaction rolled back. v2 applied, then matching non-null receipt constraint refinement applied.',
                     'nullable_history': 'Six new source columns stay NULL; zero invented sources/events.',
                     'schema_readback':schema,
                     'silent_success_logs_absent':missing,
                     'null_context_probe': read_json(f'{prefix}-context-probe.json')},
          'source_windows': windows,
          'format': {'status': 'PASS', 'leaf_files': 9, 'entry': 'scripts/Invoke-ErpBuild.ps1'},
          'strict_clippy': {'status': 'FAIL' if clippy_counts or 'Cargo clippy failed' in clippy else 'UNKNOWN',
                            'package_error_counts': clippy_counts, 'observed_owned_diagnostics': owned_diagnostics,
                            'limit': 'No owned diagnostic does not mean dependent packages or unvisited API lint passed.'},
          'openapi': read_json(f'{prefix}-openapi-inputs.json'),
          'owned_current_sha256': {name: digest(root / name) for name in owned},
          'historical_sha256': {f'qa/acceptance/{name}': digest(base / name) for name in historical},
          'remaining_gates': ['Real password login, account/seat/multi-device sessions and current source query/UI.',
                              'Whole sales/work-order transitions, source consumption and legacy writer/log atomic migration.',
                              'Fractional source request input, full migration/restore and all writers.',
                              'Release source-reservation workload, lock/pool/SQL/commit/HTTP percentiles and peak memory.',
                              'Desktop/Web editing, saving and commit-disconnect checkpoint recovery.',
                              'Stable complete dependency inputs, strict Clippy and official public service deployment.']}
destination.write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n', encoding='utf8')
print(json.dumps({'status': report['status'], 'full': full['registered_tests'], 'independent_checks': full['checks'],
                  'owned_inputs': len(owned), 'changed_dependency_inputs': windows['full']['changed_inputs'],
                  'clippy_package_errors': clippy_counts, 'observed_owned_diagnostics': owned_diagnostics}, ensure_ascii=False))
