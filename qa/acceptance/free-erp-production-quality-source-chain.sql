\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438
  or exists(select 1 from pg_namespace where nspname='qa_inspection_source_chain') then
  raise exception 'protected rollback-only inspection chain fixture required';
 end if;
end $$;
create schema qa_inspection_source_chain;
set local search_path=qa_inspection_source_chain,pg_catalog;
do $$ declare name text; begin
 foreach name in array array['command_receipts','mfg_work_orders','mfg_receipts','shared_sales_revisions',
  'shared_inspection_templates','shared_inspection_freezes','shared_inspection_freeze_lines',
  'mfg_quality_releases','mfg_quality_items'] loop
  execute format('create table %I(like public.%I including defaults including generated including indexes)',name,name);
 end loop;
end $$;
insert into command_receipts(tenant_id,request_id,payload_hash,status,effect_id)
 select gen_random_uuid(),'QA-UNCOMPRESSED-LARGE-COMMAND',string_agg(md5(n::text),'')::text,
  'SUCCEEDED',string_agg(md5((n+10000)::text),'')::text from generate_series(1,1000) n;
\ir ../../migrations/0291_inspection_source_bindings.sql
\ir ../../migrations/0292_inspection_command_identity.sql
\ir ../../migrations/0293_inspection_bounded_command_keys.sql
do $$ begin
 if not exists(select 1 from command_receipts where octet_length(payload_hash)=32000 and octet_length(effect_id)=32000)
  or exists(select 1 from pg_indexes where schemaname='qa_inspection_source_chain' and tablename='command_receipts'
   and(indexdef like '%payload_hash%' or indexdef like '%effect_id%'))
  or exists(select 1 from information_schema.columns where table_schema='qa_inspection_source_chain'
   and table_name='mfg_quality_items' and column_name='work_order_id' and is_nullable<>'YES') then
  raise exception 'canonical command text was truncated, indexed, or sales ownership was fabricated';
 end if;
end $$;
select jsonb_build_object('database',current_database(),'role',current_user,'host',inet_server_addr(),
 'port',inet_server_port(),'scope','rollback-only 0291-0293 subset chain; not complete ERP chain',
 'canonical_payload_bytes',(select max(octet_length(payload_hash)) from command_receipts),
 'canonical_effect_bytes',(select max(octet_length(effect_id)) from command_receipts),
 'constraints',(select jsonb_agg(jsonb_build_object('table',c.conrelid::regclass::text,'name',c.conname,
 'definition',pg_get_constraintdef(c.oid),'validated',c.convalidated) order by c.conname)
 from pg_constraint c join pg_namespace n on n.oid=c.connamespace where n.nspname='qa_inspection_source_chain'),
 'indexes',(select jsonb_agg(indexdef order by indexname) from pg_indexes where schemaname='qa_inspection_source_chain'),
 'triggers',(select jsonb_agg(pg_get_triggerdef(t.oid) order by t.tgname) from pg_trigger t
 join pg_class c on c.oid=t.tgrelid join pg_namespace n on n.oid=c.relnamespace
 where n.nspname='qa_inspection_source_chain' and not t.tgisinternal));
rollback;
