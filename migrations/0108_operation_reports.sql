-- 0108: operation reports. The operator is a worker record, not the login user.
-- A reversal keeps the original row.

create table if not exists mfg_reports (
  tenant_id uuid not null references tenants(tenant_id),
  report_id uuid not null default gen_random_uuid(),
  work_order_id uuid not null,
  operation_code text not null,
  good_micros bigint not null,
  rejected_micros bigint not null,
  rework_micros bigint not null,
  minutes bigint not null,
  operator_id text not null,
  session_user_id text not null,
  session_id text not null default '',
  crew_code text not null default '',
  started_at text not null default '',
  ended_at text not null default '',
  reverses_report_id uuid,
  created_at timestamptz not null default now(),
  primary key (tenant_id, report_id),
  constraint mfg_reports_operator_not_login
    check (operator_id <> session_user_id and operator_id <> session_id)
);
