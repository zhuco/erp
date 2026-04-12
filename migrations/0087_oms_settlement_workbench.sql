-- 0087: OMS 结算与对账工作台

create table if not exists order_settlement_records (
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid not null,
  status text not null default 'OPEN',
  locked_at timestamptz,
  export_job_id text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, order_id)
);

create index if not exists idx_order_settlement_records_status
  on order_settlement_records(tenant_id, status, updated_at desc);

create table if not exists order_reconciliation_batches (
  tenant_id uuid not null references tenants(tenant_id),
  batch_id text not null,
  platform text not null,
  shop_id uuid not null,
  period_start date not null,
  period_end date not null,
  status text not null default 'PENDING',
  note text,
  settled_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (tenant_id, batch_id)
);

create index if not exists idx_order_reconciliation_batches_lookup
  on order_reconciliation_batches(tenant_id, platform, shop_id, period_start, period_end);

create index if not exists idx_order_reconciliation_batches_status
  on order_reconciliation_batches(tenant_id, status, updated_at desc);
