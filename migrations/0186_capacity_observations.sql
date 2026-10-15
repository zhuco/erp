-- Capacity inspection stores an observed value and the next action. It does not replace a scale test.

create table if not exists capacity_observations (
  tenant_id uuid not null,
  observation_id uuid not null,
  observed_value bigint not null,
  next_action text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, observation_id)
);
