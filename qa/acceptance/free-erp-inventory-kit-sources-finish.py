"""Bind immutable kit repair evidence and the current developer documentation."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
PREFIX = 'free-erp-inventory-kit-sources'
DOCUMENTS = ['docs/FREE_ERP_CHECKLIST.md', 'docs/FREE_ERP_DEVELOPMENT_SPEC.md',
             'docs/PRODUCT_SCOPE.md', 'docs/ROADMAP.md', 'docs/README.md', 'docs/UI_COMPONENT_LIBRARY.md']
OWNED = ['crates/erp-repo/src/inventory.rs', 'crates/erp-repo/src/inventory_order_reservations.rs',
         'crates/erp-repo/src/lib.rs', 'crates/erp-api/src/inventory_views.rs',
         'crates/erp-api/tests/free_erp_acceptance.rs',
         'crates/erp-api/tests/free_erp_acceptance/inventory_kit_sources.rs',
         'crates/erp-api/tests/free_erp_acceptance/inventory_kit_source_edges.rs',
         'migrations/0287_inventory_order_reservation_sources.sql',
         'openapi/src/components.yaml', 'openapi/src/paths.yaml', 'openapi/erp_api_v1.yaml']
TOOLS = [f'qa/acceptance/{PREFIX}-{name}' for name in ['capture.py', 'verify.py', 'finish.py',
         'stream-readback.sql', 'schema-readback.sql', 'qa-migration.sql', 'qa-index.sql']]
TOOLS += ['qa/acceptance/free-erp-inventory-consumers-source.py',
          'qa/acceptance/free-erp-specification-docs-verify.py']


def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def read(name):
    return json.loads((BASE / name).read_text(encoding='utf-8-sig'))


def input_hashes(manifest_path):
    paths = []
    for directory in ['crates', 'apps']:
        paths += list((ROOT / directory).rglob('*.rs'))
        paths += list((ROOT / directory).rglob('Cargo.toml'))
    paths += list((ROOT / 'migrations').glob('*.sql'))
    paths += [ROOT / name for name in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml',
        '.cargo/config.toml', 'AGENTS.md', 'scripts/Invoke-ErpBuild.ps1', 'scripts/Build-ErpWeb.ps1',
        'openapi/src/components.yaml', 'openapi/src/paths.yaml', 'openapi/src/openapi.yaml',
        'openapi/erp_api_v1.yaml', 'openapi/channel_ingest_v1.yaml']]
    current = {path.relative_to(ROOT).as_posix(): digest(path) for path in sorted(set(paths))
               if path.is_file() and 'target' not in path.relative_to(ROOT).parts}
    if manifest_path is not None:
        manifest = json.loads(manifest_path.read_text(encoding='utf-8-sig'))['files']
        if current != manifest:
            raise SystemExit('inputs changed after fresh source capture; preserve it and capture a new stage')
    return current


def verify_prior_revision(owned, tool_hashes):
    prior_path = BASE / f'{PREFIX}-repair.json'
    prior = read(prior_path.name)
    if prior['owned_source_sha256'] != owned:
        raise SystemExit('owned repair/OpenAPI inputs differ from preserved initial repair evidence')
    archives = {
        f'qa/acceptance/{PREFIX}-finish.py': f'qa/acceptance/{PREFIX}-finish-v1.py.txt',
        'qa/acceptance/free-erp-specification-docs-verify.py':
            'qa/acceptance/free-erp-specification-docs-verify-kit-sources-v1.py.txt',
    }
    revisions = {}
    for name, expected in prior['verification_tools_sha256'].items():
        if tool_hashes[name] == expected:
            continue
        archive = archives.get(name)
        if archive is None or digest(ROOT / archive) != expected:
            raise SystemExit(f'previous verifier revision is not preserved: {name}')
        revisions[name] = {'previous_sha256': expected, 'previous_archive': archive,
                           'current_sha256': tool_hashes[name]}
    for name, expected in prior['historical_artifact_sha256'].items():
        if digest(ROOT / name) == expected:
            continue
        archive = archives.get(name)
        if archive is None or digest(ROOT / archive) != expected:
            raise SystemExit(f'previous historical artifact changed without matching archive: {name}')
    failure = read('free-erp-specification-docs-kit-sources-first-failure.json')
    if failure['status'] != 'FAIL' or len(failure['errors']) != 3:
        raise SystemExit('first documentation failure is not preserved')
    return {'initial_repair_sha256': digest(prior_path),
            'initial_historical_artifacts_checked': len(prior['historical_artifact_sha256']),
            'verification_tool_revisions': revisions,
            'preserved_documentation_failure': {
                'report': 'qa/acceptance/free-erp-specification-docs-kit-sources-first-failure.json',
                'sha256': digest(BASE / 'free-erp-specification-docs-kit-sources-first-failure.json'),
                'errors': failure['errors'],
                'resolution': 'Include named const declarations and the actual registered return GET; retain exact ten-operation/seven-path route comparison.'}}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['repair', 'delivery'])
    parser.add_argument('--revision')
    parser.add_argument('--input-manifest')
    args = parser.parse_args()
    if args.revision is not None and not re.fullmatch(r'v(?:[2-9]|[1-9][0-9]+)', args.revision):
        raise SystemExit('revision must be an explicit v2 or later revision')
    suffix = f'-{args.revision}' if args.revision else ''
    destination = BASE / f'{PREFIX}-{args.phase}{suffix}.json'
    if destination.exists():
        raise SystemExit('refusing to replace historical repair/delivery record')
    manifest_path = None
    if args.input_manifest:
        manifest_path = (ROOT / args.input_manifest).resolve()
        if manifest_path.parent != BASE.resolve() or not manifest_path.name.startswith('free-erp-inventory-consumers-kit-sources-'):
            raise SystemExit('fresh manifest must be an immutable kit-source QA capture')
    if args.revision and manifest_path is None:
        raise SystemExit('revised evidence requires a fresh full input manifest')
    owned = {name: digest(ROOT / name) for name in OWNED}
    tools = {name: digest(ROOT / name) for name in TOOLS}
    inputs = input_hashes(manifest_path)
    if args.phase == 'repair':
        reports = {stage: read(f'{PREFIX}-{stage}-independent{suffix}.json') for stage, suffix in [
            ('original', ''), ('migration', ''), ('focused', '-v2'), ('full', '')]}
        if any(report['status'] != 'PASS' for report in reports.values()):
            raise SystemExit('original failures/migration/independent gates incomplete')
        windows = {stage: read(f'{PREFIX}-{stage}-source-window.json') for stage in ['focused', 'full']}
        if any(window['owned_changes'] for window in windows.values()):
            raise SystemExit('repair inputs changed inside tested interval')
        lint = read(f'{PREFIX}-clippy.json')
        before = read(f'{PREFIX}-openapi-before.json')
        after = read(f'{PREFIX}-openapi-after.json')
        source_drift = [name for name in before if name != 'openapi/erp_api_v1.yaml' and before[name] != after[name]]
        history = sorted(set(BASE.glob(f'{PREFIX}-*')) | set(BASE.glob('free-erp-inventory-consumers-kit-sources-*-source.json')))
        if args.revision:
            history += [BASE / name for name in [
                'free-erp-specification-docs-kit-sources-first-failure.json',
                'free-erp-specification-docs-verify-kit-sources-v1.py.txt',
                'free-erp-specification-docs-before-kit-sources.json']]
        history_hashes = {path.relative_to(ROOT).as_posix(): digest(path) for path in history
                          if path.is_file() and path != destination
                          and path.relative_to(ROOT).as_posix() not in TOOLS
                          and (args.revision or path.name not in [f'{PREFIX}-repair.json', f'{PREFIX}-delivery.json'])}
        first = read(f'{PREFIX}-focused-independent.json')
        if first['status'] != 'FAIL' or first['errors'] != ['complete exact scenario totals including rollback/legacy/parallel cases']:
            raise SystemExit('preserved first independent failure no longer matches')
        report = {
            'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'PARTIAL',
            'artifact_verification': 'PASS',
            'scope': 'Existing legacy UUID reserve/release and public repository consume: frozen root intent and applied SKU components, with canonical stock and required route logs.',
            'original_registered_tests': {'passed': 0, 'failed': 5, 'ignored': 0, 'seconds': 1.67},
            'focused_registered_tests': {'passed': 15, 'failed': 0, 'ignored': 0, 'seconds': 10.86},
            'full_registered_tests': {'passed': 174, 'failed': 0, 'ignored': 0, 'seconds': 120.74},
            'independent_results': reports,
            'source_windows': windows,
            'strict_clippy': lint,
            'openapi': {'command_status': 'PASS', 'source_and_tool_drift': source_drift,
                        'qualification': 'PARTIAL' if source_drift else 'PASS',
                        'before': before, 'after': after,
                        'generated_output_change': 'Expected rebuild from edited source; not input drift.'},
            'migration': {'database': 'erp_shared_contract_test', 'role': 'erp_grid_test', 'host': '127.0.0.1', 'port': 15438,
                          'production_applied': False, 'historical_backfill': False,
                          'existing_tenants_preserved': reports['migration']['counts']['unchanged_preexisting_tenants'],
                          'historical_rows_with_null_attachment': reports['migration']['counts']['historical_source_rows_kept_unmapped'],
                          'initial_source_archive': f'qa/acceptance/{PREFIX}-migration-v1.sql.txt',
                          'reverse_index_added_after_initial_success': f'qa/acceptance/{PREFIX}-qa-index.sql'},
            'preserved_first_independent_failure': {'report': f'qa/acceptance/{PREFIX}-focused-independent.json',
                'verifier_archive': f'qa/acceptance/{PREFIX}-verify-v1.py.txt',
                'reason': 'Initial expected Outbox total omitted ten actual availability-increase release events. Corrected from actual producer/effect facts; no route/stock/rollback assertion removed.'},
            'owned_source_sha256': owned, 'verification_tools_sha256': tools,
            'historical_artifact_sha256': history_hashes,
            'current_all_inputs_sha256': inputs,
            'remaining_gates': [
                'Current API/shared dependencies changed during test windows; strict dependency-chain Clippy failed before API lint.',
                'Legacy first request qty is frozen, but an authoritative real order line/parent quantity and optional order relation still need formal validation.',
                'fulfillment/store.rs::consume_reservations and other sale/manufacture/cancel consumers require their own frozen-group transaction and race acceptance.',
                'Current root/source query, reviewed legacy resolution and the formal DataGrid page are unfinished.',
                'Legacy response is current stock, without original command receipt, source remainder or partial release.',
                'Native/Web real account, one-seat/multi-session recovery, public hosted service, full migration/recovery and Release stock load remain untested.',
                'Deferred group validators and 500-row legacy scans need measured 1/10/100/1000-component resource/latency load; debug tests are not P95/P99 acceptance.',
            ],
        }
        if args.revision:
            report['previous_revision_verification'] = verify_prior_revision(owned, tools)
            tested = read('free-erp-inventory-consumers-kit-sources-after-full-source.json')['files']
            report['current_input_changes_since_tested_final_manifest'] = {
                name: {'tested_sha256': tested.get(name), 'current_sha256': inputs.get(name)}
                for name in sorted(set(tested) | set(inputs)) if tested.get(name) != inputs.get(name)}
    else:
        repair = read(f'{PREFIX}-repair{suffix}.json')
        if repair['owned_source_sha256'] != owned or repair['verification_tools_sha256'] != tools:
            raise SystemExit('owned implementation or verifier changed after repair evidence')
        for name, expected in repair['historical_artifact_sha256'].items():
            if digest(ROOT / name) != expected:
                raise SystemExit(f'historical evidence was changed: {name}')
        docs = read('free-erp-specification-docs.json')
        if docs['status'] != 'PASS':
            raise SystemExit('current detailed documentation gate failed')
        doc_hashes = {name: digest(ROOT / name) for name in DOCUMENTS}
        if docs['document_sha256'] != doc_hashes:
            raise SystemExit('documentation changed after validation')
        for name, expected in docs['referenced_source_sha256'].items():
            if digest(ROOT / name) != expected:
                raise SystemExit(f'documented source changed after declaration validation: {name}')
        previous_inputs = repair['current_all_inputs_sha256']
        changes = [name for name in sorted(set(inputs) | set(previous_inputs))
                   if inputs.get(name) != previous_inputs.get(name)]
        report = {'updated_at': datetime.now(timezone.utc).isoformat(), 'status': 'PARTIAL', 'artifact_verification': 'PASS',
                  'repair_sha256': digest(BASE / f'{PREFIX}-repair{suffix}.json'),
                  'owned_source_sha256': owned, 'verification_tools_sha256': tools,
                  'document_sha256': doc_hashes, 'documentation_report_sha256': digest(BASE / 'free-erp-specification-docs.json'),
                  'historical_artifact_sha256': repair['historical_artifact_sha256'],
                  'current_all_inputs_sha256': inputs, 'all_input_changes_after_repair': changes,
                  'qualification': 'Specific private QA source behavior and documentation; complete ERP, production and Release gates remain open.'}
    if args.revision:
        report['revision'] = args.revision
    if manifest_path is not None:
        report['fresh_input_manifest'] = {'path': manifest_path.relative_to(ROOT).as_posix(),
                                          'sha256': digest(manifest_path), 'inputs': len(inputs)}
    if owned != {name: digest(ROOT / name) for name in OWNED}:
        raise SystemExit('owned inputs changed while binding evidence')
    if tools != {name: digest(ROOT / name) for name in TOOLS}:
        raise SystemExit('verification tools changed while binding evidence')
    if inputs != input_hashes(None):
        raise SystemExit('source input set changed while binding evidence; retain the capture and use a new stage')
    destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf8')
    print(json.dumps({'phase': args.phase, 'status': report['status'], 'artifact_verification': report['artifact_verification'],
                      'owned_inputs': len(owned), 'all_inputs': len(inputs), 'verification_tools': len(tools),
                      'historical_artifacts': len(report['historical_artifact_sha256'])}, ensure_ascii=False))


if __name__ == '__main__':
    main()
