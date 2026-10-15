-- Work-center load is a manual schedule warning. It is not automatic scheduling.
-- A rest day adds no capacity. A later rate version does not rewrite the earlier one.

create table if not exists shared_work_centers (
  tenant_id uuid not null,
  center_id uuid not null,
  center_code text not null,
  machines bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, center_id),
  unique (tenant_id, center_code)
);

create table if not exists shared_work_calendar_days (
  tenant_id uuid not null,
  center_id uuid not null,
  day_code text not null,
  rest boolean not null,
  shift_minutes bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, center_id, day_code)
);

create table if not exists shared_work_center_loads (
  tenant_id uuid not null,
  load_id uuid not null,
  center_id uuid not null,
  capacity_minutes bigint not null,
  load_minutes bigint not null,
  overload_minutes bigint not null,
  optimized boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, load_id)
);

create table if not exists shared_work_center_rates (
  tenant_id uuid not null,
  rate_id uuid not null,
  center_id uuid not null,
  version_no integer not null,
  rate_cent bigint not null,
  start_date text not null,
  end_date text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, rate_id),
  unique (tenant_id, center_id, version_no)
);
