-- An inventory alert stores a cutoff and a reason. It does not create a payment or a production order.

create table if not exists shared_inventory_alerts (
  tenant_id uuid not null,
  alert_id uuid not null,
  rule_code text not null,
  object_code text not null,
  cutoff_at text not null,
  reason text not null,
  observed bigint not null,
  threshold bigint not null,
  state text not null,
  notified boolean not null,
  purchase_id text not null default '',
  work_order_id text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, alert_id)
);
