-- A free login session belongs to the one business seat.
-- The session row does not occupy a second seat.

create table if not exists free_login_sessions (
  tenant_id uuid not null,
  session_id uuid not null,
  user_id text not null,
  client_kind text not null,
  occupies_seat boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, session_id)
);
