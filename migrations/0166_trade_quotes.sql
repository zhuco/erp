-- A quote revision keeps its price. Conversion stores that version and currency.
-- A credit hold is not a payment. A purchase suggestion does not pay a supplier.

create table if not exists shared_quote_revisions (
  tenant_id uuid not null,
  revision_id uuid not null,
  quote_id uuid not null,
  version_no integer not null,
  partner_code text not null,
  currency text not null,
  item_code text not null,
  qty_micros bigint not null,
  price_cent bigint not null,
  start_date text not null,
  end_date text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, revision_id),
  unique (tenant_id, quote_id, version_no)
);

create table if not exists shared_quote_conversions (
  tenant_id uuid not null,
  conversion_id uuid not null,
  quote_id uuid not null,
  version_no integer not null,
  currency text not null,
  price_cent bigint not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, conversion_id)
);

create table if not exists shared_credit_holds (
  tenant_id uuid not null,
  hold_id uuid not null,
  partner_code text not null,
  amount_cent bigint not null,
  left_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, hold_id)
);

create table if not exists shared_purchase_suggestions (
  tenant_id uuid not null,
  suggestion_id uuid not null,
  item_code text not null,
  qty_micros bigint not null,
  paid boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, suggestion_id)
);
