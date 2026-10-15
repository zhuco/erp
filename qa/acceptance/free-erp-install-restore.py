"""Fixed, owned PostgreSQL QA only. Never drops a database or deletes a backup."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance/install-restore-v323'
BIN = Path('G:/PostgreSQL/17/bin')
INSTALL = 'erp_free_v323_install_qa'
RESTORE = 'erp_free_v323_restore_qa'
DATABASES = {INSTALL, RESTORE}
MARKER = 'ERP owned v323 install/restore QA 2026-10-10; no production'
CONNECTION = ['-h', '127.0.0.1', '-p', '15438', '-U', 'erp_grid_test', '-w']


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


def ident(value):
    return '"' + value.replace('"', '""') + '"'


def save(name, value):
    with (BASE / (name + '.json')).open('x', encoding='utf-8') as file:
        json.dump(value, file, ensure_ascii=False, indent=2)
        file.write('\n')


def sources():
    return {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted((ROOT / 'migrations').glob('*.sql'))}


def guard(database):
    if database not in DATABASES | {'postgres'}:
        raise RuntimeError('Unregistered QA database')
    marker = '' if database == 'postgres' else f"""
 IF (SELECT shobj_description(oid,'pg_database') FROM pg_database
     WHERE datname=current_database()) IS DISTINCT FROM {literal(MARKER)} THEN
 RAISE EXCEPTION 'install/restore QA ownership marker required'; END IF;"""
    return f"""DO $$ BEGIN
 IF current_database()<>{literal(database)} OR current_user<>'erp_grid_test'
 OR inet_server_addr()<>'127.0.0.1'::inet OR inet_server_port()<>15438
 OR lower(replace(current_setting('data_directory'),chr(92),'/'))<>'g:/erp/qa/postgres-grid'
 THEN RAISE EXCEPTION 'install/restore QA identity required'; END IF;
 {marker} END $$;"""


def run(tool, args, sql=None, label='command'):
    environment = os.environ.copy()
    environment.update(PGCLIENTENCODING='UTF8', PGCONNECT_TIMEOUT='5',
                       PGAPPNAME='erp-v323-install-restore-qa')
    result = subprocess.run([str(BIN / (tool + '.exe'))] + args,
                            input=sql, encoding='utf-8', capture_output=True,
                            env=environment, timeout=1200)
    if BASE.exists():
        for attempt in range(1, 33):
            path = BASE / f'{label}-{attempt}.log'
            if not path.exists():
                with path.open('x', encoding='utf-8') as file:
                    file.write(f'exit_code={result.returncode}\n')
                    file.write(result.stdout + '\nSTDERR\n' + result.stderr)
                break
        else:
            raise RuntimeError('QA diagnostic attempt limit')
    if result.returncode:
        raise RuntimeError(f'{tool} failed: {result.stderr[-1500:]}')
    return result.stdout.strip()


def psql(database, sql, label):
    return run('psql', ['-X', '-qAt', '-v', 'ON_ERROR_STOP=1'] + CONNECTION
               + ['-d', database], sql, label)


def query(database, sql, write=False, label='query'):
    begin = 'BEGIN;' if write else 'BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;'
    return psql(database, begin + '\n' + guard(database) + '\n' + sql + '\nCOMMIT;', label)


def manifest():
    value = json.loads((BASE / '00-manifest.json').read_text(encoding='utf-8'))
    if value['migrations'] != sources() or len(value['migrations']) != 323:
        raise RuntimeError('Migration source changed; preserve all evidence')
    for database in DATABASES:
        query(database, 'SELECT 1;', label='admission-' + database)
    return value


def prepare():
    if BASE.exists():
        raise RuntimeError('Refuse a new preparation over existing QA evidence')
    for parent in [ROOT, ROOT / 'qa', ROOT / 'qa/acceptance']:
        if parent.is_symlink() or parent.is_junction():
            raise RuntimeError('QA output path has a reparse point')
    identity = json.loads(query('postgres', """SELECT json_build_object(
      'database',current_database(),'role',current_user,'port',inet_server_port(),
      'data_directory',current_setting('data_directory'),'version',version());""",
                               label='preparation-identity'))
    existing = json.loads(query('postgres', f"SELECT coalesce(json_agg(datname),'[]'::json) FROM pg_database WHERE datname IN ({literal(INSTALL)},{literal(RESTORE)});", label='preparation-names'))
    if existing:
        raise RuntimeError('Refuse an existing target database')
    expected = sources()
    if len(expected) != 323:
        raise RuntimeError('This fixed acceptance targets exactly 323 migration files')
    BASE.mkdir()
    values = {'identity': identity, 'databases': sorted(DATABASES), 'marker': MARKER,
              'migrations': expected,
              'fixture': {key: str(uuid.uuid4()) for key in
                          ['tenant', 'user', 'product', 'sku', 'warehouse', 'shop',
                           'order', 'report', 'document', 'revision']},
              'tool_sha256': {name: hashlib.sha256((BIN / (name + '.exe')).read_bytes()).hexdigest()
                             for name in ['psql', 'pg_dump', 'pg_restore']}}
    save('00-manifest', values)
    for database in [INSTALL, RESTORE]:
        # CREATE DATABASE cannot run in a transaction. Endpoint guard runs first.
        psql('postgres', guard('postgres') + f'\nCREATE DATABASE {ident(database)} OWNER erp_grid_test TEMPLATE template0;', 'create-' + database)
        query('postgres', f'COMMENT ON DATABASE {ident(database)} IS {literal(MARKER)};', write=True, label='mark-' + database)
        empty = query(database, "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public';", label='empty-' + database)
        if empty != '0':
            raise RuntimeError('Target was not created empty')
    save('01-prepare', {'status': 'PASS', 'original_targets_absent': True, 'empty_databases': sorted(DATABASES)})
    print('Two owned empty QA databases created', flush=True)


def installed(database):
    present = query(database, "SELECT to_regclass('public.qa_acceptance_migration_progress') IS NOT NULL;", label='ledger-present-' + database)
    if present == 'f':
        return {}
    return json.loads(query(database, "SELECT coalesce(json_object_agg(filename,sha256),'{}'::json) FROM qa_acceptance_migration_progress;", label='ledger-' + database))


def install(rerun=False):
    value = manifest()
    before = installed(INSTALL)
    if any(value['migrations'].get(name) != digest for name, digest in before.items()):
        raise RuntimeError('Unknown or changed installed migration checksum')
    sql = [guard(INSTALL), "SET statement_timeout='30s'; SELECT pg_advisory_lock(hashtextextended('erp-v323-install-qa',0)); RESET statement_timeout;",
           "CREATE TABLE IF NOT EXISTS qa_acceptance_migration_progress (filename text PRIMARY KEY,sha256 text NOT NULL,applied_at timestamptz NOT NULL DEFAULT now());"]
    todo = []
    for name, digest in value['migrations'].items():
        if name in before:
            continue
        todo.append(name)
        body = (ROOT / 'migrations' / name).read_text(encoding='utf-8-sig')
        sql.extend([f'\\echo MIGRATION_START {name} {digest}', 'BEGIN;', guard(INSTALL),
                    "SET LOCAL lock_timeout='10s'; SET LOCAL statement_timeout='120s';", body,
                    f'INSERT INTO qa_acceptance_migration_progress(filename,sha256) VALUES({literal(name)},{literal(digest)});',
                    'COMMIT;', f'\\echo MIGRATION_PASS {name}'])
    sql.append("SELECT pg_advisory_unlock(hashtextextended('erp-v323-install-qa',0));")
    name = 'rerun' if rerun else 'install'
    with (BASE / (name + '-apply.sql')).open('x', encoding='utf-8') as file:
        file.write('\n'.join(sql))
    psql(INSTALL, '\n'.join(sql), name + '-apply')
    after = installed(INSTALL)
    if after != value['migrations'] or sources() != value['migrations']:
        raise RuntimeError('Applied manifest/source mismatch')
    save('02-rerun' if rerun else '02-install', {'status': 'PASS', 'installed': len(after), 'previously_applied': len(before), 'applied_this_run': len(todo), 'source_drift': []})
    print(name, len(after), 'installed;', len(todo), 'new applications', flush=True)


def seed():
    from argon2 import PasswordHasher
    value = manifest()
    if installed(INSTALL) != value['migrations']:
        raise RuntimeError('Complete fresh schema required')
    f = {key: literal(item) for key, item in value['fixture'].items()}
    sql = (ROOT / 'qa/acceptance/free-erp-install-restore-seed.sql').read_text(encoding='utf-8')
    for key, text in f.items():
        sql = sql.replace('@' + key + '@', text)
    sql = sql.replace('@password_hash@', literal(PasswordHasher().hash('QA-only-v323-restore-password!2026')))
    query(INSTALL, sql, write=True, label='seed-real-parent-canaries')
    save('03-seed', {'status': 'PASS', 'fixture': value['fixture'], 'scope': 'direct SQL restoration canaries; no API business-effect claim'})
    print('Real ERP parent and immutable receipt canaries seeded', flush=True)


def dump():
    manifest()
    path = BASE / 'v323-install.dump'
    if path.exists():
        raise RuntimeError('Refuse to overwrite a backup')
    run('pg_dump', CONNECTION + ['--format=custom', '--file', str(path), '--dbname', INSTALL], label='custom-backup')
    toc = run('pg_restore', ['--list', str(path)], label='backup-toc')
    save('04-backup', {'status': 'PASS', 'path': str(path), 'bytes': path.stat().st_size,
                       'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'toc_entries': len(toc.splitlines()),
                       'owners_and_acls_retained': True, 'old_backup_cleanup': False})
    print('Custom backup retained', path.stat().st_size, 'bytes', flush=True)


def restore():
    manifest()
    backup = json.loads((BASE / '04-backup.json').read_text(encoding='utf-8'))
    path = Path(backup['path'])
    if hashlib.sha256(path.read_bytes()).hexdigest() != backup['sha256']:
        raise RuntimeError('Backup changed')
    empty = query(RESTORE, "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public';", label='restore-target-empty')
    if empty != '0':
        raise RuntimeError('Refuse a populated restore target')
    run('pg_restore', CONNECTION + ['--exit-on-error', '--single-transaction', '--dbname', RESTORE, str(path)], label='restore-single-transaction')
    if installed(RESTORE) != sources():
        raise RuntimeError('Restored migration ledger mismatch')
    save('05-restore', {'status': 'PASS', 'target': RESTORE, 'single_transaction': True, 'clean_option': False, 'migration_count': 323})
    print('Independent empty target restored with owner/ACL metadata', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['prepare', 'install', 'rerun', 'seed', 'dump', 'restore'])
    phase = parser.parse_args().phase
    if phase == 'rerun':
        install(True)
    else:
        globals()[phase]()
