\set ON_ERROR_STOP on
begin;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'protected location command QA identity required';
 end if;
end $$;
create index if not exists shared_stock_location_quantity_idx
 on shared_stock_ledgers(tenant_id,warehouse_id,location_id,sku_id,batch_code,quality_status,bucket)
 include(delta_micros);
commit;
