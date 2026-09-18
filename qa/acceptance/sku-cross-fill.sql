\set ON_ERROR_STOP on
-- Add-only synthetic SKU fixture; does not reset existing acceptance data.
begin;
do $$ begin
  if current_database() <> 'erp_grid_acceptance' or inet_server_port() <> 15438 then
    raise exception 'SKU cross-fill requires isolated acceptance database';
  end if;
  if not exists (select 1 from users where user_id='a1100000-0000-4000-8000-000000000030'
    and email='qa.acceptance@example.invalid' and tenant_id='a1100000-0000-4000-8000-000000000001') then
    raise exception 'QA identity missing';
  end if;
end $$;
insert into products (product_id,tenant_id,product_code,name,brand,category,base_unit,status)
values ('a1400000-0000-4000-8000-000000000001','a1100000-0000-4000-8000-000000000001',
 'QA-SKU-FILL','QA SKU cross-page fill','QA','QA SKU fill','个','ACTIVE')
on conflict do nothing;
insert into skus (sku_id,tenant_id,product_id,sku_code,barcode,name,spec,cost_cent,status,is_active)
select ('a1410000-0000-4000-8000-'||lpad(n::text,12,'0'))::uuid,
 'a1100000-0000-4000-8000-000000000001','a1400000-0000-4000-8000-000000000001',
 'QA-SF-'||lpad(n::text,3,'0'),'QA-SF-BC-'||n,'QA SKU fill name '||n,'{}'::jsonb,
 1000+n*100,'ACTIVE',true
from generate_series(1,24) n
on conflict do nothing;
commit;
