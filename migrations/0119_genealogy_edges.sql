-- Actual genealogy edges. A planned BOM row is not a source action.
create table if not exists mfg_genealogy_edges (
  tenant_id uuid not null,
  edge_id uuid not null,
  source_action text not null,
  input_code text not null,
  output_code text not null,
  qty_micros bigint not null,
  work_order_no text not null,
  output_batch text not null,
  customer_code text not null,
  reversed boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, edge_id)
);

create index if not exists idx_mfg_genealogy_input
  on mfg_genealogy_edges(tenant_id, input_code, edge_id);

create index if not exists idx_mfg_genealogy_output
  on mfg_genealogy_edges(tenant_id, output_batch, edge_id);
