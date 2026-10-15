-- A trade return cites one reviewed delivery. Good stock is usable; rejected stock is not.
-- The credit uses the original unit cost. No platform shop is created.

create table if not exists shared_sales_returns (
  tenant_id uuid not null,
  return_id uuid not null,
  delivery_id uuid not null,
  good_micros bigint not null,
  rejected_micros bigint not null,
  cost_cent bigint not null,
  credit_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, return_id)
);
