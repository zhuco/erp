-- Frozen export scopes. A later export reads this snapshot and cannot add rows.
create table if not exists export_scope_snapshots (
  tenant_id uuid not null,
  snapshot_id uuid not null,
  actor_user_id text not null,
  scope_kind text not null,
  query_snapshot text not null,
  source_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, snapshot_id, source_id),
  constraint export_scope_snapshots_kind_check check (scope_kind in ('QUERY', 'SELECTED', 'FROZEN_MATCH'))
);
