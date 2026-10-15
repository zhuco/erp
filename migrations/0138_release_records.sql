-- A release keeps the artifact SHA, migration list, and compatibility window.
-- Rolling back the artifact does not delete a committed quantity or amount.

create table if not exists release_records (
  tenant_id uuid not null,
  release_id uuid not null,
  artifact_sha text not null,
  phase text not null,
  migration_list text not null,
  current_release integer not null,
  previous_release integer not null,
  window_end text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, release_id)
);

create table if not exists release_facts (
  tenant_id uuid not null,
  fact_id uuid not null,
  release_id uuid not null,
  amount_cent bigint not null,
  qty_micros bigint not null,
  legacy_units bigint not null,
  unit_name text not null,
  wip_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, fact_id)
);

create table if not exists release_drafts (
  tenant_id uuid not null,
  draft_id uuid not null,
  release_id uuid not null,
  fact_id uuid not null,
  client_kind text not null,
  body_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, draft_id)
);

create table if not exists release_client_results (
  tenant_id uuid not null,
  result_id uuid not null,
  release_id uuid not null,
  fact_id uuid not null,
  client_kind text not null,
  decision text not null,
  amount_cent bigint not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, result_id)
);

create table if not exists release_rollbacks (
  tenant_id uuid not null,
  rollback_id uuid not null,
  release_id uuid not null,
  artifact_sha text not null,
  fact_id uuid not null,
  amount_cent bigint not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, rollback_id)
);
