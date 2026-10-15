-- A recommendation is a suggestion. The chosen issue is a separate row and checks the stock version.

create table if not exists shared_allocation_rules (
  tenant_id uuid not null,
  rule_id uuid not null,
  rule_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, rule_id)
);

create table if not exists shared_allocation_suggestions (
  tenant_id uuid not null,
  suggestion_id uuid not null,
  request_ref text not null,
  rule_code text not null,
  batch_code text not null,
  location_code text not null,
  qty_micros bigint not null,
  stock_version bigint not null,
  excluded_text text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, suggestion_id)
);

create table if not exists shared_allocation_issues (
  tenant_id uuid not null,
  issue_id uuid not null,
  suggestion_id uuid not null,
  qty_micros bigint not null,
  stock_version bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, issue_id)
);
