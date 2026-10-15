\set ON_ERROR_STOP on
begin;
do $$ begin
  if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
    or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
    raise exception 'protected receipt value QA identity required';
  end if;
end $$;
\i migrations/0298_purchase_receipt_fingerprint_timezone.sql
select proconfig from pg_proc where oid='receipt_value_legacy_facts(uuid,uuid)'::regprocedure;
set local time zone 'Asia/Shanghai';
select 'Shanghai',count(*) filter(where l.legacy_facts->'fingerprint'=receipt_value_legacy_facts(l.tenant_id,l.quality_id))
  from shared_receipt_cost_lots l where l.kind='LEGACY_BALANCE';
set local time zone 'UTC';
select 'UTC',count(*) filter(where l.legacy_facts->'fingerprint'=receipt_value_legacy_facts(l.tenant_id,l.quality_id))
  from shared_receipt_cost_lots l where l.kind='LEGACY_BALANCE';
commit;
