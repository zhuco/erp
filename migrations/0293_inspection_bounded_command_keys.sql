-- Forward repair for the first QA-only 0292 draft; all original command text is retained.
-- A fresh 0291/0292 chain already has the final shape and can safely run this migration.
do $$ declare c record; begin
 for c in select conrelid::regclass as relation,conname from pg_constraint
  where contype='f' and confrelid='command_receipts'::regclass
  and conrelid in('shared_inspection_source_bindings'::regclass,'mfg_quality_release_proofs'::regclass) loop
  execute format('alter table %s drop constraint %I',c.relation,c.conname);
 end loop;
end $$;
drop index if exists inspection_original_command_key;
drop index if exists inspection_command_identity;
drop index if exists inspection_binding_command_identity;
drop index if exists inspection_review_command_identity;
create index if not exists inspection_binding_command on shared_inspection_source_bindings(tenant_id,request_id);
create index if not exists inspection_review_command on mfg_quality_release_proofs(tenant_id,request_id);
alter table shared_inspection_source_bindings add constraint inspection_binding_original_command
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id)
 deferrable initially deferred;
alter table mfg_quality_release_proofs add constraint inspection_review_original_command
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id)
 deferrable initially deferred;
-- verify_inspection_original_command from 0292 checks exact payload, effect and SUCCEEDED at commit.
