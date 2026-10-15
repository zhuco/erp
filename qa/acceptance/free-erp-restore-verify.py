"""Independent catalog/row/sequence comparison and restored enforcement probes."""
import argparse
import hashlib
import json
from pathlib import Path
import runpy

ROOT = Path(__file__).resolve().parents[2]
D = runpy.run_path(str(ROOT / 'qa/acceptance/free-erp-install-restore.py'))
BASE = D['BASE']
query, literal, ident = D['query'], D['literal'], D['ident']


def fingerprint(database, label):
    names = json.loads(query(database, """SELECT json_build_object(
    'roots',(SELECT coalesce(json_agg(c.relname ORDER BY c.relname),'[]'::json)
    FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
    WHERE n.nspname='public' AND c.relkind IN ('r','p') AND NOT c.relispartition),
    'sequences',(SELECT coalesce(json_agg(c.relname ORDER BY c.relname),'[]'::json)
    FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
    WHERE n.nspname='public' AND c.relkind='S'));""", label=label + '-names'))
    statements = [(ROOT / 'qa/acceptance/free-erp-restore-catalog.sql').read_text(encoding='utf-8')]
    members = [f"SELECT {literal(table)} AS name,coalesce(json_agg(hash ORDER BY hash),'[]'::json) AS hashes FROM (SELECT md5(to_jsonb(t)::text) AS hash FROM public.{ident(table)} t) facts" for table in names['roots']]
    for offset in range(0, len(members), 8):
        statements.append('SELECT json_object_agg(name,hashes) FROM (' + '\nUNION ALL\n'.join(members[offset:offset + 8]) + ') facts;')
    for name in names['sequences']:
        statements.append(f"SELECT json_build_object('name',{literal(name)},'last_value',last_value,'is_called',is_called) FROM public.{ident(name)};")
    # Metadata, all columns/all rows, and sequence states share one read-only snapshot.
    lines = query(database, '\n'.join(statements), label=label + '-snapshot').splitlines()
    catalog = json.loads(lines[0])
    if catalog['roots'] != names['roots'] or catalog['sequences'] != names['sequences']:
        raise RuntimeError('Object set changed while assembling snapshot')
    batches = (len(members) + 7) // 8
    rows = {}
    for line in lines[1:1 + batches]:
        part = json.loads(line)
        if rows.keys() & part.keys():
            raise RuntimeError('Duplicate data snapshot member')
        rows.update(part)
    if sorted(rows) != names['roots']:
        raise RuntimeError('Incomplete all-table data snapshot')
    sequences = [json.loads(line) for line in lines[1 + batches:]]
    if [item['name'] for item in sequences] != names['sequences']:
        raise RuntimeError('Incomplete sequence snapshot')
    value = {'catalog': catalog, 'row_hashes': rows, 'sequences': sequences}
    D['save'](label, value)
    print(label, len(rows), 'root tables;', sum(len(x) for x in rows.values()), 'rows;', len(sequences), 'sequences', flush=True)
    return value


def verify():
    from argon2 import PasswordHasher
    manifest = D['manifest']()
    before = json.loads((BASE / '06-source-before-backup.json').read_text(encoding='utf-8'))
    attempt = next((n for n in range(3, 9) if not (BASE / f'07-source-verify-{n}.json').exists()), None)
    if attempt is None:
        raise RuntimeError('Verification attempt limit; preserve evidence')
    current = fingerprint(D['INSTALL'], f'07-source-verify-{attempt}')
    restored = fingerprint(D['RESTORE'], f'08-restored-verify-{attempt}')
    canonical_checks = []
    comparable = json.loads(json.dumps(restored))
    left = current['catalog']['constraints']
    right = comparable['catalog']['constraints']
    if len(left) == len(right):
        for expected, actual in zip(left, right):
            if expected == actual:
                continue
            e = {key: value for key, value in expected.items() if key != 'definition'}
            a = {key: value for key, value in actual.items() if key != 'definition'}
            if e != a or expected['type'] != 'c':
                continue
            relation = expected['relation']
            if relation not in current['catalog']['roots']:
                continue
            definitions = []
            for database, definition in [(D['INSTALL'], expected['definition']),
                                         (D['RESTORE'], actual['definition'])]:
                # PostgreSQL itself parses and re-renders the exact CHECK. No
                # stripping parentheses or dropping terms from the comparison.
                sql = f"""CREATE TEMP TABLE qa_restore_check (LIKE public.{ident(relation)});
                ALTER TABLE qa_restore_check ADD CONSTRAINT qa_restore_check_canonical {definition};
                SELECT pg_get_constraintdef(oid,false) FROM pg_constraint
                WHERE conrelid='pg_temp.qa_restore_check'::regclass
                  AND conname='qa_restore_check_canonical';"""
                definitions.append(query(database, sql, write=True, label='canonical-' + database))
            if definitions[0] != definitions[1]:
                continue
            canonical_checks.append({'relation': relation, 'name': expected['name'],
                                     'source_raw': expected['definition'], 'restored_raw': actual['definition'],
                                     'source_reparsed': definitions[0], 'restored_reparsed': definitions[1]})
            actual['definition'] = expected['definition']
    if before != current or current != comparable:
        difference = {key: {'source_equal_before': before[key] == current[key], 'restored_equal_source': current[key] == restored[key]} for key in current}
        D['save']('09-comparison-FAIL', difference)
        raise RuntimeError('Independent full restore comparison failed')
    if canonical_checks:
        D['save'](f'10-postgresql-check-canonicalization-{attempt}', canonical_checks)
    f = manifest['fixture']
    password_hash = query(D['RESTORE'], f"SELECT password_hash FROM users WHERE tenant_id={literal(f['tenant'])} AND user_id={literal(f['user'])};", label='restored-password-hash')
    PasswordHasher().verify(password_hash, 'QA-only-v323-restore-password!2026')
    immutable_message = 'receipt disposition facts are immutable; append an explicitly linked action'
    immutable = f"""DO $$ DECLARE constraint_name text; BEGIN
     BEGIN
      UPDATE mfg_operation_report_outcomes SET snapshot='{{}}'::jsonb
       WHERE tenant_id={literal(f['tenant'])} AND report_id={literal(f['report'])};
      RAISE EXCEPTION 'immutable outcome unexpectedly accepted';
     EXCEPTION WHEN SQLSTATE 'P0001' THEN
      IF SQLERRM<>{literal(immutable_message)} THEN RAISE; END IF; END;
     BEGIN
      DELETE FROM mfg_material_document_revisions WHERE tenant_id={literal(f['tenant'])}
       AND revision_id={literal(f['revision'])};
      RAISE EXCEPTION 'immutable revision unexpectedly accepted';
     EXCEPTION WHEN SQLSTATE 'P0001' THEN
      IF SQLERRM<>{literal(immutable_message)} THEN RAISE; END IF; END;
     BEGIN
      INSERT INTO mfg_material_document_effects(tenant_id,document_id,line_id,move_id,snapshot)
       VALUES({literal(f['tenant'])},{literal(f['document'])},'00000000-0000-0000-0000-000000000001',
       '00000000-0000-0000-0000-000000000002','{{}}');
      RAISE EXCEPTION 'missing real line foreign key unexpectedly accepted';
     EXCEPTION WHEN foreign_key_violation THEN
      GET STACKED DIAGNOSTICS constraint_name=CONSTRAINT_NAME;
      IF constraint_name<>'mfg_material_document_effects_tenant_id_document_id_line_i_fkey' THEN RAISE; END IF;
      PERFORM set_config('erp.qa_restore_fk_constraint',constraint_name,true);
     END;
     END $$; SELECT json_build_object('immutable_sqlstate','P0001',
       'immutable_message',{literal(immutable_message)},'foreign_key_sqlstate','23503',
       'foreign_key_constraint',current_setting('erp.qa_restore_fk_constraint'));"""
    enforcement = json.loads(query(D['RESTORE'], immutable, write=True, label='restored-immutable-and-fk-probes'))
    # Verify guard rejection using real read-only connections, never modifying markers.
    guards = {}
    for name, sql in [('wrong_database', D['guard'](D['RESTORE'])),
                      ('wrong_marker', D['guard'](D['INSTALL']).replace(literal(D['MARKER']), "'invalid-marker'"))]:
        try:
            D['psql'](D['INSTALL'], 'BEGIN READ ONLY;\n' + sql + '\nCOMMIT;', 'guard-negative-' + name)
        except RuntimeError as error:
            expected = 'identity required' if name == 'wrong_database' else 'ownership marker required'
            if expected not in str(error):
                raise
            guards[name] = 'REJECTED'
        else:
            raise RuntimeError('Identity guard did not reject mismatched target')
    D['save']('09-comparison-PASS', {
        'status': 'PASS', 'migration_count': 323,
        'raw_catalog_equal': current['catalog'] == restored['catalog'],
        'all_catalog_semantically_equal': True,
        'postgresql_reparsed_check_count': len(canonical_checks),
        'all_table_rows_equal': True, 'all_sequences_equal': True,
        'source_unchanged_during_backup': True, 'restored_password_hash_verified': True,
        'restored_enforcement': enforcement,
        'identity_guard_negative_probes': guards,
        'catalog_counts': {key: len(value) for key, value in current['catalog'].items() if isinstance(value, list)},
        'root_tables': len(current['row_hashes']),
        'total_rows': sum(len(rows) for rows in current['row_hashes'].values()),
        'fixture': f, 'scope': 'same owned PostgreSQL 17.10 cluster; SQL canaries, not business API completion',
        'input_sha256': {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in
                         [ROOT / 'qa/acceptance/free-erp-install-restore.py', ROOT / 'qa/acceptance/free-erp-install-restore-seed.sql',
                          ROOT / 'qa/acceptance/free-erp-restore-catalog.sql', Path(__file__).resolve()]}})
    print('Independent all-object/data/sequence comparison and restored enforcement PASS', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['source', 'verify'])
    if parser.parse_args().phase == 'source':
        D['manifest']()
        fingerprint(D['INSTALL'], '06-source-before-backup')
    else:
        verify()
