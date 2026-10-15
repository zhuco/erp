-- payload_hash stores the original canonical payload in several existing commands.
-- Keep that complete value, but do not put an unbounded payload in a B-tree key.
-- Both the first QA draft and a fresh installation end at the same two-key command FK.
do $$ declare c record; begin
 for c in select conrelid::regclass as relation,conname from pg_constraint
  where contype='f' and confrelid='command_receipts'::regclass
  and conrelid in('shared_inspection_source_bindings'::regclass,'mfg_quality_release_proofs'::regclass) loop
  execute format('alter table %s drop constraint %I',c.relation,c.conname);
 end loop;
end $$;
drop index if exists inspection_original_command_key;
drop index if exists inspection_command_identity;
drop index if exists inspection_binding_command;
drop index if exists inspection_review_command;
drop index if exists inspection_binding_command_identity;
drop index if exists inspection_review_command_identity;
create index inspection_binding_command on shared_inspection_source_bindings(tenant_id,request_id);
create index inspection_review_command on mfg_quality_release_proofs(tenant_id,request_id);
alter table shared_inspection_source_bindings add constraint inspection_binding_original_command
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id)
 deferrable initially deferred;
alter table mfg_quality_release_proofs add constraint inspection_review_original_command
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id)
 deferrable initially deferred;

create function verify_inspection_original_command() returns trigger language plpgsql as $$
begin
 if not exists(select 1 from command_receipts c where c.tenant_id=new.tenant_id
  and c.request_id=new.request_id and c.effect_id=new.command_effect_id
  and c.payload_hash=new.payload_hash and c.status='SUCCEEDED') then
  raise exception 'inspection original successful command and full canonical payload differ';
 end if;
 return new;
end $$;
create constraint trigger inspection_binding_original_payload after insert on shared_inspection_source_bindings
 deferrable initially deferred for each row execute function verify_inspection_original_command();
create constraint trigger inspection_review_original_payload after insert on mfg_quality_release_proofs
 deferrable initially deferred for each row execute function verify_inspection_original_command();

-- Restore the documented 0166 ownership shape: a sales reading has no fabricated work order.
alter table mfg_quality_items alter column work_order_id drop not null;
alter table mfg_quality_items add constraint inspection_reading_owner_shape
 check(work_order_id is not null or(source_kind='SALES_ORDER' and freeze_id is not null and source_id is not null))
 not valid;

-- New review proofs must own the actual approval's receipt, work order and inspector.
create function verify_inspection_review_parent() returns trigger language plpgsql as $$
begin
 if not exists(select 1 from mfg_quality_releases r where r.tenant_id=new.tenant_id
  and r.release_id=new.release_id and r.receipt_id=new.receipt_id and r.work_order_id=new.work_order_id
  and r.inspector=new.snapshot->>'inspector' and r.result='RELEASED') then
  raise exception 'inspection proof does not own the actual quality approval';
 end if;
 return new;
end $$;
create constraint trigger inspection_review_parent_complete after insert on mfg_quality_release_proofs
 deferrable initially deferred for each row execute function verify_inspection_review_parent();
