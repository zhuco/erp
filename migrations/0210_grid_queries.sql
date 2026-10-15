-- A grid query page is not the full match.
-- Selection names the current page, the cross-page rows, or one frozen snapshot.

create table if not exists grid_queries (
  tenant_id uuid not null,
  query_id uuid not null,
  page_code text not null,
  filter_text text not null,
  sort_key text not null,
  tie_break text not null,
  page_offset bigint not null,
  page_limit integer not null,
  loaded_count bigint not null,
  matched_count bigint not null,
  count_kind text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, query_id)
);

create table if not exists grid_selections (
  tenant_id uuid not null,
  selection_id uuid not null,
  query_id uuid not null,
  selection_kind text not null,
  row_id text not null,
  snapshot_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, selection_id)
);

create table if not exists grid_batch_gates (
  tenant_id uuid not null,
  gate_id uuid not null,
  current_query text not null,
  requested_query text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, gate_id)
);
