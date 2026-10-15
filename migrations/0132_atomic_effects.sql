-- One business effect binds source document, stock ledger, work-order execution,
-- and a pending partner posting. Replay of the same action does not insert a second row.

create table if not exists atomic_source_documents (
  tenant_id uuid not null,
  document_id uuid not null,
  source_action text not null,
  action_no text not null,
  qty_micros bigint not null,
  amount_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, document_id),
  unique (tenant_id, source_action, action_no)
);

create table if not exists atomic_posting_events (
  tenant_id uuid not null,
  event_id uuid not null,
  document_id uuid not null,
  source_action text not null,
  action_no text not null,
  amount_cent bigint not null,
  posting_status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, event_id)
);
