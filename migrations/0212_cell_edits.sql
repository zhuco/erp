-- A confirmed cell change stays bound to one row and one column.
-- One document saves all of its lines or none. A batch returns one result per object.

create table if not exists cell_edits (
  tenant_id uuid not null,
  edit_id uuid not null,
  document_no text not null,
  row_id text not null,
  column_id text not null,
  unit_code text not null,
  qty_micros bigint not null,
  display_text text not null,
  confirmed boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, edit_id)
);

create table if not exists document_save_results (
  tenant_id uuid not null,
  document_no text not null,
  line_count bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, document_no)
);

create table if not exists object_batch_results (
  tenant_id uuid not null,
  object_id text not null,
  version_no integer not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, object_id)
);

create table if not exists edit_import_tasks (
  tenant_id uuid not null,
  task_id uuid not null,
  row_count bigint not null,
  budget_count bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, task_id)
);
