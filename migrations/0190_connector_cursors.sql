-- Connector credentials, cursors, and external effects stay inside one tenant.
-- A stored decision is not a live call to an external system.

create table if not exists connector_credentials (
  tenant_id uuid not null,
  credential_id uuid not null,
  system_code text not null,
  secret_ref text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, credential_id),
  unique (tenant_id, system_code)
);

create table if not exists connector_cursors (
  tenant_id uuid not null,
  cursor_id uuid not null,
  system_code text not null,
  cursor_text text not null,
  external_object_id text not null,
  command_id text not null,
  mapping_version integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, cursor_id),
  unique (tenant_id, system_code, command_id)
);

create table if not exists connector_effects (
  tenant_id uuid not null,
  effect_id uuid not null,
  system_code text not null,
  command_id text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, effect_id)
);
