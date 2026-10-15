\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected kit source QA identity required';
 end if;
end $$;
select jsonb_build_object('database',current_database(),'role',current_user,'host',inet_server_addr(),'port',inet_server_port());
select jsonb_build_object(
 'columns',(select jsonb_agg(to_jsonb(c) order by table_name,ordinal_position) from information_schema.columns c
  where table_schema='public' and table_name in ('inventory_order_reservation_sources','inventory_order_reservation_components','inventory_reservations')),
 'constraints',(select jsonb_agg(jsonb_build_object('table',conrelid::regclass::text,'name',conname,'definition',pg_get_constraintdef(oid),'validated',convalidated,'deferrable',condeferrable,'initially_deferred',condeferred) order by conname)
  from pg_constraint where conrelid in ('inventory_order_reservation_sources'::regclass,'inventory_order_reservation_components'::regclass,'inventory_reservations'::regclass)),
 'triggers',(select jsonb_agg(jsonb_build_object('table',tgrelid::regclass::text,'name',tgname,'enabled',tgenabled,'definition',pg_get_triggerdef(oid)) order by tgname)
  from pg_trigger where not tgisinternal and tgrelid in ('inventory_order_reservation_sources'::regclass,'inventory_order_reservation_components'::regclass,'inventory_reservations'::regclass)),
 'functions',(select jsonb_agg(jsonb_build_object('name',proname,'definition',pg_get_functiondef(oid)) order by proname)
  from pg_proc where pronamespace='public'::regnamespace and proname like '%inventory_order_reservation%'),
 'indexes',(select jsonb_agg(to_jsonb(i) order by indexname) from pg_indexes i where schemaname='public'
  and tablename in ('inventory_order_reservation_sources','inventory_order_reservation_components','inventory_reservations')),
 'rows',jsonb_build_object('roots',(select count(*) from inventory_order_reservation_sources),'components',(select count(*) from inventory_order_reservation_components),
  'reservations',(select count(*) from inventory_reservations),'unmapped',(select count(*) from inventory_reservations where reservation_source_id is null))
);
commit;
