-- One quality status is one balance row. The values are rebuilt from the ledger.

create table if not exists shared_quality_balances (
  tenant_id uuid not null,
  warehouse_id uuid not null,
  location_id uuid not null,
  sku_id uuid not null,
  batch_code text not null,
  quality_status text not null,
  on_hand bigint not null,
  reserved_qty bigint not null,
  unavailable_qty bigint not null,
  available_qty bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, warehouse_id, location_id, sku_id, batch_code, quality_status)
);
