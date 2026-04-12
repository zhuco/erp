-- 0072: Order merge/freeze exception reasons

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'ORDER_FROZEN', '冻结订单', 'REVIEW'
from tenants
on conflict (tenant_id, code) do nothing;

insert into exception_reasons (tenant_id, code, label, category)
select tenant_id, 'ORDER_MERGED', '订单合并', 'REVIEW'
from tenants
on conflict (tenant_id, code) do nothing;
