-- 0091: Repair courier defaults data for existing environments.
-- This migration is idempotent and normalizes default courier names/status.

alter table couriers
  add column if not exists status text not null default 'ACTIVE';

create index if not exists idx_couriers_tenant_status
  on couriers(tenant_id, status);

with default_couriers(code, name, status) as (
  values
    ('SF', 'SF Express', 'ACTIVE'),
    ('YTO', 'YTO Express', 'ACTIVE'),
    ('ZTO', 'ZTO Express', 'ACTIVE'),
    ('STO', 'STO Express', 'ACTIVE'),
    ('YD', 'Yunda Express', 'ACTIVE'),
    ('EMS', 'EMS', 'ACTIVE'),
    ('JD', 'JD Logistics', 'ACTIVE'),
    ('JTX', 'J&T Express', 'ACTIVE'),
    ('DB', 'Deppon Express', 'ACTIVE'),
    ('BEST', 'Best Express', 'ACTIVE'),
    ('ZJS', 'ZJS Express', 'DISABLED'),
    ('TTK', 'TTK Express', 'DISABLED'),
    ('UC', 'UC Express', 'DISABLED'),
    ('ANE', 'ANE Logistics', 'DISABLED'),
    ('KYS', 'KYS Express', 'DISABLED'),
    ('POST', 'China Post Parcel', 'DISABLED'),
    ('YZPY', 'China Post Economy', 'DISABLED'),
    ('SUNING', 'Suning Logistics', 'DISABLED'),
    ('GTO', 'GTO Express', 'DISABLED'),
    ('DHL', 'DHL', 'DISABLED'),
    ('FEDEX', 'FedEx', 'DISABLED'),
    ('UPS', 'UPS', 'DISABLED'),
    ('TNT', 'TNT', 'DISABLED')
)
insert into couriers (tenant_id, name, code, status)
select t.tenant_id, d.name, d.code, d.status
from tenants t
cross join default_couriers d
on conflict (tenant_id, code) do update
set
  name = excluded.name,
  status = excluded.status;
