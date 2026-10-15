-- Zoom, focus, and platform results stay with one page instance.
-- An unsupported feature records the real result. A simulation is not a manual result.

create table if not exists zoom_states (
  tenant_id uuid not null,
  zoom_id uuid not null,
  page_code text not null,
  percent integer not null,
  row_id text not null,
  draft_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, zoom_id)
);

create table if not exists access_nodes (
  tenant_id uuid not null,
  node_id uuid not null,
  control_id text not null,
  role_code text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, node_id)
);

create table if not exists platform_results (
  tenant_id uuid not null,
  result_id uuid not null,
  feature_code text not null,
  decision text not null,
  source_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, result_id)
);
