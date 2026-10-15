-- One login seat may have several clients. Renewal is not a business effect.
-- Revoking one client leaves the others. A password change advances the epoch.

create table if not exists login_clients (
  tenant_id uuid not null,
  session_id uuid not null,
  user_id text not null,
  client_kind text not null,
  status text not null,
  token_epoch integer not null,
  expires_at timestamptz not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, session_id)
);

create table if not exists login_epochs (
  tenant_id uuid not null,
  user_id text not null,
  epoch_no integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, user_id)
);

create table if not exists password_recoveries (
  tenant_id uuid not null,
  recovery_id uuid not null,
  user_id text not null,
  token_text text not null,
  used boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, recovery_id)
);
