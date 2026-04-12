-- 0082: Courier defaults and status

alter table couriers
  add column if not exists status text not null default 'ACTIVE';

create index if not exists idx_couriers_tenant_status
  on couriers(tenant_id, status);

with default_couriers(code, name, status) as (
  values
    ('SF', '顺丰速运', 'ACTIVE'),
    ('YTO', '圆通速递', 'ACTIVE'),
    ('ZTO', '中通快递', 'ACTIVE'),
    ('STO', '申通快递', 'ACTIVE'),
    ('YD', '韵达快递', 'ACTIVE'),
    ('EMS', '邮政EMS', 'ACTIVE'),
    ('JD', '京东物流', 'ACTIVE'),
    ('JTX', '极兔速递', 'ACTIVE'),
    ('DB', '德邦快递', 'ACTIVE'),
    ('BEST', '百世快递', 'ACTIVE'),
    ('ZJS', '宅急送', 'DISABLED'),
    ('TTK', '天天快递', 'DISABLED'),
    ('UC', '优速快递', 'DISABLED'),
    ('ANE', '安能快递', 'DISABLED'),
    ('KYS', '跨越速运', 'DISABLED'),
    ('POST', '邮政快递包裹', 'DISABLED'),
    ('YZPY', '邮政平邮', 'DISABLED'),
    ('SUNING', '苏宁物流', 'DISABLED'),
    ('GTO', '国通快递', 'DISABLED'),
    ('DHL', 'DHL', 'DISABLED'),
    ('FEDEX', 'FedEx', 'DISABLED'),
    ('UPS', 'UPS', 'DISABLED'),
    ('TNT', 'TNT', 'DISABLED')
)
insert into couriers (tenant_id, name, code, status)
select tenants.tenant_id, default_couriers.name, default_couriers.code, default_couriers.status
from tenants
cross join default_couriers
on conflict (tenant_id, code) do nothing;
