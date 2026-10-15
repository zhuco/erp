-- Durable tasks keep task, event, and effect identities apart.
-- A lease is a conditional claim. Cancel of a posted action requires a reversal.

create table if not exists durable_tasks (
  tenant_id uuid not null,
  task_id uuid not null,
  event_id text not null,
  effect_id text not null,
  status text not null,
  lease_owner text not null default '',
  lease_until timestamptz,
  progress integer not null default 0,
  heartbeat_at timestamptz,
  attempt_count integer not null default 0,
  next_retry_seconds bigint not null default 0,
  cancel_requested boolean not null default false,
  error_text text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, task_id),
  unique (tenant_id, event_id),
  unique (tenant_id, effect_id)
);

create table if not exists durable_task_letters (
  tenant_id uuid not null,
  letter_id uuid not null,
  task_id uuid not null,
  event_id text not null,
  effect_id text not null,
  status text not null,
  error_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, letter_id)
);
