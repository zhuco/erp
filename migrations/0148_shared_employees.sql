-- An employee id is not a login user id. Leaving keeps historical labor reports.

create table if not exists shared_employees (
  tenant_id uuid not null,
  employee_id uuid not null,
  employee_code text not null,
  user_id text not null,
  department text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, employee_id),
  unique (tenant_id, employee_code)
);

create table if not exists shared_employee_assignments (
  tenant_id uuid not null,
  employee_id uuid not null,
  crew_code text not null,
  work_center text not null,
  start_date text not null,
  end_date text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, employee_id, crew_code, start_date)
);

create table if not exists shared_labor_reports (
  tenant_id uuid not null,
  report_id text not null,
  employee_id uuid not null,
  reporter_user_id text not null,
  certifier_user_id text not null,
  minutes bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, report_id)
);
