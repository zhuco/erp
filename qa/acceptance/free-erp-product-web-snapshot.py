"""Source-window and direct SQL facts for the owned complete UI QA fixture."""
import hashlib
import json
import runpy
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
helpers = runpy.run_path(str(ROOT / 'qa/acceptance/free-erp-product-web-fixture.py'))
fixture = json.loads(helpers['MANIFEST'].read_text(encoding='utf-8'))
phase = sys.argv[1]
if phase not in ['before', 'after', 'toolbar-before', 'toolbar-after', 'pending-before', 'pending-after']:
    raise SystemExit('unregistered snapshot phase')
tenant = helpers['literal'](fixture['tenant_id'])
tables = ['products', 'skus', 'grid_product_operations', 'operation_logs', 'audit_logs', 'auth_sessions']
facts = json.loads(helpers['query']('SELECT json_build_object(' + ','.join(
    f"'{table}',(SELECT coalesce(json_agg(to_jsonb(t)),'[]'::json) FROM {table} t WHERE tenant_id={tenant})" for table in tables) + ');'))
files = set()
for base in [ROOT / 'crates', ROOT / 'apps/erp-ui/src']:
    for suffix in ['*.rs', '*.toml', '*.js']:
        files.update(base.rglob(suffix))
files.update((ROOT / 'migrations').glob('*.sql'))
files.update([ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo/config.toml',
                               'apps/erp-ui/Cargo.toml', 'apps/erp-ui/index.html',
                               'apps/erp-ui/pkg/erp_ui.js', 'apps/erp-ui/pkg/erp_ui_bg.wasm',
                               'build/cargo/debug/erp-api.exe', 'scripts/Invoke-ErpBuild.ps1', 'scripts/ErpBuildStorage.ps1']])
report = {'phase': phase, 'database': 'erp_free_ui_acceptance', 'tenant_id': fixture['tenant_id'],
          'facts': facts, 'source_sha256': {file.relative_to(ROOT).as_posix(): hashlib.sha256(file.read_bytes()).hexdigest() for file in sorted(files) if file.is_file()}}
output = ROOT / f'qa/acceptance/free-erp-product-web-oct8-{phase}.json'
if output.exists():
    raise SystemExit('refusing to overwrite source-window evidence')
output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(phase, len(report['source_sha256']), 'source/artifact hashes;', len(facts['products']), 'products')
