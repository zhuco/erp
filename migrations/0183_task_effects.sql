-- Original task effects stay addressable after a worker restart. A success banner is not this row.
create table if not exists task_effects (
  tenant_id uuid not null,
  effect_id uuid not null,
  exception_id uuid not null,
  object_type text not null,
  object_id text not null,
  command_id text not null,
  query_generation bigint not null,
  effect_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, effect_id),
  unique (tenant_id, exception_id)
);
