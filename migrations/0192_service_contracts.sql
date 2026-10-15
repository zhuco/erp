-- Dedicated instances cite a passed release gate. A received ticket stays OPEN.

create table if not exists dedicated_instances (
  tenant_id uuid not null,
  instance_id uuid not null,
  artifact_sha text not null,
  core_version text not null,
  storage_bytes bigint not null,
  backup_bytes bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, instance_id)
);

create table if not exists service_quotes (
  tenant_id uuid not null,
  quote_id uuid not null,
  resource_text text not null,
  term_end text not null,
  deliverable text not null,
  response_text text not null,
  exit_path text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, quote_id)
);

create table if not exists support_tickets (
  tenant_id uuid not null,
  ticket_id uuid not null,
  ticket_no text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, ticket_id)
);
