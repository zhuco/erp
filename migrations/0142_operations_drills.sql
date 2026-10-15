-- Drills name an owner and a stop action. Finished tasks may be archived.
-- Unknown commands, recovery artifacts, and business rows stay.

create table if not exists operations_drills (
  tenant_id uuid not null,
  drill_id uuid not null,
  kind text not null,
  owner_user_id text not null,
  stop_action text not null,
  status text not null,
  due boolean not null,
  previous_date text not null,
  today text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, drill_id)
);

create table if not exists task_archives (
  tenant_id uuid not null,
  object_kind text not null,
  object_id text not null,
  status text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, object_kind, object_id)
);
