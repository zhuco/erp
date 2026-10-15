-- Stock movements append ledger rows. A balance is rebuilt from those rows and is not overwritten.

create table if not exists shared_stock_ledgers (
  tenant_id uuid not null,
  ledger_id uuid not null,
  warehouse_id uuid not null,
  location_id uuid not null,
  sku_id uuid not null,
  batch_code text not null,
  quality_status text not null,
  bucket text not null,
  delta_micros bigint not null,
  source_line text not null default '',
  request_id text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, ledger_id)
);

alter table shared_stock_ledgers add column if not exists source_line text not null default '';
alter table shared_stock_ledgers add column if not exists request_id text not null default '';
