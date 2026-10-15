-- Shared history records an action. It does not replace stock or payable rows.
-- A reversal is a new row. The original action stays readable.

create table if not exists shared_document_actions (
  tenant_id uuid not null,
  action_id uuid not null,
  document_kind text not null,
  document_id uuid not null,
  action_name text not null,
  status_after text not null,
  version_no integer not null,
  actor_user_id text not null,
  reverses_action_id uuid,
  created_at timestamptz not null default now(),
  primary key (tenant_id, action_id)
);
