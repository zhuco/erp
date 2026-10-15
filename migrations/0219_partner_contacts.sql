-- Contacts and addresses stay on the partner. They do not replace the partner id.
-- A merge preview counts references without moving them.

create table if not exists shared_partner_contacts (
  tenant_id uuid not null,
  contact_id uuid not null,
  partner_id uuid not null,
  contact_name text not null,
  phone text not null,
  address_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, contact_id)
);

create table if not exists shared_partner_merge_previews (
  tenant_id uuid not null,
  preview_id uuid not null,
  source_id uuid not null,
  target_id uuid not null,
  ref_count bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, preview_id)
);

create table if not exists shared_partner_role_balances (
  tenant_id uuid not null,
  balance_id uuid not null,
  partner_id uuid not null,
  customer_no text not null,
  supplier_no text not null,
  receivable_cent bigint not null,
  payable_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, balance_id)
);
