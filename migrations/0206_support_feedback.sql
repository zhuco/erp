-- Public status stores a code only. Feedback cites a failed document without its body.
-- An internal ticket records scope, reason, handler, and result.

create table if not exists service_statuses (
  tenant_id uuid not null,
  status_id uuid not null,
  status_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, status_id)
);

create table if not exists support_feedback (
  tenant_id uuid not null,
  feedback_id uuid not null,
  feedback_no text not null,
  request_ref text not null,
  feature_id text not null,
  document_no text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, feedback_id),
  unique (tenant_id, feedback_no)
);

create table if not exists support_ops_tickets (
  tenant_id uuid not null,
  ticket_id uuid not null,
  feedback_no text not null,
  scope_text text not null,
  reason_text text not null,
  handler_id text not null,
  result_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, ticket_id)
);

create table if not exists diagnostic_exports (
  tenant_id uuid not null,
  export_id uuid not null,
  line_count bigint not null,
  bound_count bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, export_id)
);
