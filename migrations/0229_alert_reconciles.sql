-- An alert reconcile stores the stored count beside an independent query count.
-- It does not create a purchase or a work order.

create table if not exists shared_alert_reconciles (
  tenant_id uuid not null,
  reconcile_id uuid not null,
  rule_code text not null,
  object_code text not null,
  cutoff_at text not null,
  stored_count bigint not null,
  query_count bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, reconcile_id)
);
