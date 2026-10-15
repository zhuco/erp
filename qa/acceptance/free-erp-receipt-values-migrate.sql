\set ON_ERROR_STOP on
begin;
do $$ begin
  if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
    or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
    raise exception 'protected receipt value QA identity required';
  end if;
  if to_regclass('shared_receipt_value_commands') is not null then
    raise exception 'receipt value schema exists; inspect original migration evidence';
  end if;
end $$;
\i migrations/0297_purchase_receipt_value_sources.sql
select current_database(),current_user,inet_server_addr(),inet_server_port();
select tablename from pg_tables where schemaname='public' and tablename in
  ('shared_receipt_value_commands','shared_receipt_cost_lots','shared_receipt_cost_allocations') order by tablename;
select proname from pg_proc where proname in
  ('receipt_value_legacy_facts','receipt_value_integrity','keep_receipt_value_original','check_receipt_value_commit') order by proname;
commit;
