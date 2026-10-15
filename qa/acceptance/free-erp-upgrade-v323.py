"""Owned v320 clone + synthetic old-data upgrade. Original baseline stays read-only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'qa/acceptance/upgrade-v323'
BIN = Path('G:/PostgreSQL/17/bin')
SOURCE = 'erp_free_install_acceptance'
TARGET = 'erp_free_v323_upgrade_qa'
SOURCE_MARKER = 'ERP owned free ERP install QA 2026-10-08; no production'
MARKER = 'ERP owned v320-to-v323 upgrade QA 2026-10-10; no production'
CONNECTION = ['-h','127.0.0.1','-p','15438','-U','erp_grid_test','-w']


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


def ident(value):
    return '"' + value.replace('"','""') + '"'


def save(name, value):
    with (BASE / (name + '.json')).open('x',encoding='utf-8') as file:
        json.dump(value,file,ensure_ascii=False,indent=2)
        file.write('\n')


def run(tool,args,sql=None,label='command'):
    env=os.environ.copy()
    env.update(PGCLIENTENCODING='UTF8',PGCONNECT_TIMEOUT='5',PGAPPNAME='erp-v320-upgrade-qa')
    result=subprocess.run([str(BIN/(tool+'.exe'))]+args,input=sql,encoding='utf-8',capture_output=True,env=env,timeout=1200)
    if BASE.exists():
        n=next((n for n in range(1,33) if not (BASE/f'{label}-{n}.log').exists()),None)
        if n is None:raise RuntimeError('Diagnostic attempt limit')
        with (BASE/f'{label}-{n}.log').open('x',encoding='utf-8') as file:
            file.write(f'exit_code={result.returncode}\n{result.stdout}\nSTDERR\n{result.stderr}')
    if result.returncode:raise RuntimeError(f'{tool} failed: {result.stderr[-1600:]}')
    return result.stdout.strip()


def guard(database):
    if database not in {SOURCE,TARGET,'postgres'}:raise RuntimeError('Unregistered database')
    marker=SOURCE_MARKER if database==SOURCE else MARKER
    ownership='' if database=='postgres' else f"IF (SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database()) IS DISTINCT FROM {literal(marker)} THEN RAISE EXCEPTION 'upgrade QA marker required'; END IF;"
    return f"""DO $$ BEGIN
    IF current_database()<>{literal(database)} OR current_user<>'erp_grid_test'
    OR inet_server_addr()<>'127.0.0.1'::inet OR inet_server_port()<>15438
    OR lower(replace(current_setting('data_directory'),chr(92),'/'))<>'g:/erp/qa/postgres-grid'
    THEN RAISE EXCEPTION 'upgrade QA identity required'; END IF;
    {ownership} END $$;"""


def psql(database,sql,label):
    return run('psql',['-X','-qAt','-v','ON_ERROR_STOP=1']+CONNECTION+['-d',database],sql,label)


def query(database,sql,write=False,label='query'):
    if write and database==SOURCE:raise RuntimeError('Original v320 baseline is read-only')
    begin='BEGIN;' if write else 'BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;'
    return psql(database,begin+'\n'+guard(database)+'\n'+sql+'\nCOMMIT;',label)


def sources():
    return {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((ROOT/'migrations').glob('*.sql'))}


def ledger(database):
    return json.loads(query(database,"SELECT coalesce(json_object_agg(filename,sha256),'{}'::json) FROM qa_acceptance_migration_progress;",label='ledger-'+database))


def manifest():
    value=json.loads((BASE/'00-manifest.json').read_text(encoding='utf-8'))
    if value['migrations']!=sources():raise RuntimeError('Migration inputs changed')
    query(SOURCE,'SELECT 1;',label='source-admission')
    query(TARGET,'SELECT 1;',label='target-admission')
    return value


def fingerprint(database,label):
    names=json.loads(query(database,"""SELECT json_build_object(
     'roots',(SELECT json_agg(c.relname ORDER BY c.relname) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relkind IN ('r','p') AND NOT c.relispartition),
     'sequences',(SELECT json_agg(c.relname ORDER BY c.relname) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND c.relkind='S'));""",label=label+'-names'))
    statements=[(ROOT/'qa/acceptance/free-erp-restore-catalog.sql').read_text(encoding='utf-8')]
    members=[f"SELECT {literal(name)} name,coalesce(json_agg(hash ORDER BY hash),'[]'::json) hashes FROM (SELECT encode(digest(to_jsonb(t)::text,'sha256'),'hex') hash FROM public.{ident(name)} t) facts" for name in names['roots']]
    for i in range(0,len(members),8):statements.append('SELECT json_object_agg(name,hashes) FROM ('+'\nUNION ALL\n'.join(members[i:i+8])+') facts;')
    for name in names['sequences']:statements.append(f"SELECT json_build_object('name',{literal(name)},'last_value',last_value,'is_called',is_called) FROM public.{ident(name)};")
    lines=query(database,'\n'.join(statements),label=label+'-snapshot').splitlines()
    catalog=json.loads(lines[0]);assert catalog['roots']==names['roots'] and catalog['sequences']==names['sequences']
    batches=(len(members)+7)//8;rows={}
    for line in lines[1:1+batches]:
        part=json.loads(line);assert not(rows.keys()&part.keys());rows.update(part)
    assert sorted(rows)==names['roots']
    sequence_states=[json.loads(line) for line in lines[1+batches:]]
    assert [s['name'] for s in sequence_states]==names['sequences']
    value={'catalog':catalog,'row_sha256':rows,'sequences':sequence_states}
    save(label,value)
    print(label,len(rows),'tables;',sum(map(len,rows.values())),'rows',flush=True)
    return value


def prepare():
    if BASE.exists():raise RuntimeError('Preserve existing upgrade evidence')
    for p in [ROOT,ROOT/'qa',ROOT/'qa/acceptance']:
        if p.is_symlink() or p.is_junction():raise RuntimeError('Reparse output path')
    expected=sources();old=ledger(SOURCE)
    assert len(expected)==323 and len(old)==320 and all(expected.get(k)==v for k,v in old.items())
    assert query('postgres',f'SELECT count(*) FROM pg_database WHERE datname={literal(TARGET)};',label='target-absence')=='0'
    BASE.mkdir()
    save('00-manifest',{'migrations':expected,'old_migrations':old,'source':SOURCE,'target':TARGET,'marker':MARKER,
                       'fixture':{k:str(uuid.uuid4()) for k in ['tenant','user','product','sku','warehouse','shop','order','report','document']}})
    fingerprint(SOURCE,'01-original-v320-before')
    psql('postgres',guard('postgres')+f'\nCREATE DATABASE {ident(TARGET)} OWNER erp_grid_test TEMPLATE template0;','create-upgrade-target')
    query('postgres',f'COMMENT ON DATABASE {ident(TARGET)} IS {literal(MARKER)};',write=True,label='mark-upgrade-target')
    assert query(TARGET,"SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public';",label='target-empty')=='0'
    backup=BASE/'v320-source.dump'
    run('pg_dump',CONNECTION+['--format=custom','--file',str(backup),'--dbname',SOURCE],label='backup-v320-source')
    save('01-backup',{'sha256':hashlib.sha256(backup.read_bytes()).hexdigest(),'bytes':backup.stat().st_size,'path':str(backup)})
    run('pg_restore',CONNECTION+['--exit-on-error','--single-transaction','--dbname',TARGET,str(backup)],label='restore-v320-clone')
    assert ledger(TARGET)==old
    fingerprint(TARGET,'02-clone-v320-before-seed')
    save('02-copy',{'status':'PASS','original_business_rows':0,'migration_count':320,'clean':False})
    print('Read-only v320 baseline copied to an owned empty target',flush=True)


def seed():
    from argon2 import PasswordHasher
    value=manifest();assert ledger(TARGET)==value['old_migrations']
    sql=(ROOT/'qa/acceptance/free-erp-install-restore-seed.sql').read_text(encoding='utf-8').split('INSERT INTO mfg_operation_report_outcomes')[0]
    sql=sql.replace("'qa-v323-restore-draft'","'qa-v320-legacy-command'").replace("'SUCCEEDED',@document@","'SUCCEEDED',@order@")
    for key,item in value['fixture'].items():sql=sql.replace('@'+key+'@',literal(item))
    sql=sql.replace('@password_hash@',literal(PasswordHasher().hash('QA-only-v320-upgrade-password!2026')))
    with (BASE/'v320-canaries.sql').open('x',encoding='utf-8') as file:file.write(sql)
    query(TARGET,sql,write=True,label='seed-v320-existing-data')
    fingerprint(TARGET,'03-old-data-before-upgrade')
    save('03-seed',{'status':'PASS','scope':'direct SQL synthetic v320 data in clone only','fixture':value['fixture']})
    print('Old-version canaries created before any new migration',flush=True)


def upgrade(rerun=False):
    value=manifest();before=ledger(TARGET)
    if any(value['migrations'].get(k)!=v for k,v in before.items()):raise RuntimeError('Installed checksum drift')
    todo=[k for k in value['migrations'] if k not in before]
    if not rerun and todo!=['0312_material_document_drafts.sql','0313_material_document_confirmation.sql','0314_operation_report_outcomes.sql']:raise RuntimeError('Unexpected upgrade boundary')
    if rerun and todo:raise RuntimeError('Rerun requires complete upgrade')
    name='rerun' if rerun else 'upgrade'
    sql=[guard(TARGET),"SET statement_timeout='30s'; SELECT pg_advisory_lock(hashtextextended('erp-v320-upgrade-qa',0)); RESET statement_timeout;"]
    for file in todo:
        sql+=['BEGIN;',guard(TARGET),"SET LOCAL lock_timeout='10s'; SET LOCAL statement_timeout='120s';",
              (ROOT/'migrations'/file).read_text(encoding='utf-8-sig'),
              f'INSERT INTO qa_acceptance_migration_progress(filename,sha256) VALUES({literal(file)},{literal(value["migrations"][file])});','COMMIT;']
    sql.append("SELECT pg_advisory_unlock(hashtextextended('erp-v320-upgrade-qa',0));")
    with (BASE/(name+'-apply.sql')).open('x',encoding='utf-8') as file:file.write('\n'.join(sql))
    psql(TARGET,'\n'.join(sql),name+'-apply')
    assert ledger(TARGET)==value['migrations']
    save('04-'+name,{'status':'PASS','previously_installed':len(before),'applied':todo,'installed':323})
    fingerprint(TARGET,'05-after-'+name)
    print(name,'complete;',len(todo),'new migrations',flush=True)


def verify():
    from argon2 import PasswordHasher
    value=manifest()
    read=lambda name:json.loads((BASE/(name+'.json')).read_text(encoding='utf-8'))
    original=read('01-original-v320-before');original_now=fingerprint(SOURCE,'06-original-v320-after')
    assert original==original_now,'Original v320 database changed'
    old=read('03-old-data-before-upgrade');new=read('05-after-upgrade');again=read('05-after-rerun')
    assert new==again,'Rerun changed schema or data'
    assert old['sequences']==new['sequences']
    for table,hashes in old['row_sha256'].items():
        if table=='qa_acceptance_migration_progress':assert set(hashes)<=set(new['row_sha256'][table]) and len(new['row_sha256'][table])==len(hashes)+3
        else:assert hashes==new['row_sha256'][table],table
    new_tables=sorted(set(new['row_sha256'])-set(old['row_sha256']))
    assert new_tables==['mfg_material_document_effects','mfg_material_document_lines','mfg_material_document_revisions','mfg_material_documents','mfg_operation_report_outcomes']
    assert all(new['row_sha256'][table]==[] for table in new_tables),'Historical new outcomes invented'
    key_fields={'relations':['name'],'constraints':['relation','name'],'indexes':['relation','name'],
                'triggers':['relation','name'],'functions':['name','identity'],'types':['name'],
                'policies':['relation','name'],'extensions':['name'],'sequence_parameters':['schemaname','sequencename']}
    for category,keys in key_fields.items():
        mapping={tuple(item[k] for k in keys):item for item in new['catalog'][category]}
        for item in old['catalog'][category]:assert mapping[tuple(item[k] for k in keys)]==item,(category,item)
    password=query(TARGET,f"SELECT password_hash FROM users WHERE tenant_id={literal(value['fixture']['tenant'])};",label='upgraded-credential')
    PasswordHasher().verify(password,'QA-only-v320-upgrade-password!2026')
    # Read-only source prohibition is checked before any tool is invoked.
    try:query(SOURCE,'SELECT 1;',write=True,label='forbidden-source-write')
    except RuntimeError as e:assert str(e)=='Original v320 baseline is read-only'
    else:raise RuntimeError('Original write admission accepted')
    save('07-upgrade-PASS',{'status':'PASS','from':320,'to':323,'old_roots':len(old['row_sha256']),
      'new_roots':len(new['row_sha256']),'old_rows':sum(map(len,old['row_sha256'].values())),
      'new_rows':sum(map(len,new['row_sha256'].values())),'new_business_tables_empty':new_tables,
      'all_old_complete_row_sha256_preserved':True,'all_old_catalog_objects_preserved':True,
      'old_sequence_states_preserved':True,'rerun_unchanged':True,'original_v320_source_unchanged':True,
      'credential_hash_verified':True,'scope':'synthetic old-version data; original baseline has no business rows'})
    print('Old data/objects/receipts/credentials preserved; no historical outcome backfill',flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('phase',choices=['prepare','seed','upgrade','rerun','verify']);phase=p.parse_args().phase
    if phase=='rerun':upgrade(True)
    else:globals()[phase]()
