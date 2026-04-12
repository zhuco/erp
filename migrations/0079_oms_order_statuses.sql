-- 0079: OMS order status split + exception tickets

alter table orders
  add column if not exists order_status text not null default 'PENDING_REVIEW',
  add column if not exists fulfillment_status text not null default 'NOT_ALLOCATED',
  add column if not exists exception_status text not null default 'NONE';

alter table orders
  alter column review_status set default 'NOT_REVIEWED';

update orders
set review_status = case review_status
    when 'PENDING' then 'MANUAL_REVIEW'
    when 'PASSED' then 'APPROVED'
    else review_status
end;

update orders
set fulfillment_status = case oms_status
    when 'ALLOCATED' then 'ALLOCATED'
    when 'PICKING' then 'PICKING'
    when 'PACKED' then 'READY_TO_SHIP'
    when 'SHIPPED' then 'SHIPPED'
    when 'DONE' then 'SIGNED'
    else 'NOT_ALLOCATED'
end;

update orders
set exception_status = case
    when exception_flags ? 'BLACKLISTED_CUSTOMER' then 'CUSTOMER_BLACKLIST'
    when exception_flags ? 'ADDRESS_INVALID' then 'ADDRESS_ERROR'
    when exception_flags ? 'INVENTORY_SHORTAGE' then 'OUT_OF_STOCK'
    when exception_flags ? 'DUPLICATE_ORDER' then 'RISK'
    when exception_flags ? 'REVIEW_FLAGGED' then 'MANUAL_HOLD'
    when jsonb_array_length(exception_flags) > 0 then 'MANUAL_HOLD'
    else 'NONE'
end;

update orders
set order_status = case
    when oms_status = 'CANCELLED' then 'CANCELLED'
    when oms_status = 'DONE' then 'COMPLETED'
    when oms_status = 'SHIPPED' then 'SHIPPED'
    when oms_status = 'PICKING' then 'PICKING'
    when oms_status = 'PACKED' then 'PENDING_OUTBOUND'
    when oms_status = 'ALLOCATED' then 'PENDING_OUTBOUND'
    when oms_status = 'PENDING_SHIPMENT' then 'PENDING_ALLOCATE'
    when review_status = 'REJECTED' then 'REVIEW_REJECTED'
    when jsonb_array_length(exception_flags) > 0 then 'EXCEPTION_HOLD'
    else 'PENDING_REVIEW'
end;

alter table shipments
  add column if not exists shipment_status text not null default 'REQUESTED';

update shipments
set shipment_status = status;

create table if not exists order_exception_tickets (
  ticket_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  reason_code text not null,
  status text not null default 'OPEN',
  handler_id uuid,
  handled_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, order_id, reason_code)
);

create index if not exists idx_order_exception_tickets_tenant_status
  on order_exception_tickets(tenant_id, status, updated_at desc);
create index if not exists idx_order_exception_tickets_order
  on order_exception_tickets(tenant_id, order_id);
