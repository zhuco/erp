-- Public origins are exact HTTPS names. Cookie sessions carry CSRF and cookie flags.
-- Download links are tenant-scoped and expire. Diagnostics do not store secrets.

create table if not exists public_origins (
  tenant_id uuid not null,
  origin text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, origin)
);

create table if not exists credential_epochs (
  tenant_id uuid not null,
  actor_user_id text not null,
  epoch integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, actor_user_id)
);

create table if not exists browser_sessions (
  tenant_id uuid not null,
  session_id uuid not null,
  actor_user_id text not null,
  mode text not null,
  http_only boolean not null,
  secure boolean not null,
  same_site text not null,
  csrf_token text not null default '',
  credential_epoch integer not null,
  status text not null,
  expires_at timestamptz not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, session_id)
);

create table if not exists download_links (
  tenant_id uuid not null,
  link_id uuid not null,
  actor_user_id text not null,
  object_id text not null,
  expires_at timestamptz not null,
  status text not null default 'ACTIVE',
  created_at timestamptz not null default now(),
  primary key (tenant_id, link_id)
);

alter table download_links add column if not exists status text not null default 'ACTIVE';

create table if not exists diagnostic_logs (
  tenant_id uuid not null,
  log_id uuid not null,
  redacted_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, log_id)
);
