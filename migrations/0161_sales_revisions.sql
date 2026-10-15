-- A sales revision is a snapshot. Submit does not create a production demand.
-- Fulfill keeps a stable demand line. Resale does not read a BOM.

create table if not exists shared_sales_revisions (
  tenant_id uuid not null,
  revision_id uuid not null,
  so_id uuid not null,
  version_no integer not null,
  status text not null,
  customer_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, revision_id),
  unique (tenant_id, so_id, version_no)
);

create table if not exists shared_sales_lines (
  tenant_id uuid not null,
  revision_id uuid not null,
  line_no integer not null,
  item_code text not null,
  qty_micros bigint not null,
  price_cent bigint not null,
  discount_cent bigint not null,
  tax_cent bigint not null,
  due_date text not null,
  address text not null,
  source_kind text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, revision_id, line_no)
);

create table if not exists shared_sales_demands (
  tenant_id uuid not null,
  demand_id uuid not null,
  so_id uuid not null,
  line_no integer not null,
  item_code text not null,
  qty_micros bigint not null,
  source_kind text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, demand_id),
  unique (tenant_id, so_id, line_no)
);
