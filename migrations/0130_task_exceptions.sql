-- Task-center exceptions keep tenant, object, original command, status, and next action.
-- A retry reuses the same command identity. A stale query generation cannot act on the old set.

create table if not exists task_exceptions (
  tenant_id uuid not null,
  exception_id uuid not null,
  kind text not null,
  object_type text not null,
  object_id text not null,
  command_id text not null,
  status text not null,
  next_action text not null,
  query_generation bigint not null,
  error_text text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, exception_id),
  unique (tenant_id, command_id)
);

create table if not exists task_exception_actions (
  tenant_id uuid not null,
  action_id uuid not null,
  exception_id uuid not null,
  command_id text not null,
  action text not null,
  query_generation bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, action_id)
);
