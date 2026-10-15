\set ON_ERROR_STOP on
begin;
do $$ begin
  if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
    or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
    raise exception 'protected receipt authority QA identity required';
  end if;
  if to_regclass('public.shared_receipt_value_actors') is not null then
    raise exception 'actor source table exists; review original migration evidence';
  end if;
end $$;
\i migrations/0299_purchase_receipt_current_actor.sql
select current_database(),current_user,inet_server_addr(),inet_server_port();
select column_name,data_type,is_nullable from information_schema.columns
  where table_schema='public' and table_name='shared_receipt_value_actors' order by ordinal_position;
select tgname,tgdeferrable,tginitdeferred,pg_get_triggerdef(oid) from pg_trigger
  where not tgisinternal and tgname in ('receipt_value_actor_original','zz_receipt_value_authority_commit') order by tgname;
select count(*) from shared_receipt_value_actors;
commit;
