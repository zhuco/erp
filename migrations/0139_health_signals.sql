-- Health signals stay separate. Request ids are trace fields, not metric labels.
-- Heavy work has a limit. Ordinary order entry keeps reserved slots.

create table if not exists health_signals (
  tenant_id uuid not null,
  signal_id uuid not null,
  kind text not null,
  status text not null,
  observed_value bigint not null,
  threshold_value bigint not null,
  duration_seconds bigint not null,
  request_id text not null,
  command_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, signal_id)
);

create table if not exists health_alerts (
  tenant_id uuid not null,
  alert_id uuid not null,
  signal_id uuid not null,
  threshold_value bigint not null,
  duration_seconds bigint not null,
  owner_user_id text not null,
  action_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, alert_id)
);

create table if not exists work_admissions (
  tenant_id uuid not null,
  admission_id uuid not null,
  kind text not null,
  status text not null,
  retry_after_seconds bigint not null,
  command_id text not null,
  created_at timestamptz not null default now(),
  closed_at timestamptz,
  primary key (tenant_id, admission_id)
);

alter table work_admissions add column if not exists closed_at timestamptz;
