"""Copy semantics and operational failure probes against the owned upgrade clone."""
import json
from pathlib import Path
import runpy

ROOT=Path(__file__).resolve().parents[2]
D=runpy.run_path(str(ROOT/'qa/acceptance/free-erp-upgrade-v323.py'))
BASE=D['BASE'];query=D['query'];literal=D['literal'];ident=D['ident']
value=D['manifest']()
read=lambda name:json.loads((BASE/(name+'.json')).read_text(encoding='utf-8'))

source=read('01-original-v320-before');clone=read('02-clone-v320-before-seed')
comparable=json.loads(json.dumps(clone));normalized=[]
assert len(source['catalog']['constraints'])==len(clone['catalog']['constraints'])
for old,new in zip(source['catalog']['constraints'],comparable['catalog']['constraints']):
    if old==new:continue
    assert old['type']=='c' and {k:v for k,v in old.items() if k!='definition'}=={k:v for k,v in new.items() if k!='definition'}
    assert old['relation'] in clone['catalog']['roots']
    forms=[]
    for definition in [old['definition'],new['definition']]:
        forms.append(query(D['TARGET'],f"""CREATE TEMP TABLE qa_upgrade_check (LIKE public.{ident(old['relation'])});
         ALTER TABLE qa_upgrade_check ADD CONSTRAINT qa_upgrade_check_canonical {definition};
         SELECT pg_get_constraintdef(oid,false) FROM pg_constraint WHERE conrelid='pg_temp.qa_upgrade_check'::regclass AND conname='qa_upgrade_check_canonical';""",write=True,label='copy-check-reparse'))
    assert forms[0]==forms[1]
    normalized.append({'source_raw':old,'cloned_raw':dict(new),'postgresql_reparsed':forms[0]})
    new['definition']=old['definition']
assert source==comparable,'v320 copy changed facts or objects'
D['save']('08-copy-comparison',{'status':'PASS','all_original_rows_sequences_preserved':True,'all_catalog_semantically_preserved':True,'postgresql_reparsed_checks':normalized})

# Intentionally fail a transaction after DDL + an existing-row update. No
# production migration is edited, and no persistent object may survive failure.
fault=fr"""\set VERBOSITY verbose
BEGIN;
{D['guard'](D['TARGET'])}
CREATE TABLE qa_upgrade_fault_probe(id integer PRIMARY KEY);
UPDATE products SET name='FAULT MUST ROLLBACK' WHERE tenant_id={literal(value['fixture']['tenant'])};
INSERT INTO qa_acceptance_migration_progress(filename,sha256) VALUES('0001_init.sql','intentional duplicate');
COMMIT;"""
with (BASE/'transaction-fault.sql').open('x',encoding='utf-8') as file:file.write(fault)
try:D['psql'](D['TARGET'],fault,'transaction-failure-probe')
except RuntimeError as error:
    assert '23505' in str(error) and 'qa_acceptance_migration_progress_pkey' in str(error),str(error)
else:raise RuntimeError('Expected transaction failure absent')
assert query(D['TARGET'],"SELECT to_regclass('public.qa_upgrade_fault_probe') IS NULL;",label='fault-ddl-rolled-back')=='t'
post_fault=D['fingerprint'](D['TARGET'],'09-after-transaction-fault')
assert post_fault==read('05-after-rerun'),'Failure changed old facts or schema'
D['save']('09-transaction-rollback',{'status':'PASS','sqlstate':'23505','constraint':'qa_acceptance_migration_progress_pkey','ddl_and_old_row_update_rolled_back':True,'all_catalog_complete_row_sha256_and_sequences_unchanged':True})

# Controlled checksum drift is committed only in the clone, then restored by
# compare-and-swap in finally. The original baseline cannot enter this path.
key='0001_init.sql';correct=value['migrations'][key];wrong='0'*64
query(D['TARGET'],f"""DO $$ BEGIN
 UPDATE qa_acceptance_migration_progress SET sha256={literal(wrong)}
 WHERE filename={literal(key)} AND sha256={literal(correct)};
 IF NOT FOUND THEN RAISE EXCEPTION 'checksum probe precondition mismatch'; END IF;
 END $$;""",write=True,label='checksum-drift-in-own-clone')
rejected=False
try:
    try:D['upgrade'](True)
    except RuntimeError as error:
        if str(error)!='Installed checksum drift':raise
        rejected=True
    if not rejected:raise RuntimeError('Changed historical checksum accepted')
finally:
    query(D['TARGET'],f"""DO $$ BEGIN
     UPDATE qa_acceptance_migration_progress SET sha256={literal(correct)}
     WHERE filename={literal(key)} AND sha256={literal(wrong)};
     IF NOT FOUND THEN RAISE EXCEPTION 'checksum probe recovery CAS mismatch'; END IF;
     END $$;""",write=True,label='checksum-probe-recovery-cas')
assert D['ledger'](D['TARGET'])==value['migrations']
post=D['fingerprint'](D['TARGET'],'10-after-checksum-recovery')
assert post==post_fault,'Checksum probe recovery changed retained facts'
D['save']('10-checksum-refusal',{'status':'PASS','historical_checksum_drift':'REJECTED BEFORE APPLY','recovery':'same expected wrong checksum CAS','all_facts_restored':True})

try:D['psql'](D['SOURCE'],'BEGIN READ ONLY;\n'+D['guard'](D['TARGET'])+'\nCOMMIT;','wrong-database-live-guard')
except RuntimeError as error:assert 'upgrade QA identity required' in str(error)
else:raise RuntimeError('Wrong database guard did not reject')
original_after=D['fingerprint'](D['SOURCE'],'11-original-after-probes')
assert original_after==source
D['save']('11-operational-probes-PASS',{'status':'PASS','source_before_after_equal':True,'copy_preservation':True,'failed_transaction_atomic':True,'changed_history_refused':True,'live_wrong_database_refused':True,'no_source_or_business_migration_files_changed':True})
print('Original copy, full transaction rollback, history checksum rejection/recovery and source preservation PASS',flush=True)
