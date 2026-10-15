\set ON_ERROR_STOP on
begin transaction isolation level repeatable read read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected inventory consumer QA identity required';
 end if;
end $$;
select jsonb_build_object('database',current_database(),'role',current_user,'host',inet_server_addr(),
 'port',inet_server_port(),
 'constraints',(select jsonb_agg(jsonb_build_object('name',conname,'definition',pg_get_constraintdef(oid)) order by conname)
  from pg_constraint where conrelid in ('inventory_location_reservation_events'::regclass,
   'inventory_reservations'::regclass,'command_receipts'::regclass)),
 'indexes',(select jsonb_agg(to_jsonb(i) order by indexname) from pg_indexes i
  where schemaname='public' and indexname in ('inventory_location_reservation_source_once',
   'inventory_location_reservation_events_source')),
 'functions',(select jsonb_agg(jsonb_build_object('name',proname,'definition',pg_get_functiondef(oid)) order by proname)
  from pg_proc where pronamespace='public'::regnamespace and proname in (
   'guard_inventory_location_reservation','guard_inventory_location_reservation_event',
   'verify_inventory_location_reservation_commit')),
 'triggers',(select jsonb_agg(jsonb_build_object('name',tgname,'definition',pg_get_triggerdef(oid)) order by tgname)
  from pg_trigger where not tgisinternal and tgname in (
   'inventory_location_reservation_immutable','inventory_location_reservation_event_immutable',
   'inventory_location_reservation_commit','inventory_location_reservation_event_commit')));
commit;
