-- Personal report views are isolated by tenant, owner, scope, and name.
-- A structure change records stale fields and does not rewrite the formula.

create table if not exists saved_report_views (
  tenant_id uuid not null,
  view_id uuid not null,
  owner_user_id text not null,
  report_scope text not null,
  name text not null,
  date_from text not null,
  date_to text not null,
  dimension text not null,
  measure text not null,
  chart_type text not null,
  formula text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, view_id),
  unique (tenant_id, owner_user_id, report_scope, name)
);

create table if not exists saved_report_stale_fields (
  tenant_id uuid not null,
  view_id uuid not null,
  field_name text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, view_id, field_name)
);
