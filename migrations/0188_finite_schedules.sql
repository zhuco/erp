-- Finite-capacity schedules are separate from the free manual load warning.
-- A published plan must not rewrite an operation that has already started.

create table if not exists finite_schedule_runs (
  tenant_id uuid not null,
  run_id uuid not null,
  algorithm text not null,
  seed_text text not null,
  cutoff_text text not null,
  budget_ms bigint not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, run_id)
);

create table if not exists finite_schedule_slots (
  tenant_id uuid not null,
  slot_id uuid not null,
  run_id uuid not null,
  machine_code text not null,
  operation_code text not null,
  start_minute bigint not null,
  duration_minutes bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, slot_id)
);

create table if not exists finite_schedule_facts (
  tenant_id uuid not null,
  fact_id uuid not null,
  operation_code text not null,
  started boolean not null,
  start_minute bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, fact_id)
);
