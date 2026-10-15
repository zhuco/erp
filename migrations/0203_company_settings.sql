-- Company settings keep a version and an effective time.
-- Changing a setting does not rewrite posted document amounts or dates.

create table if not exists company_settings (
  tenant_id uuid not null,
  setting_id uuid not null,
  version_no integer not null,
  company_name text not null,
  numbering_rule text not null,
  currency text not null,
  timezone_name text not null,
  qty_precision integer not null,
  buy_enabled boolean not null,
  make_enabled boolean not null,
  sell_enabled boolean not null,
  default_warehouse text not null,
  effective_at text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, setting_id),
  unique (tenant_id, version_no)
);

create table if not exists company_documents (
  tenant_id uuid not null,
  document_id uuid not null,
  document_no text not null,
  kind text not null,
  amount_cent bigint not null,
  document_date text not null,
  timezone_name text not null,
  bom_required boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, document_id),
  unique (tenant_id, document_no)
);
