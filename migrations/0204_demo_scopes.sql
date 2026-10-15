-- Demo rows stay in their own scope. Leaving the demo does not copy them into the real ledger.

create table if not exists demo_scopes (
  tenant_id uuid not null,
  scope_id uuid not null,
  scope_code text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, scope_id),
  unique (tenant_id, scope_code)
);

create table if not exists getting_started_steps (
  tenant_id uuid not null,
  step_id uuid not null,
  step_code text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, step_id)
);

create table if not exists scoped_documents (
  tenant_id uuid not null,
  document_id uuid not null,
  scope_code text not null,
  document_no text not null,
  amount_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, document_id)
);
