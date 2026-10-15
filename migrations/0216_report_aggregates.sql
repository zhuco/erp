-- A report total names its scope. Page is not the full query.
-- A chart consumes that same result. Folding does not change the total.

create table if not exists report_aggregates (
  tenant_id uuid not null,
  aggregate_id uuid not null,
  query_no text not null,
  scope_code text not null,
  dimension_code text not null,
  total_qty bigint not null,
  formula_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, aggregate_id)
);

create table if not exists report_tree_nodes (
  tenant_id uuid not null,
  node_id uuid not null,
  query_no text not null,
  level_no integer not null,
  node_code text not null,
  parent_code text not null,
  total_qty bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, node_id)
);

create table if not exists report_charts (
  tenant_id uuid not null,
  chart_id uuid not null,
  query_no text not null,
  total_qty bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, chart_id)
);

create table if not exists report_group_exports (
  tenant_id uuid not null,
  export_id uuid not null,
  query_no text not null,
  scope_code text not null,
  exported_qty bigint not null,
  sql_qty bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, export_id)
);
