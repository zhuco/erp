-- Stored report fields. Stale marking reads this catalog and does not take a caller list.
create table if not exists report_field_catalog (
  tenant_id uuid not null,
  report_scope text not null,
  field_name text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, report_scope, field_name)
);
