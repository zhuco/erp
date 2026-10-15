-- A trade order can be confirmed without a BOM.
-- The confirm row does not create a production demand.

create table if not exists shared_sales_boms (
  tenant_id uuid not null,
  bom_id uuid not null,
  so_id uuid not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, bom_id)
);

create table if not exists shared_trade_confirms (
  tenant_id uuid not null,
  confirm_id uuid not null,
  so_id uuid not null,
  line_no integer not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, confirm_id)
);
