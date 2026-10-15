\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory command QA identity required';
 end if;
end $$;
select jsonb_build_object('database',current_database(),'role',current_user,
 'host',inet_server_addr(),'port',inet_server_port(),
 'columns',(select jsonb_agg(jsonb_build_object('name',column_name,'type',data_type,'nullable',is_nullable) order by ordinal_position)
  from information_schema.columns where table_schema='public' and table_name='command_receipts'),
 'constraint',(select pg_get_constraintdef(c.oid) from pg_constraint c
  where c.conrelid='public.command_receipts'::regclass and c.conname='command_receipts_movement_context'),
 'trigger',(select pg_get_triggerdef(t.oid) from pg_trigger t
  where t.tgrelid='public.command_receipts'::regclass and t.tgname='inventory_movement_receipt_immutable'),
 'guard',(select pg_get_functiondef('public.guard_inventory_movement_command_receipt()'::regprocedure)));
commit;
