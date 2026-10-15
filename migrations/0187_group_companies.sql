-- A group links company spaces. Ownership, plant, and settlement stay separate.
-- An internal pair does not merge tenant stock.

create table if not exists group_companies (
  tenant_id uuid not null,
  company_id uuid not null,
  group_code text not null,
  company_code text not null,
  legal_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, company_id),
  unique (tenant_id, company_code)
);

create table if not exists group_stock_titles (
  tenant_id uuid not null,
  title_id uuid not null,
  company_code text not null,
  owner_code text not null,
  plant_code text not null,
  settlement_code text not null,
  qty bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, title_id)
);

create table if not exists intercompany_pairs (
  tenant_id uuid not null,
  pair_id uuid not null,
  pair_no text not null,
  seller_company text not null,
  buyer_company text not null,
  amount_cent bigint not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, pair_id),
  unique (tenant_id, pair_no)
);

create table if not exists group_eliminations (
  tenant_id uuid not null,
  elimination_id uuid not null,
  pair_no text not null,
  company_a_cent bigint not null,
  company_b_cent bigint not null,
  internal_cent bigint not null,
  combined_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, elimination_id)
);
