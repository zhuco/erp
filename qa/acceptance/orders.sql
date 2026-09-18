\set ON_ERROR_STOP on
-- Add-only order fixtures. Never resets existing edits or the product fixture.
begin;
do $$ begin
  if current_database() <> 'erp_grid_acceptance' or inet_server_port() <> 15438 then
    raise exception 'Order fixture requires the isolated ERP acceptance database';
  end if;
  if not exists (select 1 from users where user_id='a1100000-0000-4000-8000-000000000030'
    and tenant_id='a1100000-0000-4000-8000-000000000001'
    and email='qa.acceptance@example.invalid') then
    raise exception 'Existing QA identity is missing';
  end if;
end $$;

insert into permissions (tenant_id,code,name,module,description)
select 'a1100000-0000-4000-8000-000000000001',code,code,'OMS','QA order grid acceptance'
from (values ('OMS.ORDER.READ'),('OMS.ORDER.WRITE')) p(code)
on conflict (tenant_id,code) do nothing;
insert into role_permissions (tenant_id,role_id,perm_code)
select 'a1100000-0000-4000-8000-000000000001','a1100000-0000-4000-8000-000000000020',code
from (values ('OMS.ORDER.READ'),('OMS.ORDER.WRITE')) p(code)
on conflict do nothing;

insert into order_index (tenant_id,order_id,order_month,platform,platform_order_id,shop_id,order_created_at)
select 'a1100000-0000-4000-8000-000000000001',
 ('a1200000-0000-4000-8000-'||lpad(n::text,12,'0'))::uuid, date '2026-09-01',
 'taobao','QA-ORDER-'||lpad(n::text,3,'0'),'a1100000-0000-4000-8000-000000000010',
 timestamptz '2026-09-10 00:00:00+00' - n * interval '1 minute'
from generate_series(1,12) n on conflict do nothing;

insert into orders (tenant_id,order_month,order_id,platform,shop_id,platform_order_id,
 status,oms_status,order_status,fulfillment_status,exception_status,review_status,version,
 total_amount_cent,goods_amount_cent,paid_amount_cent,discount_amount_cent,order_created_at,raw)
select 'a1100000-0000-4000-8000-000000000001',date '2026-09-01',
 ('a1200000-0000-4000-8000-'||lpad(n::text,12,'0'))::uuid,'taobao',
 'a1100000-0000-4000-8000-000000000010','QA-ORDER-'||lpad(n::text,3,'0'),
 'PENDING_SHIPMENT','REVIEW_PENDING','PENDING_REVIEW','NOT_ALLOCATED','NONE','MANUAL_REVIEW',1,
 1250,1250,1200,50,timestamptz '2026-09-10 00:00:00+00' - n * interval '1 minute',
 '{"qa_fixture":"order-grid"}'::jsonb
from generate_series(1,12) n on conflict do nothing;

insert into order_items (tenant_id,order_month,order_id,line_id,sku_id,title,qty,
 price_cent,amount_cent,pay_amount_cent,discount_cent,raw)
select 'a1100000-0000-4000-8000-000000000001',date '2026-09-01',
 ('a1200000-0000-4000-8000-'||lpad(n::text,12,'0'))::uuid,
 item.line_id,item.sku_id,item.title,item.qty,item.price,item.amount,item.paid,item.discount,
 jsonb_build_object('qa_fixture','order-grid','outer_sku_id',item.sku_id)
from generate_series(1,12) n cross join (values
 ('QA-LINE-1','QA-SKU-001','验收货品一',2,500,1000,950,50),
 ('QA-LINE-2','QA-SKU-002','验收货品二',1,250,250,250,0)
) item(line_id,sku_id,title,qty,price,amount,paid,discount)
on conflict do nothing;
commit;
