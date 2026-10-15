-- Recovery checkpoints keep database and attachment cutoffs apart.
-- A shared-database rewind is refused. Compare happens in an isolated target.

create table if not exists recovery_checkpoints (
  tenant_id uuid not null,
  checkpoint_id uuid not null,
  database_cutoff text not null,
  attachment_cutoff text not null,
  backup_digest text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, checkpoint_id)
);

create table if not exists recovery_attachments (
  tenant_id uuid not null,
  checkpoint_id uuid not null,
  attachment_id text not null,
  present boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, checkpoint_id, attachment_id)
);

create table if not exists recovery_compares (
  tenant_id uuid not null,
  compare_id uuid not null,
  checkpoint_id uuid not null,
  target_database text not null,
  isolated boolean not null,
  status text not null,
  ledger_cent bigint not null,
  document_cent bigint not null,
  pending_tasks bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, compare_id)
);
