"""Fixed-purpose fresh-install QA database, official SQL in filename order.

Never drops/reinitializes a database or overwrites a successful migration hash.
Duplicate numeric prefixes are retained; filenames are the migration identity.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance'
PSQL = 'G:/PostgreSQL/17/bin/psql.exe'
arguments = argparse.ArgumentParser()
arguments.add_argument('--fresh-install', action='store_true')
args = arguments.parse_args()
DATABASE = 'erp_free_install_acceptance' if args.fresh_install else 'erp_free_ui_acceptance'
PREFIX = 'free-erp-install-migration' if args.fresh_install else 'free-erp-ui-migration'
DSN = f'postgresql://erp_grid_test@127.0.0.1:15438/{DATABASE}'
MARKER = 'ERP owned free ERP install QA 2026-10-08; no production' if args.fresh_install else 'ERP owned free UI QA baseline 2026-10-08; no production'


def psql(dsn, sql):
    return subprocess.run([PSQL, '-X', '-qAt', '-v', 'ON_ERROR_STOP=1', '-d', dsn],
                          input=sql, encoding='utf-8', capture_output=True)


def quote(text):
    return "'" + text.replace("'", "''") + "'"


def guard(database):
    return f"""DO $$ BEGIN
    IF current_database()<>{quote(database)} OR current_user<>'erp_grid_test'
       OR inet_server_addr()<>'127.0.0.1'::inet OR inet_server_port()<>15438 THEN
       RAISE EXCEPTION 'isolated UI QA database identity required';
    END IF; END $$;
    """


def main():
    admin = 'postgresql://erp_grid_test@127.0.0.1:15438/erp_grid_acceptance'
    exists = psql(admin, guard('erp_grid_acceptance') + f'SELECT datname FROM pg_database WHERE datname={quote(DATABASE)};')
    if exists.returncode:
        raise RuntimeError(exists.stderr)
    if not exists.stdout.strip():
        created = psql(admin, guard('erp_grid_acceptance') + f'CREATE DATABASE {DATABASE} TEMPLATE template0; COMMENT ON DATABASE {DATABASE} IS {quote(MARKER)};')
        if created.returncode:
            raise RuntimeError(created.stderr)
    identity = psql(DSN, guard(DATABASE) + "SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database();")
    if identity.returncode or identity.stdout.strip() != MARKER:
        raise RuntimeError('existing database is not this owned QA baseline')
    bootstrap = psql(DSN, guard(DATABASE) + "CREATE TABLE IF NOT EXISTS qa_acceptance_migration_progress(filename text primary key,sha256 text not null,applied_at timestamptz not null default now());")
    if bootstrap.returncode:
        raise RuntimeError(bootstrap.stderr)
    applied = psql(DSN, guard(DATABASE) + "SELECT coalesce(json_object_agg(filename,sha256),'{}'::json) FROM qa_acceptance_migration_progress;")
    if applied.returncode:
        raise RuntimeError(applied.stderr)
    history = json.loads(applied.stdout)
    inputs = BASE / 'free-erp-ui-migration-inputs'
    inputs.mkdir(exist_ok=True)
    manifest, statements = {}, [guard(DATABASE), "SET statement_timeout='60s'; SELECT pg_advisory_lock(hashtextextended('erp-free-ui-qa-migrations',0)); RESET statement_timeout;"]
    for file in sorted((ROOT / 'migrations').glob('*.sql')):
        raw = file.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        manifest[file.name] = digest
        if file.name in history:
            if history[file.name] != digest:
                raise RuntimeError('applied migration SHA changed: ' + file.name)
            continue
        snapshot = inputs / (file.name + '.' + digest + '.txt')
        if not snapshot.exists():
            snapshot.write_bytes(raw)
        elif snapshot.read_bytes() != raw:
            raise RuntimeError('migration snapshot mismatch')
        statements.extend([f'\\echo MIGRATION_START {file.name} {digest}', 'BEGIN;',
                           'SET LOCAL lock_timeout=\'10s\'; SET LOCAL statement_timeout=\'120s\';',
                           raw.decode('utf-8-sig'),
                           f'INSERT INTO qa_acceptance_migration_progress(filename,sha256) VALUES({quote(file.name)},{quote(digest)});',
                           'COMMIT;', f'\\echo MIGRATION_PASS {file.name}'])
    (BASE / (PREFIX + '-manifest.json')).write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    script = BASE / (PREFIX + '-apply.sql')
    script.write_text('\n'.join(statements), encoding='utf-8')
    result = psql(DSN, script.read_text(encoding='utf-8'))
    suffix = 'pass' if result.returncode == 0 else 'failed'
    # Preserve each distinct migration input run, including original errors.
    run_hash = hashlib.sha256(script.read_bytes()).hexdigest()[:16]
    (BASE / f'{PREFIX}-{suffix}-{run_hash}.log').write_text(result.stdout + '\n' + result.stderr, encoding='utf-8')
    after = psql(DSN, guard(DATABASE) + "SELECT coalesce(json_object_agg(filename,sha256),'{}'::json) FROM qa_acceptance_migration_progress;")
    current = json.loads(after.stdout) if after.returncode == 0 else {}
    drift = [name for name, digest in manifest.items() if hashlib.sha256((ROOT / 'migrations' / name).read_bytes()).hexdigest() != digest]
    report = {'status': 'PASS' if result.returncode == 0 and current == manifest and not drift else 'PARTIAL',
              'database': DATABASE, 'migration_count': len(manifest), 'applied_count': len(current),
              'previously_applied': len(history), 'source_drift': drift,
              'applied_sha256': current, 'scope': 'fresh QA install only; no old database upgrade, restore, UI or production qualification',
              'last_output': result.stdout.splitlines()[-6:], 'error': result.stderr[-4000:]}
    (BASE / (PREFIX + '-status.json')).write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(report['status'], report['applied_count'], '/', report['migration_count'], 'migrations')
    if result.returncode:
        print(result.stderr[-1400:])
        raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()
