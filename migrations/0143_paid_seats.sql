-- A paid plan may raise the login-seat limit. An invitation does not occupy a seat.
-- Downgrade keeps one selected account and revokes the others without deleting history.

create table if not exists paid_seat_plans (
  tenant_id uuid primary key,
  plan_code text not null,
  seat_limit integer not null,
  created_at timestamptz not null default now()
);

create table if not exists member_seats (
  tenant_id uuid not null,
  user_id text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id)
);

create table if not exists member_approvals (
  tenant_id uuid not null,
  approval_id uuid not null,
  submitter_user_id text not null,
  approver_user_id text not null,
  condition_text text not null,
  version_no integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, approval_id)
);

create table if not exists seat_downgrade_results (
  tenant_id uuid not null,
  user_id text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id)
);
