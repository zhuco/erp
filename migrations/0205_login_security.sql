-- Login failures store a backoff, not the secret.
-- Request limits are counted per enterprise, account, and source.

create table if not exists login_attempts (
  tenant_id uuid not null,
  attempt_id uuid not null,
  account_id text not null,
  source_text text not null,
  device_id text not null,
  failed_count integer not null,
  backoff_seconds bigint not null,
  secret_ref text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, attempt_id)
);

create table if not exists request_limits (
  tenant_id uuid not null,
  limit_id uuid not null,
  account_id text not null,
  source_text text not null,
  observed bigint not null,
  decision text not null,
  retry_seconds bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, limit_id)
);

create table if not exists device_notices (
  tenant_id uuid not null,
  notice_id uuid not null,
  account_id text not null,
  device_id text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, notice_id)
);

create table if not exists admin_factors (
  tenant_id uuid not null,
  factor_id uuid not null,
  operator_id text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, factor_id)
);
