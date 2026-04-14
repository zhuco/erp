-- 0090: Order review runtime compatibility backfill
-- This migration is intentionally idempotent for local/dev environments
-- that may have partial historical schemas.

alter table orders
  add column if not exists order_type text,
  add column if not exists sales_channel text,
  add column if not exists order_source text,
  add column if not exists salesperson text,
  add column if not exists ship_warehouse_id uuid references warehouses(warehouse_id),
  add column if not exists shipment_no text,
  add column if not exists estimated_weight_gram int,
  add column if not exists package_weight_gram int,
  add column if not exists customer_account text,
  add column if not exists receiver_tel text,
  add column if not exists paid_amount_cent bigint,
  add column if not exists subsidy_amount_cent bigint,
  add column if not exists receivable_amount_cent bigint,
  add column if not exists other_fee_cent bigint,
  add column if not exists estimated_postage_cent bigint,
  add column if not exists invoice_type text,
  add column if not exists invoice_title text,
  add column if not exists payment_status text,
  add column if not exists freeze_reason text,
  add column if not exists cancel_reason text,
  add column if not exists exception_reason text,
  add column if not exists extra_remark text;

alter table orders
  add column if not exists order_status text not null default 'PENDING_REVIEW',
  add column if not exists fulfillment_status text not null default 'NOT_ALLOCATED',
  add column if not exists exception_status text not null default 'NONE';

alter table orders
  alter column review_status set default 'NOT_REVIEWED';

update orders
set fulfillment_status = coalesce(
  nullif(fulfillment_status, ''),
  case oms_status
    when 'ALLOCATED' then 'ALLOCATED'
    when 'PICKING' then 'PICKING'
    when 'PACKED' then 'READY_TO_SHIP'
    when 'SHIPPED' then 'SHIPPED'
    when 'DONE' then 'SIGNED'
    else 'NOT_ALLOCATED'
  end
)
where fulfillment_status is null or btrim(fulfillment_status) = '';

update orders
set exception_status = coalesce(
  nullif(exception_status, ''),
  case
    when exception_flags ? 'BLACKLISTED_CUSTOMER' then 'CUSTOMER_BLACKLIST'
    when exception_flags ? 'ADDRESS_INVALID' then 'ADDRESS_ERROR'
    when exception_flags ? 'INVENTORY_SHORTAGE' then 'OUT_OF_STOCK'
    when exception_flags ? 'DUPLICATE_ORDER' then 'RISK'
    when exception_flags ? 'REVIEW_FLAGGED' then 'MANUAL_HOLD'
    when jsonb_array_length(exception_flags) > 0 then 'MANUAL_HOLD'
    else 'NONE'
  end
)
where exception_status is null or btrim(exception_status) = '';

update orders
set order_status = coalesce(
  nullif(order_status, ''),
  case
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
  end
)
where order_status is null or btrim(order_status) = '';

alter table couriers
  add column if not exists status text not null default 'ACTIVE';

create index if not exists idx_couriers_tenant_status
  on couriers(tenant_id, status);

create table if not exists product_warehouses (
  tenant_id uuid not null references tenants(tenant_id),
  product_id uuid not null references products(product_id),
  warehouse_id uuid not null references warehouses(warehouse_id),
  is_default boolean not null default false,
  allow_negative_stock boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, product_id, warehouse_id)
);

create index if not exists idx_product_warehouses_product
  on product_warehouses(tenant_id, product_id);

create index if not exists idx_product_warehouses_warehouse
  on product_warehouses(tenant_id, warehouse_id);

