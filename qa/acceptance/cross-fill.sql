\set ON_ERROR_STOP on
-- Add-only, separate from the existing product/SKU acceptance fixtures.
begin;
do $$ begin
  if current_database() <> 'erp_grid_acceptance' or inet_server_port() <> 15438 then
    raise exception 'Cross-fill fixture requires isolated ERP acceptance database';
  end if;
  if not exists (select 1 from users where user_id='a1100000-0000-4000-8000-000000000030'
      and email='qa.acceptance@example.invalid'
      and tenant_id='a1100000-0000-4000-8000-000000000001') then
    raise exception 'QA identity missing';
  end if;
end $$;
insert into products (product_id,tenant_id,product_code,name,brand,category,base_unit,status)
select ('a1300000-0000-4000-8000-'||lpad(n::text,12,'0'))::uuid,
 'a1100000-0000-4000-8000-000000000001','QA-FILL-'||lpad(n::text,3,'0'),
 'QA fill product '||n,'QA fill brand '||n,'QA fill','个','ACTIVE'
from generate_series(1,24) n
on conflict do nothing;
commit;
