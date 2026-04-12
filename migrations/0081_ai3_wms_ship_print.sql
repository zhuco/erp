-- 0081: AI-3 WMS + shipment + print fields

alter table outbound_docs
  alter column outbound_status set default 'DRAFT',
  alter column pick_status set default 'NOT_PICKED',
  alter column verify_status set default 'NOT_VERIFIED',
  alter column pack_status set default 'NOT_PACKED',
  alter column ship_status set default 'NOT_SHIPPED';

update outbound_docs
set outbound_status = case status
    when 'DRAFT' then 'DRAFT'
    when 'AUDITED' then 'PENDING_ALLOCATE'
    when 'PICKING' then 'PICKING'
    when 'PICKED' then 'PENDING_VERIFY'
    when 'PACKING' then 'PENDING_PACK'
    when 'PACKED' then 'PENDING_SHIP'
    when 'INSPECTED' then 'PENDING_SHIP'
    when 'CHECKED' then 'PENDING_SHIP'
    when 'WEIGHED' then 'PENDING_SHIP'
    when 'SHIPPED' then 'SHIPPED'
    when 'CANCELLED' then 'CLOSED'
    else 'DRAFT'
end
where outbound_status is null or outbound_status = '';

update outbound_docs
set pick_status = case
    when status = 'PICKING' then 'PICKING'
    when status in ('PICKED','PACKING','PACKED','INSPECTED','CHECKED','WEIGHED','SHIPPED')
        then 'PICKED'
    else 'NOT_PICKED'
end
where pick_status is null or pick_status = '';

update outbound_docs
set verify_status = case
    when status in ('INSPECTED','CHECKED','WEIGHED','SHIPPED') then 'VERIFIED'
    else 'NOT_VERIFIED'
end
where verify_status is null or verify_status = '';

update outbound_docs
set pack_status = case
    when status = 'PACKING' then 'PACKING'
    when status in ('PACKED','INSPECTED','CHECKED','WEIGHED','SHIPPED') then 'PACKED'
    else 'NOT_PACKED'
end
where pack_status is null or pack_status = '';

update outbound_docs
set ship_status = case
    when status = 'SHIPPED' then 'SHIPPED'
    else 'NOT_SHIPPED'
end
where ship_status is null or ship_status = '';

update outbound_docs
set picked_at = updated_at
where picked_at is null
  and status in ('PICKED','PACKING','PACKED','INSPECTED','CHECKED','WEIGHED','SHIPPED');

update outbound_docs
set packed_at = updated_at
where packed_at is null
  and status in ('PACKED','INSPECTED','CHECKED','WEIGHED','SHIPPED');

update outbound_docs d
set shipped_at = s.shipped_at
from shipments s
where d.shipped_at is null
  and d.shipment_id = s.shipment_id
  and s.shipped_at is not null;

update outbound_docs
set shipped_at = updated_at
where shipped_at is null
  and status = 'SHIPPED';

alter table outbound_pick_tasks
  alter column status set default 'NOT_PICKED';

update outbound_pick_tasks
set status = 'NOT_PICKED'
where status = 'PENDING' or status = '';

alter table inbound_docs
  alter column inbound_status set default 'DRAFT',
  alter column receipt_status set default 'NOT_RECEIVED',
  alter column inbound_qc_status set default 'NOT_REQUIRED',
  alter column putaway_status set default 'NOT_PUTAWAY';

update inbound_docs
set inbound_status = case status
    when 'DRAFT' then 'DRAFT'
    when 'AUDITED' then 'PENDING_RECEIPT'
    when 'RECEIVING' then 'RECEIVING'
    when 'PUTAWAY_DONE' then 'COMPLETED'
    when 'CANCELLED' then 'CLOSED'
    else 'DRAFT'
end
where inbound_status is null or inbound_status = '';

update inbound_docs
set receipt_status = case status
    when 'RECEIVING' then 'PART_RECEIVED'
    when 'PUTAWAY_DONE' then 'RECEIVED'
    else 'NOT_RECEIVED'
end
where receipt_status is null or receipt_status = '';

update inbound_docs
set inbound_qc_status = 'NOT_REQUIRED'
where inbound_qc_status is null or inbound_qc_status = '';

update inbound_docs
set putaway_status = case
    when status = 'PUTAWAY_DONE' then 'PUTAWAYED'
    else 'NOT_PUTAWAY'
end
where putaway_status is null or putaway_status = '';

alter table inbound_putaway_tasks
  alter column status set default 'PUTAWAYED';

update inbound_putaway_tasks
set status = 'PUTAWAYED'
where status = 'DONE' or status = '';

alter table pack_records
  add column if not exists pack_status text not null default 'PACKED',
  add column if not exists void_reason text,
  add column if not exists voided_at timestamptz;

update pack_records
set pack_status = 'PACKED'
where pack_status is null or pack_status = '';

alter table shipments
  alter column ship_status set default 'NOT_SHIPPED',
  alter column last_push_error set default '';

update shipments
set ship_status = case
    when status = 'SHIPPED' then 'SHIPPED'
    else 'NOT_SHIPPED'
end
where ship_status is null or ship_status = '';

update shipments
set last_push_error = ''
where last_push_error is null;

alter table print_jobs
  add column if not exists dispatch_status text not null default 'QUEUED';

update print_jobs
set dispatch_status = case status
    when 'PENDING' then 'QUEUED'
    when 'RUNNING' then 'DISPATCHED'
    when 'DONE' then 'PRINTED'
    when 'FAILED' then 'FAILED'
    when 'CANCELLED' then 'FAILED'
    else 'QUEUED'
end
where dispatch_status is null or dispatch_status = '';

create table if not exists wave_rules (
  rule_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  carrier_code text,
  max_orders int,
  max_qty bigint,
  cutoff_time time,
  strategy text,
  enabled boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_wave_rules_tenant_warehouse
  on wave_rules(tenant_id, warehouse_id);
create index if not exists idx_wave_rules_tenant_carrier
  on wave_rules(tenant_id, carrier_code);
