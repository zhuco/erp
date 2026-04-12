-- 0037: Printing output + logistics config/tracking

alter table if exists print_jobs
  add column if not exists output_url text,
  add column if not exists output_content text,
  add column if not exists attempts int not null default 0,
  add column if not exists last_error text not null default '';

alter table if exists print_jobs
  alter column status set default 'PENDING';

update print_jobs
set status = 'PENDING'
where status = 'QUEUED';

update print_jobs
set status = 'DONE'
where status = 'SUCCESS';

create index if not exists idx_shipments_tenant_tracking
  on shipments(tenant_id, tracking_no);

create table if not exists carriers (
  tenant_id uuid not null references tenants(tenant_id),
  carrier_code text not null,
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, carrier_code)
);

create index if not exists idx_carriers_tenant_code
  on carriers(tenant_id, carrier_code);

create table if not exists shop_carrier_accounts (
  account_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shop_id uuid not null references shops(shop_id),
  carrier_code text not null,
  account_meta_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, shop_id, carrier_code)
);

create index if not exists idx_shop_carrier_accounts_tenant_shop
  on shop_carrier_accounts(tenant_id, shop_id);

create table if not exists shipment_tracking_events (
  event_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  shipment_id uuid not null references shipments(shipment_id),
  tracking_no text not null,
  status text not null,
  description text,
  occurred_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create index if not exists idx_shipment_tracking_events_tenant_tracking
  on shipment_tracking_events(tenant_id, tracking_no, occurred_at desc);
