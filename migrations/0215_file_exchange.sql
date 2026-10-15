-- A file map stays with one enterprise. Typed cells are not formulas.
-- A large file is a backend task. Device print status comes from an agent.

create table if not exists file_maps (
  tenant_id uuid not null,
  map_id uuid not null,
  format_code text not null,
  source_column text not null,
  field_name text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, map_id)
);

create table if not exists file_cells (
  tenant_id uuid not null,
  cell_id uuid not null,
  file_no text not null,
  kind_code text not null,
  value_text text not null,
  encoding_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, cell_id)
);

create table if not exists file_deliveries (
  tenant_id uuid not null,
  delivery_id uuid not null,
  file_no text not null,
  row_count bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, delivery_id)
);

create table if not exists device_print_jobs (
  tenant_id uuid not null,
  job_id uuid not null,
  device_id text not null,
  document_no text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, job_id)
);
