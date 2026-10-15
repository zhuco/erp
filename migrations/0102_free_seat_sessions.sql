-- 0102: free-plan login seats and multi-device sessions.
-- Seat count and session count are separate. Official operators are not tenant users.
-- Existing rows are not rewritten. New business users are limited by the tenant plan.

alter table tenants
  add column if not exists plan_code text not null default 'FREE',
  add column if not exists login_seat_limit integer not null default 1,
  add column if not exists session_policy text not null default 'MULTI',
  add column if not exists max_active_sessions integer not null default 8,
  add column if not exists credentials_epoch integer not null default 0;

alter table tenants
  drop constraint if exists tenants_plan_code_check;
alter table tenants
  add constraint tenants_plan_code_check
  check (plan_code in ('FREE', 'PAID'));

alter table tenants
  drop constraint if exists tenants_login_seat_limit_check;
alter table tenants
  add constraint tenants_login_seat_limit_check
  check (login_seat_limit >= 1);

alter table tenants
  drop constraint if exists tenants_session_policy_check;
alter table tenants
  add constraint tenants_session_policy_check
  check (session_policy in ('MULTI', 'SINGLE'));

alter table tenants
  drop constraint if exists tenants_max_active_sessions_check;
alter table tenants
  add constraint tenants_max_active_sessions_check
  check (max_active_sessions >= 1 and max_active_sessions <= 32);

create table if not exists auth_sessions (
  session_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  user_id uuid not null references users(user_id),
  status text not null default 'ACTIVE',
  client_kind text not null default 'UNKNOWN',
  client_label text not null default '',
  created_at timestamptz not null default now(),
  last_seen_at timestamptz not null default now(),
  expires_at timestamptz not null,
  revoked_at timestamptz,
  revoke_reason text,
  credentials_epoch integer not null default 0,
  constraint auth_sessions_status_check
    check (status in ('ACTIVE', 'REVOKED', 'EXPIRED')),
  constraint auth_sessions_client_kind_check
    check (client_kind in ('DESKTOP', 'WEB', 'UNKNOWN'))
);

create index if not exists idx_auth_sessions_user_active
  on auth_sessions(tenant_id, user_id, status, expires_at desc);

create index if not exists idx_auth_sessions_tenant_created
  on auth_sessions(tenant_id, created_at desc);
