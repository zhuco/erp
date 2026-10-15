-- A location move keeps the stock quantity and owner on the same location id.
-- It does not insert a transfer.

create table if not exists shared_location_owners (
  tenant_id uuid not null,
  location_id uuid not null,
  sku_id uuid not null,
  owner_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, location_id, sku_id)
);

create table if not exists shared_location_transfers (
  tenant_id uuid not null,
  transfer_id uuid not null,
  location_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, transfer_id)
);
