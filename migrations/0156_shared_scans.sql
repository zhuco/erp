-- A scan is a verified intent. A print event records one distinct device state.

create table if not exists shared_scans (
  tenant_id uuid not null,
  scan_id uuid not null,
  code text not null,
  kind text not null,
  status text not null,
  actor_user_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, scan_id)
);

create table if not exists shared_print_events (
  tenant_id uuid not null,
  event_id uuid not null,
  job_ref text not null,
  state text not null,
  copies bigint not null,
  reason text not null default '',
  actor_kind text not null,
  actor_user_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, event_id)
);
