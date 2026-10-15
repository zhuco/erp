-- Device pulses, human confirmation, and official output stay in separate rows.
-- A repeated device event id is not a second pulse and is not an operation report.

create table if not exists device_events (
  tenant_id uuid not null,
  event_id uuid not null,
  device_code text not null,
  device_event_id text not null,
  device_time text not null,
  received_time text not null,
  sequence_no bigint not null,
  payload_version integer not null,
  captured_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, event_id),
  unique (tenant_id, device_code, device_event_id)
);

create table if not exists device_confirmations (
  tenant_id uuid not null,
  confirm_id uuid not null,
  device_code text not null,
  confirmed_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, confirm_id)
);

create table if not exists device_outputs (
  tenant_id uuid not null,
  output_id uuid not null,
  device_code text not null,
  shift_code text not null,
  planned_minutes bigint not null,
  downtime_minutes bigint not null,
  ideal_cycle_micros bigint not null,
  captured_micros bigint not null,
  confirmed_micros bigint not null,
  official_good_micros bigint not null,
  availability_ppm bigint not null,
  performance_ppm bigint not null,
  quality_ppm bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, output_id)
);
