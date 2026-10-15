-- A partner may be both a customer and a supplier. Numbers do not replace the partner id.
-- A term revision does not rewrite the term stored on an existing document.

create table if not exists shared_partners (
  tenant_id uuid not null,
  partner_id uuid not null,
  name text not null,
  customer boolean not null,
  supplier boolean not null,
  customer_no text not null default '',
  supplier_no text not null default '',
  term_days integer not null,
  tax_no text not null default '',
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, partner_id)
);

create table if not exists shared_partner_refs (
  tenant_id uuid not null,
  partner_id uuid not null,
  document_id text not null,
  document_date text not null,
  term_days integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, document_id)
);

create table if not exists shared_partner_merges (
  tenant_id uuid not null,
  source_id uuid not null,
  target_id uuid not null,
  moved_refs bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, source_id)
);

create table if not exists shared_partner_terms (
  tenant_id uuid not null,
  partner_id uuid not null,
  term_days integer not null,
  revision_date text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, partner_id, revision_date)
);
