\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected location command QA identity required';
 end if;
end $$;
select jsonb_build_object('database',current_database(),'role',current_user,'host',inet_server_addr(),'port',inet_server_port(),
 'location_table',to_regclass('public.warehouse_locations') is not null,
 'location_disabled',exists(select 1 from information_schema.columns where table_schema='public' and table_name='warehouse_locations' and column_name='is_disabled' and data_type='boolean'),
 'outbox_trace',exists(select 1 from information_schema.columns where table_schema='public' and table_name='outbox_events' and column_name='trace_id' and data_type='text'),
 'projection_guard',coalesce((select jsonb_build_object('enabled',t.tgenabled,'definition',pg_get_triggerdef(t.oid),'function',pg_get_functiondef(t.tgfoid)) from pg_trigger t where t.tgrelid='shared_stock_ledgers'::regclass and t.tgname='inventory_linked_shared_ledger_guard'),'null'::jsonb),
 'location_index',coalesce((select indexdef from pg_indexes where schemaname='public' and indexname='shared_stock_location_quantity_idx'),''));
commit;
