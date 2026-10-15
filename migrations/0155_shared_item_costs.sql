-- Moving-average cost is an append-only entry. A historical issue is not rewritten.

create table if not exists shared_item_costs (
  tenant_id uuid not null,
  entry_id uuid not null,
  item_code text not null,
  kind text not null,
  qty_micros bigint not null,
  amount_cent bigint not null,
  unit_cent bigint not null,
  estimated boolean not null default false,
  algorithm text not null,
  source_ref text not null,
  period_code text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, entry_id)
);

create table if not exists shared_cost_periods (
  tenant_id uuid not null,
  period_code text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, period_code)
);
