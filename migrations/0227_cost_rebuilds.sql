-- An independent cost rebuild is stored beside the ledger. It does not rewrite history.

create table if not exists shared_cost_rebuilds (
  tenant_id uuid not null,
  rebuild_id uuid not null,
  item_code text not null,
  qty_micros bigint not null,
  value_cent bigint not null,
  unit_cent bigint not null,
  residual_cent bigint not null default 0,
  estimated boolean not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, rebuild_id)
);
