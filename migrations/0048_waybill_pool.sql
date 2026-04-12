-- 0045: Waybill pool state machine

alter table if exists waybills
  drop constraint if exists waybills_tenant_id_tracking_no_key;

alter table if exists waybills
  drop column if exists platform,
  drop column if exists source;

alter table if exists waybills
  rename column tracking_no to waybill_no;

alter table if exists waybills
  rename column allocated_at to reserved_at;

alter table if exists waybills
  alter column reserved_at drop default,
  alter column reserved_at drop not null;

alter table if exists waybills
  alter column status set default 'AVAILABLE';

alter table if exists waybills
  add column if not exists shipment_id uuid references shipments(shipment_id);

update waybills
set status = 'AVAILABLE'
where status in ('ALLOCATED', 'NEW');

update waybills
set reserved_at = null
where status = 'AVAILABLE';

create unique index if not exists idx_waybills_tenant_carrier_no
  on waybills(tenant_id, carrier_code, waybill_no);

create index if not exists idx_waybills_tenant_status
  on waybills(tenant_id, status);
